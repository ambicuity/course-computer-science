# Splay Trees & Treaps

> Two BSTs that achieve balance without an explicit invariant: splay trees by self-adjustment, treaps by randomized priorities. Different philosophies, same O(log n) result.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L08-L10 (BST, rotations, balanced trees)
**Time:** ~75 minutes

## Learning Objectives

- Implement a **splay tree**: amortized O(log n) per access; rotates the accessed node to the root.
- Implement a **treap**: BST by key + heap by random priority. Expected O(log n) without amortization.
- Understand why both work: splay's potential-function argument; treap's "tree built by uniformly random insertion order" result.
- Pick the right tree for the workload: splay for working-set locality; treap for split/join O(log n).

## The Problem

AVL and RB trees enforce balance via local invariants and rotations. Two alternatives reach O(log n) by giving up the invariant:

- **Splay trees** (Sleator & Tarjan, 1985) do NO invariant maintenance. Every access *splays* the accessed node to the root via rotations. Amortized O(log n) per op via a potential argument.
- **Treaps** (Seidel & Aragon, 1996) tag each node with a random priority. The tree is a BST on key AND a max-heap on priority. Randomization gives expected O(log n) — adversary can't construct a bad case without seeing the priorities.

Both achieve O(log n) without explicit balance invariants. Each has unique strengths.

## The Concept

### Splay trees

After every search/insert/delete, **splay** the touched node to the root. Splaying is a sequence of rotations that moves a node up to the root while approximately halving the tree's height.

Three splay cases (let x = node, p = parent, g = grandparent):

1. **Zig** (g doesn't exist): rotate x up to root via single rotation at p.
2. **Zig-zig** (x and p are same-side children): rotate g, then p. NOT two single rotations — order matters.
3. **Zig-zag** (x and p are opposite-side children): rotate p, then g.

```
zig-zig (LL case)         zig-zag (LR case)
       g                          g
      / \                        / \
     p   D       →              x   D
    / \                        / \
   x   C                      p   C
  / \                        / \
 A   B                      A   B
```

Implementation: recursive splay, walking up after a normal BST find/insert. ~50 lines in C.

**Why amortized O(log n)**: define potential Φ(tree) = Σ log(size(node)). Each splay step costs ≤ 3·(rank change) + 1; summing telescopes. The full proof is in Sleator-Tarjan 1985; it's one of the more beautiful results in data-structure analysis.

**Working-set bound**: accessing the same k elements repeatedly costs O(k + log n) per access amortized, not O(log n). Hence splay trees adapt to access patterns: frequently-used keys naturally migrate near the root. This is splay's killer feature — used in caches and the early-2000s Win98 file system.

**Drawback**: every read mutates the tree → no thread-safe-read without locking. Eliminates splay from many use cases.

### Treaps

Each node has (key, priority). Priority is random — typically a 32-bit value drawn uniformly at insert time.

```c
typedef struct Node {
    int key, prio;
    struct Node *left, *right;
} Node;
```

Invariants:
- BST on key (left.key < self.key < right.key).
- Max-heap on prio (self.prio ≥ left.prio AND self.prio ≥ right.prio).

Insert: insert as in BST, then **rotate up** while priority violates heap. Each rotation moves the node up one level; expected O(log n) rotations.

Delete: rotate the target node *down* via the higher-priority child until it becomes a leaf; remove.

**Why expected O(log n)**: the tree structure depends only on the priority order, not the insertion order. So the tree is "as if we inserted in priority-decreasing order" — a random insertion sequence. The expected depth of a random BST is O(log n). The adversary controlling the keys can't influence the structure.

**Killer feature: split/join in O(log n)**. Split(T, k) splits T into T₁ (keys ≤ k) and T₂ (keys > k) — useful in interval data structures. Join(T₁, T₂) merges (when all of T₁ < all of T₂). Both via rotations. This makes treaps the data structure of choice for online interval problems.

### Comparison

|  | Splay | Treap |
|---|-------|-------|
| Balance | Amortized | Expected (randomized) |
| Mutates on read | Yes | No |
| Thread-safe reads | No | Yes |
| Split / join | Possible but complex | Native O(log n) |
| Working-set bound | Yes | No |
| Used in production | OpenBSD malloc; SQLite | Boost; competitive prog. |

## Build It

`code/main.c`:

1. **Splay tree** with `splay()`, `insert()`, `find()`, `delete()`. Test: 10K random ops; verify BST invariant.
2. **Treap** with rotate-on-insert, rotate-on-delete. Test: balanced height on adversarial sorted input.

`code/main.py` mirrors with cleaner code.

`code/main.rs` uses `Option<Box<Node>>`.

### Run

```sh
clang -O2 -fsanitize=address main.c -o sp && ./sp
```

## Use It

- **OpenBSD's `malloc(3)`**: per-thread arena tracked by a splay tree on size class.
- **SQLite's b-tree**: not strictly splay, but similar self-adjusting heuristic.
- **Boost `Polygon` / interval trees**: built on treap-like structures for O(log n) split.
- **Competitive programming**: treaps are the go-to for any problem requiring split/merge/persistent sequences.

## Read the Source

- [Sleator & Tarjan: Self-Adjusting Binary Search Trees](https://www.cs.cmu.edu/~sleator/papers/self-adjusting.pdf) — original 1985 paper.
- [Seidel & Aragon: Randomized Search Trees (1996)](https://faculty.washington.edu/aragon/pubs/rst96.pdf) — treap paper.
- [OpenBSD `lib/libc/stdlib/malloc.c`](https://cvsweb.openbsd.org/cgi-bin/cvsweb/src/lib/libc/stdlib/malloc.c) — splay tree in production.

## Ship It

This lesson ships **`outputs/treap.h`** — single-header treap with insert/delete/find.

## Exercises

1. **Easy.** Implement `find(T, k)` for both splay (which mutates the tree by splaying) and treap (no mutation). Note the difference in iterator stability.
2. **Medium.** Build a "frequency counter" of words in a large text using a splay tree. Frequent words should bubble to near-root; profile the working set.
3. **Hard.** Implement `split(Treap *T, int k)` and `join(Treap *A, Treap *B)` in O(log n) each. Then use them to support insertion at an arbitrary index in O(log n) (an "implicit treap").

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Splay | "Rotate to root" | Sequence of rotations that brings a node to root while halving its depth contributions |
| Amortized analysis | "Average across ops" | Worst-case TOTAL work for n ops divided by n, not per-op worst case |
| Priority | "Random tag" | Per-node random value; treap heap-orders on this |
| Implicit treap | "Treap with order-statistics" | Treap that supports kth-element + insert-at-index in O(log n) |
| Working set | "Recent keys" | Set of nodes accessed in last k operations |

## Further Reading

- *Data Structures and Network Algorithms* by Tarjan — splay tree analysis.
- *Randomized Algorithms* by Motwani & Raghavan, Ch. 8 — randomized search trees.
- [Treaps in competitive programming](https://cp-algorithms.com/data_structures/treap.html) — practical recipe + extensions.
