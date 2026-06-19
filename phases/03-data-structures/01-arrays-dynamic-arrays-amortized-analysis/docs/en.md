# Arrays & Dynamic Arrays — Amortized Analysis

> A push that occasionally copies the entire array can still be O(1) on average. Understanding why is the gateway to amortized analysis.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** Phase 02 (heap, ownership), Phase 01 (asymptotic notation)
**Time:** ~60 minutes

## Learning Objectives

- Implement a dynamic array (Python `list`, C++ `vector`, Rust `Vec`) from scratch with `push`, `pop`, `get`, `set`, `len`, `cap`.
- Prove `push` is **amortized O(1)** under doubling growth, and **amortized O(n)** under `+k` growth.
- Apply the three classical amortized-analysis techniques: aggregate, accounting (banker's), potential.
- Decide growth factor (1.5× vs 2× vs golden ratio) by understanding the reuse-of-freed-space trade-off.

## The Problem

A C array has a fixed size at allocation. But your program doesn't know how many elements it will hold. You could:

1. Allocate `INT_MAX` upfront — wastes memory, fails on small systems.
2. Re-`realloc` on every push — O(n) per push, O(n²) for n pushes.
3. Grow geometrically: when full, allocate 2× the capacity and copy. Most pushes are O(1); occasional ones are O(n); **on average, push is O(1)**.

Option 3 is the dynamic array. It is the single most common data structure in modern code — every `list`, `vector`, `Vec`, `ArrayList`, `Slice`, `vec!`, `[1,2,3].push(4)` you've ever touched.

The interesting analysis is *why* push is O(1) amortized when occasionally it's O(n). The technique that proves it (amortized analysis) reappears in splay trees, union-find with path compression, Fibonacci heaps, and incremental garbage collection.

## The Concept

### The data

```c
typedef struct {
    int    *data;
    size_t  len;
    size_t  cap;
} Vec;
```

Three fields. Everything follows.

### push, with doubling

```c
void vec_push(Vec *v, int x) {
    if (v->len == v->cap) {
        size_t new_cap = v->cap == 0 ? 4 : v->cap * 2;
        v->data = realloc(v->data, new_cap * sizeof(int));
        v->cap = new_cap;
    }
    v->data[v->len++] = x;
}
```

Most pushes: O(1) — one write. Occasionally (when `len == cap`): O(n) — copy n elements, then write.

### Aggregate analysis

Push n elements starting from empty. Total copy cost across n pushes:

- 1 copy at cap=1 (push #1 trigger)
- 2 copies at cap=2 (push #2 trigger... actually with cap=1 doubling, pushes 1, 2, 3, 5, 9, 17 trigger; copy costs are 1, 2, 4, 8, 16...).

Sum: 1 + 2 + 4 + 8 + ... + n/2 + n = 2n − 1 ≤ 2n. So **total work across n pushes is ≤ 3n** (n writes + ≤ 2n copies). Per push: **3 amortized**, i.e. O(1).

### Accounting (banker's) method

Charge each push **3 units**: 1 for the actual write, 2 deposited into a savings account. When a resize happens (n elements), every element you copy has already deposited 2 units, so the savings cover the copy. Each push pays a constant; the savings absorb the spikes.

### Potential method

Define potential Φ = 2·len − cap. Empty: Φ=0; right before resize: len=cap, Φ=cap; just after resize: cap=2·old_cap, len=old_cap+1, so Φ = 2 + (old_cap − 2·old_cap) ≈ −old_cap + 2. Doing the algebra: amortized cost of push = actual + ΔΦ = 1 + 2 = 3 (non-resize) or n+1 + (−n+2) ≈ 3 (resize). All push amortized costs are constant.

Three methods, same answer — different mental models for the same theorem.

### Growth factor trade-off

| Factor | Memory peak | Reuse of freed space |
|--------|-------------|----------------------|
| 1.5×   | 1.5·n peak | Allocator may reuse old chunk (next alloc fits) |
| 2×     | 2·n peak   | Old chunk never large enough; allocator extends heap |
| φ≈1.618× | Compromise | Eventually fits previous holes |

C++ `vector` (libc++/MSVC) uses 1.5–2×; Rust `Vec` uses 2×; Go slice uses 2× until 1024, then 1.25×. There's no winner — both correctness and performance depend on allocator behavior and access patterns.

### pop and shrink

Popping leaves a gap. Shrink eagerly? No — then push-pop oscillation becomes O(n) per op. The standard rule: **shrink to half-capacity only when len < cap/4**. Provably amortized O(1) for both push and pop with the same potential argument.

## Build It

`code/main.c` implements `Vec` from scratch with the doubling rule and a delete-with-shrink. The program:

1. Pushes 1..N, measuring the (amortized) cost per push.
2. Tracks how many copies happened — log₂(N) of them — vs N writes.
3. Compares 1.5× vs 2× vs +k=8 growth: prints total bytes copied for each.

`code/main.py` is the Python mirror — easier to inspect the analysis. The `bytearray` and `list` underlying CPython are both dynamic-array-backed; this lesson is built into Python.

`code/main.rs` is the Rust mirror. The standard `Vec::push` does this under the hood — see [the source](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.push) for the real one.

### Build & run

```sh
clang -O2 main.c -o vec && ./vec
python3 main.py
cargo run --release       # if you scaffold a Cargo.toml
```

## Use It

- **C++**: `std::vector<T>::push_back` — same algorithm, RAII-managed.
- **Rust**: `Vec<T>::push` — uses `RawVec` for the realloc dance.
- **Python**: `list.append` — overallocates with a formula `new_cap = (new_size + (new_size >> 3) + 6) & ~3`, i.e., ~1.125× growth.
- **Go**: `append(slice, x)` — `runtime.growslice`. Doubling until 1024, then 1.25×.

When you write `vec.push(x)`, you are paying the amortized constant of this lesson.

## Read the Source

- **Rust**: [`alloc/src/raw_vec.rs`](https://github.com/rust-lang/rust/blob/master/library/alloc/src/raw_vec.rs) — `grow_amortized` does exactly the algebra above.
- **CPython**: [`Objects/listobject.c`](https://github.com/python/cpython/blob/main/Objects/listobject.c) — `list_resize` is the doubling-ish growth, with the 1.125× tweak.
- **Go runtime**: [`runtime/slice.go`](https://github.com/golang/go/blob/master/src/runtime/slice.go) — `growslice` is the most readable production implementation.

## Ship It

This lesson ships **`outputs/vec.c`** — a clean, copy-pasteable C dynamic array with type-generic macros (à la stb-style) — and **`outputs/amortized_cheatsheet.md`** mapping push patterns to amortized cost.

## Exercises

1. **Easy.** Add `vec_reserve(Vec*, size_t)` that grows to exactly that capacity. Show that hot loops calling reserve up front are 2-3× faster than push-only loops on >100K elements.
2. **Medium.** Implement `vec_insert(Vec*, size_t i, int x)` and prove (using accounting) that its amortized cost is O(n), not O(1) — the n shift dominates.
3. **Hard.** Implement an "unrolled linked list": each list node holds a small array (k=64). Compare insertion cost, cache behavior, and pointer overhead vs `Vec`. When does each win?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Amortized | "Average over many ops" | Worst-case AVERAGE per op over any sequence — stronger than expected, weaker than worst-case |
| Capacity | "Allocated size" | Bytes available; `len` is how many are filled |
| Geometric growth | "Doubling" | new_cap = α·old_cap for α > 1; gives amortized O(1) push |
| Potential function | "Φ" | A bookkeeping value tracking "stored-up work"; ΔΦ smooths spike costs |
| Shrink-to-fit | "Realloc down" | Reclaim unused capacity; rare in hot paths |

## Further Reading

- *Introduction to Algorithms* (CLRS) Ch. 17 — the canonical amortized-analysis chapter.
- [Rust nomicon: Vec](https://doc.rust-lang.org/nomicon/vec/vec.html) — a from-scratch Vec with full amortized analysis.
- [Bjarne Stroustrup: Why std::vector is so important](https://www.stroustrup.com/) — the cache-locality argument for arrays over linked structures.
