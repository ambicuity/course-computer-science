"""
Sorting III — Heap, Intro, Tim
Phase 04 — Algorithms & Complexity Analysis

Heap sort, simplified Tim sort, and a comparison harness for
all sorting algorithms from lessons 02–04.
"""

import random
import time
from typing import Callable


# ---------------------------------------------------------------------------
# Lesson 02 sorts (included for comparison harness)
# ---------------------------------------------------------------------------

def insertion_sort(arr: list) -> list:
    a = arr[:]
    for i in range(1, len(a)):
        key = a[i]
        j = i - 1
        while j >= 0 and a[j] > key:
            a[j + 1] = a[j]
            j -= 1
        a[j + 1] = key
    return a


def selection_sort(arr: list) -> list:
    a = arr[:]
    n = len(a)
    for i in range(n):
        min_idx = i
        for j in range(i + 1, n):
            if a[j] < a[min_idx]:
                min_idx = j
        a[i], a[min_idx] = a[min_idx], a[i]
    return a


def bubble_sort(arr: list) -> list:
    a = arr[:]
    n = len(a)
    for i in range(n):
        swapped = False
        for j in range(0, n - i - 1):
            if a[j] > a[j + 1]:
                a[j], a[j + 1] = a[j + 1], a[j]
                swapped = True
        if not swapped:
            break
    return a


# ---------------------------------------------------------------------------
# Lesson 03 sorts (included for comparison harness)
# ---------------------------------------------------------------------------

def merge_sort(arr: list) -> list:
    if len(arr) <= 1:
        return arr[:]
    mid = len(arr) // 2
    left = merge_sort(arr[:mid])
    right = merge_sort(arr[mid:])
    return _merge_sorted(left, right)


def _merge_sorted(left: list, right: list) -> list:
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


def quick_sort(arr: list) -> list:
    a = arr[:]
    _quick_sort(a, 0, len(a) - 1)
    return a


def _quick_sort(a: list, lo: int, hi: int) -> None:
    if lo < hi:
        p = _partition(a, lo, hi)
        _quick_sort(a, lo, p - 1)
        _quick_sort(a, p + 1, hi)


def _partition(a: list, lo: int, hi: int) -> int:
    pivot_idx = random.randint(lo, hi)
    a[pivot_idx], a[hi] = a[hi], a[pivot_idx]
    pivot = a[hi]
    i = lo - 1
    for j in range(lo, hi):
        if a[j] <= pivot:
            i += 1
            a[i], a[j] = a[j], a[i]
    a[i + 1], a[hi] = a[hi], a[i + 1]
    return i + 1


# ---------------------------------------------------------------------------
# Lesson 04: Heap Sort
# ---------------------------------------------------------------------------

