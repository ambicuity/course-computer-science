"""graphs.py — basic graph algorithms reused throughout Phase 04 and beyond."""
from __future__ import annotations

from collections import deque
from typing import Dict, Hashable, Iterable, List, Optional, Tuple


class Graph:
    """Simple adjacency-list graph. Directed or undirected."""

    def __init__(self, directed: bool = False) -> None:
        self.directed = directed
        self.adj: Dict[Hashable, List[Hashable]] = {}

    def add_node(self, u: Hashable) -> None:
        self.adj.setdefault(u, [])

    def add_edge(self, u: Hashable, v: Hashable) -> None:
        self.add_node(u); self.add_node(v)
        self.adj[u].append(v)
        if not self.directed:
            self.adj[v].append(u)

    def __len__(self) -> int:
        return len(self.adj)

    def num_edges(self) -> int:
        total = sum(len(vs) for vs in self.adj.values())
        return total if self.directed else total // 2

    # ── Traversals ──────────────────────────────────────────────

    def bfs(self, start: Hashable) -> Tuple[Dict[Hashable, int], Dict[Hashable, Optional[Hashable]]]:
        dist: Dict[Hashable, int] = {start: 0}
        parent: Dict[Hashable, Optional[Hashable]] = {start: None}
        q = deque([start])
        while q:
            u = q.popleft()
            for v in self.adj.get(u, []):
                if v not in dist:
                    dist[v] = dist[u] + 1
                    parent[v] = u
                    q.append(v)
        return dist, parent

    def dfs(self, start: Hashable) -> List[Hashable]:
        order: List[Hashable] = []
        seen: set = set()
        stack = [start]
        while stack:
            u = stack.pop()
            if u in seen: continue
            seen.add(u); order.append(u)
            for v in reversed(self.adj.get(u, [])):
                if v not in seen:
                    stack.append(v)
        return order

    # ── Properties ──────────────────────────────────────────────

    def connected_components(self) -> List[List[Hashable]]:
        seen = set()
        comps = []
        for start in self.adj:
            if start in seen: continue
            comp = []
            stack = [start]
            while stack:
                u = stack.pop()
                if u in seen: continue
                seen.add(u); comp.append(u)
                stack.extend(self.adj.get(u, []))
            comps.append(comp)
        return comps

    def has_cycle(self) -> bool:
        """For directed graphs (3-color DFS); for undirected, checks for any cycle."""
        WHITE, GRAY, BLACK = 0, 1, 2
        color = {u: WHITE for u in self.adj}

        if self.directed:
            def visit(u):
                color[u] = GRAY
                for v in self.adj.get(u, []):
                    if color.get(v, WHITE) == GRAY:
                        return True
                    if color.get(v, WHITE) == WHITE and visit(v):
                        return True
                color[u] = BLACK
                return False
            return any(visit(u) for u in self.adj if color[u] == WHITE)
        else:
            # Undirected: DFS; cycle iff we see a visited neighbor that isn't the parent.
            def visit(u, par):
                color[u] = GRAY
                for v in self.adj.get(u, []):
                    if v == par: continue
                    if color.get(v, WHITE) == GRAY:
                        return True
                    if color.get(v, WHITE) == WHITE and visit(v, u):
                        return True
                color[u] = BLACK
                return False
            return any(visit(u, None) for u in self.adj if color[u] == WHITE)

    def is_tree(self) -> bool:
        n = len(self)
        if n == 0: return False
        if self.directed:
            return False  # 'tree' in this lesson is the undirected sense
        if self.num_edges() != n - 1:
            return False
        return len(self.connected_components()) == 1


if __name__ == "__main__":
    g = Graph()
    for u, v in [(1, 2), (2, 3), (1, 3), (4, 5)]:
        g.add_edge(u, v)
    assert len(g) == 5 and g.num_edges() == 4
    assert len(g.connected_components()) == 2

    tree = Graph()
    for u, v in [(0, 1), (0, 2), (1, 3), (1, 4)]:
        tree.add_edge(u, v)
    assert tree.is_tree()

    dg = Graph(directed=True)
    for u, v in [("A", "B"), ("B", "C"), ("C", "A")]:
        dg.add_edge(u, v)
    assert dg.has_cycle()

    print("graphs library smoke-test OK")
