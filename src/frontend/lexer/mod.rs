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
                    w if w.is_whitespace() => break 'outer Self::parse_single(&s2[..n_parsed]),
                    _ => continue,
                }
            }
        };
        Ok((token, n_parsed + (s.len() - s2.len())))
    }

    fn parse_single(s: &'a str) -> Self {
        match s {
            _ => Self::Ident(s),
        }
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

    pub fn peek(&self) -> &Token<'_> {
        self.inner.get(self.cursor).unwrap_or(&Token::EOF)
    }

    pub fn peekn(&self, n: usize) -> &[Token<'_>] {
        &self.inner[self.cursor..(self.cursor + n).min(self.inner.len())]
    }

    fn push(&mut self, token: Token<'a>) {
        self.inner.push(token);
    }

    pub fn from_str(s: &'a str) -> Result<Self, LexErr> {
        let mut stream = Self::new();
        let mut total_parsed = 0;
        loop {
            let s = &s[total_parsed..];
            let (token, parsed) = Token::parse(s)?;
            total_parsed += parsed;
            stream.push(token);
            if total_parsed >= s.len() {
                break;
            }
        }
        Ok(stream)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LexErr {
    General,
}
