# Sorting II — Merge, Quick (and Quickselect)

> Divide-and-conquer sorts that actually scale.

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 04 lessons 01–02
**Time:** ~75 minutes

## Learning Objectives

- Implement merge sort and quicksort from scratch, understanding their divide-and-conquer structure.
- Analyze best, worst, and average complexities and explain *why* they differ.
- Compare pivot strategies (first, random, median-of-3, 3-way) and their performance impact.
- Implement Quickselect for O(n) average k-th element selection.
- Explain why merge sort is stable and quicksort is not.

## The Problem

Lesson 02 proved insertion/selection/bubble sort are O(n²) — useless for millions of items. We need O(n log n) sorts. Merge sort and quicksort are the two workhorses. Quickselect gives us k-th element in O(n) without sorting.

## The Concept

### Merge Sort: Divide, Conquer, Merge

Split in half, sort each half recursively, merge two sorted halves.

```
[38, 27, 43, 3, 9, 82, 10]
           /          \
 [38,27,43,3]      [9,82,10]
    /     \           /    \
[38,27] [43,3]   [9,82]  [10]
  |  \   /  |     /  \      |
[27,38] [3,43] [9,82]    [10]
    \     /       \       /
 [3,27,38,43]   [9,10,82]
           \       /
    [3, 9, 10, 27, 38, 43, 82]
```

**Recurrence:** T(n) = 2T(n/2) + O(n) → **O(n log n) guaranteed**. Merge picks the left element on ties → **stable**. Cost: O(n) extra space for the merge buffer.

### Quicksort: Partition In-Place

Pick a pivot, partition so smaller elements go left, larger go right, recurse.

**Average:** T(n) = 2T(n/2) + O(n) → **O(n log n)**.
**Worst:** Bad pivot (always min/max) → T(n) = T(n-1) + O(n) → **O(n²)**.
**Space:** O(log n) stack, in-place partition.

### Lomuto vs Hoare

**Lomuto:** Scan left-to-right, swap smaller elements left. Simple, more swaps.
**Hoare:** Two pointers walk inward, swap misplaced. ~3× fewer swaps in practice.

### Pivot Strategies

| Strategy | Worst case | Notes |
|----------|-----------|-------|
| First | O(n²) on sorted | Trivially defeated |
| Random | O(n²) negligible prob | Expected O(n log n) always |
| Median-of-3 | O(n²) rarely | Best practical default |
| 3-way | O(n) all-equal | Dutch National Flag |

### Quickselect

Quicksort's partition, but recurse only into the half containing the k-th element.

**Average:** T(n) = T(n/2) + O(n) → **O(n)**.
**Worst:** T(n) = T(n-1) + O(n) → **O(n²)**.

## Build It

### Step 1: Merge Sort

```python
def merge_sort(arr: list[int]) -> list[int]:
    if len(arr) <= 1:
        return arr
    mid = len(arr) // 2
    return merge(merge_sort(arr[:mid]), merge_sort(arr[mid:]))

def merge(left, right):
    result, i, j = [], 0, 0
    while i < len(left) and j < len(right):
        if left[i] <= right[j]:  # <= preserves stability
            result.append(left[i]); i += 1
        else:
            result.append(right[j]); j += 1
    result.extend(left[i:])
    result.extend(right[j:])
    return result
```

### Step 2: Quicksort with Pivot Strategies

```python
import random

def quick_sort(arr, pivot_strategy="median3"):
    _qs(arr, 0, len(arr) - 1, pivot_strategy)

def _qs(arr, lo, hi, strategy):
    if lo >= hi: return
    pivot_idx = _choose_pivot(arr, lo, hi, strategy)
    arr[lo], arr[pivot_idx] = arr[pivot_idx], arr[lo]
    p = _partition_lomuto(arr, lo, hi)
    _qs(arr, lo, p - 1, strategy)
    _qs(arr, p + 1, hi, strategy)

def _choose_pivot(arr, lo, hi, strategy):
    if strategy == "first":  return lo
    if strategy == "random": return random.randint(lo, hi)
    if strategy == "median3":
        mid = (lo + hi) // 2
        c = sorted([(arr[lo],lo),(arr[mid],mid),(arr[hi],hi)], key=lambda x: x[0])
        return c[1][1]

def _partition_lomuto(arr, lo, hi):
    pivot, i = arr[lo], lo + 1
    for j in range(lo + 1, hi + 1):
        if arr[j] < pivot:
            arr[i], arr[j] = arr[j], arr[i]; i += 1
    arr[lo], arr[i-1] = arr[i-1], arr[lo]
    return i - 1
```

### Step 3: Quickselect

```python
def quickselect(arr, k):
    lo, hi = 0, len(arr) - 1
    while lo < hi:
        piv = random.randint(lo, hi)
        arr[lo], arr[piv] = arr[piv], arr[lo]
        p = _partition_lomuto(arr, lo, hi)
        if p == k:   return arr[p]
        elif p < k:  lo = p + 1
        else:        hi = p - 1
    return arr[lo]
```

## Use It

- **Java `Arrays.sort`:** Dual-pivot quicksort for primitives, Timsort for objects.
- **Python `sorted`:** Timsort — adaptive merge sort exploiting sorted runs.
- **Rust `sort_unstable`:** pdqsort — median-of-3, insertion fallback, pattern-defeating.
- **Go `slices.Sort`:** pdqsort since Go 1.19.

Primitives → quicksort variants. Objects (stability needed) → merge-sort variants. Production enhancements: insertion sort cutoff for n < 16, tail-call elimination (recurse smaller half), 3-way partition on duplicates, introsort fallback to heapsort.

## Read the Source

- **Rust pdqsort:** `library/alloc/src/slice/sort.rs`
- **CPython Timsort:** `Objects/listsort.txt`

## Ship It

`outputs/` contains **a sort library with pluggable pivot strategies and step counting** — reuse later for benchmarking.

## Exercises

1. **Easy** — Prove merge sort is stable by tracing `[(3,'a'), (1,'b'), (3,'c')]`. Show `(3,'a')` precedes `(3,'c')`.
2. **Medium** — Implement 3-way partition (Dutch National Flag). Test on 90% duplicates, verify O(n). Invariant: `arr[0..lt-1] < pivot`, `arr[lt..gt] == pivot`, `arr[gt+1..hi] > pivot`.
3. **Hard** — External merge sort: sort 1M integers exceeding RAM (limit chunks to 1000). Report I/O pass count.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Stable sort | "Preserves order of equals" | `a[i]==a[j]`, `i<j` → `a[i]` before `a[j]` in output |
| In-place | "No extra memory" | O(log n) space — quicksort's stack |
| Partition | "Split around a pivot" | `arr[lo..p-1] <= pivot <= arr[p+1..hi]` |

## Further Reading

- Cormen et al., *Introduction to Algorithms*, Ch. 7–8.
- Bentley & McIlroy, "Engineering a Sort Function," 1993 — behind glibc `qsort`.
- Tim Peters, `listsort.txt` in CPython source.
