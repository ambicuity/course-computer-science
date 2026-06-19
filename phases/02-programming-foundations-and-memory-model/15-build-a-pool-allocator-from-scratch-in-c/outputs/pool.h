/* pool.h — header-only fixed-size object pool allocator.
 *
 * Define POOL_IMPLEMENTATION in exactly one .c file before #include'ing.
 * Build with -fsanitize=address for use-after-free detection.
 *
 * Usage:
 *   #define POOL_IMPLEMENTATION
 *   #include "pool.h"
 *
 *   Pool *p = pool_create(sizeof(MyObj), 1024);
 *   MyObj *o = pool_alloc(p);
 *   ... use o ...
 *   pool_free(p, o);
 *   pool_destroy(p);
 */
#ifndef POOL_H
#define POOL_H

#include <stddef.h>

typedef struct Pool Pool;

Pool   *pool_create (size_t slot_size, size_t n_slots);
void   *pool_alloc  (Pool *p);
void    pool_free   (Pool *p, void *obj);
void    pool_destroy(Pool *p);
size_t  pool_free_count(const Pool *p);

/* When defined at build time, poison freed slots with 0xDD so use-after-free
 * shows obvious garbage. */
/* #define POOL_POISON_ON_FREE */

#endif /* POOL_H */


#ifdef POOL_IMPLEMENTATION
#include <stdlib.h>
#include <string.h>

struct Pool {
    void  *slab;
    size_t slot_size;
    size_t n_slots;
    void  *free_head;
};

static size_t _pool_round_up(size_t x, size_t align) {
    return (x + align - 1) & ~(align - 1);
}

Pool *pool_create(size_t slot_size, size_t n_slots) {
    if (slot_size < sizeof(void *)) slot_size = sizeof(void *);
    slot_size = _pool_round_up(slot_size, sizeof(void *));

    Pool *p = (Pool *)malloc(sizeof(*p));
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

void *pool_alloc(Pool *p) {
    if (!p || !p->free_head) return NULL;
    void *slot = p->free_head;
    p->free_head = *(void **)slot;
    return slot;
}

void pool_free(Pool *p, void *obj) {
    if (!p || !obj) return;
#ifdef POOL_POISON_ON_FREE
    memset(obj, 0xDD, p->slot_size);
#endif
    *(void **)obj = p->free_head;
    p->free_head = obj;
}

void pool_destroy(Pool *p) {
    if (!p) return;
    free(p->slab);
    free(p);
}

size_t pool_free_count(const Pool *p) {
    if (!p) return 0;
    size_t n = 0;
    void *cur = p->free_head;
    while (cur) { n++; cur = *(void **)cur; }
    return n;
}
#endif /* POOL_IMPLEMENTATION */
