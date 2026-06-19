# Approximation Algorithms — Vertex Cover, TSP, Set Cover

> Some problems resist exact solutions. Get close enough, prove how close, and ship it.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 04 lessons 01–24 (graphs, greedy, DP, NP-completeness)
**Time:** ~75 minutes

## Learning Objectives

- Define approximation ratio and distinguish PTAS from FPTAS
- Implement 2-approximate Vertex Cover via maximal matching
- Implement metric TSP approximations (MST-walk and Christofides)
- Implement greedy Set Cover with O(log n) approximation guarantee
- Prove approximation bounds for vertex cover and set cover

## The Problem

Many real-world optimization problems are NP-hard — scheduling, routing, feature selection. We need answers fast, even if not exact.

**Approximation algorithms** sacrifice exactness for tractability. They run in polynomial time and guarantee their output is within a known factor of the optimal. The key question: *how close can we get, and can we prove it?*

## The Concept

### Approximation Ratio

An algorithm has **approximation ratio α ≥ 1** if for every valid input:

```
cost(ALG) ≤ α · cost(OPT)    (minimization)
cost(ALG) ≥ α · cost(OPT)    (maximization)
```

A 2-approx for a problem with OPT = 500 returns at most 1000.

**PTAS**: for any fixed ε > 0, poly-time (1+ε)-approx. Runtime may depend exponentially on 1/ε.

**FPTAS**: runtime is polynomial in both n and 1/ε. Stronger — restricted to weakly NP-hard problems.

| Problem | Exact complexity | Best known approx |
|---------|-----------------|-------------------|
| Vertex Cover | O(2ⁿ) | 2-approx in O(V + E) |
| Metric TSP | O(n²·2ⁿ) | 1.5-approx in O(n³) |
| Set Cover | O(2ⁿ) | ln(n)-approx in O(n·m) |
| General TSP | no poly-approx unless P=NP | — |

### Vertex Cover — 2-Approx via Maximal Matching

A **vertex cover** is a set S where every edge has at least one endpoint in S. Minimum vertex cover is NP-hard.

**Algorithm:** Find a maximal matching M greedily. Return both endpoints of every edge in M.

**Proof of 2-approximation:**

Let M be the maximal matching, S the returned cover, OPT the minimum vertex cover.

1. Every edge in M has at least one endpoint in OPT. Since M's edges are disjoint, |OPT| ≥ |M|.
2. |S| = 2|M| (both endpoints of each matching edge).
3. Therefore |S| = 2|M| ≤ 2|OPT|. ∎

The factor 2 is tight (consider k disjoint edges).

### Metric TSP — MST + Preorder Walk

**Metric TSP**: shortest tour visiting all cities, with triangle inequality d(a,c) ≤ d(a,b) + d(b,c).

**Algorithm (2-approx):** Compute MST T. DFS preorder walk from any root. Shortcut past visited nodes.

**Proof:** (1) DFS traverses each MST edge twice → walk_cost = 2·MST_cost. (2) MST_cost ≤ OPT (removing a TSP tour edge gives a spanning tree). (3) Shortcutting never increases cost (triangle inequality). So tour_cost ≤ 2·OPT. ∎

**Christofides (1.5-approx):** (1) Compute MST. (2) Find odd-degree vertices O. (3) Min-weight perfect matching M on O. (4) Euler tour of T∪M, then shortcut. Matching weight ≤ ½·OPT → total ≤ 1.5·OPT. Standing since 1976 — no one has beaten 1.5 for general metric TSP.

### Set Cover — Greedy O(log n)-Approximation

Given universe U and sets S₁…Sₘ, find minimum sets whose union covers U.

**Algorithm:** Repeatedly pick the set covering the most uncovered elements.

**Proof sketch of ln(n) bound:** Assign cost 1/|newly_covered| to each element. Each chosen set contributes exactly 1 to total cost. For any OPT set of size s, elements get costs 1/1 + 1/2 + … + 1/s = H(s) ≤ ln(n) + 1. So greedy ≤ (ln n + 1)·OPT. ∎

The bound is tight — adversarial instances achieve Θ(log n)·OPT.

## Build It

See `code/main.py` for the full implementations. Key functions:

- `greedy_vertex_cover(edges)` — maximal matching, returns 2-approximate cover
- `metric_tsp_approx(dist)` — MST Prim + preorder walk, returns (tour, cost)
- `christofides_tsp(dist)` — 1.5-approx using bitmask DP matching (n ≤ 12)
- `greedy_set_cover(universe, sets)` — O(log n)-approximate cover

Each is paired with a brute-force optimal solver on small instances to verify the approximation ratio empirically.

## Use It

- **VLSI design**: TSP for drilling paths, set cover for test pattern selection.
- **Scheduling**: set cover for time-slot coverage. Vertex cover on conflict graphs.
- **Network design**: set cover for relay station placement. MST-based TSP for fiber routing.
- **Libraries**: `networkx.algorithms.approximation.traveling_salesman` for Christofides. Google OR-Tools for exact/heuristic TSP.

## Read the Source

- `networkx.algorithms.approximation.traveling_salesman` — Christofides implementation.
- Vazirani, *Approximation Algorithms*, Chapters 1–3 — canonical proofs for vertex cover and set cover.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`:

- **A self-contained approximation algorithm library** with vertex cover, TSP, and set cover — all with verified approximation ratios.

## Exercises

1. **Easy** — Reproduce the 2-approx vertex cover proof: show maximal matching returns cover at most 2× OPT.

2. **Medium** — Implement Christofides for small TSP (n ≤ 12). Use bitmask DP for min-weight perfect matching on odd-degree vertices. Verify 1.5-approx on 100 random metric instances.

3. **Hard** — Prove greedy set cover achieves ln(n) approximation. Construct an adversarial instance where greedy hits exactly ⌈ln n⌉ × OPT.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Approximation ratio | "How close to optimal?" | Worst-case bound: cost(ALG) ≤ α·cost(OPT) |
| PTAS | "Can get arbitrarily close" | Poly-time (1+ε)-approx for any fixed ε |
| FPTAS | "Efficiently close" | Poly in n and 1/ε; weakly NP-hard only |
| Vertex cover | "Cover all edges" | Min vertices touching every edge |
| Maximal matching | "Greedy matching" | No edge addable without violating matching |
| Metric TSP | "TSP with triangle inequality" | d(a,c) ≤ d(a,b) + d(b,c) for all a,b,c |
| Christofides | "1.5-approx for TSP" | MST + matching on odd-degree vertices |
| Set cover | "Cover universe with fewest sets" | Min subsets whose union equals U |

## Further Reading

- Vazirani, *Approximation Algorithms* (Springer, 2001) — the definitive textbook.
- Williamson & Shmoys, *The Design of Approximation Algorithms* (Cambridge, 2011) — free online.
- William Cook, *In Pursuit of the Traveling Salesman* — TSP history and algorithms.
