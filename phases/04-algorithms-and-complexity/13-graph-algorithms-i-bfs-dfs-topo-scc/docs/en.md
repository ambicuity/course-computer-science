# Graph Algorithms I — BFS, DFS, Topo, SCC

> Four algorithms that unlock every graph problem you will ever meet.

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 04 lessons 01–12
**Time:** ~75 minutes

## Learning Objectives

- Represent graphs as adjacency lists and know when adjacency matrices win.
- Implement BFS and DFS from scratch, reason about their O(V+E) complexity.
- Apply topological sort (Kahn's + DFS reverse-postorder) to DAGs.
- Find strongly connected components with Tarjan's and Kosaraju's algorithms.
- Recognise real-world uses: commit ordering (Git DAG), friend suggestions (BFS).

## The Problem

Graphs model networks, dependencies, state machines, and relationships. Without traversal and decomposition algorithms, you cannot answer basic questions: Is there a path? What depends on what? Which nodes form a tightly coupled group? Lesson 10 touched on DP over DAGs — but first you need to *build* the DAG and *order* it. This lesson provides the traversal primitives that every later graph lesson (Dijkstra, MST, flow) assumes.

## The Concept

### Graph Representation

An **adjacency list** stores, for each vertex, a list of its neighbors. An **adjacency matrix** stores an N×N table of edge weights.

| Operation | Adjacency List | Adjacency Matrix |
|-----------|---------------|-----------------|
| Space | O(V+E) | O(V²) |
| Edge check | O(degree) | O(1) |
| Iterate neighbors | O(degree) | O(V) |
| Best for | Sparse graphs | Dense / small graphs |

All algorithms below use adjacency lists — the default for real-world sparse graphs.

### BFS — Breadth-First Search

Visit vertices level by level from a source. A queue holds the frontier. Each vertex is enqueued exactly once.

```
level 0:  [A]            ← start
level 1:  [B, C]         ← A's neighbors
level 2:  [D, E, F]      ← B's and C's neighbors
```

**Properties:**
- Discovers vertices in non-decreasing distance from the source.
- On an unweighted graph, `dist[v]` is the shortest path length.
- Parent pointers reconstruct the shortest path.
- **Complexity:** O(V+E) — each vertex dequeued once, each edge scanned once.

**Applications:** connected components, bipartite check (2-coloring), shortest path in unweighted graphs, level-order processing (social network friend suggestions).

### DFS — Depth-First Search

Go as deep as possible, then backtrack. Track discovery time (preorder) and finish time (postorder).

```
discover A (t=1)
  discover B (t=2)
    discover D (t=3)
    finish   D (t=4)
  finish   B (t=5)
  discover C (t=6)
  finish   C (t=7)
finish   A (t=8)
```

**Edge classification in a DFS over a directed graph:**
- **Tree edge:** part of the DFS tree.
- **Back edge:** points to an ancestor — **cycle exists iff a back edge exists**.
- **Forward edge:** points to a proper descendant (non-tree).
- **Cross edge:** everything else.

**Complexity:** O(V+E).

**Applications:** cycle detection, path finding, topological sort (finish-time order), strongly connected components.

### Topological Sort

A linear ordering of vertices in a DAG such that for every edge u→v, u appears before v.

**Kahn's algorithm (BFS-based):**
1. Compute in-degree for every vertex.
2. Enqueue all vertices with in-degree 0.
3. Dequeue a vertex, append to order, decrement neighbors' in-degrees; enqueue any that hit 0.
4. If the order contains fewer than V vertices, the graph has a cycle.

**DFS-based:**
1. Run DFS, record finish times.
2. Reverse the finish-time order — that is a valid topological order.

Both run in O(V+E). Git uses topological sort on its commit DAG to determine display order.

### Strongly Connected Components

An SCC is a maximal set of vertices where every vertex can reach every other. SCC decomposition condenses a directed graph into a DAG of components.

**Tarjan's algorithm** — single DFS pass.
- Maintain a stack of the current DFS path and a `low_link[v]` value: the smallest discovery time reachable from v (including through descendants).
- When DFS finishes a vertex whose `low_link` equals its own discovery time, it is the root of an SCC — pop the stack down to it.

**Kosaraju's algorithm** — two DFS passes. (1) Run DFS on the original graph, record finish order. (2) Compute the **transpose** (reverse all edges), run DFS on it in decreasing finish order — each DFS tree is one SCC.

Both are O(V+E). Tarjan's uses one pass and no explicit transpose, so it is preferred in practice.

### Bipartite Check via BFS 2-Coloring

A graph is bipartite iff BFS (or DFS) can assign two colors without conflict. Start from any vertex, color it 0, color neighbors 1, their neighbors 0, etc. If an edge connects two same-colored vertices, the graph is not bipartite (it contains an odd cycle).

## Build It

All implementations use an adjacency-list `Graph` class. See `code/main.py` and `code/main.rs` for the full code.

### Step 1: Graph + BFS + DFS

```python
class Graph:
    def __init__(self, n: int, directed: bool = False):
        self.n = n
        self.directed = directed
        self.adj: list[list[int]] = [[] for _ in range(n)]

    def add_edge(self, u: int, v: int) -> None:
        self.adj[u].append(v)
        if not self.directed:
            self.adj[v].append(u)

def bfs(graph: Graph, start: int):
    from collections import deque
    dist = [-1] * graph.n
    parent = [-1] * graph.n
    dist[start] = 0
    q = deque([start])
    while q:
        u = q.popleft()
        for v in graph.adj[u]:
            if dist[v] == -1:
                dist[v] = dist[u] + 1
                parent[v] = u
                q.append(v)
    return parent, dist
```

DFS tracks discovery/finish times via a clock counter; iterative stack avoids Python recursion limits.

### Step 2: Topological Sort + SCC

Kahn's uses an in-degree array and a deque. Tarjan's maintains a stack, an `on_stack` flag array, `disc` and `low` arrays, and a global time counter. See the code files for the complete implementations.

## Use It

- **Git** — commit history is a DAG; `git log --topo-order` uses topological sort.
- **NetworkX (Python)** — `nx.bfs_tree`, `nx.dag_topological_sort`, `nx.strongly_connected_components`.
- **Petgraph (Rust)** — `petgraph::algo::kosaraju_scc`, `petgraph::visit::depth_first_search`.
- **Social networks** — BFS from a user discovers friends-of-friends in degree order (friend suggestion).
- **Compilers** — topological sort on the module dependency graph determines compilation order.

Production enhancements: adjacency-list memory layout, iterative DFS to avoid stack overflow.

## Read the Source

- **NetworkX** `networkx/algorithms/traversal/bfs.py` — production BFS with edge filtering, `source` and `depth_limit` parameters.
- **Petgraph** `src/algo/mod.rs` — `kosaraju_scc` implementation using Tarjan-style stack, ~50 lines.

## Ship It

`outputs/` contains **a graph traversal library with BFS, DFS, topological sort, Tarjan SCC, Kosaraju SCC, and bipartite check** — import into any later graph lesson.

## Exercises

1. **Easy** — Detect and print all cycles in a directed graph. Modify DFS to collect back edges and trace parent pointers to extract each cycle.
2. **Medium** — Find bridges and articulation points using DFS low-link values. Report bridge edges and cut vertices.
3. **Hard** — Check if a graph is bipartite using BFS 2-coloring. If it is, return the two partitions. If not, return an odd-cycle witness.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| BFS | "Level-order search" | Queue-based traversal visiting vertices in increasing distance order |
| DFS | "Go deep first" | Stack-based traversal that discovers/finishes vertices, classifies edges |
| Topological sort | "Dependency order" | Linear vertex ordering respecting all directed edges (DAGs only) |
| SCC | "Tightly connected group" | Maximal vertex set where every pair is mutually reachable |
| Low-link | "Earliest reachable" | Lowest discovery time reachable from a vertex via tree + back edges |

## Further Reading

- Cormen et al., *Introduction to Algorithms*, Ch. 22 (Elementary Graph Algorithms).
- Tarjan, R. E. (1972). "Depth-first search and linear graph algorithms." *SIAM J. Comput.*
