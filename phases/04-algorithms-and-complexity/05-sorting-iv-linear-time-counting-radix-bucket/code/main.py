"""
Sorting IV — Linear-Time: Counting, Radix, Bucket
Phase 04 — Algorithms & Complexity Analysis

Implements counting sort, radix sort (LSD and MSD), bucket sort,
and a multi-algorithm auto-selecting sorter. Benchmarks against
Python's built-in Timsort.
"""

import random
import time
from collections import defaultdict


# ── Counting Sort ──────────────────────────────────────────────────────────────


def counting_sort(arr: list[int], max_val: int) -> list[int]:
    """Stable counting sort. O(n + k) time, O(n + k) space."""
    count = [0] * (max_val + 1)
    output = [0] * len(arr)

    for x in arr:
        count[x] += 1
    for i in range(1, max_val + 1):
        count[i] += count[i - 1]

    # Right-to-left traversal preserves stability
    for x in reversed(arr):
        count[x] -= 1
        output[count[x]] = x
    return output


def counting_sort_shifted(arr: list[int], min_val: int, max_val: int) -> list[int]:
    """Counting sort for arrays with a known [min_val, max_val] range."""
    range_size = max_val - min_val + 1
    count = [0] * range_size
    output = [0] * len(arr)

    for x in arr:
        count[x - min_val] += 1
    for i in range(1, range_size):
        count[i] += count[i - 1]

    for x in reversed(arr):
        count[x - min_val] -= 1
        output[count[x - min_val]] = x
    return output


# ── Radix Sort (LSD) ──────────────────────────────────────────────────────────


