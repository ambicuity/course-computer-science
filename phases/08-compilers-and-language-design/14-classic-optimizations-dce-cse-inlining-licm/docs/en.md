# Classic Optimizations — DCE, CSE, Inlining, LICM

> The five optimizations that every compiler ships. Understand them, implement them, measure them.

**Type:** Learn
**Languages:** Rust
**Prerequisites:** Phase 08 lessons 01–13
**Time:** ~90 minutes

## Learning Objectives

- Explain dead code elimination, common subexpression elimination, constant folding, inlining, and loop-invariant code motion.
- Implement each as a self-contained IR pass in Rust.
- Compare against GCC/LLVM's optimization pipeline at O1, O2, O3.
- Build a reusable pass runner that applies selected optimizations.

## Why This Matters

After the frontend produces IR, the middle-end's job is to make it faster, smaller, or both. The five optimizations in this lesson — DCE, CSE, constant folding, inlining, and LICM — account for the majority of speedup that `-O2` delivers on real programs. If you understand these, you can reason about why one piece of code runs faster than another, and you can build your own optimizer.

## The Concept

### Dead Code Elimination (DCE)

An instruction is **dead** if its result is never used by any subsequent instruction (or the instruction has no side effects and its output is discarded). DCE removes dead instructions.

```
Before DCE:               After DCE:
  x = a + b                 x = a + b
  y = x * 2                 y = x * 2
  z = y + 1                 print(y)
  w = 99          ← dead
  print(y)
```

Algorithm: Mark all instructions whose results are "live" (used by other live instructions, or are outputs). Then remove anything not marked live. Repeat until no changes.

### Common Subexpression Elimination (CSE)

If the same expression is computed twice with the same operands and no intervening modification, reuse the first result.

```
Before CSE:                After CSE:
  t1 = a + b                t1 = a + b
  ...                        ...
  t2 = a + b     ← same     t2 = t1    ← reuse
```

Algorithm: Hash each expression `(op, arg1, arg2)`. When you see a duplicate, replace it with a reference to the first. Works best on SSA form where each variable has one definition.

### Constant Folding and Propagation

If all operands of an expression are known constants, compute the result at compile time.

```
Before:                    After:
  x = 3 + 4                x = 7
  y = x * 2                y = 14
```

**Constant propagation** extends this: once `x = 7` is known, substitute `7` for `x` in subsequent expressions. On SSA form, this is trivial — each variable has one definition.

### Function Inlining

Replace a function call with the body of the called function, substituting arguments for parameters.

```
Before:                    After:
  def add(a, b):            // call inlined
    return a + b            x = 5 + 3
  ...                       y = x * 2
  x = add(5, 3)
```

Benefits: eliminates call overhead, enables further optimization across the call boundary. Costs: code bloat, potential instruction-cache pressure. Most compilers inline small "leaf" functions and heuristic-based candidates.

### Loop-Invariant Code Motion (LICM)

Move computations that produce the same value on every iteration out of the loop.

```
Before LICM:               After LICM:
  loop i = 0..100:          t = x * y        ← hoisted
    t = x * y     ← invariant
    a[i] = t + i              loop i = 0..100:
                                 a[i] = t + i
```

Requirements: the expression must be loop-invariant (operands defined outside the loop or by other invariant instructions) and must dominate all loop exits (so it's guaranteed to execute).

### Optimization Levels

| Level | What it does |
|-------|-------------|
| `-O0` | No optimization. Fastest compile. |
| `-O1` | Basic: constant folding, simple DCE, peephole |
| `-O2` | Standard: CSE, LICM, aggressive DCE, inlining (small functions) |
| `-O3` | Aggressive: loop unrolling, vectorization, profile-guided |
| `-Os` | Optimize for size (like O2 but avoids code bloat) |

### Pass-Based Architecture

Compilers structure optimizations as **passes** — each pass walks the IR and applies one transformation:

```
IR → [Pass: CF] → [Pass: DCE] → [Pass: CSE] → [Pass: LICM] → [Pass: Inline] → IR
```

Passes can be run multiple times (fixed-point iteration) because one pass may create opportunities for another.

## Build It

The code in `code/main.rs` implements five optimization passes on a simple instruction list:

- `dead_code_elimination(instructions)` — mark-and-sweep live analysis, remove dead.
- `cse(instructions)` — hash-based expression table, replace duplicates.
- `constant_folding(instructions)` — evaluate constant expressions, propagate values.
- `inline_call(func, call_site)` — substitute function body at call site.
- `licm(loop_body, loop_header)` — identify invariant instructions, hoist.
- `optimize(instructions, passes)` — run selected passes in order.

Each pass prints before/after IR so you can see the transformation.

Run it:

```bash
cd code && cargo run --quiet
```

## Use It

GCC organizes its optimization passes in `gcc/passes.def`. The `-O2` pipeline runs roughly 80 passes including CSE (`cse.cc`), DCE (`tree-ssa-dce.cc`), and LICM (`tree-ssa-loop-im.cc`).

LLVM's pass manager runs passes in `lib/Passes/PassBuilderPipelines.cpp`. Key passes:
- `InstCombine` — peephole + CSE at the instruction level.
- `SimplifyCFG` — dead branch elimination.
- `LoopInvariantCodeMotion` — LICM on loop nests.
- `InlineCostAnalysis` — cost-based inlining heuristic.

Both compilers iterate passes to a fixed point — running DCE might expose new CSE opportunities, and vice versa.

## Read the Source

- `llvm/lib/Transforms/Scalar/LICM.cpp` — LLVM's loop-invariant code motion pass. Look at `hoist()` and `isLoopInvariant()`.
- `llvm/lib/Transforms/InstCombine/InstructionCombining.cpp` — LLVM's instruction combiner, which performs CSE, constant folding, and algebraic simplification in a single pass.
- `gcc/tree-into-ssa.cc` — GCC's SSA construction pass (pairs with Lesson 13).

## Ship It

The artifact is a pass runner that can apply any subset of {DCE, CSE, constant folding, inlining, LICM} to an instruction list. You will reuse this in the capstone to optimize the IR your compiler emits.

## Exercises

1. **Easy** — Write a new peephole pass that replaces `x + 0 → x` and `x * 1 → x`. Integrate it into the pass runner.

2. **Medium** — Implement loop-invariant code motion on SSA form. The key invariant: an instruction is hoistable if all its operands are defined outside the loop or by already-hoisted instructions.

3. **Hard** — Implement iterative optimization: run passes repeatedly until the IR reaches a fixed point (no pass changes anything). Measure convergence for a program with nested loops and multiple functions.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| DCE | "Dead code elimination" | Remove instructions whose results are never used and have no side effects |
| CSE | "Common subexpression elimination" | Replace duplicate computations of the same expression with a reference to the first result |
| Constant folding | "Fold constants" | Evaluate expressions with all-constant operands at compile time |
| LICM | "Loop-invariant code motion" | Hoist computations that don't change across iterations out of the loop body |
| Inlining | "Inline the function" | Replace a function call with the callee's body, substituting arguments for parameters |
| Pass | "Optimization pass" | A single traversal of the IR that applies one transformation |

## Further Reading

- Muchnick, S. "Advanced Compiler Design and Implementation." Ch. 12–18. — Comprehensive coverage of each optimization.
- Cooper, K. and Torczon, L. "Engineering a Compiler." Ch. 8–10. — Accessible textbook with pseudocode.
- LLVM's optimization pipeline: `llvm/lib/Passes/PassBuilderPipelines.cpp` — see `buildO2DefaultPipeline()`.
