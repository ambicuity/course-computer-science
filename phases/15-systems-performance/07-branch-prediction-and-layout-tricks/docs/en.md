# Branch Prediction and Layout Tricks

> Modern CPUs guess which way your `if` will go — and when they guess wrong, you pay 15–20 cycles. This lesson teaches you to make branches predictable or eliminate them entirely.

**Type:** Learn
**Languages:** C++
**Prerequisites:** Phase 15 lessons 01–06
**Time:** ~60 minutes

## Learning Objectives

- Explain how branch predictors work and why mispredictions cost 15–20 cycles on modern x86.
- Identify code patterns where branches hurt and where they don't.
- Rewrite branchy code using branchless techniques: `cmov`, lookup tables, arithmetic, and bitmasking.
- Use `__builtin_expect` and C++20 `[[likely]]`/`[[unlikely]]` hints correctly.
- Restructure data layouts to improve both branch prediction and cache performance.
- Benchmark and interpret sorted vs. unsorted data, branchy vs. branchless, and hot/cold struct splits.

## The Problem

You're processing 100 million integers. For each one, you check: is it greater than 128? If so, add it to a running sum. The code is trivial:

```cpp
for (int i = 0; i < n; i++) {
    if (data[i] > 128) sum += data[i];
}
```

With **sorted data**, this runs in ~30 ms. With **randomly shuffled data**, the same code takes ~120 ms. The CPU hasn't changed. The data hasn't changed. The algorithm hasn't changed. What happened?

The answer is **branch prediction** — and understanding it is the difference between writing code that's fast by accident and writing code that's fast by design.

Without mastering branch prediction and data layout, you can't honestly measure or tune performance. You'll optimize the wrong thing, chase the wrong bottleneck, and wonder why your "optimized" code is slower than the naive version.

## The Concept

### How Branch Prediction Works

Modern out-of-order CPUs have deep pipelines — 14–19 stages on recent Intel cores. When the fetch stage encounters a conditional branch (`if`, `switch`, loop back-edge), it doesn't know which way the branch will go for another 10+ cycles. Rather than stall, the CPU **predicts** the outcome and speculatively executes along the predicted path.

The branch predictor maintains per-branch history tables (often using a two-level adaptive scheme like TAGE). For predictable patterns — always taken, never taken, or alternating — it achieves near-100% accuracy. Across real workloads, modern predictors hit ~95% accuracy.

When the prediction is correct: you never notice. The pipeline kept flowing.

When the prediction is **wrong**: the CPU must flush the entire pipeline, discard all speculative work, and re-fetch from the correct path. On a modern x86 core, this costs **15–20 cycles**. If your branch mispredicts frequently, those 15-cycle penalties dominate your runtime.

### Worked Example: The Sorting Speedup

Consider the `if (data[i] > 128)` loop on 100 million values in [0, 255]:

| Data arrangement | Branch pattern | Misprediction rate | Time |
|---|---|---|---|
| Sorted ascending | 0→127 all "not taken", 128→255 all "taken" | ~0% | ~30 ms |
| Randomly shuffled | Each element ~50% chance either way | ~50% | ~120 ms |

With sorted data, the predictor learns the pattern almost instantly: "always not taken" for the first half, "always taken" for the second. Misprediction rate drops to near zero.

With shuffled data, every branch is a coin flip. The predictor is wrong half the time. At ~15 cycles per misprediction, that's ~750M wasted cycles — roughly 4× the baseline cost.

**The speedup from sorting isn't about the algorithm — it's about making the branch predictable.**

### When Branches Matter (And When They Don't)

Branches hurt when:

1. **The condition is unpredictable** — near 50/50 probability, random pattern.
2. **The branch is in a tight loop** — millions of iterations magnify the cost.
3. **The loop body is small** — the misprediction overhead is large relative to the work.

Branches are fine when:

