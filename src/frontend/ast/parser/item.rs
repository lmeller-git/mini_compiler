use indexmap::IndexMap;

use crate::{
    expect_token,
    frontend::{
        ast::{
            Ast, Function, Item, Line, LinkAttr, LinkMeta,
            cfg::CfgEnv,
            error::{AstErr, Diagnostics},
            parser::expr::parse_expr,
        },
        lexer::{Token, TokenStream},
    },
    parse_list, skip_until, skip_until_kw,
};

impl Ast {
    pub fn from_stream<'a>(s: &mut TokenStream<'a>, cfg_env: &CfgEnv) -> (Self, Diagnostics<'a>) {
        let mut diagnostics = Diagnostics::new();
        let mut functions = IndexMap::new();
        loop {
            let anchor = s.peek().span.clone();
            match s.peek().as_ref() {
                Token::EOF => break,
                Token::Keyword("cfg") => {
                    s.advance();
                    let cfg = cfg_env.eval_cfg_expr(&parse_expr(s, 0., &mut diagnostics));

                    let _semi = s.peek();
                    if *_semi.as_ref() != Token::Semi {
                        diagnostics.errs.push(
                            AstErr::UnclosedBlock {
                                at: s.peek().clone(),
                                expected: vec![Token::Semi],
                            }
                            .at(anchor.clone().merge(s.last_span.clone())),
                        );
                        skip_until!(s, Token::Semi);
                    }
                    s.advance();

                    if !cfg {
                        skip_until!(
                            s,
                            Token::Keyword("begin_def")
                                | Token::Keyword("extern_def")
                                | Token::Keyword("public")
                        );
                        match s.peek().as_ref() {
                            Token::Keyword("begin_def") | Token::Keyword("public") => {
                                skip_until!(s, Token::Keyword("end_def"))
                            }
                            Token::Keyword("extern_def") => skip_until!(s, Token::Semi),
                            Token::EOF => {
                                diagnostics.errs.push(
                                    AstErr::UnexecpectedEOF
                                        .at(anchor.clone().merge(s.last_span.clone())),
                                );
                            }
                            _ => unreachable!(),
                        }
                        s.advance();
                    }
                }
                Token::Keyword(kw)
                    if matches!(*kw, "link_attr" | "begin_def" | "public" | "extern_def") =>
                {
                    let link_attr = LinkAttr::parse(s, &mut diagnostics);

                    if let Some(f) =
                        Function::parse(&functions, s, link_attr, cfg_env, &mut diagnostics)
                    {
                        functions.insert(f.name.clone(), Item::Function(f));
                    } else {
                        functions.insert(
                            format!("<__malformed_{}>", diagnostics.errs.len()),
                            Item::Malformed,
                        );
                        skip_until!(
                            s,
                            Token::Keyword("end_def")
                                | Token::Keyword("cfg")
                                | Token::Keyword("link_attr")
                                | Token::Keyword("begin_def")
                                | Token::Keyword("extern_def")
                                | Token::Keyword("public")
                        );
                        expect_token!(s, &mut diagnostics, anchor, [Token::Keyword("end_def")]);
                    }
                }
                _invalid => {
                    diagnostics.errs.push(
                        AstErr::UnexpectedToken {
                            expected: vec![
                                Token::Keyword("cfg"),
                                Token::Keyword("begin_def"),
                                Token::Keyword("public"),
                                Token::Keyword("link_attr"),
                            ],
                            found: s.peek().clone(),
                        }
                        .at(anchor),
                    );
                    skip_until!(
                        s,
                        Token::Keyword("begin_def")
                            | Token::Keyword("public")
                            | Token::Keyword("cfg")
                            | Token::Keyword("link_attr")
                    )
                }
            }
        }
        (Self { functions }, diagnostics)
    }
}

