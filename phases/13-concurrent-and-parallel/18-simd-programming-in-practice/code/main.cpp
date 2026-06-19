// SIMD Programming in Practice
// Phase 13 — Concurrent & Parallel Computing, Lesson 18
//
// Compile: g++ -std=c++17 -mavx2 -O3 -fopenmp main.cpp -o simd_bench
// Run:     ./simd_bench

#include <immintrin.h>
#include <stdint.h>
#include <algorithm>
#include <chrono>
#include <cstring>
#include <iomanip>
#include <iostream>
#include <numeric>
#include <vector>

#ifndef __AVX2__
#error "This file requires AVX2. Compile with -mavx2."
#endif

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

using Clock = std::chrono::high_resolution_clock;
using us    = std::chrono::microseconds;

template <typename Func>
double time_it(Func &&fn, int trials = 5) {
    double best = 1e99;
    for (int t = 0; t < trials; ++t) {
        auto start = Clock::now();
        fn();
        auto end = Clock::now();
        double elapsed =
            std::chrono::duration_cast<us>(end - start).count();
        best = std::min(best, elapsed);
    }
    return best;
}

float rand_float() { return float(rand()) / float(RAND_MAX); }

// -----------------------------------------------------------------------
// SECTION 1 — Auto-vectorisation
// -----------------------------------------------------------------------

// Version A: no restrict, no alignment guarantees.
// The compiler *might* still auto-vectorise at -O3, but it must emit
// runtime alias checks.
// MARKED static to prevent inlining so the compiler cannot prove aliasing
// across translation-unit boundaries.
static void add_scalar(const float *a, const float *b, float *c,
                       size_t n) {
    for (size_t i = 0; i < n; ++i) c[i] = a[i] + b[i];
}

// Version B: __restrict__ + alignas(32) + #pragma omp simd.
// This is the strongest hint you can give the compiler.
static void add_autovec(const float *__restrict__ a,
                        const float *__restrict__ b,
                        float *__restrict__ c, size_t n) {
#pragma omp simd aligned(a, b, c : 32)
    for (size_t i = 0; i < n; ++i) c[i] = a[i] + b[i];
}

// Version C: scalar dot product — baseline for reduction.
static float dot_scalar(const float *a, const float *b, size_t n) {
    float sum = 0.0f;
    for (size_t i = 0; i < n; ++i) sum += a[i] * b[i];
    return sum;
}

// Version D: auto-vectorised dot product (reduction recognised by compiler).
static float dot_autovec(const float *__restrict__ a,
                         const float *__restrict__ b, size_t n) {
    float sum = 0.0f;
#pragma omp simd aligned(a, b : 32) reduction(+ : sum)
    for (size_t i = 0; i < n; ++i) sum += a[i] * b[i];
    return sum;
}

// -----------------------------------------------------------------------
// SECTION 2 — AVX2 Intrinsics
// -----------------------------------------------------------------------

// -- 2a. Element-wise addition ------------------------------------------

static void add_avx2(const float *__restrict__ a,
                     const float *__restrict__ b,
                     float *__restrict__ c, size_t n) {
    size_t i = 0;
    // Process 8 floats per iteration
    for (; i + 8 <= n; i += 8) {
        __m256 va = _mm256_load_ps(&a[i]);   // aligned 32-byte load
        __m256 vb = _mm256_load_ps(&b[i]);
        __m256 vc = _mm256_add_ps(va, vb);   // 8 x f32 add
        _mm256_store_ps(&c[i], vc);          // aligned store
    }
    // Tail (0-7 remaining elements)
    for (; i < n; ++i) c[i] = a[i] + b[i];
}

// -- 2b. Dot product via AVX2 -------------------------------------------

static float dot_avx2(const float *__restrict__ a,
                      const float *__restrict__ b, size_t n) {
    __m256 vsum = _mm256_setzero_ps();
    size_t i = 0;
    for (; i + 8 <= n; i += 8) {
        __m256 va = _mm256_load_ps(&a[i]);
        __m256 vb = _mm256_load_ps(&b[i]);
        vsum = _mm256_add_ps(vsum, _mm256_mul_ps(va, vb));
    }
    // Horizontal reduction: sum all 8 lanes into one scalar
    __m128 hi = _mm256_extractf128_ps(vsum, 1);  // upper 128 bits
    __m128 lo = _mm256_castps256_ps128(vsum);     // lower 128 bits
    __m128 sum128 = _mm_add_ps(lo, hi);           // 4-wide add
    sum128 = _mm_hadd_ps(sum128, sum128);         // pair-wise
    sum128 = _mm_hadd_ps(sum128, sum128);         // final pair
    float result = _mm_cvtss_f32(sum128);
    // Tail
    for (; i < n; ++i) result += a[i] * b[i];
    return result;
}

