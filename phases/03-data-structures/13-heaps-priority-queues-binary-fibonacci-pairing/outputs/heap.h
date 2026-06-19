/*
 * heap.h — single-header binary min-heap (int keys).
 *
 *   Heap h; heap_init(&h, 16);
 *   heap_push(&h, 42);
 *   int x = heap_pop(&h);
 *   heap_free(&h);
 */
#ifndef HEAP_H
#define HEAP_H

#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

typedef struct {
    int    *a;
    size_t  len, cap;
} Heap;

static inline void heap__swap(int *x, int *y) { int t = *x; *x = *y; *y = t; }

static inline void heap_init(Heap *h, size_t cap) {
    if (cap < 4) cap = 4;
    h->a = (int *)malloc(cap * sizeof(int));
    h->len = 0; h->cap = cap;
}

static inline void heap__siftup(Heap *h, size_t i) {
    while (i > 0) {
        size_t p = (i - 1) / 2;
        if (h->a[p] <= h->a[i]) break;
        heap__swap(&h->a[p], &h->a[i]);
        i = p;
    }
}

static inline void heap__siftdown(Heap *h, size_t i) {
    while (1) {
        size_t l = 2 * i + 1, r = 2 * i + 2, smallest = i;
        if (l < h->len && h->a[l] < h->a[smallest]) smallest = l;
        if (r < h->len && h->a[r] < h->a[smallest]) smallest = r;
        if (smallest == i) return;
        heap__swap(&h->a[i], &h->a[smallest]);
        i = smallest;
    }
}

static inline void heap_push(Heap *h, int x) {
    if (h->len == h->cap) { h->cap *= 2; h->a = (int *)realloc(h->a, h->cap * sizeof(int)); }
    h->a[h->len++] = x;
    heap__siftup(h, h->len - 1);
}

static inline int heap_pop(Heap *h) {
    int top = h->a[0];
    h->a[0] = h->a[--h->len];
    if (h->len > 0) heap__siftdown(h, 0);
    return top;
}

static inline int heap_peek(const Heap *h) { return h->a[0]; }

/* Floyd's O(n) build_heap. Copies src into h. */
static inline void heap_build(Heap *h, const int *src, size_t n) {
    if (h->cap < n) {
        h->cap = n;
        h->a = (int *)realloc(h->a, n * sizeof(int));
    }
    memcpy(h->a, src, n * sizeof(int));
    h->len = n;
    if (n < 2) return;
    for (size_t i = n / 2 - 1; ; --i) {
        heap__siftdown(h, i);
        if (i == 0) break;
    }
}

static inline void heap_free(Heap *h) { free(h->a); h->a = NULL; h->len = h->cap = 0; }

#endif /* HEAP_H */
