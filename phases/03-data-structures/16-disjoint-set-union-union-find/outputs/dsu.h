/*
 * dsu.h — single-header Union-Find with union-by-rank + path halving.
 *   DSU d; dsu_init(&d, n);
 *   dsu_unite(&d, x, y);
 *   if (dsu_find(&d, x) == dsu_find(&d, y)) ...
 *   dsu_free(&d);
 */
#ifndef DSU_H
#define DSU_H

#include <stdlib.h>
#include <stdbool.h>

typedef struct { int *parent, *rank_; int n; } DSU;

static inline void dsu_init(DSU *d, int n) {
    d->n = n;
    d->parent = (int *)malloc(n * sizeof(int));
    d->rank_ = (int *)calloc(n, sizeof(int));
    for (int i = 0; i < n; ++i) d->parent[i] = i;
}

static inline int dsu_find(DSU *d, int x) {
    while (d->parent[x] != x) {
        d->parent[x] = d->parent[d->parent[x]];
        x = d->parent[x];
    }
    return x;
}

static inline bool dsu_unite(DSU *d, int x, int y) {
    int rx = dsu_find(d, x), ry = dsu_find(d, y);
    if (rx == ry) return false;
    if (d->rank_[rx] < d->rank_[ry]) d->parent[rx] = ry;
    else if (d->rank_[rx] > d->rank_[ry]) d->parent[ry] = rx;
    else { d->parent[ry] = rx; d->rank_[rx]++; }
    return true;
}

static inline bool dsu_connected(DSU *d, int x, int y) {
    return dsu_find(d, x) == dsu_find(d, y);
}

static inline void dsu_free(DSU *d) { free(d->parent); free(d->rank_); }

#endif /* DSU_H */
