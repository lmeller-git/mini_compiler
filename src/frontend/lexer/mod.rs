use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token<'a> {
    Ident(&'a str),
    Eq,
    Not,
    NEq,
    EqEq,
    Or,
    And,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    OpenParen,
    CloseParen,
    Semi,
    EOF,
}

impl<'a> Token<'a> {
    fn parse(s: &'a str) -> Result<(Self, usize), LexErr> {
        let s2 = s.trim_start();
        if s2.is_empty() {
            return Ok((Self::EOF, 0));
        }
        let mut n_parsed = 0;
        let token = 'outer: loop {
            for (i, c) in s2.chars().enumerate() {
                n_parsed += 1;
                match c {
                    '+' => break 'outer Token::Add,
                    '-' => break 'outer Token::Sub,
                    '*' => break 'outer Token::Mul,
                    '/' => break 'outer Token::Div,
                    '%' => break 'outer Token::Mod,
                    '(' => break 'outer Token::OpenParen,
                    ')' => break 'outer Token::CloseParen,
                    ';' => break 'outer Token::Semi,
                    '|' => break 'outer Token::Or,
                    '&' => break 'outer Token::And,
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

    fn parse_single(s: &'a str, counter: &mut usize) -> Self {
        for (i, c) in s.chars().enumerate() {
            if !(c.is_digit(10) || c.is_alphabetic()) {
                *counter += i - 1;
                return Self::Ident(&s[..i]);
            }
        }
        *counter += s.len();
        Self::Ident(s)
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
        &self
            .inner
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
                Token::Mul,
                Token::Ident("5"),
                Token::Semi,
                Token::EOF
            ]
        );
    }
}
