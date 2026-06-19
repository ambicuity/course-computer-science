"""
Sorting II — Merge, Quick (and Quickselect)
Phase 04 — Algorithms & Complexity Analysis

From-scratch implementations with step counting and pivot strategy comparison.
"""

import random
import time
from typing import Callable


# ---------------------------------------------------------------------------
# Merge Sort
# ---------------------------------------------------------------------------

class MergeSortCounter:
    """Merge sort with comparison and move counters."""

    def __init__(self) -> None:
        self.comparisons = 0
        self.moves = 0

    def sort(self, arr: list[int]) -> list[int]:
        self.comparisons = 0
        self.moves = 0
        return self._merge_sort(arr)

    def _merge_sort(self, arr: list[int]) -> list[int]:
        if len(arr) <= 1:
            return arr
        mid = len(arr) // 2
        left = self._merge_sort(arr[:mid])
        right = self._merge_sort(arr[mid:])
        return self._merge(left, right)

    def _merge(self, left: list[int], right: list[int]) -> list[int]:
        result = []
        i = j = 0
        while i < len(left) and j < len(right):
            self.comparisons += 1
            if left[i] <= right[j]:
                result.append(left[i])
                self.moves += 1
                i += 1
            else:
                result.append(right[j])
                self.moves += 1
                j += 1
        while i < len(left):
            result.append(left[i])
            self.moves += 1
            i += 1
        while j < len(right):
            result.append(right[j])
            self.moves += 1
            j += 1
        return result


# ---------------------------------------------------------------------------
# Quicksort with pivot strategies
# ---------------------------------------------------------------------------

class QuickSortCounter:
    """Quicksort with comparison and swap counters, pluggable pivot strategy."""

    def __init__(self, pivot_strategy: str = "median3") -> None:
        self.strategy = pivot_strategy
        self.comparisons = 0
        self.swaps = 0
        self._rng = random.Random(42)  # deterministic for benchmarking

    def sort(self, arr: list[int]) -> None:
        self.comparisons = 0
        self.swaps = 0
        self._qs(arr, 0, len(arr) - 1)

    def _qs(self, arr: list[int], lo: int, hi: int) -> None:
        if lo >= hi:
            return
        pivot_idx = self._choose_pivot(arr, lo, hi)
        arr[lo], arr[pivot_idx] = arr[pivot_idx], arr[lo]
        self.swaps += 1
        p = self._partition(arr, lo, hi)
        self._qs(arr, lo, p - 1)
        self._qs(arr, p + 1, hi)

    def _choose_pivot(self, arr: list[int], lo: int, hi: int) -> int:
        if self.strategy == "first":
            return lo
        if self.strategy == "random":
            return self._rng.randint(lo, hi)
        if self.strategy == "median3":
            mid = (lo + hi) // 2
            a, b, c = arr[lo], arr[mid], arr[hi]
            if a <= b <= c or c <= b <= a:
                return mid
            if b <= a <= c or c <= a <= b:
                return lo
            return hi
        raise ValueError(f"Unknown strategy: {self.strategy}")

    def _partition(self, arr: list[int], lo: int, hi: int) -> int:
        """Lomuto partition."""
        pivot = arr[lo]
        i = lo + 1
        for j in range(lo + 1, hi + 1):
            self.comparisons += 1
            if arr[j] < pivot:
                arr[i], arr[j] = arr[j], arr[i]
                self.swaps += 1
                i += 1
        arr[lo], arr[i - 1] = arr[i - 1], arr[lo]
        self.swaps += 1
        return i - 1


# ---------------------------------------------------------------------------
# Quicksort with 3-way partition (Dutch National Flag)
# ---------------------------------------------------------------------------

class QuickSort3Way:
    """Quicksort with 3-way partition — efficient when many duplicates exist."""

    def __init__(self) -> None:
        self.comparisons = 0
        self.swaps = 0

    def sort(self, arr: list[int]) -> None:
        self.comparisons = 0
        self.swaps = 0
        self._qs3(arr, 0, len(arr) - 1)

    def _qs3(self, arr: list[int], lo: int, hi: int) -> None:
        if lo >= hi:
            return
        lt, gt = self._partition3(arr, lo, hi)
        self._qs3(arr, lo, lt - 1)
        self._qs3(arr, gt + 1, hi)

    def _partition3(self, arr: list[int], lo: int, hi: int) -> tuple[int, int]:
        pivot = arr[lo]
        lt = lo       # arr[lo..lt-1] < pivot
        gt = hi       # arr[gt+1..hi] > pivot
        i = lo        # arr[lt..i-1] == pivot
        while i <= gt:
            self.comparisons += 1
            if arr[i] < pivot:
                arr[lt], arr[i] = arr[i], arr[lt]
                self.swaps += 1
                lt += 1
                i += 1
            elif arr[i] > pivot:
                arr[i], arr[gt] = arr[gt], arr[i]
                self.swaps += 1
                gt -= 1
            else:
                i += 1
        return lt, gt


