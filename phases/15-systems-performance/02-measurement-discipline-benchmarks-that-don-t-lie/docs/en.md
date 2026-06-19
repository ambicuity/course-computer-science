# Measurement Discipline — Benchmarks That Don't Lie

> A benchmark that lies is worse than no benchmark at all — it gives you confidence in the wrong direction.

**Type:** Learn  
**Languages:** Rust, C++  
**Prerequisites:** Phase 15 lesson 01  
**Time:** ~75 minutes

## Learning Objectives

- Identify why naive benchmarks produce misleading numbers (cache warming, CPU scaling, compiler optimization).
- Prevent Dead Code Elimination (DCE) and other compiler tricks that silently hollow out your benchmark.
- Choose the right statistic: mean vs median vs p99, and know when each lies to you.
- Distinguish microbenchmarks from macrobenchmarks and use each where appropriate.
- Design warmup and iteration strategies that yield statistically honest results.
- Implement a benchmark harness from scratch that the compiler cannot cheat on.

## The Problem

You write a tight loop, time it, divide by iterations, and publish the number. Done, right?

No. That number is almost certainly wrong. Here is why:

1. **The compiler saw your hot loop doing no observable work and deleted it.** You measured zero instructions executing in zero nanoseconds.
2. **The CPU cached everything on iteration two.** Iteration one paid for the cache miss; iterations 2–1,000,000 ran from L1. You reported the L1 number and called it the "real" cost.
3. **The CPU boosted to turbo frequency halfway through your run.** The first 20% of iterations ran 30% slower because the chip was still ramping up. Your "average" is an average of two different machines.
4. **You averaged outliers into your mean.** One OS scheduler hiccup added 47 ms to a 200-iteration run. You never noticed because the mean smoothed it away.

This lesson builds the discipline to stop each of these from lying to you.

## The Concept

### Why Benchmarks Lie: A Catalog of Pitfalls

#### Pitfall 1 — Dead Code Elimination (DCE)

The compiler's job is to produce the fastest correct program. If your benchmark computes a result that is never used, the compiler is *correct* to remove the computation entirely:

```
int sum = 0;
for (int i = 0; i < N; i++) {
    sum += arr[i];      // compiler: "sum is never read → delete loop"
}
// sum is never used → entire loop vanishes
```

**Result:** You benchmark an empty loop body. You measure ~0 ns. You think your code is fast. It isn't — it isn't there.

**Prevention:**
- `volatile` sink: write the result to a volatile variable so the compiler must produce it.
- Inline assembly barrier: a `asm volatile("" ::: "memory")` tells the compiler that all memory is clobbered, preventing it from hoisting or removing surrounding code.
- `black_box()` (Rust): `std::hint::black_box` is specifically designed for this — it's an identity function the optimizer must treat as opaque.

#### Pitfall 2 — Warm Caches

Modern CPUs have 3–4 levels of cache. Cold access (first iteration) hits DRAM at ~100 ns. Warm access hits L1 at ~1 ns. A typical naive benchmark:

```
+------------+--------+--------+--------+-----+--------+
| Iteration  |   1    |   2    |   3    | ... |  1000  |
+------------+--------+--------+--------+-----+--------+
| Data from  |  DRAM  |  L3    |  L1    | ... |  L1    |
+------------+--------+--------+--------+-----+--------+
| Latency    | 100ns  | 10ns   | 1ns    | ... |  1ns   |
+------------+--------+--------+--------+-----+--------+
```

Your "average" of 1.1 ns per access hides the fact that a *real* user who touches this data once pays 100 ns.

**Prevention:** Report cold (first iteration) separately from warm (steady-state). Never average across the warmup boundary.

#### Pitfall 3 — CPU Frequency Scaling

Modern CPUs ramp from idle to turbo boost over ~10–50 ms. During that ramp, your code runs slower:

```
CPU Frequency During Benchmark
  5 GHz |                    _______________
        |                  /
  4 GHz |           ______/
        |          /
  3 GHz |   _____/
        |  /
  2 GHz |_/
        +----------------------------------------> time
          ~20-50ms warmup
```