def heap_sort(arr: list) -> list:
    a = arr[:]
    n = len(a)

    def sift_down(start: int, end: int) -> None:
        root = start
        while 2 * root + 1 <= end:
            child = 2 * root + 1
            swap = root
            if a[child] > a[swap]:
                swap = child
            if child + 1 <= end and a[child + 1] > a[swap]:
                swap = child + 1
            if swap == root:
                return
            a[root], a[swap] = a[swap], a[root]
            root = swap

    # Bottom-up heapify: O(n)
    for i in range(n // 2 - 1, -1, -1):
        sift_down(i, n - 1)

    # Extract max: O(n log n)
    for end in range(n - 1, 0, -1):
        a[0], a[end] = a[end], a[0]
        sift_down(0, end - 1)

    return a


# ---------------------------------------------------------------------------
# Lesson 04: Simplified Tim Sort
# ---------------------------------------------------------------------------

def _insertion_sort_range(a: list, lo: int, hi: int) -> None:
    for i in range(lo + 1, hi + 1):
        key = a[i]
        j = i - 1
        while j >= lo and a[j] > key:
            a[j + 1] = a[j]
            j -= 1
        a[j + 1] = key


def _merge_inplace(a: list, lo: int, mid: int, hi: int) -> None:
    left = a[lo:mid + 1]
    right = a[mid + 1:hi + 1]
    i = j = 0
    k = lo
    while i < len(left) and j < len(right):
        if left[i] <= right[j]:
            a[k] = left[i]
            i += 1
        else:
            a[k] = right[j]
            j += 1
        k += 1
    while i < len(left):
        a[k] = left[i]
        i += 1
        k += 1
    while j < len(right):
        a[k] = right[j]
        j += 1
        k += 1


def _compute_min_run(n: int) -> int:
    r = 0
    while n >= 32:
        r |= n & 1
        n >>= 1
    return n + r


def tim_sort_simplified(arr: list) -> list:
    a = arr[:]
    n = len(a)
    if n <= 1:
        return a

    min_run = _compute_min_run(n)

    # Phase 1: Detect natural runs and extend short ones
    runs = []
    i = 0
    while i < n:
        run_start = i

        # Detect descending run
        if i + 1 < n and a[i] > a[i + 1]:
            while i + 1 < n and a[i] > a[i + 1]:
                i += 1
            a[run_start:i + 1] = reversed(a[run_start:i + 1])
        else:
            # Ascending run
            while i + 1 < n and a[i] <= a[i + 1]:
                i += 1

        run_end = i
        # Extend short runs with insertion sort
        if run_end - run_start + 1 < min_run:
            force_end = min(run_start + min_run - 1, n - 1)
            _insertion_sort_range(a, run_start, force_end)
            run_end = force_end

        runs.append((run_start, run_end))
        i = run_end + 1

    # Phase 2: Merge runs in pairs
    while len(runs) > 1:
        new_runs = []
        for j in range(0, len(runs), 2):
            if j + 1 < len(runs):
                lo, mid = runs[j]
                _, hi = runs[j + 1]
                _merge_inplace(a, lo, mid, hi)
                new_runs.append((lo, hi))
            else:
                new_runs.append(runs[j])
        runs = new_runs

    return a


# ---------------------------------------------------------------------------
# Comparison Harness
# ---------------------------------------------------------------------------

def generate_inputs(n: int) -> dict[str, list[int]]:
    return {
        "random":        random.sample(range(n * 2), n),
        "sorted":        list(range(n)),
        "reverse":       list(range(n, 0, -1)),
        "nearly_sorted": _nearly_sorted(n),
        "few_unique":    [random.randint(0, 5) for _ in range(n)],
    }


def _nearly_sorted(n: int) -> list[int]:
    a = list(range(n))
    for _ in range(n // 10):
        i = random.randint(0, n - 2)
        a[i], a[i + 1] = a[i + 1], a[i]
    return a


def benchmark(
    sorts: dict[str, Callable[[list], list]],
    sizes: list[int],
    repeats: int = 3,
) -> None:
    patterns = ["random", "sorted", "reverse", "nearly_sorted", "few_unique"]
    header = f"{'Algorithm':<18}"
    for n in sizes:
        header += f"  n={n:<7}"
    print(header)
    print("-" * len(header))

    for name, sort_fn in sorts.items():
        row = f"{name:<18}"
        for n in sizes:
            total = 0.0
            for _ in range(repeats):
                data = generate_inputs(n)
                t0 = time.perf_counter()
                for p in patterns:
                    sort_fn(data[p])
                elapsed = time.perf_counter() - t0
                total += elapsed
            avg = total / repeats
            row += f"  {avg:<9.4f}"
        print(row)


def correctness_check() -> None:
    sorts = {
        "insertion_sort":  insertion_sort,
        "selection_sort":  selection_sort,
        "bubble_sort":     bubble_sort,
        "merge_sort":      merge_sort,
        "quick_sort":      quick_sort,
        "heap_sort":       heap_sort,
        "tim_simplified":  tim_sort_simplified,
    }
    test_cases = [
        [],
        [1],
        [2, 1],
        [3, 1, 2],
        [5, 4, 3, 2, 1],
        list(range(20)),
        [3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5],
    ]
    for name, sort_fn in sorts.items():
        for tc in test_cases:
            expected = sorted(tc)
            result = sort_fn(tc)
            assert result == expected, (
                f"{name} failed on {tc}: got {result}, expected {expected}"
            )
    print("All correctness checks passed.")


def main() -> None:
    print("=== Correctness Check ===")
    correctness_check()

    print()
    print("=== Benchmark (seconds, averaged over 5 patterns × 3 repeats) ===")
    sorts = {
        "insertion_sort":  insertion_sort,
        "selection_sort":  selection_sort,
        "bubble_sort":     bubble_sort,
        "merge_sort":      merge_sort,
        "quick_sort":      quick_sort,
        "heap_sort":       heap_sort,
        "tim_simplified":  tim_sort_simplified,
    }
    benchmark(sorts, sizes=[500, 1000, 2000])


if __name__ == "__main__":
    main()
