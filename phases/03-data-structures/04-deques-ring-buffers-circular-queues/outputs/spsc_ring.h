/*
 * spsc_ring.h — single-producer single-consumer lock-free ring buffer.
 *
 * Compile with C11+ (needs <stdatomic.h>). Power-of-2 capacity.
 *
 * Usage (define your element type and cap before including):
 *
 *     #define SPSC_T int
 *     #define SPSC_CAP 1024
 *     #include "spsc_ring.h"
 *
 *     SPSC_RING q;
 *     spsc_ring_init(&q);
 *     // producer thread:
 *     if (spsc_try_push(&q, 42)) { ... }
 *     // consumer thread:
 *     int x;
 *     if (spsc_try_pop(&q, &x)) { ... }
 *
 * Memory orderings are labeled. Do not change them — they make the buffer
 * sound on weakly-ordered CPUs (ARM, POWER) as well as strongly-ordered (x86).
 *
 * License: MIT.
 */
#ifndef SPSC_RING_H_INCLUDED
#define SPSC_RING_H_INCLUDED

#include <stdatomic.h>
#include <stddef.h>

#ifndef SPSC_T
#  define SPSC_T int
#endif
#ifndef SPSC_CAP
#  define SPSC_CAP 1024
#endif
#if (SPSC_CAP & (SPSC_CAP - 1)) != 0
#  error "SPSC_CAP must be a power of 2"
#endif

typedef struct {
    SPSC_T              buf[SPSC_CAP];
    _Atomic size_t      head;   /* consumer owns; producer reads with acquire */
    _Atomic size_t      tail;   /* producer owns; consumer reads with acquire */
} SPSC_RING;

static inline void spsc_ring_init(SPSC_RING *q) {
    atomic_store_explicit(&q->head, 0, memory_order_relaxed);
    atomic_store_explicit(&q->tail, 0, memory_order_relaxed);
}

static inline int spsc_try_push(SPSC_RING *q, SPSC_T x) {
    size_t t = atomic_load_explicit(&q->tail, memory_order_relaxed);
    size_t h = atomic_load_explicit(&q->head, memory_order_acquire);
    if (t - h == SPSC_CAP) return 0;                /* full */
    q->buf[t & (SPSC_CAP - 1)] = x;
    atomic_store_explicit(&q->tail, t + 1, memory_order_release);
    return 1;
}

static inline int spsc_try_pop(SPSC_RING *q, SPSC_T *out) {
    size_t h = atomic_load_explicit(&q->head, memory_order_relaxed);
    size_t t = atomic_load_explicit(&q->tail, memory_order_acquire);
    if (h == t) return 0;                           /* empty */
    *out = q->buf[h & (SPSC_CAP - 1)];
    atomic_store_explicit(&q->head, h + 1, memory_order_release);
    return 1;
}

static inline size_t spsc_size(const SPSC_RING *q) {
    size_t t = atomic_load_explicit(&q->tail, memory_order_acquire);
    size_t h = atomic_load_explicit(&q->head, memory_order_acquire);
    return t - h;
}

#endif /* SPSC_RING_H_INCLUDED */
