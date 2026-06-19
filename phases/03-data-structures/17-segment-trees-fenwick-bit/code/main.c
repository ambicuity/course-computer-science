/* main.c — Iterative segment tree, lazy segment tree, Fenwick BIT, bench. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <limits.h>

static double now(void) {
    struct timespec ts; clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

/* ============================================================ */
/* 1. Iterative segment tree (range-sum + point-update)         */
/* ============================================================ */
typedef struct { long *t; int n; } SegTree;

static void seg_init(SegTree *s, int n) {
    s->n = n; s->t = calloc(2 * n, sizeof(long));
}
static void seg_set(SegTree *s, int i, long x) {
    s->t[i + s->n] = x;
}
static void seg_build(SegTree *s) {
    for (int i = s->n - 1; i > 0; --i) s->t[i] = s->t[2*i] + s->t[2*i+1];
}
static void seg_update(SegTree *s, int i, long x) {
    for (s->t[i += s->n] = x; i >>= 1; ) s->t[i] = s->t[2*i] + s->t[2*i+1];
}
static long seg_query(SegTree *s, int l, int r) {        /* [l, r) */
    long res = 0;
    for (l += s->n, r += s->n; l < r; l >>= 1, r >>= 1) {
        if (l & 1) res += s->t[l++];
        if (r & 1) res += s->t[--r];
    }
    return res;
}

/* ============================================================ */
/* 2. Fenwick / BIT (range-sum + point-add)                    */
/* ============================================================ */
typedef struct { long *b; int n; } Fenwick;

static void bit_init(Fenwick *f, int n) {
    f->n = n; f->b = calloc(n + 1, sizeof(long));
}
static void bit_add(Fenwick *f, int i, long delta) {
    for (++i; i <= f->n; i += i & -i) f->b[i] += delta;
}
static long bit_prefix(Fenwick *f, int i) {
    long s = 0;
    for (; i > 0; i -= i & -i) s += f->b[i];
    return s;
}
static long bit_range(Fenwick *f, int l, int r) {        /* [l, r) */
    return bit_prefix(f, r) - bit_prefix(f, l);
}

/* ============================================================ */
/* 3. Lazy segment tree (range-add + range-sum)                */
/* ============================================================ */
typedef struct { long *t, *lazy; int n; } LazyTree;

static void lazy_init(LazyTree *L, int n) {
    L->n = n;
    L->t    = calloc(4 * n, sizeof(long));
    L->lazy = calloc(4 * n, sizeof(long));
}

static void lazy_push(LazyTree *L, int v, int vl, int vr) {
    if (L->lazy[v]) {
        int m = (vl + vr) / 2;
        L->t[2*v]   += L->lazy[v] * (m - vl + 1);
        L->lazy[2*v] += L->lazy[v];
        L->t[2*v+1] += L->lazy[v] * (vr - m);
        L->lazy[2*v+1] += L->lazy[v];
        L->lazy[v] = 0;
    }
}

static void lazy_update(LazyTree *L, int v, int vl, int vr, int l, int r, long x) {
    if (r < vl || vr < l) return;
    if (l <= vl && vr <= r) {
        L->t[v] += x * (vr - vl + 1);
        L->lazy[v] += x;
        return;
    }
    lazy_push(L, v, vl, vr);
    int m = (vl + vr) / 2;
    lazy_update(L, 2*v,   vl, m,   l, r, x);
    lazy_update(L, 2*v+1, m+1, vr, l, r, x);
    L->t[v] = L->t[2*v] + L->t[2*v+1];
}

static long lazy_query(LazyTree *L, int v, int vl, int vr, int l, int r) {
    if (r < vl || vr < l) return 0;
    if (l <= vl && vr <= r) return L->t[v];
    lazy_push(L, v, vl, vr);
    int m = (vl + vr) / 2;
    return lazy_query(L, 2*v, vl, m, l, r) + lazy_query(L, 2*v+1, m+1, vr, l, r);
}

