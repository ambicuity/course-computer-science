# Indexing — B+ Trees in DBs

> The index structure that makes "SELECT * FROM users WHERE age BETWEEN 25 AND 35" fast — by storing data only in leaves and linking them together.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 10 lessons 01–06 (buffer pool, slotted pages, physical storage)
**Time:** ~90 minutes

## Learning Objectives

- Explain why B+ Trees dominate database indexing over BSTs, hash tables, and B-Trees
- Implement insertion, search, and range scan on a B+ Tree from scratch in Rust
- Articulate how fanout reduces tree height and why that matters for page-oriented storage
- Distinguish clustered vs unclustered indexes and predict their performance on range queries
- Trace how PostgreSQL's nbtree and InnoDB's clustered index differ

## The Problem

You have a table with 10 million customer records. You need to run `SELECT * WHERE age BETWEEN 25 AND 35`. Without an index, the database does a sequential scan — reading every page from disk. At 100 MB/s sequential read, a 10 GB table takes ~100 seconds.

A binary search tree on age would help: `O(log N)` lookups. But BSTs have terrible locality. Each node is heap-allocated, scattered across virtual memory. Traversing a BST of 10M nodes touches 24+ random pages, each requiring a disk seek (~10 ms). A single lookup costs ~240 ms. Range scans are worse — you'd need to walk parent pointers back and forth, visiting scattered nodes.

A B-Tree improves on this by packing many keys per node (high fanout). But B-Trees store data in every node, making range scans awkward (you must tree-traverse). The B+ Tree solves this decisively: **all data lives in leaves, leaves are linked**, and internal nodes are pure routing. Range scans become a linear walk through a linked list of leaf pages.

## The Concept

### B+ Tree vs BST vs B-Tree

| Property | BST | B-Tree | B+ Tree |
|----------|-----|--------|---------|
| Keys per node | 1 | up to `d` | up to `d` |
| Data location | every node | every node | leaves only |
| Leaf linkage | no | no | yes (sibling pointers) |
| Height for 10⁷ rows | ~24 | 3–4 | 3–4 |
| Range scan cost | O(N log N) | O(N log N) | O(log N + K) |

The height advantage comes from **fanout**. A B+ Tree of order `d` has at most `d` keys per internal node, so each internal node has `d+1` children. Height is roughly `log_{d+1}(N)`. For `d=200` (a typical page-based order) and `N=10⁷`: height ≈ `log_{201}(10⁷)` ≈ 3. So any lookup touches at most 3 internal nodes plus 1 leaf = 4 pages.

### Structure

```
Internal node:
┌─────┬─────┬─────┬─────┐
│ k₀  │ k₁  │ k₂  │ ... │  (routing keys)
└──┬──┴──┬──┴──┬──┴──┬──┘
   c₀    c₁    c₂    c₃   (child pointers, one more than keys)

Leaf node:
┌─────┬─────┬─────┬─────┬──────────┐
│ k₀  │ k₁  │ k₂  │ ... │ next ───────► next leaf
├─────┼─────┼─────┼─────┤          │
│ v₀  │ v₁  │ v₂  │ ... │          │
└─────┴─────┴─────┴─────┴──────────┘
```

### Insert

1. Navigate to the leaf that should contain the key (using binary search at each internal node).
2. Insert the key-value pair in sorted position.
3. If the leaf now exceeds `order` entries: **split** — divide into two leaves, promote the smallest key of the right leaf to the parent.
4. If the parent now exceeds `order` keys (internal node split): **cascade** — promote the median key upward.
5. If the root splits: create a new root above it.

### Delete

1. Navigate to the leaf, remove the key-value pair.
2. If the leaf has enough entries (≥ ceil(order/2)): done.
3. If under capacity: try to **redistribute** (borrow an entry from a sibling), then update the parent separator key.
4. If no sibling can spare: **merge** the leaf with a sibling, remove the separator from the parent, and cascade up.

### Bulk Loading

Instead of inserting one by one (which causes many splits), bulk loading sorts all data first, then builds the tree from leaves up:

1. Sort all key-value pairs by key.
2. Pack them into leaf nodes (order entries each), linking leaves together.
3. For each run of `order+1` leaf pointers: build an internal node above them, promote the first key of each page.
4. Repeat until one root remains.

### Clustered vs Unclustered Indexes

**Clustered index** (InnoDB primary key): The leaf pages of the B+ Tree store the full row data. The table is the index. Range scans are fast because data is stored in key order. At most one clustered index per table.

**Unclustered index** (secondary index): The leaf pages store `(key, primary_key)` pairs. To fetch a row, the database must first find the key in the secondary index, then look up the primary key in the clustered index — a **double lookup**. Range scans that return many rows are much slower because each row may require a random I/O.

```
Clustered:   [leaf: key → full row]    ← one I/O per row
Unclustered: [leaf: key → PK] → [leaf: PK → full row]  ← two I/Os per row
```

### Composite Indexes

