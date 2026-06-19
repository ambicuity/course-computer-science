# Phase Capstone — Algorithm Cookbook + Benchmark Harness

> You've learned the tools. Now learn which tool to reach for — and prove it with numbers.

**Type:** Build
**Languages:** Rust, Python
**Prerequisites:** Phase 04 lessons 01–29
**Time:** ~120 minutes

## Learning Objectives

- Build an algorithm selection decision tree covering sorting, searching, graphs, DP, greedy, and strings.
- Construct a benchmark harness measuring wall time across input distributions.
- Produce CSV + comparison table reports from benchmark data.
- Ship a Rust CLI tool (`outputs/algorithm_cookbook/`) you can `cargo run` to benchmark algorithms.

## The Problem

You know 20+ algorithms. You know their Big-O. But when a problem lands on your desk, which algorithm do you pick? The gap between "I know this algorithm" and "I know when to use it" is where real engineering lives. This lesson closes that gap with two artifacts: a **cookbook** (the decision tree) and a **benchmark harness** (the proof).

## The Concept

### The Algorithm Cookbook

A cookbook maps problem types to algorithms with decision paths, not just lists.

#### Sorting Decision Tree

```
Need to sort?
├── Bounded range (small int domain)? → Counting Sort O(n+k) or Radix Sort O(n·d)
├── Nearly sorted / small n (<50)?    → Insertion Sort — O(n) on nearly-sorted
├── Need stability?                   → Merge Sort O(n log n) or Timsort
├── Memory constrained?               → Quicksort O(log n) space, Heapsort O(1) space
└── General case                      → Quicksort (median-of-3) or pdqsort
```

#### Searching: Binary Search Variants

```
Sorted data?
├── Exact match?     → Standard binary search O(log n)
├── First/last occ?  → Lower/upper bound
├── Unbounded stream?→ Exponential search → binary search
└── Uniform dist?    → Interpolation search O(log log n) average
```

#### Graph Algorithm Selection

```
Graph problem?
├── Shortest path unweighted?         → BFS O(V+E)
├── Weighted, non-negative?           → Dijkstra O(E log V)
├── Negative weights?                 → Bellman-Ford O(VE)
├── All-pairs shortest path?          → Floyd-Warshall O(V³)
├── Cycle detection / topo sort?      → DFS
├── Minimum spanning tree?            → Kruskal (sparse) / Prim (dense)
├── Maximum flow?                     → Dinic O(V²E)
└── Bipartite matching?               → Hopcroft-Karp O(E√V)
```

#### DP vs Greedy Decision Framework

```
Optimization problem?
├── Greedy choice property holds (local → global optimum)?
│   └── YES → Greedy. Prove with exchange argument or matroid structure.
│       Examples: Huffman, Kruskal, Dijkstra, activity selection
└── Choices interact / overlapping subproblems?
    └── YES → DP. Examples: knapsack, edit distance, LCS, matrix chain
```

#### Strings: Pattern Matching Selection

```
String matching?
├── Single pattern?   → KMP O(n+m) or Boyer-Moore (sublinear practice)
├── Multiple patterns?→ Aho-Corasick O(n+m+z)
├── Fuzzy matching?   → Edit distance DP O(nm)
└── Rolling hash?     → Rabin-Karp O(n+m) average
```

### The Benchmark Harness

A standardized framework measuring algorithm performance under controlled conditions.

**Metrics:** wall time (actual elapsed), comparisons (hardware-independent), swaps (data movement), memory allocations.

**Input generators** (benchmarks are only as good as their inputs):
- `random(n)` — uniform random integers
- `sorted(n)` — already sorted (best/worst depending on algorithm)
- `reversed(n)` — reverse sorted
- `adversarial(n)` — worst-case construction (e.g., median-of-3 killer)
- `nearly_sorted(n, k)` — k random swaps from sorted (real-world workload)

**Output:** CSV (`algorithm,input_type,n,time_ms`) + terminal comparison tables.

### Why Two Languages?

**Python** — fast prototyping, readable cookbook logic. The harness measures the *algorithm*, not the language. **Rust** — zero-cost abstractions, `std::time::Instant` for precise timing, no GC pauses. Use for the shipped benchmark harness.

## Build It
### Step 1: BenchmarkHarness Class

