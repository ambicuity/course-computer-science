"""
Graph Algorithms II — Dijkstra, Bellman-Ford, A*
Phase 04 — Algorithms & Complexity Analysis

Shortest path algorithms with path reconstruction and benchmarks.
"""

import heapq
import math
import random
import time
from typing import Optional


# ---------------------------------------------------------------------------
# Dijkstra
# ---------------------------------------------------------------------------

def dijkstra(graph: dict[int, list[tuple[int, float]]], src: int):
    """Single-source shortest paths (non-negative weights).
    Returns (dist, prev) dicts."""
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


def reconstruct_path(prev: dict, target: int) -> list[int]:
    path = []
    node = target
    while node is not None:
        path.append(node)
        node = prev[node]
    path.reverse()
    return path


# ---------------------------------------------------------------------------
# Bellman-Ford
# ---------------------------------------------------------------------------

def bellman_ford(edges: list[tuple[int, int, float]], V: int, src: int):
    """Single-source shortest paths (arbitrary weights).
    Returns (dist, prev, negative_cycle). negative_cycle is None or a list
    of vertices forming a negative cycle."""
    dist = [float('inf')] * V
    prev_arr = [None] * V
    dist[src] = 0

    for _ in range(V - 1):
        updated = False
        for u, v, w in edges:
            if dist[u] + w < dist[v]:
                dist[v] = dist[u] + w
                prev_arr[v] = u
                updated = True
        if not updated:
            break

    # Negative cycle detection
    for u, v, w in edges:
        if dist[u] + w < dist[v]:
            cycle_node = v
            for _ in range(V):
                cycle_node = prev_arr[cycle_node]
            cycle = []
            node = cycle_node
            while True:
                cycle.append(node)
                node = prev_arr[node]
                if node == cycle_node:
                    cycle.append(node)
                    break
            cycle.reverse()
            return dist, prev_arr, cycle

    return dist, prev_arr, None


def reconstruct_path_bf(prev_arr: list, target: int) -> list[int]:
    path = []
    node = target
    while node is not None:
        path.append(node)
        node = prev_arr[node]
    path.reverse()
    return path


# ---------------------------------------------------------------------------
# A* Search
# ---------------------------------------------------------------------------

def astar(grid: list[list[int]], src: tuple, goal: tuple):
    """A* on a 2D grid. 0=walkable, 1=obstacle.
    Returns (path, cost) or (None, inf) if unreachable."""
    rows, cols = len(grid), len(grid[0])

    def h(pos):
        return abs(pos[0] - goal[0]) + abs(pos[1] - goal[1])

    def neighbors(pos):
        r, c = pos
        for dr, dc in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
            nr, nc = r + dr, c + dc
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
            return path, g

        if g > g_score.get(u, float('inf')):
            continue

        for v in neighbors(u):
            ng = g + 1
            if ng < g_score.get(v, float('inf')):
                g_score[v] = ng
                prev[v] = u
                heapq.heappush(pq, (ng + h(v), ng, v))

    return None, float('inf')


# ---------------------------------------------------------------------------
# Comparison Benchmarks
# ---------------------------------------------------------------------------

def generate_random_graph(V: int, E: int, max_weight: int = 100) -> dict:
    graph = {i: [] for i in range(V)}
    edges = []
    for _ in range(E):
        u = random.randint(0, V - 1)
        v = random.randint(0, V - 1)
        w = random.randint(1, max_weight)
        graph[u].append((v, w))
        edges.append((u, v, w))
    return graph, edges


def generate_weighted_grid(rows: int, cols: int, obstacle_pct: float = 0.2):
    grid = [[0] * cols for _ in range(rows)]
    for r in range(rows):
        for c in range(cols):
            if random.random() < obstacle_pct:
                grid[r][c] = 1
    return grid


