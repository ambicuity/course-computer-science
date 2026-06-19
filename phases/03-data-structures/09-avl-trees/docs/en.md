# AVL Trees

> The first self-balancing BST. Strict height balance via at most two rotations per operation. Guaranteed O(log n).

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L08 (BST + rotations)
**Time:** ~75 minutes

## Learning Objectives

- Implement an AVL tree with height-balanced insert and delete.
- Identify the four rebalance cases — LL, LR, RR, RL — and the rotation each one demands.
- Prove that an AVL tree's height is ≤ 1.44 log₂(n+2) — the *exact* bound.
- Decide when to choose AVL over red-black or B-tree (read-heavy, latency-sensitive).

## The Problem

Plain BSTs degrade to O(n) under sorted input. AVL trees (Adelson-Velsky & Landis, 1962) were the first solution: maintain the strict invariant **|height(left) − height(right)| ≤ 1** at every node. The invariant is restored after every insert and delete by 1 or 2 rotations.

The cost: extra per-node bookkeeping (height or balance factor) and slightly more work per insert/delete than a red-black tree. The benefit: the AVL tree's height is tighter — about 1.44 log n vs RB's 2 log n — so lookups are slightly faster.

## The Concept

### Invariant

For every node N: `balance_factor(N) = height(left) - height(right) ∈ {-1, 0, +1}`.

`height(NULL)` is defined as 0; height of a leaf is 1.

### Insert: walk down, then up

1. Insert as in plain BST (recursion descends, hangs a leaf).
2. As recursion *returns* up the path, at each ancestor: update height; check balance factor.
3. If |bf| > 1: rotate to fix; return the new subtree root.

### The four rebalance cases

Let N be the unbalanced node (|bf(N)| = 2). Look at the child you came from to decide which case:

| Case | bf(N) | bf(child) | Fix |
|------|-------|-----------|-----|
| **LL** | +2 | ≥ 0 | rotate_right(N) |
| **LR** | +2 | < 0 | rotate_left(N.left); rotate_right(N) |
| **RR** | -2 | ≤ 0 | rotate_left(N) |
| **RL** | -2 | > 0 | rotate_right(N.right); rotate_left(N) |

The double rotations (LR, RL) appear because a single rotation doesn't fix a "zigzag" — you must straighten before rotating.

After a single rebalance, the AVL invariant is restored at N and at every ancestor (because the subtree's height is now what it was before the insert that triggered the imbalance). So **insert** does **at most 1 rebalance**; **delete** can chain rebalancing all the way up (in the worst case 1 per level).

### Why 1.44 log₂(n+2)?

Define F(h) = minimum number of nodes in an AVL tree of height h.

```
F(1) = 1
F(2) = 2
F(h) = 1 + F(h-1) + F(h-2)
```

This is the Fibonacci recurrence! So F(h) = Fib(h+2) − 1.

Inverting: h ≤ log_φ(n+2) − 2 ≈ 1.44 log₂(n+2).

This is the tightest bound for any height-balanced tree. Red-black trees are looser (2 log n) but cheaper to maintain.

### Insert example

Insert 1, 2, 3:

```
1                1            2
 \                \          / \
  2     becomes    2   →    1   3
                    \
                     3
                     ↑
                  imbalance at root (bf=-2, child bf=-1 → RR case → rotate_left)
```

### Delete: walk down, then up

Like insert, but the height of the subtree might decrease, propagating imbalance higher. Worst case: O(log n) rotations.

## Build It

`code/main.c` implements an AVL tree with insert, delete, contains, and height-tracking. Tests:

1. Sequential insert 1..1000 — height ≤ 14 (proving balance).
2. Random insert × 10K — height ≤ 17.
3. Adversarial insert 1..N then delete N..1 — height stays log-bounded.

`code/main.py` mirrors with cleaner code.

`code/main.rs` uses `Option<Box<Node>>` with height field.

### Run

```sh
clang -O2 -fsanitize=address main.c -o avl && ./avl
```

## Use It

- **GNU `libavl`** (Plauger / Knuth) — the classic AVL library; few uses, but archetypal.
- **WAVL trees** in academic papers — a relaxation that's nearly as fast as AVL with simpler rebalance.
- AVLs are *rarely* used in modern production — RB trees, B-trees, and skip lists dominate. AVL remains pedagogically critical for understanding rotations and amortized rebalancing.

## Read the Source

- [GNU libavl source](http://adtinfo.org/libavl.html/index.html) — Ben Pfaff's beautifully documented AVL library.
- [Sedgewick's lecture on AVL](https://algs4.cs.princeton.edu/33balanced/) — clean pseudocode walkthrough.

## Ship It

This lesson ships **`outputs/avl.h`** — single-header AVL tree.

## Exercises

1. **Easy.** Add `avl_height(Tree*)` that returns the current height by walking root only (constant time if you cache it on the node).
2. **Medium.** Implement `avl_kth(Tree*, int k)` — k-th smallest — by augmenting each node with subtree size.
3. **Hard.** Implement AVL `merge(T1, T2)` where all keys in T1 < all keys in T2, in O(log n) time. Hint: descend the taller tree's right spine until heights match, then join.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Balance factor (bf) | "Imbalance amount" | height(left) − height(right); ∈ {-1, 0, +1} in an AVL |
| LL/LR/RR/RL | "Rebalance cases" | Combinations of where in the tree the imbalance occurred |
| Fibonacci tree | "Worst AVL" | The minimum-node AVL of height h has Fib(h+2)-1 nodes |
| Height-balanced | "Strict balance" | |bf| ≤ 1 invariant; AVL's defining property |

## Further Reading

- *Adelson-Velsky & Landis 1962* — the original paper (in Russian; translations available).
- *Introduction to Algorithms* (CLRS) — Problem 13-3 covers AVL.
- *Algorithms by Sedgewick & Wayne* — comparison of AVL, RB, splay.
