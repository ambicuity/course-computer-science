// optimize.rs — Simple optimization passes for pal IR
//
// Pass 1: Constant folding — if both operands of a BinOp are constants,
//         compute the result at compile time.
// Pass 2: Dead code elimination — remove instructions whose results
//         are never used, and remove unreachable code after Goto.

use std::collections::HashSet;
use crate::ir::*;

pub fn optimize(instrs: Vec<IrInstr>) -> Vec<IrInstr> {
    let instrs = constant_fold(instrs);
    let instrs = dead_code_eliminate(instrs);
    instrs
}

// ---- Constant Folding ----

fn constant_fold(instrs: Vec<IrInstr>) -> Vec<IrInstr> {
    let mut result = Vec::new();
    for instr in instrs {
        match instr {
            IrInstr::BinOp(dest, op, IrVal::Const(a), IrVal::Const(b)) => {
                let val = match op {
                    IrOp::Add => a + b,
                    IrOp::Sub => a - b,
                    IrOp::Mul => a * b,
                    IrOp::Div => if b != 0 { a / b } else { 0 },
                    IrOp::Eq => if a == b { 1 } else { 0 },
                    IrOp::Lt => if a < b { 1 } else { 0 },
                    IrOp::Gt => if a > b { 1 } else { 0 },
                    IrOp::Le => if a <= b { 1 } else { 0 },
                    IrOp::Ge => if a >= b { 1 } else { 0 },
                };
                result.push(IrInstr::Assign(dest, IrVal::Const(val)));
            }
            other => result.push(other),
        }
    }
    result
}

// ---- Dead Code Elimination ----

fn find_used_temps(instrs: &[IrInstr]) -> HashSet<usize> {
    let mut used = HashSet::new();
    for instr in instrs {
        match instr {
            IrInstr::Assign(_, src) => { if let IrVal::Temp(t) = src { used.insert(*t); } }
            IrInstr::BinOp(_, _, a, b) => {
                if let IrVal::Temp(t) = a { used.insert(*t); }
                if let IrVal::Temp(t) = b { used.insert(*t); }
            }
            IrInstr::IfGoto(val, _) => {
                if let IrVal::Temp(t) = val { used.insert(*t); }
            }
            IrInstr::Call(_, args, _) => {
                for a in args {
                    if let IrVal::Temp(t) = a { used.insert(*t); }
                }
            }
            IrInstr::Return(val) => {
                if let IrVal::Temp(t) = val { used.insert(*t); }
            }
            IrInstr::Print(val) => {
                if let IrVal::Temp(t) = val { used.insert(*t); }
            }
            _ => {}
        }
    }
    used
}

fn dead_code_eliminate(instrs: Vec<IrInstr>) -> Vec<IrInstr> {
    let used = find_used_temps(&instrs);
    let mut result = Vec::new();
    let mut prev_was_goto = false;

    for instr in instrs {
        // Remove dead code after unconditional goto
        if prev_was_goto {
            match &instr {
                IrInstr::Label(_) => {
                    prev_was_goto = false;
                    result.push(instr);
                }
                _ => continue, // skip unreachable
            }
            continue;
        }

        match &instr {
            IrInstr::Goto(_) => {
                prev_was_goto = true;
                result.push(instr);
            }
            IrInstr::BinOp(IrVal::Temp(t), _, _, _) if !used.contains(t) => {
                // Dead: result never used
                continue;
            }
            IrInstr::Assign(IrVal::Temp(t), _) if !used.contains(t) => {
                continue;
            }
            _ => {
                prev_was_goto = false;
                result.push(instr);
            }
        }
    }

    result
}
