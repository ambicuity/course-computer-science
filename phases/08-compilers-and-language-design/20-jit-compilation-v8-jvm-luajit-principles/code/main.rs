use std::collections::HashMap;
use std::time::Instant;

// ── Bytecode Instructions ─────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum Op {
    Const(i64),        // push constant
    LoadLocal(usize),  // push local[slot]
    StoreLocal(usize), // pop → local[slot]
    Add,               // pop b, pop a, push a+b
    Sub,               // pop b, pop a, push a-b
    Mul,               // pop b, pop a, push a*b
    Lt,                // pop b, pop a, push (a < b) as i64
    Jump(usize),       // unconditional jump to offset
    JumpIfZero(usize), // pop, jump if zero
    Call(usize),       // call function by id
    Return,            // return top of stack
    Print,             // pop and print
}

// ── Function Representation ───────────────────────────────────────────────

#[derive(Clone)]
struct Func {
    name: String,
    params: usize,
    locals: usize,
    bytecode: Vec<Op>,
    call_count: usize,
    compiled: bool,
}

// ── Bytecode Interpreter ──────────────────────────────────────────────────

struct Interpreter {
    functions: Vec<Func>,
    call_counts: Vec<usize>,
}

impl Interpreter {
    fn new(functions: Vec<Func>) -> Self {
        let call_counts = functions.iter().map(|_| 0).collect();
        Self {
            functions,
            call_counts,
        }
    }

    fn execute(&mut self, func_id: usize, args: &[i64]) -> i64 {
        self.call_counts[func_id] += 1;
        self.functions[func_id].call_count = self.call_counts[func_id];

        let func = &self.functions[func_id];
        let mut locals = vec![0i64; func.locals];
        for (i, &arg) in args.iter().enumerate() {
            locals[i] = arg;
        }

        let mut stack: Vec<i64> = Vec::new();
        let mut ip = 0;

        loop {
            if ip >= func.bytecode.len() {
                break;
            }

            match &func.bytecode[ip] {
                Op::Const(v) => stack.push(*v),
                Op::LoadLocal(slot) => stack.push(locals[*slot]),
                Op::StoreLocal(slot) => {
                    let val = stack.pop().unwrap();
                    locals[*slot] = val;
                }
                Op::Add => {
                    let b = stack.pop().unwrap();
                    let a = stack.pop().unwrap();
                    stack.push(a + b);
                }
                Op::Sub => {
                    let b = stack.pop().unwrap();
                    let a = stack.pop().unwrap();
                    stack.push(a - b);
                }
                Op::Mul => {
                    let b = stack.pop().unwrap();
                    let a = stack.pop().unwrap();
                    stack.push(a * b);
                }
                Op::Lt => {
                    let b = stack.pop().unwrap();
                    let a = stack.pop().unwrap();
                    stack.push(if a < b { 1 } else { 0 });
                }
                Op::Jump(target) => {
                    ip = *target;
                    continue;
                }
                Op::JumpIfZero(target) => {
                    let val = stack.pop().unwrap();
                    if val == 0 {
                        ip = *target;
                        continue;
                    }
                }
                Op::Call(target) => {
                    let arg = stack.pop().unwrap();
                    let result = self.execute(*target, &[arg]);
                    stack.push(result);
                }
                Op::Return => return stack.pop().unwrap_or(0),
                Op::Print => {
                    let val = stack.pop().unwrap();
                    println!("    [print] {val}");
                }
            }

            ip += 1;
        }

        stack.pop().unwrap_or(0)
    }
}

// ── Hot Spot Detection ────────────────────────────────────────────────────

fn detect_hot_spots(interpreter: &Interpreter, threshold: usize) -> Vec<usize> {
    interpreter
        .call_counts
        .iter()
        .enumerate()
        .filter(|(_, &count)| count >= threshold)
        .map(|(id, _)| id)
        .collect()
}

// ── Simplified JIT Code Generator ─────────────────────────────────────────

struct CompiledFunc {
    func_id: usize,
    instruction_count: usize,
}

fn jit_compile(func: &Func) -> CompiledFunc {
    // In a real JIT, this would emit native machine code.
    // Here we simulate by counting instructions that would be emitted.
    let mut inst_count = 0;
    for op in &func.bytecode {
        match op {
            Op::Const(_) => inst_count += 2, // mov rax, imm
            Op::LoadLocal(_) => inst_count += 3, // mov rax, [rbp-offset]
            Op::StoreLocal(_) => inst_count += 3, // mov [rbp-offset], rax
            Op::Add | Op::Sub | Op::Mul => inst_count += 2, // pop rbx; add/mul rax, rbx
            Op::Lt => inst_count += 4, // pop rbx; cmp; setl; movzx
            Op::Jump(_) => inst_count += 1, // jmp label
            Op::JumpIfZero(_) => inst_count += 2, // test rax; jz label
            Op::Call(_) => inst_count += 5, // push args, call, pop result
            Op::Return => inst_count += 1, // ret
            Op::Print => inst_count += 10, // call printf (expensive)
        }
    }

    CompiledFunc {
        func_id: func.name.parse().unwrap_or(0),
        instruction_count: inst_count,
    }
}

