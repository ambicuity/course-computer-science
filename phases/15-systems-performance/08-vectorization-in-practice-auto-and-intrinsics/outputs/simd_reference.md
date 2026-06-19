# SIMD Quick Reference Card

## x86 SIMD Register Summary

| ISA       | Registers   | Width   | float32 lanes | float64 lanes | int32 lanes | Year |
|-----------|-------------|---------|---------------|---------------|-------------|------|
| SSE       | xmm0–xmm15  | 128-bit | 4             | 2             | 4           | 1999 |
| AVX       | ymm0–ymm15  | 256-bit | 8             | 4             | 8           | 2011 |
| AVX2      | ymm0–ymm15  | 256-bit | 8             | 4             | 8           | 2013 |
| AVX-512F  | zmm0–zmm31  | 512-bit | 16            | 8             | 16          | 2017 |

- AVX2 adds integer SIMD operations to AVX's floating-point vector ops.
- AVX-512 doubles the register count (16→32) and adds 8 mask registers (k0–k7).

## Common Intrinsics Cheat Sheet

### Load / Store

| Operation | SSE (128-bit) | AVX (256-bit) | Notes |
|-----------|---------------|---------------|-------|
| Unaligned load | `_mm_loadu_ps(p)` | `_mm256_loadu_ps(p)` | Safe for any alignment |
| Aligned load | `_mm_load_ps(p)` | `_mm256_load_ps(p)` | Must be 16/32-byte aligned |
| Unaligned store | `_mm_storeu_ps(p, v)` | `_mm256_storeu_ps(p, v)` | Safe for any alignment |
| Aligned store | `_mm_store_ps(p, v)` | `_mm256_store_ps(p, v)` | Must be 16/32-byte aligned |
| Set zero | `_mm_setzero_ps()` | `_mm256_setzero_ps()` | Initialize accumulator |
| Broadcast scalar | `_mm_set1_ps(x)` | `_mm256_set1_ps(x)` | Fill all lanes with one value |

### Arithmetic

| Operation | SSE | AVX | AVX-512 |
|-----------|-----|-----|---------|
| Add | `_mm_add_ps` | `_mm256_add_ps` | `_mm512_add_ps` |
| Multiply | `_mm_mul_ps` | `_mm256_mul_ps` | `_mm512_mul_ps` |
| FMA (a×b+c) | Use mul+add | `_mm256_fmadd_ps` | `_mm512_fmadd_ps` |
| Subtract | `_mm_sub_ps` | `_mm256_sub_ps` | `_mm512_sub_ps` |
| Divide | `_mm_div_ps` | `_mm256_div_ps` | `_mm512_div_ps` |

### Compare & Mask

| Operation | SSE | AVX | AVX-512 |
|-----------|-----|-----|---------|
| Compare GT | `_mm_cmpgt_ps(a,b)` | `_mm256_cmp_ps(a,b,_CMP_GT_OS)` | `_mm512_cmp_ps_mask(a,b,_CMP_GT_OS)` |
| Movemask | `_mm_movemask_ps(v)` | `_mm256_movemask_ps(v)` | Use mask registers directly |
| Masked add | N/A (blend) | N/A (blend) | `_mm512_mask_add_ps(src,k,a,b)` |

### Gather / Scatter

| Operation | AVX2 | AVX-512 |
|-----------|------|---------|
| Gather float32 | `_mm256_i32gather_ps(base, idx, 4)` | `_mm512_i32gather_ps(idx, base, 4)` |
| Scatter float32 | N/A | `_mm512_i32scatter_ps(base, idx, v, 4)` |

### Horizontal Reduction

| Method | SSE | AVX |
|--------|-----|-----|
| Extract + scalar sum | Store to array, sum elements | Store to array, sum elements |
| Shuffle-based | `_mm_hadd_ps` (2 steps) | `_mm256_hadd_ps` + finalize |
| AVX-512 | — | `_mm512_reduce_add_ps(v)` |

## Auto-Vectorization Checklist

Check these to determine if your loop will auto-vectorize:

