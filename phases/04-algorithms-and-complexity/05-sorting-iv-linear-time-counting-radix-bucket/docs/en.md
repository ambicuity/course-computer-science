# Sorting IV — Linear-Time: Counting, Radix, Bucket

## Why Ω(n log n) Isn't Always the Floor

Comparison-based sorting has an Ω(n log n) lower bound: you can't distinguish n! permutations with fewer than Ω(n log n) comparisons. But counting sort, radix sort, and bucket sort **never compare elements against each other**. They exploit structure in the data—integer ranges, digit positions, or distribution shape—to sort in linear time.

**The comparison model only limits algorithms that decide order via pairwise comparisons.** Once you use arithmetic, digit extraction, or hash-like indexing, that bound no longer binds you.

---

## Counting Sort

**Idea:** For every element x, count how many elements are ≤ x. That count tells you exactly where x belongs in the output.

**Input:** Array A[0..n−1] where each value is in {0, 1, …, k}.

### Algorithm

1. Allocate a count array C[0..k] initialized to 0.
2. For each element, increment C[value].
3. Compute prefix sums: C[i] += C[i−1]. Now C[i] is the number of elements ≤ i.
4. Walk A from right to left (right-to-left preserves **stability**), placing each element at position C[value] − 1, then decrement C[value].

### Complexity

- **Time:** O(n + k) — one pass to count, one pass to place.
- **Space:** O(n + k).
- **Stable:** Yes, when iterating from the right.

### Why It Matters

Counting sort is the workhorse inside radix sort. It's also the fastest option when k is O(n)—think sorting ages (range 0–120) or ASCII characters (range 0–255). But if k is Ω(n²), the count array blows up and comparison sort wins.

---

## Radix Sort

**Idea:** Sort by each digit position, least significant digit first (LSD) or most significant digit first (MSD). Each pass uses a stable sort—counting sort—as a subroutine.

### LSD Radix Sort

Sort by digit 0, then digit 1, then digit 2, … up to digit d−1. Each pass must be **stable** so that earlier digit orderings are preserved.

**Complexity:** O(d · (n + k)) where d = number of digits, k = range per digit (e.g., 10 for decimal, 256 for bytes).

### MSD Radix Sort

Sort by digit d−1 first, then recursively sort each bucket by digit d−2, etc. Can early-terminate on small buckets. More cache-friendly for variable-length keys but requires recursion or auxiliary structures.

### Real-World Use

The Linux kernel's `sort()` function uses a hybrid radix sort when keys are fixed-width integers. Rust's standard library `sort()` internally uses counting sort for small integer types via `sort_unstable()` when the key range is known.

---

## Bucket Sort

**Idea:** If input is uniformly distributed over an interval, divide the interval into n equal buckets, scatter elements into buckets, sort each bucket individually (insertion sort), then concatenate.

### Algorithm (for data in [0, 1))

1. Create n empty buckets.
2. For each element x, place it in bucket ⌊n · x⌋.
3. Sort each bucket with insertion sort.
4. Concatenate buckets.

### Complexity

| Case | Time |
|------|------|
| Best (uniform, no collisions) | O(n) |
| Average (uniform distribution) | O(n) |
| Worst (all in one bucket) | O(n²) with insertion sort |

The O(n) average relies on the uniformity assumption. If input is clustered, bucket sort degenerates to whatever you use to sort individual buckets.

---

## When to Use Which

| Algorithm | Key Requirement | Time | Stable? | Best For |
|-----------|----------------|------|---------|----------|
| Counting sort | Small integer range k | O(n+k) | Yes | Sorting by small keys (ages, ASCII codes) |
| LSD Radix sort | Fixed-width keys | O(d(n+k)) | Yes | Integers, fixed-length strings |
| MSD Radix sort | Variable-length keys | O(d(n+k)) | Yes* | Variable-length strings (with care) |
| Bucket sort | Uniform distribution | O(n) avg | Depends | Floats in known range |
| Comparison sort | Anything | O(n log n) | Depends | General-purpose, complex keys |

