# Red-Black Trees

> The balanced BST you've actually been using your whole programming life. Inside `std::map`, `TreeMap`, `set`, Linux's CFS scheduler, and Postgres B-tree page splits.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L08, L09 (BST, rotations, AVL)
**Time:** ~90 minutes

## Learning Objectives

- State the **five red-black invariants** and prove they imply height ≤ 2 log₂(n+1).
- Implement RB insert with the three insertion-fixup cases (uncle-red flip; uncle-black single rotation; uncle-black double rotation).
- Sketch RB delete fixup (six cases — known to be the hardest data-structure code in CLRS).
- Choose RB over AVL or B-tree given a workload: writes, reads, cache pressure, page-aligned storage.

## The Problem

Red-black trees relax the AVL invariant: instead of "height-balanced" (|bf| ≤ 1), they require "color-balanced" (every root-to-leaf path has the same number of black nodes). This allows shorter, faster rebalance — at most 3 rotations per insert and 3 per delete vs AVL's up to log n per delete.

Result: red-black is THE balanced-BST of production C++, Java, Linux kernel, Rust collections (until SwissTable replaced HashMap), and Postgres. If you've used a sorted set in any of these, you've used RB.

## The Concept

### Five invariants

1. Every node is RED or BLACK.
2. The root is BLACK.
3. Every NIL leaf is BLACK. (NIL = the implicit "off the bottom" sentinel.)
4. **A RED node's children are both BLACK** ("no two reds in a row").
5. **Every root-to-NIL path has the same number of BLACK nodes** (the "black height").

### Why this bounds height

Invariant 4 + 5: along any path, RED and BLACK nodes alternate at worst. Half the nodes on a path are BLACK. So if black-height is b, total height ≤ 2b. And black-height ≤ log₂(n+1) (because each black layer at least doubles the subtree size).

⇒ **height ≤ 2 log₂(n+1)**.

Looser than AVL (1.44 log₂) but still O(log n). The cost of the relaxation: ~30% taller in practice. The benefit: less work per rebalance.

### Insert fixup — three cases

After plain BST insert of a RED leaf, walk back up; while the parent is RED (violating invariant 4):

| Case | Uncle | Fix |
|------|-------|-----|
| **1** | RED | Recolor: parent + uncle → BLACK, grandparent → RED. Continue from grandparent. |
| **2** | BLACK, "zig-zag" | Rotate parent (LR or RL): turns zig-zag into straight line; falls through to case 3. |
| **3** | BLACK, straight | Rotate grandparent + swap colors of parent and grandparent. Done. |

After case 3, the tree is balanced and the loop terminates. Case 1 can propagate up O(log n) levels but does no rotations — only recoloring.

After the loop, paint the root BLACK (in case case 1 turned it RED).

### Delete fixup — six cases (sketch)

Delete starts like BST delete (replace with inorder successor when two children). The deleted node had a color: if RED, no fixup needed (RED doesn't affect black-height). If BLACK, we've removed a black from one path, violating invariant 5. We fix this by propagating an "extra black" upward through one of six configurations (mirror pairs for left/right). Cases are:

1. Sibling RED → rotate to recolor.
2. Sibling BLACK, both nephews BLACK → recolor sibling RED, propagate.
3. Sibling BLACK, far nephew BLACK, near nephew RED → rotate-recolor, fall to case 4.
4. Sibling BLACK, far nephew RED → rotate grandparent, recolor. Done.

Implementing RB delete correctly is the canonical "you got this in your interview, congrats" data structure exercise. Linux's lib/rbtree.c is ~300 lines of nested cases.

### Sentinel trick

Most RB implementations use a single sentinel NIL node (BLACK) for "off the bottom" instead of NULL checks. Cleans up the code dramatically. Linux uses `rb_node *` directly and a parent pointer (so it can handle NIL with offset arithmetic). C++ libstdc++ uses an explicit sentinel.

### Red-black vs AVL

|  | AVL | RB |
|---|-----|----|
| Height bound | 1.44 log n | 2 log n |
| Insert rotations | 1 max | 2 max |
| Delete rotations | log n max | 3 max |
| Best for | Read-heavy | Mixed / write-heavy |
| In production? | Rarely | Java/C++/Linux defaults |

The constants matter: RB does fewer total rotations across an insert+delete pair. For workloads with mutation, this wins.

## Build It

`code/main.c` implements:

1. Recursive RB tree with insert + insert-fixup (3 cases).
2. Delete (full 6-case fixup, well-commented).
3. Invariant verifier: checks root-is-black, no-red-red, equal-black-height on every path.
4. Stress test: 10K random insert/delete, verify invariants after each.

`code/main.py` implements a clean recursive RB tree.

`code/main.rs` uses parent pointers with unsafe — `std::collections::BTreeMap` is the recommended Rust collection (RB requires unsafe).

### Run

```sh
clang -O2 -fsanitize=address main.c -o rb && ./rb
```

## Use It

- **C++ `std::map` / `std::set`** (libstdc++, libc++, MSVC STL).
- **Java `TreeMap` / `TreeSet`**.
- **Linux kernel `lib/rbtree.c`** — used in CFS scheduler, memory mgmt (vm_area_struct rb), epoll's interest set.
- **Postgres** internally for some interval-overlap structures.

When you need O(log n) ordered ops on a small-to-medium in-memory dataset, you almost always end up on red-black underneath an interface.

## Read the Source

- [Linux `lib/rbtree.c`](https://github.com/torvalds/linux/blob/master/lib/rbtree.c) — the canonical iterative RB. Used in scheduling, memory, fs. Read with `include/linux/rbtree.h` open.
- [CLRS Ch. 13](https://mitpress.mit.edu/9780262046305/introduction-to-algorithms/) — the reference proof + case analysis.
- [Sedgewick's left-leaning red-black trees](https://www.cs.princeton.edu/~rs/talks/LLRB/LLRB.pdf) — simplification with only 3 (not 6) delete cases. Used in newer Java and many tutorials.

## Ship It

This lesson ships **`outputs/rbtree.h`** — a single-header RB tree with insert + delete + verify.

## Exercises

1. **Easy.** Add `rb_min(Tree*)`, `rb_max(Tree*)` — leftmost / rightmost node, O(log n).
2. **Medium.** Implement RB **iterator** — successor of a node in O(1) amortized (O(log n) worst-case): if right child exists, go to leftmost there; else walk up until you came from a left child.
3. **Hard.** Implement left-leaning RB (Sedgewick, 2008) and compare line count vs full RB.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Color | "Red/black bit" | One-bit per node tracking the invariant; ~1 byte cost per node |
| Black-height | "BH" | Number of BLACK nodes from a given node to any NIL leaf; same for all paths from that node |
| Sentinel NIL | "Nil node" | Single shared BLACK node used for all NULL pointers; simplifies code |
| LLRB | "Left-leaning RB" | Sedgewick's 2008 variant; fewer cases; same asymptotic guarantees |
| Splay-tree alternative | (different idea) | Splay trees achieve amortized O(log n) without color bits |

## Further Reading

- *Bayer 1972* — the original "symmetric binary B-trees" paper, ancestors of RB.
- *Guibas & Sedgewick 1978* — RB tree's actual debut.
- *Okasaki: Purely Functional Red-Black Trees* — functional variant in ~20 lines of ML.
