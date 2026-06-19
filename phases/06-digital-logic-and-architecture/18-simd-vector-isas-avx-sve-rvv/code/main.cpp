/*
 * SIMD & Vector ISAs — AVX, SVE, RVV
 * Phase 06 — Digital Logic & Computer Architecture
 *
 * AVX2 intrinsics (x86 only).
 * Compile: g++ -mavx2 -O2 -o simd_avx main.cpp
 */
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <ctime>
#include <immintrin.h>

#define N  (1 << 20)   /* 1M elements */

/* ── Scalar Dot Product ─────────────────────────────────────────── */

static float scalar_dot(const float *a, const float *b, int n) {
    float sum = 0.0f;
    for (int i = 0; i < n; i++)
        sum += a[i] * b[i];
    return sum;
}

/* ── AVX2 Dot Product ───────────────────────────────────────────── */

static float avx2_dot_product(const float *a, const float *b, int n) {
    __m256 vsum = _mm256_setzero_ps();
    int i;
    for (i = 0; i + 8 <= n; i += 8) {
        __m256 va = _mm256_loadu_ps(&a[i]);
        __m256 vb = _mm256_loadu_ps(&b[i]);
        vsum = _mm256_add_ps(vsum, _mm256_mul_ps(va, vb));
    }
    /* horizontal reduction: 256 → 128 → scalar */
    __m128 hi = _mm256_extractf128_ps(vsum, 1);
    __m128 lo = _mm256_castps256_ps128(vsum);
    __m128 s  = _mm_add_ps(lo, hi);          /* 4 floats */
    s = _mm_hadd_ps(s, s);                    /* 2 floats */
    s = _mm_hadd_ps(s, s);                    /* 1 float  */
    float sum = _mm_cvtss_f32(s);
    /* remainder */
    for (; i < n; i++)
        sum += a[i] * b[i];
    return sum;
}

/* ── Scalar Matrix Add ──────────────────────────────────────────── */

static void scalar_matrix_add(float *c, const float *a, const float *b,
                               int rows, int cols) {
    int total = rows * cols;
    for (int i = 0; i < total; i++)
        c[i] = a[i] + b[i];
}

/* ── AVX2 Matrix Add ────────────────────────────────────────────── */

static void avx2_matrix_add(float *c, const float *a, const float *b,
                             int rows, int cols) {
    int total = rows * cols;
    int i;
    for (i = 0; i + 8 <= total; i += 8) {
        __m256 va = _mm256_loadu_ps(&a[i]);
        __m256 vb = _mm256_loadu_ps(&b[i]);
        _mm256_storeu_ps(&c[i], _mm256_add_ps(va, vb));
    }
    for (; i < total; i++)
        c[i] = a[i] + b[i];
}

/* ── Scalar Array Add ───────────────────────────────────────────── */

static void scalar_add(float *dst, const float *a, const float *b, int n) {
    for (int i = 0; i < n; i++)
        dst[i] = a[i] + b[i];
}

/* ── AVX2 Array Add ─────────────────────────────────────────────── */

static void avx2_add(float *dst, const float *a, const float *b, int n) {
    int i;
    for (i = 0; i + 8 <= n; i += 8) {
        __m256 va = _mm256_loadu_ps(&a[i]);
        __m256 vb = _mm256_loadu_ps(&b[i]);
        _mm256_storeu_ps(&dst[i], _mm256_add_ps(va, vb));
    }
    for (; i < n; i++)
        dst[i] = a[i] + b[i];
}

/* ── Timing Helper ──────────────────────────────────────────────── */

static double now() {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

/* ── Main ───────────────────────────────────────────────────────── */

int main() {
    printf("╔══════════════════════════════════════════════════════════╗\n");
    printf("║   SIMD Benchmark: AVX2 Intrinsics (x86)                 ║\n");
    printf("╚══════════════════════════════════════════════════════════╝\n\n");

    float *a = (float *)_mm_malloc(N * sizeof(float), 32);
    float *b = (float *)_mm_malloc(N * sizeof(float), 32);
    float *c = (float *)_mm_malloc(N * sizeof(float), 32);

    srand(42);
    for (int i = 0; i < N; i++) {
        a[i] = (float)rand() / RAND_MAX;
        b[i] = (float)rand() / RAND_MAX;
    }

    volatile float result;
    double t0, t1;

    /* ── Dot Product ─────────────────────────────────────────── */

    t0 = now();
    result = scalar_dot(a, b, N);
    t1 = now();
    double dot_scalar = t1 - t0;

    t0 = now();
    result = avx2_dot_product(a, b, N);
    t1 = now();
    double dot_avx = t1 - t0;

    printf("Dot Product (%d elements):\n", N);
    printf("  Scalar:     %.4f ms\n", dot_scalar * 1000);
    printf("  AVX2:       %.4f ms\n", dot_avx * 1000);
    printf("  Speedup:    %.2fx\n\n", dot_scalar / dot_avx);

    /* ── Array Add ───────────────────────────────────────────── */

    t0 = now();
    scalar_add(c, a, b, N);
    t1 = now();
    double add_scalar = t1 - t0;

    t0 = now();
    avx2_add(c, a, b, N);
    t1 = now();
    double add_avx = t1 - t0;

    printf("Array Add (%d elements):\n", N);
    printf("  Scalar:     %.4f ms\n", add_scalar * 1000);
    printf("  AVX2:       %.4f ms\n", add_avx * 1000);
    printf("  Speedup:    %.2fx\n\n", add_scalar / add_avx);

    /* ── Matrix Add ──────────────────────────────────────────── */

    const int rows = 256, cols = 256;
    int total = rows * cols;
    float *ma = (float *)_mm_malloc(total * sizeof(float), 32);
    float *mb = (float *)_mm_malloc(total * sizeof(float), 32);
    float *mc = (float *)_mm_malloc(total * sizeof(float), 32);

    for (int i = 0; i < total; i++) {
        ma[i] = (float)rand() / RAND_MAX;
        mb[i] = (float)rand() / RAND_MAX;
    }

    t0 = now();
    scalar_matrix_add(mc, ma, mb, rows, cols);
    t1 = now();
    double mat_scalar = t1 - t0;

    t0 = now();
    avx2_matrix_add(mc, ma, mb, rows, cols);
    t1 = now();
    double mat_avx = t1 - t0;

    printf("Matrix Add (%dx%d):\n", rows, cols);
    printf("  Scalar:     %.4f ms\n", mat_scalar * 1000);
    printf("  AVX2:       %.4f ms\n", mat_avx * 1000);
    printf("  Speedup:    %.2fx\n", mat_scalar / mat_avx);

    _mm_free(ma);
    _mm_free(mb);
    _mm_free(mc);
    _mm_free(a);
    _mm_free(b);
    _mm_free(c);
    return 0;
}
