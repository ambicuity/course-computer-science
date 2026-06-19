"""catalan.py — Catalan numbers and the canonical objects they count.

Used downstream by:
  - Phase 04 L08-L10 (DP): matrix-chain multiplication, optimal BSTs.
  - Phase 05 L08-L10 (parsing): counting grammar parse trees.
"""
from __future__ import annotations

from math import comb
from typing import Iterable

# ── Closed form + memoized recurrence ─────────────────────────────

def catalan(n: int) -> int:
    """Cₙ = C(2n, n) / (n + 1)."""
    return comb(2 * n, n) // (n + 1)


_cache = {0: 1}
def catalan_recurrence(n: int) -> int:
    """Cₙ = Σ Cₖ · Cₙ₋₁₋ₖ for k = 0..n-1. Memoized."""
    if n in _cache:
        return _cache[n]
    _cache[n] = sum(catalan_recurrence(k) * catalan_recurrence(n - 1 - k) for k in range(n))
    return _cache[n]


# ── Objects counted by Cₙ ─────────────────────────────────────────

def balanced_parens(n: int) -> Iterable[str]:
    """All Cₙ balanced parenthesis strings with n pairs."""
    if n == 0:
        yield ""; return
    for k in range(n):
        for left in balanced_parens(k):
            for right in balanced_parens(n - 1 - k):
                yield "(" + left + ")" + right


def binary_trees(n: int) -> list:
    """All Cₙ rooted binary trees with n internal nodes (leaves omitted)."""
    if n == 0:
        return [None]
    out = []
    for k in range(n):
        for left in binary_trees(k):
            for right in binary_trees(n - 1 - k):
                out.append((left, right))
    return out


def lattice_paths(n: int) -> list:
    """All Cₙ lattice paths from (0,0) to (n,n) that stay on or below the diagonal.

    Each path is a string of n 'R' (right) and n 'U' (up). Stay-below-diagonal
    means every prefix has at least as many R's as U's."""
    out = []
    def go(s, r, u):
        if r + u == 2 * n:
            out.append(s); return
        if r < n:
            go(s + "R", r + 1, u)
        if u < r:
            go(s + "U", r, u + 1)
    go("", 0, 0)
    return out


if __name__ == "__main__":
    for n in range(8):
        assert catalan(n) == catalan_recurrence(n)
        bp = sum(1 for _ in balanced_parens(n))
        bt = len(binary_trees(n))
        lp = len(lattice_paths(n))
        assert bp == bt == lp == catalan(n), (n, bp, bt, lp, catalan(n))
    print("catalan: four bijection-verified objects for n = 0..7 ✓")
