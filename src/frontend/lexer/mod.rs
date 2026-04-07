use std::{fmt::Display, ops::Deref};

use crate::frontend::ast::error::Spanned;

static KEYWORDS: &[&str] = &[
    "if",
    "begin_def",
    "end_def",
    "extern_def",
    "public",
    "link_attr",
    "cfg",
];

impl<'a> Spanned<Token<'a>> {
    fn new(token: Token<'a>, span: Span) -> Self {
        Self { inner: token, span }
    }
}

impl<'a> Deref for Spanned<Token<'a>> {
    type Target = Token<'a>;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'a> AsRef<Token<'a>> for Spanned<Token<'a>> {
    fn as_ref(&self) -> &Token<'a> {
        &self.inner
    }
}

impl<'a> Display for Spanned<Token<'a>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.as_ref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn merge(mut self, other: Span) -> Self {
        self.end = other.end;
        self
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token<'a> {
    Ident(&'a str),
    Number(i64),
    Lit(&'a str),
    Keyword(&'a str),
    Eq,
    Not,
    NEq,
    EqEq,
    Or,
    Ampercent,
    Add,
    Sub,
    Star,
    Div,
    Mod,
    OpenParen,
    CloseParen,
    Semi,
    Gt,
    Lt,
    Hat,
    Shr,
    Shl,
    Comma,
    Comment,
    EOF,
}

impl<'a> Token<'a> {
    fn parse(s: &'a str) -> Result<(Self, usize), LexErr> {
        if s.is_empty() {
            return Ok((Self::EOF, 0));
        }
        let mut n_parsed = 0;
        let token = 'outer: {
            let mut idc = s.char_indices();
            while let Some((i, c)) = idc.next() {
                n_parsed += c.len_utf8();
                match c {
                    '#' => {
                        for (_, c) in idc.by_ref() {
                            n_parsed += c.len_utf8();
                            if c == '\n' {
                                break;
                            }
                        }
                        break 'outer Token::Comment;
                    }
                    '\'' | '\"' => break 'outer Self::parse_quoted(&s[i..], &mut n_parsed),
                    '+' => break 'outer Token::Add,
                    '-' => break 'outer Token::Sub,
                    '*' => break 'outer Token::Star,
                    '/' => break 'outer Token::Div,
                    '%' => break 'outer Token::Mod,
                    '(' => break 'outer Token::OpenParen,
                    ')' => break 'outer Token::CloseParen,
                    ';' => break 'outer Token::Semi,
                    '|' => break 'outer Token::Or,
                    '&' => break 'outer Token::Ampercent,
                    ',' => break 'outer Token::Comma,
                    '>' => {
                        if s.chars().nth(i + 1).is_some_and(|c| c == '>') {
                            n_parsed += 1;
                            break 'outer Token::Shr;
                        } else {
                            break 'outer Token::Gt;
                        }
                    }
                    '<' => {
                        if s.chars().nth(i + 1).is_some_and(|c| c == '<') {
                            n_parsed += 1;
                            break 'outer Token::Shl;
                        } else {
                            break 'outer Token::Lt;
                        }
                    }
                    '^' => break 'outer Token::Hat,
                    '=' => {
                        if s.chars().nth(i + 1).is_some_and(|c| c == '=') {
                            n_parsed += 1;
                            break 'outer Token::EqEq;
                        } else {
                            break 'outer Token::Eq;
                        }
                    }
                    '!' => {
                        if s.chars().nth(i + 1).is_some_and(|c| c == '=') {
                            n_parsed += 1;
                            break 'outer Token::NEq;
                        } else {
                            break 'outer Token::Not;
                        }
                    }
                    w if w.is_whitespace() => continue,
                    _ => break 'outer Self::parse_single(&s[i..], &mut n_parsed),
                }
            }
            panic!("malformed input");
        };
        Ok((token, n_parsed))
    }

    fn parse_quoted(s: &'a str, counter: &mut usize) -> Self {
        let quotation_open = s.chars().next().unwrap();
        let inner_name_end = s[quotation_open.len_utf8()..].find(quotation_open).expect("Quotations can currently not be nested and must be closed using the same qutation mark");
        let str_lit = &s[quotation_open.len_utf8()..=inner_name_end];
        *counter += inner_name_end + quotation_open.len_utf8();
        Self::Lit(str_lit)
    }

    fn parse_single(s: &'a str, counter: &mut usize) -> Self {
        let is_number = s.chars().next().is_some_and(|c| c.is_ascii_digit());
        let mut last_char_len = 0;
        for (i, c) in s.char_indices() {
            if !((is_number && c.is_ascii_digit())
                || (!is_number && (c.is_alphanumeric() || c == '_')))
            {
                *counter += i - last_char_len;

                if is_number {
                    return Self::Number(s[..i].parse::<i64>().unwrap());
                } else {
                    return Self::Ident(&s[..i]).map_keyword();
                }
            }
            last_char_len = c.len_utf8();
        }
        *counter += s.len();
        if is_number {
            Self::Number(s.parse::<i64>().unwrap())
        } else {
            Self::Ident(s).map_keyword()
        }
    }

    fn map_keyword(self) -> Self {
        match self {
            Self::Ident(id) if KEYWORDS.contains(&id) => Self::Keyword(id),
            _ => self,
        }
    }
}

impl Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ident(_ident) => write!(f, "Ident"),
            Self::Number(_num) => write!(f, "Number"),
            Self::Lit(_lit) => write!(f, "String Literal"),
            _ => write!(f, "{:?}", self),
        }
    }
}

