use std::path::Path;

use codegen::x86_64::AsmWriter;

use crate::{backend::codegen::ProgramIR, frontend::ast::Ast};

mod codegen;

// declarartion ->
// push value to stack and store relative position
// to access it: v = rsp + <pos>;
// func call:
// provide some global func for all funcs that are called ->
// prepare registers -> call

pub fn generate(ast: &Ast) -> Result<ProgramIR, BackendErr> {
    Ok(ProgramIR::build(ast))
}

pub fn asm_gen(code: ProgramIR, name: &Path) -> Result<(), BackendErr> {
    AsmWriter::new(name, &code).write(&code);
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendErr {
    General,
}
