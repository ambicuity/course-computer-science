# How to Think About Performance — Reference Notes

> Companion to `docs/en.md`. Keep these notes open during the rest of Phase 15.

---

## The Performance Equation

```
Time = Instructions × CPI × Clock_period

Instructions — total dynamic instruction count (algorithmic complexity)
CPI          — average Cycles Per Instruction (memory behavior, branching)
Clock_period — seconds per cycle = 1 / frequency (hardware speed)
```

**Which knob to turn:**

| Reduce | By | Trade-off |
|--------|----|-----------|
| Instructions | Better algorithm (O(n²) → O(n log n)) | May increase CPI (random access) |
| CPI | Cache optimization, branch prediction, SIMD | May increase instruction count (prefetch, padding) |
| Clock_period | Faster CPU, higher frequency | Power × frequency³, thermal limits |

---

## Amdahl's Law — Quick Reference

```
S(N) = 1 / ((1 - P) + P/N)

P = fraction of runtime that can be improved
N = speedup factor for the improved portion
S(N) = overall speedup
```

### Speedup Table

| P (improved fraction) | N=2 | N=4 | N=10 | N=100 | N→∞ |
|------------------------|-----|-----|------|-------|-----|
| 0.50 | 1.33 | 1.60 | 1.82 | 1.98 | 2.0 |
| 0.75 | 1.60 | 2.29 | 3.07 | 3.80 | 4.0 |
| 0.90 | 1.82 | 3.07 | 5.26 | 9.09 | 10.0 |
| 0.95 | 1.90 | 3.48 | 6.90 | 16.81 | 20.0 |
| 0.99 | 1.98 | 3.88 | 9.17 | 49.75 | 100.0 |

Key insight: The serial fraction (1-P) dominates. 5% serial caps speedup at 20x regardless of N.

---

## Gustafson's Law — Quick Reference

```
S(N) = N - (N-1) × s

N = number of processors
s = serial fraction of the SCALED workload
```

### When to use which:

| Scenario | Use | Why |
|----------|-----|-----|
| Fixed dataset, same work, more cores | Amdahl | Problem size doesn't grow |
| Growing dataset, more cores, higher throughput | Gustafson | Problem scales with resources |
| Reduce p99 latency of existing workload | Amdahl | Latency = fixed-work problem |
| Process 2× data in same time with 2× machines | Gustafson | Throughput = scaled-work problem |

---

## Latency vs Throughput

| Metric | Measures | Unit | Optimize by |
|--------|----------|------|-------------|
| Latency | Time per operation | ns, μs, ms, s | Reducing critical path, cache locality, fewer round trips |
| Throughput | Operations per time | ops/sec, MB/s | Batching, pipelining, parallelism |

**Trade-offs:**
- Batching improves throughput, increases per-item latency
- Pipelining improves throughput without increasing per-item latency (if pipeline stays full)
- Adding parallelism improves throughput, but p99 latency may increase (tail latencies compound)

**Latency percentiles:**
- p50 (median): typical experience
- p99: 1 in 100 requests — critical for SLAs
- p99.9: 1 in 1000 — captures rare slow events
- Average: misleading (one 10s outlier among 99 10ms requests → 109ms average, useless)

---

## Memory Hierarchy — Orders of Magnitude

```
Storage         Latency          vs L1     Capacity     $/GB      Bandwidth
─────────────────────────────────────────────────────────────────────────────
Register        ~0 cycles        1x        ~1 KB        —         —
L1 Cache        ~0.5 ns (3cy)    1x        ~64 KB       —         ~1 TB/s
L2 Cache        ~7 ns (20cy)     14x       ~1 MB        —         ~500 GB/s
L3 Cache        ~15 ns (45cy)    30x       ~32 MB       —         ~200 GB/s
DRAM            ~100 ns (300cy)  200x      ~64 GB       $3/GB     ~50 GB/s
NVMe SSD        ~100 μs          200,000x  ~4 TB        $0.10/GB  ~7 GB/s
SATA SSD        ~500 μs          1,000,000x~4 TB        $0.10/GB  ~0.5 GB/s
HDD             ~10 ms           20,000,000x~20 TB      $0.02/GB  ~0.2 GB/s
Network (DC)    ~0.5 ms          1,000,000x—             —         ~100 Gb/s
Network (WAN)   ~30-100 ms       200,000,000x—           —         ~1-10 Gb/s

Mnemonic: "L1 is a thought, DRAM is a yawn, SSD is a coffee break, network is lunch."
```

### Cache Basics

```
Cache line size: 64 bytes (most architectures)
L1 hit: 3 cycles
L1 miss → L2:  ~20 cycles
L2 miss → L3:  ~45 cycles
L3 miss → DRAM: ~300 cycles (memory wall)

Spatial locality: accessing adjacent bytes is nearly free (prefetcher grabs whole cache line)
Temporal locality: accessing the same byte again soon is nearly free (still in cache)
Neither: random access → cache misses → 200x slower than L1 hits
```

---

## Roofline Model — How to Read It

```
Attainable GFLOP/s
  │
  │              ┌────────────────── Compute ceiling
  │             ╱   (peak GFLOP/s)
  │           ╱
  │         │╱
  │        ╱│
  │      ╱  │  ← ridge point: where slope meets ceiling
  │    ╱    │
  │  ╱      │
  │╱        │    Compute-bound
  │          │
  ├──────────┼────────────────────
  Memory-   Ridge  Compute-bound
  bound     point
  (slope = bandwidth / compute)
  │
  └───────────────────────────── Operational Intensity (FLOP/byte)
     Low ←──────────────────→ High
```

**Operational intensity examples:**

