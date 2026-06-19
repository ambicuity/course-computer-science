/*
 * Loop Optimization & Vectorization
 * Phase 08 — Compilers & Programming Language Design
 *
 * C demos: auto-vectorization, manual AVX intrinsics, loop unrolling.
 * Compile with:
 *   gcc -O2 -mavx2 -fopt-info-vec main.c -o vec_demo
 *   gcc -O2 -mavx2 -Rpass=loop-vectorize main.c -o vec_demo  (Clang)
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

/* ------------------------------------------------------------------ */
/* 1. Vector addition — compiler should auto-vectorize this at -O2    */
/* ------------------------------------------------------------------ */

#define N 1024

void vector_add(float *a, const float *b, const float *c, int n) {
    for (int i = 0; i < n; i++) {
        a[i] = b[i] + c[i];
    }
}

/* ------------------------------------------------------------------ */
/* 2. Manual AVX2 intrinsics — 8 floats at a time                     */
/* ------------------------------------------------------------------ */

#ifdef __AVX2__
#include <immintrin.h>

void vector_add_avx(float *a, const float *b, const float *c, int n) {
    int i = 0;
    /* Process 8 floats (256 bits) per iteration */
    for (; i + 7 < n; i += 8) {
        __m256 vb = _mm256_loadu_ps(&b[i]);
        __m256 vc = _mm256_loadu_ps(&c[i]);
        __m256 va = _mm256_add_ps(vb, vc);
        _mm256_storeu_ps(&a[i], va);
    }
    /* Handle remainder */
    for (; i < n; i++) {
        a[i] = b[i] + c[i];
    }
}
#endif

/* ------------------------------------------------------------------ */
/* 3. Loop unrolling — 4-way                                          */
/* ------------------------------------------------------------------ */

void vector_add_unrolled(float *a, const float *b, const float *c, int n) {
    int i = 0;
    for (; i + 3 < n; i += 4) {
        a[i]     = b[i]     + c[i];
        a[i + 1] = b[i + 1] + c[i + 1];
        a[i + 2] = b[i + 2] + c[i + 2];
        a[i + 3] = b[i + 3] + c[i + 3];
    }
    for (; i < n; i++) {
        a[i] = b[i] + c[i];
    }
}

/* ------------------------------------------------------------------ */
/* 4. Reduction — sum of array (vectorizable with -ffast-math)        */
/* ------------------------------------------------------------------ */

float array_sum(const float *a, int n) {
    float sum = 0.0f;
    for (int i = 0; i < n; i++) {
        sum += a[i];
    }
    return sum;
}

/* ------------------------------------------------------------------ */
/* Benchmark helper                                                    */
/* ------------------------------------------------------------------ */

static double elapsed_ms(struct timespec *start, struct timespec *end) {
    return (end->tv_sec - start->tv_sec) * 1000.0 +
           (end->tv_nsec - start->tv_nsec) / 1e6;
}

void benchmark(const char *label, void (*fn)(float*, const float*, const float*, int),
               float *a, const float *b, const float *c, int n, int iterations) {
    struct timespec t0, t1;

    clock_gettime(CLOCK_MONOTONIC, &t0);
    for (int iter = 0; iter < iterations; iter++) {
        fn(a, b, c, n);
    }
    clock_gettime(CLOCK_MONOTONIC, &t1);

    double ms = elapsed_ms(&t0, &t1);
    printf("  %-25s %8.3f ms (%d iterations)\n", label, ms, iterations);
}

/* ------------------------------------------------------------------ */
/* Main                                                                */
/* ------------------------------------------------------------------ */

int main(void) {
    printf("Loop Optimization & Vectorization (C)\n");
    printf("======================================\n\n");

    /* Allocate aligned arrays */
    float *a = aligned_alloc(32, N * sizeof(float));
    float *b = aligned_alloc(32, N * sizeof(float));
    float *c = aligned_alloc(32, N * sizeof(float));

    /* Initialize */
    for (int i = 0; i < N; i++) {
        b[i] = (float)i;
        c[i] = (float)(i * 2);
    }

    /* --- Vector addition variants --- */
    printf("Vector addition (N=%d):\n", N);
    int iters = 100000;

    vector_add(a, b, c, N);
    printf("  Result check: a[0]=%.1f, a[N-1]=%.1f\n\n", a[0], a[N - 1]);

    benchmark("Scalar (auto-vec?)", vector_add, a, b, c, N, iters);
    benchmark("4-way unrolled", vector_add_unrolled, a, b, c, N, iters);

#ifdef __AVX2__
    benchmark("AVX2 intrinsics (8-wide)", vector_add_avx, a, b, c, N, iters);
#else
    printf("  AVX2 not available (compile with -mavx2)\n");
#endif

    printf("\n");

    /* --- Reduction --- */
    printf("Array reduction (sum of %d elements):\n", N);
    float sum = array_sum(b, N);
    printf("  Sum: %.1f (expected: %.1f)\n\n", sum, (float)(N - 1) * N / 2.0f);

    /* --- Cache behavior demo --- */
    printf("Cache locality: row-major vs column-major access\n");
    #define DIM 256
    static float matrix[DIM][DIM];
    for (int i = 0; i < DIM; i++)
        for (int j = 0; j < DIM; j++)
            matrix[i][j] = (float)(i * DIM + j);

    struct timespec t0, t1;
    float s = 0.0f;

    /* Row-major (stride-1, cache-friendly) */
    clock_gettime(CLOCK_MONOTONIC, &t0);
    for (int iter = 0; iter < 10000; iter++) {
        for (int i = 0; i < DIM; i++)
            for (int j = 0; j < DIM; j++)
                s += matrix[i][j];
    }
    clock_gettime(CLOCK_MONOTONIC, &t1);
    printf("  Row-major:    %.3f ms\n", elapsed_ms(&t0, &t1));

    /* Column-major (stride-DIM, cache-hostile) */
    s = 0.0f;
    clock_gettime(CLOCK_MONOTONIC, &t0);
    for (int iter = 0; iter < 10000; iter++) {
        for (int j = 0; j < DIM; j++)
            for (int i = 0; i < DIM; i++)
                s += matrix[i][j];
    }
    clock_gettime(CLOCK_MONOTONIC, &t1);
    printf("  Column-major: %.3f ms\n", elapsed_ms(&t0, &t1));

    printf("\n  (Row-major should be ~2-5x faster due to cache line utilization)\n");

    free(a);
    free(b);
    free(c);
    return 0;
}