**Prevention:** Run warmup iterations first (discarded from stats) so the CPU reaches steady-state before you start measuring.

#### Pitfall 4 — Branch Prediction

A sorted array lets the CPU's branch predictor learn the pattern quickly. An unsorted array causes branch mispredictions (~15-cycle penalty each):

```
Binary search on SORTED array:
  Branch predictor: "always left, then right at level 3"
  Mispredict rate: <1%
  → Fast branch prediction, pipelined execution

Binary search on RANDOM array (same values, shuffled):
  Branch predictor: no pattern to learn
  Mispredict rate: ~15-20%
  → Pipeline stalls, ~15 cycle penalty per mispredict
```

This is a *real* cost that shows up in production, but only if your data is unsorted.

#### Pitfall 5 — Inlining and Const Propagation

If the compiler knows the input at compile time, it can precompute the answer:

```cpp
int add(int a, int b) { return a + b; }
// Call site: add(3, 4)
// Compiler: "I know this returns 7, I'll just emit a constant"
```

In a microbenchmark where inputs are known at compile time, the compiler can fold the entire computation into a constant.

**Prevention:** Pass inputs through a `volatile` read or `black_box` so the compiler cannot see through them.

### Statistics: Which Number Do You Report?

#### Mean (Arithmetic Average)

- **Good for:** Symmetric distributions with no outliers
- **Bad for:** Skewed distributions; a single outlier pulls the mean dramatically

Example: 99 runs at 100 ns + 1 run at 10,000 ns
- Mean = (99 × 100 + 10,000) / 100 = 199 ns
- The 99 "normal" runs are ~100 ns each. The mean says 199 ns. That's almost 2x the typical cost, driven by one outlier.

#### Median

- **Good for:** Robust estimate of "typical" performance; resistant to outliers
- **Bad for:** Hiding the existence of outliers entirely

The median of the above distribution is 100 ns — accurate for the typical case, but it hides the 10,000 ns outlier. You must also report the tail.

#### Percentiles (p99, p95)

- **Good for:** Describing the tail — what the worst 1% or 5% of users experience
- **Bad for:** Cannot be used alone; you also need the center (median) for context

**Rule of thumb:** Always report at minimum: **min, median, mean, p99, max**. The gap between mean and median tells you about skew. The gap between p99 and max tells you about outliers.

#### Standard Deviation

Useful but misleading for non-normal distributions. If your benchmark has a bimodal distribution (e.g., cache-hit vs cache-miss), standard deviation is essentially meaningless.

```
Bimodal distribution (cache hit vs miss):

Frequency
  |   *                                           *
  |   *                                           *
  |   *                                           *
  |   *                                           *
  +---+---+---+---+---+---+---+---+---+---+---+---
     1ns                5ns                100ns

  σ = 35ns — but "35ns deviation" tells you nothing useful
  about a distribution that is clearly two separate clusters.
```

### Microbenchmark vs Macrobenchmark

| Aspect | Microbenchmark | Macrobenchmark |
|--------|---------------|----------------|
| Scope | One function, one operation | Full system or subsystem |
| Iterations | Millions (for statistical power) | Tens to hundreds |
| Noise | Low (controlled environment) | High (OS, network, disk) |
| Risk | DCE, inlining, unrealistic data | Setup/teardown dominates |
| Best for | Comparing algorithms, choosing data structures | End-to-end SLA, regression testing |
| Caveat | "Fast in micro, slow in macro" is common | Must accept higher variance |

**Guideline:** Use microbenchmarks to choose between algorithms. Use macrobenchmarks to verify real-world performance. Never trust a microbenchmark alone.

### Warmup Iterations

The first N runs of any benchmark are meaningless for "steady-state" performance:

1. **Cache filling:** Data must be loaded from DRAM (cold → warm transition).
2. **JIT compilation (Rust/LLVM):** Tiered compilation can recompile hot paths mid-benchmark.
3. **CPU turbo boost:** The core ramps from idle to max frequency over ~10–50 ms.
4. **Page faults:** Memory pages are demand-paged on first access.

