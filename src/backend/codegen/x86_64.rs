use std::{
    collections::HashMap,
    fmt::{Arguments, Display},
    fs::File,
    io::Write,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{frontend::ast::Operation, print_if};

use super::{CodeTree, CodeUnit, Operand};

pub struct AsmWriter {
    fh: File,
}

impl AsmWriter {
    pub fn new(path: &Path) -> Self {
        let mut file = File::create(path).unwrap();
        print_if!(1, "writing asm code to {}", path.display());

        write!(
            file,
            "section .data\n\tformat db \"%ld\", 10, 0\nsection .text\n\tglobal main\n\textern printf\n\textern exit\n\n"
        )
        .unwrap();

        Self { fh: file }
    }

    pub fn write(mut self, code: &CodeTree) {
        writeln!(self.fh, "main:").unwrap();
        // ensure stack is 16-byte aligned. It is currently misaligned due to the C-runtime calling main
        self.write_in_fn(format_args!("sub rsp, 8"));
        let mut all_vars = Vars::default();
        let mut temps = TempVarTrack::default();

        for unit in &code.units {
            match unit {
                CodeUnit::FuncCall { name, args } => {
                    // for now assuming a single argument
                    // TODO move temp in rdi also (mutate temps accordingly)
                    // currently not necessary, as no native funcs exist

                    let func_name = self.get_func_name(name);
                    if self.is_builtin(func_name) {
                        self.call_builtin(func_name, args, &all_vars, &mut temps);
                    } else {
                        if USAGE[Reg::RDI as usize].load(Ordering::Relaxed) {
                            self.write_in_fn(format_args!("push rdi"));
                            temps.inc_stack(8);
                        }
                        for op in args.iter() {
                            //TODO mov in other regs depending on i
                            self.write_in_fn(format_args!(
                                "mov rdi, {}",
                                self.get_var_str(op, &all_vars, &temps)
                            ));
                        }
                        self.write_in_fn(format_args!("call {}", func_name));
                        if USAGE[Reg::RDI as usize].load(Ordering::Relaxed) {
                            self.write_in_fn(format_args!("pop rdi"));
                            temps.dec_stack(8);
                        }
                    }
                }
                CodeUnit::Operation { op, lhs, rhs, dest } => {
                    match dest {
                        Operand::Variable(name) => {
                            self.write_in_fn(format_args!(
                                "mov rax, {}",
                                self.get_var_str(lhs, &all_vars, &temps)
                            ));
                            self.write_op(op, rhs, &mut temps, &all_vars);

                            self.write_in_fn(format_args!("mov qword [{}], rax", name));
                        }
                        Operand::Temp(name) => {
                            self.write_in_fn(format_args!(
                                "mov rax, {}",
                                self.get_var_str(lhs, &all_vars, &temps)
                            ));
                            self.write_op(op, rhs, &mut temps, &all_vars);

                            let addr = self.get_or_init_temp(name, &mut temps);
                            self.write_in_fn(format_args!("mov {}, rax", addr));
                        }
                        _ => panic!("cannot assign to rvalue"),
                    };
                    // self.write_in_fn(format_args!());
                }
                CodeUnit::Assignment { name, value } => {
                    all_vars.add(name.clone());
                    let reg = self.get_var_from_reg(value, &all_vars, &temps);
                    self.write_in_fn(format_args!("mov qword [{}], {}", name, reg));
                }
            }
        }

        self.write_in_fn(format_args!("mov edi, 0"));
        self.write_in_fn(format_args!("call exit"));

        write!(self.fh, "\nsection .bss\n{}", all_vars).unwrap();
        write!(
            self.fh,
            "\nsection .note.GNU-stack noalloc noexec nowrite progbits"
        )
        .unwrap();
    }

    fn get_func_name<'a>(&self, name: &'a str) -> &'a str {
        match name {
            "print" => "printf",
            _ => name,
        }
    }

    fn is_builtin(&self, name: &str) -> bool {
        matches!(name, "printf" | "exit")
    }

    fn call_builtin(
        &mut self,
        name: &str,
        args: &[Operand],
        vars: &Vars,
        temps: &mut TempVarTrack,
    ) {
        match name {
            "printf" => {
                // TODO should also modify temps accordingly
                if USAGE[Reg::RSI as usize].load(Ordering::Relaxed) {
                    self.write_in_fn(format_args!("push rsi"));
                    temps.inc_stack(8);
                }
                if USAGE[Reg::RDI as usize].load(Ordering::Relaxed) {
                    self.write_in_fn(format_args!("push rdi"));
                    temps.inc_stack(8);
                }
                if !temps.stack_aligned() {
                    self.write_in_fn(format_args!("sub rsp, 8"));
                    temps.inc_stack(8);
                }
                self.write_in_fn(format_args!(
                    "mov rsi, {}",
                    self.get_var_str(&args[0], vars, temps)
                ));
                self.write_in_fn(format_args!("mov rdi, format"));
                self.write_in_fn(format_args!("xor rax, rax"));
                self.write_in_fn(format_args!("call printf"));
                if USAGE[Reg::RDI as usize].load(Ordering::Relaxed) {
                    self.write_in_fn(format_args!("pop rdi"));
                    temps.dec_stack(8);
                }
                if USAGE[Reg::RSI as usize].load(Ordering::Relaxed) {
                    self.write_in_fn(format_args!("pop rsi"));
                    temps.dec_stack(8);
                }
            }
            "exit" => {
                if !temps.stack_aligned() {
                    self.write_in_fn(format_args!("sub rsp, 8"));
                    temps.inc_stack(8);
                }
                self.write_in_fn(format_args!("mov edi, 0"));
                self.write_in_fn(format_args!("call exit"));
            }
            _ => {}
        }
    }

    fn get_or_init_temp(&mut self, k: &str, temps: &mut TempVarTrack) -> String {
        let loc = if let Some(loc) = temps.get(k) {
            loc
        } else {
            if USAGE.iter().all(|s| s.load(Ordering::Relaxed)) {
                // all regs are in usage, the temp will be pushed to the stack, and we must allocate memory for it
                self.write_in_fn(format_args!("sub rsp, 8"));
                temps.inc_stack(8);
            }
            temps.add(k.to_string());
            let Some(loc) = temps.get(k) else {
                panic!("???");
            };
            loc
        };
        match loc {
            Location::Reg(reg) => format!("{}", reg),
            Location::Stack(s) => format!("[rsp + {}]", s),
        }
    }

    fn get_var_str(&self, v: &Operand, _vars: &Vars, temps: &TempVarTrack) -> String {
        match v {
            Operand::Immediate(val) => format!("{}", val),
            Operand::Variable(name) => format!("[{}]", name),
            Operand::Temp(name) => {
                let Some(loc) = temps.get(name) else {
                    panic!("temp referenced but not initialized: {}", name)
                };
                match loc {
                    Location::Reg(reg) => format!("{}", reg),
                    Location::Stack(s) => format!("[rsp + {}]", s),
                }
            }
        }
    }

    fn get_var_from_reg(&mut self, v: &Operand, _vars: &Vars, temps: &TempVarTrack) -> String {
        // assuming rax is usable
        match v {
            Operand::Immediate(val) => format!("{}", val),
            Operand::Variable(name) => {
                self.write_in_fn(format_args!("mov rax, qword [{}]", name));
                "rax".to_string()
            }
            Operand::Temp(name) => {
                let Some(loc) = temps.get(name) else {
                    panic!("temp referenced but not initialized: {}", name)
                };
                match loc {
                    Location::Reg(reg) => format!("{}", reg),
                    Location::Stack(s) => {
                        self.write_in_fn(format_args!("mov rax, [rsp + {}]", s));
                        "rax".to_string()
                    }
                }
            }
        }
    }

    fn write_op(&mut self, op: &Operation, rhs: &Operand, temps: &mut TempVarTrack, vars: &Vars) {
        // assuming lhs is in rax, leaves res in rax
        let op_str = match op {
            Operation::Mul => "imul",
            Operation::Sub => "sub",
            Operation::Add => "add",
            Operation::Div | Operation::Mod => {
                // sign extend RDX:RAX
                self.write_in_fn(format_args!("cqo"));
                if USAGE[Reg::RCX as usize].load(Ordering::Relaxed) {
                    self.write_in_fn(format_args!("push rcx"));
                    temps.inc_stack(8);
                }
                self.write_in_fn(format_args!(
                    "mov rcx, {}",
                    self.get_var_str(rhs, vars, temps)
                ));
                self.write_in_fn(format_args!("idiv rcx"));
                if *op == Operation::Mod {
                    self.write_in_fn(format_args!("mov rax, rdx"));
                }
                if USAGE[Reg::RCX as usize].load(Ordering::Relaxed) {
                    self.write_in_fn(format_args!("pop rcx"));
                    temps.dec_stack(8);
                }
                return;
            }
        };
        self.write_in_fn(format_args!(
            "{} rax, {}",
            op_str,
            self.get_var_str(rhs, vars, temps)
        ));
    }

    fn write_in_fn(&mut self, line: Arguments) {
        writeln!(self.fh, "\t{}", line).unwrap()
    }

    fn write_default_funcs(&mut self) {
        todo!()
    }
}