1. **The condition is highly predictable** — e.g., "almost always true" or follows a regular pattern.
2. **The branch is outside hot loops** — one misprediction in a million outer-loop iterations is noise.
3. **The branch body is expensive** — if the work per iteration is 1000 cycles, a 15-cycle misprediction is 1.5% overhead.

The takeaway: **don't blindly eliminate branches. Eliminate unpredictable branches in hot loops.**

## Build It

### Step 1: Minimal Version — See the Problem

```cpp
#include <vector>
#include <random>
#include <iostream>
#include <algorithm>

int main() {
    const int N = 100'000'000;
    std::vector<int> data(N);
    std::mt19937 rng(42);
    std::uniform_int_distribution<int> dist(0, 255);
    for (auto& x : data) x = dist(rng);

    // Unsorted: unpredictable branches
    long long sum = 0;
    auto t1 = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < N; i++) {
        if (data[i] > 128) sum += data[i];
    }
    auto t2 = std::chrono::high_resolution_clock::now();
    std::cout << "Unsorted: " << (t2-t1).count() << " ns, sum=" << sum << "\n";

    // Sorted: predictable branches
    std::sort(data.begin(), data.end());
    sum = 0;
    t1 = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < N; i++) {
        if (data[i] > 128) sum += data[i];
    }
    t2 = std::chrono::high_resolution_clock::now();
    std::cout << "Sorted:   " << (t2-t1).count() << " ns, sum=" << sum << "\n";
}
```

Run this and you'll see a 3–4× speedup from sorting alone. No algorithm changed. Only data layout changed.

### Step 2: Branchless Techniques

Now we eliminate the branch entirely. When there's no branch, the predictor has nothing to mispredict.

**Conditional move (cmov):**

The compiler can transform some `if` statements into conditional moves. But it often needs help — especially when the `if` body has side effects or is complex. The simplest pattern uses arithmetic:

```cpp
// Branchless: instead of if (x > 128) sum += x
sum += (x > 128) * x;   // boolean promotes to 0 or 1
```

On x86, the compiler typically emits `cmp; seta; imul; add` or a `cmov` sequence — no branch.

**Lookup table:**

For multi-way branches (switch-like), a lookup table eliminates the branch:

```cpp
// Branchy: classify into 4 buckets
int bucket;
if (x < 64) bucket = 0;
else if (x < 128) bucket = 1;
else if (x < 192) bucket = 2;
else bucket = 3;

// Branchless: lookup table
static const int thresholds[] = {64, 128, 192, 256};
int bucket = 0;
for (int t : thresholds) bucket += (x >= t);
// Equivalent: bucket = (x >= 64) + (x >= 128) + (x >= 192);
```

**Bitmasking (min/max clamping):**

```cpp
// Branchy: clamp x to [0, 255]
if (x < 0) x = 0;
if (x > 255) x = 255;

// Branchless: using std::min/std::max (compilers emit cmov)
x = std::max(0, std::min(x, 255));

// Branchless: bitwise (for power-of-2 ranges)
x &= 0xFF;  // clamps x to [0, 255] assuming x >= 0
```

### Step 3: likely/unlikely Hints

C++20 introduces `[[likely]]` and `[[unlikely]]` attributes. GCC and Clang also support `__builtin_expect`. These tell the compiler which path to optimize for in the branch layout:

```cpp
// C++20 attributes
if (error_code) [[unlikely]] {
    handle_error();  // cold path — placed out-of-line
}

// GCC/Clang builtin
if (__builtin_expect(ptr != nullptr, 1)) {
    use_ptr(ptr);  // hot path — likely taken
}
```

These hints affect:
1. **Code layout**: the likely path is placed in the fall-through position (better I-cache behavior).
2. **Branch prediction**: static predictor initialized to favor the hint.
3. **Assembly generation**: `likely` paths use forward jumps (often "not taken" = faster).

They do NOT help when the branch is truly 50/50. They help when one path is taken >90% of the time.

### Step 4: Struct Layout — Hot/Cold Splitting