/* ============================================================ */
/* Demo                                                          */
/* ============================================================ */
int main(void) {
    /* 1. Iterative segment tree */
    int A[] = {1, 3, 5, 7, 9, 11, 13, 15};
    int n = sizeof(A) / sizeof(A[0]);
    SegTree st; seg_init(&st, n);
    for (int i = 0; i < n; ++i) seg_set(&st, i, A[i]);
    seg_build(&st);
    printf("== Iterative segment tree ==\n");
    printf("  sum[0..8) = %ld (expect 64)\n", seg_query(&st, 0, n));
    printf("  sum[2..5) = %ld (expect 21)  (5+7+9)\n", seg_query(&st, 2, 5));
    seg_update(&st, 2, 100);
    printf("  after update(2, 100): sum[2..5) = %ld (expect 116)\n", seg_query(&st, 2, 5));
    free(st.t);

    /* 2. Fenwick */
    Fenwick f; bit_init(&f, n);
    for (int i = 0; i < n; ++i) bit_add(&f, i, A[i]);
    printf("\n== Fenwick / BIT ==\n");
    printf("  sum[0..8) = %ld (expect 64)\n", bit_range(&f, 0, n));
    printf("  sum[2..5) = %ld (expect 21)\n", bit_range(&f, 2, 5));
    bit_add(&f, 2, 95);  /* A[2] = 5 + 95 = 100 */
    printf("  after add(2, +95): sum[2..5) = %ld (expect 116)\n", bit_range(&f, 2, 5));
    free(f.b);

    /* 3. Lazy segment tree */
    LazyTree L; lazy_init(&L, n);
    for (int i = 0; i < n; ++i) lazy_update(&L, 1, 0, n-1, i, i, A[i]);
    printf("\n== Lazy segment tree (range-add, range-sum) ==\n");
    printf("  sum[0..7] = %ld (expect 64)\n", lazy_query(&L, 1, 0, n-1, 0, n-1));
    lazy_update(&L, 1, 0, n-1, 2, 5, 10);   /* add 10 to A[2..5] */
    printf("  after range-add +10 to [2..5]: sum[0..7] = %ld (expect 104)\n",
           lazy_query(&L, 1, 0, n-1, 0, n-1));
    printf("  sum[2..5] (inclusive): %ld  (expect 5+7+9+11 + 4×10 = 72)\n",
           lazy_query(&L, 1, 0, n-1, 2, 5));
    free(L.t); free(L.lazy);

    /* Bench: 1M ops on N=1M */
    const int N = 1000000;
    const int OPS = 1000000;
    int *arr = malloc(N * sizeof(int));
    for (int i = 0; i < N; ++i) arr[i] = rand() % 1000;

    SegTree big; seg_init(&big, N);
    for (int i = 0; i < N; ++i) seg_set(&big, i, arr[i]);
    seg_build(&big);
    double t = now();
    long sink = 0;
    for (int i = 0; i < OPS; ++i) {
        if (i & 1) seg_update(&big, rand() % N, rand() % 1000);
        else       sink += seg_query(&big, rand() % (N/2), N/2 + rand() % (N/2));
    }
    printf("\n== Bench (N=%d, %d ops) ==\n", N, OPS);
    printf("  Segment tree: %.3fs  (%.1f ns/op, sink=%ld)\n",
           now() - t, (now() - t) * 1e9 / OPS, sink);
    free(big.t);

    Fenwick bbf; bit_init(&bbf, N);
    for (int i = 0; i < N; ++i) bit_add(&bbf, i, arr[i]);
    t = now();
    sink = 0;
    for (int i = 0; i < OPS; ++i) {
        if (i & 1) bit_add(&bbf, rand() % N, rand() % 1000);
        else       sink += bit_range(&bbf, rand() % (N/2), N/2 + rand() % (N/2));
    }
    printf("  Fenwick BIT : %.3fs  (%.1f ns/op, sink=%ld)\n",
           now() - t, (now() - t) * 1e9 / OPS, sink);
    free(bbf.b);
    free(arr);
    return 0;
}