#[derive(Default, Debug)]
struct TempVarTrack {
    inner: HashMap<String, Location>,
    stack_pushes: usize,
}

impl TempVarTrack {
    fn inc_stack(&mut self, increase: usize) {
        self.stack_pushes += increase;
        for loc in self.inner.values_mut() {
            match loc {
                Location::Reg(_) => {}
                Location::Stack(pos) => *pos += increase,
            }
        }
    }

    fn dec_stack(&mut self, decrease: usize) {
        self.stack_pushes -= decrease;
        let mut drop = Vec::new();
        for (name, loc) in &mut self.inner {
            match loc {
                Location::Reg(_) => {}
                Location::Stack(pos) => {
                    if decrease > *pos {
                        drop.push(name.clone());
                    } else {
                        *pos -= decrease;
                    }
                }
            }
        }
        for should_drop in &drop {
            self.inner.remove(should_drop);
        }
    }

    fn add(&mut self, k: String) {
        for (i, is_used) in USAGE.iter().enumerate() {
            if !is_used.load(Ordering::Relaxed) {
                is_used.store(true, Ordering::Relaxed);
                self.inner.insert(k, Location::Reg(REGS[i]));
                return;
            }
        }
        self.inner.insert(k, Location::Stack(0));
    }

    fn get(&self, k: &str) -> Option<&Location> {
        self.inner.get(k)
    }

