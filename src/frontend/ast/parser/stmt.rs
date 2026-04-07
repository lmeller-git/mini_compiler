use crate::{
    expect,
    frontend::{
        ast::{
            LValue, Line,
            error::{AstErr, Diagnostics},
            parser::expr::parse_expr,
        },
        lexer::{Token, TokenStream},
    },
    kw, skip_until_or_over, unclosed_block, unexpected,
};

impl LValue {
    fn from_tokens<'a>(stream: &mut TokenStream<'a>, diagnostics: &mut Diagnostics<'a>) -> Self {
        let anchor = stream.peek().span.clone();
        match stream.peek().as_ref() {
            Token::Ident(i) => {
                let ident = i.to_string();
                stream.advance();
                Self::Variable(ident)
            }
            Token::Star => {
                stream.advance();
                let inner = Self::from_tokens(stream, diagnostics);
                Self::Deref(Box::new(inner))
            }
            _tok => {
                unexpected!(
                    diagnostics,
                    [Token::Star, Token::Ident("<variable>")],
                    stream.peek().clone(),
                    anchor.merge(stream.last_span.clone())
                );
                Self::Malformed
            }
        }
    }
}

impl Line {
    /// parses a single Line. Leaves stream after the next semi. If err: skips until next Semi/kw
    pub fn parse<'a>(stream: &mut TokenStream<'a>, diagnostics: &mut Diagnostics<'a>) -> Self {
        let anchor = stream.peek().span.clone();

        let line = match stream.peek().as_ref() {
            Token::Ident(_) | Token::Star => {
                let var = LValue::from_tokens(stream, diagnostics);

                'parse_inner: {
                    match (stream.peek().as_ref(), var) {
                        (_, LValue::Malformed) => Self::Malformed,
                        (Token::Eq, var) => {
                            stream.advance();
                            Self::Decl(var, parse_expr(stream, 0., diagnostics))
                        }
                        (_, LValue::Variable(func)) => {
                            let mut exprs = Vec::new();

                            while *stream.peek().as_ref() != Token::Semi {
                                exprs.push(parse_expr(stream, 0., diagnostics));
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
                                    break 'parse_inner Self::Malformed;
                                }
                            }
                            Self::Call(func, exprs)
                        }
                        (_, _) => {
                            unexpected!(
                                diagnostics,
                                [Token::Eq],
                                stream.peek().clone(),
                                anchor.clone().merge(stream.last_span.clone())
                            );
                            Self::Malformed
                        }
                    }
                }
            }
            Token::Keyword(kw) if matches!(kw, &"if") => {
                stream.advance();
                let cond = parse_expr(stream, 0., diagnostics);

                expect!(stream, diagnostics, anchor, [Token::Semi], unclosed_block);

                let then = Self::parse(stream, diagnostics);
                // then consumed the Semi already
                return Self::Cond(cond, Box::new(then));
            }
            Token::EOF => {
                diagnostics.errs.push(
                    AstErr::UnexecpectedEOF.at(anchor.clone().merge(stream.peek().span.clone())),
                );
                Self::Malformed
            }
            _ => {
                unexpected!(
                    diagnostics,
                    [Token::Star, Token::Ident("<ident>"), Token::Keyword("if")],
                    stream.peek().clone(),
                    anchor.clone()
                );
                Self::Malformed
            }
        };

        match (stream.peek().as_ref(), line) {
            (Token::Semi, line) => {
                stream.advance();
                line
            }
            (_, Self::Malformed) => {
                // error was already logged
                skip_until_or_over!(stream, kw!(Token::Semi), Token::Semi);
                Self::Malformed
            }
            (_, _) => {
                unclosed_block!(
                    diagnostics,
                    [Token::Semi],
                    stream.peek().clone(),
                    anchor.clone().merge(stream.last_span.clone())
                );
                skip_until_or_over!(stream, kw!(Token::Semi), Token::Semi);
                Self::Malformed
            }
        }
    }
}