Consider a game entity struct:

```cpp
struct EntityNaive {
    int x, y, z;            // 12 bytes — hot: updated every frame
    float health;           // 4 bytes — hot: updated every frame
    char name[64];          // 64 bytes — cold: rarely accessed
    char description[256]; // 256 bytes — cold: rarely accessed
    int inventory[20];     // 80 bytes — cold: accessed on interaction
};
// Total: ~420 bytes. Only 16 bytes are hot.
```

When you iterate over an array of `EntityNaive` to update positions, you pull 420 bytes through cache per element but only use 16. That's ~4% cache utilization.

**Hot/cold split:**

```cpp
struct EntityHot {
    int x, y, z;     // 12 bytes
    float health;    // 4 bytes
    int cold_id;     // 4 bytes — index into cold data
}; // 20 bytes — fits multiple per cache line

struct EntityCold {
    int entity_id;
    char name[64];
    char description[256];
    int inventory[20];
};
// Hot loop now processes 20 bytes per entity instead of 420.
// Cache holds 21 entities per line instead of 1.
```

**Cache-line alignment** can also help when multiple threads touch different instances:

```cpp
struct alignas(64) PaddedCounter {
    std::atomic<int64_t> count;
};
// Prevents false sharing between threads.
```

### Step 5: Data Layout for Branch Prediction

The sorting example from Step 1 shows a key insight: **data ordering affects branch predictability**. You can exploit this structurally:

1. **Sort data before processing** when the branch condition is monotonic (e.g., `x > threshold`).
2. **Partition data** by predicate into separate arrays, then process each array without branches:

```cpp
auto mid = std::partition(data.begin(), data.end(),
                          [](int x) { return x <= 128; });
// Process values <= 128 — no branch needed
for (auto it = data.begin(); it != mid; it++) process_below(*it);
// Process values > 128 — no branch needed
for (auto it = mid; it != data.end(); it++) process_above(*it);
```

3. **Branchless sorting networks** for small fixed-size arrays (N ≤ 16) — use `std::sort` for large arrays but consider sorting networks for small hot loops:

```cpp
// Sorting network for 4 elements (5 comparisons, all branchless with cmov)
void sort4(int& a, int& b, int& c, int& d) {
    if (a > b) std::swap(a, b);  // compiler emits conditional moves
    if (c > d) std::swap(c, d);
    if (a > c) std::swap(a, c);
    if (b > d) std::swap(b, d);
    if (b > c) std::swap(b, c);
}
```

### When NOT to Go Branchless

Branchless code has costs:

1. **Predictable branches are free** — a correctly predicted branch costs ~1 cycle. A `cmov` also costs ~1 cycle. No win, but readability suffers.
2. **Branchless can hurt on data-dependent paths** — `cmov` evaluates both paths and adds a data dependency. On long dependency chains, a well-predicted branch can break the chain and be faster.
3. **Readability** — `(x > 128) * x` is less readable than `if (x > 128) sum += x`. Future maintainers will thank you for writing clear code and only going branchless in hot loops.
4. **Compiler already does it** — modern compilers with `-O2` often emit `cmov` for simple ternaries. Check the assembly before hand-optimizing.

The rule: **profile first, optimize branches second, and only in hot loops with unpredictable conditions.**

## Use It

### How production systems handle this

**Linux kernel** uses `likely()`/`unlikely()` macros (wrappers around `__builtin_expect`) extensively — over 25,000 uses in the source tree. Error paths, NULL checks, and rare conditions are always annotated.

**PostgreSQL** sorts data for sequential scans when the planner predicts it will improve cache behavior. The `tuplesort` module sorts small batches in memory precisely because sorted access patterns are faster for both cache and branch prediction.

**LLVM** implements branchless `std::sort` for small arrays (N ≤ 16) using sorting networks. The `llvm::sort` utility also has optimized paths for already-sorted data (branch on comparison result — predictable) vs. random data.

