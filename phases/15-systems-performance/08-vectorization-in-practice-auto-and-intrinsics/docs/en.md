# Vectorization in Practice (auto and intrinsics)

> Modern CPUs can crunch 4, 8, or even 16 values per instruction. This lesson teaches you how to unlock that throughput — and why the compiler sometimes refuses to do it for you.

**Type:** Learn
**Languages:** C++, Rust
**Prerequisites:** Phase 15 lessons 01–07
**Time:** ~75 minutes

## Learning Objectives

- Explain SIMD (Single Instruction, Multiple Data) and how SSE/AVX/AVX2/AVX-512 differ in register width and capability.
- Predict when a compiler will auto-vectorize a loop and when it will not.
- Write explicit SIMD intrinsics in C++ (`__m128`, `__m256`, `_mm_*` functions) and Rust (`std::arch::x86_64`).
- Benchmark scalar vs auto-vectorized vs intrinsics versions of the same algorithm.
- Understand alignment requirements for SIMD loads and stores.
- Use masks and predicates with AVX-512-style operations.

## The Problem

You have an array of a million `float` values. You need to sum them, compute a dot product, or filter out elements below a threshold. Your CPU runs at 3 GHz, so a naïve scalar loop processes one float per cycle — one million cycles just for the adds. But your CPU has 512-bit-wide vector registers sitting idle. Each `__m512` can hold 16 floats, and a single `_mm512_add_ps` computes 16 additions in the same time as one scalar `+`. The theoretical speedup is 16×.

The catch: the compiler does not always vectorize your loops. Data dependencies, unknown trip counts, pointer aliasing, and complex control flow all block auto-vectorization. When the compiler gives up, you must write intrinsics yourself — or restructure your code until the compiler can help you.

This lesson sits in **Phase 15 — Systems Programming & Performance**. Without understanding vectorization, you cannot reach the performance ceiling of modern hardware. Every hot loop that processes arrays is leaving 4–16× performance on the table if it runs scalar.

## The Concept

### SIMD: One Instruction, Many Lanes

A SIMD (Single Instruction, Multiple Data) instruction applies the same operation to multiple data elements simultaneously. Think of it as a wide highway — instead of one car (scalar) traveling one lane at a time, 4/8/16 cars travel side-by-side in parallel lanes.

```
Scalar:    add r1, r2          — one value
SSE:       addps xmm1, xmm2   — 4 × float32  (128-bit)
AVX:       vaddps ymm1, ymm2  — 8 × float32  (256-bit)
AVX-512:   vaddps zmm1, zmm2  — 16 × float32 (512-bit)
```

### The x86 SIMD Family Tree

| ISA       | Registers | Width   | float32 lanes | float64 lanes | Introduced |
|-----------|-----------|---------|---------------|---------------|------------|
| SSE       | xmm0–15   | 128-bit | 4             | 2             | 1999       |
| AVX       | ymm0–15   | 256-bit | 8             | 4             | 2011       |
| AVX2      | ymm0–15   | 256-bit | 8             | 4             | 2013       |
| AVX-512F  | zmm0–31   | 512-bit | 16            | 8             | 2017       |

AVX2 added integer SIMD operations to AVX. AVX-512 added mask registers (k0–k7), more registers (32 instead of 16), and gather/scatter instructions. Not all CPUs support all ISAs — check `cpuid` or `/proc/cpuinfo`.

### Auto-Vectorization: When the Compiler Helps

Modern compilers (gcc, clang, MSVC, rustc via LLVM) can automatically transform simple loops into SIMD code. The compiler's vectorizer analyzes your loop and, if it can prove correctness, emits SIMD instructions.

**Conditions for auto-vectorization:**

1. **No loop-carried dependencies** — each iteration is independent.
2. **Known or computable trip count** — the compiler needs to know how many iterations run, or at least prove the trip count is a multiple of the vector width.
3. **Simple control flow** — no complex branching inside the loop. `if` statements become masked operations, but deeply nested control defeats vectorization.
4. **No pointer aliasing** — in C/C++, the compiler must assume that `float* a` and `float* b` might overlap. Use `__restrict__` or `restrict` to promise they don't.
5. **Straight-line code** — function calls (except inlined ones), `printf`, and unknown operations block vectorization.

**Example the compiler can vectorize:**

```cpp
void sum_auto(const float* __restrict__ data, int n, float* __restrict__ out) {
    float acc = 0.0f;
    for (int i = 0; i < n; i++) {
        acc += data[i];
    }
    *out = acc;
}
```

**Example the compiler will NOT vectorize:**

```cpp
void sum_deps(float* data, int n, float* out) {
    // data[i] depends on data[i-1] — loop-carried dependency
    for (int i = 1; i < n; i++) {
        data[i] += data[i-1];
    }
}
```

### When Auto-Vectorization Fails

