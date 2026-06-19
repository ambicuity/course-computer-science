# Lesson 16: Register Allocation — Linear Scan vs Graph Coloring

Registers are the fastest storage on a CPU — zero latency, directly on the datapath. A compiler's register allocator decides which values live in registers at each program point, and which spill to memory. Getting this right is the difference between fast code and slow code.

## The Problem

Compilers generate intermediate code with an unlimited supply of **virtual registers** (infinite temporaries). But real CPUs have a finite set — typically 16 general-purpose registers on x86-64, 31 on AArch64, 32 on RISC-V. Register allocation maps virtual registers to physical registers. Two variables that are **live at the same time** — meaning both hold values that will be used later — cannot share a register. When there aren't enough registers, some values must be **spilled** to the stack, requiring load and store instructions.

The mapping must satisfy one constraint: **at every program point, two variables that are simultaneously live must occupy different physical registers**. Violating this produces incorrect code. The allocator's job is to find a mapping that satisfies this constraint while minimizing spills.

Modern architectures complicate this further: not all registers are interchangeable. Some are dedicated to the stack pointer, frame pointer, or return address. Calling conventions reserve registers for function arguments and return values. The allocator must respect these constraints.

## Liveness Analysis

Before allocation, the compiler computes **liveness**: at each point in the program, which variables are live? A variable is live from its **definition** (write) to its last **use** (read). The range is called a **live interval**.

```
t1 = a + b      // def t1
t2 = t1 * c     // use t1, def t2
t3 = t2 + d     // use t2, def t3
// t1 dead after line 2
```

Liveness is computed backward: start from uses and propagate. A variable is live at a point if there exists a path from that point to a use, with no redefinition on the path.

The data structure used is a **live interval** — a contiguous range [start, end] in the instruction stream. Real compilers extend this with multiple sub-ranges for variables that become dead and then live again inside loops, but the linear approximation is sufficient for most programs.

## Interference Graph

Two variables **interfere** if they are live at the same time. The **interference graph** has a node per virtual register and an edge between any pair that interfere.

```
t1 live: [1, 2]
t2 live: [2, 3]
t3 live: [3, 3]

Interference: t1—t2 (overlap at line 2), t2—t3 (overlap at line 3)
No edge: t1—t3 (no overlap)
```

If we can color this graph with K colors (where K = number of physical registers), every virtual register gets a physical register and no two interfering variables share one.

The interference graph is undirected. Its maximum clique (fully connected subgraph) represents the peak register pressure — at that program point, at least that many registers are needed. If peak pressure exceeds K, spilling is unavoidable.

## Graph Coloring: Chaitin-Briggs

The classic algorithm (Chaitin 1981, improved by Briggs 1994):

1. **Build**: construct the interference graph from liveness.
2. **Coalesce**: if two nodes are connected by a `move` instruction and do not interfere, merge them into one — this eliminates the move.
3. **Simplify**: repeatedly remove a node with fewer than K neighbors (it is guaranteed to be colorable after the rest is). Push it onto a stack.
4. **Spill**: if no node has fewer than K neighbors, select one as a **spill candidate**. Mark it and continue simplifying.
5. **Select**: pop nodes from the stack, assign each a color not used by its already-assigned neighbors. If a spill candidate has no color, it actually spills — insert loads and stores, rebuild intervals, and repeat.

Chaitin-Briggs produces high-quality allocations but is **expensive**: building and coloring the interference graph is O(V^2) in the number of virtual registers. For ahead-of-time (AOT) compilation this is acceptable. For just-in-time (JIT) compilation, it's too slow.

### Why Simplification Works

A node with fewer than K neighbors is guaranteed to be colorable after its neighbors are colored — there are at most K-1 colors used, so one of the K colors remains free. By removing such nodes first and coloring them last (on the stack), we guarantee that each popped node finds a free color. Only when **all** remaining nodes have degree ≥ K is a spill necessary.

### Pre-colored Nodes

