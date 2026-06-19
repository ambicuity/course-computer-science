"""relation_ops.py — reusable library for relations and functions.

Used directly by Lesson 05 (equivalence relations) and Phase 04 (graph
algorithms; a graph is just a relation E ⊆ V × V).
"""
from __future__ import annotations

from itertools import product
from typing import Any, Dict, FrozenSet, Iterable, List, Set, Tuple


Relation = Set[Tuple[Any, Any]]


def cartesian(A: Iterable, B: Iterable) -> Set[Tuple]:
    return set(product(A, B))


def powerset(s: Iterable) -> List[FrozenSet]:
    items = list(s)
    out: List[FrozenSet] = []
    for bits in range(1 << len(items)):
        out.append(frozenset(items[i] for i in range(len(items)) if bits & (1 << i)))
    return out


def compose(R: Relation, S: Relation) -> Relation:
    return {(a, c) for (a, b1) in R for (b2, c) in S if b1 == b2}


def inverse(R: Relation) -> Relation:
    return {(b, a) for (a, b) in R}


def identity(A: Iterable) -> Relation:
    return {(a, a) for a in A}


def transitive_closure(R: Relation) -> Relation:
    out = set(R)
    while True:
        new = compose(out, R) | out
        if new == out:
            return out
        out = new


def reflexive_closure(R: Relation, A: Iterable) -> Relation:
    return R | identity(A)


def symmetric_closure(R: Relation) -> Relation:
    return R | inverse(R)


def equivalence_closure(R: Relation, A: Iterable) -> Relation:
    """Smallest equivalence relation on A containing R (reflexive, symmetric, transitive)."""
    return transitive_closure(symmetric_closure(reflexive_closure(R, A)))


# ── Function classifiers ──────────────────────────────────────────

def is_function(R: Relation, domain: Iterable) -> bool:
    domain_set = set(domain)
    by_input: Dict[Any, Any] = {}
    for a, b in R:
        if a not in domain_set:
            return False
        if a in by_input and by_input[a] != b:
            return False
        by_input[a] = b
    return set(by_input.keys()) == domain_set


def is_injective(f: Relation) -> bool:
    image_with_inputs = {}
    for a, b in f:
        if b in image_with_inputs and image_with_inputs[b] != a:
            return False
        image_with_inputs[b] = a
    return True


def is_surjective(f: Relation, codomain: Iterable) -> bool:
    return {b for _, b in f} == set(codomain)


def is_bijective(f: Relation, codomain: Iterable) -> bool:
    return is_injective(f) and is_surjective(f, codomain)


if __name__ == "__main__":
    # Smoke test
    f = {('a', 1), ('b', 2), ('c', 3)}
    assert is_bijective(f, {1, 2, 3})
    assert inverse(inverse(f)) == f
    print("relation_ops smoke-test OK")
