# SSA Form — Construction and Dominance

> Every variable assigned exactly once. Optimization made tractable.

**Type:** Learn
**Languages:** Rust
**Prerequisites:** Phase 08 lessons 01–12
**Time:** ~90 minutes

## Learning Objectives

- Define SSA and explain why compilers use it.
- Compute dominance relations and dominance frontiers on a CFG.
- Insert φ-functions and rename variables to convert IR into SSA form.
- Compare your implementation against LLVM's SSA-based IR.

## Why This Matters

Optimization passes need to answer: "which definition reaches this use?" In ordinary three-address code a variable can be assigned many times, and reaching-definition analysis is expensive and must be re-run after every change. **Static Single Assignment (SSA)** eliminates the problem structurally: every variable is assigned exactly once. After that simplification, most classic optimizations become local, one-pass algorithms.

## The Concept

### SSA: One Name, One Definition

Consider this IR:

```
x = 1
if (cond)
    x = 2
y = x + 3
```

In SSA form, each assignment gets a unique version:

```
x₁ = 1
if (cond)
    x₂ = 2
x₃ = φ(x₁, x₂)     ← merges the two reaching defs
y₁ = x₃ + 3
```

The **φ-function** (phi) appears at join points in the CFG where a variable has different definitions on different incoming edges. It selects the correct value based on which edge was taken.

### Dominance

Node **A dominates B** (A dom B) if every path from the entry node to B passes through A. Every node dominates itself.

```
Entry
 ├──> B1
 ├──> B2
 │     └──> B3
 └──> B3

Entry dom {Entry, B1, B2, B3}
B1 dom {B1} only
B2 dom {B2, B3}
B3 dom {B3}
```

The **immediate dominator** (idom) of a node is its closest strict dominator. Immediate dominators form a **dominator tree**.

### Dominance Frontier

The **dominance frontier** DF(X) of a node X is the set of all nodes Y where:
- X dominates a predecessor of Y, but
- X does not strictly dominate Y.

Intuition: the frontier is where a definition in X "stops" being guaranteed to reach. These are exactly the places where φ-functions must be inserted.

```
DF(B1) = {B3}   (B1 dominates B1's predecessors... B3 has B2 as predecessor
                  which B1 doesn't dominate)
DF(B2) = {B3}
```

### SSA Construction Algorithm (Cytron et al.)

The classic algorithm has three phases:

1. **Compute dominance.** Use the iterative dataflow algorithm or the Lengauer-Tarjan algorithm to build the dominator tree.

2. **Insert φ-functions.** For each variable v, find all blocks that contain assignments to v (the def-site set). Compute iterated dominance frontier IDF(defs(v)). Insert a φ-function for v in every block in IDF.

3. **Rename variables.** Walk the dominator tree. For each assignment, create a new version. For each φ, fill in the operand corresponding to each predecessor edge.

### Worked Example

Original code:
```
Entry:   a = 1
         b = 2
         if (c) goto L1 else goto L2
L1:      a = b + 3
         goto L3
L2:      b = a * 2
         goto L3
L3:      d = a + b
         print(d)
```

**Step 1 — Dominance:**
```
dom(Entry) = {Entry, L1, L2, L3}
dom(L1) = {L1}
dom(L2) = {L2}
dom(L3) = {L3}
idom(L1) = Entry, idom(L2) = Entry, idom(L3) = Entry
```

**Step 2 — Insert φ:**
- `a` is defined in Entry, L1 → defs(a) = {Entry, L1}
  - IDF({Entry, L1}) = {L3} → insert `a₃ = φ(a₁, a₂)` at L3
- `b` is defined in Entry, L2 → defs(b) = {Entry, L2}
  - IDF({Entry, L2}) = {L3} → insert `b₃ = φ(b₁, b₂)` at L3

