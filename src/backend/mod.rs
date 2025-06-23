use codegen::{CodeBuilder, CodeTree, x86_64::AsmWriter};

use crate::frontend::ast::Ast;

mod codegen;

// declarartion ->
// push value to stack and store relative position
// to access it: v = rsp + <pos>;
// func call:
// provide some global func for all funcs that are called ->
// prepare registers -> call

pub fn generate(ast: &Ast) -> Result<CodeTree, BackendErr> {
    Ok(CodeBuilder::new().build(ast))
}

pub fn asm_gen(code: CodeTree, name: &str) -> Result<(), BackendErr> {
    AsmWriter::new(name).write(&code);
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendErr {
    General,
}
