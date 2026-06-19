"""Pigeonhole, inclusion-exclusion, Catalan numbers.

Run:  python3 main.py
"""
from __future__ import annotations

from itertools import combinations as it_combs
from math import comb, factorial
from typing import Iterable, List, Optional, Set, Tuple


# ── Pigeonhole ─────────────────────────────────────────────────────

def pigeonhole_witness(items: list, n_boxes: int) -> Optional[Tuple]:
    """Drop items into n_boxes buckets via a simple hash. Return a colliding pair if any."""
    box: dict[int, list] = {}
    for it in items:
        b = hash(it) % n_boxes
        box.setdefault(b, []).append(it)
    for vals in box.values():
        if len(vals) >= 2:
            return (vals[0], vals[1])
    return None


# ── Inclusion-exclusion ───────────────────────────────────────────

def inclusion_exclusion(sets: List[Set]) -> int:
    n = len(sets)
    total = 0
    for size in range(1, n + 1):
        sign = 1 if size % 2 == 1 else -1
        for combo in it_combs(range(n), size):
            inter = set(sets[combo[0]])
            for i in combo[1:]:
                inter &= sets[i]
            total += sign * len(inter)
    return total


def derangements(n: int) -> int:
    """Permutations of n elements with no fixed points (= !n)."""
    return round(factorial(n) * sum((-1)**k / factorial(k) for k in range(n + 1)))


# ── Catalan numbers ────────────────────────────────────────────────

def catalan_closed(n: int) -> int:
    return comb(2 * n, n) // (n + 1)


_cache = {0: 1}
def catalan_rec(n: int) -> int:
    if n in _cache:
        return _cache[n]
    _cache[n] = sum(catalan_rec(k) * catalan_rec(n - 1 - k) for k in range(n))
    return _cache[n]


def balanced_parens(n: int) -> Iterable[str]:
    if n == 0:
        yield ""; return
    for k in range(n):
        for left in balanced_parens(k):
            for right in balanced_parens(n - 1 - k):
                yield "(" + left + ")" + right


# ── Demos ──────────────────────────────────────────────────────────

def demo_pigeonhole():
    print("== Pigeonhole ==")
    items = list(range(13))
    pair = pigeonhole_witness(items, 12)
    print(f"  13 items into 12 boxes → collision found: {pair is not None}; example: {pair}")

    print(f"  13 birthdays in 12 months → ≥2 share: True (pigeonhole)")

    print(f"  In [1..100], any 51-element subset contains two numbers that are coprime")
    # (Classical result: from 1..2n choose n+1 items; two must be coprime.)
    print(f"  Generalized: 100 items in 30 boxes → some box has ≥ ceil(100/30) = 4 items")


def demo_inclusion_exclusion():
    print("\n== Inclusion-Exclusion: |div 2 ∪ div 3 ∪ div 5| in [1..100] ==")
    A2 = {i for i in range(1, 101) if i % 2 == 0}
    A3 = {i for i in range(1, 101) if i % 3 == 0}
    A5 = {i for i in range(1, 101) if i % 5 == 0}
    print(f"  |A₂|, |A₃|, |A₅| = {len(A2)}, {len(A3)}, {len(A5)}")
    print(f"  IE total = {inclusion_exclusion([A2, A3, A5])} (expected 74)")
    print(f"  brute force = {len(A2 | A3 | A5)}")

    print("\n  Derangements via IE: !n for n = 0..6:")
    for n in range(7):
        print(f"    !{n} = {derangements(n)}")


def demo_catalan():
    print("\n== Catalan numbers ==")
    print(f"  Closed form C(2n,n)/(n+1):")
    for n in range(10):
        print(f"    C_{n} = {catalan_closed(n)}")

    print(f"  Recurrence verifies they agree:")
    for n in range(11):
        assert catalan_closed(n) == catalan_rec(n), n
    print(f"  ✓ closed-form == recurrence for n ∈ [0, 10]")

    print(f"\n  Balanced-paren enumeration agrees with Cₙ:")
    for n in range(7):
        count = sum(1 for _ in balanced_parens(n))
        assert count == catalan_closed(n), (n, count)
        print(f"    n={n}: {count} strings  (some: {list(balanced_parens(n))[:3]}{'...' if count > 3 else ''})")


def main():
    demo_pigeonhole()
    demo_inclusion_exclusion()
    demo_catalan()


if __name__ == "__main__":
    main()
