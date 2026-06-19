# Phase Capstone — A Generic DS Library in Rust with Invariants

> Phase 3 in one crate: dynamic array, linked list, hash map, min-heap, AVL tree — all generic over `<T>` and invariant-checked.

**Type:** Capstone
**Languages:** Rust
**Prerequisites:** All of Phase 3
**Time:** ~120 minutes

## Learning Objectives

- Integrate the data structures of Phase 3 — Vec, linked list, hash map, heap, balanced BST — into one consistent generic Rust library.
- Use **traits + generics** to provide a uniform API (`Collection`, `OrderedCollection`, `Map`).
- Encode invariants as `debug_assert!` calls; verify them after every mutation in debug builds.
- Run an integration test: insert 100K items, perform a mix of operations, verify every structure's invariant.

## The Problem

Across Phase 3 we built 24 structures in isolation. Real systems compose them: a hash map of vectors, a min-heap of (key, value) pairs, a graph using adjacency lists. The capstone packages a curated subset into one Rust crate with:

- One trait surface (`Collection<T>`, `OrderedCollection<T>`, `Map<K, V>`).
- Generic over element type.
- Invariants checked in debug builds via `debug_assert!`.
- Tests that exercise every structure.

This is the canonical exercise of going from "implements an algorithm" to "ships a library." The skills carry to phase 5+ where many of these structures appear as building blocks of larger systems.

## The Concept

### API surface

```rust
pub trait Collection<T> {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }
    fn clear(&mut self);
}

pub trait OrderedCollection<T: Ord>: Collection<T> {
    fn insert(&mut self, value: T);
    fn contains(&self, value: &T) -> bool;
    fn remove(&mut self, value: &T) -> bool;
}

pub trait Stack<T>: Collection<T> {
    fn push(&mut self, value: T);
    fn pop(&mut self) -> Option<T>;
    fn peek(&self) -> Option<&T>;
}

pub trait Map<K, V> {
    fn insert(&mut self, key: K, value: V) -> Option<V>;
    fn get(&self, key: &K) -> Option<&V>;
    fn remove(&mut self, key: &K) -> Option<V>;
}
```

### Implementations

- `DynVec<T>` — dynamic array with amortized doubling growth.
- `LinkedList<T>` — singly linked list with head/tail pointers, behind safe `Box`.
- `HashMap<K, V>` — open-addressing Robin-Hood (from L05) over the `Hash` trait.
- `MinHeap<T>` — binary heap over `Ord`.
- `AvlSet<T>` — height-balanced BST.

### Invariants

| Structure | Invariant | Checked when |
|-----------|-----------|--------------|
| DynVec | len ≤ cap | every push/pop |
| HashMap | load < 0.9 OR resize triggered | every insert |
| MinHeap | A[i] ≤ A[2i+1] && A[i] ≤ A[2i+2] for all i | after every push/pop in debug |
| AvlSet | |h(left)−h(right)| ≤ 1 for every node | after every insert/delete in debug |

Each invariant is encoded as `debug_assert!` so release builds pay nothing.

### Integration test

`tests/integration.rs` inserts 10K items into every structure, performs 100K random ops, and verifies:

- DynVec sorts after `sort_unstable`.
- HashMap returns all inserted keys.
- MinHeap returns elements in sorted order.
- AvlSet maintains balance.

## Build It

`code/main.rs` is the demo binary that exercises every collection.

`Cargo.toml` declares the crate. (Already set up at the course root.)

```sh
rustc -O code/main.rs -o capstone && ./capstone
```

## Use It

This library is reused in Phase 4 (Algorithms) — Dijkstra uses MinHeap, BFS uses DynVec as queue, Kruskal uses external DSU. Phase 5 (Theory) uses HashMap for memoization.

## Read the Source

After completing this lesson, compare your designs to:

- [Rust `std::collections`](https://doc.rust-lang.org/std/collections/) — the production hierarchy.
- [crossbeam-rs](https://github.com/crossbeam-rs/crossbeam) — concurrent variants.
- [`im` crate](https://docs.rs/im/) — persistent versions.

## Ship It

This capstone ships **`outputs/lib.rs`** — the integrated library, ready to embed in other Phase 3+ projects.

## Exercises

1. **Easy.** Add `Iterator` implementation for `DynVec<T>` and `AvlSet<T>` (inorder).
2. **Medium.** Add a `BTreeSet<T>` (m=6 B-tree) implementation conforming to `OrderedCollection<T>`. Compare with `AvlSet` on insert + range queries.
3. **Hard.** Add concurrent variants: `ConcurrentVec<T>` (Mutex<Vec<T>>) and a hand-rolled lock-free `ConcurrentStack<T>` (Treiber). Bench at 4 / 8 threads.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Trait surface | "API layer" | Set of related traits that downstream code programs against |
| Generic | "Parametric polymorphism" | Code works for any T satisfying the bounds |
| `Ord`, `Hash`, `Clone` | "Trait bounds" | Constraints on T that the impl can use |
| Invariant | "Always-true property" | A condition checked after every mutation |
| `debug_assert!` | "Optional check" | Active in debug, no-op in release |

## Further Reading

- *The Rust Book*, Chapters 10 + 17 — generics and trait objects.
- *Rustonomicon* — unsafe Rust for low-level data structures.
- [Rust `std::collections` source](https://github.com/rust-lang/rust/tree/master/library/alloc/src/collections).
