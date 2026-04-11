use indexmap::IndexMap;

use crate::{
    frontend::{
        ast::{
            Ast, Function, Item, Line, LinkAttr, LinkMeta,
            cfg::CfgEnv,
            error::{AstErr, Diagnostics, IntoSpanned, Spanned},
            parser::expr::parse_expr,
        },
        lexer::{Token, TokenStream},
    },
    kw, skip_until, skip_until_or_over, unclosed_block, unexpected,
};

impl Ast {
    pub fn from_stream<'a>(s: &mut TokenStream<'a>, cfg_env: &CfgEnv) -> (Self, Diagnostics<'a>) {
        let mut diagnostics = Diagnostics::new();
        let mut functions = IndexMap::new();
        let mut skip_next = false;
        loop {
            let anchor = s.peek().span.clone();
            match s.peek().as_ref() {
                Token::EOF => break,
                Token::Keyword("cfg") => {
                    s.advance();
                    skip_next = !cfg_env.eval_cfg_expr(&parse_expr(s, 0., &mut diagnostics));
                    if *s.peek().as_ref() != Token::Semi {
                        unexpected!(
                            diagnostics,
                            [Token::Semi],
                            s.peek().clone(),
                            anchor.clone().merge(s.last_span.clone())
                        );
                    }
                    skip_until_or_over!(s, kw!(Token::Semi), Token::Semi);
                }
                Token::Keyword(kw)
                    if matches!(*kw, "link_attr" | "begin_def" | "public" | "extern_def") =>
                {
                    let mut item_diagnostic = Diagnostics::new();
                    let link_attr_start = s.peek().span.clone();
                    let link_attr = LinkAttr::parse(s, &mut item_diagnostic);

                    if let Some(f) = Function::parse(
                        s,
                        link_attr.into_spanned(link_attr_start.merge(s.last_span.clone())),
                        cfg_env,
                        &mut item_diagnostic,
                    ) && !skip_next
                    {
                        functions.insert(
                            f.name.as_ref().clone(),
                            Item::Function(f.into_spanned(anchor.merge(s.last_span.clone()))),
                        );
                    }

                    diagnostics.warns.append(&mut item_diagnostic.warns);

                    if skip_next {
                        diagnostics.warns.extend(
                            item_diagnostic
                                .errs
                                .into_iter()
                                .map(|e| e.inner.into_warn("cfg".into()).at(e.span)),
                        );
                    } else {
                        diagnostics.errs.append(&mut item_diagnostic.errs);
                    }
                    skip_next = false;
                }
                _invalid => {
                    unexpected!(
                        diagnostics,
                        [
                            Token::Keyword("cfg"),
                            Token::Keyword("begin_def"),
                            Token::Keyword("public"),
                            Token::Keyword("link_attr")
                        ],
                        s.peek().clone(),
                        anchor
                    );

                    skip_next = false;
                    skip_until!(
                        s,
                        Token::Keyword("begin_def")
                            | Token::Keyword("extern_def")
                            | Token::Keyword("public")
                            | Token::Keyword("cfg")
                            | Token::Keyword("link_attr")
                    );
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
                            unexpected!(
                                diagnostics,
                                [Token::Ident("public"), Token::Ident("private")],
                                stream.peekn(1).clone(),
                                anchor.merge(stream.peek().span.clone())
                            );

                            skip_until!(
                                stream,
                                Token::Keyword("begin_def")
                                    | Token::Keyword("extern_def")
                                    | Token::Keyword("public")
                                    | Token::Keyword("cfg")
                                    | Token::Keyword("link_attr")
                            );
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
                    unexpected!(
                        diagnostics,
                        [
                            Token::Keyword("section"),
                            Token::Keyword("raw"),
                            Token::Keyword("vis"),
                            Token::Keyword("section")
                        ],
                        stream.peek().clone(),
                        anchor
                    );

                    skip_until!(
                        stream,
                        Token::Keyword("begin_def")
                            | Token::Keyword("extern_def")
                            | Token::Keyword("public")
                            | Token::Keyword("cfg")
                            | Token::Keyword("link_attr")
                    );
                    continue;
                }
            }

            if *stream.peek().as_ref() != Token::Semi {
                unclosed_block!(
                    diagnostics,
                    [Token::Semi],
                    stream.peek().clone(),
                    anchor.merge(stream.last_span.clone())
                );
            }

            skip_until!(
                stream,
                Token::Keyword("begin_def")
                    | Token::Keyword("extern_def")
                    | Token::Keyword("public")
                    | Token::Keyword("cfg")
                    | Token::Keyword("link_attr")
            );
        }

        zelf
    }
}