| Pattern | Why it fails | Fix |
|---------|-------------|-----|
| `a[i] = a[i-1] + 1` | Loop-carried dependency | Restructure algorithm or use scan intrinsics |
| `for (int i = 0; i < n; i++)` where `n` is unknown | Unknown trip count | Use `__restrict__` and `#pragma omp simd` or `__attribute__((optimize("O3")))` |
| `if (arr[i] > threshold) sum += arr[i]` | Complex control flow | Rewrite as masked accumulate or separate filter |
| `void add(float* a, float* b)` | Possible pointer aliasing | Add `__restrict__` |
| `sum += arr[idx[i]]` | Gather (non-contiguous access) | Use `_mm256_i32gather_ps` or restructure data |
| Virtual / indirect calls | Cannotvectorize across calls | Inline or use function pointers the compiler can devirtualize |

### Intrinsics: Taking Control

When the compiler can't or won't vectorize, you write intrinsics — C function-like macros that map 1:1 to SIMD instructions. You gain full control but take full responsibility.

**SSE dot product pattern (simplified):**

```cpp
#include <immintrin.h>

float dot_sse(const float* a, const float* b, int n) {
    __m128 sum_vec = _mm_setzero_ps();
    int i = 0;
    for (; i + 3 < n; i += 4) {
        __m128 va = _mm_loadu_ps(a + i);
        __m128 vb = _mm_loadu_ps(b + i);
        sum_vec = _mm_add_ps(sum_vec, _mm_mul_ps(va, vb));
    }
    // Horizontal sum of 4 floats in sum_vec
    float result[4];
    _mm_storeu_ps(result, sum_vec);
    float sum = result[0] + result[1] + result[2] + result[3];
    for (; i < n; i++) sum += a[i] * b[i];
    return sum;
}
```

Key points:
- `_mm_setzero_ps()` creates a zeroed vector.
- `_mm_loadu_ps` loads 4 floats (unaligned).
- `_mm_add_ps` and `_mm_mul_ps` operate on all 4 lanes.
- The tail loop handles the remainder when `n` is not a multiple of 4.
- Horizontal reduction (summing across lanes) requires shuffles or extraction.

### Rust SIMD

Rust has two paths to SIMD:

1. **`std::arch::x86_64`** (stable) — direct intrinsics, `unsafe`, same naming as C (`_mm256_add_ps` etc.).
2. **`std::simd`** (unstable, nightly) — safe, portable SIMD types like `f32x4`, `f32x8`. Higher-level, but requires nightly.

For production code on stable Rust, use `std::arch::x86_64` behind a feature gate or the `packed_simd` / `wide` crates.

```rust
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

unsafe fn dot_avx(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = _mm256_setzero_ps();
    let mut i = 0;
    let n = a.len();
    while i + 8 <= n {
        let va = _mm256_loadu_ps(a.as_ptr().add(i));
        let vb = _mm256_loadu_ps(b.as_ptr().add(i));
        sum = _mm256_add_ps(sum, _mm256_mul_ps(va, vb));
        i += 8;
    }
    // Reduce 8 floats in sum ...
    let mut result = [0.0f32; 8];
    _mm256_storeu_ps(result.as_mut_ptr(), sum);
    let mut total = result.iter().sum::<f32>();
    while i < n { total += a[i] * b[i]; i += 1; }
    total
}
```

### Gather and Scatter

Not all data is contiguous. Gather loads elements at non-contiguous indices:

```cpp
// AVX2 gather: load 8 floats using 8 indices
__m256 vg = _mm256_i32gather_ps(base, vindex, 4);
// '4' is the scale factor (sizeof(float))
```

Scatter stores to non-contiguous indices (AVX-512 only):

```cpp
// AVX-512 scatter
_mm512_i32scatter_ps(base, vindex, vsrc, 4);
```

Gather/scatter are slower than contiguous loads/stores because they hit different cache lines, but they're faster than scalar loops for sparse access patterns.

### Masks and Predicates (AVX-512)

AVX-512 introduced 8 mask registers (`k0`–`k7`), enabling per-lane predication:

```cpp
// Only add where the mask says so
__m512 va = _mm512_loadu_ps(a);
__m512 vb = _mm512_loadu_ps(b);
__mmask16 mask = _mm512_cmp_ps_mask(va, vb, _MM_CMP_GT_OS);
__m512 result = _mm512_mask_add_ps(va, mask, va, vb);
```

This eliminates the need for separate scalar filter loops — you compute 16 lanes, keep only the ones that match.

### Alignment: movaps vs movups

SIMD instructions come in aligned and unaligned variants:

| Instruction | Meaning | Performance |
|-------------|---------|-------------|
| `_mm_load_ps` / `movaps` | Must be 16-byte aligned | Historically faster, now similar on modern CPUs |
| `_mm_loadu_ps` / `movups` | Can be any alignment | Safe, use unless you control alignment |

For best performance:
- Align allocations to 32 or 64 bytes (`aligned_alloc`, `_mm_malloc`, or `std::aligned_alloc`).
- Use `_mm_load_ps` (aligned) when you guarantee alignment.
- Use `_mm_loadu_ps` (unaligned) when you can't guarantee it — the penalty is tiny on modern CPUs.

## Build It

### Step 1: Scalar Baselines

First, write simple scalar versions:

