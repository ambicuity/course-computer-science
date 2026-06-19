# Matching — Bipartite, Hopcroft-Karp, Hungarian

> Assigning jobs to machines, residents to hospitals, or edges to vertices — matching turns assignment into algorithms.

**Type:** Learn
**Languages:** Python, C++
**Prerequisites:** Phase 04 lessons 01–17 (especially lesson 17 — Network Flow)
**Time:** ~75 minutes

## Learning Objectives

- Understand bipartite matching, augmenting paths, and König's theorem.
- Implement Hopcroft-Karp for O(E√V) maximum matching.
- Implement the Hungarian algorithm for minimum-weight perfect matching.
- Understand Gale-Shapley stable matching and its optimality guarantees.

## The Problem

You have tasks and workers. Each worker can do some subset of tasks. Assign the maximum number of tasks — no worker does two, no task gets two workers. This is **maximum bipartite matching**. Naive enumeration is exponential; you need polynomial-time algorithms, and you need to know *which* variant fits your problem.

## The Concept

### Bipartite Graphs and Matchings

A **bipartite graph** G = (L ∪ R, E) has vertices split into two disjoint sets with edges only between them. A **matching** M ⊆ E is a set of edges sharing no endpoints.

An **augmenting path** alternates between edges not in M and edges in M, starting and ending at unmatched vertices. **Berge's theorem:** *M is maximum iff no augmenting path exists.*

**Example:** L = {A, B, C, D}, R = {1, 2, 3}, edges: A→{1,2}, B→{1,3}, C→{2}, D→{2,3}. Starting from empty matching, we find augmenting path A→1, augment to M={A-1}. Then C→2, augment to M={A-1, C-2}. Then B→3, giving M={A-1, C-2, B-3}. No augmenting path remains — maximum matching size is 3.

An **augmenting path** alternates between edges not in M and edges in M, starting and ending at unmatched vertices. **Berge's theorem:** *M is maximum iff no augmenting path exists.*

**Proof sketch:** If P is augmenting, M ⊕ P increases |M| by 1 → M not maximum. Conversely, if M is not maximum, take M* with |M*| > |M|. The symmetric difference M ⊕ M* decomposes into alternating cycles and paths; at least one component is an augmenting path.

### König's Theorem

In bipartite graphs: **max matching size = min vertex cover size**. Proved via max-flow min-cut on the network source → L → R → sink. The min-cut yields the cover: (L \ reachable) ∪ (R ∩ reachable).

### Hopcroft-Karp: O(E√V)

Sequential augmenting-path search (find one path, augment, repeat) takes O(V·E). Hopcroft-Karp improves this by finding a *maximal set of shortest augmenting paths* simultaneously per phase:

1. **BFS** from all unmatched left vertices, building distance layers. Stop expanding when an unmatched right vertex is first reached — this defines the current shortest augmenting path length d.
2. **DFS** from each unmatched left vertex, following only edges that advance one BFS layer at a time, finding vertex-disjoint augmenting paths of length d.
3. Augment all found paths simultaneously.

**Why it's fast:** Each phase increases matching size by at least 1. After √V phases, the shortest augmenting path has length > √V. A counting lemma (based on vertex-disjoint shortest paths) shows at most √V unmatched vertices remain, bounding total phases at √V. Each phase scans all edges once in BFS and once in DFS, giving O(E√V) total.

### Hungarian Algorithm: O(V³)

Minimum-weight perfect matching via dual variables. Maintain u[i] (left) and v[j] (right) with **dual feasibility**: u[i] + v[j] ≤ w(i,j). Optimal when **complementary slackness** holds: u[i] + v[j] = w(i,j) for matched edges.

The algorithm grows alternating trees from unmatched left vertices, tightening duals until a new edge becomes tight. When an unmatched right vertex is reached, augmentation occurs. See `code/main.py` for the full implementation.

The tight edges (where u[i] + v[j] = w(i,j)) form a subgraph where the current matching lives. The algorithm maintains that the matching on tight edges is always maximum — it just needs to grow the tight subgraph until it covers all vertices.

### Gale-Shapley Stable Matching: O(n²)

Find a **stable matching** — no blocking pair (m,w) where both prefer each other to their partners. **Deferred acceptance:** men propose to their best remaining choice; women tentatively accept their best offer, reject the rest.

**Proven properties:** Always terminates with a perfect, stable matching. **Man-optimal** (each man gets his best stable partner), **woman-pessimal** (each woman gets her worst).

## Build It

