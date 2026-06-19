# Time Complexity Classes — P, NP, EXP

> P, NP, EXP — the complexity landscape that shapes cryptography, optimization, and what "efficient" means.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 05 lessons 01–14
**Time:** ~75 minutes

## Learning Objectives

- Define P, NP, and EXP in terms of Turing machine running time.
- Understand the P vs NP question and why it matters.
- Verify NP membership by implementing verifiers for SAT, Hamiltonian path, and clique.
- See the empirical gap between verifying and solving an NP problem.

## The Problem

This lesson sits in **Phase 05 — Theory of Computation**. Without the concept it teaches, you cannot
build the phase's capstone (A regex engine plus a Turing-machine simulator.). Concretely, *not* knowing this means you get stuck the
moment you try to understand why some problems are hard — and whether "hard" means "impossible" or just "expensive."

Undecidability (lessons 13–14) showed us questions no algorithm can answer. But among *decidable* problems, there is a huge range of difficulty. Complexity classes carve this landscape.

## The Concept

### Complexity Classes (TM model)

**P (Polynomial time):**
```
P = { L | some TM decides L in O(n^k) time for constant k }
```
"Efficiently solvable." Examples: sorting, BFS, shortest path.

**NP (Nondeterministic Polynomial time):**
```
NP = { L | ∃ poly-time verifier V: x ∈ L ⟺ ∃ certificate c, V(x,c) accepts }
```
Solutions *verifiable* in polynomial time. Equivalently: accepted by an NTM in poly time.

**EXP (Exponential time):**
```
EXP = { L | some TM decides L in O(2^{n^k}) time for constant k }
```

### The Hierarchy: P ⊆ NP ⊆ EXP

- **P ⊆ NP:** Solving is a special case of verifying (the solution itself is the certificate).
- **NP ⊆ EXP:** Brute-force over all poly-length certificates is exponential.

### The P vs NP Question

**Open:** Is P = NP? Most believe **P ≠ NP** — finding solutions is fundamentally harder than checking them. Evidence: thousands of NP problems have resisted poly-time algorithms; if P = NP, most cryptography collapses.

### Classic NP Problems

| Problem | Certificate | Verification |
|---|---|---|
| SAT | Truth assignment | Evaluate formula in O(n) |
| Hamiltonian Path | Vertex ordering | Check edges in O(n) |
| Clique | k-vertex subset | Check all edges in O(k²) |
| Subset Sum | Subset of S | Sum and compare in O(n) |

### NP-Completeness (Preview)

A problem is **NP-complete** if it is in NP and every NP problem reduces to it in poly time. SAT was the first (Cook-Levin, 1971). If any NP-complete problem is in P, then P = NP.

## Build It

### NP Verifiers

`code/main.py` contains verifiers that check certificates in polynomial time:

- `sat_verifier(assignment, formula)` — evaluates a CNF formula under a truth assignment.
- `hamiltonian_path_verifier(path, graph)` — checks a vertex ordering visits every node via valid edges.
- `clique_verifier(vertices, k, graph)` — checks k vertices are pairwise adjacent.
- `subset_sum_verifier(subset, numbers, target)` — checks a subset sums to the target.
- `graph_coloring_verifier(coloring, k, graph)` — checks no adjacent vertices share a color.

Each runs in polynomial time relative to the input size.

### Brute-Force SAT Solver

`brute_force_sat(formula, variables)` in `code/main.py` enumerates all 2ⁿ truth assignments and calls `sat_verifier` on each — exponential time, demonstrating why SAT is hard to *solve* even though it is easy to *verify*.

### Empirical Complexity Check

`is_in_p(algorithm, input_sizes)` in `code/main.py` runs an algorithm on increasing input sizes and estimates the time exponent. It uses log-log regression: for T(n) ≈ c·nᵏ, the slope of log(T) vs log(n) gives k.

### Verification Gap Demo

`demonstrate_verification_gap()` compares verification (O(n)) vs brute-force solving (O(2ⁿ)) on a 5-variable SAT instance, printing the time ratio.

## Use It

Cryptography assumes P ≠ NP. RSA, elliptic curves, and lattice-based schemes rely on problems being hard to *solve* but easy to *verify*. If P = NP, these schemes break.

Industry faces NP-hard problems daily: vehicle routing, scheduling, protein folding. Engineers use approximations, heuristics (simulated annealing, genetic algorithms), and relaxations. P vs NP tells you *why* exact solutions are infeasible.

## Ship It

The verifiers and `is_in_p` checker are reusable. Key insight: **verification and solution have fundamentally different costs for NP problems** — this gap makes cryptography possible and optimization hard.

## Exercises

### Level 1 — Identify

Classify each as in P, NP, or NP-complete:
1. Sorting a list of n numbers
2. Determining if a graph has a Hamiltonian cycle
3. Shortest path between two nodes in a weighted graph
4. Determining if a boolean formula is satisfiable

### Level 2 — Implement

Write a verifier for **Graph Coloring**: given graph G and integer k, verify a proposed k-coloring is valid (no adjacent vertices share a color).

### Level 3 — Analyze

Run `is_in_p` on (a) trial division for primality (O(√n)) and (b) brute-force SAT on n variables (O(2ⁿ)). Confirm the empirical exponent matches theory.
