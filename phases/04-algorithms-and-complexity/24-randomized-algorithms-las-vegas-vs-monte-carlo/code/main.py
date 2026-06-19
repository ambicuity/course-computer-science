"""
Randomized Algorithms — Las Vegas vs Monte Carlo
Phase 04 — Algorithms & Complexity Analysis
"""

import copy
import math
import random


# ---------------------------------------------------------------------------
# Las Vegas: Randomized Quicksort
# ---------------------------------------------------------------------------

def randomized_quicksort(arr: list[int]) -> tuple[list[int], int]:
    """Sort in-place. Returns (sorted_list, comparison_count)."""
    a = arr[:]
    comparisons = [0]

    def partition(lo: int, hi: int) -> int:
        ri = random.randint(lo, hi)
        a[ri], a[hi] = a[hi], a[ri]
        pivot = a[hi]
        i = lo
        for j in range(lo, hi):
            comparisons[0] += 1
            if a[j] <= pivot:
                a[i], a[j] = a[j], a[i]
                i += 1
        a[i], a[hi] = a[hi], a[i]
        return i

    def sort(lo: int, hi: int) -> None:
        if lo < hi:
            p = partition(lo, hi)
            sort(lo, p - 1)
            sort(p + 1, hi)

    sort(0, len(a) - 1)
    return a, comparisons[0]


# ---------------------------------------------------------------------------
# Las Vegas: Randomized Select (Quickselect)
# ---------------------------------------------------------------------------

def randomized_select(arr: list[int], k: int) -> int:
    """Return the k-th smallest element (0-indexed). Expected O(n)."""
    if len(arr) == 1:
        return arr[0]
    pivot = random.choice(arr)
    lows = [x for x in arr if x < pivot]
    highs = [x for x in arr if x > pivot]
    pivots = [x for x in arr if x == pivot]
    if k < len(lows):
        return randomized_select(lows, k)
    elif k < len(lows) + len(pivots):
        return pivot
    else:
        return randomized_select(highs, k - len(lows) - len(pivots))


# ---------------------------------------------------------------------------
# Monte Carlo: Miller-Rabin Primality Test
# ---------------------------------------------------------------------------

def miller_rabin(n: int, k: int = 40) -> bool:
    """Return True if n is (probably) prime. Error ≤ 4^(-k)."""
    if n < 2:
        return False
    if n < 4:
        return True
    if n % 2 == 0:
        return False

    # Write n - 1 = 2^r * d, d odd
    r, d = 0, n - 1
    while d % 2 == 0:
        r += 1
        d //= 2

    for _ in range(k):
        a = random.randrange(2, n - 1)
        x = pow(a, d, n)
        if x == 1 or x == n - 1:
            continue
        for _ in range(r - 1):
            x = pow(x, 2, n)
            if x == n - 1:
                break
        else:
            return False
    return True


# ---------------------------------------------------------------------------
# Monte Carlo: Karger's Randomized Min-Cut
# ---------------------------------------------------------------------------

def karger_min_cut(graph: dict[int, list[int]]) -> int:
    """Single trial of Karger's contraction algorithm."""
    g: dict[int, list[int]] = {k: list(v) for k, v in graph.items()}
    vertices = list(g.keys())
    while len(vertices) > 2:
        u = random.choice(vertices)
        v = random.choice(g[u])
        # Merge v into u
        for w in g[v]:
            g[w] = [u if x == v else x for x in g[w]]
        g[u].extend(g[v])
        g[u] = [x for x in g[u] if x != u]
        vertices.remove(v)
    return len(g[vertices[0]])


def karger_min_cut_repeated(graph: dict[int, list[int]], trials: int | None = None) -> int:
    """Run Karger n^2 ln n times and return the smallest cut found."""
    n = len(graph)
    if trials is None:
        trials = n * n
    best = float("inf")
    for _ in range(trials):
        cut = karger_min_cut(graph)
        if cut < best:
            best = cut
    return best


# ---------------------------------------------------------------------------
# Variance analysis on quicksort comparisons
# ---------------------------------------------------------------------------

