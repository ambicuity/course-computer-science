# Sparse Tables & RMQ

> O(n log n) preprocessing → O(1) range-min queries. The fastest static-array range query when updates aren't needed.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L17 (segment tree)
**Time:** ~60 minutes

## Learning Objectives

- Implement a **sparse table** for range minimum query (RMQ) — O(n log n) build, O(1) query.
- Identify "idempotent operations" (min, max, gcd, AND, OR) that admit O(1) sparse-table queries.
- Compare sparse table with segment tree: when each wins.
- Sketch the **O(n) preprocessing, O(1) RMQ** algorithm by reducing RMQ to LCA on a Cartesian tree.

## The Problem

Given a STATIC array (no updates), answer many range-minimum queries: `query(l, r) = min(A[l..r])`. Used in:

- LCA preprocessing (LCA reduces to RMQ on Euler tour).
- Suffix-array-based string algorithms (longest common extension).
- Computational geometry (kd-trees for static data).

If you need updates, use a segment tree (O(log n) per op). If the array is static AND the operation is idempotent (min, max, gcd), a sparse table beats segment tree for queries — O(1) vs O(log n).

## The Concept

### Idempotent operation

An operation ⊕ is idempotent if `x ⊕ x = x`. Examples: min, max, gcd, AND, OR. The key property: you can OVERLAP two ranges to cover any range, and the answer doesn't change.

### Construction (O(n log n))

`sparse[k][i] = min(A[i .. i + 2^k - 1])` — the minimum over a range of length 2^k starting at i.

Build by doubling:

```c
for (int i = 0; i < n; ++i) sparse[0][i] = A[i];
for (int k = 1; (1 << k) <= n; ++k)
    for (int i = 0; i + (1 << k) - 1 < n; ++i)
        sparse[k][i] = min(sparse[k-1][i], sparse[k-1][i + (1 << (k-1))]);
```

Memory: O(n log n). Time: O(n log n).

### Query (O(1))

For range [l, r]: let `k = floor(log2(r - l + 1))`. Then `[l, l+2^k)` and `(r-2^k+1, r]` together cover [l, r] (with overlap). For idempotent min, overlap is fine:

```c
int rmq(int l, int r) {
    int k = log2_table[r - l + 1];
    return min(sparse[k][l], sparse[k][r - (1 << k) + 1]);
}
```

The `log2_table[]` precomputation (O(n)) makes this branch-free.

### Why idempotent matters

For sum: overlapping ranges would double-count. So sparse table doesn't work for sum — back to segment tree / BIT.

For min: min(x, x) = x, so overlap is harmless. Sparse table is O(1).

### Sparse table vs segment tree

| | Sparse table | Segment tree |
|---|--------------|--------------|
| Build | O(n log n) | O(n) |
| Memory | O(n log n) | O(n) |
| Query | O(1) | O(log n) |
| Updates | NO | O(log n) |
| Operations | Idempotent only | Any monoid |

For RMQ on static data: sparse table wins. For dynamic: segment tree.

### The O(1) RMQ result

Bender-Farach (2000) proved RMQ can be done in O(n) preprocessing + O(1) query. The trick: reduce RMQ on the array to LCA on a Cartesian tree, then LCA to RMQ on the Euler tour, then exploit ±1-RMQ structure for sub-block O(1) queries.

The constants are huge — sparse tables are usually preferred in practice unless n > 10⁶. The Bender-Farach result is mostly theoretical.

## Build It

`code/main.c`:

1. Sparse table for RMQ.
2. Verify against a naïve O(n) per query on a 1000-element array.
3. Bench: 10K queries on N=1M, compare with segment tree.

`code/main.py` mirrors with cleaner code.

`code/main.rs` standard idiomatic Rust.

### Run

```sh
clang -O2 main.c -o sparse && ./sparse
```

## Use It

- **LCA preprocessing**: every LCA query reduces to RMQ on the Euler-tour array.
- **Static range queries in OLAP databases**: precompute sparse tables for min/max columns.
- **Suffix array operations**: longest common extension between any two suffixes via RMQ on LCP array.
- **Competitive programming**: standard tool when updates aren't required.

## Read the Source

- *Bender-Farach 2000*: "The LCA Problem Revisited" — sparse table approach.
- [Competitive Programmer's Handbook](https://cses.fi/book/) — sparse table chapter.

## Ship It

This lesson ships **`outputs/sparse_table.h`** — single-header RMQ.

## Exercises

1. **Easy.** Build a sparse table for range MAX instead of min (one-line change).
2. **Medium.** Range-GCD sparse table. (Note: gcd is idempotent because gcd(x,x)=x.) Useful for "longest subarray with gcd ≥ k" problems.
3. **Hard.** Implement Bender-Farach's O(n) preprocessing for ±1-RMQ (where adjacent elements differ by ±1). Used in LCA implementations.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| RMQ | "Range min query" | Find min of A[l..r] |
| Idempotent | "f(x,x)=x" | Lets overlapping ranges cover any range without double-counting |
| Sparse table | "Power-of-2 precompute" | sparse[k][i] = ⊕ over 2^k elements starting at i |
| LCA | "Lowest common ancestor" | Deepest node ancestor of both a and b |
| Cartesian tree | "Tree from array" | Min-at-root recursive tree of an array; LCA in it = RMQ on the array |

## Further Reading

- Bender-Farach 2000 — "The LCA Problem Revisited".
- *Competitive Programmer's Handbook* by Antti Laaksonen.
