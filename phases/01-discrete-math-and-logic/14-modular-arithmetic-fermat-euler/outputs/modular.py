"""modular.py — modular-arithmetic toolkit used in Lessons 15, 17 and Phase 12.

Includes modpow (binary exponentiation), Euler's totient φ(n), a Fermat-witness
checker, and a toy RSA helper.
"""
from __future__ import annotations

import math
from typing import Tuple


def modpow(a: int, e: int, n: int) -> int:
    if n == 1: return 0
    result, a = 1, a % n
    while e > 0:
        if e & 1:
            result = (result * a) % n
        a = (a * a) % n
        e >>= 1
    return result


def phi(n: int) -> int:
    if n == 1: return 1
    out, m = n, n
    p = 2
    while p * p <= m:
        if m % p == 0:
            while m % p == 0:
                m //= p
            out -= out // p
        p += 1
    if m > 1:
        out -= out // m
    return out


def totient_table(n: int) -> list:
    """Sieve-style totient computation for all i in [0, n]. O(n log log n)."""
    t = list(range(n + 1))
    for p in range(2, n + 1):
        if t[p] == p:                # p is prime
            for k in range(p, n + 1, p):
                t[k] -= t[k] // p
    return t


def is_fermat_witness(a: int, n: int) -> bool:
    """True if pow(a, n-1, n) != 1 — a is a 'witness' that n is composite."""
    if math.gcd(a, n) != 1:
        return True
    return pow(a, n - 1, n) != 1


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


def toy_rsa(p: int = 61, q: int = 53, e: int = 17) -> Tuple[int, int, int, int]:
    """Returns (n, e, d, φ(n)) for the chosen toy parameters."""
    n = p * q
    phin = (p - 1) * (q - 1)
    d = modinv(e, phin)
    return n, e, d, phin


if __name__ == "__main__":
    # Fermat
    assert pow(7, 12, 13) == 1
    # Totient
    assert phi(36) == 12
    # Totient sieve
    table = totient_table(20)
    assert table[1] == 1 and table[12] == 4 and table[20] == 8
    # RSA round-trip
    n, e, d, _ = toy_rsa()
    msg = 999
    c = pow(msg, e, n)
    m = pow(c, d, n)
    assert m == msg
    print("modular library smoke-test OK")
