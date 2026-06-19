# SIMD Programming in Practice

> Use CPU vector extensions (SSE, AVX2, NEON) to get 4–8× throughput on
> data-parallel loops.  Auto-vectorization is fragile — intrinsics and
> portable SIMD libraries give you reliable speed.

**Type:** Build
**Languages:** C++, Rust
**Prerequisites:** Phase 13 lessons 01–17 (especially 17 "Parallel Patterns")
**Time:** ~75 minutes

---

## Learning Objectives

- Explain the SIMD (Single Instruction, Multiple Data) model and contrast
  it with SISD and MIMD.
- Name four real SIMD ISA extensions and their vector widths: SSE (128 b),
  AVX2 (256 b), AVX-512 (512 b), NEON (128 b).
- Identify why compilers fail to auto-vectorize common loops (pointer
  aliasing, non-unit strides, loop-carried dependencies).
- Write explicit AVX2 intrinsics in C++ for element-wise addition, dot
  product, and 4×4 matrix multiplication.
- Write equivalent code using Rust's portable SIMD (`std::simd`).
- Measure speedup over scalar code and explain the gap.

---

## The Problem

You have a hot loop that processes millions of floats per second:

```c
for (int i = 0; i < N; i++) c[i] = a[i] + b[i];
```

On a modern x86-64 core this loop runs at **one float per cycle** (IPC ≈ 1
for scalar FP add).  But the same core can issue **eight float additions
per cycle** using AVX2.  You are leaving 87 % of your CPU's throughput on
the table.

Even worse: algorithms that *look* like they should vectorise often
don't, because of pointer aliasing, insufficient alignment, or irregular
memory access patterns.  Relying on the compiler is not enough — you
must understand the hardware's vector model and write code that
exploits it explicitly.

---

## The Concept

### SIMD in Flynn's Taxonomy

| Class     | Instructions | Data streams | Examples                 |
|-----------|-------------|--------------|--------------------------|
| SISD      | 1           | 1            | Scalar CPU core          |
| SIMD      | 1           | Many         | SSE, AVX, NEON, SVE      |
| MISD      | Many        | 1            | Fault-tolerant (rare)    |
| MIMD      | Many        | Many         | Multi-core CPU, GPU      |

A **SIMD instruction** executes one operation on multiple data elements
simultaneously.  Each data element occupies one **SIMD lane**.

### Vector Widths

| Extension   | Width   | Float lanes | Int lanes | Introduced |
|-------------|---------|-------------|-----------|------------|
| SSE         | 128 b   | 4 × f32     | 4 × i32   | Pentium 3  |
| AVX         | 256 b   | 8 × f32     | 8 × i32   | Sandy Br.  |
| AVX2        | 256 b   | 8 × f32     | 8 × i32   | Haswell    |
| AVX-512     | 512 b   | 16 × f32    | 16 × i32  | Skylake X  |
| NEON        | 128 b   | 4 × f32     | 4 × i32   | Cortex-A8  |
| SVE         | 128–2048| scalable    | scalable  | Fujitsu A64FX |

- **SSE** (Streaming SIMD Extensions): 128-bit XMM registers.
- **AVX2** (Advanced Vector Extensions 2): 256-bit YMM registers, plus
  gather support (`_mm256_i32gather_ps`).
- **AVX-512**: 512-bit ZMM registers with masking and predication.
- **NEON**: ARM's 128-bit SIMD, standard on all AArch64 CPUs.
- **SVE** (Scalable Vector Extension): vector length unknown at compile
  time; code adapts automatically.

### Auto-Vectorisation and Why It Fails

Compilers (GCC, Clang, MSVC) can automatically translate scalar loops
into SIMD instructions at `-O3` / `-O2`.  The transformation is called
**auto-vectorisation**.  But three common patterns prevent it:

1. **Pointer aliasing** — the compiler cannot prove that `a`, `b`, `c`
   don't overlap in memory:
   ```c
   void add(float *a, float *b, float *c, int n) {
       for (int i = 0; i < n; i++) c[i] = a[i] + b[i];
   }
   ```
   Fix: add `__restrict__` (C99) or `restrict` (C++ via compiler ext.).

2. **Non-unit strides** — the loop accesses memory with stride ≠ 1:
   ```c
   for (int i = 0; i < n; i++) c[i * 3] = a[i] + b[i];
   ```
   Gather/scatter instructions exist (AVX2 gather, AVX-512 scatter) but
   are slower than unit-stride loads.

