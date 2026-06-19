# SIMD & Vector ISAs — AVX, SVE, RVV

> A single instruction that operates on 8 floats simultaneously is worth 8 instructions that operate on one. SIMD turns data parallelism into throughput.

**Type:** Learn | **Languages:** C, C++ | **Prerequisites:** Phase 06 lessons 01–17 | **Time:** ~75 minutes

## Learning Objectives

- Explain SIMD and how it differs from scalar execution.
- Use AVX2 intrinsics to operate on 256-bit vectors of floats and doubles.
- Describe SVE and RVV as scalable alternatives to fixed-width SIMD.
- Benchmark vectorized vs scalar code and measure the speedup.

## The Problem

This lesson sits in **Phase 06 — Digital Logic & Computer Architecture**. Modern workloads — image processing, ML inference, physics simulation — operate on large arrays where the same operation applies to every element. Executing these element-by-element leaves the CPU's vector units idle.

## The Concept

### SIMD — Single Instruction, Multiple Data

A scalar add adds one pair of numbers per instruction. A SIMD add adds N pairs in parallel using a wide register:

```
Scalar:  ADD  r1, r2        → r1 = r1 + r2           (1 result)
SIMD:    VADD ymm0, ymm1    → ymm0[0..7] += ymm1[0..7]  (8 results)
```

```
  Scalar ADD               SIMD ADD (8-wide)
  ┌─────┐ ┌─────┐         ┌─────────────────────────┐
  │ 3.0 │+│ 1.0 │         │ 3.0 2.5 1.0 4.0 0.5 ... │
  └─────┘ └─────┘         └─────────────────────────┘
    = 4.0                   + ┌─────────────────────────┐
                              │ 1.0 0.5 2.0 1.0 3.0 ... │
                              └─────────────────────────┘
                                = ┌─────────────────────────┐
                                  │ 4.0 3.0 3.0 5.0 3.5 ... │
                                  └─────────────────────────┘
```

### x86 SIMD: SSE → AVX → AVX-512

| Extension | Year | Register Width | Registers | Typical Use |
|-----------|------|---------------|-----------|-------------|
| SSE | 1999 | 128-bit (XMM) | 16 | 4 floats or 2 doubles |
| AVX | 2011 | 256-bit (YMM) | 16 | 8 floats or 4 doubles |
| AVX2 | 2013 | 256-bit (YMM) | 16 | Integer + float SIMD |
| AVX-512 | 2016 | 512-bit (ZMM) | 32 | 16 floats or 8 doubles |

**Intrinsics** are C/C++ functions that map 1:1 to SIMD instructions:

```c
#include <immintrin.h>

__m256 a = _mm256_set_ps(7,6,5,4,3,2,1,0);   // load 8 floats
__m256 b = _mm256_set_ps(7,6,5,4,3,2,1,0);
__m256 c = _mm256_add_ps(a, b);                // c[i] = a[i] + b[i]
```

Key intrinsics: `_mm256_add_ps` (add packed single), `_mm256_mul_pd` (multiply packed double), `_mm256_loadu_ps` (unaligned load), `_mm256_storeu_ps` (unaligned store), `_mm256_hadd_ps` (horizontal add).

### ARM SVE — Scalable Vector Extension

SVE breaks from fixed-width SIMD. The vector length (VL) is determined **at runtime** by the hardware — it can be 128, 256, 512, or up to 2048 bits. Code is written to be **vector-length agnostic (VLA)**.

```c
// SVE: vector length is unknown at compile time
svfloat32_t a = svld1_f32(pg, ptr_a);   // load as many floats as VL allows
svfloat32_t b = svld1_f32(pg, ptr_b);
svfloat32_t c = svadd_f32_m(pg, a, b);  // add all of them
svst1_f32(pg, ptr_c, c);
```

The same binary runs optimally on any SVE implementation. Predication masks (`pg`) handle remainders when the array length is not a multiple of VL.

### RISC-V RVV — Vector Extension

RVV is even more flexible. The vector length register (`vl`) and configuration (`vsetvli`) let software control the effective vector width per instruction:

```asm
vsetvli t0, a0, e32, m1, ta, ma    # set VL based on remaining elements
vle32.v v1, (a1)                    # load VL floats
vle32.v v2, (a2)
vfadd.vv v3, v1, v2                 # add VL floats
vse32.v v3, (a3)                    # store VL floats
```

RVV supports masking, segment load/stores, and indexed (scatter/gather) access natively.

### Alignment

SIMD loads are fastest when data is **aligned** to the vector width (16 bytes for SSE, 32 for AVX, 64 for AVX-512). Unaligned loads work but may cost an extra cycle. Use `_mm256_load_ps` (aligned) vs `_mm256_loadu_ps` (unaligned).

```c
float *data = aligned_alloc(32, N * sizeof(float));  // 32-byte aligned
__m256 v = _mm256_load_ps(data);  // fast aligned load
```

## Build It

### Step 1: Portable SIMD with GCC Vector Extensions (C)

`code/main.c` uses GCC's `__attribute__((vector_size(32)))` for portable 256-bit vector operations that work on any GCC-supported target:

```c
typedef float v8f __attribute__((vector_size(32)));

v8f vec_add(v8f a, v8f b) { return a + b; }

float vec_dot_product(const float *a, const float *b, int n) {
    v8f sum = {0};
    for (int i = 0; i < n; i += 8) {
        v8f va = *(const v8f *)&a[i];
        v8f vb = *(const v8f *)&b[i];
        sum += va * vb;
    }
    // horizontal reduction
    float result[8] __attribute__((aligned(32)));
    *(v8f *)result = sum;
    float total = 0;
    for (int j = 0; j < 8; j++) total += result[j];
    return total;
}
```

### Step 2: AVX2 Intrinsics (C++, x86 only)

`code/main.cpp` uses explicit AVX2 intrinsics for maximum control:

```cpp
float avx2_dot_product(const float *a, const float *b, int n) {
    __m256 sum = _mm256_setzero_ps();
    for (int i = 0; i < n; i += 8) {
        __m256 va = _mm256_loadu_ps(&a[i]);
        __m256 vb = _mm256_loadu_ps(&b[i]);
        sum = _mm256_add_ps(sum, _mm256_mul_ps(va, vb));
    }
    // horizontal reduction
    __m128 hi = _mm256_extractf128_ps(sum, 1);
    __m128 lo = _mm256_castps256_ps128(sum);
    __m128 s = _mm_add_ps(lo, hi);
    s = _mm_hadd_ps(s, s);
    s = _mm_hadd_ps(s, s);
    return _mm_cvtss_f32(s);
}
```

Both files benchmark vectorized vs scalar dot product and array addition, reporting speedup.

## Use It

SIMD is everywhere in production:

- **BLAS/LAPACK**: Intel MKL, OpenBLAS use AVX-512 for matrix multiply.
- **Image processing**: libjpeg-turbo uses SSE/NEON for JPEG encode/decode. Pillow uses AVX2 for resize.
- **Video encoding**: x264/x265 use AVX2 for motion estimation and DCT.
- **Machine learning**: ONNX Runtime uses AVX-512 VNNI for INT8 inference.
- **String operations**: glibc `strlen` and `memcpy` use SSE/AVX at runtime.

The compiler also **auto-vectorizes** simple loops when given `-O2 -march=native`. Writing explicit intrinsics is necessary when the compiler cannot prove safety (aliasing, dependency) or when you need a specific instruction sequence.

## Read the Source

- glibc `sysdeps/x86_64/multiarch/strlen-avx2.S` — hand-tuned AVX2 strlen.
- OpenBLAS `kernel/x86_64/dgemm_kernel_4x8_haswell.S` — AVX2 matrix kernel.
- Intel Intrinsics Guide: https://www.intel.com/content/www/us/en/docs/intrinsics-guide/

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`simd_toolkit` — scalar vs vectorized dot product and array add with benchmark harness.**

## Exercises

1. **Easy** — Write a scalar dot product and a GCC vector-extension dot product. Verify they produce the same result on 1024 random floats. Print the speedup.

2. **Medium** — Implement `avx2_matrix_add` that adds two 256×256 float matrices using AVX2 intrinsics. Compare wall-clock time against a scalar triple-nested loop. What is the speedup?

3. **Hard** — Implement a vectorized histogram: given an array of `uint8_t` values (0–255), count occurrences of each value. Use AVX2 `_mm256_cmpeq_epi8` to compare 32 bytes at once and accumulate into bins. Compare performance against a scalar histogram.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| SIMD | "vector instructions" | Single instruction operates on N data elements in parallel using wide registers. |
| Intrinsic | "built-in function" | A C/C++ function mapping directly to a specific SIMD instruction. |
| SSE | "128-bit SIMD" | x86 SIMD extension: 128-bit XMM registers, 4 floats or 2 doubles per instruction. |
| AVX / AVX2 | "256-bit SIMD" | x86 extension: 256-bit YMM registers, 8 floats or 4 doubles. AVX2 adds integer ops. |
| AVX-512 | "512-bit SIMD" | x86 extension: 512-bit ZMM registers, 16 floats, 32 registers, masking. |
| SVE | "ARM scalable vectors" | ARM SIMD where vector length is hardware-determined at runtime (128–2048 bits). |
| RVV | "RISC-V vector" | RISC-V vector extension with configurable `vl` (vector length) per instruction. |
| VLA | "vector-length agnostic" | Code that works correctly regardless of the hardware's actual SIMD width. |
| Horizontal add | "hadd" | Summing elements within a single vector register (vs. vertical: element-wise). |
| Alignment | "32-byte aligned" | Data address is a multiple of the vector width; aligned loads are faster. |

## Further Reading

- *Computer Architecture: A Quantitative Approach* (Hennessy & Patterson), Ch. 4 — Data-Level Parallelism.
- Intel Intrinsics Guide — searchable reference for every SSE/AVX/AVX-512 intrinsic.
- ARM SVE Programmer's Guide — https://developer.arm.com/documentation/102476/
- RISC-V Vector Extension Specification — https://github.com/riscv/riscv-v-spec