**Rule of thumb:** If your keys have exploitable structure (small range, fixed width, uniform distribution), a linear sort beats comparison sort. If keys are arbitrary or structure is unknown, stick with O(n log n).

---

## Build From Scratch

### Counting Sort (Stable)

```python
def counting_sort(arr, max_val):
    count = [0] * (max_val + 1)
    output = [0] * len(arr)

    for x in arr:
        count[x] += 1
    for i in range(1, max_val + 1):
        count[i] += count[i - 1]

    # Right-to-left preserves stability
    for x in reversed(arr):
        count[x] -= 1
        output[count[x]] = x
    return output
```

### Radix Sort (LSD)

```python
def counting_sort_by_digit(arr, exp):
    count = [0] * 10
    output = [0] * len(arr)

    for x in arr:
        index = (x // exp) % 10
        count[index] += 1
    for i in range(1, 10):
        count[i] += count[i - 1]
    for x in reversed(arr):
        index = (x // exp) % 10
        count[index] -= 1
        output[count[index]] = x
    return output

def radix_sort_lsd(arr):
    if not arr:
        return arr
    max_val = max(arr)
    exp = 1
    while max_val // exp > 0:
        arr = counting_sort_by_digit(arr, exp)
        exp *= 10
    return arr
```

### Bucket Sort

```python
def bucket_sort(arr, n_buckets=None):
    if not arr:
        return arr
    n = len(arr)
    n_buckets = n_buckets or n
    buckets = [[] for _ in range(n_buckets)]
    min_val, max_val = min(arr), max(arr)
    span = max_val - min_val or 1

    for x in arr:
        idx = min(int((x - min_val) / span * n_buckets), n_buckets - 1)
        buckets[idx].append(x)

    result = []
    for b in buckets:
        b.sort()  # insertion sort for small buckets is ideal
        result.extend(b)
    return result
```

---

## Use It: Multi-Algorithm Sorter

A practical sorter that auto-selects based on input characteristics:

```python
def smart_sort(arr):
    """Auto-select sorting algorithm based on input analysis."""
    if not arr or len(arr) <= 1:
        return arr

    # Small integer range → counting sort
    if all(isinstance(x, int) for x in arr):
        min_val, max_val = min(arr), max(arr)
        range_size = max_val - min_val + 1
        if range_size <= len(arr) * 4:
            return counting_sort_shifted(arr, min_val, max_val)

    # Floating point, roughly uniform → bucket sort
    if all(isinstance(x, float) for x in arr) and len(arr) >= 100:
        return bucket_sort(arr)

    # Fallback: Timsort
    return sorted(arr)
```

---

## Exercises

### Exercise 1: Prove Counting Sort Is Stable

Show that two elements with equal value appear in the output in the same relative order as in the input. **Hint:** Walk through a small example [4, 2, 4, 1, 2] and trace which position each duplicate 4 gets. Why does right-to-left iteration matter?

### Exercise 2: Radix Sort for Variable-Length Strings

Implement radix sort that handles strings of different lengths (e.g., ["cat", "car", "a", "bat"]). Treat "shorter" as having padding that sorts before any letter. What changes from integer LSD radix sort?

### Exercise 3: Bucket Sort Expected-Case Proof

Given n elements drawn uniformly from [0, 1), show that bucket sort runs in O(n) expected time. **Hint:** Compute the expected number of comparisons inside bucket i. Let X_i be the number of elements in bucket i. E[X_i] = 1. The expected cost of sorting bucket i with insertion sort is E[X_i²], which you can compute as:

E[X_i²] = Var(X_i) + (E[X_i])² = 1 − 1/n + 1 = 2 − 1/n

Sum over all n buckets: n · (2 − 1/n) = 2n − 1 = O(n).

---

## Ship It: What You Should Know

1. Linear sorts break the Ω(n log n) bound by **not comparing**.
2. Counting sort is the stable subroutine that makes radix sort work.
3. Radix sort's cost is O(d · (n + k))—linear when d and k are constants.
4. Bucket sort is O(n) average **only under uniformity**—a strong assumption.
5. In practice, hybrid approaches (radix for integers, Timsort for general) are common in systems code.
