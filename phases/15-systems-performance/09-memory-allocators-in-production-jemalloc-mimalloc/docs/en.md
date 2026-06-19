# Memory Allocators in Production — jemalloc, mimalloc

> Every program allocates memory. Most never think about *how*. When you do, everything changes.

**Type:** Learn
**Languages:** C
**Prerequisites:** Phase 15 lessons 01–08
**Time:** ~75 minutes

## Learning Objectives

- Understand how `malloc` works internally: bump allocators, free lists, `sbrk`/`mmap`
- Distinguish internal vs external fragmentation and measure both
- Explain slab allocator design and why size classes matter
- Describe jemalloc's architecture: arenas, tcache, bins
- Describe mimalloc's architecture: segments, pages, free lists
- Benchmark allocators and choose the right one for a workload
- Know when a custom allocator beats the system allocator

## The Problem

Phase 15 is about knowing the machine. Memory allocation is the single hottest path in most programs — every data structure, every string, every object passes through an allocator. Yet most developers treat `malloc` as a black box.

This lesson opens that box. Without understanding allocators, you cannot diagnose fragmentation bugs, you cannot tune for throughput, and you cannot reason about latency spikes caused by lock contention in the heap.

The phase capstone — a profile-guided optimization walkthrough — requires you to read allocator profiles and act on them. This lesson gives you the vocabulary and mental models to do that.

## How malloc Works

### The Bump Allocator

The simplest allocator possible. Maintain a pointer to the next free byte. On allocation, advance the pointer by the requested size. On free, do nothing.

```
+---+---+---+---+---+---+---+---+
| A | B | C |               free|
+---+---+---+---+---+---+---+---+
            ^next

allocate(32) → returns next, advances next by 32
free(A)      → no-op (cannot reclaim)
```

**Properties:** O(1) allocate, O(1) free (no-op), zero fragmentation at allocation time, impossible to reuse freed memory. Only suitable for arena-style lifetimes where you free everything at once.

This is what `mmap`-backed arena allocators use: allocate forward, `munmap` the whole region when done.

### Free-List Allocator

When you need to reclaim individual allocations, you need a free list. The classic design:

1. Each free block stores its size and a pointer to the next free block.
2. On `free(ptr)`, insert the block into the free list (often merging with adjacent free blocks — coalescing).
3. On `malloc(size)`, walk the free list looking for a block large enough (first-fit, best-fit, or next-fit).

```c
struct free_block {
    size_t size;
    struct free_block *next;
};
```

**Properties:** O(n) worst-case allocation (must walk the list), O(1) free with coalescing, suffers from external fragmentation.

The production `malloc` in glibc (`ptmalloc2`) uses segregated free lists organized by size class, plus binning for fast lookup.

### sbrk and mmap

The allocator must obtain memory from the OS. Two syscalls serve this purpose:

- **`sbrk(increment)`** — moves the program break (end of the heap) up or down. Old-school, single-threaded, contiguous heap. Still used for small allocations in glibc.
- **`mmap(NULL, size, prot, flags, fd, offset)`** — maps an arbitrary region of virtual memory. Modern allocators use this for large allocations (typically > 128 KiB) and for arena creation.

Production allocators prefer `mmap` because:
- Each arena gets its own mapped region (no single heap lock)
- Large allocations can be returned to the OS immediately on free
- `madvise(MADV_DONTNEED)` can release physical pages without unmapping

## Fragmentation

### Internal Fragmentation

The allocator rounds up your request to a size class (e.g., you ask for 17 bytes, get 32). The difference is internal fragmentation — wasted space *inside* the allocated block.

| Requested | Size class | Wasted | % wasted |
|-----------|-----------|--------|----------|
| 1         | 8         | 7      | 87.5%    |
| 17        | 32        | 15     | 46.9%    |
| 65        | 128       | 63     | 49.2%    |
| 1025      | 2048      | 1023   | 50.0%    |

Finer size classes reduce internal fragmentation but increase metadata overhead and bin count.

### External Fragmentation

Free memory exists in total, but it's scattered in small holes between live allocations. A request for 1 KiB fails even though 10 KiB is free — just not contiguous.

External fragmentation gets worse over time with:
- Long-lived allocations interspersed with short-lived ones
- Variable-size allocations
- Lack of coalescing in the allocator

**The worst case:** 50% external fragmentation is the theoretical limit for first-fit with random sizes (the "50% rule").

## Slab Allocators

Slab allocation solves fragmentation by dividing memory into *slabs*, each serving a single size class.

1. Create a slab (one or more pages) for size class 32.
2. Divide the slab into 32-byte slots. Each slot tracks an allocated/free bit.
3. `malloc(20)` → rounds to 32 → pulls a free slot from the 32-byte slab. O(1).
4. `free(ptr)` → returns the slot to its slab. O(1).

**Why this works:**
- No internal fragmentation within a slab (every slot is the same size)
- No external fragmentation for that size class (slabs are homogeneous)
- Allocation and free are both O(1)
- Cache-friendly: same-sized objects live together

