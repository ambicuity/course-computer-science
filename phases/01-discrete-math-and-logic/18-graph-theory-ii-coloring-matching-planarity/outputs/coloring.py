"""coloring.py — greedy coloring + bipartiteness test.

Used in Phase 08 (register allocation via interference graph coloring).
"""
from __future__ import annotations

from typing import Dict, List, Optional


def greedy_color(adj: Dict, order: Optional[List] = None) -> Dict:
    order = order or list(adj)
    color: Dict = {}
    for u in order:
        used = {color[v] for v in adj.get(u, []) if v in color}
        c = 0
        while c in used: c += 1
        color[u] = c
    return color


def chromatic_upper_bound(adj: Dict) -> int:
    color = greedy_color(adj)
    return max(color.values()) + 1 if color else 0


def is_bipartite(adj: Dict) -> bool:
    color: Dict = {}
    for start in adj:
        if start in color: continue
        color[start] = 0
        stack = [start]
        while stack:
            u = stack.pop()
            for v in adj.get(u, []):
                if v not in color:
                    color[v] = 1 - color[u]; stack.append(v)
                elif color[v] == color[u]:
                    return False
    return True


if __name__ == "__main__":
    # K_4 needs 4 colors
    k4 = {i: [j for j in range(4) if j != i] for i in range(4)}
    assert chromatic_upper_bound(k4) == 4
    # C_6 is bipartite
    c6 = {i: [(i - 1) % 6, (i + 1) % 6] for i in range(6)}
    assert is_bipartite(c6)
    print("coloring library smoke-test OK")
