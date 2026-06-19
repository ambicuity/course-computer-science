# Benchmark Checklist — Reference Card

> Pin this to your monitor. Every time you write a benchmark, check every item.

## Common Pitfalls & Prevention

| Pitfall | What Happens | How to Prevent |
|---------|-------------|----------------|
| **Dead Code Elimination** | Compiler removes computation whose result is never used; you measure 0 ns | Write result to `volatile` sink; use `asm volatile("" ::: "memory")` barrier; use `std::hint::black_box()` in Rust |
| **Warm Cache** | First iteration pays DRAM latency (~100 ns); subsequent iterations hit L1 (~1 ns) | Run warmup iterations (discard from stats); report cold-access separately if relevant |
| **CPU Turbo Boost Ramp** | CPU runs at lower frequency during first 10-50 ms; early iterations are slower | Discard warmup iterations; pin CPU frequency with `cpufreq-set` for controlled tests |
| **Const Propagation** | Compiler precomputes results when inputs are known at compile time | Pass inputs through `volatile` read or `black_box` to prevent the compiler from seeing them |
| **Inlining** | Compiler inlines the benchmarked function and surrounding code into the loop, removing overhead you intended to measure | Use `__attribute__((noinline))` or `#[inline(never)]` when measuring function-call overhead specifically |
| **Branch Prediction Luck** | Sorted data produces predictable branches; random data causes mispredictions | Benchmark both sorted and random data layouts; report both |
| **OS Noise** | Scheduler preemptions, interrupts, background processes add latency spikes | Pin process to a core (`taskset`); raise priority (`nice -20`); discard outliers > 3× MAD from median |
| **Page Faults** | First memory access triggers demand paging; adds ~1-10 µs per page | Touch all pages before measuring (mmap + memset); or use `mlock()` to lock pages in RAM |

## When to Use Which Statistic

| Statistic | Use When | Caveat |
|-----------|----------|--------|
| **Min** | Best-case (cache-hot, no noise) — useful as a lower bound | Unrealistic for production; hides outliers |
| **Median** | Typical-case performance — resistant to outliers | Hides tail latency; always pair with p99 |
| **Mean** | Symmetric distributions with no outliers | Skewed by even a single extreme outlier |
| **p99** | Tail latency — what 1% of users experience | Cannot be used alone; needs median for context |
| **p95** | Moderate tail — less sensitive than p99 to extreme outliers | Still hides the worst 5% |
| **Max** | Worst-case latency — useful for SLA bounds | Often dominated by OS noise; may not be representative |
| **Stddev** | Measuring consistency / variance | Meaningless for bimodal distributions |
| **Mean/Median ratio** | Detecting skew — ratio > 1.5 indicates heavy tail | Doesn't tell you the shape of the tail |

## Minimum Reporting Standard

Always report at least this table:

```
Benchmark       min   median   mean   p99    max   stddev
────────────────────────────────────────────────────────
your_bench     X ns   Y ns    Z ns   W ns   V ns   S ns
```

A single number is never acceptable.

## What Production Frameworks Provide

| Feature | Google Benchmark (C++) | Criterion (Rust) |
|---------|----------------------|-------------------|
| Auto-iteration count | Yes (adapts N for stability) | Yes (iterates until CI converges) |
| Warmup phase | Configurable, auto | Configurable, auto |
| DCE prevention | `DoNotOptimize()` | `black_box()` |
| Bootstrapped CI | No (uses repeated runs) | Yes (10K resamples) |
| Outlier detection | No | Yes (Severe / Mild / OK) |
| Regression detection | Via `--benchmark_filter` + external tools | Built-in (compares to saved baseline) |
| HTML reports | No | Yes |
| Fixture setup/teardown | Yes (templated fixtures) | Yes (iter macro) |

## Confidence Interval Quick Formula

For 95% CI on the mean:

```
n ≥ (1.96 × σ / ε)²

Where:
  1.96 = z-value for 95% confidence
  σ    = estimated standard deviation
  ε    = desired margin of error (e.g., 0.01 × mean for 1% precision)
```

**Practical rule:** Start with 100 iterations. If CV = σ/μ < 0.05, you're done. If CV > 0.05, double iterations and re-measure.

## Outlier Removal Policy

1. **Never** remove more than 5% of samples
2. Use MAD (Median Absolute Deviation): discard points > 3× MAD from median
3. **Keep** outliers caused by the system itself (cache misses, TLB shootdowns — these are real costs)
4. **Discard** outliers caused by external noise (OS scheduler, unrelated process activity — these are not your program's fault)
5. Always report how many outliers were removed and why

## Quick Pre-Flight Checklist

Before publishing any benchmark number:

- [ ] DCE is prevented (volatile / asm barrier / black_box)
- [ ] Warmup iterations are discarded (≥ 10, or enough for > 50 ms)
- [ ] Inputs are opaque to the compiler (prevent const propagation)
- [ ] Statistics include: min, median, mean, p99, max (not just average)
- [ ] At least 100 measured iterations
- [ ] Both sorted and random data layouts tested (if relevant)
- [ ] Cold and warm access measured separately (if relevant)
- [ ] Outlier removal policy documented
- [ ] Hardware and compiler version documented