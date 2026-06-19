# Searching — Binary, Exponential, Ternary, Interpolation

> Binary search is the single most important algorithm after sorting — and getting the off-by-one right is a rite of passage.

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 04 lessons 01–05
**Time:** ~60 minutes

## Learning Objectives

- Implement binary search from scratch (iterative + recursive) and debug the off-by-one errors that plague every first attempt
- Build lower_bound / upper_bound — the real workhorses behind Python's `bisect` and Rust's `partition_point`
- Analyze exponential, ternary, and interpolation search: when each beats binary and when it doesn't

## The Problem

A sorted array of 10 million elements: linear search takes 10 million comparisons, binary search takes at most 24. Every language runtime, database index, and OS scheduler relies on binary search or its variants. But it's deceptively hard to write correctly — the first bug-free version wasn't published until 1962, sixteen years after invention.

## The Concept

### Binary Search: The Loop Invariant

Maintain pointers `lo`, `hi` such that `arr[0..lo) < target` and `arr[hi..n) ≥ target`. Pick `mid = lo + (hi - lo) // 2`, compare, shrink. When `lo == hi`, `lo` is the first position where `arr[pos] ≥ target`.

```
arr = [1, 3, 5, 7, 9, 11, 13],  target = 6

lo=0, hi=7, mid=3, arr[3]=7 > 6  → hi=3
lo=0, hi=3, mid=1, arr[1]=3 < 6  → lo=2
lo=2, hi=3, mid=2, arr[2]=5 < 6  → lo=3
lo=3, hi=3 → stop. lo=3 is the first index where arr[i] ≥ 6.
```

### Lower Bound vs Upper Bound

- **lower_bound(target):** first index `i` where `arr[i] ≥ target` — insertion point preserving sort order
- **upper_bound(target):** first index `i` where `arr[i] > target` — after all copies

Single-character difference in comparison (`<` vs `≤`). Range `[lower_bound(x), upper_bound(x))` gives every copy of x.

### Exponential Search

For unknown-length arrays, probe 1, 2, 4, 8, ... until overshooting, then binary search the last range. **O(log i)** where i is the answer position — beats O(log n) when target is near the front.

### Ternary Search

Maximize a unimodal function on a continuous domain: pick two interior points, evaluate both, discard one third. Two comparisons per step, discards 1/3 — converges slower than binary but applies where binary can't (continuous domains).

### Interpolation Search

On uniformly distributed data, guess position via linear interpolation. **O(log log n)** average, **O(n)** worst on pathological distributions.

## Build It

### Step 1: Binary Search (Iterative + Recursive)

```python
def binary_search(arr, target):
    lo, hi = 0, len(arr)
    while lo < hi:
        mid = lo + (hi - lo) // 2
        if arr[mid] < target:
            lo = mid + 1
        else:
            hi = mid
    return lo if lo < len(arr) and arr[lo] == target else -1
```

The recursive version mirrors the same invariant — see `code/main.py` for both forms.

### Step 2: Lower Bound, Upper Bound

```python
def lower_bound(arr, target):
    lo, hi = 0, len(arr)
    while lo < hi:
        mid = lo + (hi - lo) // 2
        if arr[mid] < target:    # < for lower_bound
            lo = mid + 1
        else:
            hi = mid
    return lo
```

`upper_bound` is identical except `arr[mid] <= target` — the entire distinction is one character.

### Step 3: Exponential Search

Double the bound until `arr[bound] >= target`, then binary search `[bound/2, bound]`. See `code/main.py` for the full implementation.

### Step 4: Ternary Search

Split interval into thirds at m1, m2. If `f(m1) < f(m2)`, maximum is in right two thirds — set `lo = m1`. Otherwise set `hi = m2`. Repeat until interval < eps. See `code/main.py`.

### Step 5: Interpolation Search

Guess position: `pos = lo + (target - arr[lo]) * (hi - lo) / (arr[hi] - arr[lo])`. Handle division-by-zero when `arr[lo] == arr[hi]`. See `code/main.py`.

### The Four Classic Off-by-One Bugs

1. **`mid = (lo + hi) // 2`** — overflows in fixed-width ints. Always `lo + (hi - lo) // 2`.
2. **`hi = len(arr) - 1`** — misses the last element. Upper bound is exclusive; use `len(arr)`.
3. **`lo = mid`** instead of `lo = mid + 1` — infinite loop when `hi = lo + 1`.
4. **Returning mid without checking equality** — invariant gives first *≥ target*, not exact match.

## Use It

### Python: `bisect` module

```python
import bisect
arr = [1, 3, 5, 7, 7, 7, 9, 11]
bisect.bisect_left(arr, 7)   # → 3  (lower_bound)
bisect.bisect_right(arr, 7)  # → 6  (upper_bound)
```

### Rust: `partition_point`

```rust
let arr = [1, 3, 5, 7, 7, 7, 9, 11];
arr.partition_point(|&x| x < 7)   // → 3  (lower_bound)
arr.partition_point(|&x| x <= 7)  // → 6  (upper_bound)
```

`partition_point` is more general: any monotonic predicate, not just equality.

### What production does that yours doesn't

- `bisect` runs in C — tight loop, no Python overhead
- Rust's `partition_point` uses branchless comparisons
- Both handle empty arrays and boundaries without special-casing

## Read the Source

- [CPython `bisectmodule.c`](https://github.com/python/cpython/blob/main/Modules/_bisectmodule.c) — tight C while loop
- [Rust `slice::partition_point`](https://doc.rust-lang.org/src/core/slice/mod.rs.html) — generic predicate binary search

## Ship It

The artifact is a generic search library in `outputs/` with binary_search, lower_bound, upper_bound, exponential_search, ternary_search, and interpolation_search.

## Exercises

1. **Easy.** Binary search on a **rotated sorted array** (e.g., `[4, 5, 6, 7, 0, 1, 2]`) — find target in O(log n).
2. **Medium.** Find a **peak element** using ternary search — a peak is ≥ its neighbors.
3. **Hard.** Implement interpolation search and prove O(log log n) expected complexity under uniform distribution. Provide a counterexample degrading to O(n).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Binary search | "Divide and conquer on sorted data" | Half-interval invariant [lo, hi), halve each step — O(log n) |
| Lower bound | "First occurrence" | Smallest i where arr[i] ≥ target |
| Upper bound | "Last occurrence + 1" | Smallest i where arr[i] > target |
| Exponential search | "Doubling search" | Probe 1, 2, 4, 8, ... then binary search the range — O(log i) |
| Ternary search | "Trinary search" | Discard one third per step — unimodal continuous optimization |
| Interpolation search | "Smart binary search" | Linear interpolation guess — O(log log n) avg, O(n) worst |
| Unimodal function | "Single-humped" | Increases then decreases over its domain |

## Further Reading

- [Bentley, *Programming Pearls*, Column 4](https://www.cs.bell-labs.com/cm/cs/pearles/) — binary search correctness
- [Knuth, *TAOCP* Vol. 3, §6.2.1](https://www-cs-faculty.stanford.edu/~knuth/taocp.html) — exhaustive search analysis
- [Bloch, "Nearly All Binary Searches are Broken" (2006)](https://research.google/blog/extra-extra-read-all-about-it-nearly-all-binary-searches-and-mergesorts-are-broken/) — the overflow bug in production for decades
