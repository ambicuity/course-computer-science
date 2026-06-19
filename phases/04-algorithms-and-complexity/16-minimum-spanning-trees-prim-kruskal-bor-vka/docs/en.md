# Minimum Spanning Trees — Prim, Kruskal, Borůvka

> Connect everything. Pay nothing you don't have to.

**Type:** Build
**Languages:** Python, Rust
**Prerequisites:** Phase 03 (graphs, heaps, Union-Find), Phase 01 (greedy intuition)
**Time:** ~60 minutes

## Learning Objectives

- Implement Kruskal's, Prim's, and Borůvka's MST algorithms
- Understand and prove the cut property and cycle property
- Implement Union-Find with path compression and union by rank
- Decide which MST algorithm to use for dense vs sparse graphs

## The Problem

You are building a fiber-optic network across 50 cities. Every pair of cities has a known
installation cost, but you do not need direct links between every pair — you just need a
path. Your goal: connect all cities with **minimum total cable cost** and **no redundant
links** that would form a cycle. This is the **Minimum Spanning Tree** problem.

Formally: given a connected, undirected, weighted graph G = (V, E), find a spanning tree
T — a connected, acyclic subgraph on all |V| vertices — that minimizes the sum of its
edge weights. A spanning tree always has exactly |V| − 1 edges.

If all edge weights are distinct, the MST is unique. If weights can repeat, multiple MSTs
may exist, but they all share the same total weight.

## The Concept

```
Graph:              MST (weight = 11):
  0                    0
 /| \                 / \
1 2  4               1   4
|\| /|              |
3 5  6               3
                    / \
                   5   6
```

### Cut Property and Cycle Property

**Cut Property.** For any partition (S, V\S), the lightest edge crossing the cut belongs
to every MST. Proof: if absent from some MST T, adding it creates a cycle. That cycle
must contain another crossing edge — remove it to get a lighter spanning tree. Contradiction.

**Cycle Property.** The heaviest edge on any cycle belongs to no MST. If included in some
MST T, removing it disconnects T. Reconnecting via a lighter cycle edge is strictly cheaper.
Contradiction.

### Three Greedy Strategies — One Guarantee

| Algorithm | Strategy | Key data structure | Complexity | Best for |
|-----------|----------|-------------------|------------|----------|
| Kruskal | Lightest edge that merges two components | Union-Find | O(E log E) | Edge lists, sparse graphs |
| Prim | Lightest edge crossing the current tree frontier | Min-heap | O(E log V) | Dense graphs, adjacency lists |
| Borůvka | Lightest outgoing edge per component (parallel!) | Union-Find | O(E log V) | Parallel / distributed |

All three are greedy algorithms justified by the cut property. Each picks the lightest
safe edge at every step — they just differ in *which cut* they inspect.

### Union-Find — The Backbone

Kruskal's and Borůvka's need fast "are these two vertices in the same component?" queries.
Union-Find answers in amortized O(α(n)) per operation using two tricks:

- **Path compression:** on `find(x)`, redirect all traversed nodes to the root.
- **Union by rank:** always attach the shorter tree under the taller one.

The inverse Ackermann function α(n) is ≤ 4 for any conceivable input size — effectively
constant.

## Build It

### Step 1: Union-Find

```python
class UnionFind:
    def __init__(self, n):
        self.parent = list(range(n))
        self.rank = [0] * n

    def find(self, x):
        while self.parent[x] != x:
            self.parent[x] = self.parent[self.parent[x]]  # path splitting
            x = self.parent[x]
        return x

    def union(self, x, y):
        rx, ry = self.find(x), self.find(y)
        if rx == ry:
            return False
        if self.rank[rx] < self.rank[ry]:
            rx, ry = ry, rx
        self.parent[ry] = rx
        if self.rank[rx] == self.rank[ry]:
            self.rank[rx] += 1
        return True
```

### Step 2: Kruskal's Algorithm

Sort all edges by weight. Scan the sorted list; add each edge if its endpoints are in
different components. Stop after V − 1 edges.

```python
def kruskal(edges, v):
    edges.sort(key=lambda e: e[2])
    uf = UnionFind(v)
    mst, total = [], 0
    for u, vtx, w in edges:
        if uf.union(u, vtx):
            mst.append((u, vtx, w))
            total += w
            if len(mst) == v - 1:
                break
    return mst, total
```

### Step 3: Prim's Algorithm

