/*
 * SIMD & Vector ISAs — AVX, SVE, RVV
 * Phase 06 — Digital Logic & Computer Architecture
 *
 * Portable SIMD using GCC vector extensions.
 * Compiles on any GCC/Clang target with -O2.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define N  (1 << 20)   /* 1M elements */

/* ── GCC Vector Extension Types ─────────────────────────────────── */

typedef float  v8f  __attribute__((vector_size(32)));   /* 8 floats  = 256 bits */
typedef double v4d  __attribute__((vector_size(32)));   /* 4 doubles = 256 bits */

/* ── Scalar Dot Product ─────────────────────────────────────────── */

static float scalar_dot(const float *a, const float *b, int n) {
    float sum = 0.0f;
    for (int i = 0; i < n; i++)
        sum += a[i] * b[i];
    return sum;
}

/* ── Vectorized Dot Product (GCC vector extensions) ─────────────── */

static float vector_dot(const float *a, const float *b, int n) {
    v8f vsum = {0};
    int i;
    for (i = 0; i + 8 <= n; i += 8) {
        v8f va = *(const v8f *)&a[i];
        v8f vb = *(const v8f *)&b[i];
        vsum += va * vb;
    }
    /* horizontal reduction */
    float result[8] __attribute__((aligned(32)));
    *(v8f *)result = vsum;
    float sum = 0;
    for (int j = 0; j < 8; j++)
        sum += result[j];
    /* handle remainder */
    for (; i < n; i++)
        sum += a[i] * b[i];
    return sum;
}

/* ── Scalar Array Add ───────────────────────────────────────────── */

static void scalar_add(float *dst, const float *a, const float *b, int n) {
    for (int i = 0; i < n; i++)
        dst[i] = a[i] + b[i];
}

/* ── Vectorized Array Add ───────────────────────────────────────── */

static void vector_add(float *dst, const float *a, const float *b, int n) {
    int i;
    for (i = 0; i + 8 <= n; i += 8) {
        v8f va = *(const v8f *)&a[i];
        v8f vb = *(const v8f *)&b[i];
        *(v8f *)&dst[i] = va + vb;
    }
    for (; i < n; i++)
        dst[i] = a[i] + b[i];
}

/* ── Timing Helper ──────────────────────────────────────────────── */

static double now(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

/* ── Benchmark Harness ──────────────────────────────────────────── */

static void bench_dot_product(const float *a, const float *b, int n) {
    double t0, t1;
    volatile float result;

    /* scalar */
    t0 = now();
    result = scalar_dot(a, b, n);
    t1 = now();
    double scalar_time = t1 - t0;

    /* vectorized */
    t0 = now();
    result = vector_dot(a, b, n);
    t1 = now();
    double vector_time = t1 - t0;

    printf("Dot Product (%d elements):\n", n);
    printf("  Scalar:     %.4f ms  (result=%.2f)\n", scalar_time * 1000, result);
    printf("  Vectorized: %.4f ms  (result=%.2f)\n", vector_time * 1000, result);
    printf("  Speedup:    %.2fx\n\n", scalar_time / vector_time);
}

static void bench_array_add(const float *a, const float *b, int n) {
    float *dst_scalar = malloc(n * sizeof(float));
    float *dst_vector = malloc(n * sizeof(float));
    double t0, t1;

    /* scalar */
    t0 = now();
    scalar_add(dst_scalar, a, b, n);
    t1 = now();
    double scalar_time = t1 - t0;

    /* vectorized */
    t0 = now();
    vector_add(dst_vector, a, b, n);
    t1 = now();
    double vector_time = t1 - t0;

    /* verify correctness */
    int correct = 1;
    for (int i = 0; i < n; i++) {
        if (dst_scalar[i] != dst_vector[i]) { correct = 0; break; }
    }

    printf("Array Add (%d elements):\n", n);
    printf("  Scalar:     %.4f ms\n", scalar_time * 1000);
    printf("  Vectorized: %.4f ms\n", vector_time * 1000);
    printf("  Speedup:    %.2fx\n", scalar_time / vector_time);
    printf("  Correct:    %s\n\n", correct ? "YES" : "NO");

    free(dst_scalar);
    free(dst_vector);
}

/* ── Main ───────────────────────────────────────────────────────── */

int main(void) {
    printf("╔══════════════════════════════════════════════════════════╗\n");
    printf("║   SIMD Benchmark: GCC Vector Extensions (Portable)      ║\n");
    printf("╚══════════════════════════════════════════════════════════╝\n\n");

    /* allocate aligned data */
    float *a = aligned_alloc(32, N * sizeof(float));
    float *b = aligned_alloc(32, N * sizeof(float));

    srand(42);
    for (int i = 0; i < N; i++) {
        a[i] = (float)rand() / RAND_MAX;
        b[i] = (float)rand() / RAND_MAX;
    }

    bench_dot_product(a, b, N);
    bench_array_add(a, b, N);

    free(a);
    free(b);
    return 0;
}
