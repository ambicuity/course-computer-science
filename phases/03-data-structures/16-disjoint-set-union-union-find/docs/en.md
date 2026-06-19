# Disjoint Set Union (Union-Find)

> Twenty lines of code, the inverse Ackermann function in the analysis, and the foundation of Kruskal's MST, dynamic connectivity, and equivalence-class tracking.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L01 (dynamic array)
**Time:** ~60 minutes

## Learning Objectives

- Implement Union-Find with **union-by-rank** + **path compression**: amortized α(n) per op.
- Apply it to two classic problems: Kruskal's MST and connected components in a graph.
- State the **inverse Ackermann α(n)** function — for any practical n, α(n) ≤ 4. So in practice DSU is "constant".
- Recognize where you've used DSU: Kruskal, image segmentation (Felzenszwalb), Linux's epoll wakeup deduplication.

## The Problem

You have n elements partitioned into disjoint sets. Two operations:

- `find(x)`: return a canonical "representative" of the set containing x.
- `union(x, y)`: merge the sets containing x and y.

Equivalent: maintain a relation that grows over time; answer "are x and y equivalent?" by `find(x) == find(y)`.

Naïve implementation: store a `parent[i]` array, where parent[i] is the parent of i (or i if a root). Walk parents to find root. Worst case: O(n) per find — a linked-list chain.

The trick: keep trees shallow with two heuristics that, together, give α(n) amortized — essentially constant.

## The Concept

### Setup

```c
int parent[n];                  /* parent[i] = parent in the union tree; i if root */
int rank[n];                    /* upper bound on tree height; for union-by-rank */
for (int i = 0; i < n; ++i) { parent[i] = i; rank[i] = 0; }
```

Each element starts in its own set; n trees of size 1.

### find with path compression

```c
int find(int x) {
    if (parent[x] == x) return x;
    return parent[x] = find(parent[x]);  /* point directly at root */
}
```

After a find, all nodes on the path point directly at the root → next find is O(1).

### union by rank

```c
void unite(int x, int y) {
    int rx = find(x), ry = find(y);
    if (rx == ry) return;
    if (rank[rx] < rank[ry]) parent[rx] = ry;
    else if (rank[rx] > rank[ry]) parent[ry] = rx;
    else { parent[ry] = rx; rank[rx]++; }
}
```

Attach the shorter tree under the taller — keeps the height balanced.

### Why α(n) is "near-constant"

The Tarjan-Van Leeuwen analysis (1984): with both union-by-rank AND path compression, amortized cost is O(α(n)) where α is the inverse Ackermann function. For any n ≤ 2^65536, α(n) ≤ 4. So in any actual program, it's bounded by a tiny constant.

Tarjan proved it's also a tight lower bound: no pointer-machine algorithm can do better. DSU is optimally efficient.

### Without path compression

Union-by-rank alone gives O(log n) per op — still good. Path compression alone (no rank) gives O(log n) amortized too. Both together is what beats the log bound.

### Variants

- **Union by size**: instead of rank (height bound), track set size; attach smaller under larger. Same asymptotic guarantees.
- **Path halving / splitting**: cheaper alternatives to full path compression that still give O(α(n)). Used when iterative cleanness matters.
- **Persistent DSU**: support "undo" via a stack of changes. Useful for backtracking algorithms.

### Killer applications

1. **Kruskal's MST**: sort edges by weight; for each edge, union if endpoints disconnected. DSU's union/find is the algorithm.
2. **Connected components** of a graph after streaming edges: O((n + m) α(n)).
3. **Image segmentation** (Felzenszwalb-Huttenlocher): pixels are nodes; merge similar pixels with DSU.
4. **Cycle detection in unions**: a duplicate union (both already in same set) signals a cycle.
5. **Linux epoll**: deduplicate wakeup notifications across fds via a DSU-like structure.

## Build It

`code/main.c`:

1. DSU with union-by-rank + path compression.
2. Build connected components from random edges.
3. Kruskal's MST on a small weighted graph.
4. Benchmark: 1M unions on a 1M-node graph; measure ns/op.

`code/main.py` mirrors with a class.

`code/main.rs` standard recursive find with path compression.

### Run

```sh
clang -O2 -fsanitize=address main.c -o dsu && ./dsu
python3 main.py
```

## Use It

- **Kruskal's MST** in network design, spatial clustering.
- **Connected components** of streaming graphs.
- **Image segmentation** in computer vision (Felzenszwalb).
- **Percolation theory** simulations (Princeton's classic algs4 example).
- **Equivalence-class tracking** in type-inference and theorem provers.

## Read the Source

- [Boost `disjoint_sets`](https://www.boost.org/doc/libs/1_83_0/boost/pending/disjoint_sets.hpp) — production C++ DSU with multiple variants.
- [Java's `WeightedQuickUnionUF` (Sedgewick algs4)](https://algs4.cs.princeton.edu/15uf/WeightedQuickUnionUF.java.html) — clean reference.
- *Tarjan & Van Leeuwen, 1984*: original α(n) analysis paper.

## Ship It

This lesson ships **`outputs/dsu.h`** — single-header DSU with both make-set/union/find.

## Exercises

1. **Easy.** Implement `connected(x, y)` returning whether x and y share a root.
2. **Medium.** Implement Kruskal's MST using DSU. Test on a random weighted graph of 1K vertices.
3. **Hard.** Implement **rollback DSU**: a stack records every parent/rank change; `rollback()` undoes the most recent union. Useful for offline problems (queries answered after batches of edges).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| DSU / Union-Find | "Disjoint-set tracker" | Data structure that tracks equivalence classes under merge |
| Find | "Lookup representative" | Returns the canonical root of the set containing x |
| Union | "Merge classes" | Combines the sets of x and y; one root becomes the parent of the other |
| Path compression | "Flatten on find" | Update every node on the find path to point directly at root |
| Union by rank/size | "Shorter under taller" | Heuristic for choosing which root becomes the parent |
| α(n) | "Inverse Ackermann" | Grows so slowly it's bounded by 4 for any practical n |

## Further Reading

- *Tarjan: Algorithm Engineering — Disjoint Set Forests* (1975-1984 papers).
- [Sedgewick algs4 §1.5](https://algs4.cs.princeton.edu/15uf/) — clean exposition.
- *Competitive Programmer's Handbook* — DSU recipes for contest problems.
