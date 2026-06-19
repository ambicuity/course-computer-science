# Graph Algorithms III — Floyd-Warshall, Johnson, APSP

> You can get from anywhere to anywhere — if you know how to compute every shortest path at once.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 04 lessons 01–14 (especially lesson 14: Dijkstra, Bellman-Ford, A*)
**Time:** ~60 minutes

## Learning Objectives

- Implement Floyd-Warshall for all-pairs shortest paths and reconstruct actual paths via a next-hop matrix
- Implement Johnson's algorithm: reweight edges with Bellman-Ford potentials, then run Dijkstra from every vertex
- Understand when Floyd-Warshall wins (dense graphs, negative weights) vs. Johnson's (sparse graphs, no negative cycles)
- Build a working APSP solver that detects negative cycles

## The Problem

You have a weighted directed graph and need the shortest distance between *every* pair of vertices. Running Dijkstra from each source works, but that fails when edges have negative weights. Running Bellman-Ford from each source handles negatives but costs O(V²·E). You need something faster.

Floyd-Warshall solves this in O(V³) with a triple-nested loop and no negative-weight restriction (it just needs no negative *cycles*). Johnson's algorithm reweights the graph so Dijkstra applies everywhere, then runs Dijkstra V times — winning on sparse graphs where V·E log V < V³.

Without these algorithms you cannot build routing tables, compute graph diameter, detect reachability in dense graphs, or answer "what is the shortest path from any node to any node?"

## The Concept

### Floyd-Warshall: Dynamic Programming over Intermediate Vertices

The core idea is simple: consider every possible intermediate vertex k on the path from i to j. For each k, ask: "Is the shortest path from i to j that goes through k shorter than what I already have?"

**State:** `dist[i][j]` = shortest distance from vertex i to vertex j using only vertices {0, 1, ..., k} as intermediates.

**Transition:**
```
dist[i][j] = min(dist[i][j], dist[i][k] + dist[k][j])
```

You iterate k from 0 to V-1. At iteration k, every shortest path is allowed to pass through vertex k as an intermediate. The key insight: when you process k, `dist[i][k]` and `dist[k][j]` already reflect shortest paths using vertices {0, ..., k-1}, so `dist[i][k] + dist[k][j]` is the best path from i to j that *must* pass through k.

**Worked example** — 4 vertices, adjacency matrix (INF = no edge):

```
Initial:        After k=0:       After k=1:       After k=2:       After k=3:
  0  3  8  INF    0  3  8  INF    0  3  8  10    0  3  8  10    0  3  8  10
INF  0  2  INF  INF  0  2  INF  INF  0  2   4  INF  0  2   4  INF  0  2   4
INF INF  0   1  INF INF  0   1  INF INF  0   1  INF INF  0   1  INF INF  0   1
INF INF INF   0  INF INF INF   0  INF INF INF   0  INF INF INF   0  INF INF INF   0

Shortest paths: 0→1=3, 0→2=5 (0→1→2), 0→3=6 (0→1→2→3),
                1→2=2, 1→3=3 (1→2→3), 2→3=1
```

**Path reconstruction** uses a `next[i][j]` matrix. Initially `next[i][j] = j` if an edge exists, else `None`. When updating `dist[i][j]` through k, set `next[i][j] = next[i][k]`. To recover the full path from i to j, follow `next` from i until you reach j.

**Negative cycle detection:** after the algorithm, if `dist[i][i] < 0` for any i, vertex i lies on (or reaches) a negative cycle. Equivalently, run one more relaxation pass — if any distance improves, a negative cycle exists.

### Johnson's Algorithm: Reweight + V × Dijkstra

The bottleneck of running Dijkstra V times is negative edges. Johnson's removes them:

1. **Add a virtual source s** connected to every vertex with a 0-weight edge.
2. **Run Bellman-Ford from s** to get `h[v]` = shortest distance from s to v. If this detects a negative cycle, abort.
3. **Reweight each edge:** `w'(u, v) = w(u, v) + h[u] - h[v]`. This preserves shortest paths (the h-values cancel along any path) and ensures `w'(u, v) ≥ 0`.
4. **Run Dijkstra from each vertex** on the reweighted graph. Translate distances back: `d(u, v) = d'(u, v) - h[u] + h[v]`.