    fn drop(&mut self, k: &str) {
        if let Some(Location::Reg(r)) = self.inner.remove(k) {
            USAGE[r as usize].store(false, Ordering::Relaxed);
        }
    }

    fn stack_aligned(&self) -> bool {
        // assuming rel stack == 0 is aligned
        self.stack_pushes % 16 == 0
    }
}

// rax + rdx are used for calculations
#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(usize)]
enum Reg {
    R8,
    R9,
    R10,
    R11,
    RDI,
    RSI,
    RCX,
    RDX,
    RAX,
}

impl Display for Reg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::R8 => write!(f, "r8"),
            Self::R9 => write!(f, "r9"),
            Self::R10 => write!(f, "r10"),
            Self::R11 => write!(f, "r11"),
            Self::RDI => write!(f, "rdi"),
            Self::RSI => write!(f, "rsi"),
            Self::RCX => write!(f, "rcx"),
            Self::RDX => write!(f, "rdx"),
            Self::RAX => write!(f, "rax"),
        }
    }
}

const REGS: [Reg; 7] = [
    Reg::R8,
    Reg::R9,
    Reg::R10,
    Reg::R11,
    Reg::RDI,
    Reg::RSI,
    Reg::RCX,
];

static USAGE: [AtomicBool; 7] = [const { AtomicBool::new(false) }; 7];

#[derive(Default, Debug)]
struct Vars {
    inner: Vec<String>,
}

impl Vars {
    fn add(&mut self, var: String) {
        if !self.inner.contains(&var) {
            self.inner.push(var);
        }
    }
}

impl Display for Vars {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for var in &self.inner {
            writeln!(f, "\t{} resq 1", var)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
enum Location {
    Reg(Reg),     // in reg Reg.0
    Stack(usize), // on stack at rsp + Stack.0
}
