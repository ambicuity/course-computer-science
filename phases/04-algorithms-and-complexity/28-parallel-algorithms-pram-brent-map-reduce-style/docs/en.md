# Parallel Algorithms — PRAM, Brent, Map-Reduce style

> How to reason about algorithms when more than one thing happens at once.

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 04 lessons 01–27
**Time:** ~75 minutes

## Learning Objectives

- Classify shared-memory machines by PRAM access restrictions (EREW, CREW, CRCW).
- Use Brent's theorem to bound parallel time from work and depth.
- Implement work-efficient parallel prefix sum and parallel merge sort.
- Recognize Map-Reduce as a practical parallel programming pattern.
- Measure real speedup against theoretical work-depth bounds.

## The Problem

Sequential algorithms assume one instruction at a time. Real hardware — multi-core CPUs, GPUs, distributed clusters — runs many operations simultaneously. Without a parallelism model you cannot predict how fast an algorithm *could* be, tell whether your implementation is close to optimal, or choose between fork-join, Map-Reduce, and GPU kernel strategies.

## The Concept

### 1. The PRAM Model

The **Parallel Random-Access Machine** is the parallel analogue of the RAM model: *p* processors sharing a single global memory, executing in lock-step synchronous rounds. Four variants based on concurrent memory access:

| Variant | Reads | Writes |
|---------|-------|--------|
| **EREW** | exclusive | exclusive |
| **CREW** | concurrent | exclusive |
| **CRCW** | concurrent | concurrent (needs tie-breaking) |

CRCW sub-variants: *common* (all writers must agree), *arbitrary* (one wins at random), *priority* (lowest-index wins). Stronger models simulate weaker ones with at most logarithmic overhead.

### 2. Work-Depth Model

Instead of counting processors we count two quantities:

- **Work** `T₁` — total operations (what a single processor would do).
- **Depth** `T∞` — longest dependency chain (critical path).

**Brent's theorem** bounds parallel time on *p* processors:

```
T₁/p  ≤  T_p  ≤  T₁/p + T∞
```

If your implementation exceeds the upper bound, you have scheduling overhead. **Work-efficiency** means T₁ matches the best sequential algorithm.

### 3. Parallel Prefix Sum (Scan)

Given `[a₀, ..., aₙ₋₁]`, compute exclusive prefix sums `sᵢ = a₀ + ⋯ + aᵢ₋₁`.

**Up-sweep** — build a reduce tree bottom-up. At each level *d*, every active node adds its left child into its right child. After ⌈log n⌉ levels the root holds the total sum.

**Down-sweep** — distribute prefixes. Set root to 0. At each level top-down, for each node: swap left↔right, then add old left into new right.

- **Work:** O(n). **Depth:** O(log n). Work-efficient — matches the best sequential O(n).

### 4. Parallel Merge Sort

**Practical fork-join:** divide in half, sort each half in parallel, merge. With sequential merge: work O(n log n), depth O(n). With parallel merge (co-rank based): depth O(log² n). Cole's optimal CRCW sort achieves O(n log n) + O(log n) depth but is rarely implemented outside theory.

### 5. Map-Reduce Style

```
Input → [Split] → [Map (parallel)] → [Shuffle/Sort] → [Reduce (parallel)] → Output
```

- **Map:** each worker processes its chunk independently (embarrassingly parallel).
- **Shuffle:** keys redistributed so all values for one key land on the same reducer.
- **Reduce:** each worker aggregates values for its assigned keys.

Assumes failures are common, data is large, communication cost dominates — different from PRAM's shared-memory assumption. But the algorithmic decomposition (divide → parallel process → combine) is identical.

## Build It

### Step 1: Parallel Prefix Sum (Python)

```python
from concurrent.futures import ThreadPoolExecutor
import math

def parallel_prefix_sum(arr):
    n = len(arr)
    if n <= 1: return [0] * n
    m = 1 << math.ceil(math.log2(n))
    buf = list(arr) + [0] * (m - n)

    d = 1  # Up-sweep
    while d < m:
        for i in range(d, m, 2 * d): buf[i + d - 1] += buf[i - 1]
        d *= 2

    buf[m - 1] = 0  # exclusive scan
    d = m // 2       # Down-sweep
    while d >= 1:
        for i in range(d, m, 2 * d):
            t = buf[i - 1]; buf[i - 1] = buf[i + d - 1]; buf[i + d - 1] += t
        d //= 2
    return buf[:n]
```

