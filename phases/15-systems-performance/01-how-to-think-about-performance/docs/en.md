# How to Think About Performance

> Measure first, optimize second, and never trust your intuition about where time goes.

**Type:** Learn
**Languages:** Markdown
**Prerequisites:** Phase 13 (Concurrency vs Parallelism), Phase 14 (Graphics & Visualization)
**Time:** ~45 minutes

## Learning Objectives

- Apply Amdahl's Law and Gustafson's Law to predict speedup limits for a given workload, and identify which law applies to a given scenario.
- Distinguish latency from throughput, explain why optimizing one can degrade the other, and choose the right metric for a given problem.
- Recite the memory hierarchy cost ladder (L1 < L2 < L3 < DRAM < SSD < network) with approximate orders of magnitude, and predict which level dominates a given access pattern.
- Decompose execution time using the performance equation (Time = Instructions × CPI × Clock_period) and identify which factor to optimize for a given bottleneck.
- Read a roofline model plot, classify a workload as compute-bound or memory-bound, and predict which optimization will have the most impact.

## The Problem

Most programmers think about performance the way they think about a messy garage: they open the door, see clutter everywhere, and start reorganizing the first thing they notice. They optimize the function that *looks* slow, add a cache because caches are *supposed* to be fast, and multithread a loop because more cores *should* mean more speed. Then they measure and discover the program got slower.

This is not a joke. It is the most common failure mode in performance engineering. Intuition about where time goes is wrong — catastrophically, systematically wrong. The function you think is hot might account for 2% of runtime. The cache you added might increase miss rate because it evicts data the CPU was about to use. The parallelized loop might spend more time synchronizing than computing.

Phase 15 is about measuring honestly, tuning cache and branches and I/O, and winning 10x by knowing the machine. This lesson is the foundation: how to *think* about performance before you touch a single line of code. Without it, every subsequent lesson — profiling, cache optimization, branch prediction, I/O tuning — will be built on guesswork. You will optimize the wrong thing, measure the wrong metric, and wonder why performance got worse instead of better.

Consider a concrete scenario: your web service handles 10,000 requests per second, and p99 latency just jumped from 200 ms to 2 seconds. Where do you look? If you guess "the database query," you might spend two weeks adding indexes only to discover that GC pauses account for 90% of the latency spike. If you guess "the JSON parser," you might rewrite it in C only to find that network round-trips dominate. The difference between a senior performance engineer and everyone else is that the senior engineer measures first, then decides.

## The Concept

### The Performance Mindset

Performance engineering has three rules:

1. **Measure before you optimize.** If you haven't measured, you don't know what's slow.
2. **Optimize the bottleneck.** The bottleneck is the component that determines overall speed. Speeding up anything else is wasted effort.
3. **Measure after you optimize.** If you didn't measure again, you don't know if you helped.

These rules sound obvious. They are not. Every year, thousands of engineers violate rule 1 by "prematurely optimizing" code that isn't hot. They violate rule 2 by optimizing the function they wrote most recently instead of the function the CPU spends the most time in. They violate rule 3 by declaring victory without re-measuring.

Donald Knuth famously said "premature optimization is the root of all evil." The full quote is more nuanced: "We should forget about small efficiencies, say about 97% of the time: premature optimization is the root of all evil." The other 3% of the time — when you're in a hot loop that accounts for 90% of runtime — optimization is not premature. It is the entire job. The mistake is *when* you optimize, not *whether*. Optimize after profiling, not before.

### Latency vs Throughput

Two fundamentally different performance goals:

| Metric | Question | Unit | Analogy |
|--------|----------|------|---------|
| **Latency** | How long does one operation take? | Time (ms, μs, ns) | How fast can one car reach the destination? |
| **Throughput** | How many operations per unit time? | Ops/sec, MB/s | How many cars per hour can the road carry? |

A system can have low latency and low throughput (a single-person sports car). A system can have high throughput and high latency (a bus that moves 50 people but takes twice as long). Optimizing for one often degrades the other.

```
Low latency, low throughput:     ████░░░░░░  (one fast request at a time)
High latency, high throughput:   ██████████░  (batch 100 requests together)

Pipeline: overlap latency for throughput:
  Request 1: ████░░░░░░
  Request 2: ░░████░░░░
  Request 3: ░░░░████░░
  Throughput: 1 completion per pipeline stage time
```

