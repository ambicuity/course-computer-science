# Build a Pool Allocator (from scratch in C)

> A pool allocator is fixed-size objects on top of a slab of memory. Each free slot stores its own next-pointer; alloc and free are both O(1) and exact. No fragmentation, ever.

**Type:** Build
**Languages:** C
**Prerequisites:** Phase 02, Lessons 05, 06
**Time:** ~75 minutes

## Learning Objectives

- Implement a pool allocator for fixed-size objects in <100 lines of C.
- Reuse free-slot memory as a singly-linked free-list (each freed slot's first bytes are the "next" pointer).
- Benchmark against `malloc`/`free` — observe the typical 5-20× speedup on hot allocation paths.
- Recognize when a pool allocator is the right tool: many short-lived equally-sized objects (network packet headers, AST nodes, listener objects).

## The Problem

Generic `malloc`/`free` (Phase 02 Lesson 6) supports arbitrary sizes, generic patterns, multi-threading, and so on — at a cost of dozens to hundreds of cycles per allocation. When your hot loop allocates 4096-byte network buffers a million times, or 64-byte AST nodes during parsing, you're paying for flexibility you don't use.

A pool allocator specializes: one object size, one allocator, almost no work per call. For workloads that match its shape, it's an order of magnitude faster.

## The Concept

### Design

```
   ┌──────────────────┐
   │ slot 0           │ ← free; "next" pointer here
   ├──────────────────┤
   │ slot 1           │ ← in use (holds your object)
   ├──────────────────┤
   │ slot 2           │ ← free
   ├──────────────────┤
   │ ...              │
   └──────────────────┘

   free-list head ──> slot 0 ──> slot 2 ──> slot 5 ──> NULL
```

The pool reserves a contiguous slab divided into N slots, each `slot_size` bytes (≥ `sizeof(void*)` so we can store the next-pointer in free slots).

- **alloc**: pop the head of the free list. O(1).
- **free**: push the slot onto the free list. O(1).
- **init**: link every slot into the free list. O(N).

When a slot is *free*, its first `sizeof(void*)` bytes store the next-pointer. When a slot is *in use*, those same bytes are user data. The free-list is "embedded" in the free memory itself — no extra metadata structure.

### Alignment

Each slot must satisfy the alignment of the largest type it'll hold. For arbitrary user types use `_Alignof(max_align_t)` (typically 16 bytes on x86_64).

### Trade-offs

| Property | Pool allocator | malloc |
|----------|----------------|--------|
| Speed per alloc | ~3 cycles (load + store + compare) | dozens to hundreds |
| Size flexibility | One fixed size | Any size |
| Multi-thread | Per-thread pool or external lock | Built in |
| Fragmentation | None (every slot identical) | Possible |
| Memory overhead | One ptr per pool + slot alignment | Per-allocation header |
| Lifetime | Bound to the pool's lifetime | Per-allocation |

Real use: Linux's `kmem_cache_*` (slab allocator), Apache's per-connection memory pool, V8's zone allocator for compiler IR.

### Variants

- **Slab allocator** (Linux kernel): pool + per-CPU caches + multi-class size; production-grade.
- **Region allocator** (LLVM, V8): one big arena per phase, all freed at once.
- **Object-tracking pool**: combine alloc-bit + free-list to detect double-free.

## Build It

The lesson's `code/main.c` implements:

```c
typedef struct Pool Pool;

Pool *pool_create(size_t slot_size, size_t slots);
void *pool_alloc(Pool *p);
void  pool_free (Pool *p, void *obj);
void  pool_destroy(Pool *p);
```

### Step 1: Initialization

Allocate a `slot_size * slots` byte block. Loop through, threading each slot's first 8 bytes as a pointer to the next slot.

### Step 2: alloc — pop the free-list head

```c
void *pool_alloc(Pool *p) {
    if (!p->free_head) return NULL;        /* exhausted */
    void *slot = p->free_head;
    p->free_head = *(void **)slot;          /* advance head to "next" */
    return slot;
}
```

### Step 3: free — push onto the free-list

```c
void pool_free(Pool *p, void *obj) {
    *(void **)obj = p->free_head;           /* link old head as next */
    p->free_head = obj;                     /* push onto list */
}
```

### Step 4: Benchmark vs malloc

Allocate + free 1M times. Pool typically beats malloc by 5-20× on the same size.

### Step 5: Safety extras (in `outputs/`)

- Poison freed slots (`memset(slot, 0xDD, slot_size)`) so reads-after-free show a recognizable pattern.
- Check `obj` belongs to this pool (compare against `[slab, slab + size)`).
- Bitmap for "in-use" tracking to detect double-free.

## Use It

- **Kernel memory caches**: Linux's `kmem_cache_create`/`alloc`/`free` is exactly this, with size classes and per-CPU caches added.
- **V8 / SpiderMonkey**: per-function zone-allocator pools for compiler IR.
- **Networking servers**: per-connection pool for protocol headers.
- **Games**: per-frame pool for transient objects; reset at frame end.

## Read the Source

- *Linux kernel source*, `mm/slab.c` and `mm/slub.c` — production slab allocators.
- *Jeff Bonwick's 1994 paper, "The Slab Allocator: An Object-Caching Kernel Memory Allocator"* — the seminal work.
- *Doug Lea's malloc essay* — for the contrast with general-purpose allocation.

## Ship It

This lesson ships **`outputs/pool.h`** — a header-only, alignment-safe pool allocator with optional poisoning for use-after-free detection. Drop in any C project.

## Exercises

1. **Easy.** Build the lesson's pool with `slot_size = 64`, `slots = 1000`. Alloc 1000 objects; free them in reverse order; alloc 1000 more. Confirm pointers match the originals exactly (LIFO behavior).
2. **Medium.** Implement `pool_for_each_inuse(p, fn)` that calls `fn(slot)` on every in-use slot. Hint: maintain a bitmap or scan the slab + check against the free list.
3. **Hard.** Make the pool thread-safe with per-thread caches: each thread has a small local free list, falling back to the central pool under a spinlock when local is empty. Benchmark vs single-locked pool.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Pool allocator | "Slab allocator" | Allocator for fixed-size objects; O(1) alloc/free; no fragmentation |
| Free list | "Linked list of free slots" | Embedded singly-linked list; "next" pointer stored in each free slot's own bytes |
| Slab | "The memory block" | The contiguous region of memory the pool subdivides into slots |
| Slot | "One pool element" | A fixed-size region within the slab, either holding a user object or storing a next-pointer |
| Poisoning | "Filling freed memory with a pattern" | Writing 0xDD or similar to freed slots; reads-after-free show the pattern, easier to spot |

## Further Reading

- *Bonwick (1994)* — "The Slab Allocator: An Object-Caching Kernel Memory Allocator." USENIX paper.
- *The Mesh allocator* — modern academic allocator with hardware-assisted compaction.
- *Hoard* by Berger et al. — multi-threaded allocator that influenced jemalloc and tcmalloc.
