use ast::Ast;
use lexer::{LexErr, TokenStream};

use crate::{frontend::cfg::CfgEnv, print_if};

pub mod ast;
pub mod cfg;
mod lexer;

pub fn get_ast(s: &str, cfg_env: &CfgEnv) -> Result<Ast, FrontendErr> {
    let mut token_stream = TokenStream::from_str(s).map_err(FrontendErr::Lex)?;
    print_if!(2, "stream: {}", token_stream);
    Ok(Ast::from_stream(&mut token_stream, cfg_env))
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FrontendErr {
    Lex(LexErr),
}