3. **Loop-carried dependencies** — each iteration depends on the previous:
   ```c
   for (int i = 1; i < n; i++) c[i] = c[i-1] + a[i];  // prefix sum
   ```
   This is a recurrence; the instruction-level parallelism is limited.
   Some patterns can be vectorised with techniques like parallel-prefix.

Even when the compiler *can* vectorise, the generated code may be
suboptimal — missed optimisations, unnecessary alignment checks, or
failure to use the widest available registers.

### Intrinsics vs Portable Libraries

- **Intrinsics** (C++ `#include <immintrin.h>`): compiler built-in
  functions that map 1:1 to hardware instructions.  Maximum control,
  zero portability.
- **Portable SIMD** (C++ `std::experimental::simd`, Rust `std::simd`,
  `xsimd` library, `highway` library): compile once, target multiple
  ISAs via the same source.
- **Auto-vectorisation pragmas** (`#pragma omp simd`, `#pragma clang loop
  vectorize`): hints to the compiler rather than guarantees.

---

## Build It

We will write three implementations of the same operations:

1. **Scalar C++** — baseline.
2. **Auto-vectorised C++** — with `__restrict__` and aligned storage.
3. **AVX2 intrinsics C++** — explicit `_mm256_*` calls.
4. **Rust portable SIMD** — `std::simd::f32x8`.

### Setup

**C++** — compile with:

```bash
g++ -std=c++17 -mavx2 -O3 -fopenmp main.cpp -o simd_bench
```

**Rust** — requires **nightly** channel and `#![feature(portable_simd)]`:

```bash
rustup default nightly
cargo build --release
```

### Step 1: Auto-Vectorisation

`code/main.cpp` contains three versions of `add`:

**Bad** — no annotations, heap allocations may be unaligned:

```c
void add_scalar(const float *a, const float *b, float *c, size_t n) {
    for (size_t i = 0; i < n; i++) c[i] = a[i] + b[i];
}
```

**Good** — `__restrict__` + `alignas(32)` guarantees:

```c
void add_autovec(const float *__restrict__ a,
                 const float *__restrict__ b,
                 float *__restrict__ c, size_t n) {
    #pragma omp simd aligned(a, b, c : 32)
    for (size_t i = 0; i < n; i++) c[i] = a[i] + b[i];
}
```

The `__restrict__` keyword tells the compiler that `a`, `b`, `c` point to
disjoint memory.  `alignas(32)` ensures the data starts at a 32-byte
boundary (matching AVX2 load/store requirements).  The `#pragma omp simd`
explicitly requests SIMD code generation.

Without these annotations, the compiler may still vectorise at `-O3`, but
it will emit runtime alignment checks and fallback paths.

### Step 2: AVX2 Intrinsics

The same operations expressed with `_mm256_*` intrinsics:

```c
#include <immintrin.h>

void add_avx2(const float *__restrict__ a,
              const float *__restrict__ b,
              float *__restrict__ c, size_t n) {
    size_t i = 0;
    for (; i + 8 <= n; i += 8) {
        __m256 va = _mm256_load_ps(&a[i]);   // aligned 256-bit load
        __m256 vb = _mm256_load_ps(&b[i]);
        __m256 vc = _mm256_add_ps(va, vb);    // 8 × f32 add
        _mm256_store_ps(&c[i], vc);           // aligned store
    }
    // Tail handling for remaining elements
    for (; i < n; i++) c[i] = a[i] + b[i];
}
```

Key intrinsics used in the lesson:

| Intrinsic                      | Operation                            | Latency | Throughput |
|-------------------------------|--------------------------------------|---------|------------|
| `_mm256_load_ps`              | Aligned 256-bit load                 | ~4 cy   | 0.5/c      |
| `_mm256_store_ps`             | Aligned 256-bit store                | ~4 cy   | 0.5/c      |
| `_mm256_add_ps`               | 8 × f32 addition                     | ~3 cy   | 0.5/c      |
| `_mm256_mul_ps`               | 8 × f32 multiplication               | ~5 cy   | 0.5/c      |
| `_mm256_hadd_ps`              | Horizontal add (2×2 pairs)           | ~5 cy   | 1/c        |
| `_mm256_set1_ps`              | Broadcast scalar to all lanes        | —       | —          |
| `_mm256_castps256_ps128`      | Extract low 128 bits                 | 0       | 0.25/c     |