### Step 2: Parallel Merge Sort (Python)

```python
def parallel_merge_sort(arr, executor=None, depth=0, max_depth=4):
    if len(arr) <= 1: return arr
    if executor is None: executor = ThreadPoolExecutor()
    mid = len(arr) // 2
    if depth < max_depth:
        l = executor.submit(parallel_merge_sort, arr[:mid], executor, depth+1).result()
        r = executor.submit(parallel_merge_sort, arr[mid:], executor, depth+1).result()
    else:
        l, r = sorted(arr[:mid]), sorted(arr[mid:])
    return sequential_merge(l, r)
```

### Step 3: Speedup Measurement

```python
import time
def measure_speedup(parallel_fn, sequential_fn, data):
    t0 = time.perf_counter(); sequential_fn(data); t_seq = time.perf_counter() - t0
    t0 = time.perf_counter(); parallel_fn(data);   t_par = time.perf_counter() - t0
    print(f"  Seq: {t_seq:.4f}s  Par: {t_par:.4f}s  Speedup: {t_seq/t_par:.2f}x")
```

## Use It

| System | Pattern | How it relates |
|--------|---------|---------------|
| **CUDA / GPU** | CRCW PRAM in silicon | Blelloch scan is the hardware prefix sum |
| **Apache Spark** | Map-Reduce with caching | `reduceByKey` = shuffle + reduce |
| **Rust rayon** | Work-stealing pool | `par_iter().scan()` implements prefix sum |
| **Go goroutines** | Fork-join | `sync.WaitGroup` for work-depth scheduling |

Production systems add: work-stealing schedulers, fault tolerance (Spark lineage), memory hierarchy awareness (CUDA shared memory), load balancing.

## Read the Source

- `rayon::iter::ParallelIterator` — Rust data-parallel library; see `src/iter/mod.rs`.
- `spark/python/pyspark/rdd.py` — Python RDD `reduce`/`map` implementations.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A parallel algorithm toolkit: prefix sum + merge sort with speedup benchmarks, in Python and Rust.**

## Exercises

1. **Prove parallel prefix sum correctness.** Show by induction that after up-sweep at level *d*, each node at position *i·2^d − 1* holds the sum of *2^d* elements below it. Then prove the down-sweep restores correct prefix sums.

2. **Implement parallel matrix multiplication.** Given *n × n* matrices A and B, parallelize the *n²* inner-product computations. What are work and depth? Compare against sequential O(n³).

3. **Measure actual speedup vs theoretical bounds.** Run prefix sum on arrays of size 2^10 through 2^20. Plot speedup vs *p*. Where does Amdahl's law kick in? Where does thread overhead dominate?

## Key Terms

| Term | What it actually means |
|------|------------------------|
| PRAM | p processors, one memory, synchronous rounds, classified by read/write conflicts |
| EREW | No two processors may read or write the same cell simultaneously |
| CREW | Multiple reads OK; writes exclusive |
| CRCW | Both reads and writes may overlap; needs conflict resolution rule |
| Work (T₁) | Total operations across all processors; equals sequential time |
| Depth (T∞) | Longest dependency chain; minimum possible parallel time |
| Brent's theorem | T_p ≤ ⌈T₁/p⌉ + T∞ for any PRAM computation |
| Prefix sum / Scan | All partial sums in O(n) work, O(log n) depth |
| Map-Reduce | Map (transform) → Shuffle (regroup by key) → Reduce (aggregate) |
| Work-efficiency | Parallel T₁ matches best sequential complexity |

## Further Reading

- JáJá, J. *An Introduction to Parallel Algorithms.* Addison-Wesley, 1992.
- Blelloch, G. "Prefix Sums and Their Applications." CMU CS-90-190, 1990.
- Dean, J. & Ghemawat, S. "MapReduce: Simplified Data Processing on Large Clusters." OSDI 2004.
