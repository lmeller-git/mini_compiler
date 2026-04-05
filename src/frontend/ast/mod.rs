use std::fmt::{Debug, Display};

use ariadne::{Label, Report, Source};
use indexmap::IndexMap;

use crate::frontend::{
    cfg::CfgEnv,
    lexer::{Span, Spanned},
};

use super::lexer::{Token, TokenStream};

macro_rules! parse_list {
    (
        $stream:expr,
        $end_tok:pat,
        $sep_tok:pat,
        $item_pat:pat => $extract:expr
    ) => {{
        let mut items = Vec::new();

        if let Some(e) = loop {
            let peeked = $stream.peek().as_ref();

            if matches!(*peeked, $end_tok) {
                break None;
            }
            let $item_pat = peeked else {
                break Some(
                    AstErr::UnexpectedToken {
                        expected: vec![],
                        found: $stream.peek().clone(),
                    }
                    .at($stream.last_span.clone()),
                );
            };

            items.push($extract);
            $stream.advance();

            if matches!(*$stream.peek().as_ref(), $sep_tok) {
                $stream.advance();
            }
        } {
            Err(e)
        } else {
            $stream.advance();
            Ok(items)
        }
    }};
}

macro_rules! expect_token {
    (
        $stream:expr,
        $diagnostics:expr,
        $expected:pat
    ) => {
        let _tok = $stream.peek();
        if !matches!(*_tok.as_ref(), $expected) {
            $diagnostics.errs.push(
                AstErr::UnclosedBlock {
                    at: _tok.clone(),
                    expected: Token::Semi,
                }
                .at($stream.last_span.clone()),
            )
        } else {
            $stream.advance();
        }
    };
}

macro_rules! skip_until {
    (
        $stream:expr,
        $stop_at:pat
    ) => {
        while !matches!(*$stream.peek().as_ref(), $stop_at | Token::EOF) {
            $stream.advance();
        }
    };
}

fn parse_expr<'a>(
    stream: &mut TokenStream<'a>,
    min_bp: f32,
    diagnostics: &mut Diagnostics<'a>,
) -> Expr {
    let mut lhs = match stream.peek().as_ref() {
        Token::Ident(_) | Token::Lit(_) => Expr::Val(Val::parse(stream, diagnostics)),
        Token::Semi | Token::Comma | Token::EOF | Token::CloseParen => {
            diagnostics.errs.push(
                AstErr::UnexpectedToken {
                    expected: vec![
                        Token::Star,
                        Token::Ampercent,
                        Token::Not,
                        Token::OpenParen,
                        Token::Ident(""),
                        Token::Lit(""),
                    ],
                    found: stream.peek().clone(),
                }
                .at(stream.peek().span.clone()),
            );
            return Expr::Malformed;
        }
        Token::OpenParen => {
            stream.advance();
            let lhs = parse_expr(stream, 0., diagnostics);
            if let Token::CloseParen = stream.peek().as_ref() {
                stream.advance();
            } else {
                diagnostics.errs.push(
                    AstErr::UnclosedBlock {
                        at: stream.peek().clone(),
                        expected: Token::CloseParen,
                    }
                    .at(stream.last_span.clone()),
                );
            }
            lhs
        }
        _tok => {
            let op = Operation::from_token_as_single(stream.peek(), diagnostics);
            stream.advance();
            let rhs = parse_expr(stream, op.infix_power().0, diagnostics);
            Expr::Op(Box::new(Expr::Val(Val::V(0))), op, Box::new(rhs))
        }
    };
    while let Some(op) = Operation::try_from_token(stream.peek()) {
        let (l, r) = op.infix_power();
        if r < min_bp {
            break;
        }
        stream.advance();
        let rhs = parse_expr(stream, l, diagnostics);
        lhs = Expr::Op(Box::new(lhs), op, Box::new(rhs));
    }
    lhs
}