**Dot product** with horizontal reduction:

```c
float dot_avx2(const float *__restrict__ a,
               const float *__restrict__ b, size_t n) {
    __m256 sum = _mm256_setzero_ps();
    size_t i = 0;
    for (; i + 8 <= n; i += 8) {
        __m256 va = _mm256_load_ps(&a[i]);
        __m256 vb = _mm256_load_ps(&b[i]);
        sum = _mm256_add_ps(sum, _mm256_mul_ps(va, vb));
    }
    // Horizontal reduction
    __m128 hi = _mm256_extractf128_ps(sum, 1);
    __m128 lo = _mm256_castps256_ps128(sum);
    __m128 sum128 = _mm_add_ps(lo, hi);
    sum128 = _mm_hadd_ps(sum128, sum128);
    sum128 = _mm_hadd_ps(sum128, sum128);
    float result = _mm_cvtss_f32(sum128);
    // Tail
    for (; i < n; i++) result += a[i] * b[i];
    return result;
}
```

**4×4 matrix multiply** using 128-bit (SSE) vectors for column access
plus FMA-like multiply-add:

```c
void matmul_4x4_avx2(const float *__restrict__ A,
                     const float *__restrict__ B,
                     float *__restrict__ C) {
    // Load columns of B (transposed view)
    __m128 b_col0 = _mm_loadu_ps(&B[0]);
    __m128 b_col1 = _mm_loadu_ps(&B[4]);
    __m128 b_col2 = _mm_loadu_ps(&B[8]);
    __m128 b_col3 = _mm_loadu_ps(&B[12]);

    for (int i = 0; i < 4; i++) {
        __m128 c_row = _mm_setzero_ps();
        c_row = _mm_add_ps(c_row, _mm_mul_ps(_mm_set1_ps(A[i*4+0]), b_col0));
        c_row = _mm_add_ps(c_row, _mm_mul_ps(_mm_set1_ps(A[i*4+1]), b_col1));
        c_row = _mm_add_ps(c_row, _mm_mul_ps(_mm_set1_ps(A[i*4+2]), b_col2));
        c_row = _mm_add_ps(c_row, _mm_mul_ps(_mm_set1_ps(A[i*4+3]), b_col3));
        _mm_storeu_ps(&C[i*4], c_row);
    }
}
```

### Step 3: Rust Portable SIMD

`code/main.rs` implements the same three operations using
`std::simd::f32x8` (256-bit, 8 × f32).

```rust
#![feature(portable_simd)]
use std::simd::{f32x8, f32x4, Simd};

fn add_simd(a: &[f32], b: &[f32], c: &mut [f32]) {
    let mut i = 0;
    while i + 8 <= a.len() {
        let va = f32x8::from_slice(&a[i..]);
        let vb = f32x8::from_slice(&b[i..]);
        (va + vb).copy_to_slice(&mut c[i..]);
        i += 8;
    }
    for j in i..a.len() { c[j] = a[j] + b[j]; }
}
```

The dot product and matrix multiply follow the same patterns as the C++
AVX2 code, using `f32x8::reduce_sum()` (or `f32x4` for 4-wide
operations).

Rust's portable SIMD is **ISA-agnostic**: the same code compiles to
AVX2 on x86, NEON on AArch64, and (eventually) SVE on scalable
hardware.  The compiler selects the best available vector width.

---

## Use It

Production code rarely uses raw intrinsics directly.  Instead:

- **Highway** (`google/highway`): C++ library that provides a portable
  `HWY_*` API dispatching to SSE/AVX/NEON/SVE at runtime.
- **`std::experimental::simd`** (C++ parallel TS 2): proposed standard
  SIMD wrapper.  Available in GCC 11+.
- **`xsimd`** (QuantStack): standalone C++ SIMD library used by xtensor.
- **Rust `core_simd` crate**: nightly-only, but merges into std over time.
- **Intel IPP / MKL**: hand-tuned SIMD kernels for BLAS, FFT, image
  processing.
- **PyTorch / TensorFlow**: use SIMD (via oneDNN, XNNPACK) for
  convolution and matrix multiplication.

### Read the Source

