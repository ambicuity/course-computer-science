# Divide & Conquer Patterns

> The recurring move: split the problem in half, solve each half, glue the answers together.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 04 lessons 01–06
**Time:** ~60 minutes

## Learning Objectives

- Identify the three-step D&C skeleton (divide, conquer, combine) and map it onto recurrence relations
- Apply the Master Theorem to classify D&C recurrences and predict running times
- Build four classic D&C algorithms from scratch and verify correctness against brute force
- Know when D&C wins and when the overhead isn't worth it

## The Problem

Merge sort and quicksort are O(n log n) — D&C for free. But the paradigm extends far beyond sorting. Matrix multiplication, integer multiplication, closest-pair geometry, and maximum subarray all admit D&C solutions that beat naive approaches by exploiting problem structure.

## The Concept

### The D&C Skeleton

1. **Divide** — split the problem into independent subproblems of the same type.
2. **Conquer** — solve each subproblem recursively (base case: brute-force small instances).
3. **Combine** — merge subproblem solutions into a solution for the original.

The recurrence is always `T(n) = a·T(n/b) + f(n)` where a = subproblem count, b = size reduction, f(n) = divide+combine cost.

### When D&C Beats Naive

| Problem | Naive | D&C | Saving |
|---------|-------|-----|--------|
| Matrix multiply | O(n³) | O(n^2.81) Strassen | Wins for large n |
| Integer multiply | O(n²) | O(n^1.58) Karatsuba | Wins for 100+ digit numbers |
| Closest pair | O(n²) | O(n log n) | Always wins |
| Max subarray | O(n²) | O(n log n), but Kadane's O(n) is better | D&C not best here |

Key insight: D&C doesn't always give the optimal algorithm. Always check if a simpler approach exists.

### Closest Pair of Points — O(n log n)

**Problem:** Given n points, find the pair with minimum distance.

**D&C:** Sort by x. Divide into halves at median x. Recurse. For the combine, only points within distance d of the midline matter — sort strip by y, each point checks ≤6 neighbours.

**Recurrence:** T(n) = 2T(n/2) + O(n) → **O(n log n)**.

### Strassen's Matrix Multiplication — O(n^2.81)

**Naive:** O(n³). **Naive D&C:** 8 recursive multiplies on n/2 blocks → T(n) = 8T(n/2) + O(n²) → n³. No improvement.

**Strassen's trick:** 7 multiplications via clever linear combinations of sub-blocks.

**Recurrence:** T(n) = 7T(n/2) + O(n²) → n^(log_2 7) ≈ **n^2.807**.

### Karatsuba Multiplication — O(n^1.58)

**Naive D&C:** 4 sub-multiplications → T(n) = 4T(n/2) + O(n) → n². No improvement.

**Karatsuba's trick:** `x1y0 + x0y1 = (x1+x0)(y1+y0) - x1y1 - x0y0` — one multiply instead of two.

**Recurrence:** T(n) = 3T(n/2) + O(n) → n^(log_2 3) ≈ **n^1.585**.

### Maximum Subarray — D&C O(n log n) vs Kadane's O(n)

**D&C:** Split at midpoint, find max in each half plus crossing, return best of three. T(n) = 2T(n/2) + O(n) → **O(n log n)**.

**Kadane's:** Single pass, reset running sum on negative → **O(n)**.

## Build It

### Step 1: Closest Pair

```python
def closest_pair(points):
    px = sorted(points, key=lambda p: p[0])
    return _closest_rec(px)

def _closest_rec(px):
    if len(px) <= 3: return _brute_force(px)
    mid, mid_x = len(px) // 2, px[len(px)//2][0]
    dl, dr = _closest_rec(px[:mid]), _closest_rec(px[mid:])
    d, pair = dl if dl[0] <= dr[0] else dr
    strip = sorted((p for p in px if abs(p[0]-mid_x) < d), key=lambda p: p[1])
    for i in range(len(strip)):
        for j in range(i+1, min(i+7, len(strip))):
            dist = math.hypot(strip[i][0]-strip[j][0], strip[i][1]-strip[j][1])
            if dist < d: d, pair = dist, (strip[i], strip[j])
    return d, pair
```

### Step 2: Strassen

