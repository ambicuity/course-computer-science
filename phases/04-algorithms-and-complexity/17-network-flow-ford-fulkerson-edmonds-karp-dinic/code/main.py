"""
Network Flow — Ford-Fulkerson, Edmonds-Karp, Dinic
Phase 04 — Algorithms & Complexity Analysis

Three max-flow algorithms + min-cut recovery + bipartite matching reduction.
"""

from collections import deque


# ---------------------------------------------------------------------------
# Ford-Fulkerson with DFS augmenting paths
# ---------------------------------------------------------------------------

def ford_fulkerson(capacity: list[list[int]], source: int, sink: int) -> int:
    """Max-flow via DFS-based Ford-Fulkerson. O(E * max_flow)."""
    n = len(capacity)
    residual = [row[:] for row in capacity]
    flow = 0

    def dfs(u, min_cap, visited):
        if u == sink:
            return min_cap
        visited.add(u)
        for v in range(n):
            if v not in visited and residual[u][v] > 0:
                pushed = dfs(v, min(min_cap, residual[u][v]), visited)
                if pushed > 0:
                    residual[u][v] -= pushed
                    residual[v][u] += pushed
                    return pushed
        return 0

    while True:
        visited = set()
        pushed = dfs(source, float("inf"), visited)
        if pushed == 0:
            break
        flow += pushed
    return flow


# ---------------------------------------------------------------------------
# Edmonds-Karp (BFS augmenting paths)
# ---------------------------------------------------------------------------

def edmonds_karp(capacity: list[list[int]], source: int, sink: int) -> int:
    """Max-flow via BFS-based Edmonds-Karp. O(V E^2)."""
    n = len(capacity)
    residual = [row[:] for row in capacity]
    flow = 0

    def bfs():
        parent = [-1] * n
        parent[source] = source
        q = deque([source])
        while q:
            u = q.popleft()
            for v in range(n):
                if parent[v] == -1 and residual[u][v] > 0:
                    parent[v] = u
                    if v == sink:
                        return parent
                    q.append(v)
        return None

    while True:
        parent = bfs()
        if parent is None:
            break
        # bottleneck
        path_flow = float("inf")
        v = sink
        while v != source:
            path_flow = min(path_flow, residual[parent[v]][v])
            v = parent[v]
        # update residual
        v = sink
        while v != source:
            u = parent[v]
            residual[u][v] -= path_flow
            residual[v][u] += path_flow
            v = u
        flow += path_flow
    return flow


# ---------------------------------------------------------------------------
# Dinic's algorithm (level graph + blocking flow)
# ---------------------------------------------------------------------------

def dinics(capacity: list[list[int]], source: int, sink: int) -> int:
    """Max-flow via Dinic's algorithm. O(V^2 E)."""
    n = len(capacity)
    residual = [row[:] for row in capacity]
    flow = 0

    def bfs_level():
        level = [-1] * n
        level[source] = 0
        q = deque([source])
        while q:
            u = q.popleft()
            for v in range(n):
                if level[v] == -1 and residual[u][v] > 0:
                    level[v] = level[u] + 1
                    q.append(v)
        return level

    def dfs_blocking(u, pushed, level, ptr):
        if u == sink:
            return pushed
        while ptr[u] < n:
            v = ptr[u]
            if level[v] == level[u] + 1 and residual[u][v] > 0:
                bottleneck = dfs_blocking(v, min(pushed, residual[u][v]), level, ptr)
                if bottleneck > 0:
                    residual[u][v] -= bottleneck
                    residual[v][u] += bottleneck
                    return bottleneck
            ptr[u] += 1
        return 0

    while True:
        level = bfs_level()
        if level[sink] == -1:
            break
        ptr = [0] * n
        while True:
            pushed = dfs_blocking(source, float("inf"), level, ptr)
            if pushed == 0:
                break
            flow += pushed
    return flow


# ---------------------------------------------------------------------------
# Minimum cut recovery
# ---------------------------------------------------------------------------

def min_cut(capacity: list[list[int]], source: int, sink: int) -> tuple[int, list[tuple[int, int]]]:
    """Compute max-flow, then recover the minimum cut edges."""
    n = len(capacity)
    residual = [row[:] for row in capacity]

    # Run Dinic's to build residual graph
    def bfs_level():
        level = [-1] * n
        level[source] = 0
        q = deque([source])
        while q:
            u = q.popleft()
            for v in range(n):
                if level[v] == -1 and residual[u][v] > 0:
                    level[v] = level[u] + 1
                    q.append(v)
        return level

    def dfs_blocking(u, pushed, level, ptr):
        if u == sink:
            return pushed
        while ptr[u] < n:
            v = ptr[u]
            if level[v] == level[u] + 1 and residual[u][v] > 0:
                bottleneck = dfs_blocking(v, min(pushed, residual[u][v]), level, ptr)
                if bottleneck > 0:
                    residual[u][v] -= bottleneck
                    residual[v][u] += bottleneck
                    return bottleneck
            ptr[u] += 1
        return 0

    max_flow = 0
    while True:
        level = bfs_level()
        if level[sink] == -1:
            break
        ptr = [0] * n
        while True:
            pushed = dfs_blocking(source, float("inf"), level, ptr)
            if pushed == 0:
                break
            max_flow += pushed

    # Find nodes reachable from source in residual graph
    visited = set()
    stack = [source]
    while stack:
        u = stack.pop()
        if u in visited:
            continue
        visited.add(u)
        for v in range(n):
            if v not in visited and residual[u][v] > 0:
                stack.append(v)

    # Min-cut edges: original edges crossing from visited to not-visited
    cut_edges = []
    for u in visited:
        for v in range(n):
            if v not in visited and capacity[u][v] > 0:
                cut_edges.append((u, v))

    return max_flow, cut_edges


