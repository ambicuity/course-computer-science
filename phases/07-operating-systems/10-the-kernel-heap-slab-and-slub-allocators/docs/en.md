# Lesson 10: The Kernel Heap — slab and slub allocators

## Why This Matters

The kernel allocates memory constantly: every `open()` creates a file object, every `fork()` clones a task structure, every network packet gets a buffer header. These objects are small (64–512 bytes) and allocated/freed millions of times per second. Using `malloc`-style general-purpose allocation for every one would cause massive internal fragmentation and cache pollution. The kernel needs a fast, per-object-type allocator that recycles fixed-size objects with zero initialization overhead.

## kmalloc and the Buddy Allocator

`kmalloc` is the kernel's general-purpose allocator. Underneath, it uses the **buddy allocator**:

```
Buddy allocator — power-of-2 block sizes

  Order 0:  4 KB    ┌──┐┌──┐┌──┐┌──┐┌──┐┌──┐┌──┐┌──┐
  Order 1:  8 KB    ┌──────┐┌──────┐┌──────┐┌──────┐
  Order 2:  16 KB   ┌──────────────┐┌──────────────┐
  Order 3:  32 KB   ┌──────────────────────────────┐
```

The buddy system maintains free lists for each power-of-2 order (0 through MAX_ORDER, typically 11). When you need 4 KB, it pops from order 0. When order 0 is empty, it splits an order-1 block into two order-0 "buddies." On free, adjacent buddies are **coalesced** back into larger blocks.

**Problem**: allocating a 64-byte `task_struct` from a 4 KB page wastes 97.5% of the page. Even with slabs, we need a higher-level strategy.

## The Slab Allocator (Bonwick 1994)

Jeff Bonwick's insight: the kernel reuses the same types of objects over and over. Instead of carving bytes from a general heap, **pre-allocate caches of fixed-size objects**.

```
Slab Cache: "task_struct"  (size = 512 bytes)
┌──────────────────────────────────────────────────┐
│  Slab page 0                                     │
│  ┌────┐┌────┐┌────┐┌────┐┌────┐ ... ┌────┐      │
│  │free││free││used││free││used│     │free│      │
│  └────┘└────┘└────┘└────┘└────┘     └────┘      │
├──────────────────────────────────────────────────┤
│  Slab page 1                                     │
│  ┌────┐┌────┐┌────┐┌────┐┌────┐ ... ┌────┐      │
│  │used││used││free││used││free│     │used│      │
│  └────┘└────┘└────┘└────┘└────┘     └────┘      │
└──────────────────────────────────────────────────┘
```

**Key concepts**:

- **Slab cache**: a named pool for one object type (e.g., `"task_struct"`, `"inode_cache"`, `"dentry"`).
- **Slab**: one or more contiguous page frames divided into equal-sized objects.
- **Free list**: a singly-linked list threading through free objects in a slab. Allocation = pop from list. Free = push to list. O(1).
- **Coloring**: objects are offset from the start of each slab page to spread cache-line usage across sets.

**Benefits**:

| Benefit | Why |
|---------|-----|
| O(1) alloc/free | Just pointer manipulation on the free list |
| Zero initialization | Reused objects retain their type; no memset needed |
| Cache-friendly | Objects of the same type sit together; hot fields stay in cache |
| Minimal fragmentation | Every byte in a slab belongs to exactly one object |

## The SLUB Allocator (Linux Default)

SLUB (Christoph Lameter, 2007) is a simplified slab design that replaced SLAB in Linux 2.6.22+:

```
SLAB (old)                    SLUB (default)
┌─────────────────────┐      ┌─────────────────────┐
│  Slab metadata:     │      │  No per-slab metadata│
│  - Free list header │      │  - Free list in      │
│  - Coloring table   │      │    first free object │
│  - Per-CPU cache    │      │  - Per-CPU partial   │
│  - Partial list     │      │    list              │
└─────────────────────┘      └─────────────────────┘
```

**What SLUB simplifies**:

1. **No per-slab metadata structure**: SLAB used a complex `struct slab` object per page. SLUB stores the free list pointer directly in the page's first word.
2. **No per-object metadata**: SLAB tracked object state in a bitmap. SLUB uses the free pointer chain embedded in the objects themselves.
3. **Better NUMA**: SLUB's `kmem_cache` tracks per-node partial lists directly, avoiding cross-node allocations.
4. **Debug-friendly**: `SLUB_DEBUG` enables poison patterns and tracking without changing the allocator's structure.

**SLUB in production**: Linux, FreeBSD, and Android all use SLUB. Run `slabinfo` or `cat /proc/slabinfo` to see active caches.

