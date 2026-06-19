/*
 * Lesson 15: Deadlock — Detection, Prevention, Avoidance
 *
 * Implements Resource Allocation Graph cycle detection,
 * Banker's algorithm, and deadlock detection.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

/* ─── Resource Allocation Graph (cycle detection) ──────────────── */

#define MAX_NODES 32

typedef enum { NODE_PROC, NODE_RES } node_type_t;

typedef struct {
    node_type_t type;
    int         id;        /* process or resource id */
    int         instances; /* for resources */
} rag_node_t;

typedef struct {
    int from;
    int to;
} rag_edge_t;

typedef struct {
    rag_node_t nodes[MAX_NODES];
    rag_edge_t edges[MAX_NODES * MAX_NODES];
    int        node_count;
    int        edge_count;
} rag_t;

static void rag_init(rag_t *g) {
    g->node_count = 0;
    g->edge_count = 0;
}

static int rag_add_node(rag_t *g, node_type_t type, int id, int instances) {
    if (g->node_count >= MAX_NODES) return -1;
    g->nodes[g->node_count] = (rag_node_t){ type, id, instances };
    return g->node_count++;
}

static void rag_add_edge(rag_t *g, int from, int to) {
    g->edges[g->edge_count++] = (rag_edge_t){ from, to };
}

static bool dfs_cycle(rag_t *g, int node, int *visited, int *stack) {
    visited[node] = 1;
    stack[node] = 1;
    for (int i = 0; i < g->edge_count; i++) {
        if (g->edges[i].from != node) continue;
        int v = g->edges[i].to;
        if (!visited[v]) {
            if (dfs_cycle(g, v, visited, stack)) return true;
        } else if (stack[v]) {
            return true;
        }
    }
    stack[node] = 0;
    return false;
}

static bool rag_has_cycle(rag_t *g) {
    int visited[MAX_NODES] = {0};
    int stack[MAX_NODES] = {0};
    for (int i = 0; i < g->node_count; i++) {
        if (!visited[i] && g->nodes[i].type == NODE_PROC) {
            if (dfs_cycle(g, i, visited, stack)) return true;
        }
    }
    return false;
}

/* ─── Banker's Algorithm ───────────────────────────────────────── */

typedef struct {
    int n; /* processes */
    int m; /* resource types */
    int *available;    /* [m] */
    int *max_demand;   /* [n][m] */
    int *allocation;   /* [n][m] */
    int *need;         /* [n][m] */
} banker_t;

static banker_t *banker_create(int n, int m, int *available,
                                int *max_demand, int *allocation) {
    banker_t *b = malloc(sizeof(banker_t));
    b->n = n; b->m = m;
    b->available = malloc(m * sizeof(int));
    b->max_demand = malloc(n * m * sizeof(int));
    b->allocation = malloc(n * m * sizeof(int));
    b->need = malloc(n * m * sizeof(int));
    memcpy(b->available, available, m * sizeof(int));
    memcpy(b->max_demand, max_demand, n * m * sizeof(int));
    memcpy(b->allocation, allocation, n * m * sizeof(int));
    for (int i = 0; i < n; i++)
        for (int j = 0; j < m; j++)
            b->need[i * m + j] = b->max_demand[i * m + j] - b->allocation[i * m + j];
    return b;
}

static void banker_free(banker_t *b) {
    free(b->available); free(b->max_demand);
    free(b->allocation); free(b->need); free(b);
}

/* Returns safe sequence in `safe_seq` if safe, else returns false */
static bool banker_safety(banker_t *b, int *safe_seq) {
    int m = b->m, n = b->n;
    int *work = malloc(m * sizeof(int));
    memcpy(work, b->available, m * sizeof(int));
    bool *finish = calloc(n, sizeof(bool));
    int count = 0;

    bool changed = true;
    while (changed) {
        changed = false;
        for (int i = 0; i < n; i++) {
            if (finish[i]) continue;
            bool can_finish = true;
            for (int j = 0; j < m; j++) {
                if (b->need[i * m + j] > work[j]) { can_finish = false; break; }
            }
            if (can_finish) {
                for (int j = 0; j < m; j++)
                    work[j] += b->allocation[i * m + j];
                finish[i] = true;
                if (safe_seq) safe_seq[count] = i;
                count++;
                changed = true;
            }
        }
    }
    free(work);
    bool safe = (count == n);
    free(finish);
    return safe;
}