```python
def strassen(A, B):
    n = len(A)
    if n == 1: return [[A[0][0]*B[0][0]]]
    mid = n // 2
    A11,A12,A21,A22 = _split(A,mid); B11,B12,B21,B22 = _split(B,mid)
    M1=strassen(_add(A11,A22),_add(B11,B22))
    M2=strassen(_add(A21,A22),B11)
    M3=strassen(A11,_sub(B12,B22))
    M4=strassen(A22,_sub(B21,B11))
    M5=strassen(_add(A11,A12),B22)
    M6=strassen(_sub(A21,A11),_add(B11,B12))
    M7=strassen(_sub(A12,A22),_add(B21,B22))
    C11=_add(_sub(_add(M1,M4),M5),M7)
    C12=_add(M3,M5); C21=_add(M2,M4)
    C22=_add(_sub(_add(M1,M3),M2),M6)
    return _join(C11,C12,C21,C22,mid)
```

### Step 3: Karatsuba

```python
def karatsuba(x, y):
    if x < 10 or y < 10: return x * y
    n = max(len(str(x)), len(str(y)))
    half = n // 2
    hi_x, lo_x = divmod(x, 10**half)
    hi_y, lo_y = divmod(y, 10**half)
    z0 = karatsuba(lo_x, lo_y)
    z2 = karatsuba(hi_x, hi_y)
    z1 = karatsuba(lo_x+hi_x, lo_y+hi_y) - z2 - z0
    return z2*10**(2*half) + z1*10**half + z0
```

### Step 4: Max Subarray

```python
def max_subarray_dnc(arr):
    if len(arr) == 1: return arr[0], 0, 0
    mid = len(arr) // 2
    ls, l_s, l_e = max_subarray_dnc(arr[:mid])
    rs, r_s, r_e = max_subarray_dnc(arr[mid:]); r_s += mid; r_e += mid
    cs, c_s, c_e = _max_crossing(arr, mid)
    if ls >= rs and ls >= cs: return ls, l_s, l_e
    if rs >= ls and rs >= cs: return rs, r_s, r_e
    return cs, c_s, c_e

def kadane(arr):
    max_sum = cur_sum = arr[0]; start = bs = be = 0
    for i in range(1, len(arr)):
        if cur_sum < 0: cur_sum, start = arr[i], i
        else: cur_sum += arr[i]
        if cur_sum > max_sum: max_sum, bs, be = cur_sum, start, i
    return max_sum, bs, be
```

## Use It

D&C is everywhere once you know the pattern:

- **FFT (Cooley-Tukey):** Splits DFT of size n into two DFTs of size n/2 → O(n log n). Engine behind fast polynomial multiplication.
- **Merge sort / Quicksort:** Both D&C — merge sort splits+merges, quicksort splits by partition.
- **Counting inversions:** Modified merge sort counts cross-inversions during merge → O(n log n).
- **Python's `sorted()`:** Uses Timsort (adaptive merge sort) — the D&C pattern runs millions of times daily.

## Read the Source

- **CPython Timsort:** `Objects/listsort.txt` — merge-sort foundation of Python's built-in sort.

## Ship It

The artifact is a **D&C pattern library** in `outputs/` bundling all four algorithms with correctness tests.

## Exercises

1. **Easy.** Implement polynomial multiplication via D&C: split P(x), compute 3 sub-multiplications Karatsuba-style, verify against naive convolution.
2. **Medium.** Prove Strassen's recurrence T(n) = 7T(n/2) + O(n²) gives O(n^2.81). Show each Master Theorem step.
3. **Hard.** Solve the skyline problem using D&C: given buildings as (left, height, right), recursively split, compute skylines, merge in O(n).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Divide and Conquer | "Split and recurse" | Three-step paradigm: divide, solve recursively, combine |
| Master Theorem | "Tells you the answer" | Classifies T(n) = aT(n/b) + f(n) by comparing f(n) to n^(log_b(a)) |
| Strassen's trick | "7 multiplies instead of 8" | One fewer recursive call via algebraic identities: O(n³) → O(n^2.81) |
| Karatsuba's trick | "3 multiplies instead of 4" | Cross term identity saves one multiply: O(n²) → O(n^1.58) |
| Strip check | "Clever part of closest pair" | Only ~6 neighbours per point keeps the combine step O(n) |
| Kadane's algorithm | "Greedy version" | O(n) single-pass that beats D&C — reset running sum on negative |

## Further Reading

- Cormen et al., *Introduction to Algorithms*, Ch. 4 and Ch. 33.
- Skiena, *The Algorithm Design Manual*, Ch. 5.
- Volker Strassen, "Gaussian Elimination is not Optimal," 1969.
- Karatsuba & Ofman, "Multiplication of Multidigit Numbers on Automata," 1963.
