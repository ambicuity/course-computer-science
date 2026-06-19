# Segment Trees & Fenwick (BIT)

> Range query + point/range update in O(log n). Two classic structures: segment tree (general) and Fenwick tree (clever, half the code, sums only).

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L07 (tree recursion), L13 (heap array layout)
**Time:** ~90 minutes

## Learning Objectives

- Implement an **iterative segment tree** for range-sum + point-update in O(log n).
- Implement a **lazy segment tree** for range-update + range-query.
- Implement a **Fenwick tree (Binary Indexed Tree)**: half the code, only for invertible operations (sum, XOR).
- Choose: segment tree for arbitrary monoids; Fenwick for sums; sparse table for static range-min.

## The Problem

You have an array A of n elements. You want:

- `update(i, x)`: change A[i] to x (or A[i] += x).
- `query(l, r)`: compute sum (or min, or max) of A[l..r].

Naïve: update O(1), query O(n). Or: precompute prefix sums — query O(1), update O(n) (rebuild prefix). Both O(n²) total over n operations.

**Segment tree**: both ops in O(log n) → O(n log n) total.

Appears constantly in competitive programming and a few production places (range-based query systems, OLAP cubes, online order books).

## The Concept

### Segment tree (array-backed)

Represent the array as the leaves of a binary tree; each internal node stores the SUM of its subtree's leaves. The tree has size ~2n.

```
n=8: indices 1..15 (1-indexed)
                  1 (sum of all 8)
              /        \
            2            3
          /   \       /     \
         4     5     6       7
        / \  / \   / \      / \
       8  9 10 11 12 13 14 15
```

`tree[i]` represents a range; tree[1] is the whole array; tree[2] is the left half; tree[3] is the right half; etc.

**Point update**: change A[i], walk from leaf i to root, updating sums. O(log n).

**Range query [l, r]**: recurse from root; for each subtree, three cases — entirely outside (return 0), entirely inside (return tree[v]), partial overlap (recurse on both children). O(log n).

### Iterative segment tree

A clever array-layout trick (Codeforces blog by Atcoder community) gives an iterative implementation:

```c
int t[2 * n];

void update(int i, int x) {
    for (t[i += n] = x; i >>= 1; ) t[i] = t[i << 1] + t[i << 1 | 1];
}

int query(int l, int r) {  /* [l, r) */
    int res = 0;
    for (l += n, r += n; l < r; l >>= 1, r >>= 1) {
        if (l & 1) res += t[l++];
        if (r & 1) res += t[--r];
    }
    return res;
}
```

Six lines for each. The same shape as the heap layout: 2i and 2i+1 are children of i.

### Lazy propagation

For RANGE updates (add x to all of A[l..r]), naïve is O(n). Lazy propagation: store a "pending update" on each node; defer applying to children until a query/update needs them.

```c
void update_range(int v, int vl, int vr, int l, int r, int x) {
    push_down(v, vl, vr);                         /* apply pending */
    if (r < vl || vr < l) return;                 /* no overlap */
    if (l <= vl && vr <= r) {                     /* full cover */
        tree[v] += (vr - vl + 1) * x;
        lazy[v] += x;
        return;
    }
    int m = (vl + vr) / 2;
    update_range(2*v,   vl, m,   l, r, x);
    update_range(2*v+1, m+1, vr, l, r, x);
    tree[v] = tree[2*v] + tree[2*v+1];
}
```

Both range update and range query become O(log n).

### Fenwick tree (BIT)

Half the code, but only handles invertible operations (sum, XOR, sometimes max with care).

```c
int bit[N + 1];

void update(int i, int delta) {
    for (++i; i <= N; i += i & -i) bit[i] += delta;
}

int prefix(int i) {                                 /* sum of A[0..i-1] */
    int s = 0;
    for (; i > 0; i -= i & -i) s += bit[i];
    return s;
}

int range(int l, int r) { return prefix(r) - prefix(l); }   /* [l, r) */
```

The magic: `i & -i` is the lowest set bit. Each node covers `i & -i` elements ending at i. Updates and queries each touch O(log n) nodes.

Fenwick is significantly faster than a segment tree in practice — fewer ops, better cache. But only for prefix-summable operations. For min/max, use segment tree (or sparse table for static).

### Operation types

| Operation | Segment tree | Fenwick (BIT) |
|-----------|--------------|---------------|
| Range sum + point update | ✓ | ✓ (preferred) |
| Range sum + range update (add) | ✓ (lazy) | ✓ (with 2 BITs trick) |
| Range min/max + point update | ✓ | ✗ |
| Range XOR + point update | ✓ | ✓ |
| Arbitrary associative op | ✓ | ✗ |

## Build It

`code/main.c`:

1. Iterative segment tree (range-sum + point-update).
2. Lazy segment tree (range-add + range-sum).
3. Fenwick tree (range-sum + point-add).
4. Benchmark: 1M updates + 1M queries on N=1M for all three.

`code/main.py` mirrors with cleaner code.

`code/main.rs` provides idiomatic Rust segment tree.

### Run

```sh
clang -O2 main.c -o seg && ./seg
```

## Use It

- **Competitive programming**: range queries are the bread-and-butter.
- **OLAP cubes**: pre-aggregated data with multi-dimensional range queries.
- **Genome interval queries**: how many SNPs in interval [start, end]?
- **Order books**: range query of bids/asks within a price range.
- **Time-series stores**: range aggregation over time intervals (TimescaleDB has segment-tree-like indexes).

## Read the Source

- [AtCoder Library `segtree.cpp`](https://github.com/atcoder/ac-library/blob/master/atcoder/segtree.hpp) — production segment tree for monoid operations.
- [BOOST `interval_set`](https://www.boost.org/doc/libs/1_83_0/libs/icl/doc/html/index.html) — alternative for set-of-intervals semantics.
- *Competitive Programmer's Handbook* by Antti Laaksonen — clean exposition of both.

## Ship It

This lesson ships **`outputs/segtree.h`** — iterative segment tree + Fenwick header.

## Exercises

1. **Easy.** Range-sum + point-update segment tree, n=10. Test on a small example.
2. **Medium.** Range-min segment tree (replace + with min in iterative variant). Identity is INT_MAX.
3. **Hard.** Implement a "wavelet tree" or "merge sort tree" — answers "k-th smallest in range" queries in O(log² n). Built once over an array of n values.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Segment tree | "Tree of intervals" | Each node represents a range; operations defined per node |
| Fenwick / BIT | "Binary Indexed Tree" | Cleverer half of segment tree using i & -i indexing |
| Lazy propagation | "Defer the update" | Mark a node as 'will apply this later'; flush before reading children |
| Monoid | "Associative + identity" | The algebraic structure required by segment trees |
| Range update | "Multi-element write" | Update [l, r] in one op via lazy propagation |

## Further Reading

- *Competitive Programmer's Handbook* — best free book on these structures.
- [Codeforces blog: iterative segment tree](https://codeforces.com/blog/entry/18051) — Petr Mitrichev's classic.
- *Fenwick (1994)* — original BIT paper.
