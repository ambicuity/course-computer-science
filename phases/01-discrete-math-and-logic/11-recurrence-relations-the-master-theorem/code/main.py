"""Recurrence relations + Master theorem.

Run:  python3 main.py
"""
from __future__ import annotations

import math
from typing import List, Tuple


# ── Linear homogeneous recurrence ─────────────────────────────────

def solve_linear(coeffs: List[int], initial: List[int], n: int) -> int:
    """Compute aₙ for aₙ = c₁·aₙ₋₁ + c₂·aₙ₋₂ + ... + cₖ·aₙ₋ₖ by iteration."""
    a = list(initial)
    k = len(coeffs)
    assert len(initial) == k, "need k initial conditions for a k-th order recurrence"
    while len(a) <= n:
        a.append(sum(coeffs[i] * a[-1 - i] for i in range(k)))
    return a[n]


# ── Master theorem classifier ─────────────────────────────────────

def master(a: int, b: int, f_degree: float, f_log_factor: int = 0) -> Tuple[int, str]:
    """Classify T(n) = a · T(n/b) + n^f_degree · (log n)^f_log_factor.
    Returns (case_number, big-Θ description)."""
    crit = math.log(a, b)
    eps = 1e-9
    if f_degree < crit - eps:
        return 1, f"Θ(n^{crit:.4f})"
    if abs(f_degree - crit) < eps:
        return 2, f"Θ(n^{crit:.4f} · log^{f_log_factor + 1} n)"
    return 3, f"Θ(n^{f_degree} · log^{f_log_factor} n)   (regularity assumed)"


# ── Recursion-tree summation ──────────────────────────────────────

def recursion_tree(a: int, b: int, f, n: int) -> int:
    """Sum work across the recursion tree: a^k · f(n/b^k) for k=0..⌊log_b n⌋."""
    total = 0
    size = n
    coef = 1
    while size >= 1:
        total += coef * f(size)
        size //= b
        coef *= a
    return total


# ── Demo ──────────────────────────────────────────────────────────

def demo_fibonacci():
    print("== Linear recurrence: Fibonacci ==")
    f30 = solve_linear([1, 1], [0, 1], 30)
    print(f"  F_30 (iterative) = {f30}")

    phi = (1 + math.sqrt(5)) / 2
    psi = (1 - math.sqrt(5)) / 2
    def binet(n): return round((phi**n - psi**n) / math.sqrt(5))
    print(f"  F_30 (Binet)     = {binet(30)}")
    assert f30 == binet(30)
    print(f"  ✓ characteristic-equation closed form matches iterative")


def demo_master():
    print("\n== Master theorem ==")
    cases = [
        ("Merge sort:      T(n) = 2T(n/2) + n",     2, 2, 1.0,   0),
        ("Binary search:   T(n) = T(n/2) + 1",      1, 2, 0.0,   0),
        ("Karatsuba:       T(n) = 3T(n/2) + n",     3, 2, 1.0,   0),
        ("Strassen:        T(n) = 7T(n/2) + n²",    7, 2, 2.0,   0),
        ("Trivial recurse: T(n) = T(n/2) + n",      1, 2, 1.0,   0),
        ("Quad work:       T(n) = 4T(n/2) + n",     4, 2, 1.0,   0),
        ("Heavy combine:   T(n) = 2T(n/2) + n²",    2, 2, 2.0,   0),
    ]
    for name, a, b, fd, fl in cases:
        case, ans = master(a, b, fd, fl)
        print(f"  {name:48s}  case {case}  →  {ans}")


def demo_tree_sum():
    print("\n== Recursion-tree summation vs Master theorem (merge sort) ==")
    a, b = 2, 2
    n = 1024
    work = recursion_tree(a, b, lambda x: x, n)
    print(f"  T({n}) via tree sum, T(n) = 2T(n/2) + n: {work}")
    print(f"  n·log₂(n) = {n * int(math.log2(n))}")
    print(f"  ratio = {work / (n * math.log2(n)):.3f}    (≈ 1, as expected)")


def demo_T_simulation():
    print("\n== Direct simulation of T(n) = 2T(n/2) + n ==")
    cache: dict = {}
    def T(n):
        if n <= 1: return 1
        if n in cache: return cache[n]
        cache[n] = 2 * T(n // 2) + n
        return cache[n]
    for n in [1024, 4096, 16384, 65536]:
        nl = n * math.log2(n)
        print(f"  n={n:6d}: T(n)={T(n):>9d}  ratio T/n log n = {T(n)/nl:.4f}")


def main():
    demo_fibonacci()
    demo_master()
    demo_tree_sum()
    demo_T_simulation()


if __name__ == "__main__":
    main()
