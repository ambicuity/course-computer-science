use std::collections::{HashMap, HashSet};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Data Structures
// ---------------------------------------------------------------------------

/// A virtual register (unlimited supply from the frontend/optimizer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct VReg(usize);

/// A live interval: the virtual register is live from `start` to `end` (inclusive).
#[derive(Debug, Clone)]
struct LiveInterval {
    var: VReg,
    start: usize,
    end: usize,
}

/// An IR instruction used for demonstration.
#[derive(Debug, Clone)]
enum IrInstr {
    /// Binary op: dest = lhs op rhs
    BinOp { dest: VReg, op: String, lhs: VReg, rhs: VReg },
    /// Load constant: dest = imm
    Const { dest: VReg, value: i64 },
    /// Move: dest = src
    Move { dest: VReg, src: VReg },
    /// Use: consume an operand (for liveness purposes)
    Use { src: VReg },
    /// Store to memory: store src to addr
    Store { addr: VReg, src: VReg },
    /// Comment (for display)
    Comment(String),
}

impl IrInstr {
    fn defs(&self) -> Option<VReg> {
        match self {
            IrInstr::BinOp { dest, .. } => Some(*dest),
            IrInstr::Const { dest, .. } => Some(*dest),
            IrInstr::Move { dest, .. } => Some(*dest),
            _ => None,
        }
    }

    fn uses(&self) -> Vec<VReg> {
        match self {
            IrInstr::BinOp { lhs, rhs, .. } => vec![*lhs, *rhs],
            IrInstr::Move { src, .. } => vec![*src],
            IrInstr::Use { src } => vec![*src],
            IrInstr::Store { addr, src } => vec![*addr, *src],
            _ => vec![],
        }
    }
}

// ---------------------------------------------------------------------------
// Liveness Analysis
// ---------------------------------------------------------------------------

/// Compute live intervals for all virtual registers from a list of IR instructions.
/// Each instruction is assigned a line number (1-indexed).
fn compute_liveness(instructions: &[IrInstr]) -> Vec<LiveInterval> {
    let mut starts: HashMap<VReg, usize> = HashMap::new();
    let mut ends: HashMap<VReg, usize> = HashMap::new();

    for (i, instr) in instructions.iter().enumerate() {
        let line = i + 1;
        if let Some(d) = instr.defs() {
            starts.entry(d).or_insert(line);
            ends.insert(d, line);
        }
        for u in instr.uses() {
            starts.entry(u).or_insert(line);
            ends.insert(u, line);
        }
    }

    let mut intervals: Vec<LiveInterval> = starts
        .into_iter()
        .map(|(var, s)| LiveInterval {
            var,
            start: s,
            end: *ends.get(&var).unwrap(),
        })
        .collect();

    intervals.sort_by_key(|iv| (iv.start, iv.end));
    intervals
}

// ---------------------------------------------------------------------------
// Interference Graph
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct InterferenceGraph {
    /// Adjacency set for each node.
    adj: HashMap<VReg, HashSet<VReg>>,
}

impl InterferenceGraph {
    fn new() -> Self {
        Self {
            adj: HashMap::new(),
        }
    }

    fn add_node(&mut self, v: VReg) {
        self.adj.entry(v).or_default();
    }

    fn add_edge(&mut self, a: VReg, b: VReg) {
        self.adj.entry(a).or_default().insert(b);
        self.adj.entry(b).or_default().insert(a);
    }

    fn nodes(&self) -> Vec<VReg> {
        self.adj.keys().copied().collect()
    }

    fn neighbors(&self, v: VReg) -> &HashSet<VReg> {
        self.adj.get(&v).unwrap()
    }

    fn degree(&self, v: VReg) -> usize {
        self.neighbors(v).len()
    }

    fn remove_node(&mut self, v: VReg) {
        let neighbors = self.adj.remove(&v).unwrap_or_default();
        for n in &neighbors {
            if let Some(set) = self.adj.get_mut(n) {
                set.remove(&v);
            }
        }
    }
}

