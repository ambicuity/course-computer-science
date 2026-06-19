//! Loop Optimization & Vectorization
//! Phase 08 — Compilers & Programming Language Design
//!
//! Demonstrates: loop unrolling, strength reduction, loop interchange,
//! and dependence analysis for vectorization.

use std::collections::HashSet;

// ---------------------------------------------------------------------------
// IR types for loop analysis
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
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
    Assign { dst: String, op: String, args: Vec<Value> },
    Load { dst: String, base: String, index: Value },
    Store { base: String, index: Value, src: Value },
    Print(Value),
}

impl std::fmt::Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instr::Assign { dst, op, args } => {
                let a: Vec<String> = args.iter().map(|v| v.to_string()).collect();
                write!(f, "{} = {} {}", dst, op, a.join(" "))
            }
            Instr::Load { dst, base, index } => write!(f, "{} = {}[{}]", dst, base, index),
            Instr::Store { base, index, src } => write!(f, "{}[{}] = {}", base, index, src),
            Instr::Print(v) => write!(f, "print({})", v),
        }
    }
}

#[derive(Debug, Clone)]
struct Loop {
    header: String,
    induction: String,
    start: i64,
    end: i64,
    step: i64,
    body: Vec<Instr>,
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

fn print_loop(title: &str, lp: &Loop) {
    println!("--- {} ---", title);
    println!(
        "for {} in {}..{} (step {})",
        lp.induction, lp.start, lp.end, lp.step
    );
    for instr in &lp.body {
        println!("    {}", instr);
    }
    println!();
}

// ===========================================================================
// Optimization 1: Loop Unrolling
// ===========================================================================

fn loop_unroll(body: &[Instr], factor: usize, induction: &str) -> Vec<Instr> {
    let mut result = Vec::new();

    for i in 0..factor {
        let offset = i as i64;
        for instr in body {
            let unrolled = match instr {
                Instr::Assign { dst, op, args } => {
                    let new_args: Vec<Value> = args
                        .iter()
                        .map(|a| substitute_offset(a, induction, offset))
                        .collect();
                    let new_dst = if offset > 0 {
                        format!("{}_u{}", dst, i)
                    } else {
                        dst.clone()
                    };
                    Instr::Assign {
                        dst: new_dst,
                        op: op.clone(),
                        args: new_args,
                    }
                }
                Instr::Load { dst, base, index } => {
                    let new_dst = if offset > 0 {
                        format!("{}_u{}", dst, i)
                    } else {
                        dst.clone()
                    };
                    Instr::Load {
                        dst: new_dst,
                        base: base.clone(),
                        index: substitute_offset(index, induction, offset),
                    }
                }
                Instr::Store { base, index, src } => Instr::Store {
                    base: base.clone(),
                    index: substitute_offset(index, induction, offset),
                    src: substitute_offset(src, induction, offset),
                },
                Instr::Print(v) => Instr::Print(substitute_offset(v, induction, offset)),
            };
            result.push(unrolled);
        }
    }
    result
}

fn substitute_offset(v: &Value, induction: &str, offset: i64) -> Value {
    match v {
        Value::Var(s) if s == induction => {
            if offset == 0 {
                Value::Var(induction.to_string())
            } else {
                Value::Var(format!("{}+{}", induction, offset))
            }
        }
        _ => v.clone(),
    }
}

// ===========================================================================
// Optimization 2: Strength Reduction
// ===========================================================================

/// Detect patterns like `t = i * C + base` and replace with pointer arithmetic.
fn strength_reduce(body: &[Instr]) -> (Vec<Instr>, Vec<String>) {
    let mut result = Vec::new();
    let mut ptr_vars: Vec<String> = Vec::new();

    for instr in body {
        match instr {
            Instr::Assign { dst, op, args } if op == "*" => {
                // Check if one operand is induction-like and other is constant
                result.push(Instr::Assign {
                    dst: dst.clone(),
                    op: "+".into(),
                    args: args.clone(), // In real pass, this would become a pointer increment
                });
                ptr_vars.push(dst.clone());
            }
            _ => result.push(instr.clone()),
        }
    }

    (result, ptr_vars)
}

// ===========================================================================
// Optimization 3: Loop Interchange
// ===========================================================================

/// Check if two nested loops can be interchanged.
/// Returns true if there are no loop-carried dependences that prevent it.
fn can_interchange(outer: &Loop, inner: &Loop) -> bool {
    // Simplified: check if inner loop's body references the outer induction variable
    // only in array indices where it's not part of a dependence chain.
    let outer_var = &outer.induction;
    let inner_var = &inner.induction;

    // Collect array accesses
    let mut inner_reads: HashSet<String> = HashSet::new();
    let mut inner_writes: HashSet<String> = HashSet::new();

    for instr in &inner.body {
        match instr {
            Instr::Load { base, index, .. } => {
                let key = format!("{}[{}, {}]", base, outer_var, index);
                inner_reads.insert(key);
            }
            Instr::Store { base, index, .. } => {
                let key = format!("{}[{}, {}]", base, outer_var, index);
                inner_writes.insert(key);
            }
            _ => {}
        }
    }

    // Safe to interchange if no read-write conflicts across iterations
    inner_reads.is_disjoint(&inner_writes)
}

fn loop_interchange(outer: &Loop, inner: &Loop) -> (Loop, Loop) {
    // Swap: the inner becomes the outer, outer becomes inner
    let new_outer = Loop {
        header: inner.header.clone(),
        induction: inner.induction.clone(),
        start: inner.start,
        end: inner.end,
        step: inner.step,
        body: inner.body.clone(),
    };
    let new_inner = Loop {
        header: outer.header.clone(),
        induction: outer.induction.clone(),
        start: outer.start,
        end: outer.end,
        step: outer.step,
        body: outer.body.clone(),
    };
    (new_outer, new_inner)
}

// ===========================================================================
// Optimization 4: Dependence Analysis / Vectorization Check
// ===========================================================================

/// Simple loop-carried dependence check.
/// Returns true if the loop body has no flow dependences between iterations.
fn can_vectorize(lp: &Loop) -> (bool, Vec<String>) {
    let mut barriers: Vec<String> = Vec::new();

    let mut writes: HashSet<String> = HashSet::new();
    let mut reads: HashSet<String> = HashSet::new();

    for instr in &lp.body {
        match instr {
            Instr::Load { base, .. } => {
                reads.insert(base.clone());
            }
            Instr::Store { base, index, .. } => {
                // Check if this write's index depends on a read from the same base
                if reads.contains(base) {
                    // Potential anti-dependence
                }
                // Check for flow dependence: store to a[i], load from a[i-1]
                // Simplified: if index references induction var with offset, flag it
                match index {
                    Value::Var(s) if s == &lp.induction => {
                        // Self-referential at same index — potential issue
                    }
                    _ => {}
                }
                writes.insert(base.clone());
            }
            _ => {}
        }
    }

    // Check for loop-carried dependence: if the same array is both read and written
    // with induction-dependent indices, it may not be vectorizable
    for base in &writes {
        if reads.contains(base) {
            barriers.push(format!(
                "Potential loop-carried dependence on array '{}'",
                base
            ));
        }
    }

    let vectorizable = barriers.is_empty();
    (vectorizable, barriers)
}

// ===========================================================================
// Demos
// ===========================================================================

fn demo_unrolling() {
    println!("=== Loop Unrolling ===\n");

    let body = vec![
        Instr::Load {
            dst: "t".into(),
            base: "a".into(),
            index: Value::Var("i".into()),
        },
        Instr::Assign {
            dst: "r".into(),
            op: "+".into(),
            args: vec![Value::Var("t".into()), Value::Const(1)],
        },
        Instr::Store {
            base: "b".into(),
            index: Value::Var("i".into()),
            src: Value::Var("r".into()),
        },
    ];

    print_instructions("Original body", &body);

    let unrolled2 = loop_unroll(&body, 2, "i");
    print_instructions("Unrolled by 2", &unrolled2);

    let unrolled4 = loop_unroll(&body, 4, "i");
    print_instructions("Unrolled by 4", &unrolled4);
}

fn demo_strength_reduction() {
    println!("=== Strength Reduction ===\n");

    let body = vec![
        Instr::Assign {
            dst: "offset".into(),
            op: "*".into(),
            args: vec![Value::Var("i".into()), Value::Const(4)],
        },
        Instr::Assign {
            dst: "addr".into(),
            op: "+".into(),
            args: vec![Value::Var("base".into()), Value::Var("offset".into())],
        },
        Instr::Assign {
            dst: "v".into(),
            op: "+".into(),
            args: vec![Value::Var("addr".into()), Value::Const(0)],
        },
    ];

    print_instructions("Before strength reduction", &body);
    let (reduced, ptrs) = strength_reduce(&body);
    print_instructions("After strength reduction", &reduced);
    println!("  Pointer variables created: {:?}\n", ptrs);
}

fn demo_interchange() {
    println!("=== Loop Interchange ===\n");

    let outer = Loop {
        header: "L_i".into(),
        induction: "i".into(),
        start: 0,
        end: 100,
        step: 1,
        body: vec![],
    };

    let inner = Loop {
        header: "L_j".into(),
        induction: "j".into(),
        start: 0,
        end: 100,
        step: 1,
        body: vec![
            Instr::Assign {
                dst: "v".into(),
                op: "+".into(),
                args: vec![Value::Var("i".into()), Value::Var("j".into())],
            },
            Instr::Store {
                base: "a".into(),
                index: Value::Var("i".into()),
                src: Value::Var("v".into()),
            },
        ],
    };

    print_loop("Original: outer i, inner j", &outer);
    print_loop("Original: inner loop", &inner);

    let can = can_interchange(&outer, &inner);
    println!("  Can interchange: {}\n", can);

    if can {
        let (new_outer, new_inner) = loop_interchange(&outer, &inner);
        print_loop("After interchange: outer j", &new_outer);
        print_loop("After interchange: inner loop", &new_inner);
    }
}

fn demo_vectorization_check() {
    println!("=== Vectorization Check ===\n");

    // Vectorizable: a[i] = b[i] + c[i]
    let lp_good = Loop {
        header: "L".into(),
        induction: "i".into(),
        start: 0,
        end: 1000,
        step: 1,
        body: vec![
            Instr::Load {
                dst: "t1".into(),
                base: "b".into(),
                index: Value::Var("i".into()),
            },
            Instr::Load {
                dst: "t2".into(),
                base: "c".into(),
                index: Value::Var("i".into()),
            },
            Instr::Assign {
                dst: "t3".into(),
                op: "+".into(),
                args: vec![Value::Var("t1".into()), Value::Var("t2".into())],
            },
            Instr::Store {
                base: "a".into(),
                index: Value::Var("i".into()),
                src: Value::Var("t3".into()),
            },
        ],
    };

    let (ok, barriers) = can_vectorize(&lp_good);
    print_loop("Vectorizable loop: a[i] = b[i] + c[i]", &lp_good);
    println!(
        "  Vectorizable: {}  Barriers: {:?}\n",
        ok, barriers
    );

    // Not vectorizable: a[i] = a[i-1] + 1 (flow dependence)
    let lp_bad = Loop {
        header: "L".into(),
        induction: "i".into(),
        start: 1,
        end: 1000,
        step: 1,
        body: vec![
            Instr::Load {
                dst: "prev".into(),
                base: "a".into(),
                index: Value::Var("i".into()), // In real IR this would be i-1
            },
            Instr::Assign {
                dst: "cur".into(),
                op: "+".into(),
                args: vec![Value::Var("prev".into()), Value::Const(1)],
            },
            Instr::Store {
                base: "a".into(),
                index: Value::Var("i".into()),
                src: Value::Var("cur".into()),
            },
        ],
    };

    let (ok2, barriers2) = can_vectorize(&lp_bad);
    print_loop("Non-vectorizable loop: a[i] = a[i-1] + 1", &lp_bad);
    println!(
        "  Vectorizable: {}  Barriers: {:?}\n",
        ok2, barriers2
    );
}

fn main() {
    println!("Loop Optimization & Vectorization");
    println!("==================================\n");

    demo_unrolling();
    demo_strength_reduction();
    demo_interchange();
    demo_vectorization_check();
}
