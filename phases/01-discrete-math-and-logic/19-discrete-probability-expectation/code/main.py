"""Discrete probability: linearity of expectation, birthday paradox, coupon collector.

Run:  python3 main.py
"""
from __future__ import annotations

import math
import random
from typing import Callable


def expectation_estimate(rv: Callable[[], float], trials: int = 100000) -> float:
    return sum(rv() for _ in range(trials)) / trials


# ── 1. Linearity of expectation: ascents in a permutation ─────────

def count_ascents(perm) -> int:
    return sum(1 for i in range(len(perm) - 1) if perm[i] < perm[i + 1])


def demo_linearity():
    print("== Linearity of expectation: ascents in a random permutation of [0..19] ==")
    n = 20

    def trial():
        p = list(range(n))
        random.shuffle(p)
        return count_ascents(p)

    e_emp = expectation_estimate(trial, trials=20000)
    e_theory = (n - 1) / 2
    print(f"  Empirical E[ascents] over 20k trials:   {e_emp:.4f}")
    print(f"  Theoretical (n-1)/2:                    {e_theory}")
    assert abs(e_emp - e_theory) < 0.1


# ── 2. Birthday paradox ─────────────────────────────────────────

def birthday_prob_closed(n: int, year: int = 365) -> float:
    p_no = 1.0
    for i in range(n):
        p_no *= (year - i) / year
    return 1 - p_no


def birthday_sim(n: int, trials: int = 20000, year: int = 365) -> float:
    rng = random.Random(0)
    hits = 0
    for _ in range(trials):
        bdays = [rng.randint(0, year - 1) for _ in range(n)]
        if len(set(bdays)) < len(bdays):
            hits += 1
    return hits / trials


def demo_birthday():
    print("\n== Birthday paradox: 365 buckets ==")
    print("  n     simulated   closed-form")
    for n in [5, 10, 20, 23, 30, 50, 100]:
        print(f"  {n:3d}    sim={birthday_sim(n):.4f}   closed={birthday_prob_closed(n):.4f}")


# ── 3. Coupon collector ─────────────────────────────────────────

def coupon_collector_sim(n: int, trials: int = 3000) -> float:
    rng = random.Random(7)
    total = 0
    for _ in range(trials):
        seen = set()
        count = 0
        while len(seen) < n:
            seen.add(rng.randint(0, n - 1))
            count += 1
        total += count
    return total / trials


def harmonic(n: int) -> float:
    return sum(1.0 / k for k in range(1, n + 1))


def demo_coupon():
    print("\n== Coupon collector ==")
    for n in [10, 50, 100, 365]:
        emp = coupon_collector_sim(n)
        theory = n * harmonic(n)
        print(f"  n={n:4d}: simulated mean = {emp:>9.2f},  n·H_n = {theory:>9.2f}")


# ── 4. Markov / Chebyshev vs empirical ──────────────────────────

def demo_markov_chebyshev():
    print("\n== Markov vs Chebyshev vs empirical: 1000 fair coin tosses ==")
    n = 1000
    mu = n / 2          # 500
    var = n * 0.5 * 0.5 # 250
    sigma = math.sqrt(var)

    a = 700
    markov = mu / a
    k = 200 / sigma
    cheb = 1 / (k * k)

    rng = random.Random(0)
    trials = 30000
    hits = sum(1 for _ in range(trials) if sum(rng.randint(0, 1) for _ in range(n)) >= a)
    emp = hits / trials

    print(f"  P(X ≥ {a}) with μ=500, σ≈{sigma:.2f}:")
    print(f"    Markov bound:    ≤ {markov:.4f}    (loose)")
    print(f"    Chebyshev bound: ≤ {cheb:.6f}   (one-sided is half — still loose)")
    print(f"    Empirical:       ≈ {emp:.6f}")


def main():
    demo_linearity()
    demo_birthday()
    demo_coupon()
    demo_markov_chebyshev()


if __name__ == "__main__":
    main()
