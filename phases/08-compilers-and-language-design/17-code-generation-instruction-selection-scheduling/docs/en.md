# Lesson 17: Code Generation — Instruction Selection, Scheduling

After optimization and register allocation, the compiler must emit real machine code. This final phase is code generation — turning register-allocated IR into target architecture instructions, ordered to exploit the hardware pipeline efficiently.

Code generation is where the compiler meets the hardware. Every design decision — which instruction to use, in what order to place them, how to pass function arguments — directly affects runtime performance.

## Instruction Selection

Instruction selection maps IR operations to sequences of machine instructions. The target architecture's instruction set is more complex than the IR: it has addressing modes, fused multiply-add, bit-field operations, and conditional moves. A good instruction selector exploits these to produce compact, fast code.

### Tree Pattern Matching

The IR can be viewed as a tree (or DAG). Each node is an operation; leaves are operands. The instruction selector tiles this tree with patterns that match subtrees to machine instructions.

For example, on a RISC architecture:

```
IR:   (load (add base (mul index 4)))
RV:   slli t0, a1, 2       # t0 = index * 4
      add  t1, a0, t0       # t1 = base + t0
      lw   a0, 0(t1)        # a0 = *(t1)
```

A CISC architecture like x86-64 can do this in one instruction:

```asm
mov eax, [base + index*4]   ; all in one LEA-style address mode
```

### Optimal Tiling via Dynamic Programming

To minimize the total cost (instruction count, latency, or code size), the selector uses dynamic programming. For each subtree, it computes the cheapest tiling using available patterns and their costs. This produces optimal instruction sequences for a given cost model.

For example, given patterns `{add(r, r, r): cost 1, lea(r, r*8+r): cost 1}`, the selector can tile `(x + x*8)` with a single `lea` instruction (cost 1) instead of a shift + add (cost 2).

In practice, optimal tiling is done with a **BURG** (Bottom-Up Rewrite Generator) or **LLVM's TableGen** — tools that generate matchers from a declarative description of patterns and costs.

## Instruction Scheduling

Modern CPUs are pipelined: multiple instructions are in different stages of execution simultaneously. But a pipeline **stalls** if an instruction needs a result that hasn't been produced yet — a **data hazard**.

Instruction scheduling reorders instructions to minimize these stalls, while preserving the data flow (you can't use an instruction's result before it's computed).

### List Scheduling

The most common algorithm:

1. Build a **dependency graph**: edges connect producer to consumer (read-after-write), and also write-after-write and write-after-read dependencies.
2. Maintain a **ready list**: instructions whose dependencies are all satisfied.
3. At each step, pick the highest-priority ready instruction (priority = longest path to the end — schedule the critical path first).
4. Repeat until all instructions are scheduled.

List scheduling is greedy and produces near-optimal results for single basic blocks.

### Scheduling Example

```
# Unscheduled:
t1 = a + b      # latency 1
t2 = t1 * c     # depends on t1
t3 = d + e      # latency 1, independent
t4 = t3 * f     # depends on t3
t5 = t2 + t4    # depends on t2, t4

# After scheduling (interleave independent chains):
t1 = a + b      # ─┐ chain 1
t3 = d + e      # ─┘ chain 2 (can start immediately)
t2 = t1 * c     # waits for t1
t4 = t3 * f     # waits for t3 (but t3 finishes before t2)
t5 = t2 + t4    # waits for t2, t4
```

Interleaving hides latency: while one multiplier result is being computed, the other chain's multiply can also issue.

### Instruction-Level Parallelism (ILP)

The scheduler's goal is to maximize **instruction-level parallelism** — the number of instructions that can execute simultaneously in the CPU pipeline. A superscalar processor can issue 2–6 instructions per cycle if there are no dependencies. The scheduler fills the pipeline by finding and exploiting this parallelism.

## Calling Conventions

When compiling function calls, the compiler must follow the **calling convention** — the ABI agreement on how arguments and return values are passed.

### RISC-V Calling Convention (RV64ILP32 / LP64)

| Purpose | Registers |
|---|---|
| Arguments (first 8) | `a0`–`a7` |
| Return values | `a0`–`a1` |
| Temporary (caller-saved) | `t0`–`t6` |
| Saved (callee-saved) | `s0`–`s11` |
| Stack pointer | `sp` (x2) |
| Return address | `ra` (x1) |

- Arguments beyond 8 go on the stack.
- The caller pushes extra arguments, calls `jal`, then pops them.
- Callee saves any `s` registers it uses (prologue), restores them before `ret`.

### Prologue / Epilogue

```asm
# prologue (function entry)
addi sp, sp, -32      # allocate stack frame
sd   ra, 24(sp)       # save return address
sd   s0, 16(sp)       # save callee-saved registers

# epilogue (function exit)
ld   s0, 16(sp)       # restore callee-saved
ld   ra, 24(sp)       # restore return address
addi sp, sp, 32       # deallocate frame
ret
```

## Build It

Implement an instruction selector and scheduler targeting RISC-V. Start with a simple IR, map each operation to one or more RISC-V instructions, then schedule for minimal pipeline stalls.

## Use It

LLVM uses **TableGen** to define instruction selection patterns declaratively. The patterns are compiled into a fast tree-pattern-matcher at build time. GCC uses machine descriptions in `.md` files. Both backends pair instruction selection with a list scheduler.

## Ship It: Code Generator

A complete code generator takes register-allocated IR and produces target machine assembly — with prologue/epilogue, calling conventions, and optimized instruction sequences.

## Exercises

**Level 1 — Understand**: Given the IR instruction `t1 = load(base + index * 8)`, write the equivalent RISC-V instructions (three instructions using `slli`, `add`, `ld`). Why can't a single RISC-V instruction do this?

**Level 2 — Implement**: Extend `schedule_instructions` to handle **anti-dependencies** (write-after-read). Rename registers using register copies to break anti-dependencies and allow more instruction-level parallelism.

**Level 3 — Optimize**: Implement a **peephole optimizer** that runs after instruction selection. Define at least three peephole rules (e.g., `add r, r, 0` → remove; `mul r, v, 1` → `mv r, v`; consecutive stores to the same base → merge). Measure the instruction count reduction on a test program.
