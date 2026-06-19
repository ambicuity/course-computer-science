# Sorting I — Insertion, Selection, Bubble, and Why They Lose

> The quadratic sorts are the baseline you measure everything else against.

**Type:** Build
**Languages:** Python, Rust
**Prerequisites:** Phase 01 (asymptotic notation), Phase 02 (arrays, loops)
**Time:** ~45 minutes

## Learning Objectives

- Implement insertion sort, selection sort, and bubble sort from scratch
- Prove each is O(n²) and compare best/worst/average cases
- Understand stability and why it matters
- Identify when insertion sort actually wins (small n, nearly sorted)

## The Problem

You have a list of integers. You need them in order. This is the most fundamental operation in computer science — nearly every non-trivial program calls a sort at some point.

Three simple algorithms dominate introductory courses: insertion sort, selection sort, and bubble sort. They all run in O(n²) time, so why learn all three? Because each exposes a different idea — local insertion, global minimum selection, pairwise swapping — and each has a different best/worst case profile. They aren't dead code: Python's `sorted()` uses Tim sort, which calls insertion sort on every small subarray. C++'s `std::sort` switches to insertion sort below 16 elements. The quadratic sorts are the building blocks inside the sorts you actually use.

## The Concept

### How Each Sort Works

**Insertion Sort** — maintain a sorted prefix, insert the next element into position.

```
[5 3 8 1 2]
[3 5 8 1 2]  3 inserted before 5
[3 5 8 1 2]  8 in place
[1 3 5 8 2]  1 bubbles to front
[1 2 3 5 8]  2 inserted after 1
```

On nearly sorted data the scan is short → O(n) best case.

**Selection Sort** — find the minimum in the unsorted suffix, swap to front.

```
[5 3 8 1 2]
[1 3 8 5 2]  min=1, swap to pos 0
[1 2 8 5 3]  min=2, swap to pos 1
[1 2 3 5 8]  min=3, swap to pos 2
[1 2 3 5 8]  min=5, already in place
```

Always n(n-1)/2 comparisons regardless of input. At most n swaps — useful when writes are expensive.

**Bubble Sort** — repeatedly sweep adjacent pairs, swap if out of order.

```
[5 3 8 1 2]
[3 5 1 2 8]  8 bubbles to end
[3 1 2 5 8]  5 bubbles right
[1 2 3 5 8]  3 bubbles right
[1 2 3 5 8]  no swaps, early exit
```

O(n) on sorted input with early-exit flag. Stable by nature.

### Stability

A sort is **stable** if equal elements keep their original relative order.

```
Input:  [(3,A) (1,B) (3,C)]
Stable: [(1,B) (3,A) (3,C)]   <- A before C preserved
Unstable: [(1,B) (3,C) (3,A)] <- A and C swapped
```

Why it matters: sort students by grade, then by name — stability means name ordering survives. Insertion sort and bubble sort are stable. Selection sort is not.

### Comparison Table

| Algorithm      | Best    | Average | Worst   | Stable | In-place | Swaps (worst) |
|----------------|---------|---------|---------|--------|----------|---------------|
| Insertion sort | O(n)    | O(n²)   | O(n²)   | Yes    | Yes      | O(n²)         |
| Selection sort | O(n²)   | O(n²)   | O(n²)   | No     | Yes      | O(n)          |
| Bubble sort    | O(n)    | O(n²)   | O(n²)   | Yes    | Yes      | O(n²)         |

### Why They Lose

All three are quadratic in the average case. For n = 1,000,000, that's roughly 10¹² operations. A O(n log n) sort does about 20,000,000 — 50,000x fewer. But for n ≤ 32, the constant factors of simple sorts beat divide-and-conquer overhead.

## Build It

### Step 1: Insertion Sort

```python
def insertion_sort(arr):
    comparisons, swaps = 0, 0
    for i in range(1, len(arr)):
        key, j = arr[i], i - 1
        while j >= 0:
            comparisons += 1
            if arr[j] > key:
                arr[j + 1] = arr[j]
                swaps += 1; j -= 1
            else: break
        arr[j + 1] = key
    return comparisons, swaps
```

### Step 2: Selection Sort

```python
def selection_sort(arr):
    comparisons, swaps = 0, 0
    for i in range(len(arr) - 1):
        min_idx = i
        for j in range(i + 1, len(arr)):
            comparisons += 1
            if arr[j] < arr[min_idx]: min_idx = j
        if min_idx != i:
            arr[i], arr[min_idx] = arr[min_idx], arr[i]
            swaps += 1
    return comparisons, swaps
```

### Step 3: Bubble Sort

```python
def bubble_sort(arr):
    comparisons, swaps, n = 0, 0, len(arr)
    for i in range(n - 1):
        swapped = False
        for j in range(n - 1 - i):
            comparisons += 1
            if arr[j] > arr[j + 1]:
                arr[j], arr[j + 1] = arr[j + 1], arr[j]
                swaps += 1; swapped = True
        if not swapped: break
    return comparisons, swaps
```

### Step 4: Benchmark Comparison

The `main.py` file runs all three sorts on random, sorted, reverse-sorted, and sawtooth inputs of size 100, 1000, and 5000, then prints a comparison table. Run it to see the numbers match the theory.

## Use It

Python's `sorted()` uses **Tim sort** (Tim Peters, 2002). Tim sort scans for natural runs, extends short runs with insertion sort (threshold 32), then merges. Insertion sort is the inner engine — low overhead, no recursion, excellent cache locality. The O(n²) cost doesn't matter when n < 32.

Rust's `sort()` also uses Tim sort. Rust's `sort_unstable()` uses intro sort (quicksort + heapsort + insertion sort). The quadratic sorts are not obsolete — they're components inside the sorts that *are* fast enough.

## Read the Source

- CPython `Objects/listsort.txt` — Tim Peters' design doc for Tim sort, explains the insertion sort threshold.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained sorting benchmark harness you can reuse in later phases.**

## Exercises

1. **Easy.** Prove insertion sort is stable: show that equal elements never pass each other.
2. **Medium.** Add a shellsort variant using gap sequence [n/2, n/4, ..., 1]. Compare its operation count against insertion sort.
3. **Hard.** Count exact comparisons and swaps for each sort on random, sorted, reverse-sorted, and sawtooth inputs of sizes 10–10,000. Plot the results and identify crossover points.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Stable sort | "Preserves order of ties" | Equal elements maintain their original relative position |
| In-place sort | "Doesn't need extra memory" | Sorts using O(1) additional space beyond the input |
| Adaptive sort | "Faster on sorted data" | Running time improves when input is partially sorted |
| Comparison sort | "Uses < or >" | Sort whose only access to elements is pairwise comparisons |

## Further Reading

- Cormen et al. *Introduction to Algorithms*, Chapter 2 (Insertion Sort).
- Sedgewick, Wayne. *Algorithms*, Chapter 2.1 (Elementary Sorts).
- Tim Peters. "listsort.txt" — the original Tim sort design document.
