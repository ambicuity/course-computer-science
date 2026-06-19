# Phase Capstone — A Profile-Guided Optimization Walk-Through

> You spent Lessons 01–19 learning individual weapons. Now you pick them up together and fight a real battle.

**Type:** Build
**Languages:** Rust, C++
**Prerequisites:** Phase 15 lessons 01–19
**Time:** ~150 minutes

## Learning Objectives

- Execute a full profile-guided optimization (PGO) cycle: measure → profile → identify hotspots → optimize → verify.
- Apply at least five distinct optimization techniques from this phase in a single codebase.
- Produce a documented before/after comparison that justifies every speedup with data.
- Recognize which optimizations compose (cache-friendly layout + branchless + SIMD) and which conflict (naive threading on shared caches).

## The Problem

You have a string-processing workload that mirrors real-world log analytics: scan lines, find patterns, count occurrences, apply transforms. A naive implementation works but is slow — and you don't know *why* it's slow without measuring. This capstone walks through the entire PGO loop:

1. **Start naive** — write the clearest version first.
2. **Profile** — instrument with timing counters, generate flamegraphs.
3. **Identify hotspots** — use the profiling skills from L03/L04.
4. **Optimize** — apply cache awareness (L05), fix false sharing (L06), rewrite branches (L07), vectorize (L08), swap the allocator (L09), eliminate copies (L10).
5. **Verify** — benchmark every change with the discipline from L02.
6. **Document** — produce a comparison table connecting each speedup to the lesson that taught it.

This is the loop that performance engineers run daily. The individual techniques are necessary but not sufficient — you must also know *when* and *where* to apply them, and that knowledge comes only from profiling.

## The Concept

### Why PGO Matters

Lesson 01 taught us that performance intuition is unreliable. "It feels slow" is not data. PGO forces you to *measure first, optimize second*, which is the discipline from L02. The workflow is a loop, not a one-shot:

```
  ┌─── measure ──── profile ──── identify ──── optimize ──── verify ───┐
  │                                                                        │
  └─────────────────────── repeat until diminishing returns ──────────────┘
```

Each iteration targets the *current* bottleneck. Optimizing something that isn't the bottleneck is wasted effort — this is Amdahl's Law with测量proof.

### Connecting the Phase

