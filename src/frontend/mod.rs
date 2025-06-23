use ast::Ast;
use lexer::{LexErr, TokenStream};

pub mod ast;
mod lexer;

pub fn get_ast(s: &str) -> Result<Ast, FrontendErr> {
    let mut token_stream = TokenStream::from_str(s).map_err(|e| FrontendErr::Lex(e))?;
    // println!("stream: {}", token_stream);
    Ok(Ast::from_stream(&mut token_stream))
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FrontendErr {
    Lex(LexErr),
}
