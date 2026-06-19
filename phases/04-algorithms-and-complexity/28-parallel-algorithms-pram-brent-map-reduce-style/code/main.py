"""
Parallel Algorithms — PRAM, Brent, Map-Reduce style
Phase 04 — Algorithms & Complexity Analysis

Parallel prefix sum, parallel merge sort, speedup benchmarks.
"""

import math
import time
from concurrent.futures import ThreadPoolExecutor


# ─── Parallel Prefix Sum (Blelloch-style scan) ────────────────────────────


def sequential_prefix_sum(arr: list[int]) -> list[int]:
    """Exclusive prefix sum: out[i] = sum of arr[0..i)."""
    out = []
    acc = 0
    for x in arr:
        out.append(acc)
        acc += x
    return out


def parallel_prefix_sum(arr: list[int]) -> list[int]:
    """Work-efficient exclusive prefix sum using up-sweep + down-sweep.

    Work:  O(n)       (matches sequential)
    Depth: O(log n)   (log n tree levels)

    Uses ThreadPoolExecutor to simulate PRAM concurrent access.
    """
    n = len(arr)
    if n == 0:
        return []
    if n == 1:
        return [0]

    # Pad to next power of two
    m = 1 << math.ceil(math.log2(n))
    buf = list(arr) + [0] * (m - n)

    # ── Up-sweep (reduce) ──
    d = 1
    while d < m:
        pairs = [(i, d) for i in range(d, m, 2 * d)]

        def _up(pair, _buf=buf):
            i, step = pair
            _buf[i + step - 1] += _buf[i - 1]

        with ThreadPoolExecutor() as pool:
            list(pool.map(_up, pairs))
        d *= 2

    # Set last element to 0 (exclusive scan)
    buf[m - 1] = 0

    # ── Down-sweep ──
    d = m // 2
    while d >= 1:
        pairs = [(i, d) for i in range(d, m, 2 * d)]

        def _down(pair, _buf=buf):
            i, step = pair
            old_left = _buf[i - 1]
            _buf[i - 1] = _buf[i + step - 1]
            _buf[i + step - 1] += old_left

        with ThreadPoolExecutor() as pool:
            list(pool.map(_down, pairs))
        d //= 2

    return buf[:n]


# ─── Parallel Merge Sort ──────────────────────────────────────────────────


def sequential_merge(left: list[int], right: list[int]) -> list[int]:
    result = []
    i = j = 0
    while i < len(left) and j < len(right):
        if left[i] <= right[j]:
            result.append(left[i])
            i += 1
        else:
            result.append(right[j])
            j += 1
    result.extend(left[i:])
    result.extend(right[j:])
    return result


def parallel_merge_sort(
    arr: list[int],
    executor: ThreadPoolExecutor | None = None,
    depth: int = 0,
    max_parallel_depth: int = 4,
) -> list[int]:
    """Fork-join parallel merge sort.

    Work:  O(n log n)  (same as sequential merge sort)
    Depth: O(n)        (naive sequential merge); O(log^2 n) with parallel merge.

    Limits fork depth to max_parallel_depth to avoid thread explosion.
    """
    if len(arr) <= 1:
        return arr

    own_executor = executor is None
    if own_executor:
        executor = ThreadPoolExecutor()

    mid = len(arr) // 2

    try:
        if depth < max_parallel_depth:
            left_fut = executor.submit(
                parallel_merge_sort, arr[:mid], executor, depth + 1, max_parallel_depth
            )
            right_fut = executor.submit(
                parallel_merge_sort, arr[mid:], executor, depth + 1, max_parallel_depth
            )
            left = left_fut.result()
            right = right_fut.result()
        else:
            left = sorted(arr[:mid])
            right = sorted(arr[mid:])
    finally:
        if own_executor:
            executor.shutdown(wait=True)

    return sequential_merge(left, right)


# ─── Work-Depth Analysis ──────────────────────────────────────────────────


def prefix_sum_work_depth(n: int) -> dict:
    """Return theoretical work and depth for parallel prefix sum."""
    return {
        "work": f"O({n})",
        "depth": f"O({int(math.log2(max(n, 1)))})",
        "work_expr": f"O(n) = O({n})",
        "depth_expr": f"O(log n) = O({int(math.log2(max(n, 1)))})",
    }


