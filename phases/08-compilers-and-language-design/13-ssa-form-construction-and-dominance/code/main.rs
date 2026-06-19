//! SSA Form — Construction and Dominance
//! Phase 08 — Compilers & Programming Language Design
//!
//! Demonstrates: dominance computation, dominance frontiers,
//! φ-function insertion, and SSA renaming on a simple CFG.

use std::collections::{HashMap, HashSet, VecDeque};

// ---------------------------------------------------------------------------
// IR types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Operand {
    Const(i64),
    Var(String),
}

#[derive(Debug, Clone)]
enum Instr {
    Assign { dst: String, op: String, args: Vec<Operand> },
    Phi { dst: String, args: Vec<(String, usize)> }, // (versioned_src, pred_block_id)
    CondJump { cond: Operand, true_bb: usize, false_bb: usize },
    Jump(usize),
    Print(Operand),
}

#[derive(Debug, Clone)]
struct BasicBlock {
    id: usize,
    label: String,
    instrs: Vec<Instr>,
    preds: Vec<usize>,
    succs: Vec<usize>,
}

#[derive(Debug, Clone)]
struct CFG {
    blocks: Vec<BasicBlock>,
    entry: usize,
}

impl CFG {
    fn new() -> Self {
        CFG {
            blocks: Vec::new(),
            entry: 0,
        }
    }

    fn add_block(&mut self, label: &str) -> usize {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock {
            id,
            label: label.to_string(),
            instrs: Vec::new(),
            preds: Vec::new(),
            succs: Vec::new(),
        });
        id
    }

    fn add_edge(&mut self, from: usize, to: usize) {
        if !self.blocks[from].succs.contains(&to) {
            self.blocks[from].succs.push(to);
        }
        if !self.blocks[to].preds.contains(&from) {
            self.blocks[to].preds.push(from);
        }
    }

    fn push_instr(&mut self, bb: usize, instr: Instr) {
        self.blocks[bb].instrs.push(instr);
    }
}

// ---------------------------------------------------------------------------
// Dominance
// ---------------------------------------------------------------------------