```python
import time, csv

class BenchmarkHarness:
    def __init__(self):
        self.results = []
        self._generators = {}
        self._algorithms = {}

    def register_algorithm(self, name, fn):
        self._algorithms[name] = fn

    def register_generator(self, name, fn):
        self._generators[name] = fn

    def run(self, n_values, repeats=5):
        for n in n_values:
            for gname, gen in self._generators.items():
                base = gen(n)
                for aname, alg in self._algorithms.items():
                    times = []
                    for _ in range(repeats):
                        arr = list(base)
                        t0 = time.perf_counter()
                        alg(arr)
                        times.append((time.perf_counter() - t0) * 1000)
                    self.results.append({
                        "algorithm": aname, "input": gname,
                        "n": n, "time_ms": round(sum(times)/len(times), 3)
                    })

    def to_csv(self, path):
        with open(path, "w", newline="") as f:
            w = csv.DictWriter(f, fieldnames=["algorithm","input","n","time_ms"])
            w.writeheader(); w.writerows(self.results)

    def print_table(self):
        print(f"{'Algorithm':<16} {'Input':<14} {'N':>8} {'Time (ms)':>10}")
        print("-" * 52)
        for r in self.results:
            print(f"{r['algorithm']:<16} {r['input']:<14} {r['n']:>8} {r['time_ms']:>10.3f}")
```

### Step 2: Register + Run

The full implementations live in `code/main.py`. Registration is straightforward:

```python
h = BenchmarkHarness()
h.register_algorithm("insertion", insertion_sort)
h.register_algorithm("merge", merge_sort); h.register_algorithm("quick", quick_sort)
h.register_algorithm("heap", heap_sort)
h.register_generator("random", lambda n: [random.randint(0,n) for _ in range(n)])
h.register_generator("sorted", lambda n: list(range(n)))
h.run(n_values=[1000, 5000, 10000], repeats=3)
h.print_table(); h.to_csv("outputs/benchmark_results.csv")
```

### Step 3: Rust Harness

`outputs/algorithm_cookbook/` uses `std::time::Instant` with inline sort/search/graph implementations. Subcommands: `cargo run -- sort|search|graph|report`.

## Use It

Production teams don't benchmark with `time.time()`:

- **Google Benchmark** (C++): statistical analysis, custom counters, CSV/JSON. The gold standard.
- **Criterion** (Rust): warm-up, outlier detection, regression reports with confidence intervals.
- **Python**: `timeit` (`Lib/timeit.py`), `pyperf` for statistically rigorous results.

Key insight: **always report distributions, not single runs.** Report median, p95, and std dev across 30+ runs. Production harnesses add warm-up phases, statistical outlier removal, memory profiling, and cross-platform calibration.

## Ship It
`outputs/algorithm_cookbook/` — a Rust crate implementing the benchmark harness as a CLI tool. `cargo run -- report` prints a combined comparison table + algorithm cookbook quick reference.

## Exercises

1. **Easy** — Benchmark all 5 sorts (insertion, selection, merge, quick, heap) on n=1000, n=10000, n=100000 with random, sorted, and reversed inputs. Produce a comparison report.
2. **Medium** — Benchmark Dijkstra vs Bellman-Ford on sparse (E=2V) vs dense (E=V²/2) graphs. For which density does Bellman-Ford become competitive? Why?
3. **Hard** — Build an interactive CLI that asks problem characteristics (sorted? weighted graph? overlapping subproblems?) and outputs the recommended algorithm with justification.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Benchmark harness | "Test framework for speed" | Standardized system measuring algorithms under controlled inputs, metrics, repetitions |
| Cookbook | "Reference guide" | Decision tree mapping problem characteristics to algorithm choices |
| Input distribution | "Test data shape" | Statistical properties of test inputs that determine algorithm performance |
| Wall time | "Actual time" | Elapsed real-world time including OS scheduling, cache effects |
| Adversarial input | "Worst case" | Input constructed to trigger an algorithm's worst-case behavior |

## Further Reading

- Skiena, *The Algorithm Design Manual*, Ch. 4–5. Sedgewick & Wayne, *Algorithms*, Part I–II.
- Google Benchmark: https://github.com/google/benchmark
- Criterion (Rust): https://github.com/bheisler/criterion.rs
