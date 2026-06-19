# Heaps & Priority Queues

> Pop-min in O(log n), no full sort. The data structure behind Dijkstra, A*, Huffman coding, kernel scheduling, and online merge.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L01 (dynamic array)
**Time:** ~75 minutes

## Learning Objectives

- Implement a **binary heap** (array-backed) with push, pop, peek, build-heap-from-array.
- Master sift-up and sift-down — the two primitives.
- Implement **heapsort** in O(n log n) with O(1) extra space.
- Understand when to upgrade: **Fibonacci heap** (theoretical O(1) decrease-key), **pairing heap** (Fibonacci's pragmatic cousin used in real Dijkstras).

## The Problem

You have n elements arriving over time; you need the minimum (or maximum) one at any moment, possibly with new arrivals interleaved with extractions.

- **Sorted array**: insert O(n), extract-min O(1).
- **Unsorted array**: insert O(1), extract-min O(n).
- **Balanced BST**: insert O(log n), extract-min O(log n) — overkill for "just want min".
- **Binary heap**: insert O(log n), extract-min O(log n), build O(n) — perfect fit.

Heaps appear everywhere: Dijkstra's algorithm; A* pathfinding; Huffman coding; event-driven simulators; OS task schedulers (Linux's CFS uses a red-black tree but with heap-like ordering on vruntime); top-k queries; merge of k sorted streams.

## The Concept

### Binary heap

A binary heap is an array A where for every i:

- `A[i] ≤ A[2i+1]` (left child)
- `A[i] ≤ A[2i+2]` (right child)

(For a min-heap. Max-heap reverses ≤ to ≥.)

It's a complete binary tree stored in array form: children of index i are at 2i+1 and 2i+2; parent is (i-1)/2. No pointers needed.

### sift_up(i)

When a new element is inserted at the end (index i = n), it may violate the heap order vs its parent. Sift it up:

```c
while (i > 0) {
    p = (i - 1) / 2;
    if (A[p] <= A[i]) break;
    swap(A[p], A[i]);
    i = p;
}
```

O(log n).

### sift_down(i)

When the root is removed (replaced by the last element), sift it down to its proper place:

```c
while (true) {
    l = 2*i + 1; r = 2*i + 2;
    smallest = i;
    if (l < n && A[l] < A[smallest]) smallest = l;
    if (r < n && A[r] < A[smallest]) smallest = r;
    if (smallest == i) break;
    swap(A[i], A[smallest]);
    i = smallest;
}
```

O(log n).

### build_heap

Naïve: insert n elements, each O(log n) → O(n log n).

Floyd's trick: starting from index n/2 - 1 down to 0, sift_down each. Total work: **O(n)**. The proof uses the fact that nodes at depth d cost O(h-d), and Σ d=0..h (#nodes at d) · (h-d) = O(n).

### heapsort

1. build_heap on the array (O(n)).
2. for i from n-1 down to 1: swap A[0] with A[i] (move max to end), then sift_down on A[0..i-1].

In-place sort, O(n log n) worst-case. Slower than quicksort in practice (worse cache locality, more swaps), but **worst-case guaranteed** — useful in real-time systems.

### Fibonacci heap

A heap-ordered forest of trees with lazy reorganization:

- insert: O(1) amortized — just add to a root list.
- decrease-key: O(1) amortized — cut node out, add to root list.
- extract-min: O(log n) amortized — clean up at this point.

Theoretical breakthrough (Fredman & Tarjan, 1984): made Dijkstra's algorithm O(m + n log n) instead of O((m+n) log n). The asymptotic optimum for any priority-queue-based shortest-path algorithm.

**But almost nobody uses Fibonacci heaps in production.** The constants are too large; cache behavior is poor. Pairing heap, d-ary heap, and even simple binary heap usually beat it in wall-clock time.

### Pairing heap

A simpler variant with the same amortized bounds (almost; decrease-key complexity is still an open problem). Used in real Dijkstra implementations:

```c
typedef struct PNode { int key; struct PNode *child, *sibling; } PNode;

PNode *meld(PNode *a, PNode *b) {
    if (!a) return b; if (!b) return a;
    if (a->key < b->key) { b->sibling = a->child; a->child = b; return a; }
    a->sibling = b->child; b->child = a; return b;
}
```

Beautifully simple. Used in Boost, GCC's loop optimizer, and competitive programming.

### d-ary heap

Binary heap generalized: each node has d children. With d = 4 or 8:

- sift_up costs less per step (you climb less, since the tree is shorter).
- sift_down costs more per step (you compare d children to pick the smallest).

Optimal: d ≈ degree-of-decrease-key (theoretical), or whatever fits in a cache line. The decrease-key heavy Dijkstra implementations use d = 4.

## Build It

`code/main.c`:

1. Binary heap (min) with push, pop, peek, build_heap.
2. Floyd's O(n) build_heap verified against the heap invariant.
3. Heapsort.
4. Bench: heapsort vs qsort on 1M random integers.

`code/main.py` mirrors and uses the stdlib `heapq` for comparison.

`code/main.rs` uses `std::collections::BinaryHeap` (a max-heap, so wrap values in `Reverse` for min behavior).

### Run

```sh
clang -O2 main.c -o heap && ./heap
python3 main.py
```

## Use It

- **Dijkstra's algorithm**: priority queue keyed by tentative distance.
- **A\***: same, keyed by f = g + h.
- **Huffman coding**: build the optimal prefix code by repeatedly extracting the two smallest-frequency nodes.
- **Top-k**: maintain a min-heap of size k over a stream.
- **Event-driven sim**: events ordered by timestamp.
- **Linux CFS scheduler**: tasks ordered by virtual runtime (RB-tree but heap semantics).

## Read the Source

- [Python's `heapq` module](https://github.com/python/cpython/blob/main/Lib/heapq.py) — pure-Python clean binary heap; very readable.
- [Rust `std::collections::BinaryHeap`](https://github.com/rust-lang/rust/blob/master/library/alloc/src/collections/binary_heap/mod.rs).
- *Boost.Heap* — has pairing, binomial, Fibonacci, d-ary; good comparative reference.

## Ship It

This lesson ships **`outputs/heap.h`** — single-header binary min-heap (int keys).

## Exercises

1. **Easy.** Implement `top_k(int *arr, int n, int k)` — find the k largest using a min-heap of size k. O(n log k), not O(n log n).
2. **Medium.** Implement a **d-ary heap** with d as a runtime parameter; benchmark insert and extract-min over {2, 4, 8}. Plot the curve.
3. **Hard.** Implement a **pairing heap** and use it inside Dijkstra's algorithm (you'll meld pairing heaps to simulate decrease-key cheaply). Compare runtime with a plain binary-heap Dijkstra on a random sparse graph of 100K vertices.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Min-heap | "Smallest at top" | Tree with parent ≤ children invariant |
| Sift up / down | "Bubble" | Restore heap property by swapping with parent or smallest child |
| Build heap | "Heapify" | Convert array to heap in O(n) via Floyd's method |
| Heapsort | "In-place sort via heap" | O(n log n) worst-case; slower than quicksort in practice |
| Amortized O(1) | "Average constant" | Total work for n ops is O(n), individual ops can be O(log n) |

## Further Reading

- *Introduction to Algorithms* (CLRS) Ch. 6 — binary heaps; Ch. 19 — Fibonacci heaps.
- *Algorithms* (Sedgewick) Ch. 2.4 — heap-based priority queues.
- [Fredman-Tarjan 1984 Fibonacci heap paper](https://dl.acm.org/doi/10.1145/28869.28874) — the classic.