// -- 2c. 4x4 matrix multiply via SSE/AVX2 ------------------------------
//
// C[i][j] = sum_k A[i][k] * B[k][j]
//
// Strategy: load B as column vectors so we can broadcast A[i][k] and
// multiply-add into the result row.  We use __m128 (4-wide SSE) because
// the matrix is 4x4; AVX2 would be overkill here.

__attribute__((noinline)) static void
matmul_4x4_avx2(const float *__restrict__ A, const float *__restrict__ B,
                float *__restrict__ C) {
    // Treat B as column-major for this access pattern
    __m128 b_col0 = _mm_loadu_ps(&B[0]);
    __m128 b_col1 = _mm_loadu_ps(&B[4]);
    __m128 b_col2 = _mm_loadu_ps(&B[8]);
    __m128 b_col3 = _mm_loadu_ps(&B[12]);

    for (int i = 0; i < 4; ++i) {
        __m128 crow = _mm_setzero_ps();
        crow = _mm_add_ps(crow,
                          _mm_mul_ps(_mm_set1_ps(A[i * 4 + 0]), b_col0));
        crow = _mm_add_ps(crow,
                          _mm_mul_ps(_mm_set1_ps(A[i * 4 + 1]), b_col1));
        crow = _mm_add_ps(crow,
                          _mm_mul_ps(_mm_set1_ps(A[i * 4 + 2]), b_col2));
        crow = _mm_add_ps(crow,
                          _mm_mul_ps(_mm_set1_ps(A[i * 4 + 3]), b_col3));
        _mm_storeu_ps(&C[i * 4], crow);
    }
}

// Scalar 4x4 matmul for comparison
static void matmul_4x4_scalar(const float *A, const float *B, float *C) {
    for (int i = 0; i < 4; ++i) {
        for (int j = 0; j < 4; ++j) {
            float sum = 0.0f;
            for (int k = 0; k < 4; ++k) {
                sum += A[i * 4 + k] * B[k * 4 + j];
            }
            C[i * 4 + j] = sum;
        }
    }
}

// -- 2d. AVX2 gather scatter (indexed load) -----------------------------
//
// Sum of a[indices[i]] for all i.  The gather intrinsic performs 8
// independent loads from non-contiguous addresses in one instruction.

__attribute__((noinline)) static float
gather_sum_avx2(const float *__restrict__ arr,
                const int *__restrict__ indices, size_t n) {
    __m256 vsum = _mm256_setzero_ps();
    size_t i = 0;
    for (; i + 8 <= n; i += 8) {
        __m256i idx = _mm256_load_si256((const __m256i *)&indices[i]);
        __m256 vg  = _mm256_i32gather_ps(arr, idx, 4);  // scale factor 4
        vsum = _mm256_add_ps(vsum, vg);
    }
    // Horizontal reduction
    __m128 hi = _mm256_extractf128_ps(vsum, 1);
    __m128 lo = _mm256_castps256_ps128(vsum);
    __m128 sum128 = _mm_add_ps(lo, hi);
    sum128 = _mm_hadd_ps(sum128, sum128);
    sum128 = _mm_hadd_ps(sum128, sum128);
    float result = _mm_cvtss_f32(sum128);
    for (; i < n; ++i) result += arr[indices[i]];
    return result;
}

// Scalar equivalent of gather for comparison
static float gather_sum_scalar(const float *arr, const int *indices,
                               size_t n) {
    float sum = 0.0f;
    for (size_t i = 0; i < n; ++i) sum += arr[indices[i]];
    return sum;
}

// -- 2e. Strided copy — non-unit stride --------------------------------
//
// This loop cannot auto-vectorise efficiently because c[i * stride]
// has a non-unit stride.  AVX2 gather can help but is much slower
// than unit-stride loads.

static void add_strided_scalar(const float *__restrict__ a,
                               const float *__restrict__ b,
                               float *__restrict__ c,
                               size_t n, int stride) {
    for (size_t i = 0; i < n; ++i) c[i * stride] = a[i] + b[i];
}

