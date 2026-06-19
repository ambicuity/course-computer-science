# Network Flow — Ford-Fulkerson, Edmonds-Karp, Dinic

> How much can you actually push through a pipeline? Network flow gives the answer — and the answer powers matching, scheduling, and every optimization you'd otherwise solve by guessing.

**Type:** Build
**Languages:** Python, C++
**Prerequisites:** Phase 04 lessons 01–16
**Time:** ~90 minutes

## Learning Objectives

- Model real problems as flow networks with capacities, flows, and residual graphs.
- Prove and apply the max-flow min-cut theorem to recover minimum cuts.
- Implement Ford-Fulkerson, Edmonds-Karp, and Dinic's algorithm from scratch.
- Reduce bipartite matching, project selection, and scheduling to max-flow.

## The Problem

You run a logistics company. Your network of roads between cities has capacity limits — no road can carry more than *c* trucks per hour. Given a source city and a destination city, what is the maximum throughput? Brute force (try all paths, greedily fill) is exponential and wrong.

More generally: given a directed graph with edge capacities, a source *s*, and a sink *t*, find the maximum flow from *s* to *t*. This single primitive solves dozens of seemingly unrelated problems: maximum bipartite matching (job assignment), minimum cut (network reliability), airline crew scheduling, image segmentation, and more.

## The Concept

### Flow networks

A **flow network** is a directed graph G = (V, E) with:

- A **capacity** function c(u, v) ≥ 0 on every edge (c(u, v) = 0 if (u, v) ∉ E).
- A distinguished **source** node *s* and a **sink** node *t*.

A **flow** is a function f(u, v) satisfying:

1. **Capacity constraint**: 0 ≤ f(u, v) ≤ c(u, v) — never exceed capacity.
2. **Skew symmetry**: f(u, v) = −f(v, u) — flow in one direction is negative flow in the other.
3. **Flow conservation**: for every node v ≠ s, t: ∑ᵤ f(u, v) = 0 — what comes in goes out.

The **value** of the flow is |f| = ∑ᵥ f(s, v) — everything leaving the source.

### Residual graph and augmenting paths

After committing flow f, the **residual capacity** is r(u, v) = c(u, v) − f(u, v). The **residual graph** G_f has the same nodes but edges for every pair with r(u, v) > 0. An **augmenting path** is any s→t path in G_f.

The key insight: pushing flow along an augmenting path increases total flow. When no augmenting path exists, we're done. But *how* we find augmenting paths determines the algorithm's speed.

### Worked example

```
       10
  s ──────► a ──────► t
  │  4       8       │
  │         ▲        │
  └────► b ─┘        │
     9      6        │
     └─────►─────────┘
              10
```

Capacities: s→a = 10, s→b = 9, a→t = 8, b→t = 10, b→a = 6.

Iteration 1: Path s→a→t, bottleneck = min(10, 8) = 8. Flow = 8.
Residual: s→a has 2 left; a→t is full; b→a has 6 forward + 8 reverse.

Iteration 2: Path s→b→a→t, bottleneck = min(9, 6, 2) = 2. Flow = 10.
Residual: s→a is full, s→b has 7 left.

Iteration 3: Path s→b→t, bottleneck = min(7, 10) = 7. Flow = 17.
No more augmenting paths. Max flow = 17.

### Max-flow min-cut theorem

A **cut** (S, T) partitions V into two sets with s ∈ S, t ∈ T. The **capacity** of the cut is the sum of capacities of all edges from S to T.

**Theorem (Ford-Fulkerson, 1956):** The maximum flow equals the minimum cut capacity.

