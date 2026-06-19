"""Computational companions to the four proof techniques.

These do NOT prove claims — they verify them on many cases. If a verification
fails, the proof must be wrong (or the claim itself is wrong). If verifications
pass, the proof might still be wrong, but at least the claim is plausible.

Run:  python3 main.py
"""
from __future__ import annotations

import random
from dataclasses import dataclass
from typing import Union


# ── 1. Direct: n even → n² even ─────────────────────────────────────

def check_even_squared():
    for n in range(-100, 101):
        if n % 2 == 0:
            assert (n*n) % 2 == 0, n
    print("✓ n even → n² even (direct):     verified for n ∈ [-100, 100]")


# ── 2. Contrapositive: n² odd → n odd ──────────────────────────────
# Equivalent to: n even → n² even (already verified above). Same content,
# different surface; that's the point of the contrapositive.

def check_n_sq_odd_implies_n_odd():
    for n in range(-100, 101):
        if (n*n) % 2 == 1:
            assert n % 2 != 0, n
    print("✓ n² odd → n odd (contrapositive): verified for n ∈ [-100, 100]")


# ── 3. Contradiction: infinitely many primes (Euclid) ──────────────

def is_prime(x: int) -> bool:
    if x < 2: return False
    return all(x % d for d in range(2, int(x**0.5) + 1))


def check_euclid_witness(primes: list[int]) -> bool:
    """Given a finite list of primes, Euclid's construction produces a number
    that's either prime (and not in the list) or has a prime factor not in
    the list. Verifies the contradiction-step computationally."""
    N = 1
    for p in primes:
        N *= p
    N += 1
    # N is not divisible by any p in `primes`:
    for p in primes:
        assert N % p == 1, (p, N)
    # Either N is prime (new), or it has a prime factor not in `primes`.
    if is_prime(N):
        return True
    for d in range(2, int(N**0.5) + 1):
        if N % d == 0 and is_prime(d) and d not in primes:
            return True
    # Should never reach here under Euclid's argument.
    return False


def check_euclid():
    primes = [2, 3, 5, 7, 11]
    assert check_euclid_witness(primes)
    primes = [2, 3, 5, 7, 11, 13, 17, 19, 23]
    assert check_euclid_witness(primes)
    print("✓ Euclid's construction produces a contradiction witness for two prime lists")


# ── 4. Induction: 1 + 2 + ... + n = n(n+1)/2 ──────────────────────

def check_gauss():
    # Base + many cases of the step.
    assert sum(range(0+1)) == 0*1//2
    for n in range(0, 1000):
        assert sum(range(n+1)) == n*(n+1)//2
    print("✓ Gauss's formula 1+...+n = n(n+1)/2 (induction): verified for n ∈ [0, 999]")


# ── 5. Strong induction preview: every n ≥ 2 has a prime factorization ─

def prime_factor(n: int) -> list[int]:
    out = []
    d = 2
    while d * d <= n:
        while n % d == 0:
            out.append(d)
            n //= d
        d += 1
    if n > 1: out.append(n)
    return out


def check_prime_factorization():
    for n in range(2, 1000):
        factors = prime_factor(n)
        # Each factor is prime, and their product is n
        for f in factors:
            assert is_prime(f), (n, f, factors)
        prod = 1
        for f in factors:
            prod *= f
        assert prod == n, (n, factors)
    print("✓ Prime factorization exists (strong induction): verified for n ∈ [2, 999]")


# ── 6. Structural induction: full binary trees ─────────────────────

@dataclass
class Leaf: pass
@dataclass
class Node:
    l: "Tree"
    r: "Tree"

Tree = Union[Leaf, Node]


def count_nodes(t: Tree) -> int:
    return 1 if isinstance(t, Leaf) else 1 + count_nodes(t.l) + count_nodes(t.r)


def count_leaves(t: Tree) -> int:
    return 1 if isinstance(t, Leaf) else count_leaves(t.l) + count_leaves(t.r)


def random_full_tree(depth: int) -> Tree:
    if depth == 0 or random.random() < 0.25:
        return Leaf()
    return Node(random_full_tree(depth - 1), random_full_tree(depth - 1))


def check_full_binary_tree_invariant():
    random.seed(0)
    for _ in range(1000):
        t = random_full_tree(7)
        n = count_nodes(t)
        leaves = count_leaves(t)
        assert n == 2 * leaves - 1, (n, leaves)
    print("✓ Full-tree invariant n = 2·leaves - 1 (structural induction): 1000 random trees")


def main():
    print("== Computational checks for proof claims ==\n")
    check_even_squared()
    check_n_sq_odd_implies_n_odd()
    check_euclid()
    check_gauss()
    check_prime_factorization()
    check_full_binary_tree_invariant()
    print("\nAll claims verified on the sample. The proofs (in docs/en.md) tell you WHY.")


if __name__ == "__main__":
    main()
