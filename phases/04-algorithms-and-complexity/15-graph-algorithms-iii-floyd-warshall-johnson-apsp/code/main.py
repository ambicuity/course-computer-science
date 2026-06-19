"""
Graph Algorithms III — Floyd-Warshall, Johnson, APSP
Phase 04 — Algorithms & Complexity Analysis

All-pairs shortest paths: Floyd-Warshall (O(V³)), Johnson's (O(V·E log V)),
repeated Dijkstra baseline, negative cycle detection, path reconstruction.
"""

import heapq
import random
import time


INF = float('inf')


# ---------------------------------------------------------------------------
# Floyd-Warshall — DP over intermediate vertices
# ---------------------------------------------------------------------------

def floyd_warshall(adj_matrix):
    """Compute all-pairs shortest paths via Floyd-Warshall.

    Args:
        adj_matrix: V×V list of lists. adj_matrix[i][j] is the weight of the
                    edge from i to j, or INF if no edge. Diagonal should be 0.

    Returns:
        (dist, next_hop) where dist[i][j] is the shortest distance and
        next_hop[i][j] is the next vertex on the shortest path from i to j
        (None if no path exists).
    """
    V = len(adj_matrix)

    dist = [row[:] for row in adj_matrix]
    next_hop = [[None] * V for _ in range(V)]

    for i in range(V):
        for j in range(V):
            if i == j:
                dist[i][j] = 0
                next_hop[i][j] = j
            elif dist[i][j] != INF:
                next_hop[i][j] = j

    for k in range(V):
        for i in range(V):
            for j in range(V):
                if dist[i][k] + dist[k][j] < dist[i][j]:
                    dist[i][j] = dist[i][k] + dist[k][j]
                    next_hop[i][j] = next_hop[i][k]

    return dist, next_hop


def reconstruct_path(next_hop, src, dst):
    """Reconstruct the shortest path from src to dst using next-hop matrix.

    Returns:
        List of vertices on the path, or empty list if no path exists.
    """
    if next_hop[src][dst] is None:
        return []
    path = [src]
    while src != dst:
        src = next_hop[src][dst]
        if src is None:
            return []
        path.append(src)
    return path


def detect_negative_cycles_floyd(adj_matrix):
    """Detect negative cycles using Floyd-Warshall.

    Returns:
        List of vertex indices that lie on or reachable from a negative cycle,
        or empty list if no negative cycle exists.
    """
    dist, _ = floyd_warshall(adj_matrix)
    V = len(adj_matrix)
    negative_vertices = []
    for i in range(V):
        if dist[i][i] < 0:
            negative_vertices.append(i)
    return negative_vertices


# ---------------------------------------------------------------------------
# Johnson's Algorithm — reweight + V × Dijkstra
# ---------------------------------------------------------------------------

def _dijkstra(graph, src, V):
    """Single-source shortest paths via Dijkstra's algorithm.

    Args:
        graph: adjacency list — graph[u] = [(v, weight), ...]
        src:   source vertex
        V:     number of vertices

    Returns:
        List of V distances from src.
    """
    dist = [INF] * V
    dist[src] = 0
    pq = [(0, src)]

    while pq:
        d, u = heapq.heappop(pq)
        if d > dist[u]:
            continue
        for v, w in graph[u]:
            nd = dist[u] + w
            if nd < dist[v]:
                dist[v] = nd
                heapq.heappush(pq, (nd, v))

    return dist


