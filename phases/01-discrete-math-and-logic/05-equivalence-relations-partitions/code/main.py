"""Equivalence relations, classes, partitions, and equivalence closure.

Run:  python3 main.py
"""
from __future__ import annotations

from typing import Any, Iterable, List, Set, Tuple


Relation = Set[Tuple[Any, Any]]


# ── Axiom checks ───────────────────────────────────────────────────

def is_reflexive(R: Relation, A: Iterable) -> bool:
    return all((a, a) in R for a in A)

def is_symmetric(R: Relation) -> bool:
    return all((b, a) in R for (a, b) in R)

def is_transitive(R: Relation) -> bool:
    return all((a, c) in R
               for (a, b1) in R
               for (b2, c) in R if b1 == b2)

def is_equivalence(R: Relation, A: Iterable) -> bool:
    A = list(A)
    return is_reflexive(R, A) and is_symmetric(R) and is_transitive(R)


# ── Classes / partition ────────────────────────────────────────────

def classes(R: Relation, A: Iterable) -> List[Set]:
    """Return the equivalence classes induced by R on A (R must be an equivalence)."""
    rep = {a: a for a in A}

    def find(x):
        while rep[x] != x:
            x = rep[x]
        return x

    for a, b in R:
        ra, rb = find(a), find(b)
        if ra != rb:
            rep[ra] = rb

    buckets: dict = {}
    for a in A:
        buckets.setdefault(find(a), set()).add(a)
    return list(buckets.values())


def from_partition(P: List[Set]) -> Relation:
    R: Relation = set()
    for block in P:
        for a in block:
            for b in block:
                R.add((a, b))
    return R


def is_partition(P: List[Set], A: Iterable) -> bool:
    A = set(A)
    union = set()
    for block in P:
        if not block:
            return False
        if union & block:
            return False
        union |= block
    return union == A


# ── Closures (R, S, T, equivalence) ───────────────────────────────

def reflexive_closure(R: Relation, A: Iterable) -> Relation:
    return R | {(a, a) for a in A}

def symmetric_closure(R: Relation) -> Relation:
    return R | {(b, a) for (a, b) in R}

def transitive_closure(R: Relation) -> Relation:
    out = set(R)
    while True:
        new = out | {(a, c) for (a, b1) in out for (b2, c) in out if b1 == b2}
        if new == out:
            return out
        out = new

def equivalence_closure(R: Relation, A: Iterable) -> Relation:
    A = list(A)
    return transitive_closure(symmetric_closure(reflexive_closure(R, A)))


# ── Demo ───────────────────────────────────────────────────────────

def main() -> None:
    print("== Three-axiom check ==")
    A = {1, 2, 3}
    R_equiv = {(1, 1), (2, 2), (3, 3), (1, 2), (2, 1)}
    print(f"  R = {sorted(R_equiv)}")
    print(f"  reflexive: {is_reflexive(R_equiv, A)}")
    print(f"  symmetric: {is_symmetric(R_equiv)}")
    print(f"  transitive: {is_transitive(R_equiv)}")
    print(f"  equivalence: {is_equivalence(R_equiv, A)}")

    print("\n== Equivalence classes ==")
    cls = classes(R_equiv, A)
    print(f"  classes on {sorted(A)}: {[sorted(c) for c in cls]}")

    print("\n== Partition round-trip ==")
    rebuilt = from_partition(cls)
    print(f"  from_partition(classes(R)) == R ?  {rebuilt == R_equiv}")
    print(f"  is_partition ? {is_partition(cls, A)}")

    print("\n== Equivalence closure of a non-equivalence relation ==")
    R_raw = {(1, 2), (2, 3)}
    print(f"  R = {sorted(R_raw)}  (neither reflexive, symmetric, nor transitive)")
    R_eq = equivalence_closure(R_raw, {1, 2, 3, 4})
    print(f"  closure adds reflexive pairs, symmetric pairs, transitive chains")
    print(f"  classes after closure on {{1,2,3,4}}: {[sorted(c) for c in classes(R_eq, {1,2,3,4})]}")

    print("\n== Modular arithmetic: x ≡ y (mod n) ==")
    for n in (2, 3, 5, 7):
        A = set(range(0, 4 * n))
        R = {(a, b) for a in A for b in A if (a - b) % n == 0}
        cls = classes(R, A)
        print(f"  mod {n}: {len(cls)} classes  (expected {n})  sample: {sorted(min(c) for c in cls)}")


if __name__ == "__main__":
    main()
