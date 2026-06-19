/* main.c — Sparse table for range-minimum (RMQ).
 * O(n log n) build, O(1) query.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <assert.h>
#include <limits.h>

#define LOG 21                            /* enough for n up to 2^21 */

static int **sparse;
static int *log2_table;
static int n_arr;

static int imin(int a, int b) { return a < b ? a : b; }

static void rmq_build(const int *a, int n) {
    n_arr = n;
    log2_table = malloc((n + 1) * sizeof(int));
    log2_table[1] = 0;
    for (int i = 2; i <= n; ++i) log2_table[i] = log2_table[i / 2] + 1;

    int max_k = log2_table[n] + 1;
    sparse = malloc(max_k * sizeof(int *));
    for (int k = 0; k < max_k; ++k) sparse[k] = malloc(n * sizeof(int));

    for (int i = 0; i < n; ++i) sparse[0][i] = a[i];
    for (int k = 1; (1 << k) <= n; ++k) {
        for (int i = 0; i + (1 << k) - 1 < n; ++i) {
            sparse[k][i] = imin(sparse[k-1][i], sparse[k-1][i + (1 << (k-1))]);
        }
    }
}

static int rmq_query(int l, int r) {                  /* inclusive [l, r] */
    int k = log2_table[r - l + 1];
    return imin(sparse[k][l], sparse[k][r - (1 << k) + 1]);
}

static int naive_min(const int *a, int l, int r) {
    int m = INT_MAX;
    for (int i = l; i <= r; ++i) if (a[i] < m) m = a[i];
    return m;
}

static double now(void) {
    struct timespec ts; clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

int main(void) {
    /* Correctness test */
    const int N = 500;
    int *a = malloc(N * sizeof(int));
    srand(42);
    for (int i = 0; i < N; ++i) a[i] = rand() % 1000;
    rmq_build(a, N);

    int ok = 1;
    for (int t = 0; t < 1000; ++t) {
        int l = rand() % N;
        int r = l + rand() % (N - l);
        if (rmq_query(l, r) != naive_min(a, l, r)) { ok = 0; break; }
    }
    printf("== Sparse Table RMQ ==\n");
    printf("  N=%d, 1000 random queries vs naive: %s\n", N, ok ? "ALL MATCH" : "MISMATCH");

    /* Bench: free first build's tables */
    {
        int max_k = log2_table[N] + 1;
        for (int k = 0; k < max_k; ++k) free(sparse[k]);
        free(sparse); free(log2_table);
    }

    const int N_big = 1000000;
    a = realloc(a, N_big * sizeof(int));
    for (int i = 0; i < N_big; ++i) a[i] = rand();
    double t = now();
    rmq_build(a, N_big);
    double t_build = now() - t;

    t = now();
    long sink = 0;
    const int Q = 1000000;
    for (int q = 0; q < Q; ++q) {
        int l = rand() % N_big;
        int r = l + rand() % (N_big - l);
        sink += rmq_query(l, r);
    }
    double t_query = now() - t;

    printf("\n== Bench N=%d ==\n", N_big);
    printf("  build:  %.3fs  (%.0f Mb log2(N) ≈ %d levels)\n",
           t_build, t_build, log2_table[N_big]);
    printf("  %d queries: %.3fs  (%.1f ns/query, sink=%ld)\n",
           Q, t_query, t_query * 1e9 / Q, sink);

    free(a); free(log2_table);
    return 0;
}
