use std::{collections::HashMap, fmt::Display};

use indexmap::IndexMap;

use crate::frontend::ast::{Ast, Expr, LValue, Line, Operation, Val, is_builtin_func};
pub mod x86_64;

#[derive(Debug)]
pub struct ProgramIR {
    pub functions: indexmap::IndexMap<String, FunctionIR>,
    pub external: indexmap::IndexMap<String, FunctionIR>,
}

impl ProgramIR {
    pub fn build(ast: &Ast) -> Self {
        let mut functions = IndexMap::new();
        let mut external = IndexMap::new();
        let mut builder = CodeBuilder::new();
        for func in ast.funcs() {
            if let Some(func_body) = func.body() {
                functions.insert(
                    func.name.clone(),
                    FunctionIR {
                        body: builder.build(func_body, func.name.clone()),
                        name: func.name.clone(),
                        args: func
                            .args
                            .clone()
                            .into_iter()
                            .map(|mut arg| {
                                builder.rename_ident(&mut arg);
                                arg
                            })
                            .collect(),
                        is_public: func.is_public,
                    },
                );
            } else {
                external.insert_full(
                    func.name.clone(),
                    FunctionIR {
                        name: func.name.clone(),
                        args: func
                            .args
                            .clone()
                            .into_iter()
                            .map(|mut arg| {
                                builder.rename_ident(&mut arg);
                                arg
                            })
                            .collect(),
                        body: CodeTree::default(),
                        is_public: func.is_public,
                    },
                );
            }
        }
        Self {
            functions,
            external,
        }
    }
}

