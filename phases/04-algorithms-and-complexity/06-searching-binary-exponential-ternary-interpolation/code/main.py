"""
Searching — Binary, Exponential, Ternary, Interpolation
Phase 04 — Algorithms & Complexity Analysis

From-scratch implementations of all major search variants.
"""

import bisect
import math
import random
import time


# ─── Binary Search ───────────────────────────────────────────────────────────


def binary_search(arr: list, target) -> int:
    """Iterative binary search. Returns index of target or -1."""
    lo, hi = 0, len(arr)
    while lo < hi:
        mid = lo + (hi - lo) // 2  # never (lo + hi) // 2
        if arr[mid] < target:
            lo = mid + 1
        else:
            hi = mid
    return lo if lo < len(arr) and arr[lo] == target else -1


def binary_search_rec(arr: list, target, lo: int = 0, hi: int = None) -> int:
    """Recursive binary search. Returns index of target or -1."""
    if hi is None:
        hi = len(arr)
    if lo >= hi:
        return lo if lo < len(arr) and arr[lo] == target else -1
    mid = lo + (hi - lo) // 2
    if arr[mid] < target:
        return binary_search_rec(arr, target, mid + 1, hi)
    return binary_search_rec(arr, target, lo, mid)


# ─── Lower / Upper Bound ────────────────────────────────────────────────────


def lower_bound(arr: list, target) -> int:
    """First index i where arr[i] >= target."""
    lo, hi = 0, len(arr)
    while lo < hi:
        mid = lo + (hi - lo) // 2
        if arr[mid] < target:
            lo = mid + 1
        else:
            hi = mid
    return lo


def upper_bound(arr: list, target) -> int:
    """First index i where arr[i] > target."""
    lo, hi = 0, len(arr)
    while lo < hi:
        mid = lo + (hi - lo) // 2
        if arr[mid] <= target:
            lo = mid + 1
        else:
            hi = mid
    return lo


# ─── Exponential Search ─────────────────────────────────────────────────────


def exponential_search(arr: list, target) -> int:
    """Search in an unbounded/infinite sorted sequence. O(log i) where i is the answer position."""
    if not arr:
        return -1
    if arr[0] == target:
        return 0
    bound = 1
    while bound < len(arr) and arr[bound] < target:
        bound *= 2
    lo = bound // 2
    hi = min(bound + 1, len(arr))
    idx = lower_bound(arr[lo:hi], target) + lo
    return idx if idx < len(arr) and arr[idx] == target else -1


# ─── Ternary Search (Unimodal Function Maximum) ─────────────────────────────


def ternary_search(f, lo: float, hi: float, eps: float = 1e-9) -> float:
    """Find the maximum of a unimodal function f on [lo, hi]."""
    while hi - lo > eps:
        m1 = lo + (hi - lo) / 3
        m2 = hi - (hi - lo) / 3
        if f(m1) < f(m2):
            lo = m1
        else:
            hi = m2
    return (lo + hi) / 2


# ─── Interpolation Search ───────────────────────────────────────────────────


def interpolation_search(arr: list, target) -> int:
    """Interpolation search. O(log log n) average on uniform data, O(n) worst."""
    lo, hi = 0, len(arr) - 1
    while lo <= hi and arr[lo] <= target <= arr[hi]:
        if arr[lo] == arr[hi]:
            if arr[lo] == target:
                return lo
            break
        # Linear interpolation to guess the position
        pos = lo + (target - arr[lo]) * (hi - lo) // (arr[hi] - arr[lo])
        if arr[pos] == target:
            return pos
        elif arr[pos] < target:
            lo = pos + 1
        else:
            hi = pos - 1
    return -1


# ─── Off-by-One Debugging Demonstrations ────────────────────────────────────


def demonstrate_off_by_one():
    """Show the four classic binary search bugs and their fixes."""

    arr = [1, 3, 5, 7, 9, 11, 13]

    # Bug 1: Overflow — (lo + hi) // 2 can overflow in fixed-width ints
    # In Python this is fine (big integers), but in C/Rust it's a real bug.
    lo, hi = 2_000_000_000, 2_000_000_001
    bad_mid = (lo + hi) // 2    # works in Python, overflows in C
    good_mid = lo + (hi - lo) // 2  # always safe
    assert bad_mid == good_mid  # Python handles big ints, but the pattern is wrong in principle

    # Bug 2: hi = len(arr) - 1 misses the last element
    arr2 = [1, 3, 5]
    target = 5
    lo, hi = 0, len(arr2)  # correct: hi is exclusive
    while lo < hi:
        mid = lo + (hi - lo) // 2
        if arr2[mid] < target:
            lo = mid + 1
        else:
            hi = mid
    assert lo == 2 and arr2[lo] == 5  # found!

    # Bug 3: lo = mid instead of lo = mid + 1 → infinite loop
    # arr = [1, 2], target = 2
    # mid = 0, arr[0]=1 < 2, lo=mid → lo stays 0 forever!
    # Fix: lo = mid + 1

    # Bug 4: Returning mid without checking equality
    # The invariant gives first index >= target, not exact match.
    arr3 = [1, 3, 5, 7]
    lo, hi = 0, len(arr3)
    while lo < hi:
        mid = lo + (hi - lo) // 2
        if arr3[mid] < 4:
            lo = mid + 1
        else:
            hi = mid
    # lo=2, arr3[2]=5, which is >= 4 but != 4
    assert arr3[lo] >= 4  # correct: first >= 4
    assert arr3[lo] != 4  # but 4 isn't in the array

    print("All off-by-one assertions passed.")