// ── Inline Cache ──────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum InlineCache {
    Uninitialized,
    Monomorphic {
        shape: String,
        offset: usize,
        hit_count: usize,
    },
    Polymorphic {
        entries: Vec<(String, usize)>,
        hit_count: usize,
        miss_count: usize,
    },
}

struct PropertyAccess {
    cache: InlineCache,
    property: String,
}

impl PropertyAccess {
    fn new(property: &str) -> Self {
        Self {
            cache: InlineCache::Uninitialized,
            property: property.to_string(),
        }
    }

    fn lookup(&mut self, shape: &str, actual_offset: usize) -> (usize, bool) {
        match &mut self.cache {
            InlineCache::Uninitialized => {
                self.cache = InlineCache::Monomorphic {
                    shape: shape.to_string(),
                    offset: actual_offset,
                    hit_count: 0,
                };
                (actual_offset, false) // miss
            }
            InlineCache::Monomorphic {
                shape: cached_shape,
                offset,
                hit_count,
            } => {
                if cached_shape == shape {
                    *hit_count += 1;
                    (*offset, true) // hit
                } else {
                    // Upgrade to polymorphic
                    let old_entry = (cached_shape.clone(), *offset);
                    self.cache = InlineCache::Polymorphic {
                        entries: vec![old_entry, (shape.to_string(), actual_offset)],
                        hit_count: 0,
                        miss_count: 1,
                    };
                    (actual_offset, false) // miss
                }
            }
            InlineCache::Polymorphic {
                entries,
                hit_count,
                miss_count,
            } => {
                for (cached_shape, offset) in entries.iter() {
                    if cached_shape == shape {
                        *hit_count += 1;
                        return (*offset, true); // hit
                    }
                }
                *miss_count += 1;
                if entries.len() < 4 {
                    entries.push((shape.to_string(), actual_offset));
                }
                (actual_offset, false) // miss
            }
        }
    }

    fn stats(&self) -> String {
        match &self.cache {
            InlineCache::Uninitialized => "uninitialized".to_string(),
            InlineCache::Monomorphic {
                shape,
                offset,
                hit_count,
            } => format!("monomorphic(shape={shape}, offset={offset}, hits={hit_count})"),
            InlineCache::Polymorphic {
                entries,
                hit_count,
                miss_count,
            } => format!(
                "polymorphic({} shapes, hits={}, misses={})",
                entries.len(),
                hit_count,
                miss_count
            ),
        }
    }
}

fn inline_cache_demo() {
    println!("\n=== Inline Cache Demo ===\n");

    let mut access = PropertyAccess::new("x");

    // Simulate property access patterns
    let accesses = [
        ("Point", 0),
        ("Point", 0),
        ("Point", 0),
        ("Point", 0),
        ("Point", 0),
        ("Rect", 2),  // different shape — monomorphic → polymorphic
        ("Rect", 2),
        ("Rect", 2),
        ("Circle", 1), // third shape added to polymorphic cache
        ("Point", 0),
    ];

    let mut hits = 0;
    let mut misses = 0;

    for (shape, offset) in accesses {
        let (_, was_hit) = access.lookup(shape, offset);
        if was_hit {
            hits += 1;
            println!("  Access .x on {shape} → HIT (cached)");
        } else {
            misses += 1;
            println!("  Access .x on {shape} → miss (cached now)");
        }
    }

    println!("\n  Results: {hits} hits, {misses} misses");
    println!("  Cache state: {}", access.stats());
}

// ── Hidden Classes Demo ───────────────────────────────────────────────────

fn hidden_classes_demo() {
    println!("\n=== Hidden Classes (Shapes) Demo ===\n");

    // Simulate V8's hidden class transitions
    let mut shape_table: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut shape_counter = 0;

    fn get_or_create_shape(
        table: &mut HashMap<String, HashMap<String, usize>>,
        counter: &mut usize,
        properties: &[&str],
    ) -> String {
        let key = properties.join(",");
        if let Some(shape) = table.keys().find(|k| table[*k].keys().eq(properties.iter().map(|s| s.to_string()))) {
            return shape.clone();
        }
        let shape_id = format!("S{counter}");
        *counter += 1;
        let mut offsets = HashMap::new();
        for (i, &prop) in properties.iter().enumerate() {
            offsets.insert(prop.to_string(), i);
        }
        table.insert(shape_id.clone(), offsets);
        shape_id
    }

    // Create objects with same property order → same shape
    let s1 = get_or_create_shape(&mut shape_table, &mut shape_counter, &["x", "y"]);
    let s2 = get_or_create_shape(&mut shape_table, &mut shape_counter, &["x", "y"]);
    let s3 = get_or_create_shape(&mut shape_table, &mut shape_counter, &["x", "y"]);
    println!("  Point objects: p1={s1}, p2={s2}, p3={s3}");

    // Different property order → different shape
    let s4 = get_or_create_shape(&mut shape_table, &mut shape_counter, &["y", "x"]);
    println!("  Reversed order: {s4} (different shape!)");

    // Add property creates new shape
    let s5 = get_or_create_shape(&mut shape_table, &mut shape_counter, &["x", "y", "z"]);
    println!("  Extended: {s5}");

    println!("\n  Total shapes created: {shape_counter}");
    println!("  Benefits: objects with same shape share property offsets,");
    println!("  enabling inline caching to use direct memory reads.");
}