#[derive(Debug)]
pub struct FunctionIR {
    pub name: String,
    pub args: Vec<String>,
    pub body: CodeTree,
    pub is_public: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct CodeTree {
    data: HashMap<DataUnit, String>,
    units: Vec<CodeUnit>,
}

impl CodeTree {
    fn new() -> Self {
        Self {
            data: HashMap::new(),
            units: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DataUnit {
    StrLit(String),
}

impl DataUnit {
    pub fn write_data(&self) -> &str {
        match self {
            Self::StrLit(lit) => lit,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodeUnit {
    FuncCall {
        name: String,
        args: Vec<Operand>,
    },
    Operation {
        op: Operation,
        lhs: Operand,
        rhs: Operand,
        dest: Operand,
    },
    Assignment {
        name: LValue,
        value: Operand,
    },
    Condition {
        eval: Operand,
        then: Vec<CodeUnit>,
        label: String,
    },
    Cleanup,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Immediate(i64),
    Variable(String),
    Temp(String),
}

impl Display for Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Immediate(value) => write!(f, "{value}"),
            Self::Variable(var) | Self::Temp(var) => write!(f, "{var}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeBuilder {
    inner: CodeTree,
    temp_name: usize,
    context_name: String,
}

impl CodeBuilder {
    pub fn new() -> Self {
        Self {
            context_name: String::default(),
            inner: CodeTree::new(),
            temp_name: 0,
        }
    }

    fn new_temp(&mut self) -> String {
        let name = format!("_temp_{}", self.temp_name);
        self.temp_name += 1;
        name
    }

    pub fn build<'a>(
        &mut self,
        lines: impl Iterator<Item = &'a Line>,
        context_name: String,
    ) -> CodeTree {
        self.context_name = context_name;
        for line in lines {
            self.lower_line(line);
            self.inner.units.push(CodeUnit::Cleanup);
        }
        core::mem::take(&mut self.inner)
    }

    fn lower_line(&mut self, line: &Line) {
        match line {
            Line::Expr(e) => _ = self.lower_unit(e),
            Line::Call(f, e) => {
                let args = if is_builtin_func(f) {
                    self.lower_builtin(f, e)
                } else {
                    e.iter().map(|e| self.lower_unit(e)).collect()
                };
                self.inner.units.push(CodeUnit::FuncCall {
                    name: f.clone(),
                    args,
                });
            }
            Line::Decl(v, e) => {
                let val = self.lower_unit(e);
                let mut name = v.clone();
                self.rename_lvalue(&mut name);
                self.inner
                    .units
                    .push(CodeUnit::Assignment { name, value: val });
            }
            Line::Cond(cond, then) => {
                let cond = self.lower_unit(cond);
                let label = self.new_temp();
                let mut builder = CodeBuilder::new();
                builder.temp_name = self.temp_name;
                builder.context_name = self.context_name.clone();
                builder.lower_line(then);
                self.temp_name = builder.temp_name;
                self.inner.data.extend(builder.inner.data.drain());
                self.inner.units.push(CodeUnit::Condition {
                    eval: cond,
                    then: builder.inner.units,
                    label,
                });
            }
        }
    }

    fn lower_builtin(&mut self, name: &str, exprs: &[Expr]) -> Vec<Operand> {
        match name {
            "addr_of" => {
                // here we expect two args: [Variable(Ident), Operation (the target to store the result to)]
                // We search the current function for variables of matching names, if one is found we return its addr, else we assume this to be an external symbol
                debug_assert_eq!(exprs.len(), 2);
                let var = &exprs[0];
                let to = &exprs[1];
                let Expr::Val(Val::Var(ident)) = var else {
                    panic!("currently only idents may be passed to addr_of");
                };

                let mut ident_as_var = ident.to_string();
                self.rename_ident(&mut ident_as_var);

                let ident = if self.inner.units.iter().any(|unit| {
                    if let CodeUnit::Assignment { name, value: _ } = unit
                        && let LValue::Variable(name) = name
                        && *name == ident_as_var
                    {
                        true
                    } else {
                        false
                    }
                }) {
                    ident_as_var
                } else {
                    ident.to_string()
                };

                vec![Operand::Variable(ident), self.lower_unit(to)]
            }
            "asm" => {
                // we expect on argument, which is a string literal (or an ident?). we will emit this again as Variable/Ident.
                debug_assert_eq!(exprs.len(), 1);
                let (Expr::Val(Val::Var(lit)) | Expr::Val(Val::Lit(lit))) = &exprs[0] else {
                    panic!("cannot interpret non string literals/idents as assembly");
                };
                vec![Operand::Variable(lit.to_string())]
            }
            _ => exprs.iter().map(|e| self.lower_unit(e)).collect(),
        }
    }

    fn lower_unit(&mut self, expr: &Expr) -> Operand {
        match expr {
            Expr::Val(v) => match v {
                Val::Var(name) => {
                    let mut name = name.clone();
                    self.rename_ident(&mut name);
                    Operand::Variable(name)
                }
                Val::V(val) => Operand::Immediate(*val),
                Val::Lit(lit) => {
                    let unit = DataUnit::StrLit(lit.clone());
                    let temp_var = if let Some(temp_var) = self.inner.data.get(&unit) {
                        temp_var.clone()
                    } else {
                        let new_temp = self.new_temp();
                        self.inner.data.insert(unit, new_temp.clone());
                        new_temp
                    };
                    let in_scope_temp = self.new_temp();
                    self.inner.units.push(CodeUnit::Operation {
                        op: Operation::AsRef,
                        lhs: Operand::Immediate(0), // random value. will be overwitten by load
                        rhs: Operand::Variable(temp_var),
                        dest: Operand::Temp(in_scope_temp.clone()),
                    });
                    Operand::Temp(in_scope_temp)
                }
            },
            Expr::Op(lhs, op, rhs) => {
                let lhs = self.lower_unit(lhs.as_ref());
                let rhs = self.lower_unit(rhs.as_ref());
                let res = self.new_temp();
                self.inner.units.push(CodeUnit::Operation {
                    op: *op,
                    lhs,
                    rhs,
                    dest: Operand::Temp(res.clone()),
                });
                Operand::Temp(res)
            }
        }
    }

    fn rename_lvalue(&mut self, lvalue: &mut LValue) {
        match lvalue {
            LValue::Variable(var) => self.rename_ident(var),
            LValue::Deref(lvalue) => self.rename_lvalue(lvalue.as_mut()),
        }
    }

    fn rename_ident(&mut self, ident: &mut String) {
        *ident = format!("__{}_var_{}", self.context_name, ident);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::get_ast;

    #[test]
    fn code() {
        let s = "
        begin_def main;
            x = 5 + (1 * 2);
            print x + 3;
            y = x * (5 - 2);
            x + 2;
            end_def
        ";
        let ast = get_ast(s).unwrap();
        let code =
            CodeBuilder::new().build(ast.funcs().next().unwrap().body().unwrap(), "main".into());
        let code_true = CodeTree {
            data: HashMap::new(),
            units: vec![
                CodeUnit::Operation {
                    op: Operation::Mul,
                    lhs: Operand::Immediate(1),
                    rhs: Operand::Immediate(2),
                    dest: Operand::Temp("_temp_0".into()),
                },
                CodeUnit::Operation {
                    op: Operation::Add,
                    lhs: Operand::Immediate(5),
                    rhs: Operand::Temp("_temp_0".into()),
                    dest: Operand::Temp("_temp_1".into()),
                },
                CodeUnit::Assignment {
                    name: LValue::Variable("__main_var_x".into()),
                    value: Operand::Temp("_temp_1".into()),
                },
                CodeUnit::Cleanup,
                CodeUnit::Operation {
                    op: Operation::Add,
                    lhs: Operand::Variable("__main_var_x".into()),
                    rhs: Operand::Immediate(3),
                    dest: Operand::Temp("_temp_2".into()),
                },
                CodeUnit::FuncCall {
                    name: "print".into(),
                    args: vec![Operand::Temp("_temp_2".into())],
                },
                CodeUnit::Cleanup,
                CodeUnit::Operation {
                    op: Operation::Sub,
                    lhs: Operand::Immediate(5),
                    rhs: Operand::Immediate(2),
                    dest: Operand::Temp("_temp_3".into()),
                },
                CodeUnit::Operation {
                    op: Operation::Mul,
                    lhs: Operand::Variable("__main_var_x".into()),
                    rhs: Operand::Temp("_temp_3".into()),
                    dest: Operand::Temp("_temp_4".into()),
                },
                CodeUnit::Assignment {
                    name: LValue::Variable("__main_var_y".into()),
                    value: Operand::Temp("_temp_4".into()),
                },
                CodeUnit::Cleanup,
                CodeUnit::Operation {
                    op: Operation::Add,
                    lhs: Operand::Variable("__main_var_x".into()),
                    rhs: Operand::Immediate(2),
                    dest: Operand::Temp("_temp_5".into()),
                },
                CodeUnit::Cleanup,
            ],
        };

        assert_eq!(code, code_true)
    }
}