Why it wins on sparse graphs: Floyd-Warshall is always O(V³). Johnson's is O(V·E log V) — dominated by V Dijkstra runs with a binary-heap priority queue. When E ≪ V², this is much faster.

### Repeated Dijkstra (No Negative Weights)

If the graph has no negative edges, skip the reweighting. Just run Dijkstra from each vertex: O(V·E log V). This is a baseline comparison point.

### Algorithm Comparison

| Property | Floyd-Warshall | Johnson's | Repeated Dijkstra |
|----------|---------------|-----------|-------------------|
| Time | O(V³) | O(V·E log V) | O(V·E log V) |
| Negative weights | Yes (no neg. cycles) | Yes (via reweighting) | No |
| Negative cycle detection | Yes | Yes (Bellman-Ford step) | No |
| Memory | O(V²) | O(V²) + O(E) | O(V²) + O(E) |
| Best for | Dense graphs (E ≈ V²) | Sparse graphs (E ≪ V²) | Sparse, non-negative |
| Dense graph winner | **Yes** — V³ < V·V² log V | V·V² log V = V³ log V | Same as Johnson's |
| Sparse graph winner | V³ but E small | **Yes** — V·E log V ≪ V³ | **Yes** (if non-negative) |

## Build It

### Step 1: Floyd-Warshall

```python
def floyd_warshall(adj_matrix):
    V = len(adj_matrix)
    INF = float('inf')

    dist = [row[:] for row in adj_matrix]
    next_hop = [[None] * V for _ in range(V)]

    for i in range(V):
        for j in range(V):
            if i == j:
                dist[i][j] = 0
                next_hop[i][j] = j
            elif adj_matrix[i][j] != INF:
                next_hop[i][j] = j

    for k in range(V):
        for i in range(V):
            for j in range(V):
                if dist[i][k] + dist[k][j] < dist[i][j]:
                    dist[i][j] = dist[i][k] + dist[k][j]
                    next_hop[i][j] = next_hop[i][k]

    return dist, next_hop
```

### Step 2: Johnson's Algorithm

```python
import heapq

def johnson(adj_list, V):
    INF = float('inf')
    h = [0] * V

    for i in range(V):
        h[i] = 0
        for _ in range(V - 1):
            for u in range(V):
                for v, w in adj_list[u]:
                    if h[u] + w < h[v]:
                        h[v] = h[u] + w

    for u in range(V):
        for v, w in adj_list[u]:
            if h[u] + w < h[v]:
                raise ValueError("Negative cycle detected")

    reweighted = [[] for _ in range(V)]
    for u in range(V):
        for v, w in adj_list[u]:
            reweighted[u].append((v, w + h[u] - h[v]))

    dist = [[INF] * V for _ in range(V)]
    for src in range(V):
        dist[src] = _dijkstra(reweighted, src, V)

    for u in range(V):
        for v in range(V):
            if dist[u][v] != INF:
                dist[u][v] = dist[u][v] - h[u] + h[v]

    return dist


def _dijkstra(graph, src, V):
    dist = [float('inf')] * V
    dist[src] = 0
    pq = [(0, src)]

    while pq:
        d, u = heapq.heappop(pq)
        if d > dist[u]:
            continue
        for v, w in graph[u]:
            if dist[u] + w < dist[v]:
                dist[v] = dist[u] + w
                heapq.heappush(pq, (dist[v], v))

    return dist
```

### Step 3: Path Reconstruction and Negative Cycle Detection

```python
def reconstruct_path(next_hop, src, dst):
    if next_hop[src][dst] is None:
        return []
    path = [src]
    while src != dst:
        src = next_hop[src][dst]
        path.append(src)
    return path

def detect_negative_cycles_floyd(adj_matrix):
    dist, _ = floyd_warshall(adj_matrix)
    V = len(adj_matrix)
    for i in range(V):
        if dist[i][i] < 0:
            return True
    return False
```

