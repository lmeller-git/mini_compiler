use std::fmt::{Debug, Display};

use indexmap::IndexMap;

pub mod cfg;
pub mod error;
pub mod parser;

pub(crate) fn is_builtin_func(ident: &str) -> bool {
    matches!(
        ident,
        "print_str" | "print" | "exit" | "goto" | "label" | "addr_of" | "asm"
    )
}

pub struct Ast {
    functions: IndexMap<String, Item>,
}

impl Ast {
    pub fn funcs(&self) -> impl Iterator<Item = &Item> {
        self.functions.values()
    }
}

#[derive(Debug, Clone)]
pub struct LinkAttr {
    pub section: String,
    pub external: bool,
    pub is_public: bool,
    pub meta: LinkMeta,
}

impl LinkAttr {
    fn into_external(mut self) -> Self {
        self.external = true;
        self
    }

    fn into_pub(mut self) -> Self {
        self.is_public = true;
        self
    }

    fn with_section(mut self, section: String) -> Self {
        self.section = section;
        self
    }

    fn with_meta(mut self, meta: LinkMeta) -> Self {
        self.meta = meta;
        self
    }
}

impl Default for LinkAttr {
    fn default() -> Self {
        Self {
            section: ".text".into(),
            external: false,
            is_public: false,
            meta: LinkMeta::default(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub enum LinkMeta {
    Raw,
    #[default]
    WithMeta,
}

#[derive(Debug)]
pub enum Item {
    Function(Function),
    Malformed,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub body: Option<Vec<Line>>,
    pub args: Vec<String>,
    pub link_attr: LinkAttr,
}

impl Function {
    pub fn body(&self) -> Option<impl Iterator<Item = &Line>> {
        self.body.as_ref().map(|b| b.iter())
    }
}

#[derive(PartialEq, Eq)]
pub enum Line {
    Expr(Expr),
    Decl(LValue, Expr),
    Call(String, Vec<Expr>),
    Cond(Expr, Box<Line>),
    Malformed,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Expr {
    Val(Val),
    Op(Box<Expr>, Operation, Box<Expr>),
    Malformed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LValue {
    Variable(String),
    Deref(Box<LValue>),
    Malformed,
}

impl Display for LValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Variable(v) => write!(f, "{v}"),
            Self::Deref(val) => write!(f, "*{}", *val),
            Self::Malformed => write!(f, "malformed"),
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
    Not,
    Gt,
    Lt,
    EqEq,
    NEq,
    BitAND,
    BitOR,
    BitXOR,
    Shr,
    Shl,
    Malformed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Val {
    Var(String),
    V(i64),
    Lit(String),
    Malformed,
}

impl Debug for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fn {}(", self.name)?;
        writeln!(f, "{}) {{", self.args.join(","))?;
        if let Some(body) = self.body() {
            for line in body {
                writeln!(f, "{};", line)?;
            }
        }
        write!(f, "}}")
    }
}

impl Display for Val {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Var(i) => write!(f, "{}", i),
            Self::V(v) => write!(f, "{}", v),
            Self::Lit(lit) => write!(f, "{}", lit),
            Self::Malformed => write!(f, "malformed"),
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
            Self::Not => write!(f, "!"),
            Self::Gt => write!(f, ">"),
            Self::Lt => write!(f, "<"),
            Self::EqEq => write!(f, "=="),
            Self::BitAND => write!(f, "&"),
            Self::BitOR => write!(f, "|"),
            Self::BitXOR => write!(f, "^"),
            Self::Shr => write!(f, ">>"),
            Self::Shl => write!(f, "<<"),
            Self::NEq => write!(f, "!="),
            Self::Malformed => write!(f, "malformed"),
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Val(v) => write!(f, "{}", v),
            Self::Op(lhs, op, rhs) => write!(f, "({} {} {})", lhs.as_ref(), op, rhs.as_ref()),
            Self::Malformed => write!(f, "malformed"),
        }
    }
}

impl Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Expr(e) => write!(f, "{}", e),
            Self::Decl(i, e) => write!(f, "declare {} = {}", i, e),
            Self::Call(i, e) => write!(
                f,
                "call {} {}",
                i,
                e.iter()
                    .map(|ele| ele.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::Cond(c, e) => write!(f, "if {}; {}", c, e),
            Self::Malformed => write!(f, "malformed"),
        }
    }
}

impl Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Function(func) => write!(f, "{}", func),
            Self::Malformed => write!(f, "malformed"),
        }
    }
}

impl Display for Ast {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for e in self.funcs() {
            writeln!(f, "{};", e)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::frontend::{ast::cfg::CfgEnv, lexer::TokenStream};

    use super::*;

    #[test]
    fn ast() {
        let s = "
            begin_def main;
          x = 1+ 2;
          print x * (5+2);
          y = x / (3 + 2);
          k = x + y / 5 * 4;
          end_def
        ";
        let mut stream = TokenStream::from_str(s).unwrap();
        let ast = Ast::from_stream(&mut stream, &CfgEnv::default());
        assert_eq!(
            format!("{}", ast.0),
            "fn main() {\ndeclare x = (1 + 2);\ncall print (x * (5 + 2));\ndeclare y = (x / (3 + 2));\ndeclare k = (x + ((y / 5) * 4));\n};\n"
        )
    }
}
