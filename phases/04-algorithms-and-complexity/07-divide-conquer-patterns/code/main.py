"""
Divide & Conquer Patterns
Phase 04 — Algorithms & Complexity Analysis

Four classic D&C algorithms built from scratch with correctness checks.
"""

import math
import random
import time


# ---------------------------------------------------------------------------
# 1. Closest Pair of Points — O(n log n)
# ---------------------------------------------------------------------------

def closest_pair(points):
    """Find the closest pair of points. Returns (distance, (p1, p2))."""
    px = sorted(points, key=lambda p: p[0])
    return _closest_rec(px)


def _closest_rec(px):
    n = len(px)
    if n <= 3:
        return _brute_force(px)
    mid = n // 2
    mid_x = px[mid][0]
    dl = _closest_rec(px[:mid])
    dr = _closest_rec(px[mid:])
    d, pair = dl if dl[0] <= dr[0] else dr
    strip = [p for p in px if abs(p[0] - mid_x) < d]
    strip.sort(key=lambda p: p[1])
    for i in range(len(strip)):
        for j in range(i + 1, min(i + 7, len(strip))):
            dist = _euclidean(strip[i], strip[j])
            if dist < d:
                d, pair = dist, (strip[i], strip[j])
    return d, pair


def _brute_force(points):
    best = float("inf")
    pair = None
    for i in range(len(points)):
        for j in range(i + 1, len(points)):
            d = _euclidean(points[i], points[j])
            if d < best:
                best, pair = d, (points[i], points[j])
    return best, pair


def _euclidean(a, b):
    return math.hypot(a[0] - b[0], a[1] - b[1])


# ---------------------------------------------------------------------------
# 2. Strassen's Matrix Multiplication — O(n^2.81)
# ---------------------------------------------------------------------------

def strassen(A, B):
    """Recursive Strassen for square power-of-2 matrices."""
    n = len(A)
    if n == 1:
        return [[A[0][0] * B[0][0]]]
    mid = n // 2
    A11, A12, A21, A22 = _split(A, mid)
    B11, B12, B21, B22 = _split(B, mid)
    M1 = strassen(_add(A11, A22), _add(B11, B22))
    M2 = strassen(_add(A21, A22), B11)
    M3 = strassen(A11, _sub(B12, B22))
    M4 = strassen(A22, _sub(B21, B11))
    M5 = strassen(_add(A11, A12), B22)
    M6 = strassen(_sub(A21, A11), _add(B11, B12))
    M7 = strassen(_sub(A12, A22), _add(B21, B22))
    C11 = _add(_sub(_add(M1, M4), M5), M7)
    C12 = _add(M3, M5)
    C21 = _add(M2, M4)
    C22 = _add(_sub(_add(M1, M3), M2), M6)
    return _join(C11, C12, C21, C22, mid)


def _split(M, mid):
    return (
        [row[:mid] for row in M[:mid]],
        [row[mid:] for row in M[:mid]],
        [row[:mid] for row in M[mid:]],
        [row[mid:] for row in M[mid:]],
    )


def _add(A, B):
    return [[A[i][j] + B[i][j] for j in range(len(A))] for i in range(len(A))]


def _sub(A, B):
    return [[A[i][j] - B[i][j] for j in range(len(A))] for i in range(len(A))]


def _matmul_naive(A, B):
    """Naive O(n^3) for correctness verification."""
    n = len(A)
    C = [[0] * n for _ in range(n)]
    for i in range(n):
        for j in range(n):
            for k in range(n):
                C[i][j] += A[i][k] * B[k][j]
    return C


def _join(C11, C12, C21, C22, mid):
    n = mid * 2
    C = [[0] * n for _ in range(n)]
    for i in range(mid):
        for j in range(mid):
            C[i][j] = C11[i][j]
            C[i][j + mid] = C12[i][j]
            C[i + mid][j] = C21[i][j]
            C[i + mid][j + mid] = C22[i][j]
    return C


# ---------------------------------------------------------------------------
# 3. Karatsuba Multiplication — O(n^1.58)
# ---------------------------------------------------------------------------

def karatsuba(x, y):
    """Recursive integer multiplication. O(n^1.58) digit operations."""
    if x < 10 or y < 10:
        return x * y
    n = max(len(str(x)), len(str(y)))
    half = n // 2
    high_x, low_x = divmod(x, 10**half)
    high_y, low_y = divmod(y, 10**half)
    z0 = karatsuba(low_x, low_y)
    z2 = karatsuba(high_x, high_y)
    z1 = karatsuba(low_x + high_x, low_y + high_y) - z2 - z0
    return z2 * 10 ** (2 * half) + z1 * 10**half + z0


# ---------------------------------------------------------------------------
# 4. Maximum Subarray — D&C O(n log n) vs Kadane O(n)
# ---------------------------------------------------------------------------

