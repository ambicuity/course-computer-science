# Graphs — Representations and APIs

> Three layouts (adjacency list, matrix, CSR), three trade-offs. Pick by graph density and what queries you'll ask.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L01 (arrays), L05 (hash tables)
**Time:** ~60 minutes

## Learning Objectives

- Implement **adjacency list** (Vec of Vecs) — standard for sparse graphs.
- Implement **adjacency matrix** — dense graphs, O(1) edge-exists.
- Implement **CSR** (Compressed Sparse Row) — sparse + cache-friendly.
- Choose representation by graph density, edge weights, query mix.
- Recognize the production sources: Boost.Graph, NetworkX (Python), petgraph (Rust).

## The Problem

A graph G = (V, E) has |V| = n vertices and |E| = m edges. Operations we want:

- `add_edge(u, v)`: insert.
- `has_edge(u, v)`: lookup.
- `neighbors(u)`: iterate over u's neighbors.
- For weighted graphs: store + retrieve weights.

The three classic representations differ in time and space per operation. Pick the one that matches your workload.

## The Concept

### Adjacency list

For each vertex, a list (or Vec) of its neighbors:

```c
typedef struct { int dst; int weight; } Edge;
typedef struct { Edge **lists; int *lens; int n; } Graph;
```

Space: O(n + m). Iterate neighbors: O(deg(u)). has_edge: O(deg(u)).

Most graphs in CS are sparse (m << n²), and adjacency list is the standard for them.

### Adjacency matrix

A 2D bit matrix (or weight matrix):

```c
int adj[N][N];
```

Space: O(n²). has_edge: O(1). neighbors: O(n).

Wins for dense graphs (m close to n²) and for algorithms that do "for each pair" comparisons (Floyd-Warshall, transitive closure).

### CSR (Compressed Sparse Row)

Two arrays:

```c
int row_starts[n + 1];     /* row_starts[i] = start of vertex i's neighbors in next[] */
int next[m];               /* flattened list of all neighbors */
```

To iterate u's neighbors: `for (int i = row_starts[u]; i < row_starts[u+1]; ++i) visit(next[i])`.

Space: O(n + m). Cache-friendly: contiguous memory. Mutation expensive (insert requires shifting m elements). Used in numerical linear algebra (sparse matrices), GPU graph algorithms, large-scale static graphs.

### Comparison

|  | Adjacency list | Matrix | CSR |
|---|----------------|--------|-----|
| Space | O(n + m) | O(n²) | O(n + m) |
| has_edge | O(deg) | O(1) | O(log deg) [binary search] |
| neighbors | O(deg) | O(n) | O(deg), cache-friendly |
| add_edge | O(1) | O(1) | O(m) — must rebuild |
| Best for | sparse, mutable | dense, "for-each-pair" | sparse, static, perf |

### Real-world choices

- **Social network**: 1B vertices, average degree ~200. Sparse → adjacency list.
- **Road network**: 10M vertices, average degree ~4. Sparse + static → CSR.
- **Image segmentation grid graph**: every pixel has 4-8 neighbors → adjacency list or implicit (no edges stored).
- **All-pairs distance on a small workflow**: 100 nodes → matrix.

### APIs

Production graph libraries layer high-level traits over these representations:

- **Boost.Graph**: vertex/edge property maps; algorithms templated over graph shape.
- **petgraph (Rust)**: `Graph<N, E>`, `StableGraph`, `Csr`.
- **NetworkX (Python)**: dict-of-dicts (slow but extremely flexible).
- **igraph (R/Python/C)**: CSR-backed, fast for huge graphs.

## Build It

`code/main.c`:

1. All three: adjacency list, matrix, CSR.
2. BFS over each (just to show the iterator differs).
3. Memory + iteration time comparison on a 1000-vertex random graph.

`code/main.py` builds with dicts (NetworkX-style).

`code/main.rs` uses Vec-of-Vec adjacency list.

### Run

```sh
clang -O2 -fsanitize=address main.c -o graph && ./graph
```

## Use It

- **Database query optimizers**: graph of relations + join predicates → choose plan.
- **Compilers**: dependency graphs, call graphs, control-flow graphs.
- **Routing**: road networks, IP networks, transit.
- **Recommendation systems**: bipartite graphs of users ↔ items.
- **Static analyzers**: control-flow + data-flow graphs.

## Read the Source

- [Boost.Graph documentation](https://www.boost.org/doc/libs/1_83_0/libs/graph/doc/index.html).
- [petgraph source](https://github.com/petgraph/petgraph/blob/master/src/graph_impl/mod.rs).
- *Algorithm Design* by Kleinberg & Tardos — graph-algorithms chapters all use adjacency list.

## Ship It

This lesson ships **`outputs/graph.h`** — single-header adjacency-list + CSR graph.

## Exercises

1. **Easy.** Compute the average degree from your adjacency list. Verify ≈ 2m/n for undirected graphs.
2. **Medium.** Convert adjacency list to CSR in O(n + m). Used as a finalization step before running graph algorithms.
3. **Hard.** Implement BFS three ways (adjacency list, matrix, CSR) on a graph with 1M vertices, 10M edges. Measure ns/edge for each.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Sparse | "Few edges" | m << n² (most real graphs) |
| Dense | "Many edges" | m ≈ n² (rare; complete graphs, all-pairs problems) |
| Adjacency list | "Vec of Vec" | Each vertex has its own list of neighbors |
| Adjacency matrix | "n × n bits/weights" | A[i][j] = 1/weight if (i,j) is an edge |
| CSR | "Compressed sparse row" | Flat array of neighbors with per-row offsets |

## Further Reading

- *Boost.Graph Library: User Guide* (Siek, Lee, Lumsdaine).
- *Algorithms* by Sedgewick — Chapter 4 covers all graph algorithms on adjacency list.
- *Graph Algorithms in the Language of Linear Algebra* (Kepner, Gilbert) — matrix-formulated graph algos.
