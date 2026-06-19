# Parallel Patterns — Map, Reduce, Pipeline, Scan

> Map, Reduce, Pipeline, Scan — the four fundamental parallel patterns.

**Type:** Build
**Languages:** Rust, Python
**Prerequisites:** Phase 13, Lessons 01–15 (threads, atomics, locks, async, CSP, actors, STM, lock-free)
**Time:** ~75 minutes

## Learning Objectives

- Distinguish data parallelism (same op, different data) from task parallelism (different ops, pipelined data).
- Implement a parallel map using rayon and multiprocessing.Pool — see that it scales linearly for pure computations.
- Implement a parallel tree reduction and explain why it achieves O(log n) span despite O(n) work.
- Build a multi-stage pipeline with channels; identify the bottleneck stage and compute throughput.
- Implement two parallel prefix scan algorithms (Hillis-Steele, Blelloch) and compare their work–span trade-offs.
- Apply the work–time framework (W, T, P, speedup bound) to reason about parallel algorithm efficiency.

## The Problem

You have a large array and need to process it fast. The simplest approach — a single loop — leaves cores idle. But throwing threads at every problem doesn't automatically make it faster. The real question is: **which parallel pattern fits the computation?**

Consider a real-time audio processing pipeline:

```
Sample stream → FFT → Frequency filter → Inverse FFT → Volume ramp → Output
```

Each stage does different work. FFT is CPU-heavy, filtering is memory-bound, volume ramp is trivial. Deploying all 16 cores on the FFT stage starves the other stages. Deploying one core per stage in a pipeline means idle cores. The correct decomposition depends on the *pattern* of data dependences in your algorithm.

There are exactly four fundamental patterns that cover almost all parallel algorithms:

| Pattern | Dependence pattern | Example |
|---------|-------------------|---------|
| **Map** | Element-wise independent | Apply gamma correction to every pixel |
| **Reduce** | Associative tree combine | Sum all elements, find max, dot product |
| **Pipeline** | Stage-to-stage streaming | Producer → transform → consumer |
| **Scan** | Cumulative prefix | Prefix sum, running average |

Choosing the wrong pattern means leaving performance on the table — or worse, writing a parallel program that's *slower* than the sequential version due to synchronization overhead.

This lesson builds all four patterns from scratch, measures them, and teaches you to reason about the work–time trade-offs.

## The Concept

### Data Parallelism vs Task Parallelism

**Data parallelism:** same operation applied to different pieces of data. The work splits along the data axis. Examples: map, reduce, scan.

**Task parallelism:** different operations run concurrently, each operating on a stream of data. The work splits along the processing axis. Example: pipeline.

These are complementary — a real system uses both.

### The Work–Time (Work–Span) Framework

The formal way to reason about parallel algorithms:

| Symbol | Meaning |
|--------|---------|
| **W** | **Work** — total number of operations (same as sequential) |
| **T** | **Span** (critical path) — length of the longest chain of dependent operations |
| **P** | Number of processors |
| **S** | **Speedup** — time(1) / time(P) |

**Key inequality:** `time(P) ≥ max(W/P, T)` — you're limited by either the work per processor or the span.

**Parallel slack:** the ratio `W/T`. If slack ≫ P, you can achieve near-linear speedup. If slack < P, the span dominates and adding processors doesn't help.

### 1. Map — Embarrassingly Parallel

```
Input:  [x0, x1, x2, …, x_{n-1}]
Output: [f(x0), f(x1), f(x2), …, f(x_{n-1})]
```

Every output element depends on exactly one input element — no sharing, no communication. This is *embarrassingly parallel*.

- **Work:** W = n (call f n times; same as sequential)
- **Span:** T = O(1) (all n calls in parallel given enough processors)
- **Speedup:** S = O(P) — linear speedup achievable in theory
- **Reality check:** If f is trivial (e.g., `x*2`), memory bandwidth limits speedup. If f is heavy (e.g., FFT of a 4096-point window), near-linear speedup is real.

### 2. Reduce (Fold) — Tree Reduction

```
Input:  [x0, x1, x2, …, x_{n-1}]
Output: x0 ⊕ x1 ⊕ x2 ⊕ … ⊕ x_{n-1}
```