// ── Benchmark: Interpreter vs Simulated JIT ────────────────────────────────

fn benchmark() {
    println!("\n=== Interpreter vs JIT Benchmark ===\n");

    // Function 0: compute sum of 0..n (interpreted)
    // bytecode: loop from 0 to n, accumulate
    let sum_func = Func {
        name: "sum".to_string(),
        params: 1,
        locals: 3, // [n, i, acc]
        bytecode: vec![
            Op::StoreLocal(0),       // local[0] = n
            Op::Const(0),
            Op::StoreLocal(1),       // local[1] = i = 0
            Op::Const(0),
            Op::StoreLocal(2),       // local[2] = acc = 0
            // loop start (offset 6):
            Op::LoadLocal(1),        // i
            Op::LoadLocal(0),        // n
            Op::Lt,                  // i < n
            Op::JumpIfZero(19),      // if !(i < n) jump to end
            Op::LoadLocal(2),        // acc
            Op::LoadLocal(1),        // i
            Op::Add,                 // acc + i
            Op::StoreLocal(2),       // acc = acc + i
            Op::LoadLocal(1),        // i
            Op::Const(1),
            Op::Add,                 // i + 1
            Op::StoreLocal(1),       // i = i + 1
            Op::Jump(6),             // jump to loop start
            // end (offset 19):
            Op::LoadLocal(2),        // return acc
            Op::Return,
        ],
        call_count: 0,
        compiled: false,
    };

    let functions = vec![sum_func];
    let mut interp = Interpreter::new(functions);

    // Run the sum function many times to simulate hot loop
    let iterations = 1000;
    let n = 100i64;

    let start = Instant::now();
    let mut result = 0;
    for _ in 0..iterations {
        result = interp.execute(0, &[n]);
    }
    let interp_time = start.elapsed();

    println!("  Sum(0..{n}) = {result}");
    println!("  Interpreted: {iterations} calls in {interp_time:?}");
    println!("  Call count: {}", interp.call_counts[0]);

    // Detect hot spots
    let hot = detect_hot_spots(&interp, 100);
    println!("\n  Hot functions (threshold=100): {:?}", hot);

    // Simulate JIT compilation of hot functions
    let compiled = jit_compile(&interp.functions[0]);
    println!(
        "  JIT would emit ~{} native instructions for sum()",
        compiled.instruction_count
    );

    // Simulated JIT speedup (native code runs ~10-50x faster for tight loops)
    let jit_factor = 25.0;
    let estimated_jit_time_us = interp_time.as_micros() as f64 / jit_factor;
    println!(
        "  Estimated JIT time: ~{estimated_jit_time_us:.0} µs ({jit_factor:.0}x speedup)"
    );

    println!("\n  Tiered compilation summary:");
    println!("    Interpreter  → 0 calls   → always interpreted");
    println!("    Baseline JIT → 100 calls  → quick native code");
    println!("    Opt JIT      → 10k calls  → optimized with type info");
}

// ── OSR Demo ──────────────────────────────────────────────────────────────

fn osr_demo() {
    println!("\n=== On-Stack Replacement (OSR) Demo ===\n");

    println!("  Problem: hot loop inside a rarely-called function");
    println!("");
    println!("  function init() {        // called once");
    println!("    for (i = 0; i < 10M) { // hot loop!");
    println!("      compute(i);");
    println!("    }");
    println!("  }");
    println!("");
    println!("  Without OSR: function call count = 1, never triggers JIT");
    println!("  With OSR: detect hot loop back-edge, compile mid-execution");
    println!("");
    println!("  Steps:");
    println!("    1. Interpreter counts back-edges (loop iterations)");
    println!("    2. Threshold reached (e.g., 10,000 iterations)");
    println!("    3. JIT compiles function, maps interpreter state → compiled state");
    println!("    4. Replace current stack frame with compiled frame");
    println!("    5. Continue executing native code");
    println!("");
    println!("  LuaJIT uses OSR extensively — traces are compiled on loop back-edges");
    println!("  JVM enables OSR for tiered compilation of hot loops");
}

// ── Main ───────────────────────────────────────────────────────────────────

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Lesson 20: JIT Compilation Simulation                 ║");
    println!("║  Hot Spots, Inline Caching, Hidden Classes, OSR        ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    inline_cache_demo();
    hidden_classes_demo();
    benchmark();
    osr_demo();

    println!("\n=== JIT Compilation Summary ===\n");
    println!("  Concept              | Purpose");
    println!("  ─────────────────────────────────────────────────────");
    println!("  Tiered compilation   | Spend compile time only on hot code");
    println!("  Inline caching       | Cache method/property lookups");
    println!("  Hidden classes       | Fast property access via shapes");
    println!("  OSR                  | Compile hot loops mid-execution");
    println!("  Deoptimization       | Bail out when type assumptions fail");
}