# ---------------------------------------------------------------------------
# Bipartite matching via max-flow
# ---------------------------------------------------------------------------

def bipartite_matching(left: int, right: int, edges: list[tuple[int, int]]) -> tuple[int, list[tuple[int, int]]]:
    """
    Max bipartite matching via Dinic's.

    left:  number of left-side nodes (indexed 0..left-1)
    right: number of right-side nodes (indexed 0..right-1)
    edges: list of (l, r) pairs representing allowed matches

    Returns (matching_size, list of matched (l, r) pairs).
    """
    total = left + right + 2
    src = 0
    snk = total - 1
    cap = [[0] * total for _ in range(total)]

    for l in range(left):
        cap[src][1 + l] = 1
    for r in range(right):
        cap[1 + left + r][snk] = 1
    for l, r in edges:
        cap[1 + l][1 + left + r] = 1

    # Dinic's in-place on cap (no copy)
    n = len(cap)
    flow = 0

    def bfs_level():
        level = [-1] * n
        level[src] = 0
        q = deque([src])
        while q:
            u = q.popleft()
            for v in range(n):
                if level[v] == -1 and cap[u][v] > 0:
                    level[v] = level[u] + 1
                    q.append(v)
        return level

    def dfs_blocking(u, pushed, level, ptr):
        if u == snk:
            return pushed
        while ptr[u] < n:
            v = ptr[u]
            if level[v] == level[u] + 1 and cap[u][v] > 0:
                bottleneck = dfs_blocking(v, min(pushed, cap[u][v]), level, ptr)
                if bottleneck > 0:
                    cap[u][v] -= bottleneck
                    cap[v][u] += bottleneck
                    return bottleneck
            ptr[u] += 1
        return 0

    while True:
        level = bfs_level()
        if level[snk] == -1:
            break
        ptr = [0] * n
        while True:
            pushed = dfs_blocking(src, float("inf"), level, ptr)
            if pushed == 0:
                break
            flow += pushed

    # Recover matched pairs: original edges with cap now 0 were saturated
    matched = []
    for l, r in edges:
        if cap[1 + l][1 + left + r] == 0:
            matched.append((l, r))

    return flow, matched


# ---------------------------------------------------------------------------
# Demo
# ---------------------------------------------------------------------------

def demo_pipeline():
    """Pipeline network from the lesson worked example."""
    print("=" * 60)
    print("DEMO 1: Pipeline network (worked example)")
    print("=" * 60)
    #
    #       10
    #  s ──────► a ──────► t
    #  │  4       8       │
    #  │         ▲        │
    #  └────► b ─┘        │
    #     9      6        │
    #     └─────►─────────┘
    #              10
    #
    # Nodes: s=0, a=1, b=2, t=3
    cap = [
        # s   a   b   t
        [ 0, 10,  9,  0],  # s
        [ 0,  0,  0,  8],  # a
        [ 0,  6,  0, 10],  # b
        [ 0,  0,  0,  0],  # t
    ]

    ff = ford_fulkerson(cap, 0, 3)
    ek = edmonds_karp(cap, 0, 3)
    di = dinics(cap, 0, 3)
    print(f"  Ford-Fulkerson: {ff}")
    print(f"  Edmonds-Karp:   {ek}")
    print(f"  Dinic's:        {di}")
    print(f"  All three agree: {ff == ek == di}")

    # Min cut
    flow, cut_edges = min_cut(cap, 0, 3)
    names = ["s", "a", "b", "t"]
    print(f"  Min-cut value: {flow}")
    print(f"  Cut edges: {[(names[u], names[v]) for u, v in cut_edges]}")
    print()


def demo_bipartite():
    """Bipartite matching: 4 workers, 5 jobs."""
    print("=" * 60)
    print("DEMO 2: Bipartite matching (4 workers, 5 jobs)")
    print("=" * 60)

    # Worker 0 can do jobs 0, 1
    # Worker 1 can do jobs 1, 2
    # Worker 2 can do jobs 0, 2, 3
    # Worker 3 can do jobs 3, 4
    edges = [
        (0, 0), (0, 1),
        (1, 1), (1, 2),
        (2, 0), (2, 2), (2, 3),
        (3, 3), (3, 4),
    ]

    size, matched = bipartite_matching(4, 5, edges)
    print(f"  Maximum matching size: {size}")
    print(f"  Matched pairs (worker, job): {matched}")

    # Verify via the flow network directly
    # source=0, workers=1..4, jobs=5..9, sink=10
    total = 4 + 5 + 2
    cap = [[0] * total for _ in range(total)]
    src, snk = 0, total - 1
    for l in range(4):
        cap[src][1 + l] = 1
    for r in range(5):
        cap[1 + 4 + r][snk] = 1
    for l, r in edges:
        cap[1 + l][1 + 4 + r] = 1
    di = dinics(cap, src, snk)
    print(f"  Dinic's direct verification: {di}")
    print()


def main() -> None:
    demo_pipeline()
    demo_bipartite()


if __name__ == "__main__":
    main()
