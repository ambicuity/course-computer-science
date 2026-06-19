# PGO Benchmark Framework — Phase 15 Capstone Artifact

## What Is This?

A self-contained profile-guided optimization (PGO) benchmark framework that demonstrates the complete PGO workflow taught across Phase 15 (Lessons 01–19). It implements both a naive and an optimized string processor, benchmarks them side by side, and produces a comparison table mapping each speedup to the relevant phase lesson.

## The PGO Workflow

The program follows the six-step PGO loop:

1. **MEASURE** (L02) — Establish a trusted baseline with 31 iterations, reporting median and P99.
2. **PROFILE** (L03/L04) — Would attach `perf` / Instruments to identify hotspots; the framework identifies them implicitly by benchmarking each operation.
3. **IDENTIFY** — Pattern matching and case transformation are the dominant costs.
4. **OPTIMIZE** — Apply targeted optimizations, each drawn from a specific lesson.
5. **VERIFY** — Correctness checks confirm optimized output matches naive output.
6. **DOCUMENT** — Comparison table with P50/P99 and speedup ratios.

## Optimization → Lesson Mapping

| Optimization | Phase Lesson | How It Connects |
|---|---|---|
| Trusted baseline measurement | L02 — Measurement Discipline | Median + P99 over 31 iterations; no optimization without proof |
| Cache-friendly contiguous buffer | L05 — Cache-Aware Design | Naive uses `vector<string>` (pointer-chasing); optimized packs into one contiguous buffer |
| Branchless pattern matching | L07 — Branch Prediction | Arithmetic match flags instead of `if/else`; eliminates mispredicts on unpredictable data |
| SIMD first-character filter (AVX2) | L08 — Vectorization | `_mm256_cmpeq_epi8` + `_mm256_movemask_epi8` scans 32 positions per cycle |
| Arena/bump allocator | L09 — Memory Allocators | O(1) alloc/reset; zero fragmentation; replaces per-iteration `malloc`/`free` |
| Zero-copy mmap input | L10 — Zero-Copy & mmap | File mapped directly; no `read()` into intermediate buffer (C++ implementation) |
| Thread-local lock-free counters | L06/L13 — False Sharing & Lock Contention | Per-thread accumulation avoids cache-line ping-pong; merge once at end |
| P50/P99 latency reporting | L18 — Tail Latency | P99 catches rare slow paths that P50 hides |
| CPU frequency awareness | L17 — Power/Frequency Scaling | Pin frequency for reproducible benchmarks; document thermal throttling |
| Capacity planning (Little's Law) | L19 — Capacity Planning | Throughput = concurrency / latency; know your saturation point |
| Rust `UnsafeCell`/alignment | L16 — Rust High Performance | Arena allocator uses raw pointers with alignment guarantees |
| Coroutine pipeline design | L14 — Coroutines & Concurrency | Pipeline stages (read → process → write) without thread-per-stage overhead |
| Low-latency idioms | L15 — C++ Low-Latency | `__builtin_prefetch`, `[[likely]]`, `restrict` for hot paths |
| Profiling with perf/eBPF | L03 — Profiling Tools | The tools that *find* the hotspots PGO targets |

## Building and Running

### C++

```bash
cd code/
g++ -O2 -mavx2 -pthread -o pgo_capstone main.cpp
./pgo_capstone
```

### Rust

```bash
cd code/
RUSTFLAGS="-C target-cpu=native" cargo run --release
```

## Output Format

The program prints:

1. **Comparison table** — Benchmark name, median time, P99 time, and speedup vs baseline.
2. **Lesson mapping** — Each optimization connected to its Phase 15 lesson.
3. **Correctness verification** — Naive vs optimized count/transform results.
4. **PGO summary** — The six-step workflow recap.

## Interpreting Results

- **Single-character search** benefits most from pure SIMD (32 positions per cycle).
- **Multi-character search** benefits from SIMD first-character filter + branchless verification.
- **Case transform** benefits from SIMD arithmetic (no branches in the hot path).
- **Arena allocation** helps when the workload creates many temporaries.
- **Parallel search** helps on multi-core but may not scale linearly due to memory bandwidth limits.
- The *specific* speedup ratios depend on your hardware (cache sizes, SIMD width, core count).