Grow the MST from vertex 0. A min-heap tracks the lightest edge from the current tree
to each non-tree vertex. Extract the minimum, add its destination vertex, push new
frontier edges.

```python
import heapq

def prim(graph, v):
    visited = [False] * v
    heap, mst, total = [], [], 0
    visited[0] = True
    for nb, w in graph[0]:
        heapq.heappush(heap, (w, 0, nb))
    while heap and len(mst) < v - 1:
        w, u, vtx = heapq.heappop(heap)
        if visited[vtx]:
            continue
        visited[vtx] = True
        mst.append((u, vtx, w))
        total += w
        for nb, nw in graph[vtx]:
            if not visited[nb]:
                heapq.heappush(heap, (nw, vtx, nb))
    return mst, total
```

### Step 4: Borůvka's Algorithm

Each component independently finds its lightest outgoing edge, then all merges happen.
Components at least halve each phase → O(log V) phases.

```python
def boruvka(edges, v):
    uf = UnionFind(v)
    mst, total, nc = [], 0, v
    while nc > 1:
        cheap = [-1] * v
        for i, (u, vtx, w) in enumerate(edges):
            ru, rv = uf.find(u), uf.find(vtx)
            if ru == rv:
                continue
            if cheap[ru] == -1 or edges[cheap[ru]][2] > w:
                cheap[ru] = i
            if cheap[rv] == -1 or edges[cheap[rv]][2] > w:
                cheap[rv] = i
        merged = False
        for i in range(v):
            ci = cheap[i]
            if ci == -1:
                continue
            u, vtx, w = edges[ci]
            if uf.union(u, vtx):
                mst.append((u, vtx, w))
                total += w
                nc -= 1
                merged = True
        if not merged:
            break
    return mst, total
```

### Step 5: Verification

Run all three on the same graph and assert identical total weights. This is the
definitive correctness check — if the total matches, the implementations are sound.

## Use It

- **Kruskal's** — cable/fiber network design. Edge lists from site surveys feed directly
  into sorted-edge processing. `networkx.minimum_spanning_tree` uses Kruskal internally.
- **Prim's** — maze generation (carve passages via Prim's on a grid cell graph). The
  O(V²) adjacency-matrix variant is optimal for dense graphs where |E| ≈ |V|².
- **Borůvka's** — parallel and distributed MST. Each machine finds its component's
  lightest outgoing edge independently; only the merge step needs synchronization.

Rust's `petgraph::algo::min_spanning_tree` implements Kruskal with Union-Find.
Production code adds generics, directed-graph guards, and iterator-based streaming
over massive edge lists.

## Ship It

An MST library (`main.py` / `main.rs`) providing Kruskal, Prim, and Borůvka with a
common return signature `(edge_list, total_weight)`.

## Exercises

1. **Prove the cut property.** Show that if e is the lightest edge crossing cut (S, V\S),
   then e belongs to every MST. (Hint: assume e ∉ MST T, add e → cycle, apply cycle property.)

2. **Find the second-best MST.** For each edge e in the MST, remove it, recompute the MST
   on the remaining edges, take the minimum replacement. Verify on a 6-vertex graph.

3. **Implement Borůvka's** and prove it parallelizable: each component finds its lightest
   outgoing edge independently per phase; phases halve component count → O(log V).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Spanning tree | "A tree that covers all vertices" | Subgraph: V vertices, V−1 edges, connected, acyclic |
| MST | "Cheapest way to connect everything" | Spanning tree minimizing edge-weight sum |
| Cut | "A partition of vertices" | Any (S, V\S) split; cut edge has one endpoint in each side |
| Cut property | "Lightest crossing edge is safe" | Lightest edge crossing any cut is in every MST |
| Cycle property | "Heaviest cycle edge is doomed" | Heaviest edge in any cycle is in no MST |
| Union-Find | "Disjoint set thing" | Partition-tracking structure with near-O(1) find/union |
| Path compression | "Flatten on find" | Redirect traversed nodes to root during find |
| Union by rank | "Attach short under tall" | Link smaller-rank tree under larger to keep depth low |

## Further Reading

- T. Cormen et al., *Introduction to Algorithms*, 4th ed., Ch. 21.
- D. West, *Introduction to Graph Theory*, 2nd ed., §2.1.
- B. Chazelle, "A Minimum Spanning Tree Algorithm with Inverse-Ackermann Type Complexity," JACM 2000.