pub(crate) fn is_func(funcs: &IndexMap<String, Item>, ident: &str) -> bool {
    funcs
        .get(ident)
        .is_some_and(|item| matches!(item, Item::Function(_)))
        || is_builtin_func(ident)
}

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
    pub fn from_stream<'a>(s: &mut TokenStream<'a>, cfg_env: &CfgEnv) -> (Self, Diagnostics<'a>) {
        let mut diagnostics = Diagnostics::new();
        let mut functions = IndexMap::new();
        loop {
            match s.peek().as_ref() {
                Token::EOF => break,
                Token::Keyword("cfg") => {
                    s.advance();
                    let cfg = cfg_env.eval_cfg_expr(&parse_expr(s, 0., &mut diagnostics));

                    let _semi = s.peek();
                    if *_semi.as_ref() != Token::Semi {
                        diagnostics.errs.push(
                            AstErr::UnclosedBlock {
                                at: s.peek().clone(),
                                expected: Token::Semi,
                            }
                            .at(s.last_span.clone()),
                        );
                        skip_until!(s, Token::Semi);
                    }
                    s.advance();

                    if !cfg {
                        skip_until!(
                            s,
                            Token::Keyword("begin_def")
                                | Token::Keyword("extern_def")
                                | Token::Keyword("public")
                        );
                        match s.peek().as_ref() {
                            Token::Keyword("begin_def") | Token::Keyword("public") => {
                                skip_until!(s, Token::Keyword("end_def"))
                            }
                            Token::Keyword("extern_def") => skip_until!(s, Token::Semi),
                            Token::EOF => {
                                diagnostics
                                    .errs
                                    .push(AstErr::UnexecpectedEOF.at(s.last_span.clone()));
                            }
                            _ => unreachable!(),
                        }
                        s.advance();
                        continue;
                    }
                }
                Token::Keyword(kw)
                    if matches!(*kw, "link_attr" | "begin_def" | "public" | "extern_def") =>
                {
                    let link_attr = LinkAttr::parse(s, &mut diagnostics);

                    if let Some(f) =
                        Function::parse(&functions, s, link_attr, cfg_env, &mut diagnostics)
                    {
                        functions.insert(f.name.clone(), Item::Function(f));
                    } else {
                        functions.insert(
                            format!("<__malformed_{}>", diagnostics.errs.len()),
                            Item::Malformed,
                        );
                    }
                }
                _invalid => {
                    diagnostics.errs.push(
                        AstErr::UnexpectedToken {
                            expected: vec![
                                Token::Keyword("cfg"),
                                Token::Keyword("begin_def"),
                                Token::Keyword("public"),
                                Token::Keyword("link_attr"),
                            ],
                            found: s.peek().clone(),
                        }
                        .at(s.peek().span.clone()),
                    );
                    skip_until!(
                        s,
                        Token::Keyword("begin_def")
                            | Token::Keyword("public")
                            | Token::Keyword("cfg")
                            | Token::Keyword("link_attr")
                    )
                }
            }
        }
        (Self { functions }, diagnostics)
    }

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
    fn parse<'a>(stream: &mut TokenStream<'a>, diagnostics: &mut Diagnostics<'a>) -> Self {
        let mut zelf = Self::default();
        while let Token::Keyword("link_attr") = stream.peek().as_ref() {
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
                            diagnostics.errs.push(SpannedErr {
                                err: AstErr::UnexpectedToken {
                                    expected: vec![
                                        Token::Keyword("public"),
                                        Token::Keyword("private"),
                                    ],
                                    found: stream.peekn(1).clone(),
                                },
                                span: stream.last_span.clone(),
                            });
                            skip_until!(stream, Token::Semi);
                            stream.advance();
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
                    diagnostics.errs.push(SpannedErr {
                        err: AstErr::UnexpectedToken {
                            expected: vec![
                                Token::Keyword("section"),
                                Token::Keyword("raw"),
                                Token::Keyword("vis"),
                            ],
                            found: stream.peek().clone(),
                        },
                        span: stream.last_span.clone(),
                    });
                    skip_until!(stream, Token::Semi);
                    stream.advance();
                    continue;
                }
            }

            let _tok = stream.peek();
            if *_tok.as_ref() != Token::Semi {
                diagnostics.errs.push(SpannedErr {
                    err: AstErr::UnclosedBlock {
                        at: _tok.clone(),
                        expected: Token::Semi,
                    },
                    span: stream.last_span.clone(),
                });
                skip_until!(
                    stream,
                    Token::Keyword("begin_def") | Token::Keyword("extern_def")
                );
            } else {
                stream.advance();
            }
        }

        zelf
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
    fn parse<'a>(
        funcs: &IndexMap<String, Item>,
        stream: &mut TokenStream<'a>,
        mut link_attr: LinkAttr,
        cfg_env: &CfgEnv,
        diagnostics: &mut Diagnostics<'a>,
    ) -> Option<Self> {
        let mut kw = stream.peek();

        let is_public = *kw.as_ref() == Token::Keyword("public");
        if is_public {
            stream.advance();
            kw = stream.peek();
            link_attr = link_attr.into_pub();
        }
        let has_body = match kw.as_ref() {
            Token::Keyword("extern_def") => false,
            Token::Keyword("begin_def") => true,
            _tok => {
                diagnostics.errs.push(
                    AstErr::UnexpectedToken {
                        expected: vec![Token::Keyword("extern_def"), Token::Keyword("begin_def")],
                        found: kw.clone(),
                    }
                    .at(stream.last_span.clone()),
                );
                return None;
            }
        };

        if !has_body {
            link_attr = link_attr.into_external();
        }

        stream.advance();

        let ident = stream.peek();
        let Token::Ident(_ident) = ident.as_ref() else {
            diagnostics.errs.push(
                AstErr::UnexpectedToken {
                    expected: vec![Token::Ident("")],
                    found: ident.clone(),
                }
                .at(stream.last_span.clone()),
            );
            return None;
        };
        let name = _ident.to_string();
        stream.advance();

        let args = parse_list!(stream, Token::Semi, Token::Comma, Token::Ident(ident) => ident.to_string());

        let args = match args {
            Ok(a) => a,
            Err(e) => {
                diagnostics.errs.push(e);
                Vec::new()
            }
        };

        let body = if has_body {
            let mut body = Vec::new();

            while *stream.peek().as_ref() != Token::Keyword("end_def") {
                if let Token::Keyword("cfg") = stream.peek().as_ref() {
                    stream.advance();
                    let cfg = cfg_env.eval_cfg_expr(&parse_expr(stream, 0., diagnostics));
                    stream.advance();
                    if !cfg {
                        loop {
                            let tok_is_if = *stream.peek().as_ref() == Token::Keyword("if");
                            skip_until!(stream, Token::Semi);
                            stream.advance();
                            if !tok_is_if {
                                break;
                            }
                        }
                        continue;
                    }
                }
                let line = Line::parse(funcs, stream, diagnostics);
                body.push(line);
                if *stream.peek().as_ref() == Token::EOF {
                    diagnostics.errs.push(
                        AstErr::UnclosedBlock {
                            at: stream.peek().clone(),
                            expected: Token::Keyword("end_def"),
                        }
                        .at(stream.peek().span.clone()),
                    );
                    break;
                }
            }
            stream.advance();
            Some(body)
        } else {
            None
        };

        Some(Self {
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
    Malformed,
}

#[derive(Debug)]
pub enum Expr {
    Val(Val),
    Op(Box<Expr>, Operation, Box<Expr>),
    Malformed,
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

    fn from_token_as_single<'a>(token: &Spanned<'a>, diagnostics: &mut Diagnostics<'a>) -> Self {
        match token.as_ref() {
            Token::Star => Self::Load,
            Token::Ampercent => Self::AsRef,
            Token::Not => Self::Not,
            _tok => {
                diagnostics.errs.push(
                    AstErr::UnexpectedToken {
                        expected: vec![Token::Star, Token::Ampercent, Token::Not],
                        found: token.clone(),
                    }
                    .at(token.span.clone()),
                );
                Self::Malformed
            }
        }
    }

    fn try_from_token<'a>(token: &Spanned<'a>) -> Option<Self> {
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
            Token::Not => Self::Not,
            Token::Gt => Self::Gt,
            Token::Lt => Self::Lt,
            Token::EqEq => Self::EqEq,
            Token::NEq => Self::NEq,
            _tok => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub enum Val {
    Var(String),
    V(i64),
    Lit(String),
    Malformed,
}

#[derive(Debug)]
pub struct Diagnostics<'a> {
    pub errs: Vec<SpannedErr<'a>>,
}

impl<'a> Diagnostics<'a> {
    fn new() -> Self {
        Self { errs: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpannedErr<'a> {
    pub err: AstErr<'a>,
    pub span: Span,
}

impl<'a> SpannedErr<'a> {
    pub fn report(&self, file: &str, source: &str) {
        let mut report = Report::build(
            ariadne::ReportKind::Error,
            (file, self.span.start..self.span.end),
        );

        report = match &self.err {
            AstErr::UnexecpectedEOF => report.with_message("unexpected EOF"),
            AstErr::UnclosedBlock { at, expected } => report
                .with_message("unclosed code block")
                .with_label(
                    Label::new((file, at.span.start..at.span.end))
                        .with_message(format!("expected {}", expected)),
                )
                .with_label(
                    Label::new((file, self.span.start..self.span.end))
                        .with_message("while parsing this block"),
                ),
            AstErr::UnexpectedToken { expected, found } => report
                .with_message("unexpected token")
                .with_label(
                    Label::new((file, found.span.start..found.span.end)).with_message(format!(
                        "expected one of {:?}, found {}",
                        expected, found.token
                    )),
                )
                .with_label(
                    Label::new((file, self.span.start..self.span.end))
                        .with_message("while parsing this block"),
                ),
            AstErr::UndefinedFunctionCall { name } => report
                .with_message(format!("tried to call undefined function {}", name))
                .with_label(Label::new((file, self.span.start..self.span.end)))
                .with_help("functions must be defined above the call site"),
        };
        report.finish().print((file, Source::from(source))).unwrap();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstErr<'a> {
    UnexecpectedEOF,
    UnclosedBlock {
        at: Spanned<'a>,
        expected: Token<'a>,
    },
    UnexpectedToken {
        expected: Vec<Token<'a>>,
        found: Spanned<'a>,
    },
    UndefinedFunctionCall {
        name: String,
    },
}

impl<'a> AstErr<'a> {}

impl<'a> AstErr<'a> {
    fn at(self, span: Span) -> SpannedErr<'a> {
        SpannedErr { err: self, span }
    }
}

impl Line {
    fn parse<'a>(
        funcs: &IndexMap<String, Item>,
        stream: &mut TokenStream<'a>,
        diagnostics: &mut Diagnostics<'a>,
    ) -> Self {
        let r = match stream.peek().as_ref() {
            Token::Ident(i) => match i {
                i if is_func(funcs, i) => {
                    let i = i.to_string();
                    let anchor = stream.peek().span.clone();
                    stream.advance();
                    let mut exprs = Vec::new();
                    while *stream.peek().as_ref() != Token::Semi {
                        exprs.push(parse_expr(stream, 0., diagnostics));
                        let next = stream.peek();
                        if *next.as_ref() == Token::Comma {
                            stream.advance();
                        } else if *next.as_ref() != Token::Semi {
                            println!("{:?}", anchor);
                            diagnostics.errs.push(
                                AstErr::UnclosedBlock {
                                    at: next.clone(),
                                    expected: Token::Semi,
                                }
                                .at(anchor.clone()),
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
                        skip_until!(stream, Token::Semi);
                        Self::Malformed
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

                expect_token!(stream, diagnostics, Token::Semi);

                let then = Self::parse(funcs, stream, diagnostics);
                return Self::Cond(cond, Box::new(then));
            }
            Token::EOF => {
                diagnostics
                    .errs
                    .push(AstErr::UnexecpectedEOF.at(stream.peek().span.clone()));
                return Self::Malformed;
            }
            _ => Self::Expr(parse_expr(stream, 0., diagnostics)),
        };
        let tok = stream.peek();
        if *tok.as_ref() != Token::Semi {
            diagnostics.errs.push(
                AstErr::UnclosedBlock {
                    at: tok.clone(),
                    expected: Token::Semi,
                }
                .at(stream.last_span.clone()),
            );
            skip_until!(stream, Token::Semi);
        }
        stream.advance();
        r
    }
}

impl Val {
    fn parse<'a>(stream: &mut TokenStream<'a>, diagnostics: &mut Diagnostics<'a>) -> Self {
        let zelf = match stream.peek().as_ref() {
            Token::Ident(t) => t
                .parse::<i64>()
                .map(Self::V)
                .unwrap_or(Self::Var(t.to_string())),
            Token::Lit(t) => Self::Lit(t.to_string()),
            _tok => {
                diagnostics.errs.push(
                    AstErr::UnexpectedToken {
                        expected: vec![Token::Ident(""), Token::Lit("")],
                        found: stream.peek().clone(),
                    }
                    .at(stream.peek().span.clone()),
                );
                return Self::Malformed;
            }
        };
        stream.advance();
        zelf
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