def quicksort_variance(n: int, trials: int = 500) -> tuple[float, float]:
    """Empirical mean and variance of comparison count on random arrays."""
    counts = []
    for _ in range(trials):
        arr = list(range(n))
        random.shuffle(arr)
        _, c = randomized_quicksort(arr)
        counts.append(c)
    mean = sum(counts) / len(counts)
    var = sum((c - mean) ** 2 for c in counts) / len(counts)
    return mean, var


# ---------------------------------------------------------------------------
# Demonstration
# ---------------------------------------------------------------------------

def main() -> None:
    random.seed(42)

    # 1. Randomized quicksort
    print("=== Randomized Quicksort ===")
    for n in [100, 1000, 10000]:
        arr = list(range(n))
        random.shuffle(arr)
        _, comps = randomized_quicksort(arr)
        expected = 2 * n * math.log(n)
        ratio = comps / expected
        print(f"  n={n:>5}: comparisons={comps:>7},  2n ln n={expected:>9.0f},  ratio={ratio:.3f}")

    # 2. Variance analysis
    print("\n=== Quicksort Variance Analysis ===")
    for n in [100, 500]:
        mean, var = quicksort_variance(n, trials=500)
        theoretical_mean = 2 * n * math.log(n)
        print(f"  n={n:>4}: empirical mean={mean:>8.1f}, theoretical={theoretical_mean:>8.1f}, variance={var:>8.0f}")

    # 3. Randomized select
    print("\n=== Randomized Select (Quickselect) ===")
    arr = list(range(100))
    random.shuffle(arr)
    for k in [0, 49, 99]:
        val = randomized_select(arr, k)
        print(f"  k={k:>2}: {val} (correct={val == k})")

    # 4. Miller-Rabin
    print("\n=== Miller-Rabin Primality Test ===")
    known_primes = [2, 3, 5, 7, 11, 13, 17, 19, 97, 65537]
    known_composites = [1, 4, 15, 561, 1105, 1729, 29341]  # 561..29341 are Carmichael numbers
    for p in known_primes:
        assert miller_rabin(p), f"Failed on prime {p}"
    for c in known_composites:
        assert not miller_rabin(c), f"False positive on composite {c}"
    print("  All known primes/composites passed.")

    # Large prime test
    big_prime = (1 << 61) - 1  # Mersenne prime 2^61 - 1
    print(f"  2^61 - 1 = {big_prime}: prime = {miller_rabin(big_prime)}")

    # Generate a random probable prime (simulating key generation)
    for bits in [32, 64, 128]:
        candidate = random.getrandbits(bits) | 1  # ensure odd
        while not miller_rabin(candidate, k=20):
            candidate = random.getrandbits(bits) | 1
        print(f"  Random {bits}-bit probable prime: {candidate}")

    # 5. Karger's min-cut
    print("\n=== Karger's Min-Cut ===")
    # Square graph: 4 vertices, min-cut = 2
    square = {
        0: [1, 3],
        1: [0, 2],
        2: [1, 3],
        3: [0, 2],
    }
    cut = karger_min_cut_repeated(square, trials=100)
    print(f"  Square (min-cut=2): found cut = {cut}")

    # Complete graph K4: min-cut = 3
    k4 = {
        0: [1, 2, 3],
        1: [0, 2, 3],
        2: [0, 1, 3],
        3: [0, 1, 2],
    }
    cut = karger_min_cut_repeated(k4, trials=200)
    print(f"  K4 (min-cut=3):     found cut = {cut}")

    # Petersen graph: min-cut = 3
    petersen = {
        0: [1, 4, 5], 1: [0, 2, 6], 2: [1, 3, 7],
        3: [2, 4, 8], 4: [0, 3, 9], 5: [0, 7, 8],
        6: [1, 8, 9], 7: [2, 5, 9], 8: [3, 5, 6],
        9: [4, 6, 7],
    }
    cut = karger_min_cut_repeated(petersen, trials=500)
    print(f"  Petersen (min-cut=3): found cut = {cut}")

    print("\nAll demonstrations complete.")


if __name__ == "__main__":
    main()