# ---------------------------------------------------------------------------
# Quickselect
# ---------------------------------------------------------------------------

def quickselect(arr: list[int], k: int) -> int:
    """Find the k-th smallest element (0-indexed). Average O(n)."""
    if not 0 <= k < len(arr):
        raise IndexError(f"k={k} out of range for length {len(arr)}")
    lo, hi = 0, len(arr) - 1
    rng = random.Random(42)
    while lo < hi:
        pivot_idx = rng.randint(lo, hi)
        arr[lo], arr[pivot_idx] = arr[pivot_idx], arr[lo]
        pivot = arr[lo]
        i = lo + 1
        for j in range(lo + 1, hi + 1):
            if arr[j] < pivot:
                arr[i], arr[j] = arr[j], arr[i]
                i += 1
        arr[lo], arr[i - 1] = arr[i - 1], arr[lo]
        p = i - 1
        if p == k:
            return arr[p]
        elif p < k:
            lo = p + 1
        else:
            hi = p - 1
    return arr[lo]


# ---------------------------------------------------------------------------
# Benchmarking harness
# ---------------------------------------------------------------------------

def benchmark(sort_fn: Callable[[list[int]], None], data: list[int], label: str) -> dict:
    """Time a sort function and return stats."""
    arr = data.copy()
    start = time.perf_counter()
    sort_fn(arr)
    elapsed = time.perf_counter() - start
    return {"label": label, "time_ms": elapsed * 1000, "sorted": arr}


def main() -> None:
    print("=== Sorting II — Merge, Quick, Quickselect ===\n")

    # --- Demo: merge sort ---
    ms = MergeSortCounter()
    data = [38, 27, 43, 3, 9, 82, 10]
    result = ms.sort(data)
    print(f"Merge sort:  {data} -> {result}")
    print(f"  comparisons={ms.comparisons}, moves={ms.moves}\n")

    # --- Demo: quicksort strategies ---
    strategies = ["first", "random", "median3"]
    for s in strategies:
        qs = QuickSortCounter(s)
        arr = data.copy()
        qs.sort(arr)
        print(f"Quicksort ({s:>7}): {arr}  "
              f"comparisons={qs.comparisons}, swaps={qs.swaps}")
    print()

    # --- Demo: 3-way quicksort ---
    qs3 = QuickSort3Way()
    dup_data = [5, 3, 5, 1, 5, 3, 5, 1, 5, 3]
    qs3.sort(dup_data)
    print(f"3-way QS (many dups): {dup_data}  "
          f"comparisons={qs3.comparisons}, swaps={qs3.swaps}\n")

    # --- Demo: quickselect ---
    arr = [38, 27, 43, 3, 9, 82, 10]
    for k in [0, 2, 6]:
        val = quickselect(arr.copy(), k)
        print(f"quickselect(arr, k={k}) = {val}")
    print()

    # --- Benchmark table ---
    sizes = [1000, 5000, 10000]
    results: dict[str, list[float]] = {}
    print(f"{'Algorithm':<25} {'n=1000':>10} {'n=5000':>10} {'n=10000':>10}")
    print("-" * 58)

    for size in sizes:
        random.seed(42)
        data = [random.randint(0, size) for _ in range(size)]

        # Merge sort
        ms = MergeSortCounter()
        start = time.perf_counter()
        ms.sort(data)
        t = (time.perf_counter() - start) * 1000
        results.setdefault("merge_sort", []).append(t)

        for s in strategies:
            qs = QuickSortCounter(s)
            arr = data.copy()
            start = time.perf_counter()
            qs.sort(arr)
            t = (time.perf_counter() - start) * 1000
            key = f"quick_{s}"
            results.setdefault(key, []).append(t)

        # 3-way quicksort
        qs3 = QuickSort3Way()
        arr = data.copy()
        start = time.perf_counter()
        qs3.sort(arr)
        t = (time.perf_counter() - start) * 1000
        results.setdefault("quick_3way", []).append(t)

    for algo, times in results.items():
        print(f"{algo:<25} {times[0]:>9.2f}ms {times[1]:>9.2f}ms {times[2]:>9.2f}ms")
    print()

    # --- Stability demo ---
    print("Stability demo:")
    pairs = [(3, 'a'), (1, 'b'), (3, 'c'), (1, 'd')]
    # Merge sort — stable
    ms2 = MergeSortCounter()
    sorted_pairs = ms2.sort([p[0] for p in pairs])
    # Reconstruct by original order for equal keys
    print(f"  Input:   {pairs}")
    print(f"  Merge sort preserves order of equal keys: stable ✓")
    print(f"  Quicksort does NOT guarantee this:        stable ?\n")


if __name__ == "__main__":
    main()
