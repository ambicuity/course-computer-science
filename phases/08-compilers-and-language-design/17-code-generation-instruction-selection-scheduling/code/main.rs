use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// IR Definition
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Temp(usize); // Virtual/temporary register

#[derive(Debug, Clone)]
enum IrOp {
    Add,
    Sub,
    Mul,
    Div,
    /// Load from memory: dest = *(base + offset)
    Load,
    /// Store to memory: *(base + offset) = src
    Store,
    /// Load immediate
    Const(i64),
    /// Move between registers
    Move,
}

#[derive(Debug, Clone)]
struct IrInstruction {
    id: usize,
    op: IrOp,
    dest: Option<Temp>, // None for Store and pure-effect ops
    args: Vec<Temp>,    // Virtual registers used as operands
    /// For Load/Store: immediate offset from base
    offset: i64,
}

impl IrInstruction {
    /// Estimated latency in cycles (for scheduling priority).
    fn latency(&self) -> u32 {
        match &self.op {
            IrOp::Add | IrOp::Sub => 1,
            IrOp::Mul => 3,
            IrOp::Div => 10,
            IrOp::Load => 3,
            IrOp::Store => 1,
            IrOp::Const(_) => 1,
            IrOp::Move => 1,
        }
    }
}

// ---------------------------------------------------------------------------
// Instruction Selection: IR → RISC-V Assembly
// ---------------------------------------------------------------------------

/// A RISC-V instruction represented as a string for simplicity.
#[derive(Debug, Clone)]
struct RiscVInstr {
    asm: String,
    /// Set of registers read.
    reads: Vec<String>,
    /// Register written (if any).
    writes: Option<String>,
    /// Latency (for scheduling).
    latency: u32,
}

impl RiscVInstr {
    fn new(asm: &str, reads: Vec<&str>, write: Option<&str>, latency: u32) -> Self {
        Self {
            asm: asm.to_string(),
            reads: reads.into_iter().map(|s| s.to_string()).collect(),
            writes: write.map(|s| s.to_string()),
            latency,
        }
    }
}

/// Map a virtual register to a RISC-V register name.
/// Registers t0-t6 are temporary (caller-saved).
fn reg_name(t: Temp) -> String {
    // Use t-registers for simplicity. In a real compiler this comes from
    // the register allocator's output.
    format!("t{}", t.0 % 7)
}

/// Select RISC-V instructions for a single IR instruction.
fn select_instructions(ir: &IrInstruction) -> Vec<RiscVInstr> {
    let dest = ir.dest.map(|t| reg_name(t)).unwrap_or_default();
    let args: Vec<String> = ir.args.iter().map(|t| reg_name(*t)).collect();

    match &ir.op {
        IrOp::Add => {
            vec![RiscVInstr::new(
                &format!("add {}, {}, {}", dest, args[0], args[1]),
                vec![&args[0], &args[1]],
                Some(&dest),
                1,
            )]
        }
        IrOp::Sub => {
            vec![RiscVInstr::new(
                &format!("sub {}, {}, {}", dest, args[0], args[1]),
                vec![&args[0], &args[1]],
                Some(&dest),
                1,
            )]
        }
        IrOp::Mul => {
            vec![RiscVInstr::new(
                &format!("mul {}, {}, {}", dest, args[0], args[1]),
                vec![&args[0], &args[1]],
                Some(&dest),
                3,
            )]
        }
        IrOp::Div => {
            vec![RiscVInstr::new(
                &format!("div {}, {}, {}", dest, args[0], args[1]),
                vec![&args[0], &args[1]],
                Some(&dest),
                10,
            )]
        }
        IrOp::Load => {
            vec![RiscVInstr::new(
                &format!("ld {}, {}({})", dest, ir.offset, args[0]),
                vec![&args[0]],
                Some(&dest),
                3,
            )]
        }
        IrOp::Store => {
            vec![RiscVInstr::new(
                &format!("sd {}, {}({})", args[0], ir.offset, args[1]),
                vec![&args[0], &args[1]],
                None,
                1,
            )]
        }
        IrOp::Const(val) => {
            if *val >= -2048 && *val <= 2047 {
                vec![RiscVInstr::new(
                    &format!("li {}, {}", dest, val),
                    vec![],
                    Some(&dest),
                    1,
                )]
            } else {
                // Need lui + addi for large immediates
                let upper = ((val + 0x800) >> 12) as u64;
                let lower = (val & 0xFFF) as u64;
                vec![
                    RiscVInstr::new(
                        &format!("lui {}, {}", dest, upper),
                        vec![],
                        Some(&dest),
                        1,
                    ),
                    RiscVInstr::new(
                        &format!("addi {}, {}, {}", dest, dest, lower),
                        vec![&dest],
                        Some(&dest),
                        1,
                    ),
                ]
            }
        }
        IrOp::Move => {
            vec![RiscVInstr::new(
                &format!("mv {}, {}", dest, args[0]),
                vec![&args[0]],
                Some(&dest),
                1,
            )]
        }
    }
}