impl LinkAttr {
    fn parse<'a>(stream: &mut TokenStream<'a>, diagnostics: &mut Diagnostics<'a>) -> Self {
        let mut zelf = Self::default();
        while let Token::Keyword("link_attr") = stream.peek().as_ref() {
            let anchor = stream.peek().span.clone();
            stream.advance();
            match (
                stream.peek().as_ref(),
                stream.peekn(1).as_ref(),
                stream.peekn(2).as_ref(),
            ) {
                (Token::Ident("section"), Token::Ident(sec), _) => {
                    zelf = zelf.with_section(sec.to_string());
                    stream.advance();
                    stream.advance();
                }
                (Token::Ident("raw"), Token::Ident("section"), Token::Ident(sec)) => {
                    zelf = zelf.with_meta(LinkMeta::Raw).with_section(sec.to_string());
                    stream.advance();
                    stream.advance();
                    stream.advance();
                }
                (Token::Ident("vis"), next, _) => {
                    match next {
                        Token::Ident("public") => zelf = zelf.into_pub(),
                        Token::Ident("private") => zelf.is_public = false,
                        _tok => {
                            diagnostics.errs.push(
                                AstErr::UnexpectedToken {
                                    expected: vec![
                                        Token::Keyword("public"),
                                        Token::Keyword("private"),
                                    ],
                                    found: stream.peekn(1).clone(),
                                }
                                .at(anchor.merge(stream.peek().span.clone())),
                            );
                            skip_until!(stream, Token::Semi);
                            stream.advance();
                            continue;
                        }
                    }
                    stream.advance();
                    stream.advance();
                }
                (Token::Ident("extern"), _, _) => {
                    zelf = zelf.into_external();
                    stream.advance();
                }
                (_tok, _, _) => {
                    diagnostics.errs.push(
                        AstErr::UnexpectedToken {
                            expected: vec![
                                Token::Keyword("section"),
                                Token::Keyword("raw"),
                                Token::Keyword("vis"),
                            ],
                            found: stream.peek().clone(),
                        }
                        .at(anchor),
                    );
                    skip_until!(stream, Token::Semi);
                    stream.advance();
                    continue;
                }
            }

            let _tok = stream.peek();
            if *_tok.as_ref() != Token::Semi {
                diagnostics.errs.push(
                    AstErr::UnclosedBlock {
                        at: _tok.clone(),
                        expected: vec![Token::Semi],
                    }
                    .at(anchor.merge(stream.last_span.clone())),
                );
                skip_until!(
                    stream,
                    Token::Keyword("begin_def")
                        | Token::Keyword("extern_def")
                        | Token::Semi
                        | Token::Keyword("link_attr")
                );
            } else {
                stream.advance();
            }
        }

        zelf
    }
}

impl Function {
    fn parse<'a>(
        funcs: &IndexMap<String, Item>,
        stream: &mut TokenStream<'a>,
        mut link_attr: LinkAttr,
        cfg_env: &CfgEnv,
        diagnostics: &mut Diagnostics<'a>,
    ) -> Option<Self> {
        let mut kw = stream.peek();
        let anchor = kw.span.clone();

        let is_public = *kw.as_ref() == Token::Keyword("public");
        if is_public {
            stream.advance();
            kw = stream.peek();
            link_attr = link_attr.into_pub();
        }
        let has_body = match kw.as_ref() {
            Token::Keyword("extern_def") => false,
            Token::Keyword("begin_def") => true,
            _tok => {
                diagnostics.errs.push(
                    AstErr::UnexpectedToken {
                        expected: vec![Token::Keyword("extern_def"), Token::Keyword("begin_def")],
                        found: kw.clone(),
                    }
                    .at(stream.last_span.clone()),
                );
                return None;
            }
        };

        if !has_body {
            link_attr = link_attr.into_external();
        }

        stream.advance();

        let ident = stream.peek();
        let Token::Ident(_ident) = ident.as_ref() else {
            diagnostics.errs.push(
                AstErr::UnexpectedToken {
                    expected: vec![Token::Ident("<function name>")],
                    found: ident.clone(),
                }
                .at(anchor.merge(stream.last_span.clone())),
            );
            return None;
        };
        let name = _ident.to_string();
        stream.advance();

        let args = parse_list!(stream, anchor, Token::Semi, Token::Comma, Token::Ident(ident) => ident.to_string());

        let args = match args {
            Ok(a) => a,
            Err(e) => {
                diagnostics.errs.push(e);
                Vec::new()
            }
        };

        let body = if has_body {
            let mut body = Vec::new();

            while *stream.peek().as_ref() != Token::Keyword("end_def") {
                if matches!(
                    *stream.peek().as_ref(),
                    Token::EOF
                        | Token::Keyword("begin_def")
                        | Token::Keyword("extern_def")
                        | Token::Keyword("link_attr")
                        | Token::Keyword("public")
                ) {
                    break;
                }
                if let Token::Keyword("cfg") = stream.peek().as_ref() {
                    stream.advance();
                    let cfg = cfg_env.eval_cfg_expr(&parse_expr(stream, 0., diagnostics));
                    stream.advance();
                    if !cfg {
                        loop {
                            let tok_is_if = *stream.peek().as_ref() == Token::Keyword("if");
                            skip_until!(stream, Token::Semi);
                            stream.advance();
                            if !tok_is_if {
                                break;
                            }
                        }
                        continue;
                    }
                }
                let line = Line::parse(funcs, stream, diagnostics);
                if line == Line::Malformed {
                    skip_until_kw!(stream, Token::Semi);
                }
                body.push(line);
            }
            Some(body)
        } else {
            None
        };

        if !has_body || matches!(stream.peek().as_ref(), Token::Keyword("end_def")) {
            if has_body {
                stream.advance();
            }
        } else {
            diagnostics.errs.push(
                AstErr::UnclosedBlock {
                    at: stream.peek().clone(),
                    expected: vec![Token::Keyword("end_def")],
                }
                .at(anchor.merge(stream.last_span.clone())),
            );
        }

        Some(Self {
            name,
            body,
            args,
            link_attr,
        })
    }
}
