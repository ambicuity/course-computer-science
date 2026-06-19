"""Sieve of Eratosthenes + trial division + Miller-Rabin + Pollard's rho.

Run:  python3 main.py
"""
from __future__ import annotations

import math
import random
from typing import List, Optional


# ── Sieve ──────────────────────────────────────────────────────────

def sieve(N: int) -> List[int]:
    is_prime = [True] * (N + 1)
    if N >= 0: is_prime[0] = False
    if N >= 1: is_prime[1] = False
    for i in range(2, int(N**0.5) + 1):
        if is_prime[i]:
            for j in range(i * i, N + 1, i):
                is_prime[j] = False
    return [i for i, p in enumerate(is_prime) if p]


# ── Trial division ─────────────────────────────────────────────────

def is_prime_trial(n: int) -> bool:
    if n < 2: return False
    if n % 2 == 0: return n == 2
    d = 3
    while d * d <= n:
        if n % d == 0: return False
        d += 2
    return True


# ── Miller-Rabin ───────────────────────────────────────────────────

# Deterministic witness set sufficient for n < 3.3 · 10^24 (and for all u64).
WITNESSES_DETERMINISTIC = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37]


def miller_rabin(n: int, witnesses: Optional[List[int]] = None) -> bool:
    if n < 2: return False
    if n in (2, 3): return True
    if n % 2 == 0: return False

    # Factor n - 1 as 2^s · d, with d odd
    d = n - 1
    s = 0
    while d % 2 == 0:
        d //= 2
        s += 1

    witnesses = witnesses if witnesses is not None else WITNESSES_DETERMINISTIC
    for a in witnesses:
        if a >= n: continue
        x = pow(a, d, n)
        if x == 1 or x == n - 1:
            continue
        composite = True
        for _ in range(s - 1):
            x = (x * x) % n
            if x == n - 1:
                composite = False
                break
        if composite:
            return False
    return True


# ── Pollard's rho ──────────────────────────────────────────────────

def pollard_rho(n: int) -> Optional[int]:
    """Find a non-trivial factor of n (n > 1, composite). Returns None on failure."""
    if n % 2 == 0: return 2
    while True:
        c = random.randint(1, n - 1)
        f = lambda x: (x * x + c) % n
        x = random.randint(2, n - 1)
        y = x
        d = 1
        while d == 1:
            x = f(x)
            y = f(f(y))
            d = math.gcd(abs(x - y), n)
        if d != n:
            return d


# ── Demo ───────────────────────────────────────────────────────────

def demo_sieve():
    print("== Sieve of Eratosthenes ==")
    for N in [100, 1000, 10000, 100000, 1000000]:
        primes = sieve(N)
        pnt = N / math.log(N)
        print(f"  π({N:>7d}) = {len(primes):>7d}    N/ln(N) ≈ {pnt:>10.1f}    ratio = {len(primes)/pnt:.4f}")


def demo_trial_vs_sieve():
    print("\n== Trial division agrees with the sieve on n ≤ 1000 ==")
    primes_set = set(sieve(1000))
    for n in range(1, 1001):
        assert is_prime_trial(n) == (n in primes_set), n
    print(f"  ✓ 1000 numbers checked")


def demo_miller_rabin():
    print("\n== Miller-Rabin ==")
    # Carmichael number
    n = 561
    print(f"  is_prime_trial(561) = {is_prime_trial(561)}    (composite)")
    print(f"  Fermat's test would say PROBABLY PRIME for every coprime witness (561 is Carmichael)")
    print(f"  miller_rabin(561) = {miller_rabin(561)}   ← correctly identified as composite")
    # Mersenne primes
    for p in [(2**31 - 1, "2³¹−1 (Mersenne M_31)"),
              (2**61 - 1, "2⁶¹−1 (Mersenne M_61)"),
              (2**127 - 1, "2¹²⁷−1 (Mersenne M_127)")]:
        n, label = p
        print(f"  miller_rabin({label}) = {miller_rabin(n)}")

    # Random 64-bit primes
    print("  Random 64-bit primes:")
    random.seed(42)
    for _ in range(3):
        while True:
            cand = random.getrandbits(64) | 1
            if cand > 1 and miller_rabin(cand):
                print(f"    {cand}")
                break


def demo_pollard_rho():
    print("\n== Pollard's rho factoring ==")
    random.seed(7)
    targets = [12, 91, 8051, 10403, 1009 * 1013]
    for n in targets:
        f = pollard_rho(n)
        other = n // f if f else None
        print(f"  pollard_rho({n}) = {f}   (other factor = {other})")
        if f is not None:
            assert f * other == n


def main():
    demo_sieve()
    demo_trial_vs_sieve()
    demo_miller_rabin()
    demo_pollard_rho()


if __name__ == "__main__":
    main()