#[derive(Debug)]
pub struct TokenStream<'a> {
    inner: Vec<Spanned<Token<'a>>>,
    cursor: usize,
    pub last_span: Span,
}

impl<'a> TokenStream<'a> {
    fn new() -> Self {
        Self {
            inner: Vec::new(),
            cursor: 0,
            last_span: Span { start: 0, end: 0 },
        }
    }

    pub fn next<'b>(&'b mut self) -> &'b Spanned<Token<'a>> {
        self.last_span = self.peek().span.clone();
        let r = self
            .inner
            .get(self.cursor)
            .unwrap_or_else(|| self.inner.last().unwrap());
        self.cursor = self.inner.len().min(self.cursor + 1);
        r
    }

    pub fn advance(&mut self) {
        self.last_span = self.peek().span.clone();
        self.cursor = self.inner.len().min(self.cursor + 1);
    }

    pub fn peek<'b>(&'b self) -> &'b Spanned<Token<'a>> {
        self.inner
            .get(self.cursor)
            .unwrap_or_else(|| self.inner.last().unwrap())
    }

    pub fn peekn<'b>(&'b self, n: usize) -> &'b Spanned<Token<'a>> {
        self.inner
            .get((self.cursor + n).min(self.inner.len()))
            .unwrap_or_else(|| self.inner.last().unwrap())
    }

    fn push(&mut self, token: Spanned<Token<'a>>) {
        self.inner.push(token);
    }

    pub fn from_str(s: &'a str) -> Result<Self, LexErr> {
        let mut stream = Self::new();
        let mut total_parsed = 0;
        while total_parsed < s.len() {
            let remainder = &s[total_parsed..];
            let trimmed = remainder.trim_start();
            if trimmed.is_empty() {
                break;
            }
            let whitespace_len = remainder.len() - trimmed.len();

            let (token, parsed) = Token::parse(trimmed)?;
            if token != Token::Comment {
                stream.push(Spanned::new(
                    token,
                    Span {
                        start: total_parsed + whitespace_len,
                        end: total_parsed + whitespace_len + parsed,
                    },
                ));
            }

            total_parsed += parsed + whitespace_len;
        }
        stream.push(Spanned::new(
            Token::EOF,
            Span {
                start: s.len(),
                end: s.len(),
            },
        ));
        Ok(stream)
    }
}

impl Display for TokenStream<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tokens: {:#?}\ncursor: {}", self.inner, self.cursor)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LexErr {
    General,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize() {
        let val = "1;";
        assert_eq!(
            TokenStream::from_str(val)
                .unwrap()
                .inner
                .into_iter()
                .map(|item| item.as_ref().clone())
                .collect::<Vec<_>>(),
            vec![Token::Number(1), Token::Semi, Token::EOF]
        );
        let val = "foo;";
        assert_eq!(
            TokenStream::from_str(val)
                .unwrap()
                .inner
                .into_iter()
                .map(|item| item.as_ref().clone())
                .collect::<Vec<_>>(),
            vec![Token::Ident("foo"), Token::Semi, Token::EOF]
        );

        let line = "foo = 5;";
        assert_eq!(
            TokenStream::from_str(line)
                .unwrap()
                .inner
                .into_iter()
                .map(|item| item.as_ref().clone())
                .collect::<Vec<_>>(),
            vec![
                Token::Ident("foo"),
                Token::Eq,
                Token::Number(5),
                Token::Semi,
                Token::EOF
            ]
        );

        let line = "(foo  + 2) *5;";

        assert_eq!(
            TokenStream::from_str(line)
                .unwrap()
                .inner
                .into_iter()
                .map(|item| item.as_ref().clone())
                .collect::<Vec<_>>(),
            vec![
                Token::OpenParen,
                Token::Ident("foo"),
                Token::Add,
                Token::Number(2),
                Token::CloseParen,
                Token::Star,
                Token::Number(5),
                Token::Semi,
                Token::EOF
            ]
        );
    }

    #[test]
    fn underscore() {
        let text = "hello_world";
        assert_eq!(
            TokenStream::from_str(text)
                .unwrap()
                .inner
                .into_iter()
                .map(|item| item.as_ref().clone())
                .collect::<Vec<_>>(),
            vec![Token::Ident("hello_world"), Token::EOF]
        )
    }

    #[test]
    fn str_lit() {
        let txt1 = "\"hello world\"";
        assert_eq!(
            TokenStream::from_str(txt1)
                .unwrap()
                .inner
                .into_iter()
                .map(|item| item.as_ref().clone())
                .collect::<Vec<_>>(),
            vec![Token::Lit("hello world"), Token::EOF]
        );

        let txt2 = "\'hello world\'";
        assert_eq!(
            TokenStream::from_str(txt2)
                .unwrap()
                .inner
                .into_iter()
                .map(|item| item.as_ref().clone())
                .collect::<Vec<_>>(),
            vec![Token::Lit("hello world"), Token::EOF]
        );

        let txt3 = "\"hello world\";";
        assert_eq!(
            TokenStream::from_str(txt3)
                .unwrap()
                .inner
                .into_iter()
                .map(|item| item.as_ref().clone())
                .collect::<Vec<_>>(),
            vec![Token::Lit("hello world"), Token::Semi, Token::EOF]
        );

        let txt4 = "print_str \"hello world\";";
        assert_eq!(
            TokenStream::from_str(txt4)
                .unwrap()
                .inner
                .into_iter()
                .map(|item| item.as_ref().clone())
                .collect::<Vec<_>>(),
            vec![
                Token::Ident("print_str"),
                Token::Lit("hello world"),
                Token::Semi,
                Token::EOF
            ]
        )
    }
}
