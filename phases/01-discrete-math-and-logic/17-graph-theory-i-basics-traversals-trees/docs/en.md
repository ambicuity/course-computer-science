# Graph Theory I — Basics, Traversals, Trees

> A graph is the universal data structure: networks, dependencies, social ties, parse trees, control flow. Two algorithms (BFS and DFS) unlock 60% of practical graph problems.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 01, Lessons 04 (relations), 06 (posets / topo sort)
**Time:** ~75 minutes

## Learning Objectives

- Define a graph (directed / undirected / weighted), state its standard representations (adjacency list, adjacency matrix, edge list) and their trade-offs.
- Implement Breadth-First Search (BFS) and Depth-First Search (DFS) iteratively and recursively; recognize each by the data structure it uses (queue vs stack).
- Use BFS to find shortest paths in an unweighted graph; use DFS to detect cycles, find connected components, and topologically sort a DAG.
- Define a tree (connected acyclic graph) and a spanning tree; understand the basic counting identity `|E| = |V| - 1` for trees.

## The Problem

Three problems that all look different until you see they're the same:

1. "What's the shortest sequence of friend-of-friend hops between two users?"
2. "Which packages must I rebuild after editing this header?"
3. "Are there any unreachable test cases in this state machine?"

Each is a graph problem. The first is BFS. The second is reachability + topological sort. The third is "is the graph strongly connected from a root?" — also a traversal. Master BFS and DFS, and you've solved the foundation for hundreds of CS problems.

## The Concept

### Graphs

A graph G = (V, E) is a set of vertices V and a set of edges E. Variants:

| Type | Edges are | Self-loops? | Use cases |
|------|-----------|-------------|-----------|
| Undirected | unordered pairs `{u, v}` | sometimes | friendship, road maps |
| Directed (digraph) | ordered pairs `(u, v)` | yes | dependencies, web links, CFG |
| Weighted | edges carry a number | — | distances, capacities, costs |
| Multigraph | multiple edges between u, v | yes | network flows with parallel links |
| Simple | no self-loops, no multi-edges | no | the default for algorithm analysis |

A **path** is a sequence of vertices `v₀, v₁, …, vₖ` with each consecutive pair connected by an edge. A **cycle** is a path that returns to v₀. A graph is **connected** (in the undirected case) iff there's a path between every pair of vertices.

### Representations

| Representation | Space | Edge query | Neighbors query | Best for |
|----------------|-------|-----------|-----------------|----------|
| **Adjacency list** | O(V + E) | O(deg(u)) | O(deg(u)) | sparse graphs (most CS uses) |
| **Adjacency matrix** | O(V²) | O(1) | O(V) | dense graphs, frequent edge-tests |
| **Edge list** | O(E) | O(E) | O(E) | edge-iteration algorithms like Kruskal |

In Python:

```python
# Adjacency list
adj = {0: [1, 2], 1: [0, 3], 2: [0], 3: [1]}

# Adjacency matrix
mat = [[0, 1, 1, 0],
       [1, 0, 0, 1],
       [1, 0, 0, 0],
       [0, 1, 0, 0]]
```

Default to adjacency list. 90%+ of real-world graphs are sparse (E = O(V)).

### Breadth-First Search

BFS visits vertices in order of distance from a start vertex. Uses a queue.

```python
from collections import deque

def bfs(adj, start):
    dist = {start: 0}
    parent = {start: None}
    q = deque([start])
    while q:
        u = q.popleft()
        for v in adj[u]:
            if v not in dist:
                dist[v] = dist[u] + 1
                parent[v] = u
                q.append(v)
    return dist, parent
```

Runtime: O(V + E). Output: shortest-path distances from `start` in an unweighted graph; the parent pointers reconstruct paths.

### Depth-First Search

DFS goes as deep as possible before backtracking. Uses a stack (implicit or explicit).

```python
def dfs(adj, start):
    visited = set()
    order = []
    def rec(u):
        visited.add(u); order.append(u)
        for v in adj[u]:
            if v not in visited: rec(v)
    rec(start)
    return order
```

