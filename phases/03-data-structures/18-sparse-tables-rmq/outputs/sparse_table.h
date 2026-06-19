/*
 * sparse_table.h — single-header sparse table for range-min query (RMQ).
 *
 *   SparseTable st;
 *   sparse_build(&st, a, n);
 *   int m = sparse_min(&st, l, r);     // inclusive [l, r]
 *   sparse_free(&st);
 */
#ifndef SPARSE_TABLE_H
#define SPARSE_TABLE_H

#include <stdlib.h>
#include <limits.h>

typedef struct {
    int **t;            /* t[k][i] = min over A[i..i+2^k-1] */
    int  *log2;
    int   n;
    int   max_k;
} SparseTable;

static inline int sparse__min(int a, int b) { return a < b ? a : b; }

static inline void sparse_build(SparseTable *s, const int *a, int n) {
    s->n = n;
    s->log2 = (int *)calloc(n + 1, sizeof(int));
    for (int i = 2; i <= n; ++i) s->log2[i] = s->log2[i / 2] + 1;
    s->max_k = s->log2[n] + 1;
    s->t = (int **)malloc(s->max_k * sizeof(int *));
    for (int k = 0; k < s->max_k; ++k) s->t[k] = (int *)malloc(n * sizeof(int));
    for (int i = 0; i < n; ++i) s->t[0][i] = a[i];
    for (int k = 1; (1 << k) <= n; ++k)
        for (int i = 0; i + (1 << k) - 1 < n; ++i)
            s->t[k][i] = sparse__min(s->t[k-1][i], s->t[k-1][i + (1 << (k-1))]);
}

static inline int sparse_min(const SparseTable *s, int l, int r) {
    int k = s->log2[r - l + 1];
    return sparse__min(s->t[k][l], s->t[k][r - (1 << k) + 1]);
}

static inline void sparse_free(SparseTable *s) {
    for (int k = 0; k < s->max_k; ++k) free(s->t[k]);
    free(s->t); free(s->log2);
    s->t = NULL; s->log2 = NULL;
}

#endif /* SPARSE_TABLE_H */
