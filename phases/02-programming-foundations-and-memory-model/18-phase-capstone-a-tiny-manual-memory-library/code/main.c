/* main.c — Phase 02 Capstone: memlib (arena + pool + bounded slice). */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <stddef.h>
#include <assert.h>
#include <time.h>

#ifndef NDEBUG
#  define MEMLIB_DEBUG 1
#else
#  define MEMLIB_DEBUG 0
#endif

#define REQUIRE(cond)  do { if (MEMLIB_DEBUG && !(cond)) { \
    fprintf(stderr, "REQUIRE failed: %s at %s:%d\n", #cond, __FILE__, __LINE__); abort(); } } while (0)
#define INVARIANT(cond) REQUIRE(cond)

/* ============================================================ */
/* Arena                                                        */
/* ============================================================ */

typedef struct MemArena {
    char  *base;
    size_t used;
    size_t capacity;
} MemArena;

static size_t mem_align_up(size_t n, size_t a) { return (n + a - 1) & ~(a - 1); }

MemArena *arena_create(size_t cap) {
    MemArena *a = malloc(sizeof(*a));
    if (!a) return NULL;
    a->base = aligned_alloc(16, mem_align_up(cap, 16));
    if (!a->base) { free(a); return NULL; }
    a->used = 0;
    a->capacity = mem_align_up(cap, 16);
    return a;
}

void *arena_alloc(MemArena *a, size_t bytes, size_t align) {
    REQUIRE(a != NULL);
    REQUIRE(align >= 1 && (align & (align - 1)) == 0);   /* power of 2 */
    size_t aligned = mem_align_up(a->used, align);
    if (aligned + bytes > a->capacity) return NULL;
    void *p = a->base + aligned;
    a->used = aligned + bytes;
    INVARIANT(a->used <= a->capacity);
    return p;
}

char *arena_strdup(MemArena *a, const char *s) {
    REQUIRE(s != NULL);
    size_t n = strlen(s) + 1;
    char *dst = arena_alloc(a, n, 1);
    if (!dst) return NULL;
    memcpy(dst, s, n);
    return dst;
}

void arena_reset(MemArena *a)   { REQUIRE(a != NULL); a->used = 0; }
size_t arena_used(const MemArena *a) { REQUIRE(a != NULL); return a->used; }

void arena_destroy(MemArena *a) {
    if (!a) return;
    free(a->base);
    free(a);
}

/* ============================================================ */
/* Pool                                                         */
/* ============================================================ */

typedef struct MemPool {
    void  *slab;
    size_t slot_size;
    size_t n_slots;
    void  *free_head;
} MemPool;

MemPool *pool_create(size_t slot_size, size_t n_slots) {
    if (slot_size < sizeof(void *)) slot_size = sizeof(void *);
    slot_size = mem_align_up(slot_size, sizeof(void *));
    MemPool *p = malloc(sizeof(*p));
    if (!p) return NULL;
    p->slab = aligned_alloc(16, slot_size * n_slots);
    if (!p->slab) { free(p); return NULL; }
    p->slot_size = slot_size;
    p->n_slots = n_slots;

    char *cur = (char *)p->slab;
    for (size_t i = 0; i + 1 < n_slots; ++i) {
        *(void **)cur = cur + slot_size;
        cur += slot_size;
    }
    *(void **)cur = NULL;
    p->free_head = p->slab;
    return p;
}

void *pool_alloc(MemPool *p) {
    REQUIRE(p != NULL);
    if (!p->free_head) return NULL;
    void *slot = p->free_head;
    p->free_head = *(void **)slot;
    return slot;
}

static int pool_owns(const MemPool *p, const void *obj) {
    const char *b = (const char *)p->slab;
    const char *o = (const char *)obj;
    if (o < b || o >= b + p->n_slots * p->slot_size) return 0;
    return ((size_t)(o - b)) % p->slot_size == 0;
}

void pool_free(MemPool *p, void *obj) {
    REQUIRE(p != NULL);
    REQUIRE(obj != NULL);
    REQUIRE(pool_owns(p, obj));  /* arg must come from this pool */
    *(void **)obj = p->free_head;
    p->free_head = obj;
}

size_t pool_free_count(const MemPool *p) {
    size_t n = 0;
    void *cur = p->free_head;
    while (cur) { n++; cur = *(void **)cur; }
    return n;
}

void pool_destroy(MemPool *p) {
    if (!p) return;
    free(p->slab);
    free(p);
}

/* ============================================================ */
/* Bounded Slice                                                */
/* ============================================================ */

typedef struct { void *data; size_t len, stride; } MemSlice;

void *slice_get(MemSlice s, size_t i) {
    REQUIRE(s.data != NULL);
    REQUIRE(i < s.len);
    return (char *)s.data + i * s.stride;
}

/* ============================================================ */
/* Demo                                                          */
/* ============================================================ */

typedef struct Node {
    int value;
    struct Node *next;
    char filler[40];
} Node;

static double now_seconds(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

int main(void) {
    printf("== memlib capstone (MEMLIB_DEBUG=%d) ==\n", MEMLIB_DEBUG);

    /* Arena demo: 100K strings of varying length */
    printf("\n-- Arena: 100,000 strings, capacity 8 MiB --\n");
    MemArena *a = arena_create(8 * 1024 * 1024);
    assert(a);
    const char *templates[] = {"hi", "hello world", "the quick brown fox", "x"};
    int n_strings = 100000;
    double t0 = now_seconds();
    for (int i = 0; i < n_strings; ++i) {
        const char *s = templates[i & 3];
        char *cp = arena_strdup(a, s);
        if (!cp) { printf("  arena OOM at i=%d (used=%zu)\n", i, arena_used(a)); break; }
    }
    double t = now_seconds() - t0;
    printf("  used=%zu bytes  time=%.3fs (%.1f ns/op)\n",
           arena_used(a), t, t * 1e9 / n_strings);

    /* Pool demo: 100K nodes, free in reverse */
    printf("\n-- Pool: 100,000 Node slots --\n");
    MemPool *p = pool_create(sizeof(Node), 100000);
    assert(p);
    Node *nodes[1000];
    for (int i = 0; i < 1000; ++i) {
        nodes[i] = pool_alloc(p);
        assert(nodes[i]);
        nodes[i]->value = i;
    }
    printf("  free_count after alloc 1000: %zu (expected 99000)\n", pool_free_count(p));
    for (int i = 999; i >= 0; --i) pool_free(p, nodes[i]);
    printf("  free_count after free  1000: %zu (expected 100000)\n", pool_free_count(p));

    /* Bounded slice demo */
    printf("\n-- Bounded slice --\n");
    int arr[8] = {10, 20, 30, 40, 50, 60, 70, 80};
    MemSlice s = { .data = arr, .len = 8, .stride = sizeof(int) };
    int *got = slice_get(s, 3);
    printf("  slice_get(s, 3) = %d  (expected 40)\n", *got);
    /* Uncommenting the next line would abort under MEMLIB_DEBUG: */
    /* slice_get(s, 99); */

    /* Pool error-detection demo: try to free a pointer that isn't from the pool */
#if MEMLIB_DEBUG
    printf("\n-- Defensive: invalid pool_free is caught in debug --\n");
    printf("  (skipping the abort demo — toggle by uncommenting in source)\n");
    /* int dummy; pool_free(p, &dummy);  -- aborts via REQUIRE(pool_owns(...)) */
#endif

    arena_destroy(a);
    pool_destroy(p);
    printf("\n== capstone complete ==\n");
    return 0;
}