| Lesson | Concept | PGO Application |
|--------|---------|-----------------|
| L01 — How to Think About Performance | Intuition is unreliable; measure everything | PGO is the structured application of this principle |
| L02 — Measurement Discipline | Benchmarks that don't lie | Every optimization must be verified with a controlled benchmark |
| L03 — Profiling (perf, dtrace, eBPF) | Finding where time is spent | Step 2 of PGO: attach profilers, collect samples |
| L04 — Flamegraphs & Hotspots | Visualizing stack depth and cost | Step 3: the flamegraph tells you *which* function to attack |
| L05 — Cache-Aware Design | Cache misses dominate | Restructuring data layout (AoS → SoA) eliminates cache misses |
| L06 — False Sharing & NUMA | Unintended cache-line ping-pong | Per-thread counters avoid false sharing in the parallel path |
| L07 — Branch Prediction | Mispredictions stall pipelines | Converting `if/else` to arithmetic (branchless) removes stalls |
| L08 — Vectorization (SIMD) | Process 4–8 elements per cycle | Replacing scalar loops with SIMD intrinsics or auto-vectorized code |
| L09 — Memory Allocators | Allocation overhead and fragmentation | Switching from `malloc` to `mimalloc` or arena allocation |
| L10 — Zero-Copy & mmap | Avoiding kernel↔user copies | `mmap`-based input removes `read()` copies |
| L11 — io_uring | Async I/O for high throughput | Batching I/O submissions with io_uring for file reads |
| L12 — Kernel Bypass | DPDK/SPDK for extreme latency | (Not needed here — our workload isn't that extreme) |
| L13 — Lock Contention | Mutexes serialize threads | Lock-free counters eliminate contention in the parallel path |
| L14 — Coroutines | Stackful vs stackless concurrency | Coroutines orchestrate pipeline stages without thread overhead |
| L15 — C++ Low-Latency Idioms | `restrict`, `likely`, prefetch | `__builtin_prefetch` and `[[likely]]` guide the CPU |
| L16 — Rust High Performance | `UnsafeCell`, `MaybeUninit`, alignment | Rust's `UnsafeCell` for interior mutability, aligned allocation |
| L17 — Power & Frequency Scaling | Thermal throttling | Pin CPU frequency to avoid benchmark noise |
| L18 — Tail Latency & Hedging | P99 matters more than P50 | Report P50 and P99 for every benchmark |
| L19 — Capacity Planning | Little's Law | Throughput = concurrency / latency — know your saturation point |

### The PGO Workflow in Detail

#### Step 0: Establish the Baseline

Before touching any code, you need a *trusted* number. This is Lesson 02 in action:

- Run the benchmark at least 30 iterations to get a stable mean.
- Report median and P99 (Lesson 18).
- Pin CPU frequency (Lesson 17) or at least record it.
- Disable turbo boost for reproducibility.

The baseline is the single most important number in the entire optimization campaign. Without it, you cannot demonstrate improvement.

#### Step 1: Write Naive Code

Write the *simplest* correct implementation. No cleverness. This is your control:

```cpp
// Naive: readable, correct, slow
size_t count_pattern(const std::string& text, const std::string& pattern) {
    size_t count = 0;
    for (size_t i = 0; i + pattern.size() <= text.size(); ++i) {
        if (text.substr(i, pattern.size()) == pattern) {
            ++count;
        }
    }
    return count;
}
```

This version is O(n·m) and allocates on every iteration. That's fine — the point is to *first make it correct*, then make it fast.

#### Step 2: Profile

Use the tools from L03 and L04:

```bash
perf record -g ./string_processor_naive
perf script | stackcollapse-perf.pl | flamegraph.pl > naive.svg
```

The flamegraph will show you where time is actually spent. Common findings in string workloads:

- **Allocations dominate** (L09) — `substr` creates temporary strings.
- **Cache misses** (L05) — string data is scattered across the heap.
- **Branch mispredicts** (L07) — pattern matching has unpredictable branches.
- **Scalar loops** (L08) — the compiler couldn't auto-vectorize.

#### Step 3: Identify Hotspots

The flamegraph from L04 tells you which function accounts for the most samples. Start there. If `count_pattern` accounts for 60% of runtime, optimizing it can at most give you a 2.5× speedup (Amdahl's Law). Optimizing a function that's 5% of runtime can only give you 1.05×.

#### Step 4: Optimize (Apply Phase Lessons)

Each optimization targets a specific bottleneck the profiler revealed:

**Cache-friendly layout (L05):** Instead of a `vector<string>` where each string's data lives in a separate heap allocation, pack all text into a single contiguous buffer. Iteration now hits L1/L2 cache instead of chasing pointers to L3/DRAM.

**Branchless patterns (L07):** Replace conditional increments with arithmetic:

```cpp
// Branchless: no mispredict, just arithmetic
count += (text[i:i+pattern_size] == pattern) ? 1 : 0;
// Or even: compute match as a mask, use popcount
```

**SIMD (L08):** Process 16 or 32 characters at once using SSE/AVX intrinsics. The first character of the pattern is broadcast and compared against 32 positions simultaneously; only matching positions proceed to full comparison.

**Arena allocator (L09):** Replace per-operation allocations with a bump allocator that recycles memory across iterations. Fragmentation drops to zero, allocation is a single pointer increment.

**Zero-copy input (L10):** `mmap` the input file instead of `read()`-ing into a buffer. One fewer kernel copy, and the OS handles paging.

**Lock-free counters (L13):** In the parallel version, use thread-local counters and merge at the end. No mutex, no contention, no false sharing (L06).

#### Step 5: Verify

After each optimization, re-run the benchmark from Step 0. Compare against the baseline. If the change made things worse, *revert it*. Not every optimization applies to every workload — that's the point of PGO.

Report both P50 and P99 (L18). An optimization that improves P50 by 20% but doubles P99 is a regression in latency-sensitive systems.

#### Step 6: Document

Create a table:

| Optimization | Lesson | P50 Before | P50 After | P99 Before | P99 After | Technique |
|---|---|---|---|---|---|---|
| Baseline | L02 | 450ms | — | 480ms | — | Trusted measurement |
| Cache-friendly layout | L05 | 450ms | 180ms | 480ms | 210ms | AoS → SoA |
| Branchless count | L07 | 180ms | 120ms | 210ms | 145ms | Conditional → arithmetic |
| SIMD scan | L08 | 120ms | 65ms | 145ms | 82ms | SSE4.1 pcmpeqb |
| Arena allocator | L09 | 65ms | 58ms | 82ms | 74ms | Bump allocator |
| Zero-copy input | L10 | 58ms | 52ms | 74ms | 61ms | mmap |
| Parallel + lock-free | L13/L06 | 52ms | 14ms | 61ms | 18ms | Thread-local counters |

This table is your artifact. It proves that each optimization was *necessary* (the profiler said so), *effective* (the benchmark confirms), and *connected to theory* (the lesson explains why).

## Build It

### Step 1: Naive String Processor

We'll build a program that:
1. Reads text from a file.
2. Counts occurrences of a pattern.
3. Applies a case transformation.
4. Reports statistics.

The naive version uses `std::string` operations, `std::string::substr`, and `std::transform`. It is correct and readable. See `code/main.cpp` for the full implementation.

### Step 2: Instrumented Version with Timing

Wrap each operation with a high-resolution timer (Lesson 02's measurement discipline). Report P50 and P99 across 30 runs.

### Step 3: Optimized Version

Apply all optimizations from the table above. Each optimization is a separate, measurable step. The `OptimizedStringProcessor` class in `code/main.cpp` includes:

- Contiguous buffer instead of `vector<string>` (L05).
- Branchless pattern matching (L07).
- SIMD first-character filter + full verification (L08).
- Arena/stack allocator for temporaries (L09).
- mmap-based input (L10).
- Thread-local counters with merge (L13/L06).

### Step 4: Before/After Benchmark Framework

The `BenchmarkFramework` class runs both implementations, collects statistics (P50, P99, mean, stddev), and prints a formatted comparison table.

## Use It

### Production PGO Workflows

**GCC/Clang PGO:** The compiler itself can do profile-guided optimization:

```bash
# Step 1: Build with instrumentation
clang++ -fprofile-instr-generate=code.profraw main.cpp -o main_instr
# Step 2: Run with representative inputs to collect profiles
LLVM_PROFILE_FILE=code.profraw ./main_instr
# Step 3: Merge profile data
llvm-profdata merge -sparse code.profraw -o code.profdata
# Step 4: Rebuild with profile guidance
clang++ -fprofile-instr-use=code.profdata main.cpp -o main_pgo
```

This tells the compiler which branches are likely, which functions are hot, and which code paths to optimize for. This is complementary to our manual PGO — the compiler optimizes *within* functions while we optimize *between* functions (by restructuring data and algorithms).

**AutoFDO (Google):** Uses hardware perf events to collect profiles without instrumentation overhead. This is what Google uses for production PGO on their C++ codebase — see the Linux kernel's `tools/perf` integration.

**Bolt (Meta):** A post-link optimizer that takes a binary and a profile, then lays out code for better cache behavior and branch prediction. It's the production version of the "reorder functions by hotness" technique from L07.

### Rust PGO

```bash
# Same workflow with rustc
RUSTFLAGS="-C profile-generate=/tmp/pgo_data" cargo build --release
./target/release/my_app  # run workload
RUSTFLAGS="-C profile-use=/tmp/pgo_data/merged.profdata" cargo build --release
```

Rust's PGO is especially valuable for match-heavy code (L07) and enum dispatch — the compiler learns which arms are hot and optimizes accordingly.

## Read the Source

- **LLVM PGO infrastructure:** `llvm/lib/ProfileData/` — how profile data is collected, merged, and consumed.
- **Bolt post-link optimizer:** `bolt/lib/` — how function layout is reordered based on profiles.
- **mimalloc:** `mimalloc/src/page.c` — how arena-based allocation recycles memory pages.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. It is:

- **A PGO benchmark framework** — a self-contained program that implements naive and optimized string processing, benchmarks both, and produces a comparison table mapping each speedup to the relevant phase lesson.

## Exercises

1. **Easy** — Run the provided program, collect the baseline numbers, and identify which optimization contributes the largest single speedup on your hardware. Explain why (considering your CPU's cache sizes and SIMD width).

2. **Medium** — Remove the SIMD optimization and replace it with a different vectorization strategy (e.g., AVX2 instead of SSE). Compare the results. Does the wider vector width help on your workload size?

3. **Hard** — Extend the framework with a concurrent pipeline (L14): one thread reads via io_uring (L11), one thread processes, one thread writes results. Use lock-free channels between stages. Measure whether the pipeline improves P99 or just P50.

## Key Terms

| Term | What people say | What it actually means |
|------|-----------------|------------------------|
| PGO | "Profile-guided optimization" | The disciplined loop: measure → profile → identify → optimize → verify. Not just compiler PGO — applies to your entire optimization workflow. |
| Hotspot | "The slow function" | The code region accounting for the largest fraction of execution time, as identified by a profiler — not by intuition (L01). |
| Baseline | "The original time" | A trusted, reproducibly-measured number (L02) against which all optimizations are compared. Without it, you can't prove improvement. |
| Branchless | "No if-statements" | Replacing conditional branches with arithmetic operations that the CPU can execute without mispredicting (L07). Not always better — profile first. |
| Arena allocator | "Custom malloc" | A bump allocator that recycles a contiguous memory region. O(1) alloc, O(1) dealloc, zero fragmentation (L09). |
| P99 | "The slow 1%" | The latency that 99% of operations beat. More important than P50 for latency SLAs (L18). |

## Further Reading

- **LLVM PGO documentation:** https://clang.llvm.org/docs/UsersManual.html#profile-guided-optimization
- **AutoFDO:** "AutoFDO: Automatic Feedback-Directed Optimization for Warehouse-Scale Applications" (Google, 2022)
- **Bolt:** "BOLT: A Practical Binary Optimizer for Data Centers and Beyond" (Meta, 2023)
- **Amdahl's Law:** The original 1967 paper — the mathematical foundation for why PGO works.
- **Gallery of Processor Cache Effects:** Igor Ostrovsky's classic visualization — intuition for why cache-friendly layout matters (L05).
- **The Lost Art of Loop Nest Optimization:** How compilers reason about cache tiling — connects L05 and L08.

## Appendix: Full PGO Iteration Log Template

When doing PGO in production, maintain a log like this for each iteration:

```
Iteration N:
  Date:         YYYY-MM-DD
  Baseline:     Xms (from iteration 0)
  Change:       [describe the optimization applied]
  Profiler:     perf record -g ./program → flamegraph.svg
  Hotspot:      [function + % of total samples]
  Measurement:  median=Yms, P99=Zms, iterations=31
  Speedup:      X/Y = Nx faster
  Regression:   [any P99 increase?]
  Verdict:      KEEP / REVERT / INCONCLUSIVE
  Lesson ref:   [which L01-L19 concept]
```

This log is your evidence. When someone asks "why did you change the data structure?" you point to the profiler output and the before/after numbers. No intuition, no hand-waving — just data.

## Appendix: Common PGO Pitfalls

### Pitfall 1: Optimizing Without Profiling

"I think the bottleneck is in the parser." Without a flamegraph (L04), you're guessing. The profiler might reveal that 80% of time is in memory allocation (L09), not parsing at all. Every optimization applied without profiler evidence is potentially wasted.

### Pitfall 2: Measuring Once

Running a benchmark once gives you a number with high variance. L02's measurement discipline requires at least 30 iterations, reporting both P50 and P99. If the P99 is 5× the P50, your measurement is too noisy to trust.

### Pitfall 3: Ignoring Tail Latency

An optimization that improves P50 by 30% but adds a rare 10ms pause (GC, page fault, context switch) makes P99 worse. L18 teaches us to always check P99. If you only report P50, you're hiding the worst-case.

### Pitfall 4: Over-Parallelizing

Throwing threads at a problem without checking for lock contention (L13) or false sharing (L06) can make things *worse*. The parallel version of our string processor only helps when the data is large enough to amortize thread creation overhead. Profile first. Measure both P50 and P99.

### Pitfall 5: Benchmarking on a Noisy Machine

If the CPU is frequency-scaling (L17), your benchmark numbers are untrustworthy. Pin the CPU governor to `performance` mode, disable turbo boost, and close background processes. If you can't control the environment, at least record CPU frequency during the benchmark.

### Pitfall 6: Micro-Optimizing Before Macro-Optimizing

Moving from SSE to AVX2 (L08) might give you 2× in the inner loop. But if the inner loop is only 10% of total runtime (Amdahl's Law), you get 1.05× overall. First restructure the data (L05), then eliminate allocations (L09), then vectorize. The order matters because each step changes the bottleneck.

## Appendix: Connecting PGO to System Design

PGO doesn't stop at single-function optimization. The same cycle applies to system-level decisions:

- **Capacity planning (L19):** Little's Law tells you the maximum throughput given latency and concurrency. Profile to find your actual latency, then compute whether you need more capacity.
- **Kernel bypass (L12):** If your profiler shows 40% of time in kernel syscalls, it might be time to consider DPDK/SPDK. But only after you've verified your data structures are cache-friendly and your I/O isn't buffered unnecessarily.
- **Coroutines (L14):** If profiling shows threads spending most time blocked on I/O, coroutines can reduce context-switch overhead. But measure first — the coroutine overhead might exceed the I/O wait time on fast networks.
- **Tail latency hedging (L18):** If P99 is dominated by rare slow requests, send hedged requests and take the faster response. This is a system-level PGO outcome — you only know P99 is a problem because you measured it.