## The Buddy Allocator — Deeper Look

When a slab cache needs a new page, it asks the buddy allocator:

```
Free lists (example for MAX_ORDER = 4):

  Order 0 (4KB):   [page_a] -> [page_b] -> NULL
  Order 1 (8KB):   NULL (empty)
  Order 2 (16KB):  [page_c] -> NULL
  Order 3 (32KB):  NULL

Allocating 8KB (order 1):
  1. Order 1 list is empty
  2. Split one order-2 block (page_c) into two order-1 blocks
  3. Give one to the caller, put the other on order 1's list
  4. Result: Order 1 = [page_d], Order 2 = NULL

Freeing 8KB (page_d):
  1. Check if buddy page is also free
  2. If yes: merge into one order-2 block
  3. If no: add to order 1's free list
```

This coalescing is what keeps external fragmentation low for large allocations.

## Build It

We'll build a simplified slab allocator: create named caches, allocate/free objects from them, and benchmark against `malloc`.

### Step 1: Data Structures

```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define SLAB_PAGES    4       /* pages per slab */
#define PAGE_SIZE     4096
#define SLAB_SIZE     (SLAB_PAGES * PAGE_SIZE)
#define MAX_CACHES    32
#define MAX_SLABS     128

typedef struct FreeNode {
    struct FreeNode *next;
} FreeNode;

typedef struct {
    void       *base;          /* start of page memory */
    size_t      total_objects; /* objects per slab */
    size_t      free_count;    /* free objects remaining */
    FreeNode   *free_list;     /* head of free list */
} Slab;

typedef struct {
    char        name[64];
    size_t      obj_size;      /* size of each object */
    size_t      slabs_count;   /* number of slab pages */
    Slab        slabs[MAX_SLABS];
} SlabCache;
```

### Step 2: Cache Creation

```c
static SlabCache *slab_cache_create(const char *name, size_t obj_size) {
    SlabCache *cache = calloc(1, sizeof(SlabCache));
    if (!cache) return NULL;

    strncpy(cache->name, name, sizeof(cache->name) - 1);
    cache->obj_size = (obj_size < sizeof(FreeNode)) ? sizeof(FreeNode) : obj_size;
    cache->slabs_count = 0;
    return cache;
}
```

### Step 3: Growing the Cache

When the free list is empty, we allocate a new slab page:

```c
static int slab_grow(SlabCache *cache) {
    if (cache->slabs_count >= MAX_SLABS) return -1;

    void *mem = malloc(SLAB_SIZE);
    if (!mem) return -1;

    Slab *slab = &cache->slabs[cache->slabs_count++];
    slab->base = mem;
    slab->total_objects = SLAB_SIZE / cache->obj_size;
    slab->free_count = slab->total_objects;
    slab->free_list = NULL;

    /* Build free list by chaining objects */
    char *ptr = (char *)mem;
    for (size_t i = 0; i < slab->total_objects; i++) {
        FreeNode *node = (FreeNode *)ptr;
        node->next = slab->free_list;
        slab->free_list = node;
        ptr += cache->obj_size;
    }

    return 0;
}
```

### Step 4: Allocation and Free

```c
static void *slab_alloc(SlabCache *cache) {
    /* Try to find a slab with free objects */
    for (size_t i = 0; i < cache->slabs_count; i++) {
        Slab *slab = &cache->slabs[i];
        if (slab->free_count > 0) {
            FreeNode *node = slab->free_list;
            slab->free_list = node->next;
            slab->free_count--;
            return (void *)node;
        }
    }

    /* No free objects — grow the cache */
    if (slab_grow(cache) < 0) return NULL;

    /* Try again with the new slab */
    Slab *slab = &cache->slabs[cache->slabs_count - 1];
    FreeNode *node = slab->free_list;
    slab->free_list = node->next;
    slab->free_count--;
    return (void *)node;
}

static void slab_free(SlabCache *cache, void *ptr) {
    if (!ptr) return;

    /* Find which slab this pointer belongs to */
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
```

### Step 5: Cleanup

```c
static void slab_cache_destroy(SlabCache *cache) {
    for (size_t i = 0; i < cache->slabs_count; i++) {
        free(cache->slabs[i].base);
    }
    free(cache);
}
```

### Step 6: Benchmark

