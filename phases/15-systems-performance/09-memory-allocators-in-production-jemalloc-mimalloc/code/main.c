#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <assert.h>
#include <time.h>

#define KiB(x) ((x) * 1024)
#define MiB(x) ((x) * 1024 * 1024)

/* ── Bump Allocator ────────────────────────────────────────────────── */

typedef struct {
    uint8_t *base;
    size_t   offset;
    size_t   capacity;
} bump_allocator;

static void bump_init(bump_allocator *b, size_t capacity) {
    b->base = (uint8_t *)malloc(capacity);
    if (!b->base) {
        perror("bump_init malloc");
        exit(1);
    }
    b->offset   = 0;
    b->capacity  = capacity;
}

static void *bump_alloc(bump_allocator *b, size_t size) {
    size = (size + 15) & ~(size_t)15; /* 16-byte alignment */
    if (b->offset + size > b->capacity) return NULL;
    void *ptr = b->base + b->offset;
    b->offset += size;
    return ptr;
}

static size_t bump_used(bump_allocator *b) {
    return b->offset;
}

static double bump_utilization(bump_allocator *b) {
    return b->capacity > 0 ? (double)b->offset / b->capacity : 0.0;
}

static void bump_destroy(bump_allocator *b) {
    free(b->base);
    b->base = NULL;
    b->offset = b->capacity = 0;
}

/* ── Free-List Allocator ───────────────────────────────────────────── */

#define ALIGN_UP(x, a) (((x) + (a) - 1) & ~((a) - 1))

typedef struct free_block {
    size_t            size;
    struct free_block *next;
} free_block;

typedef struct {
    uint8_t     *region;
    size_t       region_size;
    free_block  *free_list;
} freelist_allocator;

static void fl_init(freelist_allocator *fl, size_t size) {
    fl->region = (uint8_t *)malloc(size);
    if (!fl->region) { perror("fl_init malloc"); exit(1); }
    fl->region_size = size;
    fl->free_list = (free_block *)fl->region;
    fl->free_list->size = size;
    fl->free_list->next = NULL;
}

static void *fl_alloc(freelist_allocator *fl, size_t size) {
    size = ALIGN_UP(size + sizeof(free_block), 16);
    free_block **prev = &fl->free_list;
    free_block  *blk  = fl->free_list;
    free_block  *best = NULL;
    free_block **best_prev = NULL;
    while (blk) {
        if (blk->size >= size) {
            if (!best || blk->size < best->size) {
                best = blk;
                best_prev = prev;
            }
        }
        prev = &blk->next;
        blk  = blk->next;
    }
    if (!best) return NULL;

    if (best->size >= size + sizeof(free_block) + 16) {
        free_block *split = (free_block *)((uint8_t *)best + size);
        split->size = best->size - size;
        split->next = best->next;
        *best_prev = split;
        best->size = size;
    } else {
        *best_prev = best->next;
    }
    return (void *)((uint8_t *)best + sizeof(free_block));
}

static void fl_free(freelist_allocator *fl, void *ptr) {
    if (!ptr) return;
    free_block *blk = (free_block *)((uint8_t *)ptr - sizeof(free_block));
    blk->next = fl->free_list;
    fl->free_list = blk;
}

static size_t fl_fragmentation(freelist_allocator *fl) {
    size_t total_free = 0;
    size_t count = 0;
    size_t largest = 0;
    free_block *b = fl->free_list;
    while (b) {
        total_free += b->size;
        if (b->size > largest) largest = b->size;
        count++;
        b = b->next;
    }
    (void)count;
    if (total_free == 0) return 0;
    return total_free - largest;
}

static void fl_destroy(freelist_allocator *fl) {
    free(fl->region);
    fl->region = NULL;
    fl->free_list = NULL;
    fl->region_size = 0;
}

/* ── Timing Helper ─────────────────────────────────────────────────── */

