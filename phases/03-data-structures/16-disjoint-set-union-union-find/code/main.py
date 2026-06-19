"""main.py — Union-Find in Python with Kruskal MST."""
from __future__ import annotations


class DSU:
    def __init__(self, n: int) -> None:
        self.parent = list(range(n))
        self.rank = [0] * n

    def find(self, x: int) -> int:
        while self.parent[x] != x:
            self.parent[x] = self.parent[self.parent[x]]  # path halving
            x = self.parent[x]
        return x

    def unite(self, x: int, y: int) -> bool:
        rx, ry = self.find(x), self.find(y)
        if rx == ry: return False
        if self.rank[rx] < self.rank[ry]: self.parent[rx] = ry
        elif self.rank[rx] > self.rank[ry]: self.parent[ry] = rx
        else:
            self.parent[ry] = rx
            self.rank[rx] += 1
        return True

    def connected(self, x: int, y: int) -> bool:
        return self.find(x) == self.find(y)


def kruskal(n: int, edges: list[tuple[int, int, int]]) -> int:
    edges = sorted(edges, key=lambda e: e[2])
    d = DSU(n)
    total = 0; picked = 0
    for u, v, w in edges:
        if d.unite(u, v):
            total += w
            picked += 1
            if picked == n - 1: break
    return total if picked == n - 1 else -1


def main() -> None:
    d = DSU(10)
    for u, v in [(1, 2), (2, 3), (5, 6), (7, 6)]: d.unite(u, v)
    print("connected(1, 3) =", d.connected(1, 3), "(expect True)")
    print("connected(1, 5) =", d.connected(1, 5), "(expect False)")

    edges = [
        (0, 1, 4), (0, 2, 3), (1, 2, 1), (1, 3, 2),
        (2, 3, 4), (3, 4, 2), (4, 0, 4), (4, 2, 4),
    ]
    print("\nKruskal MST weight:", kruskal(5, edges), "(expect 8)")


if __name__ == "__main__":
    main()