- **glibc `memcpy`**: `sysdeps/x86_64/multiarch/memmove-vec-unaligned-erms.S`
  — hand-written AVX-512 copy loop.
- **Highway**: `hwy/highway.h` — the dispatch layer.
- **Linux kernel**: `arch/x86/include/asm/simd.h` — SIMD context-save
  routines.
- **PyTorch**: `aten/src/ATen/native/cpu/` — SIMD-optimised operators
  via `vec256` (AVX2) and `vec512` (AVX-512).

---

## Ship It

The reusable artifact from this lesson lives in `outputs/`:

- **`outputs/README.md`** — benchmark results and usage guide.
- **Compiled binaries** — `simd_bench` (C++) and (optionally)
  `simd_bench_rs` (Rust).

Add the vectorised dot-product and matrix-multiply helpers to your
personal toolkit — they will reappear in the capstone (work-stealing
scheduler with lock-free queue).

---

## Exercises

1. **Easy** — Re-implement `add_avx2` using `_mm256_loadu_ps` (unaligned
   load) and compare timing with aligned loads.  How much slower is it?

2. **Medium** — Extend the dot product to handle the **complex dot
   product** `Σ re(a[i])×re(b[i]) + im(a[i])×im(b[i])` using AVX2
   intrinsics.  Use `_mm256_permute_ps` to separate real and imaginary
   parts.

3. **Hard** — Implement a **6×6 matrix multiply** using AVX2.  6 is not
   a power of two — you will need to handle partial vectors and
   consider data-layout transposition.  Compare your performance against
   a naive scalar 6×6 multiply.  What speedup do you measure?

4. **Rust challenge** — Port your AVX2 C++ dot product to Rust portable
   SIMD and measure both with `criterion` benchmarks.  Is the
   performance identical?  If not, why?

---

## Key Terms

| Term                | What people say                                    | What it actually means                                         |
|---------------------|----------------------------------------------------|----------------------------------------------------------------|
| SIMD                | Single Instruction, Multiple Data                  | One CPU instruction operates on a vector of data in parallel.  |
| SSE                 | Streaming SIMD Extensions                          | 128-bit SIMD on x86; 4 × f32 or 2 × f64 per instruction.       |
| AVX                 | Advanced Vector Extensions                         | 256-bit SIMD on x86 (first generation: only FP, no integer).   |
| AVX2                | Advanced Vector Extensions 2                       | 256-bit SIMD with integer ops, gather, FMA3 (Haswell+).        |
| AVX-512             | AVX-512 Foundation                                 | 512-bit SIMD with masking, embedded rounding.                  |
| NEON                | ARM NEON                                           | 128-bit SIMD on all AArch64 CPUs; 4 × f32 / 2 × f64.           |
| SVE                 | Scalable Vector Extension                          | ARM vector ISA with compile-time-unknown width (128–2048 b).   |
| auto-vectorisation  | Compiler translates scalar loops to SIMD           | Loop-level analysis + cost model decides whether to vectorise. |
| intrinsic           | Compiler built-in mapping 1:1 to an ISA inst.      | `_mm256_add_ps` → `vaddps` instruction.                       |
| SIMD lane           | One element slot in a vector register              | Lane 0 holds element 0; all lanes execute the same op.         |
| vector width        | Total bits in one SIMD register                    | 128 (SSE), 256 (AVX2), 512 (AVX-512).                         |
| alignment           | Data address is a multiple of some power of two    | AVX2 aligned load/store requires 32-byte alignment.            |
| gather / scatter    | SIMD indexed load / store                          | `_mm256_i32gather_ps` loads 8 elements from non-contiguous addresses. |
| reduction           | Combining vector lanes into a single scalar        | Horizontal add (`_mm256_hadd_ps` → extract → scalar).          |

---

## Further Reading

- Intel Intrinsics Guide: https://www.intel.com/content/www/us/en/docs/intrinsics-guide/
- **Agner Fog's optimisation manuals** — instruction tables, microarchitecture
  guides.  Essential reference.
- **Highway library**: https://github.com/google/highway
- **Rust portable SIMD tracking issue**: https://github.com/rust-lang/rust/issues/86656
- **ARM NEON intrinsics reference**: https://developer.arm.com/architectures/instruction-sets/intrinsics/
- Patterson & Hennessy, *Computer Organization and Design* — Chapter on
  SIMD and vector processors.
