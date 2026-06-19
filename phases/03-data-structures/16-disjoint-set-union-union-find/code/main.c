/* main.c — DSU with union-by-rank + path compression + Kruskal MST + bench. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <assert.h>

typedef struct { int *parent, *rank_; int n; } DSU;

static void dsu_init(DSU *d, int n) {
    d->n = n;
    d->parent = malloc(n * sizeof(int));
    d->rank_ = calloc(n, sizeof(int));
    for (int i = 0; i < n; ++i) d->parent[i] = i;
}

static int dsu_find(DSU *d, int x) {
    while (d->parent[x] != x) {
        d->parent[x] = d->parent[d->parent[x]];   /* path halving (iterative compression) */
        x = d->parent[x];
    }
    return x;
}

static int dsu_unite(DSU *d, int x, int y) {
    int rx = dsu_find(d, x), ry = dsu_find(d, y);
    if (rx == ry) return 0;
    if (d->rank_[rx] < d->rank_[ry]) d->parent[rx] = ry;
    else if (d->rank_[rx] > d->rank_[ry]) d->parent[ry] = rx;
    else { d->parent[ry] = rx; d->rank_[rx]++; }
    return 1;
}

static void dsu_free(DSU *d) { free(d->parent); free(d->rank_); }

/* Kruskal MST */
typedef struct { int u, v, w; } Edge;
static int cmp_edge(const void *a, const void *b) { return ((const Edge *)a)->w - ((const Edge *)b)->w; }

static int kruskal(Edge *edges, int m, int n) {
    qsort(edges, m, sizeof(Edge), cmp_edge);
    DSU d; dsu_init(&d, n);
    int total = 0, picked = 0;
    for (int i = 0; i < m && picked < n - 1; ++i) {
        if (dsu_unite(&d, edges[i].u, edges[i].v)) {
            total += edges[i].w;
            picked++;
        }
    }
    dsu_free(&d);
    return picked == n - 1 ? total : -1;
}

static double now(void) {
    struct timespec ts; clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

int main(void) {
    /* Functional check on a small graph. */
    DSU d; dsu_init(&d, 10);
    dsu_unite(&d, 1, 2);
    dsu_unite(&d, 2, 3);
    dsu_unite(&d, 5, 6);
    dsu_unite(&d, 7, 6);
    printf("== DSU functional check ==\n");
    printf("  connected(1, 3) = %d (expect 1)\n", dsu_find(&d, 1) == dsu_find(&d, 3));
    printf("  connected(1, 5) = %d (expect 0)\n", dsu_find(&d, 1) == dsu_find(&d, 5));
    printf("  connected(5, 7) = %d (expect 1)\n", dsu_find(&d, 5) == dsu_find(&d, 7));
    dsu_free(&d);

    /* Kruskal MST */
    int n = 5;
    Edge edges[] = {
        {0, 1, 4}, {0, 2, 3}, {1, 2, 1}, {1, 3, 2},
        {2, 3, 4}, {3, 4, 2}, {4, 0, 4}, {4, 2, 4},
    };
    int m = sizeof(edges) / sizeof(edges[0]);
    int mst = kruskal(edges, m, n);
    printf("\n== Kruskal MST ==\n");
    printf("  total weight = %d (expect 8: edges 1-2(1), 1-3(2), 3-4(2), 0-2(3))\n", mst);

    /* Benchmark: 1M unions on a 1M-vertex random graph. */
    const int N = 1000000;
    const int M = 1000000;
    DSU big; dsu_init(&big, N);
    srand(42);
    double t0 = now();
    for (int i = 0; i < M; ++i) dsu_unite(&big, rand() % N, rand() % N);
    double t = now() - t0;
    /* Count components */
    int components = 0;
    for (int i = 0; i < N; ++i) if (dsu_find(&big, i) == i) ++components;
    printf("\n== Bench: %d unions on %d-vertex graph ==\n", M, N);
    printf("  time: %.3fs  (%.1f ns/union)\n", t, t * 1e9 / M);
    printf("  components after: %d\n", components);
    dsu_free(&big);

    return 0;
}
