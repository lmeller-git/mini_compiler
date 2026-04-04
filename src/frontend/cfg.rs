use std::collections::{HashMap, HashSet};

use crate::frontend::ast::{Expr, Operation};

#[derive(Debug)]
pub struct CfgEnv {
    flags: HashSet<String>,
    mappings: HashMap<String, String>,
}

impl CfgEnv {
    pub fn populate(mut self, cfgs: &[String]) -> Self {
        for cfg_str in cfgs {
            if let Some((key, value)) = cfg_str.split_once('=') {
                let clean_value = value.trim_matches('"');
                self.mappings
                    .insert(key.to_string(), clean_value.to_string());
            } else {
                self.flags.insert(cfg_str.to_string());
            }
        }
        self
    }

    pub fn eval_cfg_expr(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Val(v) => match v {
                super::ast::Val::Lit(str) => {
                    if let Some((key, value)) = str.split_once('=') {
                        self.mappings.get(key).is_some_and(|v| v == value)
                    } else {
                        self.flags.contains(str)
                    }
                }
                crate::frontend::ast::Val::Var(str) => self.flags.contains(str),
                _ => false,
            },
            Expr::Op(lhs, op, rhs) => match op {
                Operation::BitAND => {
                    let lhs = self.eval_cfg_expr(lhs.as_ref());
                    if !lhs {
                        return false;
                    }
                    self.eval_cfg_expr(rhs.as_ref())
                }
                Operation::Not => !self.eval_cfg_expr(rhs.as_ref()),
                Operation::BitOR => {
                    let lhs = self.eval_cfg_expr(lhs.as_ref());
                    if lhs {
                        return true;
                    }
                    self.eval_cfg_expr(rhs.as_ref())
                }
                Operation::BitXOR => {
                    let lhs = self.eval_cfg_expr(lhs.as_ref());
                    let rhs = self.eval_cfg_expr(rhs.as_ref());
                    lhs ^ rhs
                }
                _ => false,
            },
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for CfgEnv {
    fn default() -> Self {
        Self {
            flags: HashSet::default(),
            mappings: HashMap::default(),
        }
    }
}
