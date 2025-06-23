use crate::frontend::ast::{Ast, Expr, Line, Operation, Val};
pub mod x86_64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeTree {
    units: Vec<CodeUnit>,
}

impl CodeTree {
    fn new() -> Self {
        Self { units: Vec::new() }
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
        name: String,
        value: Operand,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Immediate(i64),
    Variable(String),
    Temp(String),
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
            }
        }
        self.inner
    }

    fn lower_unit(&mut self, expr: &Expr) -> Operand {
        match expr {
            Expr::Val(v) => match v {
                Val::Var(name) => Operand::Variable(name.clone()),
                Val::V(val) => Operand::Immediate(*val),
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
                    name: "x".into(),
                    value: Operand::Temp("_temp_1".into()),
                },
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
                    name: "y".into(),
                    value: Operand::Temp("_temp_4".into()),
                },
                CodeUnit::Operation {
                    op: Operation::Add,
                    lhs: Operand::Variable("x".into()),
                    rhs: Operand::Immediate(2),
                    dest: Operand::Temp("_temp_5".into()),
                },
            ],
        };

        assert_eq!(code, code_true)
    }
}
