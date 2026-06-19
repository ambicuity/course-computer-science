"""Modular arithmetic, Fermat's little theorem, Euler's theorem, a toy RSA.

Run:  python3 main.py
"""
from __future__ import annotations

import math
from typing import Tuple


def modpow(a: int, e: int, n: int) -> int:
    """Binary exponentiation: a^e mod n in O(log e) multiplications."""
    if n == 1:
        return 0
    result = 1
    a %= n
    while e > 0:
        if e & 1:
            result = (result * a) % n
        a = (a * a) % n
        e >>= 1
    return result


def phi(n: int) -> int:
    """Euler's totient: count of integers in [1, n] coprime to n."""
    if n == 1: return 1
    out = n
    p = 2
    m = n
    while p * p <= m:
        if m % p == 0:
            while m % p == 0:
                m //= p
            out -= out // p
        p += 1
    if m > 1:
        out -= out // m
    return out


def is_carmichael(n: int) -> bool:
    """Composite n with a^(n-1) ≡ 1 (mod n) for every a coprime to n."""
    if n < 2: return False
    # Composite check
    for d in range(2, int(n**0.5) + 1):
        if n % d == 0: break
    else:
        return False  # prime
    # Coprime check
    for a in range(2, n):
        if math.gcd(a, n) == 1:
            if pow(a, n - 1, n) != 1:
                return False
    return True


# ── Toy RSA ───────────────────────────────────────────────────────

def extended_gcd(a, b):
    if b == 0: return abs(a), (1 if a >= 0 else -1), 0
    g, x1, y1 = extended_gcd(b, a % b)
    return g, y1, x1 - (a // b) * y1


def modinv(a, m):
    g, x, _ = extended_gcd(a, m)
    if g != 1: raise ValueError(f"no inverse: gcd({a},{m})={g}")
    return x % m


def toy_rsa() -> None:
    p, q = 61, 53          # tiny primes (real RSA uses 2048+ bits)
    n = p * q              # 3233
    phin = (p - 1) * (q - 1)
    e = 17
    d = modinv(e, phin)
    msg = 123
    cipher = pow(msg, e, n)
    decrypted = pow(cipher, d, n)
    print(f"  p={p}, q={q}, n={n}, φ(n)={phin}")
    print(f"  public  e={e}")
    print(f"  private d={d}    (verify: e·d mod φ(n) = {(e * d) % phin})")
    print(f"  encrypt {msg} → {cipher}")
    print(f"  decrypt {cipher} → {decrypted}")
    assert decrypted == msg


# ── Demo ───────────────────────────────────────────────────────────

def main() -> None:
    print("== Modular exponentiation ==")
    print(f"  7^200 mod 13 (direct modpow) = {modpow(7, 200, 13)}")
    print(f"  7^200 mod 13 via Fermat (200 mod 12 = 8): 7^8 mod 13 = {modpow(7, 8, 13)}")
    assert modpow(7, 200, 13) == modpow(7, 8, 13)

    print(f"  2^1000 mod 1000 = {modpow(2, 1000, 1000)}")
    print(f"  Python's pow(2, 1000, 1000) = {pow(2, 1000, 1000)}")

    print("\n== Fermat verification ==")
    for p in [13, 17, 23, 101]:
        for a in [2, 5, 10, 12]:
            assert pow(a, p - 1, p) == 1, (a, p)
    print(f"  ✓ a^(p-1) ≡ 1 (mod p) for primes 13, 17, 23, 101")

    print("\n== Euler's totient ==")
    for n in [1, 7, 12, 36, 100, 561]:
        # Brute-force verification: count coprimes
        brute = sum(1 for k in range(1, n + 1) if math.gcd(k, n) == 1) if n >= 1 else 0
        print(f"  φ({n}) = {phi(n):<6d}  (brute count = {brute})")
        assert phi(n) == brute

    print("\n== Euler's theorem (a^φ(n) ≡ 1 mod n) ==")
    for n in [10, 12, 15, 21, 35]:
        for a in range(1, n):
            if math.gcd(a, n) == 1:
                assert pow(a, phi(n), n) == 1, (a, n)
    print(f"  ✓ verified for n ∈ {{10, 12, 15, 21, 35}} on all coprime a")

    print("\n== Carmichael number 561 ==")
    print(f"  561 = 3 · 11 · 17 (composite)")
    print(f"  is_carmichael(561) = {is_carmichael(561)}    (fools Fermat primality test)")
    # Witnesses where 561 looks 'prime' to Fermat
    sample = [2, 5, 7, 8, 10]
    for a in sample:
        if math.gcd(a, 561) == 1:
            print(f"  Fermat test: {a}^560 mod 561 = {pow(a, 560, 561)}")

    print("\n== Toy RSA ==")
    toy_rsa()


if __name__ == "__main__":
    main()
