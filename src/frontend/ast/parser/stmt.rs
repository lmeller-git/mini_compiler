use crate::{
    expect,
    frontend::{
        ast::{
            LValue, Line,
            error::{AstErr, Diagnostics, IntoSpanned},
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
                let lvalue_end = stream.last_span.clone();

                'parse_inner: {
                    match (stream.peek().as_ref(), var) {
                        (_, LValue::Malformed) => Self::Malformed,
                        (Token::Eq, var) => {
                            stream.advance();
                            let rhs_start = stream.peek().span.clone();
                            Self::Decl(
                                var.into_spanned(anchor.clone().merge(lvalue_end)),
                                parse_expr(stream, 0., diagnostics)
                                    .into_spanned(rhs_start.merge(stream.last_span.clone())),
                            )
                        }
                        (_, LValue::Variable(func)) => {
                            let mut exprs = Vec::new();

                            while *stream.peek().as_ref() != Token::Semi
                                && *stream.peek().as_ref() != Token::Colon
                            {
                                let expr_start = stream.peek().span.clone();
                                exprs.push(
                                    parse_expr(stream, 0., diagnostics)
                                        .into_spanned(expr_start.merge(stream.last_span.clone())),
                                );
                                let next = stream.peek();

                                if *next.as_ref() == Token::Comma {
                                    stream.advance();
                                } else if !matches!(*next.as_ref(), Token::Semi | Token::Colon) {
                                    unclosed_block!(
                                        diagnostics,
                                        [Token::Semi, Token::Comma, Token::Colon],
                                        next.clone(),
                                        anchor.clone().merge(stream.last_span.clone())
                                    );
                                    break 'parse_inner Self::Malformed;
                                }
                            }
                            let mut ret = None;
                            if *stream.peek().as_ref() == Token::Colon {
                                stream.advance();
                                let lvalue_start = stream.peek().span.clone();
                                let lvalue = LValue::from_tokens(stream, diagnostics);
                                if lvalue == LValue::Malformed {
                                    break 'parse_inner Self::Malformed;
                                }
                                ret.replace(
                                    lvalue
                                        .into_spanned(lvalue_start.merge(stream.last_span.clone())),
                                );
                            }

                            Self::Call(
                                func.into_spanned(anchor.clone().merge(lvalue_end)),
                                exprs,
                                ret,
                            )
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
                let cond_start = stream.peek().span.clone();
                let cond = parse_expr(stream, 0., diagnostics);
                let cond_span = cond_start.merge(stream.last_span.clone());

                expect!(stream, diagnostics, anchor, [Token::Semi], unclosed_block);

                let then_start = stream.peek().span.clone();
                let then = Self::parse(stream, diagnostics);
                // then consumed the Semi already
                return Self::Cond(
                    cond.into_spanned(cond_span),
                    Box::new(then.into_spanned(then_start.merge(stream.last_span.clone()))),
                );
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