## Use It

Floyd-Warshall and Johnson's appear across systems and research:

- **Network routing** — OSPF and IS-IS link-state protocols compute all-pairs shortest paths to build forwarding tables. Floyd-Warshall is the textbook algorithm for this.
- **Graph analysis libraries** — NetworkX's `floyd_warshall` and `johnson` functions wrap these algorithms. The Boost Graph Library provides C++ implementations used in large-scale infrastructure.
- **Social network analysis** — computing graph diameter (max shortest path) and closeness centrality requires APSP. Floyd-Warshall handles moderate-sized dense graphs efficiently.
- **Game development** — precomputing all-pairs distances for NPC pathfinding on small maps uses Floyd-Warshall's O(V²) memory table for O(1) lookup at runtime.

## Read the Source

- [NetworkX `floyd_warshall_predecessor_and_distance`](https://networkx.org/documentation/stable/reference/algorithms/generated/networkx.algorithms.shortest_paths.dense.floyd_warshall_predecessor_and_distance.html) — production Floyd-Warshall with predecessor tracking
- [CLRS Ch. 25 — All-Pairs Shortest Paths](https://mitpress.mit.edu/books/introduction-algorithms-fourth-edition) — rigorous treatment of Floyd-Warshall and Johnson's

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained APSP solver (`outputs/apsp_solver.py`) that accepts a weighted directed graph and returns all-pairs distances plus path reconstruction.**

## Exercises

1. **Easy.** Run Floyd-Warshall on the 4-vertex graph from the worked example. Verify the final distance matrix matches the expected values and reconstruct the path from vertex 0 to vertex 3.
2. **Medium.** Add a negative edge (e.g., vertex 2 → vertex 1 with weight −4) to the example graph. Run Floyd-Warshall, detect the negative cycle, and list all vertex pairs (i, j) where the shortest path passes through the negative cycle.
3. **Hard.** Implement transitive closure using a Floyd-Warshall variant: replace the `min` with logical OR (`reach[i][j] = reach[i][j] or (reach[i][k] and reach[k][j])`). Use this to compute the reflexive transitive closure of a directed graph and verify it in O(V³).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| APSP | "All-pairs shortest paths" | Computing shortest distances between every ordered pair of vertices in a graph |
| Floyd-Warshall | "The triple-loop algorithm" | DP algorithm that considers each vertex as a possible intermediate, relaxing all pairs through it — O(V³) |
| Johnson's algorithm | "Reweight and run Dijkstra" | Add virtual source, run Bellman-Ford for potentials, reweight edges to non-negative, run Dijkstra from each vertex |
| Potential function (h) | "The reweighting trick" | Assignment h[v] to each vertex such that reweighted edges w'(u,v) = w(u,v) + h[u] − h[v] are non-negative |
| Negative cycle | "A cycle whose total weight is negative" | A cycle where summing edge weights gives a negative value — shortest paths are undefined (can loop forever for −∞) |
| Next-hop matrix | "Predecessor table" | Matrix where next[i][j] stores the first vertex to visit on the shortest path from i to j, enabling path reconstruction |
| Transitive closure | "Reachability matrix" | Boolean matrix where reach[i][j] = True iff there exists any path from i to j, computable via Floyd-Warshall variant |

## Further Reading

- [CLRS, Chapter 25](https://mitpress.mit.edu/books/introduction-algorithms-fourth-edition) — all-pairs shortest paths: matrix multiplication approach, Floyd-Warshall, Johnson's
- [Jeff Erickson's Algorithms, Ch. 8](http://jeffe.cs.teaching.algorithms/books/algorithms.pdf) — shortest paths with excellent diagrams and intuition
- [Sedgewick & Wayne, *Algorithms*, Ch. 4.4](https://algs4.cs.princeton.edu/44sp/) — practical implementations with benchmarking