def johnson(adj_list, V):
    """All-pairs shortest paths via Johnson's algorithm.

    Args:
        adj_list: adjacency list — adj_list[u] = [(v, weight), ...]
        V:        number of vertices

    Returns:
        V×V distance matrix, or raises ValueError on negative cycle.

    Steps:
        1. Add virtual source, run Bellman-Ford for potential function h[v].
        2. Reweight edges: w'(u,v) = w(u,v) + h[u] - h[v]  (non-negative).
        3. Run Dijkstra from each vertex on the reweighted graph.
        4. Translate back: d(u,v) = d'(u,v) - h[u] + h[v].
    """
    # Step 1: Bellman-Ford from virtual source (vertex V added conceptually).
    h = [0] * V  # virtual source has 0-weight edges to all vertices

    for _ in range(V - 1):
        for u in range(V):
            for v, w in adj_list[u]:
                if h[u] + w < h[v]:
                    h[v] = h[u] + w

    # Check for negative cycles
    for u in range(V):
        for v, w in adj_list[u]:
            if h[u] + w < h[v]:
                raise ValueError("Negative cycle detected — Johnson's cannot proceed")

    # Step 2: Reweight edges
    reweighted = [[] for _ in range(V)]
    for u in range(V):
        for v, w in adj_list[u]:
            reweighted[u].append((v, w + h[u] - h[v]))

    # Step 3: Dijkstra from each vertex
    dist = [[INF] * V for _ in range(V)]
    for src in range(V):
        dist[src] = _dijkstra(reweighted, src, V)

    # Step 4: Translate distances back
    for u in range(V):
        for v in range(V):
            if dist[u][v] != INF:
                dist[u][v] = dist[u][v] - h[u] + h[v]

    return dist


# ---------------------------------------------------------------------------
# Repeated Dijkstra — baseline for non-negative graphs
# ---------------------------------------------------------------------------

def repeated_dijkstra(adj_list, V):
    """All-pairs shortest paths by running Dijkstra from each vertex.

    Only correct when all edge weights are non-negative.
    """
    dist = [[INF] * V for _ in range(V)]
    for src in range(V):
        dist[src] = _dijkstra(adj_list, src, V)
    return dist


# ---------------------------------------------------------------------------
# Graph generation helpers
# ---------------------------------------------------------------------------

def adj_matrix_to_adj_list(adj_matrix):
    """Convert V×V adjacency matrix (with INF) to adjacency list."""
    V = len(adj_matrix)
    adj_list = [[] for _ in range(V)]
    for u in range(V):
        for v in range(V):
            if u != v and adj_matrix[u][v] != INF:
                adj_list[u].append((v, adj_matrix[u][v]))
    return adj_list


def generate_dense_graph(V, negative=False, seed=42):
    """Generate a random dense graph as an adjacency matrix."""
    rng = random.Random(seed)
    matrix = [[INF] * V for _ in range(V)]
    for i in range(V):
        matrix[i][i] = 0
        for j in range(V):
            if i != j:
                if rng.random() < 0.8:  # 80% edge probability = dense
                    w = rng.randint(1, 20)
                    if negative and rng.random() < 0.15:
                        w = -rng.randint(1, 5)
                    matrix[i][j] = w
    return matrix


def generate_sparse_graph(V, negative=False, seed=42):
    """Generate a random sparse graph as an adjacency list."""
    rng = random.Random(seed)
    adj_list = [[] for _ in range(V)]
    edges_per_vertex = max(2, int(V * 0.3))
    for u in range(V):
        targets = rng.sample(range(V), min(edges_per_vertex, V))
        for v in targets:
            if u != v:
                w = rng.randint(1, 20)
                if negative and rng.random() < 0.1:
                    w = -rng.randint(1, 5)
                adj_list[u].append((v, w))
    return adj_list


# ---------------------------------------------------------------------------
# Transitive closure via Floyd-Warshall variant
# ---------------------------------------------------------------------------

def transitive_closure(adj_matrix):
    """Compute the transitive closure of a directed graph.

    Uses a Floyd-Warshall variant with boolean OR instead of min.
    reach[i][j] is True iff there exists any path from i to j.
    """
    V = len(adj_matrix)
    reach = [[False] * V for _ in range(V)]

    for i in range(V):
        for j in range(V):
            if i == j:
                reach[i][j] = True
            elif adj_matrix[i][j] != INF and adj_matrix[i][j] != 0:
                reach[i][j] = True

    for k in range(V):
        for i in range(V):
            for j in range(V):
                reach[i][j] = reach[i][j] or (reach[i][k] and reach[k][j])

    return reach


# ---------------------------------------------------------------------------
# Benchmarks
# ---------------------------------------------------------------------------

def benchmark(label, fn, *args):
    """Run fn(*args), return (result, elapsed_ms)."""
    start = time.perf_counter()
    result = fn(*args)
    elapsed = (time.perf_counter() - start) * 1000
    print(f"  {label}: {elapsed:.2f} ms")
    return result, elapsed


# ---------------------------------------------------------------------------
# Main — demonstrations
# ---------------------------------------------------------------------------

