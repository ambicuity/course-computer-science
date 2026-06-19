"""poset.py — reusable poset & lattice library.

Used by:
  - any DAG-shaped algorithm in later phases (build systems, dataflow lattices, etc.)
  - Lesson 07 cardinality (chain/antichain decomposition)
"""
from __future__ import annotations

from collections import defaultdict, deque
from typing import Any, Iterable, List, Optional, Set, Tuple


Relation = Set[Tuple[Any, Any]]


# ── Predicates ─────────────────────────────────────────────────────

def is_partial_order(R: Relation, A: Iterable) -> bool:
    A = list(A)
    refl = all((a, a) in R for a in A)
    antisym = not any((a, b) in R and (b, a) in R and a != b for (a, b) in R)
    trans = all((a, c) in R for (a, b1) in R for (b2, c) in R if b1 == b2)
    return refl and antisym and trans


def is_total_order(R: Relation, A: Iterable) -> bool:
    A = list(A)
    if not is_partial_order(R, A):
        return False
    return all((a, b) in R or (b, a) in R for a in A for b in A)


# ── Bounds ─────────────────────────────────────────────────────────

def join(x, y, R: Relation, A: Iterable) -> Optional[Any]:
    ubs = {z for z in A if (x, z) in R and (y, z) in R}
    cands = [z for z in ubs if all((z, u) in R for u in ubs)]
    return cands[0] if len(cands) == 1 else None


def meet(x, y, R: Relation, A: Iterable) -> Optional[Any]:
    lbs = {z for z in A if (z, x) in R and (z, y) in R}
    cands = [z for z in lbs if all((l, z) in R for l in lbs)]
    return cands[0] if len(cands) == 1 else None


def is_lattice(R: Relation, A: Iterable) -> bool:
    A = list(A)
    return all(join(a, b, R, A) is not None and meet(a, b, R, A) is not None
               for a in A for b in A)


# ── Topological sort with cycle detection ─────────────────────────

def topological_sort(edges: Iterable[Tuple[Any, Any]], nodes: Iterable) -> Optional[List]:
    """Kahn's algorithm. `edges` is the edge set (a → b means a precedes b).
    Returns a list giving a linear extension, or None if a cycle exists."""
    nodes = list(nodes)
    indeg = defaultdict(int)
    succ = defaultdict(list)
    for a, b in edges:
        succ[a].append(b)
        indeg[b] += 1
    for n in nodes:
        indeg.setdefault(n, 0)

    queue = deque(sorted([n for n in nodes if indeg[n] == 0],
                         key=lambda x: str(x)))
    out: List = []
    while queue:
        u = queue.popleft()
        out.append(u)
        for v in succ[u]:
            indeg[v] -= 1
            if indeg[v] == 0:
                queue.append(v)
    return out if len(out) == len(nodes) else None


# ── Hasse cover relation ──────────────────────────────────────────

def cover_relation(R: Relation, A: Iterable) -> Set[Tuple[Any, Any]]:
    A = list(A)
    strict = {(a, b) for (a, b) in R if a != b}
    return {(a, b) for (a, b) in strict
            if not any((a, c) in strict and (c, b) in strict for c in A)}


if __name__ == "__main__":
    # Smoke test
    nodes = ['a', 'b', 'c', 'd']
    edges = [('a', 'b'), ('a', 'c'), ('b', 'd'), ('c', 'd')]
    order = topological_sort(edges, nodes)
    assert order in [['a', 'b', 'c', 'd'], ['a', 'c', 'b', 'd']]
    print("poset library smoke-test OK; order =", order)
