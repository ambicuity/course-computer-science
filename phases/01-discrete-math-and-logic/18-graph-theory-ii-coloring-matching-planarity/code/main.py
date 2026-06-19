"""Graph coloring, bipartiteness, bipartite matching, Euler's formula check.

Run:  python3 main.py
"""
from __future__ import annotations

from typing import Dict, List, Optional


# ── Greedy coloring ───────────────────────────────────────────────

def greedy_color(adj: Dict, order: Optional[List] = None) -> Dict:
    order = order or list(adj)
    color: Dict = {}
    for u in order:
        used = {color[v] for v in adj.get(u, []) if v in color}
        c = 0
        while c in used:
            c += 1
        color[u] = c
    return color


def chromatic_greedy(adj: Dict) -> int:
    color = greedy_color(adj)
    return max(color.values()) + 1 if color else 0


# ── Bipartiteness ─────────────────────────────────────────────────

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
                    color[v] = 1 - color[u]
                    stack.append(v)
                elif color[v] == color[u]:
                    return False
    return True


# ── Bipartite max matching (Kuhn's algorithm) ─────────────────────

def bipartite_matching(adj: Dict, left: List) -> Dict:
    """adj[u] (for u in left) is u's right-neighbors. Returns right→left matching."""
    match: Dict = {}

    def try_kuhn(u, seen) -> bool:
        for v in adj.get(u, []):
            if v in seen: continue
            seen.add(v)
            if v not in match or try_kuhn(match[v], seen):
                match[v] = u
                return True
        return False

    for u in left:
        try_kuhn(u, set())
    return match


# ── Demo ──────────────────────────────────────────────────────────

def make_cycle(n: int) -> Dict:
    g: Dict = {i: [] for i in range(n)}
    for i in range(n):
        g[i].append((i + 1) % n)
        g[(i + 1) % n].append(i)
    return g


def make_complete(n: int) -> Dict:
    g: Dict = {i: [] for i in range(n)}
    for i in range(n):
        for j in range(n):
            if i != j: g[i].append(j)
    return g


def make_cube() -> Dict:
    """Cube as a graph: 8 vertices (bit triples), edges for one-bit-flips."""
    g: Dict = {i: [] for i in range(8)}
    for i in range(8):
        for b in range(3):
            j = i ^ (1 << b)
            g[i].append(j)
    return g


def main() -> None:
    print("== Greedy coloring ==")
    for name, g, expected in [
        ("K_4 (complete on 4)",  make_complete(4), 4),
        ("C_5 (odd cycle)",      make_cycle(5),    3),
        ("C_6 (even cycle)",     make_cycle(6),    2),
        ("Cube (Q_3)",           make_cube(),      2),
    ]:
        chi = chromatic_greedy(g)
        print(f"  {name:25s}  greedy uses {chi} colors  (chromatic = {expected})")

    print("\n== Bipartiteness ==")
    for name, g, expected in [
        ("C_5", make_cycle(5),  False),
        ("C_6", make_cycle(6),  True),
        ("K_4", make_complete(4), False),
        ("Cube", make_cube(),    True),
    ]:
        r = is_bipartite(g)
        print(f"  {name:6s}  is_bipartite = {r}    (expected {expected})")

    print("\n== Bipartite matching (interns → projects) ==")
    interns = ["Alice", "Bob", "Carol", "Dan"]
    prefs = {
        "Alice":  ["P1", "P2"],
        "Bob":    ["P1"],
        "Carol":  ["P2", "P3"],
        "Dan":    ["P3", "P4"],
    }
    match = bipartite_matching(prefs, interns)
    print("  match (project → intern):")
    for p, i in sorted(match.items()):
        print(f"    {p} → {i}")
    print(f"  size = {len(match)}    (expected 4 — full match)")

    print("\n== Euler's formula on the cube graph ==")
    cube = make_cube()
    V = len(cube)
    E = sum(len(v) for v in cube.values()) // 2
    F = 6   # 5 square faces + 1 outer
    print(f"  V = {V}, E = {E}, F = {F}    V - E + F = {V - E + F}    (expected 2)")
    assert V - E + F == 2

    print("\n== K_5 is non-planar — counting argument ==")
    V, E = 5, 10
    F_needed = 2 + E - V
    print(f"  K_5: V = {V}, E = {E}, so for a planar drawing F = 2 + E - V = {F_needed}")
    print(f"  Each face borders ≥ 3 edges; each edge borders 2 faces. So 2E ≥ 3F.")
    print(f"  Plug in: 2·{E} = 20 ≥ 3·{F_needed} = {3*F_needed}  →  20 ≥ 21 FALSE  ⇒ K_5 non-planar.")


if __name__ == "__main__":
    main()