impl Function {
    fn parse<'a>(
        stream: &mut TokenStream<'a>,
        link_attr: Spanned<LinkAttr>,
        cfg_env: &CfgEnv,
        diagnostics: &mut Diagnostics<'a>,
    ) -> Option<Self> {
        let func = Self::parse_inner(stream, link_attr, cfg_env, diagnostics);
        if func.is_none() {
            skip_until!(
                stream,
                Token::Keyword("begin_def")
                    | Token::Keyword("extern_def")
                    | Token::Keyword("link_attr")
                    | Token::Keyword("cfg")
            );
        }
        func
    }

    fn parse_inner<'a>(
        stream: &mut TokenStream<'a>,
        mut link_attr: Spanned<LinkAttr>,
        cfg_env: &CfgEnv,
        diagnostics: &mut Diagnostics<'a>,
    ) -> Option<Self> {
        let mut kw = stream.peek();
        let anchor = kw.span.clone();

        let is_public = *kw.as_ref() == Token::Keyword("public");
        if is_public {
            stream.advance();
            kw = stream.peek();
            link_attr = link_attr.map(|attr| attr.into_pub());
        }

        let is_local = match kw.as_ref() {
            Token::Keyword("extern_def") => false,
            Token::Keyword("begin_def") => true,
            _tok => {
                unexpected!(
                    diagnostics,
                    [Token::Keyword("begin_def"), Token::Keyword("extern_def")],
                    kw.clone(),
                    stream.last_span.clone()
                );
                return None;
            }
        };

        if !is_local {
            link_attr = link_attr.map(|attr| attr.into_external());
        }

        stream.advance();

        let ident = stream.peek();
        let Token::Ident(ident_str) = ident.as_ref() else {
            unexpected!(
                diagnostics,
                [Token::Ident("<ident>")],
                ident.clone(),
                anchor.merge(stream.last_span.clone())
            );
            return None;
        };
        let name = ident_str.to_string().into_spanned(ident.span.clone());
        stream.advance();

        let mut args = Vec::new();

        while *stream.peek().as_ref() != Token::Semi {
            let Token::Ident(ident) = stream.peek().as_ref() else {
                unexpected!(
                    diagnostics,
                    [Token::Ident("<ident>")],
                    stream.peek().clone(),
                    anchor.merge(stream.last_span.clone())
                );
                return None;
            };

            args.push(ident.to_string().into_spanned(stream.peek().span.clone()));
            stream.advance();

            let next = stream.peek();
            if *next.as_ref() == Token::Comma {
                stream.advance();
            } else if *next.as_ref() != Token::Semi {
                unclosed_block!(
                    diagnostics,
                    [Token::Semi, Token::Comma],
                    next.clone(),
                    anchor.clone().merge(stream.last_span.clone())
                );
                return None;
            }
        }

        stream.advance();

        let body = if is_local {
            let mut body = Vec::new();
            let body_anchor = stream.peek().span.clone();

            while *stream.peek().as_ref() != Token::Keyword("end_def") {
                if matches!(
                    *stream.peek().as_ref(),
                    Token::EOF
                        | Token::Keyword("begin_def")
                        | Token::Keyword("extern_def")
                        | Token::Keyword("link_attr")
                        | Token::Keyword("public")
                ) {
                    unclosed_block!(
                        diagnostics,
                        [Token::Keyword("end_def")],
                        stream.peek().clone(),
                        anchor.merge(stream.last_span.clone())
                    );
                    return None;
                }

                let cfg = if let Token::Keyword("cfg") = stream.peek().as_ref() {
                    stream.advance();
                    let cfg = cfg_env.eval_cfg_expr(&parse_expr(stream, 0., diagnostics));
                    if *stream.peek().as_ref() != Token::Semi {
                        unexpected!(
                            diagnostics,
                            [Token::Semi],
                            stream.peek().clone(),
                            anchor.clone().merge(stream.last_span.clone())
                        );
                    }
                    skip_until_or_over!(stream, kw!(Token::Semi), Token::Semi);
                    cfg
                } else {
                    true
                };

                let mut line_diagnostics = Diagnostics::new();
                let line = Line::parse(stream, &mut line_diagnostics);

                if line == Line::Malformed
                    && matches!(
                        *stream.peek().as_ref(),
                        Token::Keyword("begin_def")
                            | Token::Keyword("public")
                            | Token::Keyword("extern_def")
                            | Token::Keyword("link_attr")
                    )
                {
                    unclosed_block!(
                        diagnostics,
                        [Token::Keyword("end_def")],
                        stream.peekn(-1).clone(),
                        anchor
                    );
                    return None;
                }

                diagnostics.warns.append(&mut line_diagnostics.warns);
                if cfg {
                    body.push(line);
                    diagnostics.errs.append(&mut line_diagnostics.errs);
                } else {
                    diagnostics.warns.extend(
                        line_diagnostics
                            .errs
                            .into_iter()
                            .map(|err| err.inner.into_warn("cfg".into()).at(err.span)),
                    );
                }
            }
            Some(body.into_spanned(body_anchor.merge(stream.last_span.clone())))
        } else {
            None
        };

        if is_local {
            stream.advance();
        }

        Some(Self {
            name,
            body,
            args,
            link_attr,
        })
    }
}
