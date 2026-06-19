# Sorting III — Heap, Intro, Tim

> Heap sort gives you O(n log n) with no extra memory. Intro sort is what C++ actually uses. Tim sort is what Python actually uses. Today you build all three.

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 04 lessons 01–03
**Time:** ~75 minutes

## Learning Objectives

- Build heap sort from scratch using bottom-up heapify and understand why it is O(n).
- Understand intro sort's hybrid strategy and why it guarantees O(n log n) worst case.
- Implement a simplified Tim sort that detects natural runs and merges them.
- Compare cache performance across the sorting family from lessons 02–04.

## The Problem

Lessons 02–03 gave you insertion sort (O(n²)), merge sort (O(n log n) but O(n) extra space), and quicksort (O(n log n) average, O(n²) worst case). Production systems need O(n log n) worst case *and* good cache behavior *and* minimal extra memory. That gap is exactly what heap sort, intro sort, and Tim sort fill — and they are the algorithms under the hood of real standard libraries.

## The Concept

### Heap Sort

A **max-heap** is a complete binary tree where every parent ≥ its children. Stored as an array, node `i` has children at `2i+1` and `2i+2`.

**Bottom-up heapify** builds a heap in O(n), not O(n log n). The key insight: leaves (the bottom half of the array) are already valid heaps of size 1. You only need to sift down the internal nodes, and most of them are near the bottom where sift-down is cheap.

Summing the work across all levels:

```
Level h-1: n/2 nodes × 1 work  = n/2
Level h-2: n/4 nodes × 2 work  = n/2
Level h-3: n/8 nodes × 3 work  = 3n/8
...
Level 0:   1 node    × h work   = h
```

The sum converges to ≤ 2n (geometric series bound), so bottom-up heapify is O(n).

**Sort phase:** repeatedly extract the max (swap root with last element, shrink heap, sift down). Each of the n extractions costs O(log n), giving O(n log n) total. Properties: **in-place** (O(1) extra memory), **not stable**, **poor cache locality** (parent-child jumps scatter access).

### Intro Sort (Introspective Sort)

Intro sort is what C++ `std::sort` uses. The idea:

1. Start with quicksort (cache-friendly, fast average case).
2. Track recursion depth. If depth exceeds `2 ⌊log₂ n⌋`, the input is likely adversarial — switch to heap sort.
3. For tiny partitions (≤ 16 elements), switch to insertion sort.

This guarantees O(n log n) worst case while keeping quicksort's average-case cache performance.

### Tim Sort

Tim sort is what Python's `sorted()` uses. It exploits the fact that real-world data often contains pre-sorted **runs**.

1. **Run detection:** scan left to right, identifying naturally ascending (or descending, reversed) contiguous sequences.
2. **Minimum run length:** if a natural run is shorter than `min_run` (32–64), extend it using insertion sort.
3. **Merge with a stack invariant:** push runs onto a stack, merging adjacent runs to keep costs balanced.
4. **Galloping mode:** during a merge, if one run consistently "wins," switch to exponential search to copy chunks.

Tim sort is O(n) on already-sorted data and O(n log n) worst case. Its stability and run-awareness make it ideal for real-world sorting.

## Build It

### Step 1: Heap Sort with Bottom-Up Heapify

```python
def heap_sort(arr):
    a, n = arr[:], len(a)

    def sift_down(start, end):
        root = start
        while 2 * root + 1 <= end:
            child, swap = 2 * root + 1, root
            if a[child] > a[swap]: swap = child
            if child + 1 <= end and a[child + 1] > a[swap]: swap = child + 1
            if swap == root: return
            a[root], a[swap] = a[swap], a[root]
            root = swap

    for i in range(n // 2 - 1, -1, -1):       # build heap: O(n)
        sift_down(i, n - 1)
    for end in range(n - 1, 0, -1):            # extract max: O(n log n)
        a[0], a[end] = a[end], a[0]
        sift_down(0, end - 1)
    return a
```

### Step 2: Simplified Tim Sort

