/* main.c — stacks and queues, both backings, head-to-head benchmark. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <assert.h>

static double now(void) {
    struct timespec ts; clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

/* =================== Array stack =================== */
typedef struct { int *d; size_t len, cap; } AStack;
static void as_push(AStack *s, int x) {
    if (s->len == s->cap) { s->cap = s->cap ? s->cap * 2 : 8; s->d = realloc(s->d, s->cap * sizeof(int)); }
    s->d[s->len++] = x;
}
static int as_pop(AStack *s) { return s->d[--s->len]; }

/* =================== List stack =================== */
typedef struct LSNode { int x; struct LSNode *next; } LSNode;
typedef struct { LSNode *head; } LStack;
static void ls_push(LStack *s, int x) {
    LSNode *n = malloc(sizeof(*n));
    n->x = x; n->next = s->head; s->head = n;
}
static int ls_pop(LStack *s) {
    LSNode *n = s->head; int x = n->x; s->head = n->next; free(n); return x;
}

/* =================== Naïve array queue (head walks forward, no wrap) =================== */
typedef struct { int *d; size_t head, len, cap; } NQ;
static void nq_enq(NQ *q, int x) {
    if (q->head + q->len == q->cap) {
        if (q->head > 0) {                                  /* shift to start: O(n) */
            memmove(q->d, q->d + q->head, q->len * sizeof(int));
            q->head = 0;
        }
        if (q->len == q->cap) { q->cap = q->cap ? q->cap * 2 : 8; q->d = realloc(q->d, q->cap * sizeof(int)); }
    }
    q->d[q->head + q->len++] = x;
}
static int nq_deq(NQ *q) { int x = q->d[q->head++]; q->len--; return x; }

/* =================== Ring buffer queue =================== */
typedef struct { int *d; size_t head, tail, mask, len; } RQ;
static void rq_init(RQ *q, size_t cap) {
    size_t c = 1; while (c < cap) c *= 2;
    q->d = malloc(c * sizeof(int)); q->head = q->tail = q->len = 0; q->mask = c - 1;
}
static void rq_enq(RQ *q, int x) {
    if (q->len == q->mask + 1) {                            /* grow */
        size_t new_cap = (q->mask + 1) * 2;
        int *nd = malloc(new_cap * sizeof(int));
        for (size_t i = 0; i < q->len; ++i)
            nd[i] = q->d[(q->head + i) & q->mask];
        free(q->d); q->d = nd;
        q->head = 0; q->tail = q->len; q->mask = new_cap - 1;
    }
    q->d[q->tail] = x; q->tail = (q->tail + 1) & q->mask; q->len++;
}
static int rq_deq(RQ *q) {
    int x = q->d[q->head]; q->head = (q->head + 1) & q->mask; q->len--; return x;
}

/* =================== Linked-list queue =================== */
typedef struct LQN { int x; struct LQN *next; } LQN;
typedef struct { LQN *head, *tail; } LQ;
static void lq_enq(LQ *q, int x) {
    LQN *n = malloc(sizeof(*n)); n->x = x; n->next = NULL;
    if (q->tail) q->tail->next = n; else q->head = n;
    q->tail = n;
}
static int lq_deq(LQ *q) {
    LQN *n = q->head; int x = n->x; q->head = n->next; if (!q->head) q->tail = NULL; free(n); return x;
}

/* =================== Bench =================== */
int main(void) {
    const int N = 200000;
    printf("== Stacks & queues, N=%d push/pop pairs ==\n\n", N);

    /* Stack: array */
    AStack as = {0};
    double t = now();
    for (int i = 0; i < N; ++i) as_push(&as, i);
    for (int i = 0; i < N; ++i) (void)as_pop(&as);
    printf("Stack (array):  %.4f s  (%.1f ns/op)\n", now() - t, (now() - t) * 1e9 / (2 * N));
    free(as.d);

    /* Stack: list */
    LStack ls = {0};
    t = now();
    for (int i = 0; i < N; ++i) ls_push(&ls, i);
    for (int i = 0; i < N; ++i) (void)ls_pop(&ls);
    printf("Stack (list):   %.4f s  (%.1f ns/op)\n", now() - t, (now() - t) * 1e9 / (2 * N));

    /* Queue workload: "rolling window" - fill to W, then alternate 1 enq + 1 deq.
       This makes head walk forward without freeing cap, forcing shifts on the naive queue. */
    const int W = 10000;
    const int ROLLS = N;

    /* Pin the naive queue's cap = W so each cycle forces a shift (the textbook bad case). */
    NQ nq = {0}; nq.cap = W; nq.d = malloc(W * sizeof(int));
    for (int i = 0; i < W; ++i) { nq.d[nq.head + nq.len++] = i; }   /* fill to W exactly */
    t = now();
    for (int i = 0; i < ROLLS; ++i) {
        /* enq: head+len==W → shift W-1 down then write */
        memmove(nq.d, nq.d + nq.head, nq.len * sizeof(int));
        nq.head = 0;
        nq.d[nq.len] = i; nq.len++;
        /* deq */
        nq.head++; nq.len--;
    }
    double t_nq = now() - t;
    free(nq.d);

    RQ rq; rq_init(&rq, 16);
    for (int i = 0; i < W; ++i) rq_enq(&rq, i);
    t = now();
    for (int i = 0; i < ROLLS; ++i) { rq_enq(&rq, i); (void)rq_deq(&rq); }
    double t_rq = now() - t;
    while (rq.len) (void)rq_deq(&rq); free(rq.d);

    LQ lq = {0};
    for (int i = 0; i < W; ++i) lq_enq(&lq, i);
    t = now();
    for (int i = 0; i < ROLLS; ++i) { lq_enq(&lq, i); (void)lq_deq(&lq); }
    double t_lq = now() - t;
    while (lq.head) (void)lq_deq(&lq);

    printf("\n== Rolling-window queue (steady-state size %d, %d enq+deq pairs) ==\n", W, ROLLS);
    printf("Queue (naive):  %.4f s  (%.1f ns/op)  ← shifts every time cap fills\n", t_nq, t_nq * 1e9 / (2 * ROLLS));
    printf("Queue (ring):   %.4f s  (%.1f ns/op)\n", t_rq, t_rq * 1e9 / (2 * ROLLS));
    printf("Queue (list):   %.4f s  (%.1f ns/op)\n", t_lq, t_lq * 1e9 / (2 * ROLLS));

    return 0;
}
