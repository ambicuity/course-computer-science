"""prob.py — discrete-probability helpers used by Phase 04 (randomized algorithms)."""
from __future__ import annotations

import math
import random
from typing import Callable


def expectation_estimate(rv: Callable[[], float], trials: int = 100000) -> float:
    return sum(rv() for _ in range(trials)) / trials


def birthday_prob(n: int, m: int = 365) -> float:
    """Probability of at least one collision when n items are placed in m bins (no birthday)."""
    p_no = 1.0
    for i in range(n):
        p_no *= (m - i) / m
        if p_no <= 0:
            return 1.0
    return 1 - p_no


def expected_coupon_time(n: int) -> float:
    """n · H_n: expected time to collect all n distinct coupons."""
    return n * sum(1.0 / k for k in range(1, n + 1))


def markov_bound(mu: float, a: float) -> float:
    """Markov: P(X ≥ a) ≤ μ / a for non-negative X."""
    return mu / a


def chebyshev_bound(sigma: float, k: float) -> float:
    """Chebyshev: P(|X − μ| ≥ kσ) ≤ 1/k²."""
    return 1.0 / (k * k)


if __name__ == "__main__":
    assert abs(birthday_prob(23) - 0.5073) < 0.01
    assert abs(expected_coupon_time(10) - 29.29) < 0.5
    assert abs(markov_bound(500, 700) - 0.7143) < 1e-3
    print("prob library smoke-test OK")