**Step 3 — Rename:**
```
Entry:   a₁ = 1
         b₁ = 2
         if (c) goto L1 else goto L2
L1:      a₂ = b₁ + 3
         goto L3
L2:      b₂ = a₁ * 2
         goto L3
L3:      a₃ = φ(a₁, a₂)
         b₃ = φ(b₁, b₂)
         d₁ = a₃ + b₃
         print(d₁)
```

Every variable now has exactly one definition. Optimization passes can now treat each definition independently.

## Build It

The code in `code/main.rs` implements SSA construction on a simple control-flow graph:

- `BasicBlock` — a labeled sequence of instructions with predecessor/successor edges.
- `CFG` — a collection of blocks with an entry point.
- `compute_dominators(cfg)` — iterative dominance computation.
- `compute_dom_frontiers(cfg, dominators)` — iterated frontier calculation.
- `insert_phi_functions(cfg, def_sites, dom_frontiers)` — φ insertion.
- `rename_variables(cfg)` — versioned-variable renaming.
- `to_ssa(cfg)` — the full pipeline.

Run it:

```bash
cd code && cargo run --quiet
```

The output shows the original CFG, the dominator tree, the dominance frontiers, and the final SSA-form IR.

## Use It

LLVM IR is natively in SSA form. Every register (`%0`, `%1`, ...) is assigned exactly once. φ-nodes appear as `phi` instructions at block entry:

```llvm
L3:
  %a3 = phi i32 [ %a1, %L1 ], [ %a2, %L2 ]
  %b3 = phi i32 [ %b1, %L1 ], [ %b2, %LL2 ]
  %d1 = add i32 %a3, %b3
```

LLVM's SSA construction is built into its IR format — there is no separate "convert to SSA" pass because the IR is always SSA. The algorithm LLVM uses for out-of-SSA translation (when converting back to machine code with registers) is based on computing live ranges and inserting copies.

## Read the Source

- `llvm/lib/IR/SSAUpdater.cpp` — LLVM's on-the-fly SSA updater, which can insert φ-functions lazily during optimization.
- `llvm/lib/Transforms/Utils/PromoteMemToReg.cpp` — The `mem2reg` pass that promotes `alloca` slots to SSA registers. This is how LLVM builds SSA from frontends that emit stack-based IR.

## Ship It

The reusable artifact is the SSA construction pipeline. Later lessons on dataflow analysis, optimization passes, and register allocation all assume SSA-form input. The dominator tree is also reused by LICM (Lesson 14) and loop analysis (Lesson 15).

## Exercises

1. **Easy** — Trace the SSA construction algorithm by hand on this CFG: Entry → {B1, B2} → B3, with assignments `x = 1` in Entry, `x = 2` in B1, and `y = x + 1` in B3. Verify the φ-function placement.

2. **Medium** — Extend the Rust implementation to handle loops. Add a back edge (L3 → L1) to the demo CFG and verify that φ-functions are inserted correctly at the loop header.

3. **Hard** — Implement the Lengauer-Tarjan algorithm for dominator computation (O(E·α(E,V)) instead of the naive O(N²) iterative algorithm). Compare the results with the iterative version on a CFG with 50+ blocks.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| SSA | "Static single assignment" | Every variable name has exactly one assignment in the program text; φ-functions merge versions at join points |
| φ-function | "Phi node" | A pseudo-instruction at CFG join points that selects a value based on which predecessor edge was taken |
| Dominator | "A dominates B" | Every path from entry to B passes through A. Forms a tree (idom relation) |
| Dominance frontier | "Where a definition stops dominating" | The set of nodes where a definition from X must be merged with another definition — φ placement targets |
| Out-of-SSA | "SSA destruction" | Translating SSA back to conventional code by inserting copy instructions where φ-functions were |

## Further Reading

- Cytron, R. et al. "Efficiently Computing Static Single Assignment Form and the Control Dependence Graph." TOPLAS, 1991. — The original SSA paper.
- Appel, A. "Modern Compiler Implementation in ML." Ch. 19. — Accessible textbook treatment.
- GCC's SSA infrastructure: `gcc/tree-into-ssa.c`, `gcc/tree-outof-ssa.c`.