All implementations live in `code/main.py`. Here's the architecture:

### Step 1: Augmenting-Path Baseline (O(V·E))

`bipartite_max_matching(graph, left, right)` — simple DFS that tries to extend the matching one left vertex at a time. Clear but slow; useful for correctness verification.

### Step 2: Hopcroft-Karp (O(E√V))

`hopcroft_karp(graph, left, right)` — BFS builds distance layers, DFS finds disjoint augmenting paths along layers. The `dist` dictionary tracks BFS levels; setting `dist[u] = ∞` after a failed DFS prevents revisiting dead ends. This is the go-to algorithm for unweighted bipartite matching.

### Step 3: Hungarian Algorithm (O(V³))

`hungarian(cost_matrix)` — the Kuhn-Munkres implementation using 1-indexed dual arrays. Each iteration of the outer loop matches one more left vertex by growing an alternating tree and tightening duals. The `min_v[j]` array tracks the minimum reduced cost to reach each right vertex; `way[j]` records the predecessor for path reconstruction. See the code for the full bookkeeping.

### Step 4: Gale-Shapley (O(n²))

`gale_shapley(men_prefs, women_prefs)` — a queue of free men, each proposing down his preference list. Women maintain their current best engagement. O(n²) proposals maximum since each man proposes to each woman at most once.

The C++ implementation in `code/main.cpp` provides a standalone Hopcroft-Karp with a stress test on a random 500×500 bipartite graph at 10% edge density.

## Use It

- **OS scheduling:** Process-to-processor assignment uses bipartite matching to minimize context switches.
- **Kidney exchange programs:** Paired donation modeled as matching over donor-recipient pair cycles. Hospitals run Hopcroft-Karp variants nightly.
- **Compiler register allocation:** Interference graphs colored via matching.
- **Online advertising:** Ad-slot assignment (Google AdWords) uses weighted bipartite matching at scale.
- **NRMP:** The National Resident Matching Program uses Gale-Shapley variants to match medical residents to hospitals.
- **Network design:** König's theorem directly gives the minimum vertex cover of a bipartite graph, which has applications in sensor placement and network monitoring.

Production libraries: `scipy.optimize.linear_sum_assignment` (Hungarian), `networkx.algorithms.matching` (blossom algorithm for general graphs). For competitive programming, Hopcroft-Karp in C++ handles graphs with 10⁵+ edges comfortably.

## Read the Source

- `scipy.optimize.linear_sum_assignment` — C++/Fortran Hungarian via Jonker-Volgenant.
- Gale & Shapley, "College Admissions and the Stability of Marriage" (1962).

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained reference snippet you can reuse in later phases** — Hopcroft-Karp, Hungarian, and Gale-Shapley implementations ready to drop into future projects.

## Exercises

1. **Easy** — Reproduce the Hopcroft-Karp implementation from memory and verify on the lesson's example graph.
2. **Medium** — Implement weighted bipartite matching via min-cost max-flow (add source/sink, edge costs, run successive shortest paths). Compare with the Hungarian algorithm on the same instances.
3. **Hard** — Prove Gale-Shapley produces the proposer-optimal stable matching: show no man can get a better partner in *any* stable matching by analyzing proposal orderings and blocking-pair definitions. (Hint: assume a man m prefers w to his assigned partner, and w prefers m to hers — derive a contradiction.)

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Augmenting path | "alternating path that improves the matching" | Path alternating unmatched/matched edges, endpoints unmatched. Flipping it increases \|M\| by 1 |
| König's theorem | "max matching = min vertex cover (bipartite)" | ν(G) = τ(G) for bipartite G; proved via max-flow min-cut |
| Hopcroft-Karp | "fast bipartite matching" | O(E√V): BFS layers + DFS for maximal shortest augmenting paths per phase |
| Hungarian algorithm | "assignment problem solver" | O(V³) min-weight perfect matching via dual variables and complementary slackness |
| Stable matching | "no one wants to cheat" | No blocking pair (m,w) where both prefer each other to current partners |
| Deferred acceptance | "propose-and-reject" | Gale-Shapley mechanism: proposers offer, receivers tentatively accept best |

## Further Reading

- Hopcroft & Karp, "An n^5/2 Algorithm for Maximum Matchings in Bipartite Graphs" (1973)
- Kuhn, "The Hungarian Method for the Assignment Problem" (1955)
- Gale & Shapley, "College Admissions and the Stability of Marriage" (1962)
- West, *Introduction to Graph Theory*, Chapter 3 (Matchings)