/// Build an interference graph from live intervals.
/// Two intervals interfere if they overlap.
fn build_interference_graph(intervals: &[LiveInterval]) -> InterferenceGraph {
    let mut graph = InterferenceGraph::new();
    for iv in intervals {
        graph.add_node(iv.var);
    }
    for i in 0..intervals.len() {
        for j in (i + 1)..intervals.len() {
            let a = &intervals[i];
            let b = &intervals[j];
            // Overlap: a.start <= b.end AND b.start <= a.end
            if a.start <= b.end && b.start <= a.end {
                graph.add_edge(a.var, b.var);
            }
        }
    }
    graph
}

// ---------------------------------------------------------------------------
// Graph Coloring (Chaitin-Briggs)
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct GraphColoringResult {
    /// Mapping from virtual register to physical register index (0..k-1).
    colors: HashMap<VReg, usize>,
    /// Registers that were spilled.
    spills: HashSet<VReg>,
    /// How many registers were actually assigned (k minus spills).
    assigned: usize,
}

/// Attempt to color the interference graph with `k` colors using
/// Chaitin-Briggs simplify + spill heuristic.
fn graph_color(graph: &InterferenceGraph, k: usize) -> GraphColoringResult {
    // Work on a mutable copy.
    let mut working = graph.clone();
    let mut stack: Vec<(VReg, HashSet<VReg>)> = Vec::new();
    let mut spills: HashSet<VReg> = HashSet::new();

    // Phase 1: Simplify
    loop {
        if working.adj.is_empty() {
            break;
        }
        // Try to find a node with degree < k.
        let node_to_simplify = working
            .adj
            .iter()
            .find(|(_, neighbors)| neighbors.len() < k)
            .map(|(&v, _)| v);

        if let Some(v) = node_to_simplify {
            let neighbors = working.neighbors(v).clone();
            working.remove_node(v);
            stack.push((v, neighbors));
        } else {
            // Spill heuristic: pick the node with the highest degree.
            let victim = working
                .adj
                .keys()
                .max_by_key(|v| working.neighbors(**v).len())
                .copied()
                .unwrap();
            spills.insert(victim);
            let neighbors = working.neighbors(victim).clone();
            working.remove_node(victim);
            stack.push((victim, neighbors));
        }
    }

    // Phase 2: Select (pop from stack, assign colors).
    let mut colors: HashMap<VReg, usize> = HashMap::new();

    while let Some((v, neighbors)) = stack.pop() {
        // Find the set of colors used by already-colored neighbors.
        let mut used_colors: HashSet<usize> = HashSet::new();
        for n in &neighbors {
            if let Some(&c) = colors.get(n) {
                used_colors.insert(c);
            }
        }
        // Find an available color.
        let color = (0..k).find(|c| !used_colors.contains(c));
        match color {
            Some(c) => {
                colors.insert(v, c);
                spills.remove(&v); // It was actually colorable.
            }
            None => {
                // Truly spilled.
            }
        }
    }

    GraphColoringResult {
        colors,
        spills,
        assigned: k,
    }
}

// ---------------------------------------------------------------------------
// Linear Scan
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct LinearScanResult {
    colors: HashMap<VReg, usize>,
    spills: HashSet<VReg>,
}

