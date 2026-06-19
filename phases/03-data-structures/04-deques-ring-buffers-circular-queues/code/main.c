/* main.c — deque, overwriting trace buffer, and lock-free SPSC ring. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdatomic.h>
#include <pthread.h>
#include <assert.h>
#include <time.h>

/* ============================================================ */
/* Deque using uncapped indices                                  */
/* ============================================================ */

typedef struct {
    int    *buf;
    size_t  head;       /* uncapped */
    size_t  tail;       /* uncapped */
    size_t  mask;
} Deque;

static void dq_init(Deque *d, size_t min_cap) {
    size_t cap = 1; while (cap < min_cap) cap *= 2;
    d->buf = malloc(cap * sizeof(int));
    d->head = d->tail = 0;
    d->mask = cap - 1;
}
static size_t dq_len(const Deque *d) { return d->tail - d->head; }
static int    dq_full(const Deque *d) { return dq_len(d) == d->mask + 1; }

static void dq_push_back(Deque *d, int x)  { assert(!dq_full(d)); d->buf[d->tail++ & d->mask] = x; }
static void dq_push_front(Deque *d, int x) { assert(!dq_full(d)); d->buf[--d->head & d->mask] = x; }
static int  dq_pop_back(Deque *d)          { assert(dq_len(d)); return d->buf[--d->tail & d->mask]; }
static int  dq_pop_front(Deque *d)         { assert(dq_len(d)); return d->buf[d->head++ & d->mask]; }
static void dq_free(Deque *d)              { free(d->buf); }

/* ============================================================ */
/* Overwriting trace buffer                                      */
/* ============================================================ */

typedef struct {
    int    *buf;
    size_t  head, tail, mask, len, capw;
} Trace;

static void tr_init(Trace *t, size_t cap) {
    size_t c = 1; while (c < cap) c *= 2;
    t->buf = malloc(c * sizeof(int));
    t->head = t->tail = t->len = 0;
    t->mask = c - 1; t->capw = c;
}
static void tr_push(Trace *t, int x) {
    t->buf[t->tail] = x;
    t->tail = (t->tail + 1) & t->mask;
    if (t->len == t->capw) t->head = (t->head + 1) & t->mask;
    else                   t->len++;
}
static void tr_dump(const Trace *t, const char *label) {
    printf("  %s [", label);
    for (size_t i = 0; i < t->len; ++i)
        printf("%d%s", t->buf[(t->head + i) & t->mask], i+1 < t->len ? ", " : "");
    printf("]\n");
}
static void tr_free(Trace *t) { free(t->buf); }

/* ============================================================ */
/* Lock-free SPSC ring buffer                                    */
/* ============================================================ */

#define SPSC_CAP 1024                       /* power of 2 */
typedef struct {
    int                 buf[SPSC_CAP];
    _Atomic size_t      head;               /* consumer-owned */
    _Atomic size_t      tail;               /* producer-owned */
} SPSC;

static int spsc_try_push(SPSC *q, int x) {
    size_t t = atomic_load_explicit(&q->tail, memory_order_relaxed);
    size_t h = atomic_load_explicit(&q->head, memory_order_acquire);
    if (t - h == SPSC_CAP) return 0;        /* full */
    q->buf[t & (SPSC_CAP - 1)] = x;
    atomic_store_explicit(&q->tail, t + 1, memory_order_release);
    return 1;
}

static int spsc_try_pop(SPSC *q, int *out) {
    size_t h = atomic_load_explicit(&q->head, memory_order_relaxed);
    size_t t = atomic_load_explicit(&q->tail, memory_order_acquire);
    if (h == t) return 0;                   /* empty */
    *out = q->buf[h & (SPSC_CAP - 1)];
    atomic_store_explicit(&q->head, h + 1, memory_order_release);
    return 1;
}

static SPSC g_queue;
static const size_t N_ITEMS = 1000000;

static void *producer(void *arg) {
    (void)arg;
    for (size_t i = 0; i < N_ITEMS; ) {
        if (spsc_try_push(&g_queue, (int)i)) i++;
    }
    return NULL;
}

static void *consumer(void *arg) {
    long *sum_out = (long *)arg;
    long sum = 0;
    for (size_t i = 0; i < N_ITEMS; ) {
        int x;
        if (spsc_try_pop(&g_queue, &x)) { sum += x; i++; }
    }
    *sum_out = sum;
    return NULL;
}

int main(void) {
    /* Deque demo */
    printf("== Deque ==\n");
    Deque d; dq_init(&d, 16);
    dq_push_back(&d, 1);
    dq_push_back(&d, 2);
    dq_push_back(&d, 3);
    dq_push_front(&d, 0);
    printf("  after pushes: len=%zu\n", dq_len(&d));
    printf("  pop_front -> %d (expect 0)\n", dq_pop_front(&d));
    printf("  pop_back  -> %d (expect 3)\n", dq_pop_back(&d));
    printf("  pop_front -> %d (expect 1)\n", dq_pop_front(&d));
    printf("  pop_front -> %d (expect 2)\n", dq_pop_front(&d));
    printf("  empty: len=%zu\n", dq_len(&d));
    dq_free(&d);

    /* Trace buffer demo */
    printf("\n== Overwriting trace buffer (cap=8) ==\n");
    Trace t; tr_init(&t, 8);
    for (int i = 1; i <= 5; ++i) tr_push(&t, i);
    tr_dump(&t, "after 1..5      :");
    for (int i = 6; i <= 12; ++i) tr_push(&t, i);
    tr_dump(&t, "after 1..12     :");
    tr_free(&t);

    /* SPSC demo */
    printf("\n== Lock-free SPSC ring buffer ==\n");
    pthread_t p, c;
    long sum = 0;
    struct timespec t0, t1;
    clock_gettime(CLOCK_MONOTONIC, &t0);
    pthread_create(&p, NULL, producer, NULL);
    pthread_create(&c, NULL, consumer, &sum);
    pthread_join(p, NULL);
    pthread_join(c, NULL);
    clock_gettime(CLOCK_MONOTONIC, &t1);
    double elapsed = (t1.tv_sec - t0.tv_sec) + (t1.tv_nsec - t0.tv_nsec) * 1e-9;
    long expected = ((long)(N_ITEMS - 1) * N_ITEMS) / 2;
    printf("  consumed %zu items in %.3f s  (%.1f Mitems/s)\n",
           N_ITEMS, elapsed, N_ITEMS / 1e6 / elapsed);
    printf("  checksum %ld (expected %ld) — %s\n",
           sum, expected, sum == expected ? "OK" : "MISMATCH!");

    return 0;
}