def main():
    print("=" * 65)
    print("  GRAPH ALGORITHMS III — Floyd-Warshall, Johnson, APSP")
    print("=" * 65)

    # ------------------------------------------------------------------
    # 1. Floyd-Warshall on the worked example
    # ------------------------------------------------------------------
    print("\n--- 1. Floyd-Warshall: 4-vertex worked example ---\n")

    adj = [
        [0,   3,   8,   INF],
        [INF, 0,   2,   INF],
        [INF, INF, 0,   1  ],
        [INF, INF, INF, 0  ],
    ]

    dist, nxt = floyd_warshall(adj)
    V = 4
    print("  Distance matrix:")
    for i in range(V):
        row = []
        for j in range(V):
            row.append(f"{dist[i][j]:>6}" if dist[i][j] != INF else "   INF")
        print(f"    {'  '.join(row)}")

    print("\n  Shortest paths:")
    for i in range(V):
        for j in range(V):
            if i != j:
                path = reconstruct_path(nxt, i, j)
                if path:
                    print(f"    {i}→{j}: {' → '.join(map(str, path))}  (dist={dist[i][j]})")

    # ------------------------------------------------------------------
    # 2. Negative cycle detection
    # ------------------------------------------------------------------
    print("\n--- 2. Negative cycle detection ---\n")

    adj_neg = [
        [0,   1,   INF, INF],
        [INF, 0,  -1,   INF],
        [INF, INF, 0,   -1 ],
        [-1,  INF, INF, 0  ],
    ]
    neg_verts = detect_negative_cycles_floyd(adj_neg)
    if neg_verts:
        print(f"  Negative cycle vertices: {neg_verts}")
    else:
        print("  No negative cycle detected.")

    # ------------------------------------------------------------------
    # 3. Johnson's algorithm
    # ------------------------------------------------------------------
    print("\n--- 3. Johnson's algorithm: 4-vertex example ---\n")

    adj_list = adj_matrix_to_adj_list(adj)
    try:
        dist_j = johnson(adj_list, V)
        print("  Distance matrix (Johnson):")
        for i in range(V):
            row = []
            for j in range(V):
                row.append(f"{dist_j[i][j]:>6}" if dist_j[i][j] != INF else "   INF")
            print(f"    {'  '.join(row)}")
    except ValueError as e:
        print(f"  {e}")

    # ------------------------------------------------------------------
    # 4. Transitive closure
    # ------------------------------------------------------------------
    print("\n--- 4. Transitive closure (Floyd-Warshall variant) ---\n")

    adj_bool = [
        [0,   1,   INF, INF],
        [INF, 0,   1,   INF],
        [INF, INF, 0,   1  ],
        [INF, INF, INF, 0  ],
    ]
    tc = transitive_closure(adj_bool)
    print("  Reachability matrix:")
    for i in range(V):
        row = "  ".join("T" if tc[i][j] else "F" for j in range(V))
        print(f"    [{row}]")

    # ------------------------------------------------------------------
    # 5. Benchmarks — dense vs sparse
    # ------------------------------------------------------------------
    print("\n--- 5. Benchmarks: dense vs sparse ---\n")

    for V_bench in [50, 100, 200]:
        print(f"\n  V = {V_bench}")

        dense_mat = generate_dense_graph(V_bench, negative=False)
        dense_list = adj_matrix_to_adj_list(dense_mat)

        sparse_list = generate_sparse_graph(V_bench, negative=False)

        # Floyd-Warshall on dense
        benchmark(f"  Floyd-Warshall (dense, V={V_bench})", floyd_warshall, dense_mat)

        # Johnson on dense
        benchmark(f"  Johnson (dense, V={V_bench})", johnson, dense_list, V_bench)

        # Repeated Dijkstra on dense
        benchmark(f"  Repeated Dijkstra (dense, V={V_bench})", repeated_dijkstra, dense_list, V_bench)

        # Repeated Dijkstra on sparse
        benchmark(f"  Repeated Dijkstra (sparse, V={V_bench})", repeated_dijkstra, sparse_list, V_bench)

    print("\n" + "=" * 65)
    print("  Done.")
    print("=" * 65)


if __name__ == "__main__":
    main()
