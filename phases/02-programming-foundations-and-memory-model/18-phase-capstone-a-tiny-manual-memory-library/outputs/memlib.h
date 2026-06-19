/*
 * memlib.h — Tiny manual-memory library: arena + pool + bounded slice.
 * Single-header. Drop into a project and #include.
 *
 * In ONE .c file:
 *     #define MEMLIB_IMPLEMENTATION
 *     #include "memlib.h"
 * In every other .c file, just #include "memlib.h" normally.
 *
 * Build flags:
 *   -DMEMLIB_DEBUG=1   (default in non-NDEBUG builds) — runtime invariants
 *   -DMEMLIB_DEBUG=0   — strip checks for release hot paths
 *
 * License: MIT.
 */
#ifndef MEMLIB_H
#define MEMLIB_H

#include <stddef.h>

#ifndef MEMLIB_DEBUG
#  ifdef NDEBUG
#    define MEMLIB_DEBUG 0
#  else
#    define MEMLIB_DEBUG 1
#  endif
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct MemArena MemArena;
typedef struct MemPool  MemPool;
typedef struct { void *data; size_t len, stride; } MemSlice;

MemArena *memlib_arena_create(size_t capacity_bytes);
void     *memlib_arena_alloc(MemArena *a, size_t bytes, size_t align);
char     *memlib_arena_strdup(MemArena *a, const char *s);
void      memlib_arena_reset(MemArena *a);
size_t    memlib_arena_used(const MemArena *a);
void      memlib_arena_destroy(MemArena *a);

MemPool  *memlib_pool_create(size_t slot_size, size_t n_slots);
void     *memlib_pool_alloc(MemPool *p);
void      memlib_pool_free(MemPool *p, void *obj);
size_t    memlib_pool_free_count(const MemPool *p);
void      memlib_pool_destroy(MemPool *p);

void     *memlib_slice_get(MemSlice s, size_t i);

#ifdef __cplusplus
}
#endif

/* ============================================================ */
#ifdef MEMLIB_IMPLEMENTATION
/* ============================================================ */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MEMLIB__REQUIRE(cond)  do { if (MEMLIB_DEBUG && !(cond)) { \
    fprintf(stderr, "memlib REQUIRE failed: %s at %s:%d\n", #cond, __FILE__, __LINE__); abort(); } } while (0)

static size_t memlib__align_up(size_t n, size_t a) { return (n + a - 1) & ~(a - 1); }

struct MemArena { char *base; size_t used, capacity; };

MemArena *memlib_arena_create(size_t cap) {
    MemArena *a = (MemArena *)malloc(sizeof(*a));
    if (!a) return NULL;
    cap = memlib__align_up(cap, 16);
    a->base = (char *)aligned_alloc(16, cap);
    if (!a->base) { free(a); return NULL; }
    a->used = 0; a->capacity = cap;
    return a;
}

void *memlib_arena_alloc(MemArena *a, size_t bytes, size_t align) {
    MEMLIB__REQUIRE(a != NULL);
    MEMLIB__REQUIRE(align >= 1 && (align & (align - 1)) == 0);
    size_t aligned = memlib__align_up(a->used, align);
    if (aligned + bytes > a->capacity) return NULL;
    void *p = a->base + aligned;
    a->used = aligned + bytes;
    return p;
}

char *memlib_arena_strdup(MemArena *a, const char *s) {
    MEMLIB__REQUIRE(s != NULL);
    size_t n = strlen(s) + 1;
    char *dst = (char *)memlib_arena_alloc(a, n, 1);
    if (!dst) return NULL;
    memcpy(dst, s, n);
    return dst;
}

void memlib_arena_reset(MemArena *a)        { MEMLIB__REQUIRE(a != NULL); a->used = 0; }
size_t memlib_arena_used(const MemArena *a) { MEMLIB__REQUIRE(a != NULL); return a->used; }
void memlib_arena_destroy(MemArena *a)      { if (!a) return; free(a->base); free(a); }

struct MemPool { void *slab; size_t slot_size, n_slots; void *free_head; };

MemPool *memlib_pool_create(size_t slot_size, size_t n_slots) {
    if (slot_size < sizeof(void *)) slot_size = sizeof(void *);
    slot_size = memlib__align_up(slot_size, sizeof(void *));
    MemPool *p = (MemPool *)malloc(sizeof(*p));
    if (!p) return NULL;
    p->slab = aligned_alloc(16, slot_size * n_slots);
    if (!p->slab) { free(p); return NULL; }
    p->slot_size = slot_size; p->n_slots = n_slots;
    char *cur = (char *)p->slab;
    for (size_t i = 0; i + 1 < n_slots; ++i) {
        *(void **)cur = cur + slot_size;
        cur += slot_size;
    }
    *(void **)cur = NULL;
    p->free_head = p->slab;
    return p;
}

void *memlib_pool_alloc(MemPool *p) {
    MEMLIB__REQUIRE(p != NULL);
    if (!p->free_head) return NULL;
    void *slot = p->free_head;
    p->free_head = *(void **)slot;
    return slot;
}

static int memlib__pool_owns(const MemPool *p, const void *obj) {
    const char *b = (const char *)p->slab;
    const char *o = (const char *)obj;
    if (o < b || o >= b + p->n_slots * p->slot_size) return 0;
    return ((size_t)(o - b)) % p->slot_size == 0;
}

void memlib_pool_free(MemPool *p, void *obj) {
    MEMLIB__REQUIRE(p != NULL);
    MEMLIB__REQUIRE(obj != NULL);
    MEMLIB__REQUIRE(memlib__pool_owns(p, obj));
    *(void **)obj = p->free_head;
    p->free_head = obj;
}

size_t memlib_pool_free_count(const MemPool *p) {
    size_t n = 0; void *cur = p->free_head;
    while (cur) { n++; cur = *(void **)cur; }
    return n;
}

void memlib_pool_destroy(MemPool *p) { if (!p) return; free(p->slab); free(p); }

void *memlib_slice_get(MemSlice s, size_t i) {
    MEMLIB__REQUIRE(s.data != NULL);
    MEMLIB__REQUIRE(i < s.len);
    return (char *)s.data + i * s.stride;
}

#endif /* MEMLIB_IMPLEMENTATION */
#endif /* MEMLIB_H */