def merge_sort_work_depth(n: int) -> dict:
    """Return theoretical work and depth for parallel merge sort (naive merge)."""
    log_n = int(math.log2(max(n, 1)))
    return {
        "work": f"O({n} log {n})",
        "depth": f"O({n})  [sequential merge]; O({log_n}^2) with parallel merge",
        "work_expr": f"O(n log n) ~ O({n * log_n if n > 1 else 0})",
        "depth_expr": f"O(log^2 n) = O({log_n ** 2}) [parallel merge]",
    }


# ─── Speedup Measurement ──────────────────────────────────────────────────


def measure_speedup(
    parallel_fn,
    sequential_fn,
    data: list[int],
    label: str = "",
) -> tuple[float, float]:
    t0 = time.perf_counter()
    seq_result = sequential_fn(data)
    t_seq = time.perf_counter() - t0

    t0 = time.perf_counter()
    par_result = parallel_fn(data)
    t_par = time.perf_counter() - t0

    assert par_result == seq_result, f"Results differ!\n  seq={seq_result}\n  par={par_result}"

    speedup = t_seq / t_par if t_par > 0 else float("inf")
    print(f"  [{label}]")
    print(f"    Sequential: {t_seq:.6f}s")
    print(f"    Parallel:   {t_par:.6f}s")
    print(f"    Speedup:    {speedup:.2f}x")
    print()

    return t_seq, t_par


# ─── Map-Reduce Style Demo ────────────────────────────────────────────────


def map_reduce_word_count(lines: list[str]) -> dict[str, int]:
    """Classic Map-Reduce word count using Python's thread pool.

    Map:    each line → list of (word, 1) pairs  [parallel]
    Shuffle: group by word                        [sequential in this demo]
    Reduce: sum counts per word                   [parallel]
    """

    # Map phase
    def mapper(line: str) -> list[tuple[str, int]]:
        return [(w.lower(), 1) for w in line.split() if w]

    with ThreadPoolExecutor() as pool:
        mapped = list(pool.map(mapper, lines))

    # Shuffle phase
    groups: dict[str, list[int]] = {}
    for pairs in mapped:
        for word, count in pairs:
            groups.setdefault(word, []).append(count)

    # Reduce phase
    def reducer(item: tuple[str, list[int]]) -> tuple[str, int]:
        word, counts = item
        return word, sum(counts)

    with ThreadPoolExecutor() as pool:
        results = list(pool.map(reducer, groups.items()))

    return dict(results)


# ─── main ─────────────────────────────────────────────────────────────────


def main() -> None:
    import random

    random.seed(42)

    # ── Prefix Sum ──
    print("=== Parallel Prefix Sum ===")
    for size_exp in [8, 12, 16]:
        n = 2 ** size_exp
        data = [random.randint(1, 100) for _ in range(n)]
        ad = prefix_sum_work_depth(n)
        print(f"  n={n}: work={ad['work']}, depth={ad['depth']}")
        measure_speedup(parallel_prefix_sum, sequential_prefix_sum, data, label=f"prefix_sum n={n}")

    # ── Merge Sort ──
    print("=== Parallel Merge Sort ===")
    for size_exp in [10, 14]:
        n = 2 ** size_exp
        data = [random.randint(1, 10_000) for _ in range(n)]
        ad = merge_sort_work_depth(n)
        print(f"  n={n}: work={ad['work']}, depth={ad['depth']}")
        measure_speedup(
            lambda arr: parallel_merge_sort(arr),
            sorted,
            data,
            label=f"merge_sort n={n}",
        )

    # ── Map-Reduce Word Count ──
    print("=== Map-Reduce Word Count ===")
    lines = [
        "the quick brown fox jumps over the lazy dog",
        "the lazy dog sleeps under the brown tree",
        "a quick fox and a lazy dog play together",
    ]
    counts = map_reduce_word_count(lines)
    for word in sorted(counts):
        print(f"  {word}: {counts[word]}")

    # ── Brent's Theorem Demo ──
    print("\n=== Brent's Theorem ===")
    T1 = 1000   # work (sequential time)
    T_inf = 10  # depth (critical path)
    for p in [1, 2, 4, 8, 16, 32, 64]:
        lower = math.ceil(T1 / p)
        upper = math.ceil(T1 / p) + T_inf
        print(f"  p={p:3d}:  T1/p = {lower:4d}  ≤  T_p  ≤  {upper:4d}  (= T1/p + T∞)")


if __name__ == "__main__":
    main()