**Design rule:** Run K warmup iterations (K ≥ 10, or enough to exceed 100 ms wall time), then measure M real iterations. Discard the K warmup data entirely.

### Outlier Removal

Should you discard outliers? It depends on *why* they exist:

- **OS scheduler preemption:** Discard. This is noise your users won't experience in the same way — your process might not be preempted during this particular slice.
- **Page fault on first access:** Already handled by warmup. If it persists, your memory layout is thrashing the TLB — that's a real cost, keep it.
- **Unexpected latency spike that repeats:** Keep it. It's a real performance bug.

**Safe removal strategy:** Use the MAD (Median Absolute Deviation) rule: discard points more than N × MAD from the median (N = 3 is conservative). Never remove more than 5% of samples.

### Confidence Intervals

How many iterations do you need to be 95% confident your median is within 1% of the true value?

For a roughly normal distribution:

```
n ≥ (1.96 × σ / (0.01 × μ))²

Where:
  1.96 = z-value for 95% confidence
  σ    = estimated standard deviation
  0.01 = desired relative precision (1%)
  μ    = estimated mean
```

**Practical rule:** Start with 100 iterations. Compute the coefficient of variation (CV = σ/μ). If CV < 0.05 (5%), you're done. If CV > 0.05, double iterations and re-measure. Most stable microbenchmarks converge in 100–500 iterations.

### Production Frameworks: Google Benchmark and Criterion

#### Google Benchmark (C++)

What it gives you that a naive loop doesn't:
- Aggregated statistics (mean, median, stddev, p99)
- Warmup iterations (configurable)
- Iteration auto-tuning (finds the number of iterations for statistically stable results)
- Template-based fixture setup/teardown
- Repetition across multiple runs to check consistency

#### Criterion.rs (Rust)

-统计分析 with bootstrapped confidence intervals
- Warmup phase followed by measured phase
- Outlier detection and classification (Severe/Mild/OK)
- Change detection: compares current run against historical baseline
- HTML reports with regression detection

**Key takeaway:** Production frameworks exist because getting benchmarking right is hard. Use them for real work. But understanding what they do — and *why* — is what this lesson teaches.

### Reporting: Tables, Not Single Numbers

A single number like "150 ns/op" is worse than useless — it's misleading. Always report a table:

```
Benchmark                  min     median    mean     p99      max     stddev
───────────────────────────────────────────────────────────────────────────────
seq_access              4.2 ns   4.5 ns   4.8 ns   7.1 ns   45 ns   1.2 ns
random_access           28 ns    32 ns    38 ns    95 ns   500 ns   25 ns
binary_search_sorted    12 ns    14 ns    15 ns    22 ns    80 ns   3.1 ns
binary_search_random    18 ns    23 ns    26 ns    67 ns   300 ns   15 ns
───────────────────────────────────────────────────────────────────────────────
```

The gap between seq_access min and random_access min shows cache effects. The gap between sorted and random binary search shows branch prediction costs. The p99-to-max ratio tells you about tail behavior. None of this is visible from a single "average" number.

## Build It

### Step 1: Minimal Benchmark Harness

The minimal version times a function and computes basic stats:

```cpp
// Minimal: time a function, compute mean
auto start = std::chrono::high_resolution_clock::now();
for (int i = 0; i < N; i++) { f(); }
auto end = std::chrono::high_resolution_clock::now();
double mean_ns = std::chrono::duration<double, std::nano>(end - start).count() / N;
```

**Problem:** The compiler might optimize away `f()` if its result is unused. And the first iterations are cold.

### Step 2: Realistic Benchmark Harness

The realistic version adds:
- `volatile` sink / `asm` barrier to prevent DCE
- Warmup iterations (discarded from statistics)
- Full stat reporting: min, median, mean, p99, max, stddev
- Sorted samples for percentile computation
- Multiple benchmark scenarios (sequential vs random, sorted vs unsorted)

See the code in `code/main.cpp` and `code/main.rs`.

