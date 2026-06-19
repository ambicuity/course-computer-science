# Amortized Analysis Deep — Aggregate, Accounting, Potential

> Some operations look expensive worst-case but are dirt cheap on average. Amortized analysis proves this rigorously — and the three methods (aggregate, accounting, potential) give you increasingly powerful lenses.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 04 lessons 01–28
**Time:** ~60 minutes

## Learning Objectives

- Apply all three amortized analysis methods to concrete data structures.
- Define potential functions and derive amortized cost bounds.
- Analyze splay trees, union-find with path compression, and Fibonacci heaps.

## The Problem

Worst-case says dynamic array doubling is O(n). Splay tree access is O(n). Union-find with path compression is... unclear. These bounds are technically correct and practically useless — the expensive operations happen so rarely that the per-operation *average* is tiny.

Average-case requires probability distributions you don't have. Amortized analysis asks: what is the cost per operation averaged over *any* sequence of n operations, worst-case sequence included?

## The Concept

Amortized cost is **not** average-case. It is a guarantee: for *any* sequence of n operations, total cost is bounded. Three methods exist, from simplest to most powerful.

### Method 1: Aggregate Method

Prove upper bound T(n) on total cost of n operations. Amortized = T(n) / n.

**Dynamic array with doubling:** n insertions cost n. Doublings at sizes 1, 2, 4, ..., n/2 cost 1+2+4+...+n/2 ≈ n. Total ≈ 2n. Amortized per insert = **O(1)**.

Simple but gives a single uniform bound — cannot assign different costs to different operation types.

### Method 2: Accounting (Banker's) Method

Charge each operation an *amortized cost* (may differ from actual). Surplus accumulates as credit on data structure objects. Credit ≥ 0 always.

**Dynamic array:** charge each insert 3 units — 1 for insertion, 1 stored on the element for its future copy, 1 stored on an existing element. At doubling from k to 2k, the k existing elements carry exactly k credits for their copies.

**Multi-pop stack:** charge `push` at 2 (1 for push, 1 credit for eventual pop). Charge `pop` and `multi_pop(k)` at 0 — credits on popped elements pay. Every operation: amortized **O(1)**.

### Method 3: Potential Method

Define potential Φ(state) ≥ 0. Amortized cost:

```
ĉᵢ = cᵢ + Φ(Dᵢ) − Φ(Dᵢ₋₁)
```

Sum over n operations telescopes: Σ ĉᵢ = Σ cᵢ + Φ(Dₙ) − Φ(D₀). If Φ(D₀) = 0 and Φ(Dₙ) ≥ 0, the amortized bound holds. Most powerful — used in published proofs.

**Binary counter:** Φ = number of 1-bits. Increment flips trailing 1s to 0 (ΔΦ = −k) then 0 to 1 (ΔΦ = +1). Net ΔΦ = 1−k. Amortized = (1+k) + (1−k) = **2 = O(1)**.

### Key Applications

**Splay trees:** self-adjusting BST. Access lemma with Φ = Σ log(size(v)) proves amortized **O(log n)** per operation via zig-zig/zig-zag rotations decreasing Φ.

**Union-Find + path compression:** Tarjan 1975 proves m operations on n elements cost **O(m · α(n))** total, where α is the inverse Ackermann function. Effectively O(1) per operation.

**Fibonacci heap:** Φ = t(H) + 2·m(H) (trees + 2×marked nodes). `decrease-key` in **O(1) amortized** — cascading cuts absorbed by potential. Makes Dijkstra O(V log V + E).

### Comparison

| Method | Per... | Needs global state? | Power |
|--------|--------|--------------------|-------|
| Aggregate | op (uniform) | No | Low |
| Accounting | op type | No | Medium |
| Potential | op (flexible) | Yes (Φ) | High |

## Build It

### Step 1: Binary Counter with Cost Tracking

```python
def binary_counter_increment(counter):
    flips = 0
    i = 0
    while i < len(counter) and counter[i] == 1:
        counter[i] = 0
        flips += 1
        i += 1
    if i < len(counter):
        counter[i] = 1
        flips += 1
    else:
        counter.append(1)
        flips += 1
    return flips
```

### Step 2: Multi-Pop Stack (Accounting)

```python
class AmortizedStack:
    CHARGE = 2
    def __init__(self):
        self._data, self._credit = [], 0
    def push(self, val):
        self._data.append(val)
        self._credit += self.CHARGE - 1
    def pop(self):
        self._data.pop()
        self._credit -= 1
    def multipop(self, k):
        count = min(k, len(self._data))
        for _ in range(count): self._data.pop()
        self._credit -= count
```

### Step 3: Splay Tree and Union-Find

Full implementations with operation counters are in `code/main.py`.

## Use It

**Linux kernel:** Page cache uses self-adjusting tree structures. Frequently accessed pages stay near the root.

**Kruskal's MST:** Sorts edges by weight, unions endpoint sets. With union-by-rank + path compression, each union/find is effectively O(1) amortized, giving O(E log E) total.

**Fibonacci heaps in Dijkstra:** Boost Graph Library uses Fibonacci heaps for O(1) decrease-key, saving a log V factor on sparse graphs.

**Dynamic arrays:** Python `list`, C++ `std::vector`, Java `ArrayList` — all use doubling. O(1) amortized insert is why they work.

## Read the Source

- CPython `Objects/listobject.c` — `list_resize()` implements growth factor with amortized comments.
- Boost `boost/heap/fibonacci_heap.hpp` — production Fibonacci heap.
- Linux `lib/rbtree.c` — self-balancing tree in kernel.

## Ship It

The artifact in `outputs/` is **an amortized analysis toolkit** — binary counter, multi-pop stack, splay tree, union-find with operation counters and cost visualization.

## Exercises

1. **Easy** — Prove binary counter amortized cost O(1) via accounting method. Assign charge per increment and show credit never negative.

2. **Medium** — Define potential function Φ for splay tree proving access lemma: amortized cost of splaying node at depth d is O(log n). Show each splay step decreases Φ by ≥ 1.

3. **Hard** — Implement Dijkstra with binary heap and Fibonacci heap. Generate random graphs at varying density. Compare wall-clock time — at what density does Fibonacci heap win?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Amortized cost | "average over sequence" | Upper bound on total cost / n, for *any* sequence — not probabilistic |
| Aggregate method | "divide total by n" | T(n) ≤ f(n), amortized = f(n)/n. Uniform bound. |
| Accounting method | "banker's method" | Charge per op type; surplus = credit on objects; credit ≥ 0 |
| Potential method | "energy argument" | Φ(state) ≥ 0; amortized = actual + ΔΦ; sum telescopes |
| Access lemma | "splay tree theorem" | Splaying node at depth d costs O(log n) amortized, Φ = Σ log(size(v)) |
| Inverse Ackermann | "α(n)" | Grows slower than log*. Union-find: O(α(n)) amortized |
| Cascading cut | "Fib decrease-key" | Cutting node may trigger parent cuts; potential absorbs cost |

## Further Reading

- T. Cormen et al., *Introduction to Algorithms* (CLRS), Chapter 17.
- D. Sleator and R. Tarjan, "Self-Adjusting Binary Search Trees," JACM 1985.
- R. Tarjan, "Efficiency of a Good But Not Linear Set Union Algorithm," JACM 1975.
- M. Fredman and R. Tarjan, "Fibonacci Heaps and Their Uses," JACM 1987.
