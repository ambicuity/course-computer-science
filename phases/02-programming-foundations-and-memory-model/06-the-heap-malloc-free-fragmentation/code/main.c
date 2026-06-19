/* main.c — bump allocator + free-list allocator (first-fit, with coalescing).
 *
 * Build: gcc -O0 -g main.c -o main
 * Run:   ./main
 */

#include <stdio.h>
#include <stdint.h>
#include <stddef.h>
#include <string.h>
#include <assert.h>

/* ── Bump allocator ────────────────────────────────────────────── */

#define BUMP_ARENA_SIZE (1 << 20)
static char bump_arena[BUMP_ARENA_SIZE];
static size_t bump_offset = 0;

void *bump_alloc(size_t size) {
    /* Align to 8 bytes */
    size = (size + 7) & ~7;
    if (bump_offset + size > BUMP_ARENA_SIZE) return NULL;
    void *p = bump_arena + bump_offset;
    bump_offset += size;
    return p;
}

void bump_reset(void) { bump_offset = 0; }


/* ── Free-list allocator (first-fit + coalesce) ──────────────────── */

#define HEAP_SIZE (1 << 20)
static char heap_buf[HEAP_SIZE];

typedef struct Block {
    size_t size;             /* payload size */
    int    free;
    struct Block *next;
} Block;

#define BLOCK_OVERHEAD ((size_t)sizeof(Block))

static Block *head = NULL;

static void heap_init(void) {
    head = (Block *)heap_buf;
    head->size = HEAP_SIZE - BLOCK_OVERHEAD;
    head->free = 1;
    head->next = NULL;
}

void *my_malloc(size_t size) {
    if (!head) heap_init();
    if (size == 0) return NULL;
    size = (size + 7) & ~7;       /* 8-byte align */

    Block *cur = head;
    while (cur) {
        if (cur->free && cur->size >= size) {
            /* Split if there's room for a header + at least 16 bytes */
            if (cur->size >= size + BLOCK_OVERHEAD + 16) {
                Block *next_blk = (Block *)((char *)cur + BLOCK_OVERHEAD + size);
                next_blk->size = cur->size - size - BLOCK_OVERHEAD;
                next_blk->free = 1;
                next_blk->next = cur->next;
                cur->size = size;
                cur->next = next_blk;
            }
            cur->free = 0;
            return (char *)cur + BLOCK_OVERHEAD;
        }
        cur = cur->next;
    }
    return NULL;
}

static void coalesce(void) {
    Block *cur = head;
    while (cur && cur->next) {
        if (cur->free && cur->next->free) {
            cur->size += BLOCK_OVERHEAD + cur->next->size;
            cur->next = cur->next->next;
        } else {
            cur = cur->next;
        }
    }
}

void my_free(void *p) {
    if (!p) return;
    Block *blk = (Block *)((char *)p - BLOCK_OVERHEAD);
    blk->free = 1;
    coalesce();
}


/* ── Diagnostics ────────────────────────────────────────────────── */

static void heap_dump(const char *label) {
    printf("%s:\n", label);
    Block *cur = head;
    size_t total_free = 0, total_used = 0;
    int blocks = 0;
    while (cur) {
        printf("  [%c]  size=%zu\n", cur->free ? 'F' : 'U', cur->size);
        if (cur->free) total_free += cur->size;
        else           total_used += cur->size;
        blocks++;
        cur = cur->next;
    }
    printf("  → %d blocks; used=%zu, free=%zu\n", blocks, total_used, total_free);
}


/* ── Demo ───────────────────────────────────────────────────────── */

int main(void) {
    printf("== Bump allocator: O(1) per alloc, no individual free ==\n");
    void *a = bump_alloc(100);
    void *b = bump_alloc(200);
    void *c = bump_alloc(50);
    printf("  a=%p (100B), b=%p (200B), c=%p (50B)\n", a, b, c);
    printf("  offsets: %ld, %ld, %ld   (8-byte aligned)\n",
           (char*)a - bump_arena, (char*)b - bump_arena, (char*)c - bump_arena);
    bump_reset();

    printf("\n== Free-list allocator with coalescing ==\n");
    heap_init();
    heap_dump("Initial");

    void *p1 = my_malloc(100);
    void *p2 = my_malloc(200);
    void *p3 = my_malloc(50);
    heap_dump("\nAfter 3 allocs");

    my_free(p2);
    heap_dump("\nAfter free(p2)");

    my_free(p1);
    heap_dump("\nAfter free(p1) — should coalesce with p2's region");

    my_free(p3);
    heap_dump("\nAfter free(p3) — should coalesce back into one big block");

    /* Stress test: alloc 100 chunks, free every other, alloc 50 more */
    printf("\n== Stress: alloc 100, free every other, alloc 50 ==\n");
    heap_init();
    void *ptrs[100];
    for (int i = 0; i < 100; i++) ptrs[i] = my_malloc(64);
    for (int i = 0; i < 100; i += 2) my_free(ptrs[i]);
    int new_allocs = 0;
    for (int i = 0; i < 50; i++) {
        if (my_malloc(32)) new_allocs++;
    }
    printf("  %d / 50 small allocs succeeded after fragmentation\n", new_allocs);
    return 0;
}
