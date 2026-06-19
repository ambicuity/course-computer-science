/*
 * The Kernel Heap — slab and slub allocators
 * Phase 07 — Operating Systems
 *
 * Simplified slab allocator: named caches of fixed-size objects.
 * Compile: gcc -O2 -o slab main.c
 * Run:     ./slab
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define SLAB_PAGES    4
#define PAGE_SIZE     4096
#define SLAB_SIZE     (SLAB_PAGES * PAGE_SIZE)
#define MAX_CACHES    32
#define MAX_SLABS     128

/* ---------- Data Structures ---------- */

typedef struct FreeNode {
    struct FreeNode *next;
} FreeNode;

typedef struct {
    void       *base;
    size_t      total_objects;
    size_t      free_count;
    FreeNode   *free_list;
} Slab;

typedef struct {
    char        name[64];
    size_t      obj_size;
    size_t      slabs_count;
    Slab        slabs[MAX_SLABS];
} SlabCache;

/* ---------- Cache Lifecycle ---------- */

static SlabCache *slab_cache_create(const char *name, size_t obj_size) {
    SlabCache *cache = calloc(1, sizeof(SlabCache));
    if (!cache) return NULL;

    strncpy(cache->name, name, sizeof(cache->name) - 1);
    /* Ensure object is at least large enough for a free-list pointer */
    cache->obj_size = (obj_size < sizeof(FreeNode)) ? sizeof(FreeNode) : obj_size;
    cache->slabs_count = 0;
    return cache;
}

static int slab_grow(SlabCache *cache) {
    if (cache->slabs_count >= MAX_SLABS) return -1;

    void *mem = malloc(SLAB_SIZE);
    if (!mem) return -1;

    Slab *slab = &cache->slabs[cache->slabs_count++];
    slab->base = mem;
    slab->total_objects = SLAB_SIZE / cache->obj_size;
    slab->free_count = slab->total_objects;
    slab->free_list = NULL;

    /* Build free list: chain objects in reverse so alloc pops front */
    char *ptr = (char *)mem;
    for (size_t i = 0; i < slab->total_objects; i++) {
        FreeNode *node = (FreeNode *)ptr;
        node->next = slab->free_list;
        slab->free_list = node;
        ptr += cache->obj_size;
    }

    return 0;
}

/* ---------- Alloc / Free ---------- */

static void *slab_alloc(SlabCache *cache) {
    for (size_t i = 0; i < cache->slabs_count; i++) {
        Slab *slab = &cache->slabs[i];
        if (slab->free_count > 0) {
            FreeNode *node = slab->free_list;
            slab->free_list = node->next;
            slab->free_count--;
            return (void *)node;
        }
    }

    /* No free objects — grow */
    if (slab_grow(cache) < 0) return NULL;

    Slab *slab = &cache->slabs[cache->slabs_count - 1];
    FreeNode *node = slab->free_list;
    slab->free_list = node->next;
    slab->free_count--;
    return (void *)node;
}

static void slab_free(SlabCache *cache, void *ptr) {
    if (!ptr) return;

    for (size_t i = 0; i < cache->slabs_count; i++) {
        Slab *slab = &cache->slabs[i];
        char *base = (char *)slab->base;
        char *end  = base + SLAB_SIZE;
        if ((char *)ptr >= base && (char *)ptr < end) {
            FreeNode *node = (FreeNode *)ptr;
            node->next = slab->free_list;
            slab->free_list = node;
            slab->free_count++;
            return;
        }
    }
}

static void slab_cache_destroy(SlabCache *cache) {
    for (size_t i = 0; i < cache->slabs_count; i++) {
        free(cache->slabs[i].base);
    }
    free(cache);
}

/* ---------- Benchmark ---------- */

static void benchmark(const char *label, int count, int use_slab) {
    struct timespec start, end;
    clock_gettime(CLOCK_MONOTONIC, &start);

    if (use_slab) {
        SlabCache *cache = slab_cache_create("bench", 64);
        void **ptrs = malloc(count * sizeof(void *));
        for (int i = 0; i < count; i++)
            ptrs[i] = slab_alloc(cache);
        for (int i = 0; i < count; i++)
            slab_free(cache, ptrs[i]);
        free(ptrs);
        slab_cache_destroy(cache);
    } else {
        void **ptrs = malloc(count * sizeof(void *));
        for (int i = 0; i < count; i++)
            ptrs[i] = malloc(64);
        for (int i = 0; i < count; i++)
            free(ptrs[i]);
        free(ptrs);
    }

    clock_gettime(CLOCK_MONOTONIC, &end);
    double ms = (end.tv_sec - start.tv_sec) * 1000.0
              + (end.tv_nsec - start.tv_nsec) / 1e6;
    printf("%s: %d alloc+free pairs in %.2f ms\n", label, count, ms);
}

/* ---------- Main ---------- */

int main(void) {
    printf("Simplified Slab Allocator\n");
    printf("=========================\n\n");

    /* Demo: basic alloc/free */
    SlabCache *cache = slab_cache_create("demo", 128);
    void *a = slab_alloc(cache);
    void *b = slab_alloc(cache);
    void *c = slab_alloc(cache);
    printf("Allocated 3 objects from '%s':\n", cache->name);
    printf("  a = %p\n", a);
    printf("  b = %p\n", b);
    printf("  c = %p\n", c);

    slab_free(cache, b);
    void *d = slab_alloc(cache);
    printf("\nFreed b, re-allocated d = %p\n", d);
    printf("  (d reuses b's slot: %s)\n",
           d == b ? "yes" : "no — allocated from different slab page");

    slab_free(cache, a);
    slab_free(cache, c);
    slab_free(cache, d);

    /* Demo: auto-grow */
    printf("\nSlab info for '%s': %zu slab pages, %zu objects per slab\n",
           cache->name, cache->slabs_count,
           cache->slabs[0].total_objects);
    slab_cache_destroy(cache);

    /* Benchmark */
    printf("\n--- Benchmark: 1M alloc/free pairs (64-byte objects) ---\n");
    benchmark("malloc/free    ", 1000000, 0);
    benchmark("slab alloc/free", 1000000, 1);

    return 0;
}
