# Branch Prediction & Layout Tricks — Reference Card

## Misprediction Penalty

| Architecture | Penalty (cycles) |
|---|---|
| Modern x86 (Skylake+) | ~15-20 |
| ARM Cortex-A76 | ~8-12 |
| Apple M1/M2 | ~10-15 |

**Rule of thumb:** A misprediction costs the same as 15-20 simple integer ops.

---

## Branchless Patterns

### Conditional addition
```cpp
// Branchy
if (x > threshold) sum += x;

// Branchless: boolean multiplication
sum += (x > threshold) * x;

// Branchless: ternary (compiles to cmov)
sum += (x > threshold) ? x : 0;
```

### Min / Max / Clamp
```cpp
// Branchy
if (x < lo) x = lo;
if (x > hi) x = hi;

// Branchless: std::min/std::max (cmov)
x = std::max(lo, std::min(x, hi));

// Branchless: bitmask (power-of-2 range, unsigned)
x = x & 0xFF;  // clamp to [0, 255]
```

### Multi-way branch → Lookup table
```cpp
// Branchy: 4-way if/else
int bucket;
if (x < 64)       bucket = 0;
else if (x < 128)  bucket = 1;
else if (x < 192)  bucket = 2;
else               bucket = 3;

// Branchless: arithmetic
int bucket = (x >= 64) + (x >= 128) + (x >= 192);
```

### Partition to eliminate branch
```cpp
auto mid = std::partition(data.begin(), data.end(),
                          [](int x) { return x <= threshold; });
// Process each partition with no branch in the loop
```

### Sorting network (N=4)
```cpp
void sort4(int& a, int& b, int& c, int& d) {
    using std::swap;
    if (a > b) swap(a, b);
    if (c > d) swap(c, d);
    if (a > c) swap(a, c);
    if (b > d) swap(b, d);
    if (b > c) swap(b, c);
}
```

---

## cmov Equivalents

| Branchy pattern | Branchless equivalent |
|---|---|
| `if (c) x = y;` | `x = c ? y : x;` → `cmov` |
| `if (c) x += y;` | `x += c ? y : 0;` |
| `if (c) x = a; else x = b;` | `x = c ? a : b;` → `cmov` |
| `if (x < lo) x = lo;` | `x = std::max(x, lo);` → `cmov` |
| `if (x > hi) x = hi;` | `x = std::min(x, hi);` → `cmov` |
| `abs(x)` | `(x < 0) ? -x : x;` → `cmov` |

**Note:** Compile with `-O2` and check assembly (`-S`) to confirm cmov emission. The compiler often does this for you.

---

## likely/unlikely Syntax

### C++20 Attributes
```cpp
if (error_code) [[unlikely]] {
    handle_error();
}

if (ptr != nullptr) [[likely]] {
    use_ptr(ptr);
}
```

### GCC / Clang Builtins
```cpp
if (__builtin_expect(condition, 1))   // likely true
    hot_path();

if (__builtin_expect(condition, 0))   // unlikely true
    cold_path();
```

### Linux Kernel Macros
```cpp
// These are standard in Linux kernel code
#define likely(x)   __builtin_expect(!!(x), 1)
#define unlikely(x) __builtin_expect(!!(x), 0)
```

### When hints help vs. don't

| Scenario | Hint helps? | Why |
|---|---|---|
| Error path taken <1% of time | Yes | Cold code moved out-of-line, static predictor initialized correctly |
| Branch is ~50/50 unpredictable | No | Neither path is "likely" — no correct hint to give |
| Branch is 99% predictable | Marginal | Predictor already learns the pattern quickly |
| Branch outside hot loop | No | One misprediction is negligible |

---

## Struct Layout Tips

### Hot/Cold Split
```cpp
// BEFORE: 420 bytes, 4% cache utilization in hot loop
struct EntityNaive {
    int x, y, z;          // hot (12 bytes)
    float health;          // hot (4 bytes)
    char name[64];         // cold
    char description[256]; // cold
    int inventory[20];     // cold (80 bytes)
};

// AFTER: hot = 20 bytes, ~67% cache utilization
struct EntityHot {
    int x, y, z;     // 12 bytes
    float health;     // 4 bytes
    int cold_id;      // 4 bytes (index into cold array)
};

struct EntityCold {
    int entity_id;
    char name[64];
    char description[256];
    int inventory[20];
};
```

### Cache-line Alignment (prevent false sharing)
```cpp
struct alignas(64) PaddedCounter {
    std::atomic<int64_t> count;
};
// Each counter gets its own cache line — no false sharing between threads
```

### Field ordering by access frequency
```cpp
// Put hot fields first (lower addresses = better prefetch)
struct Good {
    int hot_field_1;     // accessed every iteration
    int hot_field_2;
    int warm_field;      // accessed sometimes
    char cold_data[256]; // rarely accessed
};

// Don't interleave hot and cold
struct Bad {
    int hot_field_1;
    char cold_data[256]; // splits hot fields across cache lines
    int hot_field_2;
};
```

---

## Quick Decision Flowchart

```
Is the branch in a hot loop?
├── No → Don't optimize. Readability > micro-optimization.
└── Yes → Is the condition predictable (>90% one way)?
    ├── Yes → Keep the branch. Likely/unlikely hint may help marginally.
    └── No → Can you sort/partition the data?
        ├── Yes → Sort or partition, then use the branch.
        └── No → Go branchless (cmov, lookup table, arithmetic).
```

---

## Compilation & Verification

```bash
# Compile with optimizations
g++ -O2 -march=native main.cpp -o bench

# Verify cmov emission (look for cmov* instructions)
g++ -O2 -S main.cpp && grep cmov main.s

# Profile with perf
perf stat -e branches,branch-misses ./bench
```