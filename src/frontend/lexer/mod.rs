use std::fmt::Display;

static KEYWORDS: &[&str] = &["if"];

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token<'a> {
    Ident(&'a str),
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
    EOF,
}

impl<'a> Token<'a> {
    fn parse(s: &'a str) -> Result<(Self, usize), LexErr> {
        let s2 = s.trim_start();
        if s2.is_empty() {
            return Ok((Self::EOF, 0));
        }
        let mut n_parsed = 0;
        let token = 'outer: {
            let mut idc = s2.char_indices();
            'inner: while let Some((i, c)) = idc.next() {
                n_parsed += c.len_utf8();
                match c {
                    '#' => {
                        for (_, c) in idc.by_ref() {
                            n_parsed += c.len_utf8();
                            if c == '\n' {
                                continue 'inner;
                            }
                        }
                        break 'outer Token::EOF;
                    }
                    '\'' | '\"' => break 'outer Self::parse_quoted(&s2[i..], &mut n_parsed),
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
                    '>' => {
                        if s2.chars().nth(i + 1).is_some_and(|c| c == '>') {
                            n_parsed += 1;
                            break 'outer Token::Shr;
                        } else {
                            break 'outer Token::Gt;
                        }
                    }
                    '<' => {
                        if s2.chars().nth(i + 1).is_some_and(|c| c == '<') {
                            n_parsed += 1;
                            break 'outer Token::Shl;
                        } else {
                            break 'outer Token::Lt;
                        }
                    }
                    '^' => break 'outer Token::Hat,
                    '=' => {
                        if s2.chars().nth(i + 1).is_some_and(|c| c == '=') {
                            n_parsed += 1;
                            break 'outer Token::EqEq;
                        } else {
                            break 'outer Token::Eq;
                        }
                    }
                    '!' => {
                        if s2.chars().nth(i + 1).is_some_and(|c| c == '=') {
                            n_parsed += 1;
                            break 'outer Token::NEq;
                        } else {
                            break 'outer Token::Not;
                        }
                    }
                    w if w.is_whitespace() => continue,
                    _ => break 'outer Self::parse_single(&s2[i..], &mut n_parsed),
                }
            }
            panic!("malformed input");
        };
        Ok((token, n_parsed + (s.len() - s2.len())))
    }

    fn parse_quoted(s: &'a str, counter: &mut usize) -> Self {
        let quotation_open = s.chars().next().unwrap();
        let inner_name_end = s[quotation_open.len_utf8()..].find(quotation_open).expect("Quotations can currently not be nested and must be closed using the same qutation mark");
        let str_lit = &s[quotation_open.len_utf8()..=inner_name_end];
        *counter += inner_name_end + quotation_open.len_utf8();
        Self::Lit(str_lit)
    }

    fn parse_single(s: &'a str, counter: &mut usize) -> Self {
        let mut last_char_len = 0;
        for (i, c) in s.char_indices() {
            if !(c.is_alphanumeric() || matches!(c, '_')) {
                *counter += i - last_char_len;
                return Self::Ident(&s[..i]).map_keyword();
            }
            last_char_len = c.len_utf8();
        }
        *counter += s.len();
        Self::Ident(s).map_keyword()
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
        write!(f, "{:#?}", self)
    }
}

pub struct TokenStream<'a> {
    inner: Vec<Token<'a>>,
    cursor: usize,
}

impl<'a> TokenStream<'a> {
    fn new() -> Self {
        Self {
            inner: Vec::new(),
            cursor: 0,
        }
    }

    pub fn next(&mut self) -> &Token<'_> {
        let r = self.inner.get(self.cursor).unwrap_or(&Token::EOF);
        self.cursor = self.inner.len().min(self.cursor + 1);
        r
    }

    pub fn advance(&mut self) {
        self.cursor = self.inner.len().min(self.cursor + 1);
    }

    pub fn peek(&self) -> &Token<'_> {
        self.inner.get(self.cursor).unwrap_or(&Token::EOF)
    }

    pub fn peekn(&self, n: usize) -> &Token<'_> {
        self.inner
            .get((self.cursor + n).min(self.inner.len()))
            .unwrap_or(&Token::EOF)
    }

    fn push(&mut self, token: Token<'a>) {
        self.inner.push(token);
    }

    pub fn from_str(s: &'a str) -> Result<Self, LexErr> {
        let s = s.trim();
        let mut stream = Self::new();
        let mut total_parsed = 0;
        loop {
            let s_ = &s[total_parsed..];
            let (token, parsed) = Token::parse(s_)?;
            total_parsed += parsed;
            stream.push(token);
            if total_parsed >= s.len() {
                break;
            }
        }
        stream.push(Token::EOF);
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
            TokenStream::from_str(val).unwrap().inner,
            vec![Token::Ident("1"), Token::Semi, Token::EOF]
        );
        let val = "foo;";
        assert_eq!(
            TokenStream::from_str(val).unwrap().inner,
            vec![Token::Ident("foo"), Token::Semi, Token::EOF]
        );

        let line = "foo = 5;";
        assert_eq!(
            TokenStream::from_str(line).unwrap().inner,
            vec![
                Token::Ident("foo"),
                Token::Eq,
                Token::Ident("5"),
                Token::Semi,
                Token::EOF
            ]
        );

        let line = "(foo  + 2) *5;";

        assert_eq!(
            TokenStream::from_str(line).unwrap().inner,
            vec![
                Token::OpenParen,
                Token::Ident("foo"),
                Token::Add,
                Token::Ident("2"),
                Token::CloseParen,
                Token::Star,
                Token::Ident("5"),
                Token::Semi,
                Token::EOF
            ]
        );
    }

    #[test]
    fn underscore() {
        let text = "hello_world";
        assert_eq!(
            TokenStream::from_str(text).unwrap().inner,
            vec![Token::Ident("hello_world"), Token::EOF]
        )
    }

    #[test]
    fn str_lit() {
        let txt1 = "\"hello world\"";
        assert_eq!(
            TokenStream::from_str(txt1).unwrap().inner,
            vec![Token::Lit("hello world"), Token::EOF]
        );

        let txt2 = "\'hello world\'";
        assert_eq!(
            TokenStream::from_str(txt2).unwrap().inner,
            vec![Token::Lit("hello world"), Token::EOF]
        );

        let txt3 = "\"hello world\";";
        assert_eq!(
            TokenStream::from_str(txt3).unwrap().inner,
            vec![Token::Lit("hello world"), Token::Semi, Token::EOF]
        );

        let txt4 = "print_str \"hello world\";";
        assert_eq!(
            TokenStream::from_str(txt4).unwrap().inner,
            vec![
                Token::Ident("print_str"),
                Token::Lit("hello world"),
                Token::Semi,
                Token::EOF
            ]
        )
    }
}
