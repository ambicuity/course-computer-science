/* main.c — three graph representations + BFS over each. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <time.h>

/* ============================================================ */
/* Adjacency list                                                */
/* ============================================================ */
typedef struct { int dst; } Edge;
typedef struct {
    Edge **adj;
    int   *deg;
    int   *cap;
    int    n;
} GraphList;

static void gl_init(GraphList *g, int n) {
    g->n = n;
    g->adj = calloc(n, sizeof(Edge *));
    g->deg = calloc(n, sizeof(int));
    g->cap = calloc(n, sizeof(int));
}

static void gl_add_edge(GraphList *g, int u, int v) {
    if (g->deg[u] == g->cap[u]) {
        g->cap[u] = g->cap[u] ? g->cap[u] * 2 : 4;
        g->adj[u] = realloc(g->adj[u], g->cap[u] * sizeof(Edge));
    }
    g->adj[u][g->deg[u]++] = (Edge){v};
}

static int gl_bfs(GraphList *g, int src) {
    int *dist = malloc(g->n * sizeof(int));
    for (int i = 0; i < g->n; ++i) dist[i] = -1;
    int *queue = malloc(g->n * sizeof(int));
    int head = 0, tail = 0;
    dist[src] = 0; queue[tail++] = src;
    int reached = 0;
    while (head < tail) {
        int u = queue[head++]; reached++;
        for (int i = 0; i < g->deg[u]; ++i) {
            int v = g->adj[u][i].dst;
            if (dist[v] == -1) { dist[v] = dist[u] + 1; queue[tail++] = v; }
        }
    }
    free(dist); free(queue);
    return reached;
}

static void gl_free(GraphList *g) {
    for (int i = 0; i < g->n; ++i) free(g->adj[i]);
    free(g->adj); free(g->deg); free(g->cap);
}

/* ============================================================ */
/* Adjacency matrix                                              */
/* ============================================================ */
typedef struct { uint8_t *m; int n; } GraphMat;

static void gm_init(GraphMat *g, int n) {
    g->n = n; g->m = calloc((size_t)n * n, sizeof(uint8_t));
}
static void gm_add_edge(GraphMat *g, int u, int v) { g->m[u * g->n + v] = 1; }
static int gm_bfs(GraphMat *g, int src) {
    int *dist = malloc(g->n * sizeof(int));
    for (int i = 0; i < g->n; ++i) dist[i] = -1;
    int *queue = malloc(g->n * sizeof(int));
    int head = 0, tail = 0;
    dist[src] = 0; queue[tail++] = src;
    int reached = 0;
    while (head < tail) {
        int u = queue[head++]; reached++;
        for (int v = 0; v < g->n; ++v) {
            if (g->m[u * g->n + v] && dist[v] == -1) {
                dist[v] = dist[u] + 1; queue[tail++] = v;
            }
        }
    }
    free(dist); free(queue);
    return reached;
}
static void gm_free(GraphMat *g) { free(g->m); }

/* ============================================================ */
/* CSR (built from adjacency list)                              */
/* ============================================================ */
typedef struct {
    int *row_starts;       /* size n+1 */
    int *neighbors;        /* size m */
    int  n, m;
} GraphCSR;

static void gc_from_list(GraphCSR *c, const GraphList *g) {
    c->n = g->n;
    c->row_starts = malloc((g->n + 1) * sizeof(int));
    int total = 0;
    for (int i = 0; i < g->n; ++i) { c->row_starts[i] = total; total += g->deg[i]; }
    c->row_starts[g->n] = total;
    c->m = total;
    c->neighbors = malloc(total * sizeof(int));
    int idx = 0;
    for (int u = 0; u < g->n; ++u)
        for (int i = 0; i < g->deg[u]; ++i) c->neighbors[idx++] = g->adj[u][i].dst;
}

static int gc_bfs(GraphCSR *c, int src) {
    int *dist = malloc(c->n * sizeof(int));
    for (int i = 0; i < c->n; ++i) dist[i] = -1;
    int *queue = malloc(c->n * sizeof(int));
    int head = 0, tail = 0;
    dist[src] = 0; queue[tail++] = src;
    int reached = 0;
    while (head < tail) {
        int u = queue[head++]; reached++;
        for (int i = c->row_starts[u]; i < c->row_starts[u+1]; ++i) {
            int v = c->neighbors[i];
            if (dist[v] == -1) { dist[v] = dist[u] + 1; queue[tail++] = v; }
        }
    }
    free(dist); free(queue);
    return reached;
}
static void gc_free(GraphCSR *c) { free(c->row_starts); free(c->neighbors); }

static double now(void) {
    struct timespec ts; clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

int main(void) {
    const int N = 1000;
    const int M = 8000;       /* avg degree 8 — sparse */
    srand(42);

    GraphList gl; gl_init(&gl, N);
    for (int e = 0; e < M; ++e) {
        int u = rand() % N, v = rand() % N;
        gl_add_edge(&gl, u, v);
    }

    /* Same edges into matrix and CSR */
    GraphMat gm; gm_init(&gm, N);
    for (int u = 0; u < N; ++u)
        for (int i = 0; i < gl.deg[u]; ++i) gm_add_edge(&gm, u, gl.adj[u][i].dst);

    GraphCSR gc; gc_from_list(&gc, &gl);

    printf("== Graph (N=%d, M=%d) ==\n\n", N, M);
    printf("Memory:\n");
    printf("  adjacency list: %zu B (n + m edges, +array overhead)\n",
           (size_t)N * sizeof(int) * 2 + M * sizeof(Edge));
    printf("  adjacency matrix: %zu B (n²)\n", (size_t)N * N);
    printf("  CSR: %zu B (n + m)\n", (size_t)(N + 1) * sizeof(int) + M * sizeof(int));

    int reached_l = 0, reached_m = 0, reached_c = 0;
    double t;

    t = now();
    for (int i = 0; i < 100; ++i) reached_l = gl_bfs(&gl, 0);
    double t_l = (now() - t) / 100;

    t = now();
    for (int i = 0; i < 100; ++i) reached_m = gm_bfs(&gm, 0);
    double t_m = (now() - t) / 100;

    t = now();
    for (int i = 0; i < 100; ++i) reached_c = gc_bfs(&gc, 0);
    double t_c = (now() - t) / 100;

    printf("\nBFS from vertex 0 (avg over 100 runs):\n");
    printf("  adjacency list: %.0f µs, reached %d nodes\n", t_l * 1e6, reached_l);
    printf("  adjacency matrix: %.0f µs, reached %d nodes\n", t_m * 1e6, reached_m);
    printf("  CSR:              %.0f µs, reached %d nodes\n", t_c * 1e6, reached_c);

    gl_free(&gl); gm_free(&gm); gc_free(&gc);
    return 0;
}
