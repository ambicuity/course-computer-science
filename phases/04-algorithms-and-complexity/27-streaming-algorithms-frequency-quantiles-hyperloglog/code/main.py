"""
Streaming Algorithms — Frequency, Quantiles, HyperLogLog
Phase 04 — Algorithms & Complexity Analysis

Implements three streaming algorithms from scratch:
  1. Reservoir Sampling — uniform k-sample from unknown-length stream
  2. Count-Min Sketch    — approximate frequency estimation
  3. HyperLogLog         — cardinality estimation (count distinct)
"""

import hashlib
import math
import random
import string
import struct
from collections import Counter


# ---------------------------------------------------------------------------
# 1. Reservoir Sampling
# ---------------------------------------------------------------------------

def reservoir_sample(stream, k):
    """Return a uniformly random k-sample from a stream of unknown length.

    Each element has exactly k/n probability of being selected after n items.
    Space: O(k).  Time: O(n).
    """
    reservoir = []
    for i, item in enumerate(stream):
        if i < k:
            reservoir.append(item)
        else:
            j = random.randint(0, i)
            if j < k:
                reservoir[j] = item
    return reservoir


# ---------------------------------------------------------------------------
# 2. Count-Min Sketch
# ---------------------------------------------------------------------------

class CountMinSketch:
    """Approximate frequency counter with ε-δ guarantees.

    After N insertions:
        true_count(x) ≤ estimate(x)
        estimate(x) ≤ true_count(x) + εN   with probability ≥ 1 − δ

    Space: O(w · d) where w = ⌈e/ε⌉, d = ⌈ln(1/δ)⌉.
    """

    def __init__(self, epsilon=0.001, delta=0.01):
        self.w = int(math.ceil(math.e / epsilon))
        self.d = int(math.ceil(math.log(1 / delta)))
        self.counters = [[0] * self.w for _ in range(self.d)]
        self.seeds = [random.randint(0, 2**31 - 1) for _ in range(self.d)]
        self.total = 0

    def _hash(self, element, seed):
        h = hashlib.md5(f"{seed}:{element}".encode()).hexdigest()
        return int(h, 16) % self.w

    def add(self, element, count=1):
        self.total += count
        for i in range(self.d):
            j = self._hash(element, self.seeds[i])
            self.counters[i][j] += count

    def estimate(self, element):
        return min(
            self.counters[i][self._hash(element, self.seeds[i])]
            for i in range(self.d)
        )

    def heavy_hitters(self, threshold=0.01):
        """Return dict of candidate elements whose estimate > threshold * total.

        Note: this scans all unique bucket values. In production you'd track
        candidates via Misra-Gries or a separate candidate set.
        """
        candidates = {}
        seen_hashes = set()
        for i in range(self.d):
            for j in range(self.w):
                if self.counters[i][j] > threshold * self.total:
                    # Cannot recover the original element from a counter —
                    # real systems maintain a separate candidate dictionary.
                    pass
        return candidates


# ---------------------------------------------------------------------------
# 3. HyperLogLog
# ---------------------------------------------------------------------------

class HyperLogLog:
    """Cardinality estimation using harmonic mean of per-bucket max leading zeros.

    Relative error: ~1.04/√m.
    Space: O(m) registers (default m = 2^14 = 16384 ≈ 16 KB).
    """

    def __init__(self, p=14):
        self.p = p
        self.m = 1 << p
        self.registers = [0] * self.m
        if self.m >= 128:
            self.alpha = 0.7213 / (1 + 1.079 / self.m)
        elif self.m >= 64:
            self.alpha = 0.7093 / (1 + 1.079 / self.m)
        elif self.m >= 32:
            self.alpha = 0.697 / (1 + 1.079 / self.m)
        else:
            self.alpha = 0.673

    def _hash(self, element):
        h = hashlib.md5(str(element).encode()).digest()
        return struct.unpack("<Q", h[:8])[0]

    @staticmethod
    def _rho(w):
        """Position of the least significant 1-bit + 1 (leading zeros + 1)."""
        if w == 0:
            return 65
        return (w ^ (w - 1)).bit_length()

    def add(self, element):
        x = self._hash(element)
        j = x & (self.m - 1)
        w = x >> self.p
        rho = self._rho(w)
        if rho > self.registers[j]:
            self.registers[j] = rho

    def count(self):
        """Return the estimated number of distinct elements."""
        Z = sum(2.0 ** (-r) for r in self.registers)
        estimate = self.alpha * self.m * self.m / Z

        # Small-range correction (many empty registers)
        if estimate <= 2.5 * self.m:
            V = self.registers.count(0)
            if V > 0:
                estimate = self.m * math.log(self.m / V)

        # Large-range correction (near hash-space exhaustion)
        two64 = 1 << 64
        if estimate > two64 / 30:
            estimate = -two64 * math.log(1 - estimate / two64)

        return int(estimate)


