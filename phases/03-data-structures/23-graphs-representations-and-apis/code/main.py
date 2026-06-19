"""main.py — graph representations in Python with BFS."""
from __future__ import annotations
from collections import defaultdict, deque


class AdjList:
    def __init__(self, n: int) -> None:
        self.n = n
        self.adj: list[list[int]] = [[] for _ in range(n)]

    def add_edge(self, u: int, v: int) -> None:
        self.adj[u].append(v)

    def neighbors(self, u: int) -> list[int]:
        return self.adj[u]


class DictGraph:
    """NetworkX-style: dict of dicts. Most flexible, slowest."""
    def __init__(self) -> None:
        self.adj: dict[int, dict[int, dict]] = defaultdict(dict)

    def add_edge(self, u: int, v: int, **attrs) -> None:
        self.adj[u][v] = attrs

    def neighbors(self, u: int) -> list[int]:
        return list(self.adj.get(u, {}).keys())


def bfs(g, src: int, n: int) -> int:
    dist = [-1] * n
    dist[src] = 0
    q = deque([src])
    reached = 0
    while q:
        u = q.popleft()
        reached += 1
        for v in g.neighbors(u):
            if dist[v] == -1:
                dist[v] = dist[u] + 1
                q.append(v)
    return reached


def main() -> None:
    import random, time
    random.seed(42)
    n, m = 1000, 8000
    edges = [(random.randrange(n), random.randrange(n)) for _ in range(m)]

    al = AdjList(n)
    for u, v in edges: al.add_edge(u, v)

    dg = DictGraph()
    for u, v in edges: dg.add_edge(u, v)

    t0 = time.perf_counter()
    for _ in range(50): r1 = bfs(al, 0, n)
    t_al = time.perf_counter() - t0

    t0 = time.perf_counter()
    for _ in range(50): r2 = bfs(dg, 0, n)
    t_dg = time.perf_counter() - t0

    print(f"BFS reaches {r1} (list) / {r2} (dict)")
    print(f"AdjList :    {t_al * 1000:.1f} ms / 50 runs")
    print(f"DictGraph:   {t_dg * 1000:.1f} ms / 50 runs (flexible but slower)")


if __name__ == "__main__":
    main()
