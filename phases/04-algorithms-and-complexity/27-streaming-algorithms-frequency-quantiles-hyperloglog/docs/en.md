# Streaming Algorithms — Frequency, Quantiles, HyperLogLog

> Process billion-element streams in kilobytes of memory — approximate counting without storing the data.

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 04 lessons 01–26
**Time:** ~75 minutes

## Learning Objectives

- Understand the streaming model: data arrives one element at a time, O(polylog n) space, can't store everything
- Implement reservoir sampling, Count-Min Sketch, and HyperLogLog from scratch
- Analyse the error bounds each algorithm provides and when they break down
- Apply streaming algorithms to real systems: query optimisers, network monitoring, analytics

## The Problem

A web server logs 10 million requests per hour. You need to answer: "Which URLs are the top 1%?" and "How many unique IPs visited?" You cannot keep all 10 million entries in memory. Classic algorithms assume you can see the whole dataset — but in a **stream** the data arrives one element at a time, possibly never-ending, and you have only O(polylog n) memory.

## The Concept

### The Streaming Model

A sequence `x_1, x_2, …, x_N` arrives one at a time. Memory must be O(polylog N). You get one pass. Every algorithm here trades **exactness** for **space**.

### Reservoir Sampling — Uniform k-Sample, O(k) Space

Given a stream of N items (N unknown), pick k uniformly at random — each item has probability k/N.

1. Fill the reservoir with the first k items.
2. For item `i > k`: generate random `j ∈ [0, i)`. If `j < k`, replace `reservoir[j]`.

**Proof sketch:** At step i, each existing item is replaced with probability `(k/i) × (1/k) = 1/i`, so it survives with probability `(1 − 1/i)`. Multiplying from step k+1 to i gives `k/i` — uniform over all i items seen so far.

### Count-Min Sketch — Frequency, O(1/ε · log(1/δ)) Space

A `d × w` counter matrix with d hash functions.

- **add(x):** increment `counters[i][h_i(x)]` for each row i.
- **estimate(x):** return `min_i counters[i][h_i(x)]`.

**Error guarantee:** After N insertions — `true_count(x) ≤ estimate(x) ≤ true_count(x) + εN` with probability ≥ 1 − δ, where `w = ⌈e/ε⌉`, `d = ⌈ln(1/δ)⌉`.

Taking the min across rows gives the tightest upper bound: any single row can be inflated by collisions, but all rows being inflated by > εN simultaneously has probability ≤ δ.

### HyperLogLog — Cardinality, O(m) Space

Count distinct elements using m registers.

1. Hash each element → bucket index j from first ⌈log₂(m)⌉ bits.
2. Remaining bits: compute `ρ` = leading zeros + 1.
3. Update: `register[j] = max(register[j], ρ)`.
4. Estimate: `ĉ = α_m · m² / Σ 2^(−register[j])` (harmonic mean).

**Relative error:** `~1.04/√m`. With m = 16384 registers (16 KB): error ≈ 0.8%.

The harmonic mean (used by HyperLogLog) is more robust than the arithmetic mean (LogLog) because it down-weights outlier registers with inflated max-leading-zero counts.

## Build It

### Step 1: Reservoir Sampling

```python
def reservoir_sample(stream, k):
    reservoir = []
    for i, item in enumerate(stream):
        if i < k:
            reservoir.append(item)
        else:
            j = random.randint(0, i)
            if j < k:
                reservoir[j] = item
    return reservoir
```

O(k) space, single pass, exactly uniform.

### Step 2: Count-Min Sketch

```python
class CountMinSketch:
    def __init__(self, epsilon=0.001, delta=0.01):
        self.w = int(math.ceil(math.e / epsilon))
        self.d = int(math.ceil(math.log(1 / delta)))
        self.counters = [[0] * self.w for _ in range(self.d)]
        self.seeds = [random.randint(0, 2**31 - 1) for _ in range(self.d)]

    def add(self, element):
        for i in range(self.d):
            self.counters[i][self._hash(element, self.seeds[i])] += 1

    def estimate(self, element):
        return min(self.counters[i][self._hash(element, self.seeds[i])]
                   for i in range(self.d))
```

### Step 3: HyperLogLog

```python
class HyperLogLog:
    def __init__(self, p=14):
        self.p = p
        self.m = 1 << p
        self.registers = [0] * self.m
        self.alpha = 0.7213 / (1 + 1.079 / self.m)

    def add(self, element):
        x = hash(element)  # simplified
        j = x & (self.m - 1)
        w = x >> self.p
        self.registers[j] = max(self.registers[j], self._rho(w))

    def count(self):
        Z = sum(2.0 ** (-r) for r in self.registers)
        return int(self.alpha * self.m * self.m / Z)
```

## Use It

Streaming algorithms appear throughout production systems:

- **Database query optimisers** — PostgreSQL uses HyperLogLog variants for column cardinality to decide join order without full table scans.
- **Network monitoring** — Apache Druid and ClickHouse embed Count-Min Sketch for real-time heavy-hitter detection on event streams.
- **Analytics** — Google Analytics uses HyperLogLog for "unique visitors" reports. Google's 2007 paper showed estimating 10¹⁰ distinct IPs with 1.5 KB of memory.

## Read the Source

- `Redis/src/hyperloglog.c` — production HyperLogLog with bias correction tables and 6-bit registers.
- `Apache DataSketches` (Java) — Yahoo's library with Count-Min, HLL, quantiles, and more.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained streaming algorithm toolkit** — reservoir sampling, Count-Min Sketch, and HyperLogLog you can drop into any project.

## Exercises

1. **Easy** — Prove reservoir sampling is uniform: show by induction that after processing i items, each item has probability k/i of being in the reservoir.

2. **Medium** — Implement Misra-Gries heavy hitters: given a stream and parameter k, find all elements with frequency > N/k using only O(k) counters. Compare against Count-Min Sketch heavy hitter detection.

3. **Hard** — Collect a real-world URL stream. Compare HyperLogLog error (m = 256, 1024, 4096, 16384) against exact count. Plot error% vs memory. At what m does error drop below 1%?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Streaming model | "Data arrives as a flow" | Sequence processed in one pass with sub-linear memory |
| Reservoir sampling | "Random sample from a stream" | O(k)-space algorithm producing exactly uniform k-sample |
| Count-Min Sketch | "Approximate frequency counter" | d×w counter matrix; ε-δ error guarantee |
| HyperLogLog | "Count distinct elements" | m-register sketch using leading-zeros-per-bucket, harmonic mean |
| Cardinality | "Number of distinct items" | |{x : x appeared in stream}| |
| Heavy hitters | "Frequent items" | Elements with frequency > threshold·N |

## Further Reading

- Cormode & Muthukrishnan, "An Improved Data Stream Summary: The Count-Min Sketch" (2005)
- Flajolet et al., "HyperLogLog: the analysis of a near-optimal cardinality estimation algorithm" (2007)
- Muthukrishnan, "Data Streams: Algorithms and Applications" (2005)
