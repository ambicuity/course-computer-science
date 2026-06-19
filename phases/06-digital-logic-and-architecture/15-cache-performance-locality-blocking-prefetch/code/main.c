/*
 * Cache Performance — Locality, Blocking, Prefetch
 * Phase 06 — Digital Logic & Computer Architecture
 *
 * Matrix multiply benchmarks: naive, transposed, and blocked.
 * Compile: gcc -O2 -o matmul main.c
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define N 512

static double A[N][N], B[N][N], C[N][N], BT[N][N];

static void fill(double m[N][N]) {
    for (int i = 0; i < N; i++)
        for (int j = 0; j < N; j++)
            m[i][j] = (double)rand() / RAND_MAX;
}

/* Naive triple-nested loop. B is accessed column-major (stride N * 8 bytes). */
static void matmul_naive(double A[N][N], double B[N][N], double C[N][N]) {
    for (int i = 0; i < N; i++)
        for (int j = 0; j < N; j++)
            for (int k = 0; k < N; k++)
                C[i][j] += A[i][k] * B[k][j];
}

/* Transpose B first, then all three accesses are row-major. */
static void transpose(double src[N][N], double dst[N][N]) {
    for (int i = 0; i < N; i++)
        for (int j = 0; j < N; j++)
            dst[i][j] = src[j][i];
}

static void matmul_transposed(double A[N][N], double BT[N][N], double C[N][N]) {
    for (int i = 0; i < N; i++)
        for (int j = 0; j < N; j++)
            for (int k = 0; k < N; k++)
                C[i][j] += A[i][k] * BT[j][k];
}

/* Blocked matrix multiply. block_size controls the working set. */
static void matmul_blocked(double A[N][N], double B[N][N], double C[N][N], int bs) {
    for (int ii = 0; ii < N; ii += bs) {
        int imax = ii + bs < N ? ii + bs : N;
        for (int jj = 0; jj < N; jj += bs) {
            int jmax = jj + bs < N ? jj + bs : N;
            for (int kk = 0; kk < N; kk += bs) {
                int kmax = kk + bs < N ? kk + bs : N;
                for (int i = ii; i < imax; i++)
                    for (int j = jj; j < jmax; j++) {
                        double sum = C[i][j];
                        for (int k = kk; k < kmax; k++)
                            sum += A[i][k] * B[k][j];
                        C[i][j] = sum;
                    }
            }
        }
    }
}

static double elapsed_ms(struct timespec *start, struct timespec *end) {
    return (end->tv_sec - start->tv_sec) * 1000.0
         + (end->tv_nsec - start->tv_nsec) / 1e6;
}

int main(void) {
    struct timespec t0, t1;

    srand(42);
    fill(A);
    fill(B);

    /* --- Naive --- */
    memset(C, 0, sizeof(C));
    clock_gettime(CLOCK_MONOTONIC, &t0);
    matmul_naive(A, B, C);
    clock_gettime(CLOCK_MONOTONIC, &t1);
    printf("Naive:       %8.2f ms\n", elapsed_ms(&t0, &t1));

    /* --- Transposed --- */
    transpose(B, BT);
    memset(C, 0, sizeof(C));
    clock_gettime(CLOCK_MONOTONIC, &t0);
    matmul_transposed(A, BT, C);
    clock_gettime(CLOCK_MONOTONIC, &t1);
    printf("Transposed:  %8.2f ms\n", elapsed_ms(&t0, &t1));

    /* --- Blocked (various block sizes) --- */
    int block_sizes[] = {8, 16, 32, 64, 128};
    for (int b = 0; b < (int)(sizeof(block_sizes)/sizeof(block_sizes[0])); b++) {
        int bs = block_sizes[b];
        memset(C, 0, sizeof(C));
        clock_gettime(CLOCK_MONOTONIC, &t0);
        matmul_blocked(A, B, C, bs);
        clock_gettime(CLOCK_MONOTONIC, &t1);
        printf("Blocked %3d:  %8.2f ms\n", bs, elapsed_ms(&t0, &t1));
    }

    return 0;
}
