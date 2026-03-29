use std::{
    collections::HashMap,
    fmt::{Arguments, Display},
    fs::File,
    io::Write,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    backend::codegen::LValue,
    frontend::ast::{Operation, is_builtin_func},
    print_if,
};

use super::{CodeTree, CodeUnit, Operand};

pub struct AsmWriter {
    fh: File,
}

impl AsmWriter {
    pub fn new(path: &Path, code: &CodeTree) -> Self {
        let mut file = File::create(path).unwrap();
        print_if!(1, "writing asm code to {}", path.display());

        writeln!(
            file,
            "section .data\n\tformat_int: db \"%ld\", 0\n\tformat_str: db \"%s\", 0\n\tdefault rel"
        )
        .unwrap();

        for (payload, ident) in code.data.iter() {
            writeln!(file, "\t{}: db `{}`, 0", ident, payload.write_data()).unwrap();
        }

        writeln!(
            file,
            "\nsection .text\n\tglobal main\n\textern printf\n\textern exit\n"
        )
        .unwrap();

        Self { fh: file }
    }

    pub fn write(mut self, code: &CodeTree) {
        writeln!(self.fh, "main:").unwrap();
        // ensure stack is 16-byte aligned. It is currently misaligned due to the C-runtime calling main
        self.write_in_fn(format_args!("sub rsp, 8"));
        let mut all_vars = Vars::default();
        let mut temps = TempVarStack::default();

        let stack_size_at_last_cleanup = temps.stack_pushes;

        for unit in &code.units {
            self.write_unit(unit, &mut all_vars, &mut temps);

            if let CodeUnit::Cleanup = unit {
                let leaked_bytes = temps.stack_pushes - stack_size_at_last_cleanup;
                if leaked_bytes > 0 {
                    print_if!(
                        4,
                        "memory leak of {} bytes detected in {:?}, cleaning up",
                        leaked_bytes,
                        unit
                    );
                    self.write_in_fn(format_args!("add rsp, {}", leaked_bytes));
                }

                temps.stack_pushes = stack_size_at_last_cleanup;
                temps.inner.clear();
                for usage in USAGE.iter() {
                    usage.store(false, Ordering::Relaxed);
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

    fn write_unit(&mut self, unit: &CodeUnit, all_vars: &mut Vars, temps: &mut TempVarStack) {
        match unit {
            CodeUnit::FuncCall { name, args } => {
                // for now assuming a single argument
                // TODO move temp in rdi also (mutate temps accordingly)
                // currently not necessary, as no native funcs exist

                let func_name = self.get_func_name(name);
                if is_builtin_func(func_name) {
                    self.call_builtin(name, args, all_vars, temps);
                } else {
                    if USAGE[Reg::RDI as usize].load(Ordering::Relaxed) {
                        self.write_in_fn(format_args!("push rdi"));
                        temps.inc_stack(8);
                    }
                    for op in args.iter() {
                        //TODO mov in other regs depending on i
                        self.write_in_fn(format_args!(
                            "mov rdi, {}",
                            self.get_var_str(op, all_vars, temps)
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
                            self.get_var_str(lhs, all_vars, temps)
                        ));
                        self.write_op(op, rhs, temps, all_vars);

                        self.write_in_fn(format_args!("mov qword [{}], rax", name));
                    }
                    Operand::Temp(name) => {
                        self.write_in_fn(format_args!(
                            "mov rax, {}",
                            self.get_var_str(lhs, all_vars, temps)
                        ));
                        self.write_op(op, rhs, temps, all_vars);

                        let addr = self.get_or_init_temp(name, temps);
                        self.write_in_fn(format_args!("mov {}, rax", addr));
                    }
                    _ => panic!("cannot assign to rvalue"),
                };
            }
            CodeUnit::Assignment { name, value } => {
                let save_rcx = if let LValue::Variable(var_name) = name {
                    all_vars.add(var_name.clone());
                    true
                } else {
                    false
                } && USAGE[Reg::RCX as usize].load(Ordering::Relaxed);

                let rhs = self.get_var_from_reg(value, all_vars, temps);
                self.write_in_fn(format_args!("mov rax, {}", rhs));

                if save_rcx {
                    self.write_in_fn(format_args!("push rcx"));
                    temps.inc_stack(8);
                }

                let resolved = self.resolve_lvalue(name);
                self.write_in_fn(format_args!("mov qword [{}], rax", resolved));

                if save_rcx {
                    self.write_in_fn(format_args!("pop rcx"));
                    temps.dec_stack(8);
                }
            }
            CodeUnit::Condition { eval, then, label } => {
                let eval_position = self.get_var_from_reg(eval, all_vars, temps);
                self.write_in_fn(format_args!("test {}, {}", eval_position, eval_position));
                self.write_in_fn(format_args!("jz {}", label));
                for unit in then {
                    self.write_unit(unit, all_vars, temps);
                }
                writeln!(self.fh, "{}:", label).unwrap();
            }
            CodeUnit::Cleanup => {}
        }
    }

    /// assume rcx unused
    fn resolve_lvalue(&mut self, value: &LValue) -> String {
        match value {
            LValue::Variable(var) => var.clone(),
            LValue::Deref(lvalue) => {
                let inner = self.resolve_lvalue(lvalue);
                self.write_in_fn(format_args!("mov rcx, [{}]", inner));
                "rcx".into()
            }
        }
    }

    fn get_func_name<'a>(&self, name: &'a str) -> &'a str {
        match name {
            "print" => "printf",
            "print_str" => "printf",
            _ => name,
        }
    }

    fn call_builtin(
        &mut self,
        name: &str,
        args: &[Operand],
        vars: &Vars,
        temps: &mut TempVarStack,
    ) {
        match name {
            "print" => {
                self.builtin_print(args, vars, temps, "format_int");
            }
            "print_str" => {
                self.builtin_print(args, vars, temps, "format_str");
            }
            "exit" => {
                if !temps.stack_aligned() {
                    self.write_in_fn(format_args!("sub rsp, 8"));
                    temps.inc_stack(8);
                }
                self.write_in_fn(format_args!("mov edi, 0"));
                self.write_in_fn(format_args!("call exit"));
            }
            "goto" => self.write_in_fn(format_args!("jmp {}", args[0])),
            "label" => writeln!(self.fh, "{}:", args[0]).unwrap(),
            "sqrt" => {
                // assuming we get a ptr to some integer value
                // result is written back to pointer
                let rcx_usage = USAGE[Reg::RCX as usize].load(Ordering::Relaxed);
                if rcx_usage {
                    self.write_in_fn(format_args!("push rcx"));
                    temps.inc_stack(8);
                }

                self.write_in_fn(format_args!(
                    "mov rcx, {}",
                    self.get_var_str(&args[0], vars, temps)
                ));
                self.write_in_fn(format_args!("mov rax, [rcx]"));

                self.write_in_fn(format_args!("cvtsi2sd xmm0, rax"));
                self.write_in_fn(format_args!("sqrtsd xmm0, xmm0"));
                self.write_in_fn(format_args!("cvttsd2si rax, xmm0"));

                self.write_in_fn(format_args!("mov [rcx], rax"));

                if rcx_usage {
                    self.write_in_fn(format_args!("pop rcx"));
                    temps.dec_stack(8);
                }
            }

            _ => {}
        }
    }

    fn builtin_print(
        &mut self,
        args: &[Operand],
        vars: &Vars,
        temps: &mut TempVarStack,
        formatter: &str,
    ) {
        // TODO should also modify temps accordingly
        if USAGE[Reg::RSI as usize].load(Ordering::Relaxed) {
            self.write_in_fn(format_args!("push rsi"));
            temps.inc_stack(8);
        }
        if USAGE[Reg::RDI as usize].load(Ordering::Relaxed) {
            self.write_in_fn(format_args!("push rdi"));
            temps.inc_stack(8);
        }
        let needs_align = !temps.stack_aligned();
        if needs_align {
            self.write_in_fn(format_args!("sub rsp, 8"));
            temps.inc_stack(8);
        }
        self.write_in_fn(format_args!(
            "mov rsi, {}",
            self.get_var_str(&args[0], vars, temps)
        ));
        self.write_in_fn(format_args!("mov rdi, {}", formatter));
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
        if needs_align {
            self.write_in_fn(format_args!("add rsp, 8"));
            temps.dec_stack(8);
        }
    }

    fn get_or_init_temp(&mut self, k: &str, temps: &mut TempVarStack) -> String {
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

    /// returns the location of the operand
    fn get_var_str(&self, v: &Operand, _vars: &Vars, temps: &TempVarStack) -> String {
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

    /// returns the location of the operand. This location will be a register
    fn get_var_from_reg(&mut self, v: &Operand, _vars: &Vars, temps: &TempVarStack) -> String {
        // assuming rax is usable
        match v {
            Operand::Immediate(val) => {
                self.write_in_fn(format_args!("mov rax, {}", val));
                "rax".to_string()
            }
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

    fn write_op(&mut self, op: &Operation, rhs: &Operand, temps: &mut TempVarStack, vars: &Vars) {
        // assuming lhs is in rax, leaves res in rax
        let op_str = match op {
            Operation::Mul => "imul",
            Operation::Sub => "sub",
            Operation::Add => "add",
            Operation::BitAND => "and",
            Operation::BitOR => "or",
            Operation::BitXOR => "xor",
            Operation::Not => {
                // ignore lhs
                self.write_in_fn(format_args!(
                    "mov rax, {}",
                    self.get_var_str(rhs, vars, temps)
                ));
                self.write_in_fn(format_args!("test rax, rax"));
                self.write_in_fn(format_args!("sete al"));
                self.write_in_fn(format_args!("movzx rax, al"));
                return;
            }
            Operation::Gt => {
                self.write_in_fn(format_args!(
                    "cmp rax, {}",
                    self.get_var_str(rhs, vars, temps)
                ));
                self.write_in_fn(format_args!("setg al"));
                self.write_in_fn(format_args!("movzx rax, al"));
                return;
            }
            Operation::Lt => {
                self.write_in_fn(format_args!(
                    "cmp rax, {}",
                    self.get_var_str(rhs, vars, temps)
                ));
                self.write_in_fn(format_args!("setl al"));
                self.write_in_fn(format_args!("movzx rax, al"));
                return;
            }
            Operation::EqEq => {
                self.write_in_fn(format_args!(
                    "cmp rax, {}",
                    self.get_var_str(rhs, vars, temps)
                ));
                self.write_in_fn(format_args!("sete al"));
                self.write_in_fn(format_args!("movzx rax, al"));
                return;
            }
            Operation::Load => {
                let var_location = self.get_var_str(rhs, vars, temps);
                // double deref, as var_location may be a ptr
                self.write_in_fn(format_args!("mov rax, {}", var_location));
                self.write_in_fn(format_args!("mov rax, [rax]"));
                return;
            }
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
            Operation::Shr => {
                let rcx_usage = USAGE[Reg::RCX as usize].load(Ordering::Relaxed);
                if rcx_usage {
                    self.write_in_fn(format_args!("push rcx"));
                    temps.inc_stack(8);
                }

                self.write_in_fn(format_args!(
                    "mov rcx, {}",
                    self.get_var_str(rhs, vars, temps)
                ));
                self.write_in_fn(format_args!("shr rax, cl"));

                if rcx_usage {
                    self.write_in_fn(format_args!("pop rcx"));
                    temps.dec_stack(8);
                }
                return;
            }
            Operation::Shl => {
                let rcx_usage = USAGE[Reg::RCX as usize].load(Ordering::Relaxed);
                if rcx_usage {
                    self.write_in_fn(format_args!("push rcx"));
                    temps.inc_stack(8);
                }

                self.write_in_fn(format_args!(
                    "mov rcx, {}",
                    self.get_var_str(rhs, vars, temps)
                ));
                self.write_in_fn(format_args!("shl rax, cl"));

                if rcx_usage {
                    self.write_in_fn(format_args!("pop rcx"));
                    temps.dec_stack(8);
                }
                return;
            }
            Operation::AsRef => {
                let addr = match rhs {
                    Operand::Immediate(val) => {
                        self.write_in_fn(format_args!("sub rsp, 8"));
                        self.write_in_fn(format_args!("mov qword [rsp], {}", val));
                        temps.inc_stack(8);
                        "rsp"
                    }
                    Operand::Variable(var) => var,
                    Operand::Temp(name) => {
                        let Some(loc) = temps.get(name) else {
                            panic!("temp referenced but not initialized: {}", name)
                        };
                        match loc {
                            Location::Reg(reg) => {
                                self.write_in_fn(format_args!("push {}", reg));
                                temps.inc_stack(8);
                                "rsp"
                            }
                            Location::Stack(s) => &format!("rsp + {}", s),
                        }
                    }
                };
                self.write_in_fn(format_args!("lea rax, [{}]", addr));
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
}

#[derive(Default, Debug)]
struct TempVarStack {
    inner: HashMap<String, Location>,
    stack_pushes: usize,
}

impl TempVarStack {
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

    #[allow(unused)]
    fn drop(&mut self, k: &str) {
        if let Some(Location::Reg(r)) = self.inner.remove(k) {
            USAGE[r as usize].store(false, Ordering::Relaxed);
        }
    }

    fn stack_aligned(&self) -> bool {
        // assuming rel stack == 0 is aligned
        #[allow(clippy::manual_is_multiple_of)]
        {
            self.stack_pushes % 16 == 0
        }
    }
}

// rax + rdx are used for calculations
#[allow(clippy::upper_case_acronyms, unused)]
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
