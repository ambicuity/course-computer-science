"""Partial orders, Hasse diagrams, lattices, and topological sort.

Run:  python3 main.py
"""
from __future__ import annotations

import math
from collections import defaultdict, deque
from typing import Any, Iterable, List, Optional, Set, Tuple


Relation = Set[Tuple[Any, Any]]


# ── Axiom checks ───────────────────────────────────────────────────

def is_reflexive(R: Relation, A: Iterable) -> bool:
    return all((a, a) in R for a in A)

def is_antisymmetric(R: Relation) -> bool:
    return not any(((a, b) in R and (b, a) in R and a != b)
                   for (a, b) in R)

def is_transitive(R: Relation) -> bool:
    return all((a, c) in R
               for (a, b1) in R
               for (b2, c) in R if b1 == b2)

def is_partial_order(R: Relation, A: Iterable) -> bool:
    A = list(A)
    return is_reflexive(R, A) and is_antisymmetric(R) and is_transitive(R)

def is_total_order(R: Relation, A: Iterable) -> bool:
    A = list(A)
    if not is_partial_order(R, A):
        return False
    return all((a, b) in R or (b, a) in R for a in A for b in A)


# ── Hasse / covers ─────────────────────────────────────────────────

def covers(R: Relation, A: Iterable) -> Set[Tuple[Any, Any]]:
    """Return the cover relation: (a, b) is a cover iff a < b and no c with a < c < b."""
    A = list(A)
    strict = {(a, b) for (a, b) in R if a != b}
    out = set()
    for (a, b) in strict:
        if not any((a, c) in strict and (c, b) in strict for c in A):
            out.add((a, b))
    return out


# ── Bounds / lattice ───────────────────────────────────────────────

def upper_bounds(x, y, R: Relation, A: Iterable) -> Set:
    return {z for z in A if (x, z) in R and (y, z) in R}

def lower_bounds(x, y, R: Relation, A: Iterable) -> Set:
    return {z for z in A if (z, x) in R and (z, y) in R}

def join(x, y, R: Relation, A: Iterable) -> Optional[Any]:
    """Least upper bound. Returns None if it doesn't exist or isn't unique."""
    ubs = upper_bounds(x, y, R, A)
    # candidate = upper bound that is ≤ every other upper bound
    candidates = [z for z in ubs if all((z, u) in R for u in ubs)]
    return candidates[0] if len(candidates) == 1 else None

def meet(x, y, R: Relation, A: Iterable) -> Optional[Any]:
    lbs = lower_bounds(x, y, R, A)
    candidates = [z for z in lbs if all((l, z) in R for l in lbs)]
    return candidates[0] if len(candidates) == 1 else None

def is_lattice(R: Relation, A: Iterable) -> bool:
    A = list(A)
    return all(join(a, b, R, A) is not None and meet(a, b, R, A) is not None
               for a in A for b in A)


# ── Topological sort (Kahn) ───────────────────────────────────────

def topological_sort(covers_rel: Set[Tuple[Any, Any]], nodes: Iterable) -> Optional[List]:
    """Return a linear extension of the poset (Kahn's algorithm).
    Input is the cover relation (a, b) ⇒ a precedes b.
    Returns None if a cycle is detected."""
    nodes = list(nodes)
    indeg = defaultdict(int)
    succ = defaultdict(list)
    for a, b in covers_rel:
        succ[a].append(b)
        indeg[b] += 1
    for n in nodes:
        indeg.setdefault(n, 0)

    queue = deque(sorted(n for n in nodes if indeg[n] == 0))
    out: List = []
    while queue:
        u = queue.popleft()
        out.append(u)
        for v in succ[u]:
            indeg[v] -= 1
            if indeg[v] == 0:
                queue.append(v)
    return out if len(out) == len(nodes) else None


# ── Demo ───────────────────────────────────────────────────────────

def main() -> None:
    print("== Subset poset 𝒫({a,b,c}) under ⊆ ==")
    sets = [frozenset(s) for s in
            [(), ('a',), ('b',), ('c',),
             ('a', 'b'), ('a', 'c'), ('b', 'c'),
             ('a', 'b', 'c')]]
    R = {(s, t) for s in sets for t in sets if s <= t}
    print(f"  is_partial_order:  {is_partial_order(R, sets)}")
    print(f"  is_total_order:    {is_total_order(R, sets)}  (no — {{a}} and {{b}} incomparable)")
    print(f"  is_lattice:        {is_lattice(R, sets)}      (yes — join=∪, meet=∩)")

    a, b = frozenset("a"), frozenset("b")
    print(f"  join({{a}}, {{b}})  = {set(join(a, b, R, sets))}")
    print(f"  meet({{a}}, {{b}})  = {set(meet(a, b, R, sets))}")

    print("\n== Divisibility on {1..12} ==")
    A = list(range(1, 13))
    R = {(a, b) for a in A for b in A if b % a == 0}
    print(f"  is_partial_order:  {is_partial_order(R, A)}")
    print(f"  is_lattice:        {is_lattice(R, A)}     (no — restricted finite range; lcm(7,11)=77 ∉ A)")

    print(f"  join(2, 3) = {join(2, 3, R, A)}   (lcm = 6)")
    print(f"  meet(6, 8) = {meet(6, 8, R, A)}   (gcd = 2)")

    print("\n== Topological sort of a small DAG ==")
    nodes = ['c.h', 'c.o', 'util.h', 'util.o', 'main.c', 'main.o', 'app']
    edges = [
        ('c.h',   'c.o'),
        ('c.h',   'main.o'),
        ('util.h','util.o'),
        ('util.h','main.o'),
        ('main.c','main.o'),
        ('c.o',   'app'),
        ('util.o','app'),
        ('main.o','app'),
    ]
    order = topological_sort(set(edges), nodes)
    print(f"  build order: {order}")

    print("\n== Hasse covers for divisibility on {1..6} ==")
    A6 = list(range(1, 7))
    R6 = {(a, b) for a in A6 for b in A6 if b % a == 0}
    covers6 = sorted(covers(R6, A6))
    print(f"  covers: {covers6}")


if __name__ == "__main__":
    main()
