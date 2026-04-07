mod expr;
mod item;
mod stmt;

#[macro_export]
macro_rules! expect {
    (
        $stream:expr,
        $diagnostics:expr,
        $anchor:expr,
        $expected:expr,
        $else:ident
    ) => {
        let _tok = $stream.peek();
        if !$expected.contains(_tok.as_ref()) {
            $else!(
                $diagnostics,
                $expected,
                _tok.clone(),
                $anchor.clone().merge($stream.last_span.clone())
            )
        } else {
            $stream.advance();
        }
    };
}

#[macro_export]
macro_rules! kw {
    ($pattern:pat) => {
        $pattern
            | Token::Keyword("begin_def")
            | Token::Keyword("extern_def")
            | Token::Keyword("end_def")
            | Token::Keyword("if")
            | Token::Keyword("cfg")
            | Token::Keyword("link_attr")
            | Token::Keyword("public")
    };
}

#[macro_export]
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

#[macro_export]
macro_rules! skip_until_or_over {
    (
        $stream:expr,
        $stop_at:pat,
        $consume:pat
    ) => {
        $crate::skip_until!($stream, $stop_at);
        if matches!(*$stream.peek().as_ref(), $consume) {
            $stream.advance();
        }
    };
}
