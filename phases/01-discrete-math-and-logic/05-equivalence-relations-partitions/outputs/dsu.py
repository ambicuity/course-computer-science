"""dsu.py — Disjoint Set Union (Union-Find) with union-by-rank + path compression.

This is the standard data structure for maintaining equivalence classes under
incremental `union(a, b)` operations. Amortized cost per operation is O(α(n)) —
inverse Ackermann, effectively constant for any n you'll ever encounter.

Used in Kruskal's MST, in incremental SAT solvers, in image-connected-components,
in cycle detection on undirected graphs.
"""
from __future__ import annotations

from typing import Dict, Hashable, List, Set


class DSU:
    """Disjoint Set Union over arbitrary hashable items."""

    def __init__(self) -> None:
        self.parent: Dict[Hashable, Hashable] = {}
        self.rank: Dict[Hashable, int] = {}

    def make_set(self, x: Hashable) -> None:
        if x not in self.parent:
            self.parent[x] = x
            self.rank[x] = 0

    def find(self, x: Hashable) -> Hashable:
        if x not in self.parent:
            raise KeyError(x)
        # Path compression: point each visited node directly at the root.
        root = x
        while self.parent[root] != root:
            root = self.parent[root]
        while self.parent[x] != root:
            self.parent[x], x = root, self.parent[x]
        return root

    def union(self, a: Hashable, b: Hashable) -> bool:
        """Merge the classes of a and b. Returns True if a merge actually happened."""
        self.make_set(a)
        self.make_set(b)
        ra, rb = self.find(a), self.find(b)
        if ra == rb:
            return False
        # Union by rank: shallower tree hangs under deeper.
        if self.rank[ra] < self.rank[rb]:
            ra, rb = rb, ra
        self.parent[rb] = ra
        if self.rank[ra] == self.rank[rb]:
            self.rank[ra] += 1
        return True

    def same(self, a: Hashable, b: Hashable) -> bool:
        return self.find(a) == self.find(b)

    def classes(self) -> List[Set[Hashable]]:
        buckets: Dict[Hashable, Set[Hashable]] = {}
        for x in list(self.parent):
            buckets.setdefault(self.find(x), set()).add(x)
        return list(buckets.values())


if __name__ == "__main__":
    dsu = DSU()
    for x in range(10):
        dsu.make_set(x)
    for a, b in [(0, 1), (1, 2), (3, 4), (5, 6), (6, 7), (8, 9)]:
        dsu.union(a, b)

    print("classes:")
    for c in sorted(dsu.classes(), key=lambda s: min(s)):
        print(f"  {sorted(c)}")

    print("same(0, 2)?  ", dsu.same(0, 2))   # True (0-1-2 are unioned)
    print("same(0, 5)?  ", dsu.same(0, 5))   # False