**FaceBook's Folly** library has `unlikely()` and `assume()` macros, plus a dedicated `branchless` header with patterns like `branchless_min`, `branchless_max`, and conditional moves for performance-critical paths.

Compare your hand-built branchless patterns against what the compiler generates: compile with `-O2 -S` and look for `cmov` instructions. If the compiler already emits branchless code, your hand-written version adds complexity without benefit.

## Read the Source

- **Linux kernel** `include/linux/compiler.h` — `likely()`/`unlikely()` macro definitions.
- **LLVM** `llvm/include/llvm/ADT/STLExtras.h` — sorting network for small N in `llvm::sort`.
- **Folly** `folly/lang/Assume.h` — `assume()` and `branchless` utilities.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`branch_layout_reference.md`** — A reference card with branchless patterns, `cmov` equivalents, `likely`/`unlikely` syntax, and struct layout tips. Keep it alongside your performance toolbox for every future optimization pass.

## Exercises

1. **Easy** — Write a benchmark comparing sorted vs. unsorted data on a simple `if (data[i] > threshold)` loop. Verify the 3–4× speedup. Then change the threshold so the branch is 90% taken. How does the unsorted case perform now?

2. **Medium** — Implement a branchless version of a 5-way classifier (bucket values into 5 ranges). Compare against the `if/else` chain on both sorted and random data. Measure the crossover point where branchless becomes faster.

3. **Hard** — Take a real data structure (e.g., a game entity with 8+ fields) and perform a hot/cold split. Benchmark the original vs. split layout: (a) iterating to update positions, (b) random access for the full entity. How does each layout affect each access pattern? Write up the trade-offs.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Branch prediction | "The CPU guesses where to go" | A hardware mechanism using per-branch history tables to predict whether a conditional branch will be taken or not, enabling speculative execution past the branch |
| Misprediction penalty | "Branch miss costs cycles" | The 15–20 cycle cost of flushing the pipeline when the predictor guessed wrong — all speculative work since the branch is discarded |
| Branchless | "Remove the if" | Coding patterns (cmov, lookup tables, arithmetic) that eliminate conditional branches entirely, trading a potential misprediction for a small fixed cost |
| `cmov` | "Conditional move instruction" | x86 instruction that selects between two values based on a condition flag — the canonical branchless primitive that compilers use for simple ternaries |
| `[[likely]]`/`[[unlikely]]` | "Branch hints" | C++20 attributes (and `__builtin_expect`) that tell the compiler which branch path is more probable, affecting code layout and static predictor initialization |
| Hot/cold split | "Split the struct" | A layout optimization where frequently accessed (hot) fields are grouped together separate from rarely accessed (cold) fields, improving cache utilization in hot loops |
| Sorting network | "Branchless sort for small N" | A fixed comparison network that sorts small arrays (N ≤ 16) using only `cmov`/conditional swap operations — no data-dependent branches |
| False sharing | "Threads fighting over a cache line" | When threads on different cores modify different variables that share the same cache line, causing the line to bounce between cores wastefully |

## Further Reading

- [What Every Programmer Should Know About Memory](https://people.freebsd.org/~lstewart/articles/cpumemory.pdf) — Ulrich Drepper, 2007. Section 3 on CPU architecture and branch prediction.
- [Fast Branchless Binary Search](https://orlp.net/blog/fast-branchless-binary-search/) — Orson Peters, 2023. Branchless binary search using `cmov` sequences.
- [Intel Optimization Manual](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html) — Chapter 3 on branch prediction internals for Intel microarchitectures.
- [Data-Oriented Design](https://www.dataorienteddesign.com/dodbook/) — Richard Fabian, on struct layout and cache-friendly data organization.
- [CppCon 2021: Chandler Carruth "There Are No Zero-Cost Abstractions"](https://www.youtube.com/watch?v=rHIkrotSwcc) — On the real costs of abstracted code including branch mispredictions.