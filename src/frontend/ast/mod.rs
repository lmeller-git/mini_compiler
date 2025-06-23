use std::fmt::{Debug, Display};

use super::lexer::{Token, TokenStream};

fn parse_expr(stream: &mut TokenStream, min_bp: f32) -> Result<Expr, AstErr> {
    let mut lhs = match stream.peek() {
        Token::Ident(_) => Expr::Val(Val::parse(stream)?),
        Token::OpenParen => {
            stream.advance();
            let lhs = parse_expr(stream, 0.)?;
            let Token::CloseParen = stream.next() else {
                return Err(AstErr::BadToken);
            };
            lhs
        }
        _ => return Err(AstErr::BadToken),
    };
    loop {
        match stream.peek() {
            Token::EOF | Token::CloseParen | Token::Semi => break,
            Token::Ident(_) => return Err(AstErr::BadToken),
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
}

pub enum Line {
    Expr(Expr),
    Decl(String, Expr),
    Call(String, Expr),
}

pub enum Expr {
    Val(Val),
    Op(Box<Expr>, Operation, Box<Expr>),
}

pub enum Operation {
    Mul,
    Sub,
    Add,
    Div,
    Mod,
}

impl Operation {
    fn infix_power(&self) -> (f32, f32) {
        match self {
            Self::Mul | Self::Div | Self::Mod => (2.1, 2.),
            Self::Sub | Self::Add => (1., 1.1),
        }
    }

    fn from_token(token: &Token<'_>) -> Result<Self, AstErr> {
        Ok(match token {
            Token::Mul => Self::Mul,
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
    V(f64),
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
                &"let" => {
                    stream.advance();
                    let Token::Ident(i) = stream.next() else {
                        return Err(AstErr::BadToken);
                    };
                    let i = i.to_string();
                    let Token::Eq = stream.next() else {
                        return Err(AstErr::BadToken);
                    };
                    Ok(Self::Decl(i, parse_expr(stream, 0.)?))
                }
                i if is_func(i) => {
                    let i = i.to_string();
                    stream.advance();
                    Ok(Self::Call(i, parse_expr(stream, 0.)?))
                }
                _ => Ok(Self::Expr(parse_expr(stream, 0.)?)),
            },
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
        let Token::Ident(t) = stream.next() else {
            return Err(AstErr::BadToken);
        };

        Ok(t.parse::<f64>()
            .map(|v| Self::V(v))
            .unwrap_or(Self::Var(t.to_string())))
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

mod tests {
    use super::*;

    #[test]
    fn ast() {
        let s = "
          let x = 1+ 2;
          print x * (5+2);
          let y = x / (3 + 2);
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
