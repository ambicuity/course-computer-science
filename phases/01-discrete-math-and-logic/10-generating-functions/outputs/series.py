"""series.py — truncated power-series library.

Coefficients are Fractions for exact arithmetic; truncation length `n` is
specified at every operation so the math stays finite.
"""
from __future__ import annotations

from fractions import Fraction
from math import comb
from typing import List


# ── Core operations ───────────────────────────────────────────────

def trim(a, n: int) -> List[Fraction]:
    out = [Fraction(x) for x in a[:n]]
    out += [Fraction(0)] * max(0, n - len(out))
    return out


def add(a, b, n: int) -> List[Fraction]:
    a, b = trim(a, n), trim(b, n)
    return [a[i] + b[i] for i in range(n)]


def sub(a, b, n: int) -> List[Fraction]:
    a, b = trim(a, n), trim(b, n)
    return [a[i] - b[i] for i in range(n)]


def mul(a, b, n: int) -> List[Fraction]:
    out = [Fraction(0)] * n
    for i in range(min(len(a), n)):
        ai = Fraction(a[i])
        if ai == 0: continue
        for j in range(min(len(b), n - i)):
            out[i + j] += ai * Fraction(b[j])
    return out


def inv(a, n: int) -> List[Fraction]:
    a0 = Fraction(a[0])
    assert a0 != 0, "a[0] must be non-zero"
    out = [Fraction(0)] * n
    out[0] = Fraction(1) / a0
    for i in range(1, n):
        s = Fraction(0)
        for j in range(1, min(i, len(a) - 1) + 1):
            s += Fraction(a[j]) * out[i - j]
        out[i] = -s / a0
    return out


def shift(a, k: int, n: int) -> List[Fraction]:
    return [Fraction(0)] * k + trim(a, n - k)


# ── Standard catalog ──────────────────────────────────────────────

def geometric(n: int) -> List[Fraction]:
    return [Fraction(1)] * n


def fibonacci_ogf(n: int) -> List[Fraction]:
    return mul([Fraction(0), Fraction(1)], inv([Fraction(1), Fraction(-1), Fraction(-1)], n), n)


def catalan_ogf(n: int) -> List[Fraction]:
    C = [Fraction(0)] * n
    C[0] = Fraction(1)
    for _ in range(n):
        rhs = mul(C, C, n)
        rhs = shift(rhs, 1, n)
        rhs[0] = Fraction(1) + rhs[0]
        C = rhs
    return C


def coefficients_int(s) -> List[int]:
    return [int(c) for c in s]


if __name__ == "__main__":
    assert coefficients_int(fibonacci_ogf(12)) == [0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89]
    assert coefficients_int(catalan_ogf(10)) == [1, 1, 2, 5, 14, 42, 132, 429, 1430, 4862]
    print("series library smoke-test OK")
