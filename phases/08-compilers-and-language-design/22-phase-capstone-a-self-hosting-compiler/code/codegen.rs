// codegen.rs — RISC-V (RV32I) code generator for pal IR
//
// Maps three-address IR instructions to RISC-V assembly.
// Uses a simple register allocation strategy: assign each temp/variable
// to a stack slot and load/store around each instruction.
// For a production compiler, you'd use graph-coloring register allocation.

use std::collections::HashMap;
use std::fmt::Write;
use crate::ir::*;

pub struct CodeGenerator {
    output: String,
    stack_offsets: HashMap<String, i32>,
    next_offset: i32,
}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {
            output: String::new(),
            stack_offsets: HashMap::new(),
            next_offset: -4,
        }
    }

    fn val_name(v: &IrVal) -> String {
        match v {
            IrVal::Const(n) => n.to_string(),
            IrVal::Var(s) => s.clone(),
            IrVal::Temp(t) => format!("t{}", t),
        }
    }

    fn get_offset(&mut self, name: &str) -> i32 {
        if let Some(&off) = self.stack_offsets.get(name) {
            off
        } else {
            let off = self.next_offset;
            self.next_offset -= 4;
            self.stack_offsets.insert(name.to_string(), off);
            off
        }
    }

    fn emit(&mut self, line: &str) {
        writeln!(&mut self.output, "{}", line).unwrap();
    }

    pub fn generate(instrs: &[IrInstr]) -> String {
        let mut gen = CodeGenerator::new();

        gen.emit("# Generated RISC-V assembly for pal");
        gen.emit(".text");
        gen.emit(".globl main");

        // Pre-compute all variable/temp names to allocate stack space
        for instr in instrs {
            match instr {
                IrInstr::Assign(dest, _) | IrInstr::BinOp(dest, _, _, _) => {
                    let name = CodeGenerator::val_name(dest);
                    gen.get_offset(&name);
                }
                _ => {}
            }
        }

        let frame_size = ((-gen.next_offset + 15) / 16) * 16;
        gen.emit(&format!("  addi sp, sp, -{}", frame_size));

        for instr in instrs {
            gen.gen_instr(instr, frame_size);
        }

        gen.emit(&format!("  addi sp, sp, {}", frame_size));
        gen.emit("  li a0, 0");
        gen.emit("  ret");

        gen.output
    }

    fn gen_instr(&mut self, instr: &IrInstr, frame_size: i32) {
        match instr {
            IrInstr::Label(name) => {
                self.emit(&format!("{}:", name));
            }
            IrInstr::Assign(dest, src) => {
                let d = Self::val_name(dest);
                let d_off = self.get_offset(&d);
                match src {
                    IrVal::Const(n) => {
                        self.emit(&format!("  li t0, {}", n));
                    }
                    IrVal::Var(s) | IrVal::Temp(s) => {
                        let s_off = self.get_offset(&s.to_string());
                        self.emit(&format!("  lw t0, {}(sp)", s_off));
                    }
                }
                self.emit(&format!("  sw t0, {}(sp)", d_off));
            }
            IrInstr::BinOp(dest, op, a, b) => {
                self.load_val("t0", a);
                self.load_val("t1", b);
                let d_off = self.get_offset(&Self::val_name(dest));
                let asm_op = match op {
                    IrOp::Add => "add",
                    IrOp::Sub => "sub",
                    IrOp::Mul => "mul",
                    IrOp::Div => "div",
                    IrOp::Eq => { self.emit("  sub t2, t0, t1"); "seqz t2, t2"; }
                    IrOp::Lt => "slt",
                    IrOp::Gt => { self.emit("  sub t2, t1, t0"); "slt t2, zero, t2"; "" }
                    IrOp::Le => { self.emit("  slt t2, t1, t0"); "xori t2, t2, 1"; "" }
                    IrOp::Ge => { self.emit("  slt t2, t0, t1"); "xori t2, t2, 1"; "" }
                };
                if !asm_op.is_empty() {
                    match op {
                        IrOp::Eq => self.emit(&format!("  {}", asm_op)),
                        IrOp::Gt | IrOp::Le | IrOp::Ge => self.emit(&format!("  {}", asm_op)),
                        _ => self.emit(&format!("  {} t2, t0, t1", asm_op)),
                    }
                }
                self.emit(&format!("  sw t2, {}(sp)", d_off));
            }
            IrInstr::IfGoto(val, label) => {
                self.load_val("t0", val);
                self.emit(&format!("  bnez t0, {}", label));
            }
            IrInstr::Goto(label) => {
                self.emit(&format!("  j {}", label));
            }
            IrInstr::Call(name, args, dest) => {
                for (i, arg) in args.iter().enumerate() {
                    if i < 8 {
                        self.load_val(&format!("a{}", i), arg);
                    }
                }
                self.emit(&format!("  call {}", name));
                let d_off = self.get_offset(&Self::val_name(dest));
                self.emit(&format!("  sw a0, {}(sp)", d_off));
            }
            IrInstr::Return(val) => {
                self.load_val("a0", val);
                self.emit(&format!("  addi sp, sp, {}", frame_size));
                self.emit("  ret");
            }
            IrInstr::Print(val) => {
                self.load_val("a0", val);
                self.emit("  # print syscall (ecall) would go here");
                self.emit("  # for now, just move to a0");
            }
        }
    }

    fn load_val(&mut self, reg: &str, val: &IrVal) {
        match val {
            IrVal::Const(n) => {
                self.emit(&format!("  li {}, {}", reg, n));
            }
            IrVal::Var(s) | IrVal::Temp(s) => {
                let off = self.get_offset(&s.to_string());
                self.emit(&format!("  lw {}, {}(sp)", reg, off));
            }
        }
    }
}
