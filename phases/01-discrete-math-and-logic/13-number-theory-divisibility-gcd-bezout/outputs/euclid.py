"""euclid.py — gcd / lcm / extended-gcd / modinv / Diophantine.

Reused in Lesson 14 (modular arithmetic), Phase 12 (cryptography).
"""
from __future__ import annotations

from typing import Optional, Tuple


def gcd(a: int, b: int) -> int:
    a, b = abs(a), abs(b)
    while b:
        a, b = b, a % b
    return a


def lcm(a: int, b: int) -> int:
    if a == 0 or b == 0: return 0
    return abs(a // gcd(a, b) * b)


def extended_gcd(a: int, b: int) -> Tuple[int, int, int]:
    if b == 0:
        return abs(a), (1 if a >= 0 else -1), 0
    g, x1, y1 = extended_gcd(b, a % b)
    return g, y1, x1 - (a // b) * y1


def modinv(a: int, m: int) -> int:
    g, x, _ = extended_gcd(a, m)
    if g != 1:
        raise ValueError(f"no inverse: gcd({a},{m})={g}")
    return x % m


def diophantine(a: int, b: int, c: int) -> Optional[Tuple[int, int, int, int]]:
    g, x0, y0 = extended_gcd(a, b)
    if c % g != 0:
        return None
    x0 *= c // g
    y0 *= c // g
    return x0, y0, b // g, -a // g


if __name__ == "__main__":
    assert gcd(462, 1071) == 21
    g, x, y = extended_gcd(11, 13)
    assert g == 1 and 11 * x + 13 * y == 1
    assert (3 * modinv(3, 7)) % 7 == 1
    print("euclid library smoke-test OK")