A B+ Tree on `(a, b, c)` sorts by `(a, b, c)` lexicographically. The **leftmost prefix rule**: a query can only use the index if it filters on `a`, or `(a, b)`, or `(a, b, c)`. A filter on `b` alone cannot use the index because the tree is sorted by `a` first.

## Build It

We'll build a B+ Tree in Rust over three steps: the core structure and insert, then range scans and delete, then tests.

### Step 1: Node and Tree Structure

```
code/main.rs
```

The full implementation in `code/main.rs` defines:

- `Node<K,V>` enum with `Internal` and `Leaf` variants.
- `BPlusTree<K,V>` struct with `order` and `root`.
- Recursive `insert` with split propagation.
- `get` via binary search at each level.

### Step 2: Range Scan and Delete

Range scanning walks the tree in-order. Our implementation finds the first child that could contain the start key, then visits all subsequent children. This is correct even without sibling pointers, though a production version uses leaf-level `next` pointers for O(K) scans.

Delete uses recursive descent: find the leaf, remove the entry, and propagate `Removed` upward when a leaf becomes empty. Internal nodes remove a child and a separator key when the child reports `Removed`, collapsing to promote a sole remaining child.

### Step 3: Tests

Run with `cargo test`:

- Integer and string key insert + get
- Range scans across partial and full key sets
- Delete from leaf, delete not found, delete all
- Large insert (1000 entries) to exercise splits

## Use It

### PostgreSQL B-Tree (nbtree)

PostgreSQL's `nbtree` implementation lives in `src/backend/access/nbtree/`. Key features ours doesn't have:

- **Metapage** (page 0): stores the root location, version info, and fast-root for single-value lookups.
- **Deduplication**: when many rows have the same key (e.g., a low-cardinality column), nbtree compresses duplicate keys into a single entry with a tid array, saving space.
- **Bloat management**: `VACUUM` reclaims space from deleted index entries; `btree` uses a "page deletion" protocol that requires a vacuum cycle.
- **Concurrency**: nbtree uses Lehman & Yao's B-link protocol with high-concurrency page locks, sibling pointers for right-links, and "half-dead" page states during deletion.

Our implementation uses `Box<Node>` for child pointers; PostgreSQL uses page IDs (integers) into a shared buffer pool, which enables pinning, dirty flags, and eviction.

### InnoDB Clustered Index

MySQL/InnoDB's primary key _is_ a clustered B+ Tree. The leaf pages store the full row (all columns). Secondary indexes store `(key, primary_key)`. A lookup via a secondary index does:

1. B+ Tree probe on the secondary index → get primary key.
2. B+ Tree probe on the clustered index → get the full row.

This double lookup is the reason InnoDB recommends short primary keys (an auto-increment integer is ideal; UUIDs bloat every secondary index).

## Read the Source

- **PostgreSQL nbtree**: `src/backend/access/nbtree/nbtinsert.c` — `_bt_doinsert` shows the full insert path with split handling.
- **InnoDB row fetch**: `storage/innobase/btr/btr0cur.cc` — `btr_cur_search_to_nth_level` is the core B+ Tree search function.
- **Rust `std::collections::BTreeMap`**: `library/alloc/src/collections/btree/` — a production B+ Tree variant in Rust's standard library.

## Ship It

The reusable artifact for this lesson is the in-memory B+ Tree library at:

- `code/main.rs` — a generic, tested B+ Tree that can be extracted and reused in later phases (the MVCC KV store in the Phase 10 capstone could use it as a primary index).
- `outputs/` — copy `main.rs` plus a `Cargo.toml` to produce a standalone library crate.

## Exercises

1. **Easy** — Add a `len()` method that returns the number of entries without traversing all leaves (hint: maintain a count field in `BPlusTree`).
2. **Medium** — Replace the recursive insert with an iterative version using a `path: Vec<&mut Node>` stack. Measure the performance difference.
3. **Hard** — Implement tree-based range scan using leaf `next` pointers instead of full traversal. Add a `BPlusTreeIterator` that yields `(K,V)` pairs in order.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Order | The size of a node | Maximum number of keys per node, which determines fanout and tree height. |
| Fanout | Branching factor | Number of child pointers per internal node (`order + 1`). High fanout → short tree. |
| Clustered index | The table itself | Leaf pages store full rows. Reordering the index reorders the table. |
| Double lookup | Two B+ Tree probes | Secondary index returns PK, then PK index returns the row. |
| Leftmost prefix | Leading columns matter | A composite index on `(a,b)` can only be used for `WHERE a=...` or `WHERE a=... AND b=...`, not `WHERE b=...`. |

## Further Reading

- [PostgreSQL B-Tree Index Docs](https://www.postgresql.org/docs/current/btree-intro.html) — The official nbtree chapter.
- [CMU 15-445 B+ Trees Lecture](https://www.youtube.com/watch?v=K1a2Bk8NrYQ) — Andy Pavlo walks through B+ Tree mechanics with slides and examples.
- [Rust BTreeMap source](https://doc.rust-lang.org/src/alloc/collections/btree/map.rs.html) — The standard library's B+ Tree variant, with navigator types and cursor-based iteration.
