use std::ops::Range;

use ariadne::{Label, Report, ReportBuilder, ReportKind, Source};

use crate::frontend::lexer::{Span, Token};

#[macro_export]
macro_rules! unexpected {
    ($diagnostics:expr,$expected:expr, $found:expr, $block:expr) => {
        $diagnostics.errs.push(
            AstErr::UnexpectedToken {
                expected: $expected.into(),
                found: $found,
            }
            .at($block),
        )
    };
}

#[macro_export]
macro_rules! unclosed_block {
    ($diagnostics:expr, $expected:expr, $found:expr, $block:expr) => {
        $diagnostics.errs.push(
            AstErr::UnclosedBlock {
                expected: $expected.into(),
                at: $found,
            }
            .at($block),
        )
    };
}

pub trait BuildReport {
    fn report<'a, 'b>(
        &self,
        builder: ReportBuilder<'a, (&'b str, Range<usize>)>,
        file: &'a str,
    ) -> ReportBuilder<'a, (&'b str, Range<usize>)>
    where
        'a: 'b;
}

#[derive(Debug, Default)]
pub struct Diagnostics<'a> {
    pub errs: Vec<Spanned<AstErr<'a>>>,
    pub warns: Vec<Spanned<AstWarn<'a>>>,
}

impl<'a> Diagnostics<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn report(&self, file: &str, source: &str) {
        for e in &self.errs {
            let mut report = Report::build(ReportKind::Error, (file, e.span.start..e.span.end));
            report = e.inner.report(report, file);
            report = report.with_label(
                Label::new((file, e.span.start..e.span.end))
                    .with_message("while parsing this block"),
            );
            report.finish().print((file, Source::from(source))).unwrap();
        }

        for w in &self.warns {
            let mut report = Report::build(ReportKind::Warning, (file, w.span.start..w.span.end));
            report = w.inner.report(report, file);
            report = report.with_label(
                Label::new((file, w.span.start..w.span.end))
                    .with_message("while parsing this block"),
            );
            report.finish().print((file, Source::from(source))).unwrap();
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Spanned<T> {
    pub inner: T,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstErr<'a> {
    UnexecpectedEOF,
    UnclosedBlock {
        at: Spanned<Token<'a>>,
        expected: Vec<Token<'a>>,
    },
    UnexpectedToken {
        expected: Vec<Token<'a>>,
        found: Spanned<Token<'a>>,
    },
    UndefinedFunctionCall {
        name: String,
    },
}

impl<'a> AstErr<'a> {
    pub fn at(self, span: Span) -> Spanned<AstErr<'a>> {
        Spanned { inner: self, span }
    }

    pub fn into_warn(self, msg: String) -> AstWarn<'a> {
        AstWarn::DeadCodeError {
            err: self,
            reason_for_dead: msg,
        }
    }
}

impl<'a> BuildReport for AstErr<'a> {
    fn report<'b, 'c>(
        &self,
        builder: ReportBuilder<'b, (&'c str, Range<usize>)>,
        file: &'b str,
    ) -> ReportBuilder<'b, (&'c str, Range<usize>)>
    where
        'b: 'c,
    {
        match self {
            AstErr::UnexecpectedEOF => builder.with_message("unexpected EOF"),
            AstErr::UnclosedBlock { at, expected } => {
                builder.with_message("unclosed code block").with_label(
                    Label::new((file, at.span.start..at.span.end))
                        .with_message(format!("expected one of {:?}", expected)),
                )
            }
            AstErr::UnexpectedToken { expected, found } => {
                builder.with_message("unexpected token").with_label(
                    Label::new((file, found.span.start..found.span.end)).with_message(format!(
                        "expected one of {:?}, found {:?}",
                        expected, found.inner
                    )),
                )
            }
            AstErr::UndefinedFunctionCall { name } => builder
                .with_message(format!("tried to call undefined function {}", name))
                .with_help("functions must be defined above the call site"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstWarn<'a> {
    DeadCodeError {
        err: AstErr<'a>,
        reason_for_dead: String,
    },
}

impl<'a> AstWarn<'a> {
    pub fn at(self, span: Span) -> Spanned<AstWarn<'a>> {
        Spanned { inner: self, span }
    }
}

impl<'c> BuildReport for AstWarn<'c> {
    fn report<'a, 'b>(
        &self,
        builder: ReportBuilder<'a, (&'b str, Range<usize>)>,
        file: &'a str,
    ) -> ReportBuilder<'a, (&'b str, Range<usize>)>
    where
        'a: 'b,
    {
        match self {
            Self::DeadCodeError {
                err,
                reason_for_dead,
            } => err
                .report(builder, file)
                .with_message(format!("ignored due to {}", reason_for_dead)),
        }
    }
}