This is *strong duality*: the primal (maximize flow) and dual (minimize cut) have the same optimal value. Proof sketch:
- For any flow f and any cut (S, T): |f| ≤ capacity(S, T) (flow can't exceed any bottleneck).
- When no augmenting path exists, let S = {nodes reachable from s in G_f}. Then |f| = capacity(S, T), so f is max and (S, T) is min.

### Ford-Fulkerson method

The **Ford-Fulkerson method** is a framework, not a specific algorithm:

```
f ← 0
while exists augmenting path p in G_f:
    push as much flow as possible along p (bottleneck of p)
return f
```

The choice of *how* to find the augmenting path is left open. Different choices give different algorithms.

**Complexity:** O(E · |f*|) where f* is the maximum flow value. Each iteration finds at least 1 unit of flow. Problem: if capacities are irrational, the algorithm may not terminate.

### Edmonds-Karp (BFS augmenting paths)

**Edmonds-Karp** picks the *shortest* augmenting path (fewest edges) using BFS.

**Why BFS helps:** Each edge can become critical (bottleneck of the shortest path) at most O(V) times. Each BFS takes O(E), so total: **O(V E²)**.

**Proof sketch:** When edge (u, v) is a bottleneck, the shortest s→t path through it is strictly longer next time it's used. Since shortest path length is at most V − 1, each edge is critical O(V) times.

Edmonds-Karp guarantees polynomial termination — even with irrational capacities.

### Dinic's algorithm

Dinic's is faster in both theory and practice. The idea: instead of finding one augmenting path at a time, find *all* shortest augmenting paths simultaneously.

**Phase 1 — Build the level graph:** BFS from s, assigning each node a "level" (distance from s). Only edges (u, v) where level[v] = level[u] + 1 are kept.

**Phase 2 — Blocking flow:** DFS greedily saturate all paths in the level graph simultaneously. A **blocking flow** is a flow that saturates at least one edge on every remaining s→t path in the level graph.

Repeat until BFS can't reach t.

**Complexity:** O(V² E). In practice, much faster — often near-linear for structured graphs.

### Recovering the minimum cut

After running any max-flow algorithm, the minimum cut is found by:

1. Build the residual graph G_f.
2. Let S = {all nodes reachable from s in G_f}.
3. The min-cut edges are all original edges (u, v) with u ∈ S and v ∉ S.

## Build It

### Step 1: Ford-Fulkerson with DFS

The simplest augmenting-path strategy: DFS to find *any* path from s to t in the residual graph.

### Step 2: Edmonds-Karp (BFS version)

Swap DFS for BFS. Same framework, polynomial guarantee.

### Step 3: Dinic's algorithm (level graph + blocking flow)

The full algorithm: BFS for levels, DFS for blocking flow, repeat.

## Use It

### Bipartite matching via max-flow

Given a bipartite graph (left set L, right set R, edges between them), find a maximum matching:

1. Create source *s*, sink *t*.
2. Add edges s→u for every u ∈ L (capacity 1).
3. Add edges v→t for every v ∈ R (capacity 1).
4. Keep original edges (u, v) with capacity 1.
5. Run max-flow. Each saturated edge in the original bipartite graph is a matched pair.

This is the reduction behind the Hungarian algorithm, Hopcroft-Karp, and more.

### Project selection (closure problem)

Projects with prerequisites and profits: build a flow network where min-cut decides which projects to accept. This is how constraint optimization engines work internally.

### Baseball elimination

Determine if a team is mathematically eliminated from winning. Each remaining game is an edge; max-flow decides if outcomes exist that let your team win.

## Read the Source

- [Boost Graph Library — `push_relabel_max_flow`](https://www.boost.org/doc/libs/1_83_0/libs/graph/doc/push_relabel_max_flow.html) — Push-relabel, the production-grade algorithm that replaced Edmonds-Karp in practice.
- [LEDA / CGAL network flow](https://www.algorithmic-solutions.info/leda_guide/graph_algorithms/flow.html) — Dinic's with capacity scaling in a geometry library context.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained max-flow solver (Dinic's) plus min-cut recovery and bipartite matching reduction you can reuse in later phases.**

## Exercises

1. **Easy** — Solve bipartite matching: given a bipartite graph with 4 left nodes and 5 right nodes, build the flow network and compute maximum matching using the code from this lesson.

2. **Medium** — Find minimum cut edges: after computing max-flow, implement the min-cut recovery and verify that the cut capacity equals the max-flow value on the worked example.

3. **Hard** — Implement the **push-relabel algorithm** (Goldberg-Tarjan). It runs in O(V³) worst-case but is faster in practice than Dinic's for dense graphs. Compare benchmark times on random graphs.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Flow network | "The network has capacities" | Directed graph with capacity function, source s, sink t |
| Residual graph | "What's left to push" | Same nodes; edges exist where residual capacity r(u,v) = c(u,v) − f(u,v) > 0 |
| Augmenting path | "An s-t path we can push more flow along" | Any path from s to t in the residual graph |
| Blocking flow | "Saturate every shortest path" | A flow that kills at least one edge on every s→t path in the level graph |
| Min-cut | "The cheapest place to cut the network" | A partition (S, T) with minimum total capacity of edges from S to T |
| Max-flow min-cut theorem | "Primal = dual" | Maximum flow value equals minimum cut capacity |
| Level graph | "Dinic's BFS layer" | Subgraph retaining only edges (u, v) where dist(s, v) = dist(s, u) + 1 |

## Further Reading

- Cormen et al., *Introduction to Algorithms* (CLRS), Chapter 26 — Network Flow. The canonical textbook treatment.
- Kleinberg & Tardos, *Algorithm Design*, Chapter 7 — More intuition on reductions (matching, project selection, baseball).
- Goldberg & Tarjan, "A New Approach to the Maximum-Flow Problem" (1988) — The push-relabel paper; worth reading for the amortized analysis technique.
