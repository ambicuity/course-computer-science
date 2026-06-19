"""
Sorting I — Insertion, Selection, Bubble, and Why They Lose
Phase 04 — Algorithms & Complexity Analysis

Implementations with comparison/swap counters and benchmark harness.
"""

import random
import time


def insertion_sort(arr):
    """Sort in-place. Returns (comparisons, swaps)."""
    comparisons = 0
    swaps = 0
    for i in range(1, len(arr)):
        key = arr[i]
        j = i - 1
        while j >= 0:
            comparisons += 1
            if arr[j] > key:
                arr[j + 1] = arr[j]
                swaps += 1
                j -= 1
            else:
                break
        arr[j + 1] = key
    return comparisons, swaps


def selection_sort(arr):
    """Sort in-place. Returns (comparisons, swaps)."""
    comparisons = 0
    swaps = 0
    n = len(arr)
    for i in range(n - 1):
        min_idx = i
        for j in range(i + 1, n):
            comparisons += 1
            if arr[j] < arr[min_idx]:
                min_idx = j
        if min_idx != i:
            arr[i], arr[min_idx] = arr[min_idx], arr[i]
            swaps += 1
    return comparisons, swaps


def bubble_sort(arr):
    """Sort in-place with early exit. Returns (comparisons, swaps)."""
    comparisons = 0
    swaps = 0
    n = len(arr)
    for i in range(n - 1):
        swapped = False
        for j in range(n - 1 - i):
            comparisons += 1
            if arr[j] > arr[j + 1]:
                arr[j], arr[j + 1] = arr[j + 1], arr[j]
                swaps += 1
                swapped = True
        if not swapped:
            break
    return comparisons, swaps


def benchmark(sort_fn, data):
    """Run sort on a copy, return (comparisons, swaps, elapsed_ms)."""
    arr = list(data)
    start = time.perf_counter()
    comps, swaps = sort_fn(arr)
    elapsed = (time.perf_counter() - start) * 1000
    assert arr == sorted(data), f"{sort_fn.__name__} produced wrong result"
    return comps, swaps, elapsed


def generate_inputs(n):
    """Return dict of input types: random, sorted, reverse, sawtooth."""
    rng = random.Random(42)
    random_data = [rng.randint(0, n * 10) for _ in range(n)]
    sorted_data = list(range(n))
    reverse_data = list(range(n, 0, -1))
    sawtooth = [(i % (n // 10 + 1)) for i in range(n)]
    return {
        "random": random_data,
        "sorted": sorted_data,
        "reverse": reverse_data,
        "sawtooth": sawtooth,
    }


def main():
    sorts = [
        ("Insertion", insertion_sort),
        ("Selection", selection_sort),
        ("Bubble   ", bubble_sort),
    ]
    sizes = [100, 1000, 5000]

    print("=" * 78)
    print("Sorting Benchmark — Insertion vs Selection vs Bubble")
    print("=" * 78)

    for n in sizes:
        inputs = generate_inputs(n)
        print(f"\n--- n = {n:,} ---")
        print(f"{'Sort':<12} {'Input':<10} {'Comparisons':>12} {'Swaps':>10} {'Time (ms)':>10}")
        print("-" * 58)

        for input_name, data in inputs.items():
            for name, sort_fn in sorts:
                comps, swaps, elapsed = benchmark(sort_fn, data)
                print(f"{name:<12} {input_name:<10} {comps:>12,} {swaps:>10,} {elapsed:>10.2f}")

    # Theoretical comparison count: n*(n-1)/2
    print("\n--- Theoretical Reference ---")
    for n in sizes:
        print(f"n={n:>6,}:  n(n-1)/2 = {n * (n - 1) // 2:>12,}")

    # Stability demonstration
    print("\n--- Stability Check ---")
    pairs = [(3, "A"), (1, "B"), (3, "C"), (2, "D"), (3, "E")]
    for name, sort_fn in sorts:
        data = list(pairs)
        sort_fn(data)
        print(f"{name}: {data}")


if __name__ == "__main__":
    main()