The operator ⊕ must be **associative**: `(a ⊕ b) ⊕ c = a ⊕ (b ⊕ c)`. This lets us reorder the computation into a tree:

```
Level 3:                 total
Level 2:        s0..3            s4..7
Level 1:    s0..1    s2..3    s4..5    s6..7
Level 0:  x0  x1    x2  x3    x4  x5    x6  x7
```

- **Work:** W = n − 1 (same as sequential — tree doesn't reduce total work)
- **Span:** T = O(log n) (tree depth)
- **Speedup:** S ≤ (n−1) / log₂(n) — grows with n, but slowly
- **Important:** Not all operators are associative. Subtraction and division are *not* associative. `(a − b) − c ≠ a − (b − c)`. Floating-point addition is approximately but not exactly associative — parallel reduce may give slightly different results than sequential.

### 3. Pipeline — Task Parallelism

```
Producer → Stage 1 → Stage 2 → … → Stage K → Consumer
```

Each stage runs on its own thread/core. Data flows through channels. Stages execute concurrently but on different items.

- **Latency** (time for one item to go end-to-end): sum of all stage latencies = L₁ + L₂ + … + Lₖ
- **Throughput** (items per second in steady state): 1 / max(L₁, L₂, …, Lₖ)
- **Bottleneck:** the slowest stage. Improving any other stage has *zero* effect on throughput.
- **Work:** W = sum of all stage work per item × n items
- **Span:** T ≈ n × max(Lⱼ) — dominated by the bottleneck

A 3-stage pipeline with latencies 4ms, 7ms, 2ms has throughput ~142 items/s (1/7ms). The total latency per item is 13ms, but that's the *latency* not the *throughput* — throughput and latency are different metrics.

### 4. Scan (Prefix Sum) — Harder to Parallelize

```
Input:  [x0, x1, x2,  …, x_{n-1}]
Output: [x0, x0⊕x1, x0⊕x1⊕x2,  …, x0⊕…⊕x_{n-1}]
```

Each output depends on **all previous inputs**. This creates an apparent sequential bottleneck — the naive algorithm is O(n) span.

Two classic parallel algorithms:

#### Hillis-Steele (1993) — Fast span, inefficient work

```
For d = 0, 1, …, ⌈log₂n⌉−1:
    For i in parallel where i ≥ 2ᵈ:
        x[i] += x[i − 2ᵈ]
```

- **Work:** W = n log₂ n (does redundant work — each element is summed multiple times)
- **Span:** T = log₂ n
- **Speedup bound:** S ≤ n log₂ n / log₂ n = n — but work efficiency is poor for large n

#### Blelloch (1990) — Work-efficient

Two phases:
1. **Up-sweep:** build a reduction tree (partial sums at power-of-2 boundaries)
2. **Down-sweep:** distribute partial sums to produce the prefix

- **Work:** W = 2n (only 2× the sequential work — the minimum possible for a parallel algorithm)
- **Span:** T = 2 log₂ n
- **Speedup bound:** S ≤ 2n / (2 log₂ n) = n / log₂ n

**Trade-off:** Blelloch is work-efficient but has higher span constant (2 log n vs log n). Hillis-Steele is better when n is small and processors are abundant; Blelloch is better when n is large and work-efficiency matters.

### PRAM Model

The theoretical model underlying these analyses is the **Parallel Random Access Machine (PRAM)**:
- P processors, shared memory
- Each processor executes its own instruction stream
- All processors run in lockstep
- Memory access takes unit time

Three variants: **CREW** (concurrent read, exclusive write), **CRCW** (concurrent read, concurrent write), **EREW** (exclusive read, exclusive write). The algorithms in this lesson assume CREW — multiple processors can read the same memory location simultaneously, but only one writes to any location at a time.

## Build It

### Step 1: Parallel Map in Rust (rayon)

The simplest parallel pattern: rayon's `.par_iter().map()` automatically splits the data across threads:

```rust
use rayon::prelude::*;

fn parallel_map(data: &[i64]) -> Vec<i64> {
    data.par_iter().map(|x| x * x + 2 * x + 1).collect()
}
```

`par_iter()` creates a parallel iterator that divides the slice into work-stealing chunks. Each chunk is processed by a thread-pool thread. The `collect()` reassembles results in order — rayon guarantees the output ordering matches the input.

**Key insight:** The call to `collect()` is a barrier — all threads must finish before the result is available. This is fine for map because all elements are independent. For streaming patterns (pipeline), a barrier every stage would kill throughput.

### Step 2: Parallel Reduce in Rust

rayon provides built-in reductions:

```rust
let sum: i64 = data.par_iter().sum();
let min = data.par_iter().min().unwrap();
let max = data.par_iter().max().unwrap();
```

For custom reductions, use `.fold().reduce()`:

```rust
let result = data.par_iter()
    .fold(|| 0i64, |acc, x| acc + x)   // per-chunk reduction
    .reduce(|| 0i64, |a, b| a + b);     // combine chunks
```

`fold` runs on each chunk independently (no synchronization). `reduce` combines chunk results in a tree. The key: the operator passed to `reduce` must be **associative**.

### Step 3: Pipeline in Rust (Channels + Threads)

A 4-stage pipeline using `std::sync::mpsc`:

```rust
// Channel endpoints per stage
let (tx1, rx1) = mpsc::channel();
let (tx2, rx2) = mpsc::channel();

// Stage 1: Producer
thread::spawn(move || {
    for i in 0..n { tx1.send(i).unwrap(); }
    drop(tx1);
});

// Stage 2: Transformer
thread::spawn(move || {
    for val in rx1 { tx2.send(val * 2).unwrap(); }
    drop(tx2);
});

// Stage 3: Consumer
thread::spawn(move || {
    for val in rx2 { results.push(val); }
});
```

**Sentinel pattern:** Send a special value (e.g., `None`) to signal end-of-stream. Without sentinels, the consumer hangs because `rx.iter()` blocks waiting for the next message.

**Bounded channels vs unbounded:** `mpsc::channel()` is unbounded — the producer can run arbitrarily ahead of the consumer. For backpressure, use `sync_channel(0)` (rendezvous) or `sync_channel(bound)` (bounded buffer).

### Step 4: Parallel Prefix Sum in Rust

#### Hillis-Steele (double-buffered)

Each step uses two arrays and swaps. Within a step, reads are from the "old" array and writes go to the "new" array — no data races:

```rust
fn hillis_steele(input: &[i64]) -> Vec<i64> {
    let n = input.len();
    let mut old = input.to_vec();
    let mut new = vec![0i64; n];
    let mut d = 1;

    while d < n {
        new.par_iter_mut().enumerate().for_each(|(i, v)| {
            *v = old[i];
            if i >= d { *v += old[i - d]; }
        });
        std::mem::swap(&mut old, &mut new);
        d <<= 1;
    }
    old
}
```

#### Blelloch (work-efficient, in-place with par_chunks_mut)

Two phases with stride-based access patterns that partition the array into independent blocks:

```rust
fn blelloch_exclusive(input: &[i64]) -> Vec<i64> {
    let n = input.len().next_power_of_two();
    let mut data: Vec<i64> = input.iter().copied()
        .chain(std::iter::repeat(0)).take(n).collect();

    // Up-sweep: build reduction tree
    for d in 0..(n as f64).log2() as usize {
        let stride = 1 << (d + 1);
        let half = 1 << d;
        data.par_chunks_mut(stride).for_each(|chunk| {
            let (first, rest) = chunk.split_at_mut(half);
            rest[stride - half - 1] += first[half - 1];
        });
    }

    // Down-sweep: distribute partial sums
    data[n - 1] = 0;
    for d in (0..(n as f64).log2() as usize).rev() {
        let stride = 1 << (d + 1);
        let half = 1 << d;
        data.par_chunks_mut(stride).for_each(|chunk| {
            let (first, rest) = chunk.split_at_mut(half);
            let left = &mut first[half - 1];
            let right = &mut rest[stride - half - 1];
            let lv = *left;
            let rv = *right;
            *left = rv;
            *right += lv;
        });
    }

    data.truncate(input.len());
    data
}
```

The `split_at_mut` trick splits each chunk into two disjoint mutable slices, allowing safe parallel mutation of two positions within the same chunk.

### Step 5: Python Multiprocessing Equivalents

```python
from multiprocessing import Pool

with Pool() as pool:
    # Parallel map
    result = pool.map(square, data)

    # Parallel reduce via map + partial sum
    chunks = [data[i:i+sz] for i in range(0, len(data), sz)]
    partials = pool.map(sum, chunks)
    total = sum(partials)
```

`Pool.map` is the direct Python analog of rayon's `par_iter().map()`. The pool maintains a worker process per CPU core and uses fork (on Unix) to share memory.

For pipeline, use `multiprocessing.Pipe` or `multiprocessing.Queue` with `Process` objects. For prefix scan, Hillis-Steele with `Pool.map` approximates the Rust version but with higher serialization overhead.

## Use It

### Rayon (Rust) — Production Parallel Iterators

Add `rayon = "1"` to `Cargo.toml`. Rayon's key patterns:

```rust
// Map
let v: Vec<T> = collection.par_iter().map(f).collect();

// Reduce
let sum: i32 = collection.par_iter().sum();
let min = collection.par_iter().min();
let product: i64 = collection.par_iter().product();

// Fold (custom reduce)
let result = collection.par_iter()
    .fold(|| identity, |acc, x| combine(acc, x))
    .reduce(|| identity, |a, b| combine(a, b));

// Chain map-then-reduce
let dot = a.par_iter().zip(b.par_iter())
    .map(|(x, y)| x * y)
    .sum();
```

Rayon uses **work stealing**: idle threads steal tasks from busy threads' queues. This provides automatic load balancing without a centralized scheduler.

### Python multiprocessing.Pool

```python
from multiprocessing import Pool, cpu_count

with Pool(processes=cpu_count()) as pool:
    # Map
    results = pool.map(func, iterable)

    # Starmap (for multiple arguments)
    results = pool.starmap(func, iterable_of_tuples)

    # Imap (lazy, ordered)
    for r in pool.imap(func, iterable):
        process(r)

    # Imap_unordered (lazy, unordered — faster if order doesn't matter)
    for r in pool.imap_unordered(func, iterable):
        process(r)
```

**Important:** `Pool.map` serializes each item via pickle. For small items and fast functions, serialization overhead dominates. Use `chunksize` to batch items:

```python
pool.map(func, data, chunksize=1000)
```

### Work–Time in Practice

| Pattern | W | T | When to use |
|---------|---|---|-------------|
| Map | O(n) | O(1) | Pure element-wise; heavy f |
| Reduce | O(n) | O(log n) | Associative combine; large n |
| Pipeline | varies | bottleneck-limited | Streaming data; different ops |
| Scan (HS) | O(n log n) | O(log n) | Small n, many cores |
| Scan (Blelloch) | O(2n) | O(2 log n) | Large n, work-efficient needed |

## Read the Source

- **Rayon source:** `rayon::iter::plumbing` module — the work-stealing fork-join implementation. The `bridge_producer` consumer is where parallelism actually happens.
- **Rust `std::sync::mpsc`:** `library/std/src/sync/mpsc/` — a multi-producer, single-consumer channel implementation using a lock-free queue internally.
- **Python `multiprocessing.pool.py`:** the `Pool.map` implementation — uses `apply_async` and a result queue. The `_map_async` helper handles chunking and error propagation.
- **Hillis & Steele, "Data Parallel Algorithms" (CACM 1986):** the original paper describing the prefix scan algorithm. Establishes the "scan" as a primitive for parallel computing.
- **Blelloch, "Prefix Sums and Their Applications" (1990):** the work-efficient scan algorithm. Shows how to build a parallel scan with only O(n) work (2n operations total).
- **Guy Blelloch's notes on parallel prefix:** https://www.cs.cmu.edu/~guyb/papers/Ble93.pdf — comprehensive treatment of parallel scan algorithms and their applications.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained parallel patterns reference suite** with Rust implementations (rayon) and Python implementations (multiprocessing.Pool) of map, reduce, pipeline, and scan. Includes both Hillis-Steele and Blelloch prefix scan algorithms. Use the benchmarks to compare sequential vs parallel speedups. Reuse the pipeline pattern for producer–consumer workflows in later lessons.

## Exercises

1. **Easy** — Run the Rust benchmark with 1, 2, 4, and 8 threads (set `RAYON_NUM_THREADS` env variable). Record speedups. Which pattern benefits most from more cores?

2. **Medium** — Modify the pipeline to use a bounded channel (`sync_channel(4)`). Measure throughput with different buffer sizes (1, 4, 16, 64). How does buffer size affect throughput? Why?

3. **Medium** — Implement a parallel map-reduce in Python: process a large corpus of text, map each document to word counts, reduce to global word frequencies. Use `Pool.map` for the map phase and a tree reduce for the combine.

4. **Hard** — Implement the Blelloch scan in Python using `multiprocessing.shared_memory` (Python 3.8+). Compare its performance against the naive Hillis-Steele with `Pool.map`. Where does the serialization overhead dominate?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Map | "Apply f to every element" | Element-wise transformation with no data dependences. Embarrassingly parallel. W = n, T = O(1). |
| Reduce | "Combine all elements with ⊕" | Associative tree combine. W = n−1, T = O(log n). The operator must be associative. |
| Pipeline | "Assembly line of stages" | Task parallelism: K stages run concurrently, each on its own thread. Throughput = 1 / max(Lⱼ). |
| Scan | "Parallel prefix sum" | Each output depends on all previous inputs. W = O(n log n) (Hillis-Steele) or W = 2n (Blelloch). T = O(log n). |
| Prefix sum | "Prefix scan" | Inclusive: output[i] = x₀⊕…⊕xᵢ. Exclusive: output[i] = x₀⊕…⊕x_{i−1}, output[0] = identity. |
| Work | "W" | Total operations. Same as sequential work for work-efficient algorithms. |
| Span | "T" | Critical path length. The longest chain of dependent operations that cannot be parallelized. |
| PRAM | "Idealized parallel machine" | P processors, shared memory, unit-time access. Three variants: CREW, CRCW, EREW. |
| Data parallelism | "Same op, different data" | Split data across cores; each core runs the same function. Map, reduce, scan. |
| Task parallelism | "Different ops, pipelined data" | Split work by processing stage; stages are functions, data flows between them. Pipeline. |
| Embarrassingly parallel | "No communication needed" | Map. The ideal case for parallelism — linear speedup achievable. |
| Work-efficient | "W(parallel) = O(W(sequential))" | The parallel algorithm does no more total work than the sequential one. Blelloch scan is work-efficient; Hillis-Steele is not. |

## Further Reading

1. **Herlihy & Shavit, "The Art of Multiprocessor Programming," 2nd ed., Chapter 1–5** — Covers the foundations: mutual exclusion, concurrent objects, and the basics of parallel reasoning. Chapter 5 on the "correctness" of concurrent objects is particularly relevant to ensuring parallel patterns are correct.

2. **McCool, Robison, & Reinders, "Structured Parallel Programming" (2012)** — The definitive practical guide to parallel patterns: map, reduce, scan, and their compositions. Uses Intel TBB, Cilk Plus, and OpenCL. Chapter 4 covers the work–span framework in depth.

3. **Hillis & Steele, "Data Parallel Algorithms," CACM 1986** — Introduces the scan primitive and shows how it can be used to implement virtually all data-parallel algorithms. The "thinking parallel" manifesto.

4. **Blelloch, "Vector Models for Data-Parallel Computing" (1990)** — The book-length treatment of the scan primitive and its applications. Includes the work-efficient scan algorithm and the "segmented scan" generalization.

5. **Rust Rayon documentation:** https://docs.rs/rayon/latest/rayon/ — Parallel iterator patterns with examples. The `ParallelIterator` trait docs are particularly good for understanding available operations.

6. **Python multiprocessing documentation:** https://docs.python.org/3/library/multiprocessing.html — Pool, Process, Queue, Pipe. The `concurrent.futures` module provides a higher-level interface (ThreadPoolExecutor, ProcessPoolExecutor).