def _counting_sort_by_digit(arr: list[int], exp: int) -> list[int]:
    """Counting sort keyed by a single digit position. Internal use only."""
    count = [0] * 10
    output = [0] * len(arr)

    for x in arr:
        count[(x // exp) % 10] += 1
    for i in range(1, 10):
        count[i] += count[i - 1]
    for x in reversed(arr):
        index = (x // exp) % 10
        count[index] -= 1
        output[count[index]] = x
    return output


def radix_sort_lsd(arr: list[int]) -> list[int]:
    """LSD radix sort for non-negative integers. O(d·(n+k)), stable."""
    if not arr:
        return []
    max_val = max(arr)
    result = arr[:]
    exp = 1
    while max_val // exp > 0:
        result = _counting_sort_by_digit(result, exp)
        exp *= 10
    return result


# ── Radix Sort (MSD) ──────────────────────────────────────────────────────────


def radix_sort_msd(arr: list[int]) -> list[int]:
    """MSD radix sort for non-negative integers. O(d·(n+k)), stable."""
    if not arr:
        return []
    max_val = max(arr)
    max_digits = len(str(max_val))
    return _msd_recursive(arr, max_digits - 1)


def _msd_recursive(arr: list[int], digit_pos: int) -> list[int]:
    """Recursive helper for MSD radix sort."""
    if len(arr) <= 1 or digit_pos < 0:
        return arr

    exp = 10**digit_pos
    buckets: dict[int, list[int]] = defaultdict(list)

    for x in arr:
        buckets[(x // exp) % 10].append(x)

    result = []
    for d in range(10):
        result.extend(_msd_recursive(buckets[d], digit_pos - 1))
    return result


# ── Bucket Sort ────────────────────────────────────────────────────────────────


def bucket_sort(arr: list[float], n_buckets: int | None = None) -> list[float]:
    """Bucket sort for floats. O(n) average on uniform distribution."""
    if not arr:
        return []
    n = len(arr)
    n_buckets = n_buckets or n
    buckets: list[list[float]] = [[] for _ in range(n_buckets)]
    min_val, max_val = min(arr), max(arr)
    span = max_val - min_val or 1

    for x in arr:
        idx = min(int((x - min_val) / span * n_buckets), n_buckets - 1)
        buckets[idx].append(x)

    result: list[float] = []
    for b in buckets:
        b.sort()  # insertion sort ideal for tiny buckets
        result.extend(b)
    return result


# ── Smart Sort (auto-select) ──────────────────────────────────────────────────


def smart_sort(arr: list) -> list:
    """Auto-select sorting algorithm based on input characteristics."""
    if not arr or len(arr) <= 1:
        return list(arr)

    if all(isinstance(x, int) for x in arr):
        min_val, max_val = min(arr), max(arr)
        range_size = max_val - min_val + 1
        if range_size <= len(arr) * 4:
            return counting_sort_shifted(arr, min_val, max_val)

    if all(isinstance(x, float) for x in arr) and len(arr) >= 100:
        return bucket_sort(arr)

    return sorted(arr)


# ── Benchmarks ─────────────────────────────────────────────────────────────────


def benchmark(name: str, func, data, repeats: int = 3) -> float:
    """Run func(data) multiple times and return average wall-clock ms."""
    times = []
    for _ in range(repeats):
        d = data[:] if isinstance(data, list) else data
        t0 = time.perf_counter()
        func(d)
        times.append(time.perf_counter() - t0)
    avg_ms = sum(times) / len(times) * 1000
    print(f"  {name:<25s} {avg_ms:>8.2f} ms")
    return avg_ms


def main() -> None:
    print("=" * 60)
    print("Counting Sort")
    print("=" * 60)
    data = [4, 2, 4, 1, 2, 7, 0, 3]
    print(f"  Input:  {data}")
    print(f"  Output: {counting_sort(data, max(data))}")
    print("  Stability: elements with equal value preserve input order.\n")

    print("=" * 60)
    print("Radix Sort (LSD)")
    print("=" * 60)
    data = [170, 45, 75, 90, 802, 24, 2, 66]
    print(f"  Input:  {data}")
    print(f"  Output: {radix_sort_lsd(data)}\n")

    print("=" * 60)
    print("Radix Sort (MSD)")
    print("=" * 60)
    data = [170, 45, 75, 90, 802, 24, 2, 66]
    print(f"  Input:  {data}")
    print(f"  Output: {radix_sort_msd(data)}\n")

    print("=" * 60)
    print("Bucket Sort (uniform floats)")
    print("=" * 60)
    random.seed(42)
    data = [random.random() for _ in range(20)]
    print(f"  Input (first 5):  {[f'{x:.4f}' for x in data[:5]]}")
    sorted_data = bucket_sort(data)
    print(f"  Output (first 5): {[f'{x:.4f}' for x in sorted_data[:5]]}\n")

    # ── Benchmarks ──
    N = 100_000

    print("=" * 60)
    print(f"Benchmarks — {N:,} elements")
    print("=" * 60)

    # Small integer range (0-9): counting sort should win
    small_range = [random.randint(0, 9) for _ in range(N)]
    print(f"\nSmall range (values 0-9):")
    benchmark("Counting sort", lambda d: counting_sort(d, 9), small_range)
    benchmark("Built-in sorted()", sorted, small_range)

    # Medium integer range (0-999): radix sort should win
    medium_range = [random.randint(0, 999) for _ in range(N)]
    print(f"\nMedium range (values 0-999):")
    benchmark("Radix sort (LSD)", radix_sort_lsd, medium_range)
    benchmark("Built-in sorted()", sorted, medium_range)

    # Large integer range (0-999,999): Timsort catches up
    large_range = [random.randint(0, 999_999) for _ in range(N)]
    print(f"\nLarge range (values 0-999,999):")
    benchmark("Radix sort (LSD)", radix_sort_lsd, large_range)
    benchmark("Built-in sorted()", sorted, large_range)

    # Uniform floats: bucket sort vs Timsort
    uniform_floats = [random.random() for _ in range(N)]
    print(f"\nUniform floats [0, 1):")
    benchmark("Bucket sort", lambda d: bucket_sort(d, N // 10), uniform_floats)
    benchmark("Built-in sorted()", sorted, uniform_floats)

    # Reverse-sorted: worst case for many algorithms
    reverse_sorted = list(range(N, 0, -1))
    print(f"\nReverse-sorted input:")
    benchmark("Radix sort (LSD)", radix_sort_lsd, reverse_sorted)
    benchmark("Built-in sorted()", sorted, reverse_sorted)

    print("\n" + "=" * 60)
    print("When Linear Sorts Win vs Lose")
    print("=" * 60)
    print("""
    WIN  — small key range (counting sort: ages, ASCII codes)
    WIN  — fixed-width integers (radix sort: 32-bit keys)
    WIN  — uniform distribution (bucket sort: random floats)
    LOSE — large key range (counting sort needs O(k) space)
    LOSE — many digits (radix sort overhead × passes)
    LOSE — skewed distribution (bucket sort degrades to O(n²))
    LOSE — complex keys (comparison sort is the only option)
    """)


if __name__ == "__main__":
    main()
