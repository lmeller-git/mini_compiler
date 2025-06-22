use std::fmt::Display;

use super::lexer::{Token, TokenStream};

fn parse_expr<'a>(stream: &'a mut TokenStream, min_bp: f32) -> Result<Expr<'a>, AstErr> {
    todo!()
}

fn is_func(ident: &str) -> bool {
    match ident {
        "print" => true,
        _ => false,
    }
}

pub struct Ast<'a> {
    inner: Vec<Line<'a>>,
}

impl Ast<'_> {
    pub fn from_stream(s: &mut TokenStream) -> Self {
        Self { inner: Vec::new() }
    }
}

pub enum Line<'a> {
    Expr(Expr<'a>),
    Decl(&'a str, Expr<'a>),
    Call(&'a str, Expr<'a>),
}

pub enum Expr<'a> {
    Val(Val<'a>),
    Op(Box<Expr<'a>>, Operation, Box<Expr<'a>>),
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
            Self::Mul | Self::Div | Self::Mod => (2., 2.1),
            Self::Sub | Self::Add => (1., 1.1),
        }
    }
}

pub enum Val<'a> {
    Var(&'a str),
    V(f64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstErr {
    BadToken,
    Eof,
}

impl<'a> Line<'a> {
    fn parse(stream: &'a mut TokenStream) -> Result<Self, AstErr> {
        todo!()
    }
}

impl Display for Val<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Var(i) => write!(f, "{}", i),
            Self::V(v) => write!(f, "{}", v),
        }
    }
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mul => write!(f, "*"),
            Self::Sub => write!(f, "-"),
            Self::Add => write!(f, "+"),
            Self::Div => write!(f, "/"),
            Self::Mod => write!(f, "%"),
        }
    }
}

impl Display for Expr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Val(v) => write!(f, "{}", v),
            Self::Op(lhs, op, rhs) => write!(f, "({} {} {})", lhs.as_ref(), op, rhs.as_ref()),
        }
    }
}

impl Display for Line<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expr(e) => write!(f, "{}", e),
            Self::Decl(i, e) => write!(f, "declare {} = {}", i, e),
            Self::Call(i, e) => write!(f, "call {} {}", i, e),
        }
    }
}

impl Display for Ast<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for e in &self.inner {
            writeln!(f, "{};", e)?;
        }
        Ok(())
    }
}
