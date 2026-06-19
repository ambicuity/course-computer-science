/* main.c — pool allocator implementation + benchmark vs malloc. */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <time.h>
#include <assert.h>

typedef struct Pool {
    void  *slab;
    size_t slot_size;
    size_t n_slots;
    void  *free_head;
} Pool;

static size_t round_up(size_t x, size_t align) {
    return (x + align - 1) & ~(align - 1);
}

Pool *pool_create(size_t slot_size, size_t n_slots) {
    if (slot_size < sizeof(void *)) slot_size = sizeof(void *);
    slot_size = round_up(slot_size, sizeof(void *));

    Pool *p = malloc(sizeof(*p));
    if (!p) return NULL;
    p->slab = aligned_alloc(16, slot_size * n_slots);
    if (!p->slab) { free(p); return NULL; }
    p->slot_size = slot_size;
    p->n_slots = n_slots;

    /* Thread the free-list through every slot. */
    char *cur = (char *)p->slab;
    for (size_t i = 0; i + 1 < n_slots; ++i) {
        *(void **)cur = cur + slot_size;
        cur += slot_size;
    }
    *(void **)cur = NULL;
    p->free_head = p->slab;
    return p;
}

void *pool_alloc(Pool *p) {
    if (!p->free_head) return NULL;
    void *slot = p->free_head;
    p->free_head = *(void **)slot;
    return slot;
}

void pool_free(Pool *p, void *obj) {
    *(void **)obj = p->free_head;
    p->free_head = obj;
}

void pool_destroy(Pool *p) {
    if (!p) return;
    free(p->slab);
    free(p);
}

size_t pool_free_count(const Pool *p) {
    size_t n = 0;
    void *cur = p->free_head;
    while (cur) { n++; cur = *(void **)cur; }
    return n;
}

typedef struct Node {
    int value;
    struct Node *next;
    char filler[40];
} Node;

double now_seconds(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

int main(void) {
    printf("== Pool allocator (64-byte slots, 100,000 of them) ==\n");
    Pool *p = pool_create(sizeof(Node), 100000);
    assert(p != NULL);
    printf("  slot_size = %zu bytes, n_slots = %zu, free_count = %zu\n",
           p->slot_size, p->n_slots, pool_free_count(p));

    printf("\n== LIFO behavior: alloc 1000, free reverse, alloc 1000 — should match ==\n");
    void *first_batch[1000];
    for (int i = 0; i < 1000; ++i) first_batch[i] = pool_alloc(p);
    for (int i = 999; i >= 0; --i) pool_free(p, first_batch[i]);
    void *second_batch[1000];
    for (int i = 0; i < 1000; ++i) second_batch[i] = pool_alloc(p);
    int matches = 0;
    for (int i = 0; i < 1000; ++i) if (first_batch[i] == second_batch[i]) matches++;
    printf("  pointer-match count: %d / 1000  (expected 1000 — perfect LIFO reuse)\n", matches);
    for (int i = 0; i < 1000; ++i) pool_free(p, second_batch[i]);

    printf("\n== Benchmark: pool vs malloc (1,000,000 alloc+free cycles, 64 bytes each) ==\n");
    int N = 1000000;

    /* Touch the allocation so the compiler can't optimize the loop away. */
    volatile unsigned long sink = 0;

    double t0 = now_seconds();
    for (int i = 0; i < N; ++i) {
        void *o = pool_alloc(p);
        ((char *)o)[0] = (char)i;          /* force a write to defeat DCE */
        sink += (unsigned long)o;
        pool_free(p, o);
    }
    double t_pool = now_seconds() - t0;
    printf("  pool:   %.4f s  (%.1f ns / op)\n", t_pool, t_pool * 1e9 / N);

    t0 = now_seconds();
    for (int i = 0; i < N; ++i) {
        void *o = malloc(64);
        ((char *)o)[0] = (char)i;
        sink += (unsigned long)o;
        free(o);
    }
    double t_malloc = now_seconds() - t0;
    printf("  malloc: %.4f s  (%.1f ns / op)\n", t_malloc, t_malloc * 1e9 / N);
    printf("  pool speedup: %.1f×    (sink=%lu — keeps loops live)\n",
           t_malloc / t_pool, sink);

    pool_destroy(p);
    return 0;
}
