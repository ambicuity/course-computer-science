# Graph Algorithms II — Dijkstra, Bellman-Ford, A*

> Shortest paths are the backbone of routing, navigation, and network optimization. Today you build three algorithms: one for non-negative weights, one for negative weights, and one for heuristic-guided search.

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 04 lessons 01–13
**Time:** ~75 minutes

## Learning Objectives

- Implement Dijkstra with a min-heap; understand why negative edges break it.
- Implement Bellman-Ford with negative cycle detection and path reconstruction.
- Implement A* with an admissible heuristic on a 2D grid.

## The Problem

Lesson 13 gave you BFS and DFS for unweighted graphs. Real graphs have weights — road distances, latencies, costs. BFS finds the fewest *edges*, not the cheapest *path*.

## The Concept

### Dijkstra — Greedy Relaxation

Solves single-source shortest paths on graphs with **non-negative edge weights**.

Greedily pick the unfinalized vertex with smallest tentative distance, finalize it, relax outgoing edges. When we extract `u` with minimum `d[u]`, no alternative path can be shorter: any detour goes through an unfinalized `v` with `d[v] >= d[u]` and `w(v,u) >= 0`, so `d[v] + w(v,u) >= d[u]`.

**Complexity:** O((V+E) log V) with binary heap, O(E+V log V) with Fibonacci heap. **Failure mode:** negative edges break the greedy argument. **Path reconstruction:** maintain `prev[v]` — set when relaxation improves `d[v]`.

### Bellman-Ford — Systematic Relaxation

Handles **arbitrary weights including negative**. Relax *all* edges V-1 times. After `k` iterations, distances are correct for paths of at most `k` edges. Run a V-th pass: if any distance improves, a negative cycle exists. **Complexity:** O(VE). Use for negative edges or cycle detection (e.g., currency arbitrage).

### A* — Guided Exploration

Dijkstra enhanced with heuristic `h(u)`. Prioritize by `f(u) = d[u] + h(u)`. An **admissible heuristic** (never overestimates) guarantees optimality: a suboptimal return at cost `C_sub` implies some optimal-path vertex has `f <= C_opt < C_sub` and would have expanded first. Manhattan distance `|dx|+|dy|` is admissible for 4-dir grids. Worst case O((V+E) log V), but explores far fewer nodes.

### Comparison

| Algorithm | Negative weights | Time | Best for |
|-----------|-----------------|------|----------|
| Dijkstra | No | O((V+E) log V) | Non-negative weights |
| Bellman-Ford | Yes | O(VE) | Negative edges, cycle detection |
| A* | No | O((V+E) log V) worst | Known goal, good heuristic |

## Build It

### Step 1: Dijkstra with Min-Heap

```python
import heapq

def dijkstra(graph, src):
    dist = {v: float('inf') for v in graph}
    prev = {v: None for v in graph}
    dist[src] = 0
    pq = [(0, src)]

    while pq:
        d, u = heapq.heappop(pq)
        if d > dist[u]:
            continue
        for v, w in graph[u]:
            nd = d + w
            if nd < dist[v]:
                dist[v] = nd
                prev[v] = u
                heapq.heappush(pq, (nd, v))

    return dist, prev
```

### Step 2: Bellman-Ford with Negative Cycle Detection

```python
def bellman_ford(edges, V, src):
    dist = [float('inf')] * V
    prev = [None] * V
    dist[src] = 0

    for _ in range(V - 1):
        updated = False
        for u, v, w in edges:
            if dist[u] + w < dist[v]:
                dist[v] = dist[u] + w
                prev[v] = u
                updated = True
        if not updated:
            break

    for u, v, w in edges:
        if dist[u] + w < dist[v]:
            cycle_node = v
            for _ in range(V):
                cycle_node = prev[cycle_node]
            cycle = []
            node = cycle_node
            while True:
                cycle.append(node)
                node = prev[node]
                if node == cycle_node:
                    cycle.append(node)
                    break
            cycle.reverse()
            return dist, prev, cycle

    return dist, prev, None
```

### Step 3: A* on a 2D Grid

```python
import heapq

def astar(grid, src, goal, rows, cols):
    def h(pos):
        return abs(pos[0] - goal[0]) + abs(pos[1] - goal[1])

    def neighbors(pos):
        r, c = pos
        for dr, dc in [(-1,0),(1,0),(0,-1),(0,1)]:
            nr, nc = r+dr, c+dc
            if 0 <= nr < rows and 0 <= nc < cols and grid[nr][nc] == 0:
                yield (nr, nc)

    g_score = {src: 0}
    prev = {src: None}
    pq = [(h(src), 0, src)]

    while pq:
        f, g, u = heapq.heappop(pq)
        if u == goal:
            path = []
            node = goal
            while node is not None:
                path.append(node)
                node = prev[node]
            path.reverse()
            return path, g_score[goal]
        if g > g_score.get(u, float('inf')):
            continue
        for v in neighbors(u):
            ng = g + 1
            if ng < g_score.get(v, float('inf')):
                g_score[v] = ng
                prev[v] = u
                heapq.heappush(pq, (ng + h(v), ng, v))

    return None, float('inf')
```

Full implementations with path reconstruction, tests, and benchmarks are in `code/main.py` and `code/main.rs`.

## Use It

**GPS / Navigation:** Google Maps and OSRM use Dijkstra variants. Contraction Hierarchies accelerate Dijkstra by orders of magnitude. **Routing:** OSPF uses Dijkstra for routing tables. **Currency arbitrage:** Bellman-Ford detects negative cycles — negate log of exchange rates; a negative cycle means profit. **Game pathfinding:** A* dominates with Manhattan/Chebyshev heuristics.

## Read the Source

- NetworkX `networkx/algorithms/shortest_paths/weighted.py` — production Dijkstra and Bellman-Ford.
- `pathfinding` Rust crate `src/astar.rs` — A* with generic heuristic support.

## Ship It

The reusable artifact in `outputs/` is **a shortest-path library with Dijkstra, Bellman-Ford, A*, and path reconstruction** — reusable in later graph lessons and the phase capstone.

## Exercises

1. **Easy** — Implement Dijkstra with decrease-key on a Fibonacci heap. Compare heap operations against binary-heap on a dense graph with 100 vertices.

2. **Medium** — After Bellman-Ford detects a negative cycle, return *all* edges involved in negative cycles. Explore from the V-th-pass relaxed vertex.

3. **Hard** — A* on a 2D grid with weighted terrain (grass=1, sand=3, water=5). Add diagonal movement (√2 cost) with octile heuristic. Compare nodes expanded vs Dijkstra.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Relaxation | "try to improve a distance" | If `d[u] + w(u,v) < d[v]`, update `d[v]`, set `prev[v] = u` |
| Admissible heuristic | "never overestimates" | `h(u) <= true_cost(u, goal)`; guarantees A* optimality |
| Negative cycle | "profit loop" | Cycle with negative total weight; shortest paths undefined |
| Decrease-key | "update priority" | Lower key of existing element in the priority queue |
| Manhattan distance | "grid distance" | `|x1-x2| + |y1-y2|`; admissible for 4-directional movement |
| Finalized vertex | "settled" | Vertex whose shortest distance has been proven correct |

## Further Reading

- T. Cormen et al., *Introduction to Algorithms* (CLRS), Chapter 24.
- P. Hart et al., "A Formal Basis for the Heuristic Determination of Minimum Cost Paths," IEEE Trans. SSC, 1968.
- R. Bellman, "On a Routing Problem," Quarterly of Applied Mathematics, 1958.
