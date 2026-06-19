"""Combinatorics: counts, Pascal, multinomial, stars-and-bars.

Run:  python3 main.py
"""
from __future__ import annotations

from itertools import combinations as it_combs


# ── Basic ─────────────────────────────────────────────────────────

def factorial(n: int) -> int:
    f = 1
    for i in range(2, n + 1):
        f *= i
    return f


def permutations(n: int, k: int) -> int:
    if k < 0 or k > n:
        return 0
    p = 1
    for i in range(n - k + 1, n + 1):
        p *= i
    return p


def combinations(n: int, k: int) -> int:
    if k < 0 or k > n:
        return 0
    if k > n - k:
        k = n - k
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
    """Non-negative integer solutions to x₁ + ... + xₖ = n."""
    return combinations(n + k - 1, k - 1)


# ── Pascal's triangle ──────────────────────────────────────────────

def pascal(rows: int) -> list[list[int]]:
    out = [[1]]
    for _ in range(rows - 1):
        prev = out[-1]
        new = [1] + [prev[i] + prev[i + 1] for i in range(len(prev) - 1)] + [1]
        out.append(new)
    return out


# ── Demo ───────────────────────────────────────────────────────────

def main() -> None:
    print("== Factorials and permutations ==")
    print(f"  10! = {factorial(10):,}")
    print(f"  P(10, 3) = {permutations(10, 3)}    (president, VP, treasurer)")

    print("\n== Combinations ==")
    print(f"  C(10, 3) = {combinations(10, 3)}    (committee of 3 from 10)")
    print(f"  C(52, 5) = {combinations(52, 5):,}    (5-card poker hands)")
    print(f"  C(49, 6) = {combinations(49, 6):,}    (lottery)")

    print("\n== Pascal's triangle (rows 0..7) ==")
    for row in pascal(8):
        cells = " ".join(f"{c:4d}" for c in row)
        print(f"  {cells.center(8 * 5)}")

    print("\n== Pascal's identity: C(n,k) = C(n-1,k-1) + C(n-1,k) ==")
    for n in range(1, 8):
        for k in range(1, n):
            assert combinations(n, k) == combinations(n-1, k-1) + combinations(n-1, k)
    print("  ✓ verified for n ∈ [1, 7]")

    print("\n== Sum of row n = 2ⁿ ==")
    for n in range(0, 10):
        assert sum(combinations(n, k) for k in range(n + 1)) == 2**n
    print("  ✓ verified for n ∈ [0, 9]")

    print("\n== Multinomial: arrangements of MISSISSIPPI (M=1, I=4, S=4, P=2) ==")
    print(f"  count = {multinomial(1, 4, 4, 2):,}    (expected 34,650)")

    print("\n== Stars and bars: distribute 10 candies among 4 kids ==")
    print(f"  C(10 + 4 - 1, 4 - 1) = C(13, 3) = {stars_and_bars(10, 4)}    (expected 286)")
    print(f"  At least 1 each: substitute → C(9, 3) = {stars_and_bars(10 - 4, 4)}    (expected 84)")

    print("\n== Brute-force verification ==")
    for n, k in [(5, 2), (6, 3), (7, 4), (8, 5)]:
        brute = sum(1 for _ in it_combs(range(n), k))
        assert brute == combinations(n, k), (n, k, brute, combinations(n, k))
    print("  ✓ itertools.combinations matches combinations(n, k) for small cases")


if __name__ == "__main__":
    main()
