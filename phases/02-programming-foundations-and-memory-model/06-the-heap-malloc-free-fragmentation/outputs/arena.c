/* arena.c — minimal arena allocator.
 *
 * Pattern: allocate from a single buffer; free everything at once with
 * `arena_reset`. Useful for short-lived workloads (per-request, per-frame,
 * per-compile-pass).
 *
 * Build:  gcc arena.c -o arena_demo
 * Run:    ./arena_demo
 */

#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>

typedef struct {
    char  *base;
    size_t size;
    size_t offset;
} Arena;

void arena_init(Arena *a, size_t size) {
    a->base = malloc(size);
    a->size = size;
    a->offset = 0;
}

void *arena_alloc(Arena *a, size_t size) {
    size = (size + 15) & ~15;       /* 16-byte align (good for any primitive) */
    if (a->offset + size > a->size) return NULL;
    void *p = a->base + a->offset;
    a->offset += size;
    return p;
}

void *arena_zalloc(Arena *a, size_t size) {
    void *p = arena_alloc(a, size);
    if (p) memset(p, 0, size);
    return p;
}

void arena_reset(Arena *a) {
    a->offset = 0;
}

void arena_destroy(Arena *a) {
    free(a->base);
    a->base = NULL;
}

#ifdef ARENA_DEMO
int main(void) {
    Arena a;
    arena_init(&a, 1 << 20);  /* 1 MB */

    int   *xs = arena_alloc(&a, 100 * sizeof(int));
    char  *s  = arena_alloc(&a, 64);
    float *fs = arena_alloc(&a, 50 * sizeof(float));

    for (int i = 0; i < 100; ++i) xs[i] = i * 2;
    snprintf(s, 64, "hello arena");
    for (int i = 0; i < 50; ++i) fs[i] = i / 2.0f;

    printf("xs[7] = %d\n", xs[7]);
    printf("s = %s\n", s);
    printf("fs[10] = %.2f\n", fs[10]);
    printf("offset after 3 allocs: %zu bytes\n", a.offset);

    arena_reset(&a);
    printf("after reset: offset = %zu\n", a.offset);

    arena_destroy(&a);
    return 0;
}
#endif