# ─── Benchmark ──────────────────────────────────────────────────────────────


def benchmark():
    """Compare search algorithms on various array sizes."""

    print(f"\n{'n':>12} {'binary':>12} {'lower_bnd':>12} {'exponential':>12} {'interpolation':>14}")
    print("-" * 66)

    for n in [1_000, 10_000, 100_000, 1_000_000, 10_000_000]:
        arr = list(range(0, n * 2, 2))  # even numbers: [0, 2, 4, ..., 2n-2]
        target = arr[n // 2]  # search for middle element

        # Binary search
        t0 = time.perf_counter()
        for _ in range(1000):
            binary_search(arr, target)
        t_binary = (time.perf_counter() - t0) / 1000

        # Lower bound
        t0 = time.perf_counter()
        for _ in range(1000):
            lower_bound(arr, target)
        t_lower = (time.perf_counter() - t0) / 1000

        # Exponential search
        t0 = time.perf_counter()
        for _ in range(1000):
            exponential_search(arr, target)
        t_exp = (time.perf_counter() - t0) / 1000

        # Interpolation search
        t0 = time.perf_counter()
        for _ in range(1000):
            interpolation_search(arr, target)
        t_interp = (time.perf_counter() - t0) / 1000

        print(f"{n:>12} {t_binary:>11.7f}s {t_lower:>11.7f}s {t_exp:>11.7f}s {t_interp:>13.7f}s")


# ─── Correctness Tests ──────────────────────────────────────────────────────


def test_all():
    """Run correctness assertions on all search variants."""

    # Binary search
    arr = [1, 3, 5, 7, 9, 11, 13]
    assert binary_search(arr, 7) == 3
    assert binary_search(arr, 1) == 0
    assert binary_search(arr, 13) == 6
    assert binary_search(arr, 6) == -1
    assert binary_search([], 5) == -1
    assert binary_search_rec(arr, 7) == 3
    assert binary_search_rec(arr, 6) == -1

    # Lower/upper bound
    arr2 = [1, 3, 5, 7, 7, 7, 9, 11]
    assert lower_bound(arr2, 7) == 3
    assert upper_bound(arr2, 7) == 6
    assert lower_bound(arr2, 6) == 3  # first >= 6 is 7 at index 3
    assert upper_bound(arr2, 6) == 3  # first > 6 is also 7 at index 3
    assert lower_bound(arr2, 0) == 0
    assert upper_bound(arr2, 12) == 8

    # Verify against Python's bisect
    for t in range(0, 13):
        assert lower_bound(arr2, t) == bisect.bisect_left(arr2, t), f"lower_bound failed for {t}"
        assert upper_bound(arr2, t) == bisect.bisect_right(arr2, t), f"upper_bound failed for {t}"

    # Exponential search
    assert exponential_search(arr, 7) == 3
    assert exponential_search(arr, 1) == 0
    assert exponential_search(arr, 13) == 6
    assert exponential_search(arr, 6) == -1
    assert exponential_search([], 5) == -1

    # Ternary search — maximize f(x) = -(x-3)^2 + 10, peak at x=3
    f = lambda x: -(x - 3) ** 2 + 10
    peak = ternary_search(f, 0, 6)
    assert abs(peak - 3.0) < 1e-6, f"ternary search found {peak}, expected ~3.0"

    # Interpolation search
    uniform = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100]
    assert interpolation_search(uniform, 50) == 4
    assert interpolation_search(uniform, 10) == 0
    assert interpolation_search(uniform, 100) == 9
    assert interpolation_search(uniform, 55) == -1
    assert interpolation_search([], 5) == -1

    # Off-by-one edge cases
    assert binary_search([5], 5) == 0
    assert binary_search([5], 3) == -1
    assert binary_search([1, 2], 2) == 1
    assert lower_bound([1], 0) == 0
    assert lower_bound([1], 2) == 1

    print("All tests passed.")


def main() -> None:
    test_all()
    demonstrate_off_by_one()
    benchmark()


if __name__ == "__main__":
    main()
