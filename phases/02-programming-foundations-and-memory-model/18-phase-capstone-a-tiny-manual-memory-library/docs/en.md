# Phase Capstone — A Tiny Manual-Memory Library

> Phase 02 in one package: a small library that combines arena, pool, and bounded-slice allocators with defensive invariants, runnable under sanitizers.

**Type:** Capstone
**Languages:** C, Rust
**Prerequisites:** All of Phase 02
**Time:** ~75 minutes

## Learning Objectives

- Integrate the lessons of Phase 02 — pointers, heap, ownership, allocators, defensive programming — into one coherent reusable library.
- Provide both a C interface (manual lifetime) and a Rust interface (compiler-enforced lifetime) over the same memory primitives.
- Run the full library under ASAN + UBSAN + LSAN with zero diagnostics.
- Ship a header (`memlib.h`) and a crate (`memlib`) usable from other projects.

## The Problem

Across Phase 02 we built six pieces in isolation: a pool allocator, a bump allocator, an arena, an X-macro registry, assertion macros, and Rust ownership/borrowing exercises. Real systems need all of these *together*, with consistent error handling, consistent invariants, and tests.

The capstone packages everything into one library that:

1. **Allocates fixed-size objects fast** (pool).
2. **Allocates variable-size objects fast with batch free** (arena/bump).
3. **Bounds every access** (bounded slice helpers).
4. **Catches every misuse loudly** (REQUIRE/ENSURE/INVARIANT macros from L17).
5. **Passes sanitizers** clean on a stress test that allocates millions of objects.

## The Concept

### API surface

```c
/* memlib.h */
typedef struct MemArena MemArena;
typedef struct MemPool  MemPool;

MemArena *arena_create(size_t initial_capacity);
void     *arena_alloc(MemArena *a, size_t bytes, size_t align);
char     *arena_strdup(MemArena *a, const char *s);
void      arena_reset(MemArena *a);
void      arena_destroy(MemArena *a);
size_t    arena_used(const MemArena *a);

MemPool  *pool_create(size_t slot_size, size_t n_slots);
void     *pool_alloc(MemPool *p);
void      pool_free(MemPool *p, void *obj);
size_t    pool_free_count(const MemPool *p);
void      pool_destroy(MemPool *p);

/* Bounded slice */
typedef struct { void *data; size_t len, stride; } MemSlice;
void *slice_get(MemSlice s, size_t i);
```

The Rust crate mirrors this with `Arena`, `Pool<T>`, and `Slice<'_, T>` — but ownership and lifetimes are compiler-enforced.

### Invariants

The library enforces these in debug builds (free in release):

- **Arena**: `used <= capacity`; allocations are aligned per request.
- **Pool**: free-list length + handed-out count == n_slots; double-free is detected; `pool_free`'s arg lies inside the slab.
- **Slice**: `i < len` on every `slice_get`.

### Failure mode

Out-of-memory in the arena returns NULL; the caller checks. Out-of-slot in the pool returns NULL. All other misuse — null pointer, bogus arg, double-free — aborts via assertion. The C convention: NULL = "I ran out"; abort = "you misused me."

### Why both arena and pool?

They cover complementary patterns:

- **Arena** is best when many objects share a lifetime — a request handler, a parse tree, a compilation unit. One `arena_reset()` reclaims everything.
- **Pool** is best when many objects of the same size are created and destroyed independently — graph nodes, free-list entries, particle systems.

Real allocators (jemalloc, mimalloc) compose both ideas: per-size classes (pool-like) + per-thread arenas. This capstone gives you both primitives in one library.

## Build It

Open `code/main.c`. It contains the full library plus a demo program. The demo:

1. Creates an arena and a pool.
2. Allocates 1 million strings in the arena (variable sizes).
3. Allocates 100K linked-list nodes via the pool, then frees them in random order.
4. Demonstrates the bounded-slice helper rejecting an out-of-bounds index.
5. Reports memory used, fragmentation, and ns/op for both allocators.

### Building & running

```sh
# Debug + sanitizers
clang -O1 -g -fsanitize=address,undefined main.c -o memlib_debug
./memlib_debug

# Release
clang -O2 -DNDEBUG main.c -o memlib_release
./memlib_release
```

ASAN output should be silent on a clean run; release should be ~3× faster than debug.

The Rust mirror lives in `code/main.rs`. It uses `Bump` (single-arena), `Pool<Node>` (typed pool with a free-list), and a custom `BSlice<'a, T>` with bounds checks. The Rust version's invariants are mostly enforced by the type system — but a few runtime `debug_assert!`s catch programmer misuse.

## Use It

Three concrete uses:

1. **Build a parser** with arena allocation: every AST node lives in the arena; one `arena_destroy()` at end-of-compilation frees them all. (Phase 08 reuses this.)
2. **Build a graph** with pool-allocated nodes: insertion and deletion are O(1) without `malloc`. (Phase 03 reuses this.)
3. **Build a request handler** with arena-per-request: every allocation in the request's lifetime goes to the per-request arena, and the arena is destroyed at response-send. No leaks possible. (Phase 09 reuses this.)

## Read the Source

- **tree-sitter's allocator** ([`subtree.c`](https://github.com/tree-sitter/tree-sitter/blob/master/lib/src/subtree.c)) — uses a similar arena+pool combo for AST nodes.
- **bumpalo** ([crates.io](https://docs.rs/bumpalo/)) — production Rust bump allocator. Same shape as our Rust mirror, with chunk-chaining for unbounded growth.
- **jemalloc** ([source tour](https://github.com/jemalloc/jemalloc)) — production-grade size-class allocator. See `src/arena.c` for the canonical structure.

## Ship It

This capstone ships **`outputs/memlib.h`** — the single-header library, copy-and-pasteable into other projects. Includes:

- Inline arena and pool implementations.
- The defensive macros from L17.
- One #define switch (`MEMLIB_DEBUG`) for invariant checks.

## Exercises

1. **Easy.** Add `arena_alloc_zeroed(MemArena*, size_t)` that zero-fills the returned bytes. Verify with valgrind / ASAN that no uninitialized read survives.
2. **Medium.** Make the arena grow when it runs out of space (chunked arena). Each `arena_alloc` either fits in the current chunk or allocates a fresh, larger chunk and links it. `arena_destroy` walks the chunk list.
3. **Hard.** Add a `pool_alloc_track(Pool *p, const char *origin)` debug API that records the file/line of each live allocation. On `pool_destroy`, report any slots not freed and their origins (homemade LeakSanitizer for pool memory).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Arena | "Region allocator" | Linear allocator with batch free; objects share a lifetime |
| Pool | "Slab allocator" | Fixed-size slot allocator with O(1) alloc/free |
| Bounded slice | "Fat pointer" | (pointer, length) pair with runtime bounds check |
| Single-header library | "stb-style" | Library shipped as one .h file; include in client to use |
| Sanitizer-clean | "ASAN passes" | Code compiles + runs under sanitizers with zero diagnostics |

## Further Reading

- [bumpalo crate docs](https://docs.rs/bumpalo/) — the production Rust bump allocator.
- *The Memory Allocator Wars* (Berger et al.) — survey of allocator designs.
- *Fast and Memory-Efficient Allocation in C++* (Alexandrescu) — talk on per-pool design.
- [Mike Acton's "Data-Oriented Design"](https://www.youtube.com/watch?v=rX0ItVEVjHc) — why allocator choice IS the architecture in performance-critical code.
