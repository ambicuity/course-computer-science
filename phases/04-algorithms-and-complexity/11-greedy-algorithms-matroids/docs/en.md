# Greedy Algorithms & Matroids

> At each step, pick the locally best option and hope the globally best solution falls out. Sometimes it does — and matroids tell you *when*.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 04 lessons 01–10
**Time:** ~75 minutes

## Learning Objectives

- Implement activity selection, Huffman coding, fractional knapsack, and job sequencing from scratch.
- Understand when greedy works vs when it fails (greedy vs DP).
- Formally define matroids and prove greedy is optimal on matroid structures.

## The Problem

Greedy algorithms appear everywhere — Dijkstra, Kruskal, Huffman, interval scheduling — but using them blindly produces wrong answers. The key question: **when does a locally optimal choice guarantee a globally optimal solution?**

Matroid theory gives a complete answer: greedy is optimal *if and only if* the feasible sets form a matroid.

## The Concept

### Greedy Choice Property

A problem has the **greedy choice property** if a globally optimal solution can be arrived at by making locally optimal choices. Two properties must hold:

1. **Greedy choice property** — a locally optimal choice is safe (part of *some* optimal solution).
2. **Optimal substructure** — after making a greedy choice, the remaining subproblem has an optimal solution that combines with the choice.

### Greedy vs DP: The Coin Change Test

Coins `[1, 5, 10]`, target 12. Greedy picks 10+1+1 = 3 coins. Optimal is 3 coins. Works.

Coins `[1, 3, 4]`, target 6. Greedy picks 4+1+1 = 3 coins. Optimal is 3+3 = 2 coins. **Fails.**

The difference: [1,5,10] is a **canonical** coin system. [1,3,4] is not. When greedy fails, DP is required.

### Activity Selection

Given activities with start/finish times, select max non-overlapping set.

**Strategy:** sort by finish time, greedily pick the next activity whose start ≥ last selected finish. Choosing the earliest-finishing activity leaves the most room for future choices — this is provably safe.

### Huffman Coding

Build variable-length prefix-free binary codes that minimise expected encoded length.

**Strategy:** repeatedly merge the two lowest-frequency nodes. The two least-frequent characters must be the deepest leaves in any optimal prefix tree (exchange argument), so merging them first preserves optimality.

### Fractional Knapsack

Maximise value within capacity W, items can be split.

**Strategy:** sort by value/weight ratio descending, take as much of each as possible. Taking the highest-ratio item first is always at least as good. This fails for 0/1 knapsack (no splitting), which needs DP.

### Job Sequencing

Schedule jobs with deadlines to maximise profit (each job = 1 time unit).

**Strategy:** sort by profit descending, place each job in the latest available slot before its deadline. Scheduling the highest-profit job as late as possible preserves slots for others.

### Matroid Theory

A **matroid** M = (E, I) has a finite ground set E and independent sets I ⊆ 2^E satisfying:

1. **Hereditary:** if B ∈ I and A ⊆ B, then A ∈ I.
2. **Exchange:** if A, B ∈ I and |A| < |B|, then ∃x ∈ B \ A such that A ∪ {x} ∈ I.

| Matroid | Ground set E | Independent sets I |
|---------|-------------|-------------------|
| Graphic matroid | Edges of a graph | Acyclic edge sets (forests) |
| Vector matroid | Columns of a matrix | Linearly independent subsets |
| Partition matroid | Partitioned elements | At most one per group |
| Uniform U(k,n) | n elements | Subsets of size ≤ k |

**The theorem:** greedy is optimal for *all* weight functions iff the feasible sets form a matroid. This characterisation is the complete answer to "when does greedy work?"

### Use It

- **Dijkstra** is greedy — always relaxes the closest unvisited vertex.
- **Kruskal's MST** is greedy on the **graphic matroid** — adds edges by weight, skipping cycles. The cycle check is the independence test.

### Read the Source

- `huffmanq.c` in [gzip/zlib](https://github.com/madler/zlib) — production Huffman for DEFLATE compression.
- `kruskal.rs` in [petgraph](https://github.com/petgraph/petgraph) — Rust graph lib with Kruskal + Union-Find.

### Ship It

The reusable artifact is a **Huffman encoder/decoder** — see `code/main.py`. It compresses text to ~48% of original size for typical English text.

## Build It

All implementations are in `code/main.py`. Key functions:

- `activity_selection(activities)` — sort by finish time, select non-overlapping. O(n log n).
- `huffman_encode(text)` / `huffman_decode(encoded, tree)` — build tree from frequencies, encode/decode. O(n log n).
- `fractional_knapsack(items, W)` — sort by value/weight ratio. O(n log n).
- `job_sequencing(jobs)` — sort by profit, fill latest available slot. O(n²).
- `is_matroid(elements, independent_fn)` — verify hereditary + exchange properties.
- `optimal_merge(files)` — Huffman-style merge to minimise total cost.

Run `python3 code/main.py` to see all demos.

## Exercises

1. **Prove greedy fails for coin change [1, 3, 4].** Trace greedy for target 6: it produces [4,1,1] = 3 coins. The optimal [3,3] = 2 coins. The greedy choice property fails because picking the largest coin first (4) leaves a subproblem (2) that requires 2 more coins.

2. **Implement optimal merge pattern.** Given file sizes, repeatedly merge the two smallest. Prove this minimises total cost by showing equivalence to Huffman tree construction — the merge cost equals the weighted path length of the tree.

3. **Prove the graphic matroid exchange property.** Let E be edges of a connected graph, A and B be forests with |A| < |B|. Prove ∃e ∈ B \ A such that A ∪ {e} is acyclic. (Hint: B has a connected component with more edges than A's corresponding component, so B contains an edge within that component not in A's spanning forest.)

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Greedy choice property | "Just pick the best option" | A locally optimal choice is part of some globally optimal solution |
| Optimal substructure | "Break it into subproblems" | Optimal solution contains optimal solutions to subproblems |
| Matroid | "Structure where greedy works" | (E, I) satisfying hereditary + exchange property on independent sets |
| Exchange property | "Swapping keeps independence" | If |A| < |B| both independent, some element of B\A extends A |
| Hereditary property | "Subsets stay independent" | Every subset of an independent set is independent |
| Canonical coin system | "Greedy works for change" | Denominations where greedy always produces fewest coins |
| Prefix-free code | "No code is a prefix of another" | Variable-length encoding where no code is a prefix of any other |

## Further Reading

- Cormen et al., *Introduction to Algorithms*, Ch. 16 (Greedy) and Ch. 23 (MST).
- Kozen, *Design and Analysis of Algorithms*, Ch. 12 — concise matroid treatment.
- Oxley, *Matroid Theory* — comprehensive algebraic reference.