# ---------------------------------------------------------------------------
# Demo helpers
# ---------------------------------------------------------------------------

def random_url():
    path = ''.join(random.choices(string.ascii_lowercase, k=8))
    return f"https://example.com/{path}"


def demo_reservoir():
    print("=" * 60)
    print("Reservoir Sampling")
    print("=" * 60)
    n = 100_000
    k = 5
    stream = (random_url() for _ in range(n))
    sample = reservoir_sample(stream, k)
    print(f"Sampled {k} items from a stream of {n}:")
    for url in sample:
        print(f"  {url}")

    # Verify uniformity: each slot's index should be uniformly distributed
    # over many trials (statistical test)
    trials = 50_000
    k_test = 3
    n_test = 10
    counts = Counter()
    for _ in range(trials):
        stream = range(n_test)
        sample = reservoir_sample(stream, k_test)
        for item in sample:
            counts[item] += 1
    expected = trials * k_test / n_test
    print(f"\nUniformity test ({trials} trials, k={k_test}, n={n_test}):")
    print(f"  Expected per-item count: {expected:.0f}")
    max_dev = max(abs(counts[i] - expected) for i in range(n_test))
    print(f"  Max deviation: {max_dev:.0f} ({max_dev / expected * 100:.1f}%)")
    print()


def demo_count_min_sketch():
    print("=" * 60)
    print("Count-Min Sketch")
    print("=" * 60)
    cms = CountMinSketch(epsilon=0.001, delta=0.01)
    exact = Counter()

    # Simulate a stream with a Zipfian distribution
    elements = [f"item_{i}" for i in range(1000)]
    for _ in range(500_000):
        idx = int(random.paretovariate(1.2)) % len(elements)
        elem = elements[idx]
        cms.add(elem)
        exact[elem] += 1

    # Measure accuracy on top-10 elements
    top_10 = exact.most_common(10)
    print(f"Sketch dimensions: {cms.d} rows × {cms.w} cols = {cms.d * cms.w} counters")
    print(f"Memory: ~{cms.d * cms.w * 4} bytes (4-byte counters)\n")
    print(f"{'Element':<12} {'Exact':>8} {'Estimate':>8} {'Error':>8}")
    print("-" * 40)
    total_abs_error = 0
    for elem, true_count in top_10:
        est = cms.estimate(elem)
        error = est - true_count
        total_abs_error += abs(error)
        print(f"{elem:<12} {true_count:>8} {est:>8} {error:>+8}")
    print(f"\nMean absolute error (top 10): {total_abs_error / 10:.1f}")
    print()


def demo_hyperloglog():
    print("=" * 60)
    print("HyperLogLog")
    print("=" * 60)

    for p in [10, 12, 14]:
        m = 1 << p
        hll = HyperLogLog(p=p)
        exact_set = set()

        n_unique = 500_000
        for _ in range(n_unique):
            url = random_url()
            exact_set.add(url)
            hll.add(url)

        exact_count = len(exact_set)
        hll_count = hll.count()
        error_pct = abs(hll_count - exact_count) / exact_count * 100
        expected_err = 104 / math.sqrt(m)

        print(f"m = {m:>6} registers  |  Exact: {exact_count:>7}  |  "
              f"HLL: {hll_count:>7}  |  Error: {error_pct:.2f}%  "
              f"(expected ~{expected_err:.2f}%)")

    print()


def demo_accuracy_vs_memory():
    """Show how HyperLogLog error decreases as register count grows."""
    print("=" * 60)
    print("HyperLogLog: Accuracy vs Memory")
    print("=" * 60)
    n = 1_000_000
    exact_set = set()
    for _ in range(n):
        exact_set.add(random_url())
    exact_count = len(exact_set)

    print(f"Stream: {n} insertions, {exact_count} unique elements\n")
    print(f"{'Registers':>10} {'Memory (KB)':>12} {'Estimate':>10} {'Error %':>10}")
    print("-" * 46)
    for p in range(6, 15):
        m = 1 << p
        hll = HyperLogLog(p=p)
        for url in exact_set:
            hll.add(url)
        est = hll.count()
        err = abs(est - exact_count) / exact_count * 100
        mem_kb = m / 1024
        print(f"{m:>10} {mem_kb:>12.1f} {est:>10} {err:>9.2f}%")
    print()


def main():
    random.seed(42)
    demo_reservoir()
    demo_count_min_sketch()
    demo_hyperloglog()
    demo_accuracy_vs_memory()


if __name__ == "__main__":
    main()