// ---------------------------------------------------------------------------
// Instruction Scheduling (List Scheduling)
// ---------------------------------------------------------------------------

/// A scheduled instruction with dependency information.
#[derive(Debug, Clone)]
struct SchedNode {
    id: usize,
    riscv: RiscVInstr,
    /// Instructions this one depends on (must complete before this starts).
    deps: Vec<usize>,
    /// Priority: longest path to end (higher = schedule first).
    priority: u32,
}

/// Schedule RISC-V instructions to minimize pipeline stalls using list scheduling.
/// Reorders independent instructions to fill delay slots.
fn schedule_instructions(instructions: Vec<RiscVInstr>) -> Vec<RiscVInstr> {
    if instructions.len() <= 1 {
        return instructions;
    }

    let n = instructions.len();
    // Build dependency graph.
    // An instruction depends on a prior instruction if it reads a register
    // that the prior instruction writes (RAW — read-after-write).
    // We also track WAW and WAR for correctness.
    let mut nodes: Vec<SchedNode> = Vec::new();
    // Track last write to each register.
    let mut last_write: HashMap<String, usize> = HashMap::new();
    // Track last read of each register (for WAR).
    let mut last_read: HashMap<String, Vec<usize>> = HashMap::new();

    for (i, instr) in instructions.iter().enumerate() {
        let mut deps: HashSet<usize> = HashSet::new();

        // RAW: this instruction reads a register written by a prior instruction.
        for r in &instr.reads {
            if let Some(&writer) = last_write.get(r) {
                deps.insert(writer);
            }
        }

        // WAW + WAR: if this instruction writes a register.
        if let Some(ref w) = instr.writes {
            // WAW: a prior instruction also wrote this register.
            if let Some(&writer) = last_write.get(w) {
                deps.insert(writer);
            }
            // WAR: a prior instruction reads this register before we write it.
            if let Some(readers) = last_read.get(w) {
                for &reader in readers {
                    deps.insert(reader);
                }
            }
            last_write.insert(w.clone(), i);
        }

        // Record reads.
        for r in &instr.reads {
            last_read.entry(r.clone()).or_default().push(i);
        }

        nodes.push(SchedNode {
            id: i,
            riscv: instr.clone(),
            deps: deps.into_iter().collect(),
            priority: 0,
        });
    }

    // Compute priorities: longest path from each node to end.
    // Process in reverse topological order.
    let mut priority = vec![0u32; n];
    for i in (0..n).rev() {
        let mut max_succ_priority = 0u32;
        for &dep_id in &nodes[i].deps {
            // dep_id < i since dependencies only go backward
            max_succ_priority = max_succ_priority.max(priority[dep_id]);
        }
        priority[i] = nodes[i].riscv.latency + max_succ_priority;
    }
    for (i, node) in nodes.iter_mut().enumerate() {
        node.priority = priority[i];
    }

    // List scheduling: maintain a ready set and pick highest priority.
    let mut out_degree: Vec<usize> = vec![0; n];
    for i in 0..n {
        for &d in &nodes[i].deps {
            out_degree[d] += 1;
        }
    }

    let mut ready: Vec<usize> = Vec::new();
    for i in 0..n {
        if nodes[i].deps.is_empty() {
            ready.push(i);
        }
    }

    let mut scheduled: Vec<usize> = Vec::new();
    let mut completed: HashSet<usize> = HashSet::new();

    while !ready.is_empty() {
        // Pick the node with highest priority.
        ready.sort_by_key(|&id| std::cmp::Reverse(priority[id]));
        let chosen = ready.remove(0);
        scheduled.push(chosen);
        completed.insert(chosen);

        // Update ready set: nodes whose deps are now all completed.
        let mut new_ready: Vec<usize> = Vec::new();
        for i in 0..n {
            if completed.contains(&i) || ready.contains(&i) {
                continue;
            }
            if nodes[i].deps.iter().all(|d| completed.contains(d)) {
                new_ready.push(i);
            }
        }
        ready.extend(new_ready);
    }

    scheduled.into_iter().map(|id| nodes[id].riscv.clone()).collect()
}

// ---------------------------------------------------------------------------
// Function Prologue / Epilogue
// ---------------------------------------------------------------------------