```python
def tim_sort_simplified(arr):
    a, n = arr[:], len(a)
    min_run = compute_min_run(n)    # 32-64 range

    runs, i = [], 0
    while i < n:
        start = i
        # Detect ascending or descending run
        if a[i] > a[i + 1]:
            while i + 1 < n and a[i] > a[i + 1]: i += 1
            a[start:i+1] = reversed(a[start:i+1])
        else:
            while i + 1 < n and a[i] <= a[i + 1]: i += 1
        # Extend short runs with insertion sort
        if i - start + 1 < min_run:
            i = min(start + min_run - 1, n - 1)
            insertion_sort(a, start, i)
        runs.append((start, i))
        i += 1

    # Merge runs pairwise until one remains
    while len(runs) > 1:
        new_runs = []
        for j in range(0, len(runs), 2):
            if j + 1 < len(runs):
                lo, mid = runs[j]; _, hi = runs[j+1]
                merge(a, lo, mid, hi)
                new_runs.append((lo, hi))
            else: new_runs.append(runs[j])
        runs = new_runs
    return a
```

Full implementations (with helpers, correctness tests, and benchmark harness) are in `code/main.py` and `code/main.rs`.

## Use It

**Python:** `sorted()` and `list.sort()` both use Tim sort. The production version adds galloping mode, binary insertion sort, and a sophisticated merge-stack invariant.

**C++:** `std::sort` uses intro sort. `std::stable_sort` uses merge sort. `std::partial_sort` uses heap sort internally.

**Rust:** `sort()` uses pdqsort (pattern-defeating quicksort) — a hybrid with intro sort's philosophy. `sort_unstable()` uses pdqsort directly.

**Java:** `Arrays.sort()` for primitives uses dual-pivot quicksort; for objects (stability required), it uses Tim sort.

| Property | Heap Sort | Intro Sort | Tim Sort |
|----------|-----------|------------|----------|
| Worst case | O(n log n) | O(n log n) | O(n log n) |
| Average case | O(n log n) | O(n log n) | O(n) to O(n log n) |
| Space | O(1) | O(log n) stack | O(n) |
| Stable | No | No | Yes |
| Cache friendly | Poor | Good | Moderate |
| Best on | Memory-constrained | Random data | Real-world / partially sorted |

## Read the Source

- CPython `Objects/listobject.c` — the C implementation of Tim sort. Look for `merge_lo`/`merge_hi` for galloping mode.
- `libstdc++` `bits/stl_algo.h` — `std::sort` intro sort. Look for `__introsort_loop` for the depth guard.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A comprehensive sorting algorithm comparison harness** — all sorts from lessons 02–04 benchmarked on sorted, reverse-sorted, random, and nearly-sorted inputs.

## Exercises

1. **Easy** — Implement bottom-up heapify. Count the number of comparisons and show it is ≤ 2n for any input size n. Run it on arrays of size 100, 1000, 10000 and verify the count.

2. **Medium** — Implement a simplified Tim sort that detects ascending natural runs, extends short runs with insertion sort, and merges them. Test it on `[5, 1, 4, 2, 8, 3, 7, 6, 0, 9]` and trace the detected runs.

3. **Hard** — Benchmark heap sort vs quicksort on arrays of size 10⁶. Now try sizes that don't fit in L3 cache (10⁷ elements). Does quicksort's cache advantage widen or shrink? Explain using spatial locality.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Bottom-up heapify | "build heap in linear time" | Sift down every internal node from the middle to the root; total work sums to O(n) |
| Intro sort | "introspective sort" | Hybrid quicksort/heap sort with a recursion-depth guard of 2 log n |
| Natural run | "presorted subsequence" | Maximal ascending/descending contiguous segment from a single left-to-right scan |
| Galloping mode | "exponential search during merge" | When one run consistently wins, switch to exponential search to copy chunks |
| min_run | "minimum run length" | Threshold (32–64) below which a run is extended via insertion sort |

## Further Reading

- T. Peters, "Tim Peters' Timsort," CPython source — the original implementation and its detailed comments.
- D. Musser, "Introspective Sorting and Selection Algorithms," Software: Practice and Experience, 1997.
- R. Sedgewick, *Algorithms in C++, Parts 1–4*, Chapter 9 — heap sort analysis and the bottom-up heapify proof.
