//! Classic Optimizations — DCE, CSE, Inlining, LICM
//! Phase 08 — Compilers & Programming Language Design
//!
//! Demonstrates five optimization passes on a simple IR.

use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// IR types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Value {
    Const(i64),
    Var(String),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Const(n) => write!(f, "{}", n),
            Value::Var(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone)]
enum Instr {
    BinOp { dst: String, op: String, lhs: Value, rhs: Value },
    Copy { dst: String, src: Value },
    Print(Value),
    Phi { dst: String, values: Vec<Value> },
}

impl std::fmt::Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instr::BinOp { dst, op, lhs, rhs } => {
                write!(f, "{} = {} {} {}", dst, lhs, op, rhs)
            }
            Instr::Copy { dst, src } => write!(f, "{} = {}", dst, src),
            Instr::Print(v) => write!(f, "print({})", v),
            Instr::Phi { dst, values } => {
                let vs: Vec<String> = values.iter().map(|v| v.to_string()).collect();
                write!(f, "{} = φ({})", dst, vs.join(", "))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pretty-print
// ---------------------------------------------------------------------------

fn print_instructions(title: &str, instrs: &[Instr]) {
    println!("--- {} ---", title);
    for (i, instr) in instrs.iter().enumerate() {
        println!("  {:2}: {}", i, instr);
    }
    println!();
}

// ===========================================================================
// Pass 1: Dead Code Elimination
// ===========================================================================

fn dead_code_elimination(instrs: &[Instr]) -> Vec<Instr> {
    // Build use-def. An instruction's dst is "live" if it's used by a live
    // instruction or is a Print operand (observable).
    let mut is_live = vec![false; instrs.len()];

    // First pass: mark Print and side-effecting instructions as live.
    for (i, instr) in instrs.iter().enumerate() {
        if matches!(instr, Instr::Print(_)) {
            is_live[i] = true;
        }
    }

    // Iterative live marking: if instruction i is live, all instructions
    // that define its operands are also live.
    let mut changed = true;
    while changed {
        changed = false;
        for i in (0..instrs.len()).rev() {
            if !is_live[i] {
                continue;
            }
            let used_vars = used_values(&instrs[i]);
            for v in &used_vars {
                if let Value::Var(name) = v {
                    // Find the defining instruction
                    for j in 0..instrs.len() {
                        if def_of(&instrs[j]) == Some(name.as_str()) && !is_live[j] {
                            is_live[j] = true;
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    // Collect live instructions
    instrs
        .iter()
        .enumerate()
        .filter(|(i, _)| is_live[*i])
        .map(|(_, instr)| instr.clone())
        .collect()
}

fn used_values(instr: &Instr) -> Vec<Value> {
    match instr {
        Instr::BinOp { lhs, rhs, .. } => vec![lhs.clone(), rhs.clone()],
        Instr::Copy { src, .. } => vec![src.clone()],
        Instr::Print(v) => vec![v.clone()],
        Instr::Phi { values, .. } => values.clone(),
    }
}

fn def_of(instr: &Instr) -> Option<&str> {
    match instr {
        Instr::BinOp { dst, .. } | Instr::Copy { dst, .. } | Instr::Phi { dst, .. } => Some(dst),
        Instr::Print(_) => None,
    }
}

// ===========================================================================
// Pass 2: Common Subexpression Elimination
// ===========================================================================

fn cse(instrs: &[Instr]) -> Vec<Instr> {
    let mut expr_table: HashMap<(String, Value, Value), String> = HashMap::new();
    let mut result = Vec::new();

    for instr in instrs {
        match instr {
            Instr::BinOp { dst, op, lhs, rhs } => {
                let key = (op.clone(), lhs.clone(), rhs.clone());
                if let Some(existing) = expr_table.get(&key) {
                    // Replace with copy from existing result
                    result.push(Instr::Copy {
                        dst: dst.clone(),
                        src: Value::Var(existing.clone()),
                    });
                } else {
                    expr_table.insert(key, dst.clone());
                    result.push(instr.clone());
                }
            }
            _ => result.push(instr.clone()),
        }
    }
    result
}

// ===========================================================================
// Pass 3: Constant Folding + Propagation
// ===========================================================================

fn constant_folding(instrs: &[Instr]) -> Vec<Instr> {
    let mut const_env: HashMap<String, i64> = HashMap::new();
    let mut result = Vec::new();

    for instr in instrs {
        match instr {
            Instr::BinOp { dst, op, lhs, rhs } => {
                let l = eval_value(lhs, &const_env);
                let r = eval_value(rhs, &const_env);

                match (l, r, op.as_str()) {
                    (Some(a), Some(b), "+") => {
                        let val = a + b;
                        const_env.insert(dst.clone(), val);
                        result.push(Instr::Copy {
                            dst: dst.clone(),
                            src: Value::Const(val),
                        });
                    }
                    (Some(a), Some(b), "-") => {
                        let val = a - b;
                        const_env.insert(dst.clone(), val);
                        result.push(Instr::Copy {
                            dst: dst.clone(),
                            src: Value::Const(val),
                        });
                    }
                    (Some(a), Some(b), "*") => {
                        let val = a * b;
                        const_env.insert(dst.clone(), val);
                        result.push(Instr::Copy {
                            dst: dst.clone(),
                            src: Value::Const(val),
                        });
                    }
                    _ => result.push(instr.clone()),
                }
            }
            Instr::Copy { dst, src } => {
                if let Value::Const(n) = src {
                    const_env.insert(dst.clone(), *n);
                }
                result.push(instr.clone());
            }
            _ => result.push(instr.clone()),
        }
    }
    result
}

fn eval_value(v: &Value, env: &HashMap<String, i64>) -> Option<i64> {
    match v {
        Value::Const(n) => Some(*n),
        Value::Var(s) => env.get(s).copied(),
    }
}

// ===========================================================================
// Pass 4: Inlining
// ===========================================================================

#[derive(Debug, Clone)]
struct FunctionDef {
    name: String,
    params: Vec<String>,
    body: Vec<Instr>,
}

/// Inline a simple function call. Returns the body with parameters
/// substituted and the return value stored in `result_var`.
fn inline_call(
    func: &FunctionDef,
    args: &[Value],
    result_var: &str,
) -> Vec<Instr> {
    let mut result = Vec::new();

    // Build substitution map: param → arg
    let mut subst: HashMap<String, Value> = HashMap::new();
    for (param, arg) in func.params.iter().zip(args.iter()) {
        subst.insert(param.clone(), arg.clone());
    }

    let mut counter = 0;
    for instr in &func.body {
        let new_instr = match instr {
            Instr::BinOp { dst, op, lhs, rhs } => {
                let fresh_dst = format!("{}_inl{}", dst, counter);
                counter += 1;
                Instr::BinOp {
                    dst: fresh_dst,
                    op: op.clone(),
                    lhs: subst_value(lhs, &subst),
                    rhs: subst_value(rhs, &subst),
                }
            }
            Instr::Copy { dst, src } => {
                let fresh_dst = format!("{}_inl{}", dst, counter);
                counter += 1;
                Instr::Copy {
                    dst: fresh_dst,
                    src: subst_value(src, &subst),
                }
            }
            other => other.clone(),
        };
        result.push(new_instr);
    }

    // Last instruction's dst is the return value → copy to result_var
    if let Some(last) = result.last() {
        let ret_val = match last {
            Instr::BinOp { dst, .. } | Instr::Copy { dst, .. } => Value::Var(dst.clone()),
            _ => Value::Const(0),
        };
        result.push(Instr::Copy {
            dst: result_var.to_string(),
            src: ret_val,
        });
    }

    result
}

fn subst_value(v: &Value, subst: &HashMap<String, Value>) -> Value {
    match v {
        Value::Var(s) => subst.get(s).cloned().unwrap_or_else(|| v.clone()),
        _ => v.clone(),
    }
}

// ===========================================================================
// Pass 5: Loop-Invariant Code Motion (simplified)
// ===========================================================================

/// Given a list of loop-body instructions, identify loop-invariant ones
/// (all operands defined outside the loop or by already-hoisted instructions)
/// and hoist them before the loop.
fn licm(loop_body: &[Instr], defined_outside: &HashSet<String>) -> (Vec<Instr>, Vec<Instr>) {
    let mut hoisted = Vec::new();
    let mut remaining = Vec::new();
    let mut invariant_defs: HashSet<String> = defined_outside.clone();

    let mut changed = true;
    while changed {
        changed = false;
        let mut new_remaining = Vec::new();
        for instr in &loop_body.clone() {
            let dominated_vars = used_values(instr);
            let all_invariant = dominated_vars.iter().all(|v| match v {
                Value::Const(_) => true,
                Value::Var(s) => invariant_defs.contains(s.as_str()),
            });

            if all_invariant && def_of(instr).is_some() {
                // Can hoist — but only if it's not a phi
                if !matches!(instr, Instr::Phi { .. }) {
                    hoisted.push(instr.clone());
                    if let Some(d) = def_of(instr) {
                        invariant_defs.insert(d.to_string());
                    }
                    changed = true;
                    continue;
                }
            }
            new_remaining.push(instr.clone());
        }
        remaining = new_remaining;
    }

    (hoisted, remaining)
}

// ===========================================================================
// Optimizer runner
// ===========================================================================

enum Pass {
    DCE,
    CSE,
    ConstantFolding,
}

fn optimize(instrs: &[Instr], passes: &[Pass]) -> Vec<Instr> {
    let mut current = instrs.to_vec();
    for pass in passes {
        current = match pass {
            Pass::DCE => dead_code_elimination(&current),
            Pass::CSE => cse(&current),
            Pass::ConstantFolding => constant_folding(&current),
        };
    }
    current
}

// ===========================================================================
// Demos
// ===========================================================================

fn demo_dce() {
    println!("=== Dead Code Elimination ===\n");
    let instrs = vec![
        Instr::BinOp { dst: "t1".into(), op: "+".into(), lhs: Value::Var("a".into()), rhs: Value::Var("b".into()) },
        Instr::BinOp { dst: "t2".into(), op: "*".into(), lhs: Value::Var("t1".into()), rhs: Value::Const(2) },
        Instr::BinOp { dst: "t3".into(), op: "+".into(), lhs: Value::Var("t2".into()), rhs: Value::Const(1) }, // dead
        Instr::Copy { dst: "w".into(), src: Value::Const(99) }, // dead
        Instr::Print(Value::Var("t2".into())),
    ];
    print_instructions("Before DCE", &instrs);
    let optimized = dead_code_elimination(&instrs);
    print_instructions("After DCE", &optimized);
}

fn demo_cse() {
    println!("=== Common Subexpression Elimination ===\n");
    let instrs = vec![
        Instr::BinOp { dst: "t1".into(), op: "+".into(), lhs: Value::Var("a".into()), rhs: Value::Var("b".into()) },
        Instr::Print(Value::Var("t1".into())),
        Instr::BinOp { dst: "t2".into(), op: "+".into(), lhs: Value::Var("a".into()), rhs: Value::Var("b".into()) }, // duplicate
        Instr::Print(Value::Var("t2".into())),
    ];
    print_instructions("Before CSE", &instrs);
    let optimized = cse(&instrs);
    print_instructions("After CSE", &optimized);
}

fn demo_constant_folding() {
    println!("=== Constant Folding + Propagation ===\n");
    let instrs = vec![
        Instr::BinOp { dst: "x".into(), op: "+".into(), lhs: Value::Const(3), rhs: Value::Const(4) },
        Instr::BinOp { dst: "y".into(), op: "*".into(), lhs: Value::Var("x".into()), rhs: Value::Const(2) },
        Instr::Print(Value::Var("y".into())),
    ];
    print_instructions("Before Constant Folding", &instrs);
    let optimized = constant_folding(&instrs);
    print_instructions("After Constant Folding", &optimized);
}

fn demo_inlining() {
    println!("=== Function Inlining ===\n");
    let add_func = FunctionDef {
        name: "add".into(),
        params: vec!["a".into(), "b".into()],
        body: vec![
            Instr::BinOp { dst: "ret".into(), op: "+".into(), lhs: Value::Var("a".into()), rhs: Value::Var("b".into()) },
        ],
    };

    println!("Function: add(a, b) -> ret = a + b");
    println!("Call: x = add(5, 3)\n");
    let inlined = inline_call(&add_func, &[Value::Const(5), Value::Const(3)], "x");
    print_instructions("After Inlining", &inlined);
}

fn demo_licm() {
    println!("=== Loop-Invariant Code Motion ===\n");
    let defined_outside: HashSet<String> = ["a", "b", "n"].iter().map(|s| s.to_string()).collect();

    let loop_body = vec![
        Instr::BinOp { dst: "t".into(), op: "*".into(), lhs: Value::Var("a".into()), rhs: Value::Var("b".into()) }, // invariant
        Instr::BinOp { dst: "idx".into(), op: "+".into(), lhs: Value::Var("i".into()), rhs: Value::Const(0) },        // uses loop var
        Instr::BinOp { dst: "v".into(), op: "+".into(), lhs: Value::Var("t".into()), rhs: Value::Var("idx".into()) },
        Instr::Print(Value::Var("v".into())),
    ];

    print_instructions("Before LICM (loop body)", &loop_body);
    let (hoisted, remaining) = licm(&loop_body, &defined_outside);
    print_instructions("Hoisted (before loop)", &hoisted);
    print_instructions("Remaining (in loop)", &remaining);
}

fn demo_full_pipeline() {
    println!("=== Full Optimization Pipeline ===\n");
    let instrs = vec![
        Instr::BinOp { dst: "x".into(), op: "+".into(), lhs: Value::Const(3), rhs: Value::Const(4) },
        Instr::BinOp { dst: "y".into(), op: "*".into(), lhs: Value::Var("x".into()), rhs: Value::Const(2) },
        Instr::BinOp { dst: "z".into(), op: "+".into(), lhs: Value::Var("a".into()), rhs: Value::Var("b".into()) },
        Instr::BinOp { dst: "w".into(), op: "+".into(), lhs: Value::Var("a".into()), rhs: Value::Var("b".into()) }, // CSE
        Instr::BinOp { dst: "dead".into(), op: "+".into(), lhs: Value::Const(1), rhs: Value::Const(2) },
        Instr::Print(Value::Var("y".into())),
        Instr::Print(Value::Var("w".into())),
    ];

    print_instructions("Original", &instrs);
    let optimized = optimize(&instrs, &[Pass::ConstantFolding, Pass::CSE, Pass::DCE]);
    print_instructions("After CF → CSE → DCE", &optimized);
}

fn main() {
    println!("Classic Optimizations — DCE, CSE, Inlining, LICM");
    println!("=================================================\n");

    demo_dce();
    demo_cse();
    demo_constant_folding();
    demo_inlining();
    demo_licm();
    demo_full_pipeline();
}