- [ ] **No loop-carried dependencies** — iteration `i` does not depend on iteration `i-1`
- [ ] **Simple trip count** — the loop bound is known or at least a multiple of the vector width
- [ ] **No function calls** — except inlined / known functions (math intrinsics are OK)
- [ ] **No pointer aliasing** — use `__restrict__` / `restrict` or `#pragma ivdep`
- [ ] **Simple control flow** — `if` conditions can become masked ops, but avoid complex nesting
- [ ] **Access is contiguous** — `a[i]` not `a[index[i]]` (gather is harder to vectorize)
- [ ] **Compile with `-O3 -march=native`** — optimization and target ISA must be enabled

### Compiler Diagnostics

| Compiler | Flag to see vectorization decisions |
|----------|--------------------------------------|
| GCC | `-fopt-info-vec-optimized` (what vectorized) / `-fopt-info-vec-missed` (what didn't) |
| Clang | `-Rpass=loop-vectorize` / `-Rpass-missed=loop-vectorize` |
| MSVC | `/Qvec-report:2` |

## Alignment Rules

| Register Width | Required Alignment | Aligned Load | Unaligned Load |
|----------------|-------------------|--------------|----------------|
| 128-bit (SSE) | 16 bytes | `_mm_load_ps` | `_mm_loadu_ps` |
| 256-bit (AVX) | 32 bytes | `_mm256_load_ps` | `_mm256_loadu_ps` |
| 512-bit (AVX-512) | 64 bytes | `_mm512_load_ps` | `_mm512_loadu_ps` |

### Alignment Best Practices

1. **Allocate aligned**: `aligned_alloc(64, size)`, `_mm_malloc(size, 64)`, or `std::aligned_alloc(64, size)`.
2. **Use unaligned loads by default** — modern CPUs (Haswell+) have negligible penalty for `_mm256_loadu_ps`.
3. **Use aligned loads only when you control allocation** — stack arrays with `alignas(64)` or heap allocations with `aligned_alloc`.
4. **Padding** — pad arrays to a multiple of the vector width to avoid tail-loop overhead.

## Reduction Patterns

### SSE Horizontal Sum (4 floats)

```cpp
__m128 v = ...; // {a, b, c, d}
__m128 shuf = _mm_shuffle_ps(v, v, _MM_SHUFFLE(2,3,0,1)); // {b, a, d, c}
__m128 sums = _mm_add_ps(v, shuf);                          // {a+b, a+b, c+d, c+d}
shuf = _mm_movehl_ps(shuf, sums);                           // {c+d, c+d, c+d, c+d}
sums = _mm_add_ss(sums, shuf);                              // {a+b+c+d, ...}
float result = _mm_cvtss_f32(sums);
```

### AVX Horizontal Sum (8 floats)

```cpp
__m256 v = ...; // {a,b,c,d,e,f,g,h}
__m128 hi = _mm256_extractf128_ps(v, 1); // {e,f,g,h}
__m128 lo = _mm256_castps256_ps128(v);    // {a,b,c,d}
__m128 sum128 = _mm_add_ps(lo, hi);       // {a+e, b+f, c+g, d+h}
// Then SSE horizontal sum on sum128
```

### AVX-512 Horizontal Sum (16 floats)

```cpp
__m512 v = ...;
float result = _mm512_reduce_add_ps(v); // Single intrinsic!
```

## Rust SIMD Quick Reference

```rust
// Feature detection
if is_x86_feature_detected!("avx2") {
    unsafe { my_avx2_function(); }
}

// Function annotation
#[target_feature(enable = "avx2")]
unsafe fn my_avx2_function() { ... }

// Key intrinsics (same names as C, in std::arch::x86_64)
use std::arch::x86_64::*;
let v = _mm256_loadu_ps(ptr);  // AVX load
let r = _mm256_add_ps(a, b);  // AVX add
let z = _mm256_setzero_ps();  // AVX zero

// Safe SIMD (nightly): use std::simd::f32x8 for portable 8-lane float vectors
```