**Latency matters when:** a user is waiting (interactive systems, API calls, UI rendering).
**Throughput matters when:** you're processing a batch (ETL, log analysis, ML training).

**The critical trap:** batching improves throughput but increases per-request latency. Pipelining improves throughput without increasing per-request latency — but only if the pipeline stays full.

### The Memory Hierarchy — Orders of Magnitude

The single most important fact in systems performance is that memory access is not uniform. Data locality — keeping hot data in fast storage — dominates everything else.

```
┌─────────────────────────────────────────────────────────────┐
│ Level          Latency        Capacity    $/GB    Bandwidth  │
├─────────────────────────────────────────────────────────────┤
│ Register       ~0 ns          ~1 KB       -       -         │
│ L1 Cache       ~0.5 ns (3cy)  ~64 KB      -       ~1 TB/s   │
│ L2 Cache       ~7 ns (20cy)   ~1 MB       -       ~500 GB/s │
│ L3 Cache       ~15 ns (45cy)  ~32 MB      -       ~200 GB/s │
│ DRAM           ~100 ns (300cy)~64 GB      $3/GB   ~50 GB/s  │
│ SSD (NVMe)     ~100 μs        ~4 TB       $0.1/GB ~7 GB/s   │
│ SSD (SATA)     ~500 μs        ~4 TB       $0.1/GB ~0.5 GB/s │
│ HDD            ~10 ms         ~20 TB      $0.02/GB~0.2 GB/s  │
│ Network (DC)   ~500 μs        -           -       ~100 Gb/s  │
│ Network (WAN)  ~50 ms         -           -       ~10 Gb/s   │
└─────────────────────────────────────────────────────────────┘

Key ratios to memorize:
  L1 : L2 : L3 : DRAM : SSD : Network
   1 :  14 :  30 : 200 : 200000 : 100000000

A DRAM access costs ~200x an L1 access.
An SSD read costs ~200,000x an L1 access.
A network round-trip costs ~100,000,000x an L1 access.
```

If your program touches data that falls out of L1, it's already 14x slower per access than it could be. If it has to go to DRAM, it's 200x slower. If it has to go to SSD, it's 200,000x slower. The entire art of cache optimization is about keeping hot data in L1/L2 and minimizing the number of DRAM trips.

**Why this matters:** An algorithm that does 10x more *computations* but 5x fewer *memory accesses* can be faster on large inputs because memory, not compute, is the bottleneck on modern hardware.

### The Performance Equation

Every program's execution time can be decomposed as:

```
Time = Instructions × CPI × Clock_period

Where:
  Instructions  = total dynamic instruction count (how many instructions executed)
  CPI           = Cycles Per Instruction (average cycles each instruction takes)
  Clock_period  = seconds per cycle (1 / frequency)

Equivalently:
  Time = Instructions × CPI / Frequency
```

This equation tells you exactly which knob to turn:

| Reduce... | By... | Example |
|-----------|-------|---------|
| **Instructions** | Better algorithm (reduce work) | O(n²) → O(n log n) cuts instruction count |
| **CPI** | Better memory access, branch prediction | L1 hits instead of DRAM reduces CPI from ~5 to ~1 |
| **Clock_period** | Faster CPU (higher frequency) | 3 GHz → 4.5 GHz chip — costs power and money |

The equation reveals a subtle point: **Big-O analysis is not performance analysis.** Big-O tells you how *Instructions* scale. It says nothing about *CPI* or *Clock_period*. An O(n log n) sort with random memory accesses (high CPI) can be slower than an O(n²) sort with sequential accesses (low CPI) for small n, because sequential access has CPI ≈ 1 while random access has CPI ≈ 5–10.

### Amdahl's Law — Speedup of the Whole

We covered Amdahl's Law in Phase 13, but now we revisit it with a performance engineering lens.

```
S(N) = 1 / ((1 - P) + P/N)

P = fraction that can be sped up
N = speedup factor of the improved portion
```

The law generalizes beyond parallelism. If you speed up a portion of the program by factor N, the overall speedup is limited by the fraction P. Two consequences:

1. **Make the common case fast.** If P is small (you're optimizing code that accounts for 5% of runtime), even infinite speedup gives only 1/(1-0.05) ≈ 1.05x overall. Not worth the engineering time.
2. **Serial bottlenecks are ruthless.** Even if you speed up 95% of the work infinitely, the 5% remaining limits you to 20x total speedup.

**Worked example:** You profile a program and find:
- 60% of time in function A (memory-bound, can be optimized with better cache usage)
- 25% of time in function B (can be parallelized across 8 cores)
- 15% of time in function C (serial, cannot be changed)

What's the maximum speedup if you optimize A by 3x and parallelize B across 8 cores?

```
After optimizing A (3x speedup on 60%):
  New time for A = 0.60 / 3 = 0.20
  New total = 0.20 + 0.25 + 0.15 = 0.60
  Speedup so far = 1 / 0.60 ≈ 1.67x

After parallelizing B (8 cores on the remaining 25%/0.60 = 41.7%):
  New time for B = 0.25 / 8 = 0.03125
  New total = 0.20 + 0.03125 + 0.15 = 0.38125
  Overall speedup = 1 / 0.38125 ≈ 2.62x
```

Starting with the biggest win (60% → 3x) gives most of the benefit. The 8-core parallelization of 25% only adds 0.4x more speedup because the remaining serial fraction now dominates.

### Gustafson's Law — Scaling with Resources

Amdahl assumes a fixed-size problem. Gustafson argues that in practice, you use more resources to solve bigger problems:

```
S(N) = N - (N-1) × s

Where:
  N = number of processors
  s = serial fraction of the scaled workload
```

This is the right model for:
- ML training: more GPUs → larger model or bigger dataset
- Scientific simulation: more cores → finer grid resolution
- Data analytics: more machines → larger dataset processed in same time

**Which law applies?**

| Question | Amdahl | Gustafson |
|----------|--------|-----------|
| "How fast can I process this 1 TB log file?" | Yes — fixed problem size | No |
| "How much more data can I process per hour with 2x machines?" | No | Yes |
| "What's the p99 latency of this API endpoint?" | Yes — fixed work | No |
| "Can I train a bigger model with 8 GPUs vs 1?" | No | Yes |

### The Roofline Model

The roofline model (Williams, Waterman, Patterson 2009) answers a simple question: **is my code compute-bound or memory-bound?**

```
Achievable GFLOP/s
│
│         ╱‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾  Compute ceiling
│       ╱    (peak compute)
│     ╱
│   ╱  ╱‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾  Ridge point
│ ╱  ╱
│╱  ╱    ← slope = Peak bandwidth / Peak compute
│╱
└─────────────────────────────── Operational intensity (FLOPs/byte)

Left of ridge point:  MEMORY-BOUND (slope region)
Right of ridge point: COMPUTE-BOUND (flat region)
Ridge point:          Where you saturate both bandwidth and compute
```

**Operational intensity** = FLOPs per byte of data transferred from memory. It tells you how much computation you do per memory access.

- Low operational intensity (e.g., vector add: 1 FLOP per 16 bytes): memory-bound
- High operational intensity (e.g., matrix multiply inner loop: ~2 FLOPs per 8 bytes): compute-bound at large sizes

The model is powerful because it tells you *which knob to turn*:

```
If you're memory-bound:
  → Reduce memory traffic (cache blocking, data layout, compression)
  → Increase bandwidth (NUMA-aware allocation, prefetching)
  → Do NOT waste time on instruction-level optimizations

If you're compute-bound:
  → Reduce instruction count (SIMD, loop unrolling, algorithmic change)
  → Increase compute throughput (higher frequency, more execution units)
  → Do NOT waste time on cache optimization (it's not the bottleneck)
```

### Big-O vs Constant Factors

Here is a truth that stings every CS student:

> An O(n²) algorithm with a good constant factor can beat an O(n log n) algorithm for small n.

Concrete example: quicksort (expected O(n log n)) vs insertion sort (O(n²)). In practice, most quicksort implementations switch to insertion sort for n < 10–20 because insertion sort has a better constant factor on tiny arrays: no recursion overhead, no partition computation, better cache locality (sequential access pattern).

```
Actual runtime comparison for small n:

n=10:   insertion sort: ~100 comparisons, sequential access → fast
        quicksort: ~45 comparisons, random access + recursion → slower

n=1000: insertion sort: ~500,000 comparisons → slow
        quicksort:  ~10,000 comparisons → fast
```

The lesson: **Big-O tells you what happens as n → ∞. Performance engineering tells you what happens at your n.** Both matter. Use Big-O to pick the right algorithm family, then use constant-factor optimization (cache locality, branch prediction, vectorization) to make it fast at your actual input size.

### When to Optimize: Profiling-Driven, Not Intuition-Driven

The performance engineering workflow:

```
1. Write correct code first
2. Define your performance target (latency budget, throughput goal)
3. Measure against the target
4. If you meet it: stop. Ship it.
5. If you don't: profile. Find the top N hotspots.
6. Optimize the #1 hotspot. One change at a time.
7. Measure again. Did it help? If yes: commit. If no: revert.
8. Repeat 5-7 until you meet the target.
```

**One change at a time** is critical. If you make three optimizations simultaneously and performance improves 30%, you don't know which one helped. Worse, one of them might have *hurt* performance but was masked by the other two. When that masking optimization is later removed (or the data changes), the hidden regression appears.

**Profiling tools you should know:**

| Tool | What it shows | When to use |
|------|---------------|-------------|
| `perf` (Linux) | Hardware counters, cache misses, branch mispredictions | CPU-bound workloads |
| `flamegraph` | Stack trace visualization (which functions are hot) | Finding hotspots quickly |
| `strace` | System calls and their durations | I/O-bound workloads |
| `valgrind --tool=cachegrind` | Cache miss rates per line | Cache optimization |
| `time` / `/usr/bin/time` | Wall clock, user, system time | Quick first pass |

### Practical Wisdom

1. **Benchmark everything.** Use a benchmarking harness (Google Benchmark, Criterion, JMH). Run multiple iterations. Report median and p99, not just mean. Mean is distorted by outliers; p99 tells you what real users experience.

2. **Isolate variables.** Change one thing at a time. Run in a controlled environment (no background processes, CPU governor set to performance, turbo boost disabled or documented).

3. **Watch out for noise.** Modern CPUs are noisy: turbo boost varies frequency, background processes steal cache lines, DVFS throttles under load. Report confidence intervals, not single numbers.

4. **The 90/10 rule is real.** 90% of time is spent in 10% of the code. Finding that 10% is what profiling is for. Don't optimize the other 90%.

5. **Latency percentiles, not averages.** If 99% of requests complete in 10 ms but 1% take 10 seconds, the average is 110 ms — useless. The p50 (median) is 10 ms, and p99 is 10 seconds. Both numbers tell you something important.

6. **Batch amortizes overhead.** One syscall per 4 KB costs 200 ns of overhead per KB. One syscall per 1 MB costs 0.2 ns of overhead per KB. The syscall cost is amortized 1000x.

7. **Prefetch hides latency.** If you know what data you'll need next, ask for it before you need it. The CPU does hardware prefetching for sequential patterns. You can do software prefetching for irregular ones.

## Build It

This is a Learn lesson — the artifact is reference notes in `code/notes.md`. Open that file alongside this one. Here we walk through how to apply the conceptual framework to a real scenario.

### Step 1: Profile Before You Touch Anything

You have a Python service that processes JSON logs. It takes 10 seconds per million records. Your target is 2 seconds.

```
$ python -m cProfile process_logs.py

   ncalls  tottime  percall  cumtime  percall filename:lineno(function)
   1000000  4.200    0.000    4.200    0.000 json.py:314(loads)
   1000000  3.100    0.000    3.100    0.000 filter.py:45(match)
   1000000  1.500    0.000    1.500    0.000 output.py:22(write)
   1000000  0.200    0.000    0.200    0.000 main.py:10(enrich)
```

The profile says:
- JSON parsing (42%) — likely memory-bound, parsing text
- Pattern matching (31%) — possibly compute-bound
- Output writing (15%) — I/O-bound
- Enrichment (2%) — negligible

**The 2% enrichment function is not worth optimizing.** Don't even look at it.

### Step 2: Apply Amdahl's Law to Set Expectations

Maximum speedup from optimizing JSON parsing (42% of time):
```
S = 1 / (1 - 0.42 + 0.42/N)

If we can speed up JSON parsing by 10x (N=10):
  S = 1 / (0.58 + 0.042) = 1 / 0.622 ≈ 1.61x
  New time ≈ 10 / 1.61 ≈ 6.21 seconds

That alone won't reach our 2-second target.
```

Maximum speedup from optimizing JSON and pattern matching (73% of time):
```
If we can speed up both by 10x:
  New time = (4.2 + 3.1) / 10 + 1.5 + 0.2 = 0.73 + 1.7 = 2.43 seconds

Close! But we need a bit more from the I/O path too.
```

This exercise tells you: **optimizing only JSON parsing won't get you there.** You need to tackle at least the top two hotspots, and maybe the I/O.

### Step 3: Determine Your Bottleneck Type

For JSON parsing — is it compute-bound or memory-bound?

```
Operational intensity of JSON parsing:
  - Each character requires ~1-5 operations (branch, compare, copy)
  - Each character is 1 byte read from memory
  - OI ≈ 2 FLOP/byte → near the ridge point, slightly memory-bound

Answer: memory-bound. The data is walking through text sequentially.
Optimization strategy: reduce memory traffic (use simdjson for vectorized
parsing) or reduce total data (compress input).
```

For pattern matching — is it compute-bound or memory-bound?

```
Operational intensity of regex matching:
  - Each character requires multiple comparisons (NFA simulation)
  - Data is accessed sequentially
  - OI ≈ 5-10 FLOP/byte → compute-bound

Answer: compute-bound. The regex engine burns cycles.
Optimization strategy: simplify regex, use an NFA→DFA conversion, or
switch to a faster regex engine (re2, hyperscan).
```

### Step 4: Optimize — One Change at a Time

```
Iteration 1: Replace json.loads with orjson (C-based JSON parser)
  Result: 4.20s → 0.84s (5x speedup on JSON portion)
  Total: 10.0s → 6.64s. Verified with cProfile.

Iteration 2: Replace re.compile().match with aho-corasick for fixed patterns
  Result: 3.10s → 0.93s (3.3x speedup on matching portion)
  Total: 6.64s → 4.47s. Verified.

Iteration 3: Batch output writes (accumulate 1000 records, then write)
  Result: 1.50s → 0.30s (5x speedup on I/O via batching)
  Total: 4.47s → 3.27s. Not enough.

Iteration 4: Combine orjson + aho-corasick + write batching + process in parallel across 4 cores
  (Now Amdahl applies: 0.30+0.93+0.84 = 2.07s parallelizable, 0.93+0.30 = 1.23s serial-ish)
  With 4 cores on 2.07s: 2.07/4 ≈ 0.52s parallel portion
  Total ≈ 0.52 + 1.23 = 1.75s. Target hit!
```

Each change was measured in isolation. Each was either committed or reverted based on measurement. The final result: 10s → 1.75s — a 5.7x speedup achieved by optimizing the right things in the right order.

## Use It

Production systems use these principles daily:

**Google's Flume paper (2010)** — Google's data processing framework uses the roofline model to decide whether MapReduce workers should optimize for compute or I/O. Workers with high operational intensity get more CPU; workers with low operational intensity get I/O batching. The framework profiles each stage and adjusts automatically.

**Facebook's memcache** — Memcache at Facebook serves billions of requests per day. The team found that p99 latency was dominated by a small fraction of "tail" requests. Their solution was not to optimize the average path (which was already fast) but to add request hedging: send the same request to two servers and use whichever responds first. This reduced p99 latency by 50% with only 5% more total load — an Amdahl's Law application: they targeted the 1% of requests that accounted for the majority of bad latency.

**Linux kernel's `perf` subsystem** — The `perf` tool in Linux is the primary profiling interface for performance engineers at every major tech company. It reads hardware performance counters (cache misses, branch mispredictions, instructions retired, cycles) and maps them to source lines. The kernel exposes these counters through `sysfs` and `perf_event_open()`. When Linus says "talk is cheap, show me the code," the performance equivalent is "guessing is cheap, show me the counters."

**SIMD JSON parsing (simdjson)** — The simdjson library parses JSON at 2.5+ GB/s on a single core, vs ~100-300 MB/s for conventional parsers. It does this by using AVX-512/SVE instructions to process 64 bytes at a time. This is a roofline model application: JSON parsing has low operational intensity (memory-bound), so the optimization is to increase bandwidth utilization via SIMD, not to reduce instruction count.

## Read the Source

- [Linux `perf` source — `tools/perf/builtin-record.c`](https://github.com/torvalds/linux/blob/master/tools/perf/builtin-record.c) — The entry point for `perf record`. Look at how it sets up hardware performance counters via `perf_event_open()` and maps them to symbols.
- [simdjson — `src/generic/stage1/json_structural_indexer.h`](https://github.com/simdjson/simdjson/blob/master/src/generic/stage1/json_structural_indexer.h) — The core of the SIMD JSON parser. Look at how it processes 64 bytes at a time using AVX-512 instructions. This is roofline optimization in action: maximizing memory bandwidth utilization.
- [Google Benchmark — `src/benchmark.cc`](https://github.com/google/benchmark/blob/master/src/benchmark.cc) — The benchmarking framework used at Google. Look at how it handles warmup iterations, iteration counting, and statistical reporting (median, mean, p99).

## Ship It

The reusable artifact from this lesson lives in `outputs/`. It is:

- **`performance_checklist.md`** — A one-page checklist for performance investigations. Print it, pin it to your monitor, and follow it every time you need to optimize something. It covers the workflow: measure → profile → identify bottleneck type → apply the right optimization → verify.

## Exercises

1. **Easy** — You have a program that takes 60 seconds. Profiling shows 40 seconds in function X and 20 seconds in function Y. Using Amdahl's Law, what is the maximum possible speedup if you optimize function X infinitely? What if you optimize function Y by 4x instead?

2. **Medium** — A sorting algorithm does O(n²) comparisons but has sequential memory access (CPI ≈ 1). An alternative does O(n log n) comparisons but with random access (CPI ≈ 5). At what value of n does the O(n log n) algorithm become faster? Assume each comparison takes 1 cycle. Show your work using the performance equation.

3. **Hard** — You are building a real-time image processing pipeline that processes 4K frames at 30 FPS. Each frame requires: (a) decompression from disk (100 ms, I/O-bound), (b) filter kernel (50 ms, compute-bound at OI = 8 FLOP/byte), (c) encoding to network (10 ms, I/O-bound). Identify the bottleneck using Amdahl's Law. Design a pipelined architecture that overlaps the I/O-bound and compute-bound stages. Calculate the maximum throughput in frames per second, and explain what happens to per-frame latency when the pipeline is full.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Latency | "How fast it is" | Time for one operation to complete — from request to response |
| Throughput | "How much it can handle" | Operations completed per unit time — requests/sec or MB/sec |
| CPI | "Instructions per cycle" (wrong direction) | Cycles Per Instruction — average clock cycles each instruction takes; lower is better |
| Operational intensity | "How compute-heavy it is" | FLOPs per byte of memory traffic; determines whether you're compute-bound or memory-bound |
| Amdahl's Law | "More cores = more speed" (misuse) | Maximum speedup is limited by the serial fraction: S ≤ 1/(1-P). Adding cores helps less than you think. |
| Gustafson's Law | "Amdahl is too pessimistic" | Speedup grows nearly linearly if problem size scales with resources: S = N - (N-1)(1-P) |
| Roofline model | "Is it CPU or memory bound?" | A model that plots achievable performance vs operational intensity; tells you whether to optimize compute or memory |
| Premature optimization | "Never optimize" (misquote) | Optimizing before profiling — the root of evil is not optimization itself, but untimed optimization |
| Big-O | "The faster algorithm" | Asymptotic growth rate — says nothing about constant factors, cache behavior, or real-world performance at your actual n |
| Profiling | "Running tests" | Measuring where time is actually spent using hardware counters, sampling, or instrumentation; not the same as testing |

## Further Reading

- Gene Amdahl, "Validity of the Single Processor Approach to Achieving Large Scale Computing Capabilities" (1967) — The original three-page paper that defined Amdahl's Law. Read it to see the argument in its pure form.
- John L. Gustafson, "Reevaluating Amdahl's Law" (1988) — The counterargument: problem size scales with resources. Essential for understanding when Amdahl is pessimistic.
- Samuel Williams, Andrew Waterman, David Patterson, "Roofline: An Insightful Visual Performance Model for Multicore Architectures" (2009) — The original roofline model paper. Figures 2 and 3 are worth understanding in detail.
- Brendan Gregg, "Systems Performance: Enterprise and the Cloud" (2nd ed.) — The definitive reference for Linux performance analysis. Chapters 2-3 cover methodology, Chapters 4-6 cover CPU and memory profiling.
- Emmanuel Goossaert, "Performance Matters" (2013) — A practical blog post series on performance engineering mindset. Covers the "measure first" philosophy with concrete examples.
- Donald Knuth, "Structured Programming with go to Statements" (1974) — The original source of "premature optimization is the root of all evil." Read the full context — Knuth advocates measuring first, not avoiding optimization entirely.