use indexmap::IndexMap;

use crate::{
    expect_token,
    frontend::{
        ast::{
            Item, LValue, Line,
            error::{AstErr, Diagnostics, SpannedErr},
            is_func,
            parser::expr::parse_expr,
        },
        lexer::{Token, TokenStream},
    },
    skip_until, skip_until_kw,
};

impl LValue {
    fn from_tokens<'a>(stream: &mut TokenStream<'a>) -> Result<Self, SpannedErr<'a>> {
        // TODO this should be refactored along with Line::expr/func_cal/decl
        match stream.peek().as_ref() {
            Token::Ident(i) => {
                let Token::Eq = stream.peekn(1).as_ref() else {
                    return Err(AstErr::UnexpectedToken {
                        expected: vec![],
                        found: stream.peek().clone(),
                    }
                    .at(stream.last_span.clone()));
                };
                let ident = i.to_string();
                stream.advance();
                Ok(Self::Variable(ident))
            }
            Token::Star => {
                stream.advance();
                let inner = Self::from_tokens(stream)?;
                Ok(Self::Deref(Box::new(inner)))
            }
            _tok => Err(AstErr::UnexpectedToken {
                expected: vec![],
                found: stream.peek().clone(),
            }
            .at(stream.last_span.clone())),
        }
    }
}

impl Line {
    pub fn parse<'a>(
        funcs: &IndexMap<String, Item>,
        stream: &mut TokenStream<'a>,
        diagnostics: &mut Diagnostics<'a>,
    ) -> Self {
        let anchor = stream.peek().span.clone();
        let r = match stream.peek().as_ref() {
            Token::Ident(i) => match i {
                i if is_func(funcs, i) => {
                    let i = i.to_string();
                    stream.advance();
                    let mut exprs = Vec::new();
                    while *stream.peek().as_ref() != Token::Semi {
                        exprs.push(parse_expr(stream, 0., diagnostics));
                        let next = stream.peek();
                        if *next.as_ref() == Token::Comma {
                            stream.advance();
                        } else if *next.as_ref() != Token::Semi {
                            diagnostics.errs.push(
                                AstErr::UnclosedBlock {
                                    at: next.clone(),
                                    expected: vec![Token::Semi, Token::Comma],
                                }
                                .at(anchor.clone().merge(stream.last_span.clone())),
                            );
                            skip_until!(stream, Token::Comma | Token::Semi);
                            if *stream.peek().as_ref() == Token::Comma {
                                stream.advance();
                            }
                        }
                    }
                    Self::Call(i, exprs)
                }
                _i => {
                    let name = _i.to_string();
                    if let Ok(l) = LValue::from_tokens(stream) {
                        stream.advance();
                        Self::Decl(l, parse_expr(stream, 0., diagnostics))
                    } else {
                        diagnostics.errs.push(
                            AstErr::UndefinedFunctionCall { name }.at(stream.peek().span.clone()),
                        );
                        skip_until_kw!(stream, Token::Semi);
                        if *stream.peek().as_ref() == Token::Semi {
                            stream.advance();
                        }
                        return Self::Malformed;
                    }
                }
            },
            Token::Star => {
                if let Ok(l) = LValue::from_tokens(stream) {
                    stream.advance();
                    Self::Decl(l, parse_expr(stream, 0., diagnostics))
                } else {
                    Self::Expr(parse_expr(stream, 0., diagnostics))
                }
            }
            Token::Keyword(kw) if matches!(kw, &"if") => {
                stream.advance();
                let cond = parse_expr(stream, 0., diagnostics);

                expect_token!(stream, diagnostics, anchor, [Token::Semi]);

                let then = Self::parse(funcs, stream, diagnostics);
                return Self::Cond(cond, Box::new(then));
            }
            Token::EOF => {
                diagnostics.errs.push(
                    AstErr::UnexecpectedEOF.at(anchor.clone().merge(stream.peek().span.clone())),
                );
                return Self::Malformed;
            }
            _ => Self::Expr(parse_expr(stream, 0., diagnostics)),
        };
        let tok = stream.peek();
        if *tok.as_ref() != Token::Semi {
            diagnostics.errs.push(
                AstErr::UnclosedBlock {
                    at: tok.clone(),
                    expected: vec![Token::Semi],
                }
                .at(anchor.merge(stream.last_span.clone())),
            );
            skip_until_kw!(stream, Token::Semi);
        }
        if *stream.peek().as_ref() == Token::Semi {
            stream.advance();
        }
        r
    }
}
