"""primes.py — sieve + Miller-Rabin + Pollard's rho + next_prime.

Used by Phase 04 (algorithms with prime moduli), Phase 12 (RSA / ECC key gen).
"""
from __future__ import annotations

import math
import random
from typing import List, Optional


def sieve(N: int) -> List[int]:
    if N < 2:
        return []
    is_prime = bytearray(b"\x01") * (N + 1)
    is_prime[0] = is_prime[1] = 0
    for i in range(2, int(N**0.5) + 1):
        if is_prime[i]:
            is_prime[i*i::i] = bytearray(len(is_prime[i*i::i]))
    return [i for i in range(N + 1) if is_prime[i]]


_DETERMINISTIC_WITNESSES = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37]


def is_prime(n: int, witnesses: Optional[List[int]] = None) -> bool:
    """Miller-Rabin. Deterministic for n < 3.3·10^24 (covers u64 + more)."""
    if n < 2: return False
    if n in (2, 3): return True
    if n % 2 == 0: return False
    d, s = n - 1, 0
    while d % 2 == 0:
        d //= 2; s += 1
    ws = witnesses or _DETERMINISTIC_WITNESSES
    for a in ws:
        if a >= n: continue
        x = pow(a, d, n)
        if x == 1 or x == n - 1: continue
        ok = False
        for _ in range(s - 1):
            x = (x * x) % n
            if x == n - 1: ok = True; break
        if not ok: return False
    return True


def next_prime(n: int) -> int:
    """Smallest prime > n."""
    if n < 2: return 2
    n = n + 1 if n % 2 == 0 else n + 2
    while not is_prime(n):
        n += 2
    return n


def pollard_rho(n: int) -> Optional[int]:
    if n % 2 == 0:
        return 2
    while True:
        c = random.randint(1, n - 1)
        f = lambda x: (x * x + c) % n
        x = random.randint(2, n - 1)
        y, d = x, 1
        while d == 1:
            x = f(x)
            y = f(f(y))
            d = math.gcd(abs(x - y), n)
        if d != n:
            return d


if __name__ == "__main__":
    assert len(sieve(100)) == 25
    assert is_prime(2**61 - 1)
    assert not is_prime(561)
    assert next_prime(100) == 101
    assert pollard_rho(8051) in (83, 97)
    print("primes library smoke-test OK")