**The tradeoff:** you need a slab per size class. jemalloc uses ~40 size classes; mimalloc uses ~28.

## jemalloc Design

jemalloc (Jason Evans malloc) is the allocator behind Firefox, Facebook, and many high-throughput systems.

### Arenas

Memory is partitioned into *arenas* (default: number of CPUs × 2). Each thread is assigned to an arena. This eliminates the global lock — threads compete only with others in their arena.

```
Thread 0 ──→ Arena 0 ──→ Bin 8  Bin 16  Bin 32  ...
Thread 1 ──→ Arena 1 ──→ Bin 8  Bin 16  Bin 32  ...
Thread 2 ──→ Arena 0 ──→ (same arena as Thread 0)
Thread 3 ──→ Arena 2 ──→ Bin 8  Bin 16  Bin 32  ...
```

Arena assignment is sticky — a thread keeps its arena for the process lifetime. This prevents heap churn from thread migration.

### tcache (Thread Cache)

Each thread has a *tcache*: a small array of free slots per size class. Allocation from tcache is lock-free and O(1). Free returns to tcache. When tcache is empty, it refills from the arena bin (batch of 64 pointers). When tcache is full, it flushes back to the arena.

```
malloc(size):
  size_class = size2class(size)
  if tcache[size_class].avail > 0:
      return tcache[size_class].pop()     // fast path: no lock
  else:
      refill from arena bin               // slow path: arena lock
      return tcache[size_class].pop()
```

**tcache is the reason jemalloc is fast.** Most allocations never touch a lock.

### Bins

Within an arena, each size class has a *bin*. A bin manages extent information. When the bin is empty, it allocates a new *extent* (contiguous pages) from the arena and carves it into objects.

Large allocations (> 4 KiB by default) bypass bins and are served directly from the arena's extent heap.

### Key Configuration

```bash
# Set arena count
MALLOC_CONF="narenas:8"

# Enable profiling
MALLOC_CONF="prof:true,prof_prefix:jeprof"

# Disable tcache (for debugging)
MALLOC_CONF="tcache:false"
```

## mimalloc Design