## Use It

### Google Benchmark (C++)

Google Benchmark is the standard C++ microbenchmark framework:

```cpp
#include <benchmark/benchmark.h>

static void BM_SequentialAccess(benchmark::State& state) {
    std::vector<int> v(state.range(0));
    for (auto _ : state) {
        long sum = 0;
        for (int x : v) {
            benchmark::DoNotOptimize(sum += x);
        }
    }
}
BENCHMARK(BM_SequentialAccess)->Arg(10000);
```

Key: `benchmark::DoNotOptimize` is the production equivalent of our `volatile`/`asm` barrier. It forces the compiler to keep the computation.

### Criterion (Rust)

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_seq(c: &mut Criterion) {
    let v: Vec<i64> = (0..10000).collect();
    c.bench_function("seq_access", |b| {
        b.iter(|| {
            let mut sum = 0i64;
            for &x in &v { sum += black_box(x); }
            black_box(sum)
        })
    });
}
```

### What Production Does That Ours Doesn't

| Feature | Our Harness | Google Benchmark | Criterion |
|---------|------------|------------------|-----------|
| Auto-iteration count | No — fixed N | Yes | Yes |
| Bootstrapped CI | No | No (but does repeated runs) | Yes |
| Warmup | Manual | Automatic | Automatic |
| HTML regression reports | No | No | Yes |
| Outlier classification | No | No | Yes |

## Read the Source

- **Google Benchmark:** [github.com/google/benchmark](https://github.com/google/benchmark) — Look at `src/benchmark_register.cc` for the iteration auto-tuning logic, and `src/benchmark.cc` for how `DoNotOptimize` is implemented per-compiler.
- **Criterion.rs:** [github.com/bheisler/criterion.rs](https://github.com/bheisler/criterion.rs) — Look at `src/lib.rs` for the warmup/measure loop, and `src/stats/mod.rs` for bootstrapped confidence intervals.

## Ship It

The reusable artifact for this lesson lives in `outputs/benchmark_checklist.md` — a one-page reference card you can print and pin to your monitor. It covers:

- Common pitfalls and how to prevent each one
- When to use which statistic
- What production frameworks provide

## Exercises

1. **Easy** — Run the provided benchmark harness on your own machine. Compare your numbers to the lesson's. Explain why they differ.
2. **Medium** — Add a new benchmark: hash map lookup (key exists vs key misses). Measure the branch prediction cost of failed lookups. Report min/median/p99.
3. **Hard** — Implement bootstrapped 95% confidence intervals in the harness. How many resamples do you need before the CI stabilizes? Compare your CI width to the σ-based estimate.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Dead Code Elimination | "The compiler optimized my code" | The compiler proved the result was unobservable and removed the computation entirely |
| Warm cache | "Cached result" | Data resides in L1/L2/L3 — subsequent accesses are ~1-10 ns instead of ~100 ns from DRAM |
| Turbo boost | "CPU sped up" | CPU dynamically raises clock frequency under load, creating a warmup period of lower performance |
| Black box | "Don't optimize this" | A compiler barrier that prevents the optimizer from seeing through a value, preserving the computation |
| p99 | "99th percentile" | The latency below which 99% of observations fall — captures tail behavior that averages hide |
| Microbenchmark | "Tiny benchmark" | Measuring one isolated operation; fast to run but may not reflect real-world performance |
| Macrobenchmark | "End-to-end benchmark" | Measuring a full system under realistic load; slow to run but captures real costs |
| Confidence interval | "Margin of error" | A range that contains the true value with a specified probability (e.g., 95%) |

## Further Reading

- Chandler Carruth, "Benchmarking C++ Code — It's a Trap!" (CppCon 2015)
- Stefan Heule, "Statistically Rigorous Performance Evaluation" (paper on bootstrapped CI for benchmarks)
- Google Benchmark documentation: https://github.com/google/benchmark
- Criterion.rs book: https://bheisler.github.io/criterion.rs/book/
- Agner Fog, "Optimizing Software in C++" — Chapter on microarchitectural performance pitfalls