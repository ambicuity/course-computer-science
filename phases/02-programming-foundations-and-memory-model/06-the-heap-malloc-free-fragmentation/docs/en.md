# The Heap — malloc, free, fragmentation

> `malloc` is a *user-space program* sitting between you and the kernel. It hands out chunks of an arena, tracks who has what, and tries to keep fragmentation low. Knowing how it works explains a long list of mysterious perf and memory bugs.

**Type:** Build
**Languages:** C
**Prerequisites:** Phase 02, Lessons 02, 05
**Time:** ~75 minutes

## Learning Objectives

- Distinguish *stack* (function-scope, fast, fixed) from *heap* (process-scope, slower, variable-size) allocation.
- Explain how `malloc` gets memory from the kernel (`brk`/`sbrk` for small, `mmap` for large) and why.
- Implement a minimal bump allocator and a free-list allocator; understand what they trade away (`free` support vs. simplicity).
- Recognize fragmentation (internal vs external), and the strategies real allocators use to combat it (segregated free lists, size classes, slab/region allocation).

## The Problem

You write `int *arr = malloc(n * sizeof(int));` and get a pointer to n usable ints. Underneath, a sophisticated program (glibc's `ptmalloc`, jemalloc, mimalloc, tcmalloc) is:

1. Asking the kernel for memory in big chunks (4 KB pages).
2. Subdividing those chunks among your `malloc` calls.
3. Tracking free vs in-use regions so `free` knows what to return.
4. Coalescing adjacent free regions when possible.
5. Trying to satisfy each request quickly without inflating peak memory.

Knowing the *shape* of what `malloc` is doing answers questions like:

- "Why does my program's RSS keep growing even though I `free` everything?"
- "Why is `malloc(1)` not 1 byte?"
- "Why is the n-th allocation 1000x slower than the (n-1)-th?"

## The Concept

### Stack vs heap, recap

| | Stack | Heap |
|--|-------|------|
| Allocated by | `sub rsp, N` in the function prologue | `malloc` |
| Freed by | function return | `free` |
| Lifetime | until function returns | until you `free` (or process exits) |
| Speed | a single subtract | dozens-to-hundreds of cycles |
| Size limit | small (~8 MB default) | huge (terabytes on 64-bit) |
| Cache behavior | great (top of stack hot in cache) | depends |

### How `malloc` gets memory

`malloc` doesn't talk to RAM directly. It calls two kernel APIs:

1. **`brk` / `sbrk`**: extend the process's data segment (the heap). One contiguous region. Linear and cheap, but you can't release in the middle.
2. **`mmap`**: map a fresh region of anonymous pages. Used for big allocations (typically ≥ 128 KB on glibc). Each `mmap` region is independent — `munmap` returns it to the OS.

### A minimal bump allocator

The simplest non-trivial allocator:

```c
static char arena[1 << 20];     /* 1 MB */
static size_t offset = 0;

void *bump_alloc(size_t size) {
    if (offset + size > sizeof(arena)) return NULL;
    void *p = arena + offset;
    offset += size;
    return p;
}

void bump_free(void *p) { /* no-op */ }
```

Pros: ~1 instruction per allocation. Cons: can't reclaim individual frees. Used in compilers (per-pass arena) and some short-lived contexts.

### A free-list allocator

To support arbitrary `free`, maintain a linked list of free chunks. `malloc(n)`:
- Walk the list; find a chunk ≥ n.
- Split if much bigger; remove from list; return the payload region.

`free(p)`:
- Mark the chunk as free; insert into list.
- Optionally coalesce with adjacent free chunks.

Strategies for "find a chunk ≥ n":
- **First-fit**: take the first that fits. Fast, decent fragmentation.
- **Best-fit**: scan all, take the smallest that fits. Less waste per allocation, often *more* fragmentation overall.
- **Segregated free lists**: separate list per size class (16, 24, 32, 48, 64, ...); O(1) lookup. Used by glibc, jemalloc, tcmalloc.

### Per-allocation metadata

Real allocators store metadata (size, prev/next pointers, flags) near each chunk. A typical layout:

```
+------------+--------+----+--------------+----+
| prev_size  | size+f | -- |  user payload | -- |
+------------+--------+----+--------------+----+
                            ^ pointer returned by malloc
```

So `malloc(1)` actually allocates ~32 bytes — payload + metadata + alignment. This is also why `free(p)` knows how much memory to release: it reads the size from the bytes *just before* `p`.

### Fragmentation

| Kind | What |
|------|------|
| **Internal** | Wasted bytes *inside* an allocated chunk (alignment padding, rounding up to nearest size class) |
| **External** | Free chunks exist but none is big enough for the next request; total free > requested, but no single block fits |

Coalescing adjacent free chunks reduces external fragmentation. Size-class allocators avoid internal fragmentation by allocating exactly from pre-sized pools.

### Production allocators

| Allocator | Strategy | Strengths |
|-----------|----------|-----------|
| glibc `ptmalloc2` | per-thread arenas, segregated lists, fastbins | default on Linux |
| jemalloc | per-CPU arenas, run-based size classes | low fragmentation, fast multi-threaded |
| tcmalloc | thread-cache + central heap | very fast for small allocs |
| mimalloc | sharded free lists, eager coalescing | newest; often fastest |

Drop in via `LD_PRELOAD=/path/to/libjemalloc.so ./yourapp` — no recompile needed.

## Build It

Open `code/main.c`. We build a working free-list allocator in ~80 lines.

### Step 1: Bump allocator

A 1 MB arena, monotonically advancing.

### Step 2: Free-list allocator with coalescing

Header per chunk (size + free flag). `my_malloc` walks the list (first-fit) and splits large chunks. `my_free` marks free and coalesces with the next free chunk.

### Step 3: Stress test

Allocate 1000 chunks, free every other one, then allocate 500 more. Verify the allocator handles holes and coalescing.

### Step 4: Compare with glibc

`LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libjemalloc.so ./your-program` swaps allocators without recompile.

### Step 5: Inspect fragmentation

After many alloc/free cycles, walk the free list. Count and size the holes — external fragmentation made visible.

## Use It

- **Memory leaks**: a program that calls `malloc` without `free` grows its heap indefinitely. Tools: Valgrind's memcheck (Phase 00 Lesson 08), heaptrack, glibc's `mtrace`.
- **Slow malloc**: if your code's hot path is `malloc`/`free`, consider an *arena* (allocate from a big buffer, free everything at once — common in compilers).
- **Heap layout exploits** (Phase 12): predicting where `malloc` will place the next chunk is the foundation of "heap feng shui" — controlled corruption attacks.
- **GC vs manual**: garbage-collected runtimes (Go, Java) have their own allocator + collector; understanding manual allocators is half the GC mental model.

## Read the Source

- *Computer Systems: A Programmer's Perspective* (Bryant & O'Hallaron), Chapter 9 (Virtual Memory) and §9.9 (Dynamic Memory Allocation).
- [glibc `ptmalloc2` source](https://sourceware.org/git/?p=glibc.git;a=tree;f=malloc) — `malloc.c` is ~5000 lines; the comments are worth reading.
- *Doug Lea's malloc essay* — http://gee.cs.oswego.edu/dl/html/malloc.html — original ptmalloc author.

## Ship It

This lesson ships **`outputs/arena.c`** — a tiny arena allocator (allocate from a buffer, drop everything in one shot). Drop into any project that has a clear request lifetime (per-HTTP-request, per-game-frame, per-compiler-pass).

## Exercises

1. **Easy.** Write a program that mallocs 10 MB in one call and 10 MB across a million small allocations. Time both with `time ./prog`. Notice the dramatic difference.
2. **Medium.** Extend the lesson's free-list allocator with *best-fit* instead of *first-fit*. Measure fragmentation on a workload with mixed sizes.
3. **Hard.** Implement a slab allocator for fixed-size objects (e.g., 32-byte records). Compare its allocation throughput against `malloc(32)` in a loop; slab should win by 10×+.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Heap | "Long-lived memory" | A program-managed region of virtual memory used for objects whose lifetime exceeds the calling function |
| `brk` / `sbrk` | "Old-style heap extension" | Syscall that moves the *program break* (end of data segment); fast but only releases at the high end |
| `mmap` | "Modern anonymous mapping" | Syscall that maps fresh pages; each region is independent (can `munmap` in any order) |
| Fragmentation | "Wasted memory" | Internal: padding inside allocated blocks. External: many free blocks none big enough for the request |
| Coalescing | "Merging neighbors" | Combining adjacent free blocks back into one bigger block; counters external fragmentation |

## Further Reading

- *Effective Memory Management on Linux* — LWN article series on glibc's allocator internals.
- [jemalloc 5 paper](http://jemalloc.net/) — modern thread-caching allocator design.
- *The Art of Multiprocessor Programming* (Herlihy & Shavit) — Chapter on concurrent memory management.
