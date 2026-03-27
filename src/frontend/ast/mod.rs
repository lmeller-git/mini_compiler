use std::fmt::{Debug, Display};

use super::lexer::{Token, TokenStream};

fn parse_expr(stream: &mut TokenStream, min_bp: f32) -> Result<Expr, AstErr> {
    let mut lhs = match stream.peek() {
        Token::Ident(_) | Token::Lit(_) => Expr::Val(Val::parse(stream)?),
        Token::OpenParen => {
            stream.advance();
            let lhs = parse_expr(stream, 0.)?;
            let Token::CloseParen = stream.next() else {
                return Err(AstErr::BadToken);
            };
            lhs
        }
        tok => {
            let op = Operation::from_token_as_single(tok)?;
            stream.advance();
            let rhs = parse_expr(stream, op.infix_power().0)?;
            Expr::Op(Box::new(Expr::Val(Val::V(0))), op, Box::new(rhs))
        }
    };
    loop {
        match stream.peek() {
            Token::EOF | Token::CloseParen | Token::Semi => break,
            Token::Ident(_) | Token::Lit(_) => return Err(AstErr::BadToken),
            tok => {
                let op = Operation::from_token(tok)?;
                let (r, l) = op.infix_power();
                if l < min_bp {
                    break;
                }
                stream.advance();
                let rhs = parse_expr(stream, r)?;
                lhs = Expr::Op(Box::new(lhs), op, Box::new(rhs));
            }
        }
    }
    Ok(lhs)
}

fn is_func(ident: &str) -> bool {
    match ident {
        "print" => true,
        "print_str" => true,
        "label" => true,
        "goto" => true,
        _ => false,
    }
}

pub struct Ast {
    inner: Vec<Line>,
}

impl Ast {
    pub fn from_stream(s: &mut TokenStream) -> Self {
        let mut inner = Vec::new();
        loop {
            match Line::parse(s) {
                Ok(l) => inner.push(l),
                Err(AstErr::Eof) => break,
                Err(e) => panic!("err in parse: {:#?}", e),
            }
        }
        Self { inner }
    }

    pub fn lines(&self) -> impl Iterator<Item = &Line> {
        self.inner.iter()
    }
}

pub enum Line {
    Expr(Expr),
    Decl(LValue, Expr),
    Call(String, Expr),
    Cond(Expr, Box<Line>),
}

pub enum Expr {
    Val(Val),
    Op(Box<Expr>, Operation, Box<Expr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LValue {
    Variable(String),
    Deref(Box<LValue>),
}

impl Display for LValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Variable(v) => write!(f, "{v}"),
            Self::Deref(val) => write!(f, "*{}", *val),
        }
    }
}