Some registers have fixed assignments — the stack pointer (`sp`), return address (`ra`), or registers dictated by the calling convention (`a0`–`a7` for arguments). These appear as **pre-colored nodes** in the interference graph. They are never simplified, but they constrain their neighbors: a pre-colored node using color `c` forces all neighbors to avoid `c`.

### Spill Heuristics

When selecting a spill candidate, the compiler considers **spill cost** — the number of loads and stores inserted if this variable is spilled. Variables inside loops have higher spill cost because each loop iteration adds a load and store. The simplest heuristic picks the node with the highest degree (most constrained), but weighted heuristics that account for loop nesting depth produce better results.

## Linear Scan

Linear scan (Poletto 1999) takes a simpler approach:

1. Compute live intervals for all virtual registers.
2. Sort intervals by **start point** (the first definition).
3. Scan through sorted order, assigning registers greedily.
4. Maintain a list of **active** intervals (ones that are currently live).
5. When assigning a register for a new interval:
   - If a free register exists, use it.
   - Otherwise, **expire** old intervals (remove those that ended before the current start). If still no register, spill the interval with the farthest end.

Linear scan runs in **O(V log V)** time — dominated by sorting. It's 5–10× faster than graph coloring, producing slightly worse code. Production JITs (HotSpot C1, V8 Crankshaft, LuaJIT) use linear scan. Production AOT compilers (GCC, LLVM, rustc) use graph coloring (or variants like PBQP).

### Why Linear Scan Is Fast

There is no graph construction. The algorithm works entirely on sorted intervals, performing a single pass. Register decisions are greedy and local — no backtracking. The "expire" step is efficient because the active list is sorted by end point, so removing expired intervals is a linear scan of the front of the list.

### Live Range Splitting

When a variable has high register pressure in one region but is easy to keep in a register elsewhere, the allocator can **split** its live range: assign a register in the low-pressure region and a spill slot in the high-pressure region, with a move instruction at the boundary. This is more efficient than spilling the entire variable.

## Trade-offs

| Aspect | Graph Coloring | Linear Scan |
|---|---|---|
| Code quality | Optimal / near-optimal | Good, slightly worse |
| Compile time | O(V^2) | O(V log V) |
| Spill count | Lower | Higher |
| Used by | GCC, LLVM | HotSpot, V8, LuaJIT |
| Best for | AOT compilation | JIT compilation |

## Build It

Implement both allocators. Start with liveness analysis, build the interference graph, then implement each algorithm. Compare spill counts and execution time on the same input program.

## Use It

Production compilers rarely use textbook algorithms exactly. LLVM uses a **greedy allocator** (not purely linear scan, but inspired by it) with coalescing and live-range splitting. GCC uses a **Chaitin-style graph coloring** with extensions (PBQP for register-rich architectures). V8's TurboFan uses a **turboshaft allocator** based on linear scan with hints from escape analysis.

## Ship It: Register Allocator

A complete register allocator takes IR instructions and produces a register assignment — a mapping from virtual registers to physical registers or spill slots.

## Exercises

**Level 1 — Understand**: Given the following live intervals, trace the linear scan algorithm with 3 available registers. Which intervals spill?

```
v1: [1, 4]
v2: [2, 6]
v3: [5, 9]
v4: [3, 7]
v5: [1, 10]
```

Show your work: for each interval, list the active set before assignment, which register (if any) is free, and whether a spill occurs.

Hint: after sorting by start point, scan left to right. Expire any active interval whose end is before the current start.

**Level 2 — Implement**: Extend the graph coloring allocator to handle **move coalescing** — merge nodes connected by move instructions when they don't interfere. Verify that coalescing reduces the total number of moves in the output.

**Level 3 — Optimize**: Implement a **spill cost heuristic** for the graph coloring allocator. The cost of spilling a variable should be proportional to the number of its uses and definitions (more accesses = higher cost). Test on programs with loops where some variables are accessed far more frequently than others.