/* Request resources for process `pid`. Returns true if granted. */
static bool banker_request(banker_t *b, int pid, int *request) {
    int m = b->m;
    for (int j = 0; j < m; j++) {
        if (request[j] > b->need[pid * m + j]) return false;
        if (request[j] > b->available[j]) return false;
    }
    /* Tentatively allocate */
    for (int j = 0; j < m; j++) {
        b->available[j] -= request[j];
        b->allocation[pid * m + j] += request[j];
        b->need[pid * m + j] -= request[j];
    }
    int *seq = malloc(b->n * sizeof(int));
    bool safe = banker_safety(b, seq);
    if (!safe) {
        /* Rollback */
        for (int j = 0; j < m; j++) {
            b->available[j] += request[j];
            b->allocation[pid * m + j] -= request[j];
            b->need[pid * m + j] += request[j];
        }
    }
    free(seq);
    return safe;
}

/* ─── Deadlock Detection (multiple instance) ───────────────────── */

/* Returns list of deadlocked process indices. `out` must hold n ints. */
static int detect_deadlock(int *available, int *request, int *allocation,
                           int n, int m, int *out) {
    int *work = malloc(m * sizeof(int));
    memcpy(work, available, m * sizeof(int));
    bool *finish = calloc(n, sizeof(bool));

    for (int i = 0; i < n; i++) {
        bool zero = true;
        for (int j = 0; j < m; j++) {
            if (allocation[i * m + j] != 0) { zero = false; break; }
        }
        finish[i] = zero;
    }

    bool changed = true;
    while (changed) {
        changed = false;
        for (int i = 0; i < n; i++) {
            if (finish[i]) continue;
            bool can = true;
            for (int j = 0; j < m; j++) {
                if (request[i * m + j] > work[j]) { can = false; break; }
            }
            if (can) {
                for (int j = 0; j < m; j++)
                    work[j] += allocation[i * m + j];
                finish[i] = true;
                changed = true;
            }
        }
    }
    int count = 0;
    for (int i = 0; i < n; i++)
        if (!finish[i]) out[count++] = i;
    free(work); free(finish);
    return count;
}

/* ─── Helpers ──────────────────────────────────────────────────── */

static void print_matrix(const char *label, int *mat, int n, int m) {
    printf("  %s:\n", label);
    for (int i = 0; i < n; i++) {
        printf("    P%d: [", i);
        for (int j = 0; j < m; j++) {
            printf("%d%s", mat[i * m + j], j < m - 1 ? ", " : "");
        }
        printf("]\n");
    }
}

static void print_vec(const char *label, int *v, int n) {
    printf("  %s: [", label);
    for (int i = 0; i < n; i++)
        printf("%d%s", v[i], i < n - 1 ? ", " : "");
    printf("]\n");
}

/* ─── Main Demo ────────────────────────────────────────────────── */