static void add_strided_avx2(const float *__restrict__ a,
                             const float *__restrict__ b,
                             float *__restrict__ c,
                             size_t n, int stride) {
    // Build index vector: base, base+stride, base+2*stride, ...
    __m256i stride_v = _mm256_set1_epi32(stride);
    __m256i base_idx = _mm256_setr_epi32(0, 1, 2, 3, 4, 5, 6, 7);

    size_t i = 0;
    for (; i + 8 <= n; i += 8) {
        __m256i idx = _mm256_add_epi32(base_idx, _mm256_set1_epi32((int)i));
        idx = _mm256_mullo_epi32(idx, stride_v);  // element index * stride
        __m256 va = _mm256_i32gather_ps(a, idx, 4);
        __m256 vb = _mm256_i32gather_ps(b, idx, 4);
        __m256 vc = _mm256_add_ps(va, vb);
        // Scatter is expensive; for simplicity store unit-stride to a temp
        alignas(32) float tmp[8];
        _mm256_store_ps(tmp, vc);
        for (int k = 0; k < 8; ++k) c[(i + k) * stride] = tmp[k];
    }
    for (; i < n; ++i) c[i * stride] = a[i] + b[i];
}

// -----------------------------------------------------------------------
// SECTION 3 — Benchmarking
// -----------------------------------------------------------------------

static constexpr size_t N = 8UL << 20;  // 8 M elements (~32 MiB)

