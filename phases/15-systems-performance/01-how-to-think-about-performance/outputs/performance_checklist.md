# Performance Investigation Checklist

> Measure first. Optimize second. Never trust intuition about where time goes.

---

## The 8-Step Workflow

```
1. WRITE CORRECT CODE FIRST
   └── Don't optimize before the code works.

2. SET A PERFORMANCE TARGET
   └── "p99 latency < 100ms" or "throughput > 10k req/s"
   └── No target = no definition of done.

3. MEASURE BASELINE
   └── Run 100+ iterations. Report median and p99.
   └── Control the environment (no background tasks, CPU governor = performance).

4. IF YOU MEET THE TARGET: STOP. SHIP IT.

5. PROFILE
   └── Find the top N hotspots (functions that consume the most time).
   └── Use: perf record, flame graphs, cachegrind, strace.

6. CLASSIFY THE BOTTLENECK
   └── Compute-bound? (high CPI, many cycles per instruction)
   └── Memory-bound? (low operational intensity, many cache misses)
   └── I/O-bound? (time in syscalls, blocking reads/writes)

7. OPTIMIZE THE #1 HOTSPOT — ONE CHANGE AT A TIME
   └── Compute-bound → reduce instruction count (algorithm, SIMD, eliminate branches)
   └── Memory-bound → reduce memory traffic (cache blocking, data layout, prefetch)
   └── I/O-bound → batch, buffer, overlap with computation (async)

8. MEASURE AGAIN
   └── Did it help? Yes → commit. No → revert.
   └── Repeat from step 5 until target is met.
```

---

## Decision Trees

### Is it Compute-Bound or Memory-Bound?

```
Operational Intensity = FLOPs / bytes accessed

If OI < Ridge Point (Peak Compute / Peak Bandwidth):
  → MEMORY-BOUND
  → Optimize: cache blocking, data layout (AoS→SoA), compression, prefetching
  → Do NOT: unroll loops, add SIMD (already waiting for data)

If OI > Ridge Point:
  → COMPUTE-BOUND
  → Optimize: SIMD, loop unrolling, algorithmic change, reduce branches
  → Do NOT: cache-block (already compute-limited, not data-starved)
```

### Latency or Throughput?

```
Is the user waiting for a response?
├── Yes → Optimize LATENCY (reduce critical path, fewer round trips, cache hits)
└── No → Optimize THROUGHPUT (batching, pipelining, parallelism)

Interactive systems  → latency (API calls, UI rendering, real-time gaming)
Batch systems        → throughput (ETL, log processing, ML training)
Most web services    → both (low latency per request, high throughput overall)
```

---

## Amdahl's Law — Quick Formula

```
S = 1 / ((1 - P) + P/N)

P = fraction of runtime that can be improved
N = speedup factor of the improved portion
S = overall speedup

Maximum speedup (N → ∞): S_max = 1 / (1 - P)

0.95 parallelizable → max 20x speedup
0.99 parallelizable → max 100x speedup
0.999 parallelizable → max 1000x speedup
```

## Gustafson's Law — Quick Formula

```
S = N - (N-1) × s

N = number of processors
s = serial fraction of the SCALED workload

Use when: problem size grows with resources
(e.g., bigger dataset with more machines)
```

---

## Memory Hierarchy Cost Table

```
Level        Latency      vs L1     Bandwidth
──────────────────────────────────────────────
L1 Cache     ~0.5 ns      1x        ~1 TB/s
L2 Cache     ~7 ns        14x       ~500 GB/s
L3 Cache     ~15 ns       30x       ~200 GB/s
DRAM         ~100 ns      200x      ~50 GB/s
NVMe SSD     ~100 μs     200,000x  ~7 GB/s
Network DC   ~500 μs      1Mx       ~100 Gb/s
Network WAN  ~50 ms       100Mx     ~10 Gb/s

Mnemonic: "L1 is a thought, DRAM is a yawn, SSD is a coffee, network is lunch."
```

---

## Profiling Tool Cheat Sheet

| Tool | What it shows | Command |
|------|---------------|---------|
| `perf record` | Hardware counters, hot functions | `perf record -g ./myapp` |
| `perf stat` | Aggregate counters (instructions, cache misses) | `perf stat -e cache-misses,instructions,cycles ./myapp` |
| Flame graph | Visual hotspot (which functions are hot) | `perf record -g ./myapp` then `stackcollapse-perf.pl | flamegraph.pl` |
| `cachegrind` | Cache miss simulation per source line | `valgrind --tool=cachegrind ./myapp` |
| `strace -c` | Syscall counts and time | `strace -c ./myapp` |
| `iotop` | Disk I/O per process | `sudo iotop` |
| `/usr/bin/time -v` | Wall, user, system time + memory | `/usr/bin/time -v ./myapp` |

---

## Anti-Pattern Checklist

```
□ Optimizing without profiling first
  → You're guessing. Profile. Then optimize the #1 hotspot.

□ Optimizing the wrong bottleneck
  → Memory-bound code won't benefit from more compute. Use roofline model.

□ Making multiple changes at once
  → One change at a time. Measure after each. Revert if no improvement.

□ Optimizing average latency
  → Average hides tail latencies. Use p50 AND p99.

□ Assuming Big-O is enough
  → O(n log n) with CPI=5 beats O(n²) with CPI=1 only at large n.
  → Benchmark at your actual input size.

□ Ignoring the memory hierarchy
  → Random access (CPI~5-10) vs sequential access (CPI~1) can mean 5-10x difference.

□ Measuring once
  → Modern CPUs are noisy. Run 100+ iterations. Report median + p99.
  → Disable turbo boost or note it. Pin processes to cores.
```

---

## Key Numbers to Memorize

```
L1 hit:       ~0.5 ns     (3 cycles)
L2 hit:       ~7 ns       (20 cycles)
L3 hit:       ~15 ns      (45 cycles)
DRAM:         ~100 ns      (300 cycles)
SSD:          ~100 μs
HDD:          ~10 ms
Network DC:   ~500 μs
Network WAN:  ~50-150 ms

Mutex lock:   ~25 ns (uncontended)
Atomic CAS:   ~15 ns
Thread ctx switch: ~1-10 μs
Goroutine switch:  ~50 ns
Syscall:      ~200 ns - 1 μs

Rule: Work per sync should be > 10,000 cycles.
```