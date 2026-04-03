use std::fmt::{Debug, Display};

use indexmap::IndexMap;

use super::lexer::{Token, TokenStream};

macro_rules! parse_list {
    (
        $stream:expr,
        $end_tok:pat,
        $sep_tok:pat,
        $item_pat:pat => $extract:expr
    ) => {{
        let mut items = Vec::new();

        loop {
            let peeked = $stream.peek();

            if matches!(*peeked, $end_tok) {
                break;
            }
            let $item_pat = peeked else {
                return Err(AstErr::BadToken(peeked.to_string()));
            };

            items.push($extract);
            $stream.advance();

            if matches!(*$stream.peek(), $sep_tok) {
                $stream.advance();
            }
        }
        $stream.advance();
        items
    }};
}

fn parse_expr(stream: &mut TokenStream, min_bp: f32) -> Result<Expr, AstErr> {
    let mut lhs = match stream.peek() {
        Token::Ident(_) | Token::Lit(_) => Expr::Val(Val::parse(stream)?),
        Token::OpenParen => {
            stream.advance();
            let lhs = parse_expr(stream, 0.)?;
            let Token::CloseParen = stream.next() else {
                return Err(AstErr::BadToken(stream.peek().to_string()));
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
            Token::EOF | Token::CloseParen | Token::Semi | Token::Comma => break,
            Token::Ident(_) | Token::Lit(_) => {
                return Err(AstErr::BadToken(stream.peek().to_string()));
            }
            tok => {
                let op = Operation::from_token(tok)?;
                let (l, r) = op.infix_power();
                if r < min_bp {
                    break;
                }
                stream.advance();
                let rhs = parse_expr(stream, l)?;
                lhs = Expr::Op(Box::new(lhs), op, Box::new(rhs));
            }
        }
    }
    Ok(lhs)
}

pub(crate) fn is_func(funcs: &IndexMap<String, Function>, ident: &str) -> bool {
    funcs.contains_key(ident) || is_builtin_func(ident)
}

pub(crate) fn is_builtin_func(ident: &str) -> bool {
    matches!(
        ident,
        "print_str" | "print" | "exit" | "goto" | "label" | "addr_of" | "asm"
    )
}

pub struct Ast {
    functions: IndexMap<String, Function>,
}

impl Ast {
    pub fn from_stream(s: &mut TokenStream) -> Self {
        let mut functions = IndexMap::new();
        loop {
            let link_attr = match s.peek() {
                Token::Keyword("link_attr") => LinkAttr::parse(s).unwrap(),
                _ => LinkAttr::default(),
            };
            match Function::parse(&functions, s, link_attr) {
                Ok(func) => _ = functions.insert(func.name.clone(), func),
                Err(AstErr::Eof) => break,
                Err(e) => panic!("err in parse: {:#?}", e),
            }
            if *s.peek() == Token::EOF {
                break;
            }
        }
        Self { functions }
    }

    pub fn funcs(&self) -> impl Iterator<Item = &Function> {
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
    fn parse(stream: &mut TokenStream) -> Result<Self, AstErr> {
        let mut zelf = Self::default();
        while let Token::Keyword("link_attr") = stream.peek() {
            stream.advance();
            match stream.peek() {
                Token::Ident("section") if let Token::Ident(sec) = stream.peekn(1) => {
                    zelf = zelf.with_section(sec.to_string());
                    stream.advance();
                    stream.advance();
                }
                Token::Ident("raw")
                    if let Token::Ident("section") = stream.peekn(1)
                        && let Token::Ident(sec) = stream.peekn(2) =>
                {
                    zelf = zelf.with_meta(LinkMeta::Raw);
                    zelf = zelf.with_section(sec.to_string());
                    stream.advance();
                    stream.advance();
                    stream.advance();
                }
                Token::Ident("vis") => {
                    match stream.peekn(1) {
                        Token::Ident("public") => zelf = zelf.into_pub(),
                        Token::Ident("private") => zelf.is_public = false,
                        tok => return Err(AstErr::BadToken(tok.to_string())),
                    }
                    stream.advance();
                    stream.advance();
                }
                Token::Ident("extern") => {
                    zelf = zelf.into_external();
                    stream.advance();
                }
                tok => return Err(AstErr::BadToken(tok.to_string())),
            }
            let _tok = stream.peek();
            let Token::Semi = _tok else {
                return Err(AstErr::BadToken(_tok.to_string()));
            };
            stream.advance();
        }

        Ok(zelf)
    }

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
pub struct Function {
    pub name: String,
    pub body: Option<Vec<Line>>,
    pub args: Vec<String>,
    pub link_attr: LinkAttr,
}

impl Function {
    fn parse(
        funcs: &IndexMap<String, Function>,
        stream: &mut TokenStream,
        mut link_attr: LinkAttr,
    ) -> Result<Self, AstErr> {
        let mut kw = stream.peek();

        let is_public = *kw == Token::Keyword("public");
        if is_public {
            stream.advance();
            kw = stream.peek();
            link_attr = link_attr.into_pub();
        }
        let has_body = match kw {
            Token::Keyword("extern_def") => false,
            Token::Keyword("begin_def") => true,
            _ => return Err(AstErr::BadToken(kw.to_string())),
        };

        if !has_body {
            link_attr = link_attr.into_external();
        }

        stream.advance();

        let ident = stream.peek();
        let Token::Ident(ident) = ident else {
            return Err(AstErr::BadToken(ident.to_string()));
        };
        let name = ident.to_string();
        stream.advance();

        let args = parse_list!(stream, Token::Semi, Token::Comma, Token::Ident(ident) => ident.to_string());

        let body = if has_body {
            let mut body = Vec::new();

            while *stream.peek() != Token::Keyword("end_def") {
                body.push(Line::parse(funcs, stream)?);
            }
            stream.advance();
            Some(body)
        } else {
            None
        };

        Ok(Self {
            name,
            body,
            args,
            link_attr,
        })
    }

    pub fn body(&self) -> Option<impl Iterator<Item = &Line>> {
        self.body.as_ref().map(|b| b.iter())
    }
}

pub enum Line {
    Expr(Expr),
    Decl(LValue, Expr),
    Call(String, Vec<Expr>),
    Cond(Expr, Box<Line>),
}

#[derive(Debug)]
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
                    return Err(AstErr::BadToken(stream.peekn(1).to_string()));
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
            tok => Err(AstErr::BadToken(tok.to_string())),
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
        }
    }

    fn from_token_as_single(token: &Token<'_>) -> Result<Self, AstErr> {
        Ok(match token {
            Token::Star => Self::Load,
            Token::Ampercent => Self::AsRef,
            Token::Not => Self::Not,
            tok => return Err(AstErr::BadToken(tok.to_string())),
        })
    }

    fn from_token(token: &Token<'_>) -> Result<Self, AstErr> {
        Ok(match token {
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
            Token::Not => Self::Not,
            Token::Gt => Self::Gt,
            Token::Lt => Self::Lt,
            Token::EqEq => Self::EqEq,
            Token::NEq => Self::NEq,
            tok => return Err(AstErr::BadToken(tok.to_string())),
        })
    }
}

