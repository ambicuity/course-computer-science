"""Graph fundamentals: BFS, DFS, cycle detection, connected components, tree test.

Run:  python3 main.py
"""
from __future__ import annotations

from collections import defaultdict, deque
from typing import Dict, List, Optional


# ── BFS ────────────────────────────────────────────────────────────

def bfs(adj: Dict, start) -> tuple[Dict, Dict]:
    dist: Dict = {start: 0}
    parent: Dict = {start: None}
    q = deque([start])
    while q:
        u = q.popleft()
        for v in adj.get(u, []):
            if v not in dist:
                dist[v] = dist[u] + 1
                parent[v] = u
                q.append(v)
    return dist, parent


def shortest_path(parent: Dict, target) -> List:
    if target not in parent:
        return []
    path = []
    cur = target
    while cur is not None:
        path.append(cur)
        cur = parent[cur]
    return list(reversed(path))


# ── DFS ────────────────────────────────────────────────────────────

def dfs(adj: Dict, start) -> List:
    visited, order = set(), []
    def rec(u):
        visited.add(u); order.append(u)
        for v in adj.get(u, []):
            if v not in visited:
                rec(v)
    rec(start)
    return order


# ── Cycle detection in a directed graph (three-color DFS) ─────────

def has_cycle_directed(adj: Dict) -> bool:
    WHITE, GRAY, BLACK = 0, 1, 2
    color = {u: WHITE for u in adj}

    def dfs_visit(u):
        color[u] = GRAY
        for v in adj.get(u, []):
            if color.get(v, WHITE) == GRAY:
                return True
            if color.get(v, WHITE) == WHITE and dfs_visit(v):
                return True
        color[u] = BLACK
        return False

    for u in adj:
        if color[u] == WHITE:
            if dfs_visit(u):
                return True
    return False


# ── Connected components (undirected) ─────────────────────────────

def connected_components(adj: Dict) -> List[List]:
    seen = set()
    comps = []
    for start in adj:
        if start in seen: continue
        c, stack = [], [start]
        while stack:
            u = stack.pop()
            if u in seen: continue
            seen.add(u); c.append(u)
            stack.extend(adj.get(u, []))
        comps.append(c)
    return comps


def is_tree(adj: Dict) -> bool:
    """Undirected graph is a tree iff connected and |E| = |V| - 1.
    Assumes adj stores both directions for undirected edges."""
    n = len(adj)
    if n == 0: return False
    edges = sum(len(v) for v in adj.values()) // 2
    if edges != n - 1: return False
    comps = connected_components(adj)
    return len(comps) == 1


# ── Demo ──────────────────────────────────────────────────────────

def main() -> None:
    # Undirected social graph
    social = {
        "Alice": ["Bob", "Carol"],
        "Bob":   ["Alice", "Dan"],
        "Carol": ["Alice"],
        "Dan":   ["Bob"],
        "Eve":   ["Frank"],
        "Frank": ["Eve"],
    }

    print("== BFS from Alice ==")
    dist, parent = bfs(social, "Alice")
    for n in sorted(dist):
        print(f"  {n}: dist={dist[n]}, path={' → '.join(shortest_path(parent, n))}")
    print(f"  Eve / Frank are unreachable: {'Eve' not in dist}")

    print("\n== DFS from Alice ==")
    order = dfs(social, "Alice")
    print(f"  discovery order: {order}")

    print("\n== Connected components ==")
    comps = connected_components(social)
    for c in comps:
        print(f"  {sorted(c)}")

    print("\n== Directed cycle detection ==")
    # DAG: build order
    dag = {
        "A": ["B"],
        "B": ["C"],
        "C": [],
        "D": ["C"],
    }
    print(f"  Acyclic DAG → cycle? {has_cycle_directed(dag)}")
    cyclic = dict(dag)
    cyclic["C"] = ["A"]   # introduce back-edge
    print(f"  After adding C → A → cycle? {has_cycle_directed(cyclic)}")

    print("\n== Tree check ==")
    tree = {
        "root": ["a", "b"],
        "a": ["root", "c", "d"],
        "b": ["root"],
        "c": ["a"],
        "d": ["a"],
    }
    print(f"  is_tree(tree) = {is_tree(tree)}    (5 vertices, 4 edges, connected)")

    # Now add an extra edge that creates a cycle
    not_tree = {k: list(v) for k, v in tree.items()}
    not_tree["c"].append("d"); not_tree["d"].append("c")
    print(f"  After adding c-d edge: is_tree = {is_tree(not_tree)}    (cycle introduced)")


if __name__ == "__main__":
    main()
