use crate::Ordering;
use ast::Ast;
use lexer::{LexErr, TokenStream};

use crate::print_if;

pub mod ast;
mod lexer;

pub fn get_ast(s: &str) -> Result<Ast, FrontendErr> {
    let mut token_stream = TokenStream::from_str(s).map_err(|e| FrontendErr::Lex(e))?;
    print_if!(2, "stream: {}", token_stream);
    Ok(Ast::from_stream(&mut token_stream))
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FrontendErr {
    Lex(LexErr),
}
