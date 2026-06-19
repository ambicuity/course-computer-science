/*
 * graph.h — single-header adjacency list + CSR graph.
 *
 *   GraphList g; gl_init(&g, n);
 *   gl_add_edge(&g, u, v);
 *   for (int i = 0; i < g.deg[u]; ++i) {
 *       int v = g.adj[u][i].dst;
 *       ...
 *   }
 *
 *   GraphCSR c; gc_from_list(&c, &g);   // finalize for fast iteration
 *   for (int i = c.row_starts[u]; i < c.row_starts[u+1]; ++i) ...
 */
#ifndef GRAPH_H
#define GRAPH_H

#include <stdlib.h>

typedef struct { int dst; } Edge;

typedef struct {
    Edge **adj;
    int   *deg;
    int   *cap;
    int    n;
} GraphList;

static inline void gl_init(GraphList *g, int n) {
    g->n = n;
    g->adj = (Edge **)calloc(n, sizeof(Edge *));
    g->deg = (int *)calloc(n, sizeof(int));
    g->cap = (int *)calloc(n, sizeof(int));
}

static inline void gl_add_edge(GraphList *g, int u, int v) {
    if (g->deg[u] == g->cap[u]) {
        g->cap[u] = g->cap[u] ? g->cap[u] * 2 : 4;
        g->adj[u] = (Edge *)realloc(g->adj[u], g->cap[u] * sizeof(Edge));
    }
    g->adj[u][g->deg[u]++] = (Edge){v};
}

static inline void gl_free(GraphList *g) {
    for (int i = 0; i < g->n; ++i) free(g->adj[i]);
    free(g->adj); free(g->deg); free(g->cap);
}

typedef struct {
    int *row_starts;
    int *neighbors;
    int  n, m;
} GraphCSR;

static inline void gc_from_list(GraphCSR *c, const GraphList *g) {
    c->n = g->n;
    c->row_starts = (int *)malloc((g->n + 1) * sizeof(int));
    int total = 0;
    for (int i = 0; i < g->n; ++i) { c->row_starts[i] = total; total += g->deg[i]; }
    c->row_starts[g->n] = total;
    c->m = total;
    c->neighbors = (int *)malloc(total * sizeof(int));
    int idx = 0;
    for (int u = 0; u < g->n; ++u)
        for (int i = 0; i < g->deg[u]; ++i) c->neighbors[idx++] = g->adj[u][i].dst;
}

static inline void gc_free(GraphCSR *c) { free(c->row_starts); free(c->neighbors); }

#endif /* GRAPH_H */
