# B-Trees and B+-Trees

> The shape of every database index, every filesystem directory, every page table at scale. M-ary search trees with key counts tuned to your storage media.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L08 (BST, rotations)
**Time:** ~90 minutes

## Learning Objectives

- Implement a **B-tree** with order m: every internal node has between ⌈m/2⌉ and m children.
- Implement insertion via **split**, deletion via **borrow/merge**.
- Distinguish **B-tree** (data in all nodes) from **B+-tree** (data only in leaves; leaves form a linked list).
- Explain why disks/SSDs want m ≈ 100–500; why in-memory uses m = 16–32 for cache locality.

## The Problem

BSTs fit one key per node — wasteful on storage hardware where the unit of I/O is a page (4 KiB on disk). A B-tree fits many keys per node: with m=256, a 1-billion-key tree has height 4 → 4 disk I/Os per lookup vs ~30 for a binary tree.

This is why every database, filesystem, and storage system uses B-trees underneath.

## The Concept

### B-tree invariants

A B-tree of order m:

1. Every node holds between ⌈m/2⌉ − 1 and m − 1 **keys** (root may have fewer).
2. Every internal node with k keys has k + 1 **children pointers**.
3. All leaves are at the same depth (perfectly balanced!).
4. Within a node, keys are sorted; key i separates child i (keys < key_i) from child i+1.

```
          [10, 20, 30]
         /    |    |    \
   [5,7]  [12,15] [22,25] [33,35]
```

### Insert: split when full

1. Walk down to the leaf where the key belongs.
2. Insert into the leaf, keeping keys sorted.
3. If the leaf now has m keys (overfull), **split**: take the median, push it up to parent. Leaf splits into two leaves of ⌊m/2⌋ keys each.
4. Recurse if parent overflows.
5. If root splits, create a new root → tree grows by 1 in height.

B-trees grow at the **root**, not at the leaves — that's how all leaves stay at the same depth.

### Delete: borrow or merge

- **Borrow**: if a sibling has > ⌈m/2⌉ − 1 keys, rotate one key through the parent.
- **Merge**: otherwise, merge the underfull node with sibling; pull the separating parent key down.

### B+ tree

In a **B-tree**, data lives in all nodes. In a **B+-tree**, data lives ONLY in leaves; internal nodes contain only routing keys; leaves are doubly-linked for sequential scans.

Why B+ won the database race:

1. **Range queries are O(k)** — walk the leaf chain.
2. **Internal nodes are smaller** (no values) → higher fan-out.
3. **No "early exit"** → predictable latency.

Postgres, MySQL InnoDB, SQLite — all B+.

### Choosing m

| Storage | Typical m |
|---------|-----------|
| Disk (4 KiB page, 16 B entry) | 256 |
| SSD | 100–200 |
| In-memory (cache-conscious) | 16–32 |

### Bulk loading

For n sorted keys, build leaves at ~70% fill, then build internal layers bottom-up. O(n) time vs O(n log n) for incremental insert. Postgres's CREATE INDEX uses this.

## Build It

`code/main.c` implements a B-tree with m=4 (small for visibility):

1. Insert with split-on-overflow.
2. Search.
3. Print as ASCII tree showing structure.
4. Verify all-leaves-same-depth invariant.

`code/main.py` implements a B+-tree with linked leaves + range query.

`code/main.rs` discusses Rust's BTreeMap (production B-tree with m=6).

### Run

```sh
clang -O2 main.c -o bt && ./bt
python3 main.py
```

## Use It

- **PostgreSQL** `btree` index AM = B+-tree.
- **MySQL InnoDB**: clustered B+-tree on primary key.
- **SQLite**: B-tree (data in all nodes) for tables; B+ for indices.
- **Rust `std::collections::BTreeMap`**: in-memory B-tree, m=6.
- **Linux btrfs, Apple APFS**: B-tree-based filesystems.

## Read the Source

- [Postgres `src/backend/access/nbtree/`](https://github.com/postgres/postgres/tree/master/src/backend/access/nbtree) — production B+-tree, 30+ years refined.
- [Rust `BTreeMap` source](https://github.com/rust-lang/rust/blob/master/library/alloc/src/collections/btree/node.rs).
- *Modern B-Tree Techniques* by Goetz Graefe — survey paper.

## Ship It

This lesson ships **`outputs/btree.h`** — single-header B-tree (insert + search).

## Exercises

1. **Easy.** Print level-order. Verify every leaf at same depth.
2. **Medium.** Implement range query [lo, hi] → O(log n + k).
3. **Hard.** Bulk-load from sorted array at 70% fill in O(n). Compare with incremental insert.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Order (m) | "Fan-out" | Max children per node; max keys = m-1 |
| Split | "Overflow handling" | Median key pushed to parent; node divided |
| Merge | "Underflow handling" | Combine underfull node with sibling |
| B+ tree | "Data-at-leaves" | Internals are routing-only; leaves linked for range scans |
| Bulk load | "Index build" | Construct tree from sorted input in O(n) |

## Further Reading

- *The Ubiquitous B-Tree* (Comer, 1979).
- *Modern B-Tree Techniques* (Graefe, 2011).