| Algorithm | FLOP/byte | Region | Optimization |
|-----------|-----------|--------|-------------|
| Vector add (z = a + b) | 0.125 | Memory-bound (slope) | Increase bandwidth, cache blocking |
| Sparse matrix multiply | 0.5–2 | Memory-bound | Compression, better sparse format |
| Dense matrix multiply | 2–4 | Near ridge | Cache blocking (tiling), SIMD |
| Convolution (small kernel) | 4–10 | Compute-bound | Vectorize, reduce instructions |
| FFT | ~3 | Near ridge | SIMD, cache-friendly access |

**Decision tree:**

```
Is OI < ridge point?
├── Yes → Memory-bound
│   → Optimize: cache blocking, data layout (AoS→SoA), compression, prefetching
│   → Do NOT: unroll loops, add SIMD (memory is the bottleneck, not compute)
└── No → Compute-bound
    → Optimize: SIMD, loop unrolling, algorithmic change, higher frequency
    → Do NOT: cache-block (compute is the bottleneck, not memory)
```

---

## Big-O vs Constant Factors

```
The rule:
  Big-O chooses the algorithm family.
  Constant factors choose the implementation.
  Memory hierarchy chooses the constant factors.

Example:
  Quicksort: O(n log n) average, but random access (high CPI on large arrays)
  Insertion sort: O(n²), but sequential access (low CPI), no recursion

For n < ~20: insertion sort is faster than quicksort
Most production quicksorts switch to insertion sort for small subarrays.

Another example:
  Hash table: O(1) lookup, but pointer chasing (random access, high CPI)
  Sorted array: O(log n) binary search, but sequential access during scan (low CPI)

For n < ~100 on modern hardware: binary search in a sorted array
can outperform hash table lookup due to cache effects.
```

---

## Profiling-Driven Optimization Workflow

```
1. Write correct code
2. Set a performance target (e.g., "p99 < 100ms", "throughput > 10k req/s")
3. Measure baseline
4. If you meet target: STOP. Ship it.
5. Profile (perf, flamegraph, cachegrind)
6. Find #1 hotspot
7. Identify bottleneck type: compute-bound or memory-bound?
   → Compute-bound: reduce instruction count (algorithm, SIMD, eliminate branches)
   → Memory-bound: reduce memory traffic (cache blocking, locality, prefetch)
8. Optimize the #1 hotspot. ONE change at a time.
9. Measure. Did it help?
   → Yes: commit
   → No: revert. Try next idea.
10. Repeat 5-9 until target is met.
```

**Profiling tools:**

| Tool | What it measures | Command |
|------|-----------------|---------|
| `perf record` | Hardware counters, cache misses, branch mispredicts | `perf record -g ./myapp` |
| `perf stat` | Aggregate counters | `perf stat -e cache-misses,instructions ./myapp` |
| Flame graphs | Stack trace visualization (hotspot identification) | `perf record -g ./myapp && stackcollapse-perf.pl | flamegraph.pl` |
| `cachegrind` | Cache miss simulation per line | `valgrind --tool=cachegrind ./myapp` |
| `strace -c` | Syscall counts and timing | `strace -c ./myapp` |
| `iotop` | Disk I/O per process | `iotop` |
| `time` | Wall clock, user, system time | `/usr/bin/time -v ./myapp` |

---

## Performance Anti-Patterns

| Anti-pattern | Why it fails | Fix |
|-------------|-------------|-----|
| Optimizing without profiling | You optimize code that's 2% of runtime | Profile first, then optimize hotspots |
| Premature optimization | Wastes time on code that may not matter | Measure first. Optimize after profiling. |
| Late optimization | Shipped code is slow and nobody has time to fix it | Include performance targets in the design |
| Optimizing average latency | Average hides tail latencies | Optimize p99 or p99.9 |
| Ignoring constant factors | O(n log n) can lose to O(n²) for small n | Benchmark at your actual n |
| Batchless I/O | One syscall per 4KB wastes kernel time | Batch writes, use large buffers |
| Random memory access | Cache miss → 200x slower than L1 hit | Prefetch, tile, restructure data layouts |
| Measuring once | Performance is noisy (turbo boost, background tasks) | Measure 100 iterations, report median + p99 |
| Multiple changes at once | Can't tell which optimization helped | One change at a time, measure each |
| Assuming algorithmic improvement is enough | Better Big-O with bad constants can be slower | Profile actual runtime, not just complexity |

---

## Key Numbers to Memorize

```
L1 hit:           ~0.5 ns (3 cycles)
L2 hit:           ~7 ns (20 cycles)
L3 hit:           ~15 ns (45 cycles)
DRAM access:      ~100 ns (300 cycles)
SSD random read:   ~100 μs
SSD sequential read: ~1 μs per 4KB page
HDD seek:         ~10 ms
Network (same DC): ~0.5 ms
Network (cross-continental): ~50-150 ms

Context switch (OS thread): ~1-10 μs
Context switch (goroutine): ~50 ns
Mutex lock/unlock: ~25 ns (uncontended)
Atomic CAS (x86): ~15 ns

Rule of thumb: work per synchronization should be > 10,000 cycles
```

---

## When to Use Which Law

| Question | Answer framework |
|----------|-------------------|
| "Will more cores help?" | Amdahl — serial fraction limits parallel speedup |
| "Can I process bigger datasets with more cores?" | Gustafson — problem scales with resources |
| "Is my code compute or memory bound?" | Roofline — check operational intensity |
| "Should I optimize algorithm or cache?" | If compute-bound → algorithm. If memory-bound → cache. |
| "Which metric: latency or throughput?" | Interactive → latency. Batch → throughput. |
| "How do I find what's slow?" | Profile. Don't guess. |