use ast::Ast;
use lexer::{LexErr, TokenStream};

use crate::{
    frontend::ast::{cfg::CfgEnv, error::Diagnostics},
    print_if,
};

pub mod ast;
mod lexer;

pub fn get_ast<'a>(s: &'a str, cfg_env: &CfgEnv) -> (Ast, Diagnostics<'a>) {
    let mut token_stream = TokenStream::from_str(s).map_err(FrontendErr::Lex).unwrap();
    print_if!(2, "stream: {}", token_stream);
    Ast::from_stream(&mut token_stream, cfg_env)
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FrontendErr {
    Lex(LexErr),
}