int main() {
    std::srand(42);

    // Allocate indices array for gather benchmark (32-byte aligned)
    int *indices;
    if (posix_memalign((void **)&indices, 32,
                       (N / 4) * sizeof(int)) != 0) {
        std::cerr << "posix_memalign for indices failed\n";
        return 1;
    }
    for (size_t i = 0; i < N / 4; ++i) indices[i] = (int)(rand() % N);

    // Allocate 32-byte aligned memory
    float *a, *b, *c, *d;
    if (posix_memalign((void **)&a, 32, N * sizeof(float)) != 0 ||
        posix_memalign((void **)&b, 32, N * sizeof(float)) != 0 ||
        posix_memalign((void **)&c, 32, N * sizeof(float)) != 0 ||
        posix_memalign((void **)&d, 32, N * sizeof(float)) != 0) {
        std::cerr << "posix_memalign failed\n";
        return 1;
    }

    for (size_t i = 0; i < N; ++i) a[i] = rand_float();
    for (size_t i = 0; i < N; ++i) b[i] = rand_float();
    std::memset(c, 0, N * sizeof(float));
    std::memset(d, 0, N * sizeof(float));

    // 4x4 matrix data
    float A[16], B[16], C_ref[16], C_simd[16];
    for (int i = 0; i < 16; ++i) A[i] = rand_float();
    for (int i = 0; i < 16; ++i) B[i] = rand_float();

    std::cout << std::fixed << std::setprecision(3);
    std::cout << "\n=== SIMD Benchmark  (N = " << N << " floats)\n\n";

    // ---- Element-wise Add ---------------------------------------------
    {
        double t_scalar = time_it([&] { add_scalar(a, b, c, N); });
        double t_autovec = time_it([&] { add_autovec(a, b, d, N); });
        double t_avx2 = time_it([&] { add_avx2(a, b, c, N); });

        // Verify correctness
        bool ok = true;
        for (size_t i = 0; i < N; ++i) {
            if (std::abs(c[i] - d[i]) > 1e-4f) { ok = false; break; }
        }

        std::cout << "--- Element-wise Add ---\n";
        std::cout << "  scalar   : " << t_scalar << " us\n";
        std::cout << "  autovec  : " << t_autovec << " us  ("
                  << (t_scalar / t_autovec) << "x)\n";
        std::cout << "  avx2     : " << t_avx2 << " us  ("
                  << (t_scalar / t_avx2) << "x)\n";
        std::cout << "  correct  : " << (ok ? "yes" : "FAIL") << "\n\n";
    }

    // ---- Dot Product --------------------------------------------------
    {
        double t_scalar = time_it([&] { dot_scalar(a, b, N); });
        double t_autovec = time_it([&] { dot_autovec(a, b, N); });
        double t_avx2 = time_it([&] { dot_avx2(a, b, N); });

        float r_scalar = dot_scalar(a, b, N);
        float r_avx2 = dot_avx2(a, b, N);
        float err = std::abs(r_scalar - r_avx2);

        std::cout << "--- Dot Product ---\n";
        std::cout << "  scalar   : " << t_scalar << " us  (result "
                  << r_scalar << ")\n";
        std::cout << "  autovec  : " << t_autovec << " us  ("
                  << (t_scalar / t_autovec) << "x)\n";
        std::cout << "  avx2     : " << t_avx2 << " us  ("
                  << (t_scalar / t_avx2) << "x)\n";
        std::cout << "  error    : " << err << "\n\n";
    }

    // ---- 4x4 Matrix Multiply ------------------------------------------
    {
        double t_scalar = time_it([&] { matmul_4x4_scalar(A, B, C_ref); });
        double t_avx2 = time_it([&] { matmul_4x4_avx2(A, B, C_simd); });

        bool ok = true;
        for (int i = 0; i < 16; ++i) {
            if (std::abs(C_ref[i] - C_simd[i]) > 1e-4f) {
                ok = false;
                break;
            }
        }

        std::cout << "--- 4x4 Matrix Multiply ---\n";
        std::cout << "  scalar   : " << t_scalar << " us\n";
        std::cout << "  avx2     : " << t_avx2 << " us  ("
                  << (t_scalar / t_avx2) << "x)\n";
        std::cout << "  correct  : " << (ok ? "yes" : "FAIL") << "\n\n";
    }

    // ---- Gather (indexed sum) -----------------------------------------
    {
        size_t gather_n = N / 4;
        double t_scalar = time_it(
            [&] { gather_sum_scalar(a, indices, gather_n); }, 5);
        double t_avx2 = time_it(
            [&] { gather_sum_avx2(a, indices, gather_n); }, 5);

        float r_scalar = gather_sum_scalar(a, indices, gather_n);
        float r_avx2   = gather_sum_avx2(a, indices, gather_n);
        bool ok = std::abs(r_scalar - r_avx2) < 1.0f;  // allow FP diff

        std::cout << "--- Gather (indexed sum) ---\n";
        std::cout << "  scalar   : " << t_scalar << " us  (result "
                  << r_scalar << ")\n";
        std::cout << "  avx2     : " << t_avx2 << " us  ("
                  << (t_scalar / t_avx2) << "x)\n";
        std::cout << "  correct  : " << (ok ? "yes" : "FAIL") << "\n";
        std::cout << "  (gather is memory-latency-bound; speedup reflects\n"
                     "   instruction-pipeline efficiency, not 8x)\n\n";
    }

    // ---- Strided add (stride = 4) ------------------------------------
    {
        size_t stride_n = N / 16;
        int stride = 4;
        // Use a fresh aligned buffer for strided output
        float *e;
        if (posix_memalign((void **)&e, 32,
                           stride_n * stride * sizeof(float)) != 0) {
            std::cerr << "posix_memalign for e failed\n";
            return 1;
        }
        std::memset(e, 0, stride_n * stride * sizeof(float));

        double t_scalar = time_it(
            [&] { add_strided_scalar(a, b, e, stride_n, stride); }, 5);
        double t_avx2 = time_it(
            [&] { add_strided_avx2(a, b, e, stride_n, stride); }, 5);

        bool ok = true;
        for (size_t i = 0; i < stride_n; ++i) {
            if (std::abs(e[i * stride] - (a[i] + b[i])) > 1e-4f) {
                ok = false; break;
            }
        }

        std::cout << "--- Strided Add (stride=4) ---\n";
        std::cout << "  scalar   : " << t_scalar << " us\n";
        std::cout << "  avx2     : " << t_avx2 << " us  ("
                  << (t_scalar / t_avx2) << "x)\n";
        std::cout << "  correct  : " << (ok ? "yes" : "FAIL") << "\n";
        std::cout << "  (gather+scatter overhead limits AVX2 gain;\n"
                     "   stride>1 kills memory-level parallelism)\n\n";

        free(e);
    }

    // ---- Summary ------------------------------------------------------
    std::cout << "--- Summary ---\n";
    std::cout << "AVX2 theoretical peak speedup for f32 add: 8x\n";
    std::cout << "Measured speedups reflect memory bandwidth\n";
    std::cout << "and horizontal reduction overhead.\n";
    std::cout << "Gather/strided patterns see much less gain due to\n";
    std::cout << "memory latency and scatter overhead.\n\n";

    free(a);
    free(b);
    free(c);
    free(d);
    free(indices);
    return 0;
}
