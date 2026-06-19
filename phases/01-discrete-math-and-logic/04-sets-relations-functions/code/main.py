"""Sets, relations, functions — concrete operations and classifiers.

Run:  python3 main.py
"""
from __future__ import annotations

from itertools import product
from typing import FrozenSet, Iterable, List, Set, Tuple, Any


# ── Set operations (Python `set` already does the basics) ──────────

def powerset(s: Iterable) -> List[FrozenSet]:
    """All 2^n subsets of s."""
    items = list(s)
    out: List[FrozenSet] = []
    n = len(items)
    for bits in range(1 << n):
        subset = frozenset(items[i] for i in range(n) if bits & (1 << i))
        out.append(subset)
    return out


def cartesian(A: Iterable, B: Iterable) -> Set[Tuple]:
    return set(product(A, B))


# ── Relations as sets of pairs ─────────────────────────────────────

Relation = Set[Tuple[Any, Any]]


def compose(R: Relation, S: Relation) -> Relation:
    """(S ∘ R)(a, c) iff ∃b. (a, b) ∈ R and (b, c) ∈ S."""
    return {(a, c) for (a, b1) in R for (b2, c) in S if b1 == b2}


def inverse(R: Relation) -> Relation:
    return {(b, a) for (a, b) in R}


def transitive_closure(R: Relation) -> Relation:
    """Repeatedly compose with itself until stable. O((n+|R|)|R|) for small sets."""
    out = set(R)
    while True:
        new = compose(out, R)
        added = new - out
        if not added:
            return out
        out |= new


# ── Functions: classification ──────────────────────────────────────

def is_function(R: Relation, domain: Iterable) -> bool:
    """f total and single-valued over the given domain."""
    domain = set(domain)
    by_input: dict = {}
    for a, b in R:
        if a not in domain:
            return False  # input outside declared domain
        if a in by_input and by_input[a] != b:
            return False  # multi-valued
        by_input[a] = b
    return set(by_input.keys()) == domain


def classify(f: Relation, codomain: Iterable) -> dict:
    image = {b for _, b in f}
    inputs = [a for a, _ in f]
    injective  = len(image) == len(set(inputs))      # every input maps to a unique output
    surjective = image == set(codomain)
    return {
        "injective": injective,
        "surjective": surjective,
        "bijective": injective and surjective,
    }


# ── Demo ───────────────────────────────────────────────────────────

def main() -> None:
    print("== Set ops ==")
    A, B = {1, 2, 3, 4}, {3, 4, 5, 6}
    print(f"  A ∪ B = {A | B}")
    print(f"  A ∩ B = {A & B}")
    print(f"  A − B = {A - B}")
    print(f"  A △ B = {A ^ B}")
    print(f"  |A × B| = {len(cartesian(A, B))}   (expected {len(A)*len(B)})")
    P = powerset({1, 2, 3})
    print(f"  𝒫({{1,2,3}}) has {len(P)} subsets (expected {2**3})")

    print("\n== Relations ==")
    R = {(1, 'a'), (1, 'b'), (2, 'c')}
    print(f"  R = {R}")
    print(f"  is_function(R, {{1,2}})? {is_function(R, {1, 2})}  (no — multi-valued)")

    # Compose two relations: R: int → letter, S: letter → uppercase
    R = {(1, 'a'), (2, 'b'), (3, 'c')}
    S = {('a', 'A'), ('b', 'B'), ('c', 'C')}
    print(f"  R ∘ S (sql JOIN on b)  = {compose(R, S)}")

    print("\n== Functions ==")
    f_double = {(x, 2*x) for x in range(-3, 4)}
    print(f"  f(x) = 2x on [-3, 3]:  classify = {classify(f_double, {2*x for x in range(-3, 4)})}")

    f_square = {(x, x*x) for x in range(-3, 4)}
    image = {x*x for x in range(-3, 4)}
    print(f"  f(x) = x² on [-3, 3]:  classify = {classify(f_square, image)}")
    print(f"    (image as codomain → surjective; but NOT injective because f(-1) = f(1))")

    print("\n== Inverse of a bijection ==")
    f_bij = {('a', 1), ('b', 2), ('c', 3)}
    inv = inverse(f_bij)
    print(f"  f = {sorted(f_bij)}")
    print(f"  f⁻¹ = {sorted(inv)}")
    print(f"  (f⁻¹)⁻¹ == f ? {inverse(inv) == f_bij}")

    print("\n== Transitive closure (path-finding via composition) ==")
    edges = {(1, 2), (2, 3), (3, 4), (5, 6)}
    closure = transitive_closure(edges)
    print(f"  edges  = {sorted(edges)}")
    print(f"  R+     = {sorted(closure)}")


if __name__ == "__main__":
    main()
