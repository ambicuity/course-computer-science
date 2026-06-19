"""Asymptotic notation — numerical comparisons and growth-rate hierarchy.

Run:  python3 main.py
"""
from __future__ import annotations

import math


def safe_factorial(n: int) -> int:
    # cap to keep printable; real factorial would overflow display fields
    return math.factorial(n) if n <= 20 else math.factorial(20) * (n // 20)


def demo_hierarchy():
    print("== Growth-rate hierarchy at n = 10, 30, 100 ==")
    cols = [10, 30, 100]
    rows = [
        ("1",            lambda n: 1),
        ("log₂ n",       lambda n: math.log2(n)),
        ("√n",           lambda n: math.sqrt(n)),
        ("n",            lambda n: n),
        ("n log₂ n",     lambda n: n * math.log2(n)),
        ("n²",           lambda n: n * n),
        ("n³",           lambda n: n ** 3),
        ("2ⁿ",           lambda n: 2.0 ** n),
        ("n!",           lambda n: math.factorial(n)),
    ]
    print(f"  {'function':<14s} " + " ".join(f"{c:>15s}" for c in (str(x) for x in cols)))
    for name, f in rows:
        cells = []
        for n in cols:
            try:
                v = f(n)
                if isinstance(v, float):
                    cells.append(f"{v:15.3e}")
                else:
                    cells.append(f"{v:>15.3e}" if v > 1e6 else f"{v:>15,d}")
            except OverflowError:
                cells.append(f"{'OVERFLOW':>15s}")
        print(f"  {name:<14s} " + " ".join(cells))


def demo_limit_test():
    print("\n== Limit-ratio test ==")
    pairs = [
        ("n log n", "n²", lambda n: n * math.log2(n), lambda n: n * n),
        ("3n² + 5n + 7", "n²", lambda n: 3 * n * n + 5 * n + 7, lambda n: n * n),
        ("log₂ n", "log₁₀ n", lambda n: math.log2(n), lambda n: math.log10(n)),
        ("2^n", "3^n", lambda n: 2.0**n, lambda n: 3.0**n),
        ("n!", "2^n", lambda n: math.factorial(n), lambda n: 2.0**n),
    ]
    for name_f, name_g, f, g in pairs:
        print(f"  {name_f} / {name_g}:")
        for n in [10, 50, 100, 200]:
            try:
                r = f(n) / g(n)
                print(f"    n={n:4d}:  {r:.4g}")
            except (OverflowError, ZeroDivisionError):
                print(f"    n={n:4d}:  (overflow)")
        print()


def demo_polynomial_identity():
    print("== Polynomial identity: 3n² + 5n + 7 = Θ(n²) ==")
    for n in [10, 100, 1000, 10000]:
        r = (3 * n * n + 5 * n + 7) / (n * n)
        print(f"  (3n²+5n+7)/n² at n={n:<6d} = {r:.6f}    (converges to 3)")


def demo_crossover():
    print("\n== Constant-factor crossover: c₁·n vs c₂·n² ==")
    c1, c2 = 1000, 0.01    # the 'fast' algorithm has huge constants
    print(f"  Algo A: {c1}·n     (asymptotic O(n))")
    print(f"  Algo B: {c2}·n²    (asymptotic O(n²))")
    for n in [100, 1000, 10000, 50000, 100000, 1_000_000]:
        a, b = c1 * n, c2 * n * n
        winner = "A" if a < b else ("B" if b < a else "tie")
        print(f"    n={n:<8d}: A={a:>12.0f}   B={b:>12.0f}   winner: {winner}")


def main():
    demo_hierarchy()
    demo_limit_test()
    demo_polynomial_identity()
    demo_crossover()


if __name__ == "__main__":
    main()
