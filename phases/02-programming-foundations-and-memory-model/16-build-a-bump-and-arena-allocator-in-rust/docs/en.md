# Build a Bump and Arena Allocator (in Rust)

> A bump allocator is ~3 lines of logic. A Rust arena adds: lifetimes that prevent use-after-free *at compile time*. Two pages of code, full memory safety.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 02, Lessons 06, 10, 11
**Time:** ~75 minutes

## Learning Objectives

- Implement a bump allocator in Rust: a contiguous buffer + an offset; no per-allocation tracking.
- Tie the allocator's borrow lifetime to outlive everything it hands out — using Rust's lifetime system.
- Implement a typed arena `Arena<T>` that allocates `&T`/`&mut T` references whose lifetime matches the arena's; explore how `bumpalo` and `typed_arena` do this in production.
- Compare against C's `arena.c` from Lesson 06 — the Rust version is *just as fast* and *unconditionally safe*.

## The Problem

Rust's default allocation strategy is per-value, with `Drop` running individually at scope exit. For workloads where many objects share a single phase (a compiler pass; an HTTP request; a game frame), that's overhead — and the deeper problem, *Rust's ownership rules make graph-shaped data structures hard*. An AST or a graph wants many node-references; you can't easily have multiple `&mut` references to the same node, and `Rc<RefCell<Node>>` is the typical-but-clunky workaround.

An **arena allocator** solves both: bulk-allocate from one buffer, hand out `&T` references that all live as long as the arena, and free everything in one shot. The borrow checker enforces that no reference escapes the arena's lifetime.

## The Concept

### Bump allocator

```rust
pub struct Bump {
    buf: Box<[u8]>,
    offset: Cell<usize>,
}

impl Bump {
    pub fn with_capacity(n: usize) -> Self { /* ... */ }
    pub fn alloc(&self, size: usize) -> Option<&mut [u8]> { /* ... */ }
}
```

- One contiguous buffer.
- `alloc(n)` advances the offset by n (with alignment); returns a slice pointing into the buffer.
- No per-allocation tracking; no `free` for individual allocations.

The crucial Rust twist: `alloc` takes `&self`, not `&mut self`. Because the returned slice borrows from the buffer, no two concurrent `alloc` calls can overlap their results in scope. Internal mutability via `Cell<usize>` is what lets `alloc` mutate `offset` from a shared reference.

### Typed arena

```rust
pub struct Arena<T> {
    chunks: RefCell<Vec<Vec<T>>>,
}

impl<T> Arena<T> {
    pub fn alloc(&self, value: T) -> &mut T { /* ... */ }
}
```

The arena owns chunks of `Vec<T>` (so destructors of T run when the arena drops). `alloc` finds a chunk with room (or allocates a new one), `push`es into it, and returns a mutable reference into the chunk.

The signature is the magic part: `&self -> &mut T` would be unsound in general (multiple `&mut`s to the same value!). But because the arena adds new values and *never moves existing ones* (and never returns a reference twice for the same slot), each `&mut T` it returns is to a distinct cell.

### Lifetime elision in practice

For a typed arena:

```rust
impl<'a, T> Arena<T> {
    pub fn alloc(&'a self, value: T) -> &'a mut T;
}
```

The returned `&'a mut T` is bounded by the arena's borrow's lifetime. So:

```rust
let arena: Arena<Node> = Arena::new();
let n: &mut Node = arena.alloc(Node { ... });
// `n` cannot outlive arena (the borrow checker enforces it).
```

### Cycle-friendly graphs

Arenas are great for graph-shaped data. AST nodes referring to each other:

```rust
struct Node<'a> {
    name: String,
    children: Vec<&'a Node<'a>>,
}

let arena: Arena<Node> = Arena::new();
let leaf = arena.alloc(Node { name: "leaf".into(), children: vec![] });
let root = arena.alloc(Node { name: "root".into(), children: vec![leaf] });
```

No `Rc`, no `RefCell`. Just `&` references that all share the arena's lifetime. The whole graph dies in one shot when the arena drops.

### Production crates