static double now_sec(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

/* ── Benchmark: Sequential Allocation ──────────────────────────────── */

static void bench_sequential_malloc(int n) {
    double t0 = now_sec();
    void **ptrs = (void **)malloc((size_t)n * sizeof(void *));
    for (int i = 0; i < n; i++) {
        size_t sz = (size_t)(8 + (i % 248) * 8);
        ptrs[i] = malloc(sz);
        if (!ptrs[i]) { fprintf(stderr, "malloc failed at %d\n", i); exit(1); }
    }
    for (int i = 0; i < n; i++) free(ptrs[i]);
    free(ptrs);
    double elapsed = now_sec() - t0;
    printf("  %-30s %8.3f ms  (%.0f ops/sec)\n",
           "malloc sequential", elapsed * 1000, n / elapsed);
}

static void bench_sequential_bump(bump_allocator *b, int n) {
    double t0 = now_sec();
    for (int i = 0; i < n; i++) {
        size_t sz = (size_t)(8 + (i % 248) * 8);
        if (!bump_alloc(b, sz)) {
            fprintf(stderr, "bump alloc overflow at %d\n", i);
            break;
        }
    }
    double elapsed = now_sec() - t0;
    printf("  %-30s %8.3f ms  (%.0f ops/sec)\n",
           "bump sequential", elapsed * 1000, n / elapsed);
}

static void bench_sequential_freelist(freelist_allocator *fl, int n) {
    double t0 = now_sec();
    void **ptrs = (void **)malloc((size_t)n * sizeof(void *));
    for (int i = 0; i < n; i++) {
        size_t sz = (size_t)(8 + (i % 248) * 8);
        ptrs[i] = fl_alloc(fl, sz);
        if (!ptrs[i]) { fprintf(stderr, "freelist failed at %d\n", i); break; }
    }
    for (int i = 0; i < n; i++) fl_free(fl, ptrs[i]);
    free(ptrs);
    double elapsed = now_sec() - t0;
    printf("  %-30s %8.3f ms  (%.0f ops/sec)\n",
           "freelist sequential", elapsed * 1000, n / elapsed);
}

/* ── Benchmark: Random-Size Churn ──────────────────────────────────── */

static void bench_churn_malloc(int n) {
    double t0 = now_sec();
    for (int i = 0; i < n; i++) {
        size_t sz = (size_t)(8 + (rand() % 248) * 8);
        void *p = malloc(sz);
        if (!p) { fprintf(stderr, "malloc churn failed\n"); exit(1); }
        free(p);
    }
    double elapsed = now_sec() - t0;
    printf("  %-30s %8.3f ms  (%.0f ops/sec)\n",
           "malloc churn", elapsed * 1000, n / elapsed);
}

static void bench_churn_freelist(freelist_allocator *fl, int n) {
    double t0 = now_sec();
    for (int i = 0; i < n; i++) {
        size_t sz = (size_t)(8 + (rand() % 248) * 8);
        void *p = fl_alloc(fl, sz);
        if (!p) { fprintf(stderr, "freelist churn failed\n"); break; }
        fl_free(fl, p);
    }
    double elapsed = now_sec() - t0;
    printf("  %-30s %8.3f ms  (%.0f ops/sec)\n",
           "freelist churn", elapsed * 1000, n / elapsed);
}

/* ── Benchmark: Fragmentation Measurement ───────────────────────────── */

static void bench_fragmentation(void) {
    enum { POOL = 10000 };
    freelist_allocator fl;
    fl_init(&fl, MiB(4));

    void *ptrs[POOL];
    size_t sizes[POOL];
    for (int i = 0; i < POOL; i++) {
        sizes[i] = (size_t)(8 + (rand() % 60) * 8);
        ptrs[i] = fl_alloc(&fl, sizes[i]);
        if (!ptrs[i]) { fprintf(stderr, "alloc failed at %d\n", i); break; }
    }

    /* free every other allocation to create holes */
    for (int i = 0; i < POOL; i += 2) {
        fl_free(&fl, ptrs[i]);
        ptrs[i] = NULL;
    }

    size_t frag = fl_fragmentation(&fl);
    size_t total_free = 0;
    free_block *b = fl.free_list;
    while (b) { total_free += b->size; b = b->next; }

    printf("  %-30s free=%zu KB  fragmented=%zu KB  (%.1f%%)\n",
           "external fragmentation",
           total_free / 1024, frag / 1024,
           total_free > 0 ? (double)frag / total_free * 100 : 0);

    /* cleanup remaining */
    for (int i = 0; i < POOL; i++) {
        if (ptrs[i]) fl_free(&fl, ptrs[i]);
    }
    fl_destroy(&fl);
}

/* ── Bump Allocator Utilization Demo ───────────────────────────────── */

static void demo_bump_utilization(void) {
    bump_allocator b;
    bump_init(&b, KiB(64));

    const char *msgs[] = {
        "hello", "world", "memory", "allocators", "performance"
    };
    for (int i = 0; i < 5; i++) {
        char *p = (char *)bump_alloc(&b, strlen(msgs[i]) + 1);
        if (p) strcpy(p, msgs[i]);
    }
    printf("  %-30s used=%zu/%zu  utilization=%.1f%%\n",
           "bump utilization",
           bump_used(&b), b.capacity,
           bump_utilization(&b) * 100);
    bump_destroy(&b);
}

/* ── Main ──────────────────────────────────────────────────────────── */

int main(void) {
    srand((unsigned)time(NULL));
    int N = 20000;

    printf("=== Memory Allocator Benchmarks (N=%d) ===\n\n", N);

    /* 1. Sequential allocation + free */
    printf("[1] Sequential alloc+free (size 8-2000, varying by index)\n");
    bench_sequential_malloc(N);

    bump_allocator bump;
    bump_init(&bump, MiB(64));
    bench_sequential_bump(&bump, N);
    bump_destroy(&bump);

    freelist_allocator fl_seq;
    fl_init(&fl_seq, MiB(64));
    bench_sequential_freelist(&fl_seq, N);
    fl_destroy(&fl_seq);

    /* 2. Churn: alloc+free random sizes */
    printf("\n[2] Churn: alloc+free random sizes (8-2000 bytes)\n");
    bench_churn_malloc(N);

    freelist_allocator fl_churn;
    fl_init(&fl_churn, MiB(64));
    bench_churn_freelist(&fl_churn, N);
    fl_destroy(&fl_churn);

    /* 3. Fragmentation measurement */
    printf("\n[3] Fragmentation: free every other block\n");
    bench_fragmentation();

    /* 4. Bump utilization */
    printf("\n[4] Bump allocator utilization\n");
    demo_bump_utilization();

    printf("\n=== Done ===\n");
    return 0;
}