use std::{collections::HashMap, fmt::Display};

use crate::frontend::ast::{Ast, Expr, LValue, Line, Operation, Val};
pub mod x86_64;

#[derive(Clone, Debug, PartialEq, Eq)]
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
}

impl CodeBuilder {
    pub fn new() -> Self {
        Self {
            inner: CodeTree::new(),
            temp_name: 0,
        }
    }

    fn new_temp(&mut self) -> String {
        let name = format!("_temp_{}", self.temp_name);
        self.temp_name += 1;
        name
    }

    fn current_temp(&self) -> String {
        format!("_temp_{}", self.temp_name)
    }

    pub fn build(mut self, ast: &Ast) -> CodeTree {
        for line in ast.lines() {
            self.lower_line(line);
            self.inner.units.push(CodeUnit::Cleanup);
        }
        self.inner
    }

    fn lower_line(&mut self, line: &Line) {
        match line {
            Line::Expr(e) => _ = self.lower_unit(e),
            Line::Call(f, e) => {
                let arg = self.lower_unit(e);
                self.inner.units.push(CodeUnit::FuncCall {
                    name: f.clone(),
                    args: vec![arg],
                });
            }
            Line::Decl(v, e) => {
                let val = self.lower_unit(e);
                self.inner.units.push(CodeUnit::Assignment {
                    name: v.clone(),
                    value: val,
                });
            }
            Line::Cond(cond, then) => {
                let cond = self.lower_unit(cond);
                let label = self.new_temp();
                let mut builder = CodeBuilder::new();
                builder.temp_name = self.temp_name;
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

    fn lower_unit(&mut self, expr: &Expr) -> Operand {
        match expr {
            Expr::Val(v) => match v {
                Val::Var(name) => Operand::Variable(name.clone()),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::get_ast;

    #[test]
    fn code() {
        let s = "
            x = 5 + (1 * 2);
            print x + 3;
            y = x * (5 - 2);
            x + 2;
        ";
        let ast = get_ast(s).unwrap();
        let code = CodeBuilder::new().build(&ast);
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
                    name: LValue::Variable("x".into()),
                    value: Operand::Temp("_temp_1".into()),
                },
                CodeUnit::Cleanup,
                CodeUnit::Operation {
                    op: Operation::Add,
                    lhs: Operand::Variable("x".into()),
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
                    lhs: Operand::Variable("x".into()),
                    rhs: Operand::Temp("_temp_3".into()),
                    dest: Operand::Temp("_temp_4".into()),
                },
                CodeUnit::Assignment {
                    name: LValue::Variable("y".into()),
                    value: Operand::Temp("_temp_4".into()),
                },
                CodeUnit::Cleanup,
                CodeUnit::Operation {
                    op: Operation::Add,
                    lhs: Operand::Variable("x".into()),
                    rhs: Operand::Immediate(2),
                    dest: Operand::Temp("_temp_5".into()),
                },
                CodeUnit::Cleanup,
            ],
        };

        assert_eq!(code, code_true)
    }
}
