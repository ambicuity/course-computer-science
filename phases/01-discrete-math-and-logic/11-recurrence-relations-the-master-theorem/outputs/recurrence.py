"""recurrence.py — recurrence solving + Master theorem classifier.

Used in Phase 04 (algorithm analysis) and Phase 11 (distributed-system math).
"""
from __future__ import annotations

import math
from typing import Callable, List, Tuple


def solve_linear(coeffs: List[int], initial: List[int], n: int) -> int:
    """Compute aₙ for a linear homogeneous recurrence."""
    a = list(initial)
    k = len(coeffs)
    while len(a) <= n:
        a.append(sum(coeffs[i] * a[-1 - i] for i in range(k)))
    return a[n]


def master(a: int, b: int, f_degree: float, f_log_factor: int = 0) -> Tuple[int, str]:
    """Classify T(n) = a · T(n/b) + n^f_degree · (log n)^f_log_factor."""
    crit = math.log(a, b)
    eps = 1e-9
    if f_degree < crit - eps:
        return 1, f"Θ(n^{crit:.4f})"
    if abs(f_degree - crit) < eps:
        return 2, f"Θ(n^{crit:.4f} · log^{f_log_factor + 1} n)"
    return 3, f"Θ(n^{f_degree} · log^{f_log_factor} n)"


def recursion_tree_sum(a: int, b: int, f: Callable[[int], int], n: int) -> int:
    total = 0
    size = n
    coef = 1
    while size >= 1:
        total += coef * f(size)
        size //= b
        coef *= a
    return total


if __name__ == "__main__":
    assert solve_linear([1, 1], [0, 1], 10) == 55   # F_10
    case, ans = master(2, 2, 1.0)
    assert case == 2 and "log^1" in ans
    case, ans = master(7, 2, 2.0)
    assert case == 1
    print("recurrence library smoke-test OK")
