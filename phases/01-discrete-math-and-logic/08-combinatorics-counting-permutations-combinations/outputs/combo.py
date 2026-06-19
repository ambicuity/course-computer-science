"""combo.py — combinatorial library used throughout the course.

All counts return Python ints (arbitrary precision). For high performance,
swap in scipy.special.comb or maintain a memoized table.
"""
from __future__ import annotations

import math


def factorial(n: int) -> int:
    if n < 0: raise ValueError("factorial of negative")
    f = 1
    for i in range(2, n + 1): f *= i
    return f


def nPr(n: int, k: int) -> int:
    """Permutations: ordered selections."""
    if k < 0 or k > n: return 0
    out = 1
    for i in range(n - k + 1, n + 1):
        out *= i
    return out


def nCr(n: int, k: int) -> int:
    """Combinations: unordered selections."""
    if k < 0 or k > n: return 0
    if k > n - k: k = n - k
    num, den = 1, 1
    for i in range(k):
        num *= n - i
        den *= i + 1
    return num // den


def multinomial(*ks: int) -> int:
    n = sum(ks)
    out = factorial(n)
    for k in ks:
        out //= factorial(k)
    return out


def stars_and_bars(n: int, k: int) -> int:
    """Number of non-negative integer solutions to x₁ + ... + xₖ = n."""
    return nCr(n + k - 1, k - 1)


def stirling_approx(n: int) -> float:
    """Stirling's approximation to n!."""
    if n == 0: return 1.0
    return math.sqrt(2 * math.pi * n) * (n / math.e) ** n


# Quick alias
binomial = nCr


if __name__ == "__main__":
    assert nCr(52, 5) == 2_598_960
    assert nCr(49, 6) == 13_983_816
    assert multinomial(1, 4, 4, 2) == 34_650
    assert stars_and_bars(10, 4) == 286
    # Stirling's: ~1% accurate by n=10
    err = abs(stirling_approx(10) / factorial(10) - 1)
    assert err < 0.01, err
    print("combo library smoke-test OK")
