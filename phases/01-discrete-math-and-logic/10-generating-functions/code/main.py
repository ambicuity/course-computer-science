"""Generating functions: truncated power-series algebra + worked examples.

Run:  python3 main.py
"""
from __future__ import annotations

from fractions import Fraction
from math import comb
from typing import List


# ── Coefficient-level operations (lists of Fractions, truncated to N terms) ─

def trim(a: list, n: int) -> list:
    return list(a[:n]) + [Fraction(0)] * max(0, n - len(a))


def add(a: list, b: list, n: int) -> list:
    a, b = trim(a, n), trim(b, n)
    return [a[i] + b[i] for i in range(n)]


def sub(a: list, b: list, n: int) -> list:
    a, b = trim(a, n), trim(b, n)
    return [a[i] - b[i] for i in range(n)]


def mul(a: list, b: list, n: int) -> list:
    out = [Fraction(0)] * n
    for i in range(min(len(a), n)):
        ai = a[i]
        if ai == 0: continue
        for j in range(min(len(b), n - i)):
            out[i + j] += ai * b[j]
    return out


def inv(a: list, n: int) -> list:
    """Power-series reciprocal of `a`, where a[0] must be non-zero. O(n²)."""
    assert a[0] != 0, "a[0] must be non-zero"
    out = [Fraction(0)] * n
    out[0] = Fraction(1) / Fraction(a[0])
    for i in range(1, n):
        # Σⱼ≥₁ a[j] · out[i-j] for j ≤ i and j < len(a)
        s = Fraction(0)
        for j in range(1, min(i, len(a) - 1) + 1):
            s += a[j] * out[i - j]
        out[i] = -s / Fraction(a[0])
    return out


def shift(a: list, k: int, n: int) -> list:
    """x^k · a(x): shift coefficients right by k."""
    return [Fraction(0)] * k + trim(a, n - k)


# ── Standard catalog ───────────────────────────────────────────────

def geometric(n: int) -> List[Fraction]:
    """1/(1-x) = 1 + x + x² + ..."""
    return [Fraction(1)] * n


def fibonacci_ogf(n: int) -> List[Fraction]:
    """x / (1 - x - x²) — coefficients are the Fibonacci numbers."""
    denom = [Fraction(1), Fraction(-1), Fraction(-1)]    # 1 - x - x²
    numer = [Fraction(0), Fraction(1)]                    # x
    return mul(numer, inv(denom, n), n)


def catalan_ogf(n: int) -> List[Fraction]:
    """Solve C(x) = 1 + x · C(x)² by power-series iteration.
    Bootstraps from C(x) = [1, 0, 0, ...]."""
    C = [Fraction(0)] * n
    C[0] = Fraction(1)
    for _ in range(n):    # n iterations suffice (each adds one valid coefficient)
        rhs = mul(C, C, n)
        rhs = shift(rhs, 1, n)
        rhs[0] = Fraction(1) + rhs[0]
        C = rhs
    return C


def stars_and_bars_ogf(k: int, n: int) -> List[Fraction]:
    """(1/(1-x))^k — coefficients are C(n + k - 1, k - 1)."""
    g = geometric(n)
    out = [Fraction(1)] + [Fraction(0)] * (n - 1)
    for _ in range(k):
        out = mul(out, g, n)
    return out


# ── Demo ───────────────────────────────────────────────────────────

def main() -> None:
    N = 15

    print(f"== Geometric series 1/(1-x), first {N} coefficients ==")
    gs = geometric(N)
    print(f"  {[int(c) for c in gs]}")
    # Verify (1 - x) * gs = 1
    prod = mul(gs, [Fraction(1), Fraction(-1)], N)
    print(f"  (1-x) · 1/(1-x) = {[int(c) for c in prod]}  (expected [1,0,0,...])")

    print(f"\n== Fibonacci via x/(1 - x - x²), first {N} coefficients ==")
    fib = fibonacci_ogf(N)
    print(f"  {[int(c) for c in fib]}")
    # Compare with direct recurrence
    direct = [0, 1]
    while len(direct) < N:
        direct.append(direct[-1] + direct[-2])
    print(f"  direct recurrence: {direct[:N]}")
    assert [int(c) for c in fib] == direct[:N]
    print(f"  ✓ generating function matches direct recurrence")

    print(f"\n== Catalan via C(x) = 1 + x C(x)², first {N} coefficients ==")
    cat = catalan_ogf(N)
    print(f"  {[int(c) for c in cat]}")
    # Compare with closed form
    direct_cat = [comb(2*n, n) // (n + 1) for n in range(N)]
    print(f"  closed-form Cₙ:    {direct_cat}")
    assert [int(c) for c in cat] == direct_cat
    print(f"  ✓ generating function matches closed form")

    print(f"\n== Stars-and-bars: (1/(1-x))³ — ways to write n = x₁ + x₂ + x₃ ==")
    sb = stars_and_bars_ogf(3, 10)
    print(f"  [xⁿ] (1/(1-x))³ = {[int(c) for c in sb]}")
    print(f"  C(n+2, 2) for n=0..9 = {[comb(n+2, 2) for n in range(10)]}")
    for n in range(10):
        assert int(sb[n]) == comb(n + 2, 2)
    print(f"  ✓ stars-and-bars matches C(n+k-1, k-1)")

    print(f"\n== 1/(1-x)² (counts n+1; equivalently the 1, 2, 3, 4, ... sequence shifted) ==")
    g2 = mul(geometric(N), geometric(N), N)
    print(f"  {[int(c) for c in g2[:10]]}    (expected [1, 2, 3, 4, ...])")


if __name__ == "__main__":
    main()