/// Greedy linear scan register allocator.
/// `intervals` should be sorted by start point.
/// `k` is the number of available physical registers.
fn linear_scan(intervals: &[LiveInterval], k: usize) -> LinearScanResult {
    let mut colors: HashMap<VReg, usize> = HashMap::new();
    let mut spills: HashSet<VReg> = HashSet::new();

    // Active list: intervals that are currently live.
    // We store (end_point, reg_index, vreg).
    let mut active: Vec<(usize, usize, VReg)> = Vec::new();
    // Free register pool.
    let mut free_regs: Vec<usize> = (0..k).rev().collect();

    for iv in intervals {
        // Expire old intervals: remove those whose end < iv.start.
        let mut to_free: Vec<usize> = Vec::new();
        active.retain(|&(end, reg, _v)| {
            if end < iv.start {
                to_free.push(reg);
                false
            } else {
                true
            }
        });
        free_regs.extend(to_free);

        if let Some(reg) = free_regs.pop() {
            // Assign this register.
            colors.insert(iv.var, reg);
            // Insert into active list, sorted by end point.
            active.push((iv.end, reg, iv.var));
            active.sort_by_key(|&(end, _, _)| end);
        } else {
            // Spill: pick the active interval with the farthest end.
            if let Some(last) = active.last() {
                if last.0 > iv.end {
                    // Spill the active interval, give its register to iv.
                    let (_, spilled_reg, spilled_var) = active.pop().unwrap();
                    spills.insert(spilled_var);
                    colors.remove(&spilled_var);
                    colors.insert(iv.var, spilled_reg);
                    active.push((iv.end, spilled_reg, iv.var));
                    active.sort_by_key(|&(end, _, _)| end);
                } else {
                    // Spill the current interval.
                    spills.insert(iv.var);
                }
            } else {
                // No active intervals but no free registers? (Shouldn't happen if k > 0)
                spills.insert(iv.var);
            }
        }
    }

    LinearScanResult { colors, spills }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn format_reg(r: VReg) -> String {
    format!("v{}", r.0)
}

fn format_phys(r: usize) -> String {
    format!("r{}", r)
}

fn print_intervals(label: &str, intervals: &[LiveInterval]) {
    println!("{}:", label);
    for iv in intervals {
        println!("  {} = [{}, {}]", format_reg(iv.var), iv.start, iv.end);
    }
}

fn print_allocation(label: &str, result_colors: &HashMap<VReg, usize>, spills: &HashSet<VReg>) {
    println!("{}:", label);
    let mut sorted_regs: Vec<VReg> = result_colors.keys().copied().collect();
    sorted_regs.sort();
    for v in sorted_regs {
        println!("  {} -> {}", format_reg(v), format_phys(result_colors[&v]));
    }
    if !spills.is_empty() {
        let mut spill_list: Vec<VReg> = spills.iter().copied().collect();
        spill_list.sort();
        println!("  Spilled: {:?}", spill_list.iter().map(|r| format_reg(*r)).collect::<Vec<_>>());
    }
}

fn demo_program_1() -> Vec<IrInstr> {
    // A simple program with 6 virtual registers.
    // Computes: v1 = v0 + v2; v3 = v1 * v4; v5 = v3 + v0
    vec![
        IrInstr::Const { dest: VReg(0), value: 10 },
        IrInstr::Const { dest: VReg(2), value: 20 },
        IrInstr::BinOp { dest: VReg(1), op: "+".into(), lhs: VReg(0), rhs: VReg(2) },
        IrInstr::Const { dest: VReg(4), value: 3 },
        IrInstr::BinOp { dest: VReg(3), op: "*".into(), lhs: VReg(1), rhs: VReg(4) },
        IrInstr::BinOp { dest: VReg(5), op: "+".into(), lhs: VReg(3), rhs: VReg(0) },
        IrInstr::Use { src: VReg(5) },
    ]
}

fn demo_program_2() -> Vec<IrInstr> {
    // A program that creates register pressure: 8 live values in a tight range.
    vec![
        IrInstr::Const { dest: VReg(0), value: 1 },
        IrInstr::Const { dest: VReg(1), value: 2 },
        IrInstr::Const { dest: VReg(2), value: 3 },
        IrInstr::Const { dest: VReg(3), value: 4 },
        IrInstr::Const { dest: VReg(4), value: 5 },
        IrInstr::Const { dest: VReg(5), value: 6 },
        IrInstr::Const { dest: VReg(6), value: 7 },
        IrInstr::Const { dest: VReg(7), value: 8 },
        // All 8 variables are live simultaneously here (they'll be used below).
        IrInstr::BinOp { dest: VReg(8), op: "+".into(), lhs: VReg(0), rhs: VReg(1) },
        IrInstr::BinOp { dest: VReg(9), op: "+".into(), lhs: VReg(2), rhs: VReg(3) },
        IrInstr::BinOp { dest: VReg(10), op: "+".into(), lhs: VReg(4), rhs: VReg(5) },
        IrInstr::BinOp { dest: VReg(11), op: "+".into(), lhs: VReg(6), rhs: VReg(7) },
        IrInstr::BinOp { dest: VReg(12), op: "+".into(), lhs: VReg(8), rhs: VReg(9) },
        IrInstr::BinOp { dest: VReg(13), op: "+".into(), lhs: VReg(10), rhs: VReg(11) },
        IrInstr::BinOp { dest: VReg(14), op: "+".into(), lhs: VReg(12), rhs: VReg(13) },
        IrInstr::Use { src: VReg(14) },
    ]
}

fn benchmark<F: FnMut()>(label: &str, mut f: F, iterations: u32) {
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = start.elapsed();
    println!("  {}: {:.2?} ({} iterations)", label, elapsed, iterations);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    println!("=== Lesson 16: Register Allocation ===\n");

    // ----- Demo 1: Simple program -----
    println!("--- Demo 1: Simple program (low register pressure) ---\n");
    let prog1 = demo_program_1();
    println!("IR Instructions:");
    for (i, instr) in prog1.iter().enumerate() {
        println!("  {}: {:?}", i + 1, instr);
    }
    println!();

    let intervals1 = compute_liveness(&prog1);
    print_intervals("Live intervals", &intervals1);
    println!();

    let graph1 = build_interference_graph(&intervals1);
    println!("Interference graph edges:");
    for v in graph1.nodes() {
        let n: Vec<String> = graph1.neighbors(v).iter().map(|nr| format_reg(*nr)).collect();
        println!("  {} — {:?}", format_reg(v), n);
    }
    println!();

    // Graph coloring with 4 registers.
    let gc1 = graph_color(&graph1, 4);
    print_allocation("Graph coloring (k=4)", &gc1.colors, &gc1.spills);
    println!();

    // Linear scan with 4 registers.
    let ls1 = linear_scan(&intervals1, 4);
    print_allocation("Linear scan (k=4)", &ls1.colors, &ls1.spills);
    println!();

    // ----- Demo 2: High register pressure -----
    println!("--- Demo 2: High register pressure (needs spilling) ---\n");
    let prog2 = demo_program_2();
    let intervals2 = compute_liveness(&prog2);
    print_intervals("Live intervals", &intervals2);
    println!();

    let graph2 = build_interference_graph(&intervals2);
    let gc2 = graph_color(&graph2, 4);
    print_allocation("Graph coloring (k=4)", &gc2.colors, &gc2.spills);
    println!();

    let ls2 = linear_scan(&intervals2, 4);
    print_allocation("Linear scan (k=4)", &ls2.colors, &ls2.spills);
    println!();

    // ----- Comparison -----
    println!("--- Comparison ---\n");
    println!("Demo 1 (k=4):");
    println!("  Graph coloring spills: {}", gc1.spills.len());
    println!("  Linear scan spills:    {}", ls2.spills.len());
    println!("Demo 2 (k=4):");
    println!("  Graph coloring spills: {}", gc2.spills.len());
    println!("  Linear scan spills:    {}", ls2.spills.len());
    println!();

    // ----- Benchmark -----
    println!("--- Performance Benchmark ---\n");
    let prog2 = demo_program_2();
    let intervals2 = compute_liveness(&prog2);
    let graph2 = build_interference_graph(&intervals2);

    let iters = 100_000;
    benchmark("Graph coloring", || {
        graph_color(&graph2, 4);
    }, iters);
    benchmark("Linear scan", || {
        linear_scan(&intervals2, 4);
    }, iters);
    println!();

    println!("--- Summary ---");
    println!("Graph coloring: higher quality (fewer spills), slower.");
    println!("Linear scan:    faster, slightly more spills.");
    println!("Production AOT compilers use graph coloring; JITs use linear scan.");
}
