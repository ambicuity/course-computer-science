"""growth.py — numerical companions to asymptotic-notation reasoning."""
from __future__ import annotations

import math
from typing import Callable, List, Tuple


def limit_ratio(f: Callable[[int], float], g: Callable[[int], float],
                ns: List[int]) -> List[Tuple[int, float]]:
    """Compute f(n)/g(n) at each n. Trend reveals which of o, Θ, ω holds."""
    out = []
    for n in ns:
        try:
            out.append((n, f(n) / g(n)))
        except (OverflowError, ZeroDivisionError):
            out.append((n, float("inf")))
    return out


def crossover_n(c1: float, f: Callable[[int], float],
                c2: float, g: Callable[[int], float],
                hi: int = 10**9) -> int:
    """Smallest n where c1·f(n) <= c2·g(n) (i.e., f wins or ties). Returns -1 if none below hi."""
    lo, hi_ = 1, hi
    while lo < hi_:
        mid = (lo + hi_) // 2
        if c1 * f(mid) <= c2 * g(mid):
            hi_ = mid
        else:
            lo = mid + 1
    return lo if c1 * f(lo) <= c2 * g(lo) else -1


def hierarchy_at(n: int) -> dict:
    return {
        "1": 1,
        "log n": math.log2(n),
        "n^0.5": math.sqrt(n),
        "n": n,
        "n log n": n * math.log2(n),
        "n^2": n * n,
        "n^3": n ** 3,
        "2^n": 2.0**n if n < 1024 else float("inf"),
        "n!": math.factorial(n) if n <= 170 else float("inf"),
    }


if __name__ == "__main__":
    # Verify expected limits
    ratios = limit_ratio(lambda n: n * math.log2(n), lambda n: n * n, [10, 100, 1000])
    assert ratios[-1][1] < ratios[0][1], "n log n / n² should decrease"

    # Crossover: 1000·n vs 0.01·n² → n at ~100000
    cn = crossover_n(1000.0, lambda n: n, 0.01, lambda n: n * n)
    assert 90000 <= cn <= 110000, cn
    print(f"growth library: crossover at n = {cn}")