mimalloc (Microsoft's malloc, by Daan Leijen) is designed for predictable low latency and high throughput.

### Segments and Pages

- **Segment** — a contiguous memory region (typically 64 KiB) obtained via `mmap`.
- **Page** — within a segment, pages hold objects of one size class. A segment contains multiple pages.

```
Segment (64 KiB)
+-------------------+-------------------+-------------------+
| Page (size 16)    | Page (size 32)    | Page (size 64)    |
| ○ ○ ○ ○ ○ ○ ○ ○  | ○ ○ ○ ○ ○ ○       | ○ ○ ○ ○           |
+-------------------+-------------------+-------------------+
```

### Free Lists

Each page has a *local free list*. Allocation pops from this list. O(1), no atomic operations.

When the local free list is empty, the page's *thread free list* (fed by frees from other threads) is swapped in. This is the key insight: **free operations from other threads are batched and then swapped in atomically.**

```
Thread A (owner of page):
  malloc → pop from local_free

Thread B (foreign free):
  free → push to page.thread_free (atomic push)

Thread A (when local_free is empty):
  swap local_free ↔ thread_free  // one atomic operation gets a batch
```

This eliminates the need for per-thread caches as separate structures — the page *is* the cache.

### Huge Allocations

Allocations larger than 64 KiB (by default) bypass pages and allocate a dedicated segment via `mmap`. On free, the entire segment is `munmap`'d, returning memory to the OS immediately.

### Comparison with jemalloc

| Feature              | jemalloc                  | mimalloc                   |
|---------------------|---------------------------|----------------------------|
| Thread isolation     | Arenas + tcache           | Pages + thread-free lists  |
| Fast path            | tcache pop (per-thread)   | local free list (per-page) |
| Slow path            | Arena bin (locked)        | Segment page allocate      |
| Foreign free         | → tcache → arena bin      | → thread_free (atomic push)|
| Metadata overhead   | ~4 MB per process         | ~1 KB per segment          |
| Size classes         | ~40                       | 28                         |
| Peak RSS             | Higher (tcache buffers)   | Lower (eager return)      |

## Thread-Local Caches in Depth

The whole point of modern allocators is avoiding global locks. The pattern is always:

1. **Fast path** — per-thread data structure, no synchronization needed. Hit rate should be >95%.
2. **Slow path** — shared data structure, requires atomic ops or locks. Should be rare.

The fast path is typically a stack or array of free pointers. Refill/flush happens in batches, amortizing the lock cost across many allocations.

**Pitfall: tcache size matters.** Too small → frequent arena round-trips. Too large → memory waste. jemalloc defaults vary by size class (2–64 pointers). mimalloc auto-tunes based on allocation pressure.

## realloc

`realloc(ptr, new_size)` is more interesting than it looks:
- If the allocation can grow in-place (adjacent memory is free), it extends without copying.
- Otherwise, it `malloc`s new space, `memcpy`s, and `free`s the old block.
- `realloc(NULL, size)` ≡ `malloc(size)`
- `realloc(ptr, 0)` ≡ `free(ptr)` (implementation-defined in C11)

Production allocators track the usable size (often larger than requested due to size-class rounding). `realloc` to a size that fits in the same size class is a no-op.

```c
// Always check the usable size before reallocating
size_t usable = malloc_usable_size(ptr);
if (new_size <= usable) {
    // No reallocation needed
    return;
}
```

## Alignment

`aligned_alloc(alignment, size)` and `posix_memalign(ptr, alignment, size)` provide aligned allocation. Common alignments:

| Alignment | Use case                           |
|-----------|-------------------------------------|
| 16        | SSE vectors                         |
| 32        | AVX vectors                         |
| 64        | AVX-512 vectors, cache line         |
| 4096      | Page-aligned I/O buffers            |

Allocators internally round up to their minimum alignment (16 on 64-bit systems for jemalloc/mimalloc). So `malloc(1)` gives you at least 16-byte aligned memory.

**Custom alignment in a slab allocator:** choose the slab's slot size to be a multiple of the required alignment. This guarantees every slot is aligned.

## When to Use a Custom Allocator

| Use case                            | Recommended allocator          |
|-------------------------------------|-------------------------------|
| General-purpose server              | jemalloc or mimalloc          |
| Latency-sensitive trading system    | mimalloc                      |
| Memory-constrained embedded         | tlsf (Two-Level Segregated Fit)|
| Arena lifetime (parse → process → free all) | Bump allocator (mmap)  |
| Object pool (fixed-size, high churn)| Slab/pool allocator           |
| Temp allocation (within function)   | Stack/region allocator         |
| NUMA-aware allocation               | jemalloc with NUMA support     |

**Use a custom allocator when:**
1. You have a known, fixed allocation pattern (object pools)
2. You need arena-based lifetime management (parse trees, game frames)
3. System allocator profiling shows lock contention or fragmentation
4. You need predictable O(1) worst-case allocation time

**Do NOT use a custom allocator when:**
1. You haven't measured — the system allocator is probably fine
2. You're optimizing allocation speed but your program is I/O-bound
3. You're introducing allocator complexity without profiling data

## Benchmarking Allocators

The only honest way to choose an allocator is to benchmark *your* workload:

```bash
# Run with glibc malloc
./benchmark

# Run with jemalloc
LD_PRELOAD=/usr/lib/libjemalloc.so ./benchmark

# Run with mimalloc
LD_PRELOAD=/usr/lib/libmimalloc.so ./benchmark
```

Key metrics to measure:
- **Throughput** — allocations per second (varies by size pattern)
- **Latency** — p99 allocation time (lock contention shows up here)
- **RSS / peak memory** — fragmentation shows up as higher RSS
- **Cache miss rate** — `perf stat -e cache-misses ./benchmark`

Use `malloc_stats_print()` (jemalloc) or `mi_stats_print_out()` (mimalloc) to get detailed allocator statistics at program exit.

## Read the Source

- **jemalloc** — <https://github.com/jemalloc/jemalloc>
  - Start with `include/jemalloc/internal/arena_externs.h` for arena structure
  - `src/tcache.c` for the thread-cache fast path
- **mimalloc** — <https://github.com/microsoft/mimalloc>
  - `src/page.c` for the page-level allocation logic
  - `src/segment.c` for segment management

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **allocator_reference.md** — a comparison table and decision guide for choosing allocators

## Exercises

1. **Easy** — Modify the bump allocator in `main.c` to accept a region size and report utilization (used / total) after a sequence of allocations.
2. **Medium** — Add coalescing to the free-list allocator. Benchmark its impact on external fragmentation with the random-size allocation pattern.
3. **Hard** — Implement a per-thread cache layer on top of the free-list allocator using `pthread_key_create` / `pthread_setspecific`. Measure the throughput improvement with 4 threads.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Bump allocator | "arena allocator" | Advances a pointer forward; cannot free individual blocks |
| Free list | "linked-list malloc" | Walks a linked list of free blocks; O(n) worst case |
| Slab allocator | "object pool" | Fixed-size slots in pages; O(1) alloc and free |
| Arena | "memory region" | Partitioned heap that eliminates global lock contention |
| tcache | "thread-local cache" | Per-thread LIFO of free slots; lock-free fast path |
| Internal fragmentation | "wasted space inside" | Rounding to size classes wastes bytes inside allocated blocks |
| External fragmentation | "swiss cheese" | Free memory is fragmented into small holes between live blocks |
| Size class | "bin size" | A rounded allocation size; e.g., 8, 16, 32, 48, 64, 80, ... |

## Further Reading

- *The Structure and Performance of the jemalloc Allocator* — Jason Evans, 2015
- *mimalloc: Free List Sharding in Action* — Daan Leijen, 2019
- *Dynamic Storage Allocation: A Survey and Critical Review* — Wilson, Johnstone, Neely, Bozeman, 1995
- glibc malloc internals: <https://sourceware.org/glibc/wiki/MallocInternals>
- *The 50% Rule* — Wilson et al., on external fragmentation bounds