```c
static void benchmark(const char *label, int count, int use_slab) {
    struct timespec start, end;
    clock_gettime(CLOCK_MONOTONIC, &start);

    if (use_slab) {
        SlabCache *cache = slab_cache_create("bench", 64);
        void *ptrs[count];
        for (int i = 0; i < count; i++)
            ptrs[i] = slab_alloc(cache);
        for (int i = 0; i < count; i++)
            slab_free(cache, ptrs[i]);
        slab_cache_destroy(cache);
    } else {
        void *ptrs[count];
        for (int i = 0; i < count; i++)
            ptrs[i] = malloc(64);
        for (int i = 0; i < count; i++)
            free(ptrs[i]);
    }

    clock_gettime(CLOCK_MONOTONIC, &end);
    double ms = (end.tv_sec - start.tv_sec) * 1000.0
              + (end.tv_nsec - start.tv_nsec) / 1e6;
    printf("%s: %d alloc+free pairs in %.2f ms\n", label, count, ms);
}
```

### Full Program

```c
int main(void) {
    printf("Simplified Slab Allocator\n");
    printf("=========================\n\n");

    /* Demo: allocate objects from a slab cache */
    SlabCache *cache = slab_cache_create("demo", 128);
    void *a = slab_alloc(cache);
    void *b = slab_alloc(cache);
    void *c = slab_alloc(cache);
    printf("Allocated 3 objects from '%s': a=%p b=%p c=%p\n",
           cache->name, a, b, c);
    slab_free(cache, b);
    void *d = slab_alloc(cache);
    printf("Freed b, re-allocated: d=%p (should reuse b's slot)\n", d);
    slab_free(cache, a);
    slab_free(cache, c);
    slab_free(cache, d);
    slab_cache_destroy(cache);

    printf("\n--- Benchmark: 1M alloc/free pairs ---\n");
    benchmark("malloc/free", 1000000, 0);
    benchmark("slab alloc/free", 1000000, 1);

    return 0;
}
```

**Compile and run**: `gcc -O2 -o slab main.c && ./slab`

## Use It

In the Linux kernel, you create slab caches with `kmem_cache_create()` and allocate from them with `kmem_cache_alloc()`:

```c
struct kmem_cache *task_struct_cache;
/* ... in init ... */
task_struct_cache = kmem_cache_create("task_struct",
    sizeof(struct task_struct), 0, SLAB_PANIC, NULL);

/* Allocating a task_struct */
struct task_struct *tsk = kmem_cache_alloc(task_struct_cache, GFP_KERNEL);

/* Freeing */
kmem_cache_free(task_struct_cache, tsk);
```

Every `task_struct`, `inode`, `dentry`, `sock` etc. in the kernel gets its own slab cache. The `kmem_cache` is the production equivalent of our `SlabCache` — but with per-CPU freelocks, NUMA awareness, and `SLUB_DEBUG` poisoning.

## Read the Source

- `mm/slub.c` — SLUB allocator implementation (Linux 6.x). Look at `kmem_cache_alloc()` and `new_slab()`.
- `/proc/slabinfo` — live view of all active slab caches on a running Linux system.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained reference snippet you can reuse in later phases.**

## Exercises

### Level 1 — Recall

Explain why the buddy allocator alone is insufficient for kernel memory allocation. What problem does the slab allocator solve?

### Level 2 — Application

Modify the slab allocator to support **object constructors and destructors**: when a slab page is allocated, run a constructor on every object; when freed, run a destructor. This mirrors the Linux `ctor`/`dtor` callback feature of `kmem_cache_create()`.

### Level 3 — Build

Add **per-CPU freelists** to the slab allocator. Each CPU gets a small array of pre-allocated objects. `slab_alloc` pops from the per-CPU list first (no locking needed). When empty, it grabs a batch from the shared slab and refills. Benchmark the improvement under concurrent allocation from multiple threads.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Slab cache | "A pool for one object type" | A named `kmem_cache` containing slabs of fixed-size objects with a free list |
| Slab | "A page of pre-allocated objects" | One or more contiguous page frames divided into equal objects, managed by a free list |
| Free list | "Stack of free objects" | A singly-linked list through free objects; alloc = pop, free = push, both O(1) |
| Buddy allocator | "Power-of-2 block allocator" | Manages free pages in power-of-2 orders; coalesces adjacent buddies on free |
| SLUB | "Simplified slab" | Linux's default allocator; no per-slab metadata, better NUMA support, debuggable |
| Coloring | "Cache offset trick" | Staggering object start offsets across slabs to distribute cache-line usage |

## Further Reading

- Jeff Bonwick, "The Slab Allocator: An Object-Caching Kernel Memory Allocator" (1994 USENIX)
- Christoph Lameter, "SLUB: The Unqueued Slab Cache Allocator" (2007)
- Linux kernel docs: `Documentation/mm/slub.rst`