int main(void) {
    printf("=== Lesson 15: Deadlock Toolkit Demo ===\n\n");

    /* ─── Part 1: RAG Cycle Detection ──────────────────────────── */
    printf("--- Part 1: Resource Allocation Graph ---\n");

    /* Deadlock scenario: P0 holds R0, wants R1; P1 holds R1, wants R0 */
    rag_t g;
    rag_init(&g);
    int p0 = rag_add_node(&g, NODE_PROC, 0, 0);
    int p1 = rag_add_node(&g, NODE_PROC, 1, 0);
    int r0 = rag_add_node(&g, NODE_RES, 0, 1);
    int r1 = rag_add_node(&g, NODE_RES, 1, 1);

    /* Assignment: R0 -> P0, R1 -> P1 */
    rag_add_edge(&g, r0, p0);
    rag_add_edge(&g, r1, p1);
    /* Request: P0 -> R1, P1 -> R0 */
    rag_add_edge(&g, p0, r1);
    rag_add_edge(&g, p1, r0);

    printf("  Scenario: P0 holds R0 wants R1, P1 holds R1 wants R0\n");
    printf("  Cycle detected: %s\n\n", rag_has_cycle(&g) ? "YES (deadlock!)" : "NO");

    /* No-deadlock scenario */
    rag_t g2;
    rag_init(&g2);
    int q0 = rag_add_node(&g2, NODE_PROC, 0, 0);
    int q1 = rag_add_node(&g2, NODE_PROC, 1, 0);
    int s0 = rag_add_node(&g2, NODE_RES, 0, 1);
    int s1 = rag_add_node(&g2, NODE_RES, 1, 1);
    rag_add_edge(&g2, s0, q0);
    rag_add_edge(&g2, s1, q0);
    rag_add_edge(&g2, q1, s0);

    printf("  Scenario: P0 holds R0+R1, P1 wants R0\n");
    printf("  Cycle detected: %s\n\n", rag_has_cycle(&g2) ? "YES" : "NO (safe)");

    /* ─── Part 2: Banker's Algorithm ───────────────────────────── */
    printf("--- Part 2: Banker's Algorithm ---\n");
    printf("  5 processes, 3 resource types\n");

    int available[3] = {3, 3, 2};
    int max_demand[5 * 3] = {
        7, 5, 3,  /* P0 */
        3, 2, 2,  /* P1 */
        9, 0, 2,  /* P2 */
        2, 2, 2,  /* P3 */
        4, 3, 3   /* P4 */
    };
    int allocation[5 * 3] = {
        0, 1, 0,  /* P0 */
        2, 0, 0,  /* P1 */
        3, 0, 2,  /* P2 */
        2, 1, 1,  /* P3 */
        0, 0, 2   /* P4 */
    };

    banker_t *b = banker_create(5, 3, available, max_demand, allocation);
    print_vec("Available", available, 3);
    print_matrix("Max", max_demand, 5, 3);
    print_matrix("Allocation", allocation, 5, 3);
    print_matrix("Need", b->need, 5, 3);

    int safe_seq[5];
    bool safe = banker_safety(b, safe_seq);
    if (safe) {
        printf("  System is SAFE. Safe sequence: ");
        for (int i = 0; i < 5; i++)
            printf("P%d%s", safe_seq[i], i < 4 ? " -> " : "");
        printf("\n");
    } else {
        printf("  System is UNSAFE!\n");
    }

    /* P1 requests (1, 0, 2) */
    int req1[3] = {1, 0, 2};
    printf("\n  P1 requests [1, 0, 2]: ");
    if (banker_request(b, 1, req1))
        printf("GRANTED (still safe)\n");
    else
        printf("DENIED (would be unsafe)\n");

    /* P4 requests (3, 3, 0) — too much */
    int req4[3] = {3, 3, 0};
    printf("  P4 requests [3, 3, 0]: ");
    if (banker_request(b, 4, req4))
        printf("GRANTED\n");
    else
        printf("DENIED (exceeds available or unsafe)\n");

    banker_free(b);

    /* ─── Part 3: Deadlock Detection ───────────────────────────── */
    printf("\n--- Part 3: Deadlock Detection ---\n");
    printf("  4 processes, 2 resource types\n");

    int det_avail[2] = {0, 0};
    int det_request[4 * 2] = {
        0, 1,  /* P0 wants 1 of R1 */
        2, 0,  /* P1 wants 2 of R0 */
        0, 0,  /* P2 wants nothing (done) */
        1, 0   /* P3 wants 1 of R0 */
    };
    int det_alloc[4 * 2] = {
        1, 0,  /* P0 has 1 of R0 */
        0, 1,  /* P1 has 1 of R1 */
        0, 0,  /* P2 has nothing */
        0, 1   /* P3 has 1 of R1 */
    };

    print_vec("Available", det_avail, 2);
    print_matrix("Request", det_request, 4, 2);
    print_matrix("Allocation", det_alloc, 4, 2);

    int deadlocked[4];
    int nd = detect_deadlock(det_avail, det_request, det_alloc, 4, 2, deadlocked);
    if (nd == 0) {
        printf("  No deadlock detected.\n");
    } else {
        printf("  Deadlocked processes: ");
        for (int i = 0; i < nd; i++)
            printf("P%d%s", deadlocked[i], i < nd - 1 ? ", " : "");
        printf("\n");
    }

    return 0;
}
