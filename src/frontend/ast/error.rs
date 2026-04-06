use ariadne::{Label, Report, Source};

use crate::frontend::lexer::{Span, Spanned, Token};

#[derive(Debug)]
pub struct Diagnostics<'a> {
    pub errs: Vec<SpannedErr<'a>>,
}

impl<'a> Diagnostics<'a> {
    pub fn new() -> Self {
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
                        .with_message(format!("expected one of {:?}", expected)),
                )
                .with_label(
                    Label::new((file, self.span.start..self.span.end))
                        .with_message("while parsing this block"),
                ),
            AstErr::UnexpectedToken { expected, found } => report
                .with_message("unexpected token")
                .with_label(
                    Label::new((file, found.span.start..found.span.end)).with_message(format!(
                        "expected one of {:?}, found {:?}",
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
        expected: Vec<Token<'a>>,
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
    pub fn at(self, span: Span) -> SpannedErr<'a> {
        SpannedErr { err: self, span }
    }
}