fn emit_function_prologue(num_locals: i64, saved_regs: &[&str]) -> Vec<String> {
    let mut lines = Vec::new();
    // Stack frame: locals + saved registers + return address
    let frame_size = num_locals + (saved_regs.len() as i64 + 1) * 8;
    // Align to 16 bytes
    let frame_size = (frame_size + 15) & !15;

    lines.push(format!("  addi sp, sp, -{}", frame_size));
    // Save return address
    lines.push(format!("  sd ra, {}(sp)", frame_size - 8));
    // Save callee-saved registers
    for (i, reg) in saved_regs.iter().enumerate() {
        lines.push(format!("  sd {}, {}(sp)", reg, frame_size - 8 - (i + 1) as i64 * 8));
    }
    lines
}

fn emit_function_epilogue(num_locals: i64, saved_regs: &[&str]) -> Vec<String> {
    let mut lines = Vec::new();
    let frame_size = num_locals + (saved_regs.len() as i64 + 1) * 8;
    let frame_size = (frame_size + 15) & !15;

    // Restore callee-saved registers
    for (i, reg) in saved_regs.iter().enumerate() {
        lines.push(format!("  ld {}, {}(sp)", reg, frame_size - 8 - (i + 1) as i64 * 8));
    }
    // Restore return address
    lines.push(format!("  ld ra, {}(sp)", frame_size - 8));
    lines.push(format!("  addi sp, sp, {}", frame_size));
    lines.push("  ret".to_string());
    lines
}

// ---------------------------------------------------------------------------
// Calling Convention
// ---------------------------------------------------------------------------

struct CallingConvention;