Variants:
- **Cycle detection** in a directed graph: track three colors (white = unvisited, gray = on stack, black = done); finding a gray vertex among neighbors of u means cycle.
- **Topological sort** (Lesson 06 redux): DFS, record vertices in reverse-finish order.
- **Connected components**: run DFS from every unvisited vertex; each call defines one component.
- **Strongly connected components** (Tarjan / Kosaraju, Phase 04 L13): two passes of DFS on G and Gᵀ.

### Trees

A **tree** is a connected acyclic graph. Equivalent definitions for a graph with n vertices:

- Connected and has n - 1 edges.
- Acyclic and has n - 1 edges.
- Connected and removing any edge disconnects it.
- Acyclic and adding any edge creates a cycle.
- Unique path between every pair of vertices.

A **rooted tree** designates one vertex as root; this orients every edge "downward." Each non-root vertex has exactly one parent.

A **spanning tree** of a connected graph G is a subgraph that is a tree and contains every vertex. BFS and DFS both produce one (the parent pointers).

### Counting structures

| Object | Count |
|--------|-------|
| Edges in a tree on n vertices | n - 1 |
| Edges in a complete graph K_n | n(n-1)/2 |
| Spanning trees of K_n (Cayley's formula) | n^(n-2) |
| Vertices reachable from u via BFS | the connected component of u |

## Build It

Open `code/main.py`.

### Step 1: BFS on a small graph

Verify shortest-path distances and reconstruct a path via parent pointers.

### Step 2: DFS discovery order

Same graph; observe different traversal order.

### Step 3: Cycle detection (three-color DFS)

White / gray / black. A gray-to-gray edge signals a back-edge → cycle.

### Step 4: Connected components

DFS from every unvisited vertex; each component returned.

### Step 5: Tree-checking identity

For an undirected graph with n vertices: tree ⇔ connected ∧ (|E| = n - 1).

## Use It

- **Social networks**: BFS is "six degrees of separation" search.
- **Web crawling / page-rank precursor**: BFS frontier with politeness.
- **Build systems** (Phase 06): topo sort (DFS-finish-order reverse).
- **Compilers** (Phase 08): control-flow graphs are graphs; dominators are computed by DFS-based algorithms.
- **Cyber: lateral movement detection**: reachability queries on host-action graphs.
- **Static analysis**: data-flow analyses iterate over the CFG, often using BFS.

## Read the Source

- *CLRS*, Chapters 20–22 — definitive BFS, DFS, topo sort, SCC.
- *Algorithm Design* by Kleinberg & Tardos, Chapter 3 — beautiful BFS / DFS treatment.
- [NetworkX source](https://github.com/networkx/networkx/) — a reference Python graph library; read `algorithms/traversal/`.

## Ship It

This lesson ships **`outputs/graphs.py`** — `Graph` class with `bfs`, `dfs`, `connected_components`, `is_tree`, `spanning_tree`. Used by Lesson 18 (advanced graph problems) and Phase 04 L13–L18 (graph algorithms).

## Exercises

1. **Easy.** Build the graph of US states adjacent to each other; use BFS to find the shortest border-hop sequence from California to Maine.
2. **Medium.** Detect a cycle in a directed graph using DFS coloring (white/gray/black). Verify on both an acyclic and cyclic example.
3. **Hard.** Implement Tarjan's bridges-and-articulation-points algorithm: find every edge whose removal disconnects the graph, and every vertex whose removal does the same. Single DFS pass.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Adjacency list | "graph as a dict" | `node → list of neighbors`; O(V+E) space, fast for sparse graphs |
| BFS | "level-by-level traversal" | Visits vertices in order of distance from start; finds shortest paths in unweighted graphs |
| DFS | "go deep, then backtrack" | Visits as deep as possible before backtracking; used for cycles, topo sort, SCC |
| Tree | "connected, no cycles" | Equivalent: n-1 edges, connected, acyclic — pick any two |
| Spanning tree | "tree covering every vertex" | A subgraph of G that's a tree and includes all vertices; BFS/DFS produces one |

## Further Reading

- *Graph Theory* by Reinhard Diestel — definitive graduate-level text; free PDF from the author.
- *Networks, Crowds, and Markets* by Easley & Kleinberg — CS-applied graph theory; great chapter on small-world phenomena.
- [Graph algorithms in NetworkX docs](https://networkx.org/documentation/stable/reference/algorithms/index.html) — algorithm-by-algorithm reference.
