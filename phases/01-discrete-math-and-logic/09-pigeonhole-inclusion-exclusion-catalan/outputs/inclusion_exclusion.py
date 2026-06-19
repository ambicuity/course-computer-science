"""inclusion_exclusion.py — general |⋃ Aᵢ| via the alternating-sum formula.

Plus closed forms for two common applications:
  - derangements: !n
  - surjections from [n] to [k]: k! · S(n, k) (Stirling number S₂ via IE)
"""
from __future__ import annotations

from itertools import combinations as it_combs
from math import factorial
from typing import List, Set


def inclusion_exclusion(sets: List[Set]) -> int:
    """Compute |A₁ ∪ A₂ ∪ ... ∪ Aₙ| by the alternating sum over non-empty
    intersections of subsets of indices."""
    n = len(sets)
    if n == 0: return 0
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
    """!n = n! · Σ (-1)ᵏ / k! — permutations with no fixed points."""
    return round(factorial(n) * sum((-1)**k / factorial(k) for k in range(n + 1)))


def num_surjections(n: int, k: int) -> int:
    """Number of surjective functions from [n] → [k] (Stirling numbers, by IE).

    = Σ (-1)ᵢ · C(k, i) · (k - i)ⁿ   for i = 0..k
    """
    from math import comb
    out = 0
    for i in range(k + 1):
        out += (-1)**i * comb(k, i) * (k - i)**n
    return out


if __name__ == "__main__":
    # 1..100, divisible by 2, 3, or 5
    A2 = {i for i in range(1, 101) if i % 2 == 0}
    A3 = {i for i in range(1, 101) if i % 3 == 0}
    A5 = {i for i in range(1, 101) if i % 5 == 0}
    assert inclusion_exclusion([A2, A3, A5]) == 74

    # Derangements
    assert derangements(0) == 1
    assert derangements(1) == 0
    assert derangements(4) == 9
    assert derangements(6) == 265

    # Surjections [3] → [2] = 2³ - 2 = 6
    assert num_surjections(3, 2) == 6
    # Surjections [4] → [2] = 2⁴ - 2 = 14
    assert num_surjections(4, 2) == 14

    print("inclusion_exclusion: smoke-test OK")