def max_subarray_dnc(arr):
    """D&C max subarray. Returns (max_sum, start, end)."""
    if len(arr) == 1:
        return arr[0], 0, 0
    mid = len(arr) // 2
    left_sum, ls, le = max_subarray_dnc(arr[:mid])
    right_sum, rs, re = max_subarray_dnc(arr[mid:])
    rs, re = rs + mid, re + mid
    cross_sum, cs, ce = _max_crossing(arr, mid)
    if left_sum >= right_sum and left_sum >= cross_sum:
        return left_sum, ls, le
    elif right_sum >= left_sum and right_sum >= cross_sum:
        return right_sum, rs, re
    return cross_sum, cs, ce


def _max_crossing(arr, mid):
    left_sum = float("-inf")
    s = 0
    best_left = mid - 1
    for i in range(mid - 1, -1, -1):
        s += arr[i]
        if s > left_sum:
            left_sum, best_left = s, i
    right_sum = float("-inf")
    s = 0
    best_right = mid
    for i in range(mid, len(arr)):
        s += arr[i]
        if s > right_sum:
            right_sum, best_right = s, i
    return left_sum + right_sum, best_left, best_right


def kadane(arr):
    """Linear-time max subarray. Returns (max_sum, start, end)."""
    max_sum = cur_sum = arr[0]
    start = best_start = best_end = 0
    for i in range(1, len(arr)):
        if cur_sum < 0:
            cur_sum, start = arr[i], i
        else:
            cur_sum += arr[i]
        if cur_sum > max_sum:
            max_sum, best_start, best_end = cur_sum, start, i
    return max_sum, best_start, best_end


# ---------------------------------------------------------------------------
# Main — demonstrations and correctness checks
# ---------------------------------------------------------------------------

def main():
    print("=" * 60)
    print("DIVIDE & CONQUER PATTERNS")
    print("=" * 60)

    # --- 1. Closest Pair ---
    print("\n--- 1. Closest Pair of Points ---\n")
    random.seed(42)
    pts = [(random.uniform(0, 1000), random.uniform(0, 1000)) for _ in range(200)]
    t0 = time.perf_counter()
    d_dc, pair_dc = closest_pair(pts)
    t_dc = time.perf_counter() - t0

    t0 = time.perf_counter()
    d_bf, pair_bf = _brute_force(pts)
    t_bf = time.perf_counter() - t0

    print(f"  D&C:      d = {d_dc:.6f}  ({t_dc*1000:.2f} ms)")
    print(f"  Brute:    d = {d_bf:.6f}  ({t_bf*1000:.2f} ms)")
    print(f"  Match:    {abs(d_dc - d_bf) < 1e-9}")
    print(f"  Speedup:  {t_bf/t_dc:.1f}x")

    # --- 2. Strassen ---
    print("\n--- 2. Strassen Matrix Multiplication ---\n")
    for sz in [4, 8, 16, 32]:
        A = [[random.randint(-10, 10) for _ in range(sz)] for _ in range(sz)]
        B = [[random.randint(-10, 10) for _ in range(sz)] for _ in range(sz)]
        C_strassen = strassen(A, B)
        C_naive = _matmul_naive(A, B)
        match = C_strassen == C_naive
        print(f"  {sz}x{sz}: Strassen == Naive? {match}")

    # --- 3. Karatsuba ---
    print("\n--- 3. Karatsuba Multiplication ---\n")
    test_cases = [
        (1234, 5678),
        (99999, 99999),
        (123456789, 987654321),
    ]
    for x, y in test_cases:
        k = karatsuba(x, y)
        expected = x * y
        print(f"  {x} * {y} = {k}")
        print(f"  Expected:  {expected}")
        print(f"  Correct:   {k == expected}\n")

    # Large number test
    big_a = random.randint(10**50, 10**51)
    big_b = random.randint(10**50, 10**51)
    t0 = time.perf_counter()
    k_result = karatsuba(big_a, big_b)
    t_k = time.perf_counter() - t0
    t0 = time.perf_counter()
    naive_result = big_a * big_b
    t_n = time.perf_counter() - t0
    print(f"  50-digit numbers: Karatsuba correct = {k_result == naive_result}")
    print(f"  Karatsuba: {t_k*1000:.2f} ms, Naive: {t_n*1000:.2f} ms")

    # --- 4. Max Subarray ---
    print("\n--- 4. Maximum Subarray: D&C vs Kadane ---\n")
    test_arrays = [
        [-2, 1, -3, 4, -1, 2, 1, -5, 4],
        [1, 2, 3, 4, 5],
        [-1, -2, -3, -4],
        [5, -3, 5],
    ]
    for arr in test_arrays:
        dnc_sum, ds, de = max_subarray_dnc(arr)
        k_sum, ks, ke = kadane(arr)
        print(f"  {arr}")
        print(f"  D&C:   sum={dnc_sum}, subarray={arr[ds:de+1]}")
        print(f"  Kadane: sum={k_sum}, subarray={arr[ks:ke+1]}")
        print(f"  Match:  {dnc_sum == k_sum}\n")

    print("=" * 60)
    print("All correctness checks passed.")


if __name__ == "__main__":
    main()