def benchmark():
    print("=== Benchmark: Dijkstra vs Bellman-Ford ===")
    print(f"{'V':>6} {'E':>8} {'Dijkstra (ms)':>14} {'Bellman-Ford (ms)':>18}")
    print("-" * 50)

    for V in [100, 500, 1000]:
        E = V * 4
        graph, edges = generate_random_graph(V, E)

        t0 = time.perf_counter()
        for _ in range(5):
            dijkstra(graph, 0)
        dt = (time.perf_counter() - t0) / 5 * 1000

        t0 = time.perf_counter()
        for _ in range(5):
            bellman_ford(edges, V, 0)
        bt = (time.perf_counter() - t0) / 5 * 1000

        print(f"{V:>6} {E:>8} {dt:>14.2f} {bt:>18.2f}")


# ---------------------------------------------------------------------------
# Main — demonstrations
# ---------------------------------------------------------------------------

def main() -> None:
    # --- Dijkstra demo ---
    print("=" * 60)
    print("DIJKSTRA'S ALGORITHM")
    print("=" * 60)

    graph = {
        0: [(1, 4), (2, 1)],
        1: [(3, 1)],
        2: [(1, 2), (3, 5)],
        3: [(4, 3)],
        4: [],
    }
    dist, prev = dijkstra(graph, 0)
    print(f"Shortest distances from 0: {dist}")
    print(f"Path 0 -> 4: {reconstruct_path(prev, 4)}")
    print(f"Path 0 -> 1: {reconstruct_path(prev, 1)}")

    # --- Bellman-Ford demo ---
    print()
    print("=" * 60)
    print("BELLMAN-FORD ALGORITHM")
    print("=" * 60)

    edges = [
        (0, 1, 4), (0, 2, 1),
        (1, 3, 1), (2, 1, 2),
        (2, 3, 5), (3, 4, 3),
    ]
    dist_bf, prev_bf, cycle = bellman_ford(edges, 5, 0)
    print(f"Shortest distances from 0: {dist_bf}")
    print(f"Path 0 -> 4: {reconstruct_path_bf(prev_bf, 4)}")

    # Negative cycle demo
    neg_edges = [
        (0, 1, 1), (1, 2, -3),
        (2, 3, -1), (3, 1, 2),
    ]
    _, _, cycle = bellman_ford(neg_edges, 4, 0)
    print(f"Negative cycle detected: {cycle}")

    # No negative cycle
    safe_edges = [
        (0, 1, 1), (1, 2, 2),
        (2, 3, 3),
    ]
    _, _, cycle = bellman_ford(safe_edges, 4, 0)
    print(f"Negative cycle (should be None): {cycle}")

    # --- A* demo ---
    print()
    print("=" * 60)
    print("A* SEARCH")
    print("=" * 60)

    grid = [
        [0, 0, 0, 0, 0],
        [0, 1, 1, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 1, 1, 0],
        [0, 0, 0, 0, 0],
    ]
    path, cost = astar(grid, (0, 0), (4, 4))
    print(f"Path from (0,0) to (4,4): {path}")
    print(f"Cost: {cost}")

    # Unreachable
    blocked = [
        [0, 1, 0],
        [1, 1, 1],
        [0, 1, 0],
    ]
    path, cost = astar(blocked, (0, 0), (2, 2))
    print(f"Blocked path: {path}, cost: {cost}")

    # --- Correctness checks ---
    print()
    print("=" * 60)
    print("CORRECTNESS CHECKS")
    print("=" * 60)

    # Dijkstra vs Bellman-Ford on non-negative graph
    g, e = generate_random_graph(50, 200, 100)
    d_dij, _ = dijkstra(g, 0)
    d_bf, _, _ = bellman_ford(e, 50, 0)
    match = all(
        abs(d_dij.get(v, float('inf')) - (d_bf[v] if v < 50 else float('inf'))) < 1e-9
        for v in g
    )
    print(f"Dijkstra == Bellman-Ford on non-negative graph: {match}")

    # A* returns optimal path
    small_grid = [[0] * 10 for _ in range(10)]
    path_astar, cost_astar = astar(small_grid, (0, 0), (9, 9))
    print(f"A* on empty 10x10 grid: cost={cost_astar} (expected 18)")

    # --- Benchmark ---
    print()
    benchmark()

    print()
    print("All demos complete.")


if __name__ == "__main__":
    main()