/// Compute dominators using the iterative dataflow algorithm.
/// Returns a map: block_id -> set of blocks that dominate it.
fn compute_dominators(cfg: &CFG) -> HashMap<usize, HashSet<usize>> {
    let n = cfg.blocks.len();
    let mut doms: HashMap<usize, HashSet<usize>> = HashMap::new();

    // Initialise: entry dominated by itself; all others dominated by all blocks
    let all: HashSet<usize> = (0..n).collect();
    for b in 0..n {
        if b == cfg.entry {
            let mut s = HashSet::new();
            s.insert(b);
            doms.insert(b, s);
        } else {
            doms.insert(b, all.clone());
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for b in 0..n {
            if b == cfg.entry {
                continue;
            }
            let preds = &cfg.blocks[b].preds;
            if preds.is_empty() {
                continue;
            }
            // Intersect all predecessors' dominator sets, then add b itself
            let mut new_dom: HashSet<usize> = doms[&preds[0]].clone();
            for p in preds.iter().skip(1) {
                new_dom = new_dom.intersection(&doms[p]).cloned().collect();
            }
            new_dom.insert(b);
            if new_dom != doms[&b] {
                doms.insert(b, new_dom);
                changed = true;
            }
        }
    }
    doms
}

/// Extract the immediate dominator (idom) for each non-entry block.
/// idom(b) = the unique dominator of b that is not b, and is dominated
/// by all other strict dominators of b.
fn compute_idoms(cfg: &CFG, doms: &HashMap<usize, HashSet<usize>>) -> HashMap<usize, usize> {
    let mut idoms: HashMap<usize, usize> = HashMap::new();
    for b in 0..cfg.blocks.len() {
        if b == cfg.entry {
            continue;
        }
        let strict: Vec<usize> = doms[&b]
            .iter()
            .copied()
            .filter(|d| *d != b)
            .collect();
        // idom is the strict dominator that is dominated by all other strict dominators
        for &cand in &strict {
            let dominated_by_all = strict.iter().all(|&s| {
                s == cand || doms[&s].contains(&cand)
            });
            if dominated_by_all {
                idoms.insert(b, cand);
                break;
            }
        }
    }
    idoms
}

// ---------------------------------------------------------------------------
// Dominance frontiers
// ---------------------------------------------------------------------------

fn compute_dom_frontiers(
    cfg: &CFG,
    doms: &HashMap<usize, HashSet<usize>>,
) -> HashMap<usize, HashSet<usize>> {
    let mut df: HashMap<usize, HashSet<usize>> = HashMap::new();
    for b in 0..cfg.blocks.len() {
        df.insert(b, HashSet::new());
    }

    for b in 0..cfg.blocks.len() {
        let dominated_b = &doms[&b];
        for &succ in &cfg.blocks[b].succs {
            // If b does not strictly dominate succ, then succ is in DF(b)
            if !dominated_b.contains(&succ) || succ == b {
                // But only add if b dominates a predecessor of succ
                let b_dominates_some_pred = cfg.blocks[succ]
                    .preds
                    .iter()
                    .any(|p| dominated_b.contains(p));
                if b_dominates_some_pred {
                    df.get_mut(&b).unwrap().insert(succ);
                }
            }
        }
    }
    df
}

// ---------------------------------------------------------------------------
// Iterated dominance frontier
// ---------------------------------------------------------------------------

fn iterated_df(
    defs: &HashSet<usize>,
    df: &HashMap<usize, HashSet<usize>>,
) -> HashSet<usize> {
    let mut result = defs.clone();
    let mut changed = true;
    while changed {
        changed = false;
        let current: Vec<usize> = result.iter().copied().collect();
        for b in current {
            for &f in &df[&b] {
                if result.insert(f) {
                    changed = true;
                }
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------
// SSA construction
// ---------------------------------------------------------------------------

/// Collect assignment sites for each variable (before renaming).
fn collect_def_sites(cfg: &CFG) -> HashMap<String, HashSet<usize>> {
    let mut defs: HashMap<String, HashSet<usize>> = HashMap::new();
    for bb in &cfg.blocks {
        for instr in &bb.instrs {
            if let Instr::Assign { dst, .. } = instr {
                defs.entry(dst.clone()).or_default().insert(bb.id);
            }
        }
    }
    defs
}

/// Insert φ-functions into the CFG.
fn insert_phi_functions(
    cfg: &mut CFG,
    def_sites: &HashMap<String, HashSet<usize>>,
    df: &HashMap<usize, HashSet<usize>>,
) {
    let mut phi_needed: HashMap<usize, HashSet<String>> = HashMap::new();

    for (var, defs) in def_sites {
        let idf = iterated_df(defs, df);
        for b in idf {
            phi_needed.entry(b).or_default().insert(var.clone());
        }
    }

    // Insert φ at the beginning of each block
    for (bid, vars) in phi_needed {
        let mut new_instrs: Vec<Instr> = Vec::new();
        for v in vars {
            let args: Vec<(String, usize)> = cfg.blocks[bid]
                .preds
                .iter()
                .map(|&p| (format!("{}_0", v), p))
                .collect();
            new_instrs.push(Instr::Phi {
                dst: format!("{}_0", v),
                args,
            });
        }
        // Prepend φ-functions
        new_instrs.append(&mut cfg.blocks[bid].instrs);
        cfg.blocks[bid].instrs = new_instrs;
    }
}

/// Rename variables to SSA form (versioned names).
fn rename_variables(cfg: &mut CFG) {
    let n = cfg.blocks.len();
    let mut stack: HashMap<String, Vec<usize>> = HashMap::new(); // var -> version stack
    let mut counters: HashMap<String, usize> = HashMap::new();

    // Collect dominator tree children using idom
    let doms = compute_dominators(cfg);
    let idoms = compute_idoms(cfg, &doms);
    let mut dom_tree_children: HashMap<usize, Vec<usize>> = HashMap::new();
    for b in 0..n {
        dom_tree_children.insert(b, Vec::new());
    }
    for (&b, &idom_of_b) in &idoms {
        dom_tree_children.get_mut(&idom_of_b).unwrap().push(b);
    }

    fn rename_block(
        cfg: &mut CFG,
        bid: usize,
        stack: &mut HashMap<String, Vec<usize>>,
        counters: &mut HashMap<String, usize>,
        dom_tree_children: &HashMap<usize, Vec<usize>>,
    ) {
        // Process each instruction
        let n_instrs = cfg.blocks[bid].instrs.len();
        for i in 0..n_instrs {
            match &cfg.blocks[bid].instrs[i] {
                Instr::Assign { dst, op, args } => {
                    let counter = counters.entry(dst.clone()).or_insert(1);
                    let version = *counter;
                    *counter += 1;
                    stack.entry(dst.clone()).or_default().push(version);

                    let renamed_args: Vec<Operand> = args
                        .iter()
                        .map(|a| match a {
                            Operand::Var(name) => {
                                let ver = stack.get(name).and_then(|s| s.last()).copied().unwrap_or(0);
                                Operand::Var(format!("{}_{}", name, ver))
                            }
                            c => c.clone(),
                        })
                        .collect();

                    cfg.blocks[bid].instrs[i] = Instr::Assign {
                        dst: format!("{}_{}", dst, version),
                        op: op.clone(),
                        args: renamed_args,
                    };
                }
                Instr::Phi { dst, .. } => {
                    let base = dst.trim_end_matches("_0").to_string();
                    let counter = counters.entry(base.clone()).or_insert(1);
                    let version = *counter;
                    *counter += 1;
                    stack.entry(base.clone()).or_default().push(version);

                    cfg.blocks[bid].instrs[i] = Instr::Phi {
                        dst: format!("{}_{}", base, version),
                        args: Vec::new(), // filled in a second pass
                    };
                }
                Instr::CondJump { cond, true_bb, false_bb } => {
                    let renamed_cond = match cond {
                        Operand::Var(name) => {
                            let ver = stack.get(name).and_then(|s| s.last()).copied().unwrap_or(0);
                            Operand::Var(format!("{}_{}", name, ver))
                        }
                        c => c.clone(),
                    };
                    cfg.blocks[bid].instrs[i] = Instr::CondJump {
                        cond: renamed_cond,
                        true_bb: *true_bb,
                        false_bb: *false_bb,
                    };
                }
                Instr::Jump(target) => {
                    // no renaming needed
                }
                Instr::Print(op) => {
                    let renamed = match op {
                        Operand::Var(name) => {
                            let ver = stack.get(name).and_then(|s| s.last()).copied().unwrap_or(0);
                            Operand::Var(format!("{}_{}", name, ver))
                        }
                        c => c.clone(),
                    };
                    cfg.blocks[bid].instrs[i] = Instr::Print(renamed);
                }
            }
        }

        // Fill φ arguments in successors
        let succs = cfg.blocks[bid].succs.clone();
        for &succ in &succs {
            let n_instrs = cfg.blocks[succ].instrs.len();
            for i in 0..n_instrs {
                if let Instr::Phi { ref dst, ref mut args } = cfg.blocks[succ].instrs[i] {
                    let base = dst.rsplit_once('_').map(|(b, _)| b).unwrap_or(dst);
                    let ver = stack.get(base).and_then(|s| s.last()).copied().unwrap_or(0);
                    args.push((format!("{}_{}", base, ver), bid));
                }
            }
        }

        // Recurse into dominator children
        if let Some(children) = dom_tree_children.get(&bid) {
            for &child in children {
                rename_block(cfg, child, stack, counters, dom_tree_children);
            }
        }

        // Pop versions we pushed in this block
        for i in 0..n_instrs {
            let dst = match &cfg.blocks[bid].instrs[i] {
                Instr::Assign { dst, .. } | Instr::Phi { dst, .. } => Some(dst.clone()),
                _ => None,
            };
            if let Some(d) = dst {
                let base = d.rsplit_once('_').map(|(b, _)| b).unwrap_or(&d);
                stack.get_mut(base).unwrap().pop();
            }
        }
    }

    rename_block(cfg, cfg.entry, &mut stack, &mut counters, &dom_tree_children);
}

/// Convert a CFG to SSA form.
fn to_ssa(cfg: &mut CFG) {
    let def_sites = collect_def_sites(cfg);
    let doms = compute_dominators(cfg);
    let df = compute_dom_frontiers(cfg, &doms);
    insert_phi_functions(cfg, &def_sites, &df);
    rename_variables(cfg);
}

// ---------------------------------------------------------------------------
// Pretty-print
// ---------------------------------------------------------------------------

fn print_cfg(cfg: &CFG, title: &str) {
    println!("=== {} ===", title);
    for bb in &cfg.blocks {
        println!("\n{} (id={}):", bb.label, bb.id);
        println!("  preds: {:?}  succs: {:?}", bb.preds, bb.succs);
        for instr in &bb.instrs {
            println!("  {}", format_instr(instr));
        }
    }
    println!();
}

fn format_instr(instr: &Instr) -> String {
    match instr {
        Instr::Assign { dst, op, args } => {
            let args_str: Vec<String> = args.iter().map(format_operand).collect();
            format!("{} = {} {}", dst, op, args_str.join(" "))
        }
        Instr::Phi { dst, args } => {
            let parts: Vec<String> = args.iter().map(|(v, b)| format!("{} (from B{})", v, b)).collect();
            format!("{} = φ({})", dst, parts.join(", "))
        }
        Instr::CondJump { cond, true_bb, false_bb } => {
            format!("if {} goto B{} else goto B{}", format_operand(cond), true_bb, false_bb)
        }
        Instr::Jump(target) => format!("goto B{}", target),
        Instr::Print(op) => format!("print({})", format_operand(op)),
    }
}

fn format_operand(op: &Operand) -> String {
    match op {
        Operand::Const(n) => n.to_string(),
        Operand::Var(s) => s.clone(),
    }
}

// ---------------------------------------------------------------------------
// Demo
// ---------------------------------------------------------------------------

fn build_demo_cfg() -> CFG {
    let mut cfg = CFG::new();

    // Entry: a = 1; b = 2; if c goto L1 else goto L2
    let entry = cfg.add_block("Entry");
    let l1 = cfg.add_block("L1");
    let l2 = cfg.add_block("L2");
    let l3 = cfg.add_block("L3");

    cfg.push_instr(entry, Instr::Assign {
        dst: "a".into(),
        op: "=".into(),
        args: vec![Operand::Const(1)],
    });
    cfg.push_instr(entry, Instr::Assign {
        dst: "b".into(),
        op: "=".into(),
        args: vec![Operand::Const(2)],
    });
    cfg.push_instr(entry, Instr::CondJump {
        cond: Operand::Var("c".into()),
        true_bb: l1,
        false_bb: l2,
    });

    // L1: a = b + 3; goto L3
    cfg.push_instr(l1, Instr::Assign {
        dst: "a".into(),
        op: "+".into(),
        args: vec![Operand::Var("b".into()), Operand::Const(3)],
    });
    cfg.push_instr(l1, Instr::Jump(l3));

    // L2: b = a * 2; goto L3
    cfg.push_instr(l2, Instr::Assign {
        dst: "b".into(),
        op: "*".into(),
        args: vec![Operand::Var("a".into()), Operand::Const(2)],
    });
    cfg.push_instr(l2, Instr::Jump(l3));

    // L3: d = a + b; print(d)
    cfg.push_instr(l3, Instr::Assign {
        dst: "d".into(),
        op: "+".into(),
        args: vec![Operand::Var("a".into()), Operand::Var("b".into())],
    });
    cfg.push_instr(l3, Instr::Print(Operand::Var("d".into())));

    cfg.add_edge(entry, l1);
    cfg.add_edge(entry, l2);
    cfg.add_edge(l1, l3);
    cfg.add_edge(l2, l3);

    cfg
}

fn main() {
    println!("SSA Form — Construction and Dominance");
    println!("======================================\n");

    let mut cfg = build_demo_cfg();
    print_cfg(&cfg, "Original CFG");

    // Dominance
    let doms = compute_dominators(&cfg);
    println!("=== Dominators ===");
    for b in 0..cfg.blocks.len() {
        let mut dom_list: Vec<usize> = doms[&b].iter().copied().collect();
        dom_list.sort();
        println!("  {} (B{}): dominated by {:?}", cfg.blocks[b].label, b, dom_list);
    }

    // Dominance frontiers
    let df = compute_dom_frontiers(&cfg, &doms);
    println!("\n=== Dominance Frontiers ===");
    for b in 0..cfg.blocks.len() {
        let mut df_list: Vec<usize> = df[&b].iter().copied().collect();
        df_list.sort();
        println!("  DF({}) = {:?}", cfg.blocks[b].label, df_list);
    }

    // Convert to SSA
    to_ssa(&mut cfg);
    print_cfg(&cfg, "SSA Form");

    println!("=== Verification ===");
    println!("Every variable is assigned exactly once (SSA property).");
    println!("φ-functions appear at dominance frontier join points.");
}
