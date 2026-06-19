/*
 * segtree.h — single-header iterative segment tree + Fenwick BIT.
 *
 *   SegTree st; seg_init(&st, n);
 *   for (int i = 0; i < n; ++i) seg_set(&st, i, A[i]);
 *   seg_build(&st);
 *   seg_update(&st, i, x);
 *   long s = seg_query(&st, l, r);    // [l, r)
 *   seg_free(&st);
 *
 *   Fenwick f; bit_init(&f, n);
 *   bit_add(&f, i, delta);
 *   long s = bit_range(&f, l, r);
 *   bit_free(&f);
 */
#ifndef SEGTREE_H
#define SEGTREE_H

#include <stdlib.h>
#include <string.h>

typedef struct { long *t; int n; } SegTree;

static inline void seg_init(SegTree *s, int n)        { s->n = n; s->t = (long *)calloc(2 * n, sizeof(long)); }
static inline void seg_set(SegTree *s, int i, long x) { s->t[i + s->n] = x; }
static inline void seg_build(SegTree *s) {
    for (int i = s->n - 1; i > 0; --i) s->t[i] = s->t[2*i] + s->t[2*i+1];
}
static inline void seg_update(SegTree *s, int i, long x) {
    for (s->t[i += s->n] = x; i >>= 1; ) s->t[i] = s->t[2*i] + s->t[2*i+1];
}
static inline long seg_query(SegTree *s, int l, int r) {
    long res = 0;
    for (l += s->n, r += s->n; l < r; l >>= 1, r >>= 1) {
        if (l & 1) res += s->t[l++];
        if (r & 1) res += s->t[--r];
    }
    return res;
}
static inline void seg_free(SegTree *s) { free(s->t); s->t = NULL; }

typedef struct { long *b; int n; } Fenwick;

static inline void bit_init(Fenwick *f, int n)    { f->n = n; f->b = (long *)calloc(n + 1, sizeof(long)); }
static inline void bit_add(Fenwick *f, int i, long delta) {
    for (++i; i <= f->n; i += i & -i) f->b[i] += delta;
}
static inline long bit_prefix(Fenwick *f, int i) {
    long s = 0; for (; i > 0; i -= i & -i) s += f->b[i]; return s;
}
static inline long bit_range(Fenwick *f, int l, int r) { return bit_prefix(f, r) - bit_prefix(f, l); }
static inline void bit_free(Fenwick *f) { free(f->b); f->b = NULL; }

#endif /* SEGTREE_H */