- **`bumpalo`** — production-grade bump allocator; supports any type via `alloc()`, `try_alloc_layout()`.
- **`typed_arena`** — typed arena per type.
- **`id_arena`** — arena using stable indices instead of references (works well with serialization / cyclic graphs).
- **`slotmap`** — generational keys for stable identifiers even after deletions.

Real users: rustc itself uses several arenas internally for AST and HIR.

### Trade-offs vs malloc

| Property | Bump / Arena | malloc |
|----------|--------------|--------|
| Allocation cost | ~1 cycle (offset bump) | dozens to hundreds |
| Individual free | NONE | per allocation |
| Bulk free | One destructor call | per-allocation `free` |
| Cache locality | Excellent (contiguous) | depends |
| Memory efficiency | Some bytes wasted at end of chunks | exact |

## Build It

Open `code/main.rs`.

### Step 1: Minimal bump allocator

A buffer + `Cell<usize>` offset. `alloc_bytes` returns `Option<&mut [u8]>` aligned to T.

### Step 2: Typed `Bump::alloc<T>`

Generic over T; returns `&mut T` (mutable, single-owner reference) carved out of the bump buffer.

### Step 3: AST graph with arena lifetimes

Define a `Node<'a>` with `children: Vec<&'a Node<'a>>`. Build a small tree; the borrow checker ensures the tree can't outlive the arena.

### Step 4: Reset (one-shot free)

`reset()` zeroes the offset, invalidating *all* outstanding references. The borrow checker rejects code that uses references after a reset.

### Step 5: Compare with `bumpalo` (preview)

A few lines using the production `bumpalo` crate API — the Bump::new(), Bump::alloc(), get-data-back pattern looks identical to your version.

## Use It

- **Compilers** (Rust's own rustc, LLVM): per-pass arenas for IR.
- **HTTP servers**: per-request arena reset to baseline between requests.
- **Game engines**: per-frame "scratch" allocator; reset at frame start.
- **Parsers**: AST nodes in one arena, then transform into a new arena's structure for the next pass.

## Read the Source

- [`bumpalo` source on GitHub](https://github.com/fitzgen/bumpalo) — production bump allocator; read `lib.rs` (~1500 lines).
- [Rust's compiler arena](https://github.com/rust-lang/rust/blob/master/compiler/rustc_arena/src/lib.rs) — internal arena used by rustc.
- *Beyond the Borrow Checker* by Manish Goregaokar — blog series on arena-based design in Rust.

## Ship It

This lesson ships **`outputs/bump.rs`** — a small `Bump` allocator with `alloc`, `alloc_slice`, `reset`. Drop into any Rust project as a single file.

## Exercises

1. **Easy.** Allocate 1000 i32 values from a Bump; verify their addresses are contiguous + 4-byte aligned.
2. **Medium.** Implement `Arena::alloc_slice<T: Clone>(&self, items: &[T]) -> &mut [T]` — bulk-copy a slice into the arena.
3. **Hard.** Build an AST `Expr<'a>` (a recursive enum with `Box<Expr<'a>>` or arena references) for `(a + b) * c`. Implement a stack-style evaluator that doesn't recurse into Rust's call stack.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Bump allocator | "Just advance a pointer" | A buffer + offset; alloc is offset += size; no per-allocation tracking |
| Arena | "Bulk allocator with shared lifetime" | An allocator handing out references that all live as long as the arena itself |
| Reset | "Clear allocator state" | Drop / mark-free all outstanding allocations in one shot — much faster than per-allocation free |
| Internal mutability | "Mutate through shared reference" | Cell/RefCell let you mutate through `&self`; the type system tracks soundness |
| Lifetime elision | "Auto-inferred lifetime" | The compiler infers lifetime annotations from common patterns |

## Further Reading

- *Programming Rust* by Blandy/Orendorff/Tindall — Chapter 9 has an arena example.
- [The bumpalo readme](https://github.com/fitzgen/bumpalo) — practical patterns.
- *Designing Data-Intensive Applications* — chapter on arena-style memory management in storage engines.
