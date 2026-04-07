use crate::{
    frontend::{
        ast::{
            Expr, Operation, Val,
            error::{AstErr, Diagnostics, Spanned},
        },
        lexer::{Token, TokenStream},
    },
    unclosed_block, unexpected,
};

pub fn parse_expr<'a>(
    stream: &mut TokenStream<'a>,
    min_bp: f32,
    diagnostics: &mut Diagnostics<'a>,
) -> Expr {
    let anchor = stream.peek().span.clone();
    let mut lhs = match stream.peek().as_ref() {
        Token::Ident(_) | Token::Lit(_) | Token::Number(_) => {
            Expr::Val(Val::parse(stream, diagnostics))
        }
        Token::Semi | Token::Comma | Token::EOF | Token::CloseParen => {
            unexpected!(
                diagnostics,
                [
                    Token::Star,
                    Token::Ampercent,
                    Token::Not,
                    Token::OpenParen,
                    Token::Ident("<ident>"),
                    Token::Lit("<strlit>"),
                    Token::Number(0),
                ],
                stream.peek().clone(),
                stream.last_span.clone()
            );

            return Expr::Malformed;
        }
        Token::OpenParen => {
            stream.advance();
            let lhs = parse_expr(stream, 0., diagnostics);
            if lhs == Expr::Malformed {
                return lhs;
            }
            if let Token::CloseParen = stream.peek().as_ref() {
                stream.advance();
            } else {
                unclosed_block!(
                    diagnostics,
                    [Token::CloseParen],
                    stream.peek().clone(),
                    anchor.merge(stream.last_span.clone())
                );
                return Expr::Malformed;
            }
            lhs
        }
        _tok => {
            if let Some(op) = Operation::try_from_token_as_single(stream.peek()) {
                stream.advance();
                let rhs = parse_expr(stream, op.infix_power().0, diagnostics);
                if rhs == Expr::Malformed {
                    return Expr::Malformed;
                }
                Expr::Op(Box::new(Expr::Val(Val::V(0))), op, Box::new(rhs))
            } else {
                unexpected!(
                    diagnostics,
                    [
                        Token::Star,
                        Token::Ampercent,
                        Token::Not,
                        Token::Ident(""),
                        Token::Lit(""),
                        Token::Number(0),
                    ],
                    stream.peek().clone(),
                    anchor.merge(stream.last_span.clone())
                );

                return Expr::Malformed;
            }
        }
    };

    while let Some(op) = Operation::try_from_token(stream.peek()) {
        let (l, r) = op.infix_power();
        if r < min_bp {
            break;
        }
        stream.advance();
        let rhs = parse_expr(stream, l, diagnostics);
        if rhs == Expr::Malformed {
            return Expr::Malformed;
        }
        lhs = Expr::Op(Box::new(lhs), op, Box::new(rhs));
    }
    lhs
}

impl Operation {
    fn infix_power(&self) -> (f32, f32) {
        match self {
            Self::Load | Self::AsRef => (4., 4.1),
            Self::Not => (3.5, 3.6),
            Self::Mul | Self::Div | Self::Mod => (3.1, 3.),
            Self::Sub | Self::Add => (2., 2.1),
            Self::Shr | Self::Shl => (1.8, 1.9),
            Self::BitAND => (1.6, 1.7),
            Self::BitXOR => (1.5, 1.6),
            Self::BitOR => (1.4, 1.5),
            Self::Gt | Self::Lt | Self::EqEq | Self::NEq => (1., 1.1),
            Self::Malformed => (0., 0.1),
        }
    }

    fn try_from_token_as_single<'a>(token: &Spanned<Token<'a>>) -> Option<Self> {
        Some(match token.as_ref() {
            Token::Star => Self::Load,
            Token::Ampercent => Self::AsRef,
            Token::Not => Self::Not,
            _tok => return None,
        })
    }

    fn try_from_token<'a>(token: &Spanned<Token<'a>>) -> Option<Self> {
        Some(match token.as_ref() {
            Token::Star => Self::Mul,
            Token::Add => Self::Add,
            Token::Sub => Self::Sub,
            Token::Div => Self::Div,
            Token::Mod => Self::Mod,
            Token::Ampercent => Self::BitAND,
            Token::Or => Self::BitOR,
            Token::Hat => Self::BitXOR,
            Token::Shr => Self::Shr,
            Token::Shl => Self::Shl,
            Token::Gt => Self::Gt,
            Token::Lt => Self::Lt,
            Token::EqEq => Self::EqEq,
            Token::NEq => Self::NEq,
            _tok => return None,
        })
    }
}

impl Val {
    fn parse<'a>(stream: &mut TokenStream<'a>, diagnostics: &mut Diagnostics<'a>) -> Self {
        let zelf = match stream.peek().as_ref() {
            Token::Ident(t) => Self::Var(t.to_string()),
            Token::Number(num) => Self::V(*num),
            Token::Lit(t) => Self::Lit(t.to_string()),
            _tok => {
                unexpected!(
                    diagnostics,
                    [Token::Ident("<ident>"), Token::Lit("<strlit>")],
                    stream.peek().clone(),
                    stream.last_span.clone()
                );
                return Self::Malformed;
            }
        };
        stream.advance();
        zelf
    }
}