impl LValue {
    fn from_tokens(stream: &mut TokenStream) -> Result<Self, AstErr> {
        match stream.peek() {
            Token::Ident(i) => {
                let Token::Eq = stream.peekn(1) else {
                    return Err(AstErr::BadToken);
                };
                let ident = i.to_string();
                _ = stream.advance();
                Ok(Self::Variable(ident))
            }
            Token::Star => {
                _ = stream.advance();
                let inner = Self::from_tokens(stream)?;
                Ok(Self::Deref(Box::new(inner)))
            }
            _ => Err(AstErr::BadToken),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Operation {
    Mul,
    Sub,
    Add,
    Div,
    Mod,
    Load,
    AsRef,
}

impl Operation {
    fn infix_power(&self) -> (f32, f32) {
        match self {
            Self::Load | Self::AsRef => (3., 3.1),
            Self::Mul | Self::Div | Self::Mod => (2.1, 2.),
            Self::Sub | Self::Add => (1., 1.1),
        }
    }

    fn from_token_as_single(token: &Token<'_>) -> Result<Self, AstErr> {
        Ok(match token {
            Token::Star => Self::Load,
            Token::Ampercent => Self::AsRef,
            _ => return Err(AstErr::BadToken),
        })
    }

    fn from_token(token: &Token<'_>) -> Result<Self, AstErr> {
        Ok(match token {
            Token::Star => Self::Mul,
            Token::Add => Self::Add,
            Token::Sub => Self::Sub,
            Token::Div => Self::Div,
            Token::Mod => Self::Mod,
            _ => return Err(AstErr::BadToken),
        })
    }
}

pub enum Val {
    Var(String),
    V(i64),
    Lit(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstErr {
    BadToken,
    Eof,
}

impl Line {
    fn parse(stream: &mut TokenStream) -> Result<Self, AstErr> {
        let r = match stream.peek() {
            Token::Ident(i) => match i {
                i if is_func(i) => {
                    let i = i.to_string();
                    stream.advance();
                    Ok(Self::Call(i, parse_expr(stream, 0.)?))
                }
                _i => {
                    if let Ok(l) = LValue::from_tokens(stream) {
                        stream.advance();
                        Ok(Self::Decl(l, parse_expr(stream, 0.)?))
                    } else {
                        Ok(Self::Expr(parse_expr(stream, 0.)?))
                    }
                }
            },
            Token::Star => {
                if let Ok(l) = LValue::from_tokens(stream) {
                    stream.advance();
                    Ok(Self::Decl(l, parse_expr(stream, 0.)?))
                } else {
                    Ok(Self::Expr(parse_expr(stream, 0.)?))
                }
            }
            Token::Keyword(kw) if matches!(kw, &"if") => {
                stream.advance();
                let cond = parse_expr(stream, 0.)?;
                let Token::Semi = stream.next() else {
                    return Err(AstErr::BadToken);
                };

                let then = Self::parse(stream)?;
                return Ok(Self::Cond(cond, Box::new(then)));
            }
            Token::EOF => return Err(AstErr::Eof),
            _ => Ok(Self::Expr(parse_expr(stream, 0.)?)),
        };
        let Token::Semi = stream.next() else {
            return Err(AstErr::BadToken);
        };
        r
    }
}

impl Val {
    fn parse(stream: &mut TokenStream) -> Result<Self, AstErr> {
        Ok(match stream.next() {
            Token::Ident(t) => t
                .parse::<i64>()
                .map(Self::V)
                .unwrap_or(Self::Var(t.to_string())),
            Token::Lit(t) => t
                .parse::<i64>()
                .map(Self::V)
                .unwrap_or(Self::Lit(t.to_string())),
            _ => return Err(AstErr::BadToken),
        })
    }
}

impl Debug for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for Val {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Var(i) => write!(f, "{}", i),
            Self::V(v) => write!(f, "{}", v),
            Self::Lit(lit) => write!(f, "{}", lit),
        }
    }
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Mul => write!(f, "*"),
            Self::Sub => write!(f, "-"),
            Self::Add => write!(f, "+"),
            Self::Div => write!(f, "/"),
            Self::Mod => write!(f, "%"),
            Self::Load => write!(f, "*"),
            Self::AsRef => write!(f, "&"),
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Val(v) => write!(f, "{}", v),
            Self::Op(lhs, op, rhs) => write!(f, "({} {} {})", lhs.as_ref(), op, rhs.as_ref()),
        }
    }
}

impl Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Expr(e) => write!(f, "{}", e),
            Self::Decl(i, e) => write!(f, "declare {} = {}", i, e),
            Self::Call(i, e) => write!(f, "call {} {}", i, e),
            Self::Cond(c, e) => write!(f, "if {}; {};", c, e),
        }
    }
}

impl Display for Ast {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for e in &self.inner {
            writeln!(f, "{};", e)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ast() {
        let s = "
          x = 1+ 2;
          print x * (5+2);
          y = x / (3 + 2);
          x + y / 5 * 4;
        ";
        let mut stream = TokenStream::from_str(s).unwrap();
        let ast = Ast::from_stream(&mut stream);
        assert_eq!(
            format!("{}", ast),
            "declare x = (1 + 2);\ncall print (x * (5 + 2));\ndeclare y = (x / (3 + 2));\n(x + ((y / 5) * 4));\n"
        )
    }
}