```cpp
float dot_scalar(const float* a, const float* b, int n) {
    float sum = 0.0f;
    for (int i = 0; i < n; i++) sum += a[i] * b[i];
    return sum;
}

float sum_scalar(const float* data, int n) {
    float s = 0.0f;
    for (int i = 0; i < n; i++) s += data[i];
    return s;
}
```

Compile with `-O2` and no vectorization flags. This is your baseline.

### Step 2: Auto-Vectorized Versions

Add `__restrict__`, compile with `-O3 -march=native`, and the compiler should vectorize:

```cpp
float dot_auto(const float* __restrict__ a,
               const float* __restrict__ b, int n) {
    float sum = 0.0f;
    for (int i = 0; i < n; i++) sum += a[i] * b[i];
    return sum;
}
```

Check with `gcc -O3 -fopt-info-vec-optimized` or `clang -Rpass=loop-vectorize` to see if vectorization happened.

### Step 3: Intrinsics Versions

Write SSE and AVX intrinsics for dot product and sum (see full code in `code/main.cpp`).

### Step 4: Benchmark All Approaches

Time each version with `std::chrono` (C++) or `std::time::Instant` (Rust). Run many iterations and take the median. Print a comparison table.

## Use It

### How Production Systems Do This

- **BLAS** (OpenBLAS, MKL, BLIS): Hand-tuned intrinsics for every CPU microarchitecture. GEMM kernels are assembly-level optimized.
- **Eigen**: Uses intrinsics with fallbacks. Compiles with `-march=native` for best results.
- **glibc**: `memcpy` and `memset` use AVX2/AVX-512 rep movsb where available.
- **LLVM auto-vectorizer**: The engine behind clang and rustc auto-vectorization. Pass `-Rpass=loop-vectorize -Rpass-missed=loop-vectorize` to see what it vectorizes and why it skips some loops.

Compare your hand-written intrinsics against the auto-vectorized version. On simple operations like dot product, auto-vectorization often matches intrinsics. Intrinsics win when:
- You need horizontal reductions (sum across lanes) that the compiler doesn't optimize well.
- You need gather/scatter or masked operations.
- You need specific instruction sequences the compiler won't emit.

## Read the Source

- **LLVM Vectorizer**: `llvm/lib/Transforms/Vectorize/LoopVectorize.cpp` — the auto-vectorization pass that powers clang and rustc. Read the `processLoop` method to see the decision logic.
- **OpenBLAS microkernels**: `kernel/x86_64/dot.c` — hand-written dot product kernels with intrinsics and assembly for every ISA level.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`simd_reference.md`** — A quick-reference card with SSE/AVX/AVX2/AVX-512 register widths, common intrinsics, auto-vectorization checklist, alignment rules, and reduction patterns.

## Exercises

1. **Easy** — Modify the dot product benchmark to use `double` instead of `float`. How does the SIMD width change? Re-run and compare.
2. **Medium** — Implement a `filter` function that copies only elements > 0 from a source array to a destination array using AVX2 compare + mask moves. Benchmark against a scalar filter.
3. **Hard** — Implement an AVX-512 masked dot product where you skip `NaN` values in either input. Use `__mmask16` to create a mask excluding NaN lanes and `_mm512_mask_add_ps` to accumulate only valid lanes.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| SIMD | "vectorization" | Single Instruction Multiple Data — one CPU instruction processes multiple data lanes simultaneously |
| SSE | "the 128-bit one" | Streaming SIMD Extensions — 128-bit registers (`xmm`), 4×float32 or 2×float64 per operation |
| AVX | "the 256-bit one" | Advanced Vector Extensions — 256-bit registers (`ymm`), 8×float32 or 4×float64 |
| AVX-512 | "the 512-bit one" | 512-bit registers (`zmm`) plus 8 mask registers (`k0–k7`), 16×float32 |
| Auto-vectorization | "compiler does it" | The compiler's loop vectorizer pass transforms eligible loops into SIMD without programmer action |
| Intrinsics | "the _mm functions" | C macro-like functions that map 1:1 to SIMD instructions, giving explicit control over vectorization |
| Gather/Scatter | "indexed load/store" | SIMD instructions that load from or store to non-contiguous addresses using an index vector |
| Mask register | "predicate" | AVX-512's `k0–k7` registers — per-lane boolean enabling conditional execution without branches |
| Alignment | "aligned memory" | Data addresses must be multiples of the vector width (16/32/64 bytes) for aligned SIMD loads |

## Further Reading

- **Intel Intrinsics Guide** — https://www.intel.com/content/www/us/en/docs/intrinsics-guide/ — searchable reference for all x86 intrinsics.
- **Intel® 64 and IA-32 Architectures Optimization Reference Manual** — the definitive guide to SIMD optimization on Intel CPUs.
- **LLVM Loop Vectorizer Documentation** — https://llvm.org/docs/Vectorizers.html — explains auto-vectorization heuristics.
- **"SIMD for C++ Developers"** — https://www.codeproject.com/Articles/87500/SIMD-for-C-Developers — practical guide.
- **Rust `std::simd` tracking issue** — https://github.com/rust-lang/rust/issues/48556 — status of portable SIMD in Rust.