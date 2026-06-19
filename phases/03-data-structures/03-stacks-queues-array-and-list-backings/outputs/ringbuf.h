/*
 * ringbuf.h — single-header power-of-2 ring buffer for int.
 *
 * Usage:
 *     RingBuf rb;
 *     rb_init(&rb, 16);              // initial cap rounded up to next power of 2
 *     rb_push(&rb, 42);              // O(1)
 *     int x = rb_pop(&rb);           // O(1)
 *     rb_free(&rb);
 *
 * The buffer grows by doubling. Push and pop are unconditionally O(1) amortized.
 */
#ifndef RINGBUF_H
#define RINGBUF_H

#include <stdlib.h>
#include <stddef.h>
#include <assert.h>

typedef struct {
    int    *buf;
    size_t  head, tail, mask, len;
} RingBuf;

static inline void rb_init(RingBuf *q, size_t min_cap) {
    size_t cap = 1;
    while (cap < min_cap) cap *= 2;
    q->buf = (int *)malloc(cap * sizeof(int));
    assert(q->buf);
    q->head = q->tail = q->len = 0;
    q->mask = cap - 1;
}

static inline void rb_grow(RingBuf *q) {
    size_t new_cap = (q->mask + 1) * 2;
    int *nb = (int *)malloc(new_cap * sizeof(int));
    assert(nb);
    for (size_t i = 0; i < q->len; ++i)
        nb[i] = q->buf[(q->head + i) & q->mask];
    free(q->buf);
    q->buf = nb;
    q->head = 0;
    q->tail = q->len;
    q->mask = new_cap - 1;
}

static inline void rb_push(RingBuf *q, int x) {
    if (q->len == q->mask + 1) rb_grow(q);
    q->buf[q->tail] = x;
    q->tail = (q->tail + 1) & q->mask;
    q->len++;
}

static inline int rb_pop(RingBuf *q) {
    assert(q->len > 0);
    int x = q->buf[q->head];
    q->head = (q->head + 1) & q->mask;
    q->len--;
    return x;
}

static inline size_t rb_len(const RingBuf *q) { return q->len; }
static inline void   rb_free(RingBuf *q)       { free(q->buf); q->buf = NULL; q->len = 0; }

#endif /* RINGBUF_H */