#[derive(Clone, Debug)]
pub enum Val {
    Var(String),
    V(i64),
    Lit(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstErr {
    BadToken(String),
    Eof,
}

impl Line {
    fn parse(funcs: &IndexMap<String, Function>, stream: &mut TokenStream) -> Result<Self, AstErr> {
        let r = match stream.peek() {
            Token::Ident(i) => match i {
                i if is_func(funcs, i) => {
                    let i = i.to_string();
                    stream.advance();
                    let mut exprs = Vec::new();
                    while *stream.peek() != Token::Semi {
                        exprs.push(parse_expr(stream, 0.)?);
                        if let Token::Comma = stream.peek() {
                            stream.advance();
                        }
                    }
                    Ok(Self::Call(i, exprs))
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
                let _semi = stream.next();
                let Token::Semi = _semi else {
                    return Err(AstErr::BadToken(_semi.to_string()));
                };

                let then = Self::parse(funcs, stream)?;
                return Ok(Self::Cond(cond, Box::new(then)));
            }
            Token::EOF => return Err(AstErr::Eof),
            _ => Ok(Self::Expr(parse_expr(stream, 0.)?)),
        };
        let _semi = stream.next();
        let Token::Semi = _semi else {
            return Err(AstErr::BadToken(_semi.to_string()));
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
            Token::Lit(t) => Self::Lit(t.to_string()),
            tok => return Err(AstErr::BadToken(tok.to_string())),
        })
    }
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
    use super::*;

    #[test]
    fn ast() {
        let s = "
            begin_def main;
          x = 1+ 2;
          print x * (5+2);
          y = x / (3 + 2);
          x + y / 5 * 4;
          end_def
        ";
        let mut stream = TokenStream::from_str(s).unwrap();
        let ast = Ast::from_stream(&mut stream);
        assert_eq!(
            format!("{}", ast),
            "fn main() {\ndeclare x = (1 + 2);\ncall print (x * (5 + 2));\ndeclare y = (x / (3 + 2));\n(x + ((y / 5) * 4));\n};\n"
        )
    }
}