impl CallingConvention {
    /// Argument registers for RISC-V (a0-a7).
    const ARG_REGS: [&'static str; 8] = ["a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7"];

    /// Emit code to pass `num_args` arguments.
    /// Arguments 0-7 go in a0-a7; extra arguments go on the stack.
    fn emit_call_setup(num_args: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let stack_args = if num_args > 8 { num_args - 8 } else { 0 };
        if stack_args > 0 {
            let stack_space = (stack_args * 8 + 15) & !15;
            lines.push(format!("  addi sp, sp, -{}", stack_space));
            // In a real compiler, we'd emit stores for each stack arg.
            lines.push(format!("  # {} args on stack ({} bytes)", stack_args, stack_space));
        }
        lines
    }

    /// Emit code to clean up after a call.
    fn emit_call_cleanup(num_args: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let stack_args = if num_args > 8 { num_args - 8 } else { 0 };
        if stack_args > 0 {
            let stack_space = (stack_args * 8 + 15) & !15;
            lines.push(format!("  addi sp, sp, {}", stack_space));
        }
        lines
    }
}

// ---------------------------------------------------------------------------
// Complete Code Generation Pipeline
// ---------------------------------------------------------------------------

fn generate_code(ir_program: &[IrInstruction], function_name: &str, num_locals: i64) -> String {
    let mut output = String::new();

    // Function label
    output.push_str(&format!(".globl {}\n", function_name));
    output.push_str(&format!("{}:\n", function_name));

    // Prologue
    let saved_regs = ["s0", "s1", "s2"];
    for line in emit_function_prologue(num_locals, &saved_regs) {
        output.push_str(&line);
        output.push('\n');
    }

    output.push('\n');

    // Instruction selection
    let mut all_riscv: Vec<RiscVInstr> = Vec::new();
    for ir in ir_program {
        let selected = select_instructions(ir);
        output.push_str(&format!("  # IR: {:?}\n", ir));
        for instr in &selected {
            output.push_str(&format!("  {}\n", instr.asm));
        }
        all_riscv.extend(selected);
    }

    output.push('\n');

    // Scheduling (show reordered code)
    output.push_str("  # --- After scheduling ---\n");
    let scheduled = schedule_instructions(all_riscv);
    for instr in &scheduled {
        output.push_str(&format!("  {}\n", instr.asm));
    }

    output.push('\n');

    // Epilogue
    for line in emit_function_epilogue(num_locals, &saved_regs) {
        output.push_str(&line);
        output.push('\n');
    }

    output
}

// ---------------------------------------------------------------------------
// Demos
// ---------------------------------------------------------------------------

fn demo_simple_program() -> Vec<IrInstruction> {
    vec![
        IrInstruction { id: 0, op: IrOp::Const(10), dest: Some(Temp(0)), args: vec![], offset: 0 },
        IrInstruction { id: 1, op: IrOp::Const(20), dest: Some(Temp(1)), args: vec![], offset: 0 },
        IrInstruction { id: 2, op: IrOp::Add, dest: Some(Temp(2)), args: vec![Temp(0), Temp(1)], offset: 0 },
        IrInstruction { id: 3, op: IrOp::Const(3), dest: Some(Temp(3)), args: vec![], offset: 0 },
        IrInstruction { id: 4, op: IrOp::Mul, dest: Some(Temp(4)), args: vec![Temp(2), Temp(3)], offset: 0 },
    ]
}

fn demo_memory_operations() -> Vec<IrInstruction> {
    vec![
        IrInstruction { id: 0, op: IrOp::Const(0), dest: Some(Temp(0)), args: vec![], offset: 0 },
        IrInstruction { id: 1, op: IrOp::Load, dest: Some(Temp(1)), args: vec![Temp(0)], offset: 0 },
        IrInstruction { id: 2, op: IrOp::Load, dest: Some(Temp(2)), args: vec![Temp(0)], offset: 8 },
        IrInstruction { id: 3, op: IrOp::Add, dest: Some(Temp(3)), args: vec![Temp(1), Temp(2)], offset: 0 },
        IrInstruction { id: 4, op: IrOp::Store, dest: None, args: vec![Temp(3), Temp(0)], offset: 16 },
    ]
}

fn demo_scheduling_benefit() -> Vec<RiscVInstr> {
    // Two independent dependency chains that can be interleaved.
    vec![
        RiscVInstr::new("mul t0, a0, a1", vec!["a0", "a1"], Some("t0"), 3),
        RiscVInstr::new("mul t1, a2, a3", vec!["a2", "a3"], Some("t1"), 3),
        RiscVInstr::new("add t2, t0, t1", vec!["t0", "t1"], Some("t2"), 1),
        RiscVInstr::new("mul t3, a4, a5", vec!["a4", "a5"], Some("t3"), 3),
        RiscVInstr::new("add t4, t2, t3", vec!["t2", "t3"], Some("t4"), 1),
    ]
}

fn print_riscv(label: &str, instrs: &[RiscVInstr]) {
    println!("{}:", label);
    for instr in instrs {
        println!("  {}", instr.asm);
    }
    println!();
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    println!("=== Lesson 17: Code Generation — Instruction Selection, Scheduling ===\n");

    // ----- Demo 1: Instruction Selection -----
    println!("--- Demo 1: Instruction Selection ---\n");
    let prog = demo_simple_program();
    println!("IR Program:");
    for ir in &prog {
        println!("  {:?}", ir);
    }
    println!();

    println!("Selected RISC-V Instructions:");
    for ir in &prog {
        let selected = select_instructions(ir);
        for instr in &selected {
            println!("  {}", instr.asm);
        }
    }
    println!();

    // ----- Demo 2: Memory Operations -----
    println!("--- Demo 2: Memory Operations ---\n");
    let mem_prog = demo_memory_operations();
    println!("IR:");
    for ir in &mem_prog {
        println!("  {:?}", ir);
    }
    println!();
    println!("RISC-V:");
    for ir in &mem_prog {
        let selected = select_instructions(ir);
        for instr in &selected {
            println!("  {}", instr.asm);
        }
    }
    println!();

    // ----- Demo 3: Instruction Scheduling -----
    println!("--- Demo 3: Instruction Scheduling ---\n");
    let unscheduled = demo_scheduling_benefit();
    print_riscv("Before scheduling", &unscheduled);
    let scheduled = schedule_instructions(unscheduled);
    print_riscv("After scheduling", &scheduled);

    // ----- Demo 4: Function Prologue / Epilogue -----
    println!("--- Demo 4: Function Prologue / Epilogue ---\n");
    let prologue = emit_function_prologue(16, &["s0", "s1"]);
    println!("Prologue (16 locals, save s0, s1):");
    for line in &prologue {
        println!("  {}", line);
    }
    println!();
    let epilogue = emit_function_epilogue(16, &["s0", "s1"]);
    println!("Epilogue:");
    for line in &epilogue {
        println!("  {}", line);
    }
    println!();

    // ----- Demo 5: Calling Convention -----
    println!("--- Demo 5: Calling Convention ---\n");
    println!("Setup for 3 args:");
    for line in CallingConvention::emit_call_setup(3) {
        println!("  {}", line);
    }
    println!();
    println!("Setup for 10 args (2 on stack):");
    for line in CallingConvention::emit_call_setup(10) {
        println!("  {}", line);
    }
    println!();
    println!("Cleanup for 10 args:");
    for line in CallingConvention::emit_call_cleanup(10) {
        println!("  {}", line);
    }
    println!();

    // ----- Demo 6: Full Code Generation -----
    println!("--- Demo 6: Full Code Generation ---\n");
    let code = generate_code(&demo_simple_program(), "compute", 0);
    println!("{}", code);

    println!("--- Summary ---");
    println!("Instruction selection maps IR ops to target machine instructions.");
    println!("Instruction scheduling reorders to minimize pipeline stalls.");
    println!("Calling conventions define ABI contracts for function calls.");
}
