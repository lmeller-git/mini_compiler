mod expr;
mod item;
mod stmt;

#[macro_export]
macro_rules! parse_list {
    (
        $stream:expr,
        $anchor:expr,
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
                    .at($anchor.clone().merge($stream.last_span.clone())),
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

#[macro_export]
macro_rules! expect_token {
    (
        $stream:expr,
        $diagnostics:expr,
        $anchor:expr,
        $expected:expr
    ) => {
        let _tok = $stream.peek();
        if !$expected.contains(_tok.as_ref()) {
            $diagnostics.errs.push(
                AstErr::UnclosedBlock {
                    at: _tok.clone(),
                    expected: $expected.into(),
                }
                .at($anchor.clone().merge($stream.last_span.clone())),
            )
        } else {
            $stream.advance();
        }
    };
}

#[macro_export]
macro_rules! skip_until_kw {
    ($stream:expr, $stop_at:pat) => {
        skip_until!(
            $stream,
            $stop_at
                | Token::EOF
                | Token::Keyword("begin_def")
                | Token::Keyword("extern_def")
                | Token::Keyword("end_def")
                | Token::Keyword("if")
                | Token::Keyword("cfg")
                | Token::Keyword("link_attr")
                | Token::Keyword("public")
        )
    };
}

#[macro_export]
macro_rules! skip_until {
    (
        $stream:expr,
        $stop_at:pat
    ) => {
        while !matches!(*$stream.peek().as_ref(), $stop_at) {
            $stream.advance();
        }
    };
}
