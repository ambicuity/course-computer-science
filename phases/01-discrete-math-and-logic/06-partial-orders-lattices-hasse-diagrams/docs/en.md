# Partial Orders, Lattices, Hasse Diagrams

> Some things can be compared and some can't. The math of "sometimes comparable" runs schedulers, version resolvers, and program analyzers.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 01, Lessons 04–05
**Time:** ~60 minutes

## Learning Objectives

- Distinguish partial orders, total orders, and strict orders by their axioms; recognize each in real code.
- Draw and read Hasse diagrams of small posets, including divisibility and subset orderings.
- Identify least upper bound (join, ⊔) and greatest lower bound (meet, ⊓); recognize when a poset is a lattice.
- Apply topological sort to produce a linear extension of any DAG-shaped poset; cite the three or four CS settings where this is the dominant idea.

## The Problem

Total orders (≤ on integers, lexicographic on strings) are everywhere — and they're the easy case. Real CS lives on *partial* orders, where some pairs are incomparable:

- Build systems: file `foo.o` depends on `foo.c`; file `bar.o` depends on `bar.c`; they're independent. The dependency relation is a partial order.
- Concurrent events: in distributed systems, two events on different machines may have no causal order (Lamport's vector clocks; Phase 11). The "happens-before" relation is a partial order.
- Information flow / security lattices: "public ≤ secret ≤ top-secret" — but a *category* dimension (Crypto, Personnel) might be incomparable. The classification lattice is a partial order.
- Type subtyping: `Cat ≤ Animal`, `Dog ≤ Animal`, but `Cat` and `Dog` are unrelated.

Total orders' tools (sorting, binary search) don't directly apply. Partial orders need their own machinery: Hasse diagrams to visualize, topological sort to linearize, joins and meets to find "the smallest thing above both" / "the largest thing below both" — operations that appear in version resolvers, dataflow analyzers, and DB query planners.

## The Concept

### Axioms

A binary relation R on A is a **partial order** (poset, ≤) iff:

| Property | Formally |
|----------|----------|
| **Reflexive**   | ∀a. a ≤ a |
| **Antisymmetric** | a ≤ b ∧ b ≤ a ⇒ a = b |
| **Transitive**  | a ≤ b ∧ b ≤ c ⇒ a ≤ c |

A **strict partial order** (<) drops reflexivity and replaces antisymmetry with *irreflexivity*: ¬(a < a). Every partial order has a strict version (`a < b ⇔ a ≤ b ∧ a ≠ b`) and vice versa.

A **total order** is a partial order with the extra axiom: ∀a, b. a ≤ b ∨ b ≤ a (every pair is comparable).

### Hasse diagrams

A picture of a poset that hides reflexivity (no self-loops) and transitivity (no edges implied by chains). You draw an upward edge from x to y iff `x < y` and there's *no* z with `x < z < y` ("y covers x").

```
Subset poset on 𝒫({a,b,c}):

         {a,b,c}
        / | \
     {a,b}{a,c}{b,c}
       | X    X  |
       {a} {b} {c}
        \  |  /
          ∅
```

Reading: `{a} ⊆ {a, b}` is drawn as an edge; `{a} ⊆ {a, b, c}` is *not* drawn because it's implied by the chain through `{a, b}`.

### Common posets you'll meet

| Poset | Order |
|-------|-------|
| Divisibility on ℕ | `a | b` iff a divides b |
| Subset on 𝒫(S) | `A ⊆ B` |
| Topological sort of a DAG | `u ≤ v` iff there's a path from u to v |
| String containment | `s ≼ t` iff s is a (contiguous) substring of t |
| Subtyping in OOP | `S ≤ T` iff S is a subtype of T |

### Joins and meets — when they exist

For two elements x, y in a poset:
- **Upper bounds** of {x, y}: every z with x ≤ z and y ≤ z.
- **Least upper bound** (LUB, join, x ⊔ y): an upper bound that's ≤ every other upper bound. Unique if it exists.
- Dually: **lower bounds**, **greatest lower bound** (GLB, meet, x ⊓ y).

A **lattice** is a poset in which *every pair* x, y has a join and a meet.

- 𝒫(S) with ⊆ is a lattice: join = ∪, meet = ∩.
- ℕ with divisibility is a lattice: join = lcm, meet = gcd.
- A DAG might *not* be a lattice (two events might have multiple incomparable least upper bounds).

A **complete lattice** has joins and meets for *every* subset, not just pairs (𝒫(S) is complete; ℕ-divisibility is not — the join of all primes doesn't exist as a natural number).

### Topological sort: linearizing a poset

A **linear extension** of a poset is a total order that contains it. For finite posets this always exists; the algorithm is *topological sort*:

```
queue ← elements with no predecessors
while queue not empty:
    pick u from queue
    emit u
    for each v that u "covers" (immediate successor):
        remove the edge u → v
        if v has no predecessors left: queue ← v
if any node remains: the poset isn't a DAG (cycle).
```

Topo sort runs in O(V + E). It's the basis of:

- Build systems (Make, Bazel): build `foo` only after every prereq of `foo` builds.
- Spreadsheet recalculation: evaluate cells in dependency order.
- Package managers: install dependencies before dependents.
- Course prerequisites: emit a valid degree plan.
- Instruction scheduling in compilers: emit instructions respecting data dependencies.

### Antichains and chains

- **Chain**: a subset totally ordered by ≤. Length of the longest chain = "depth" of the poset.
- **Antichain**: a subset of pairwise incomparable elements. Largest antichain = "width."

**Dilworth's theorem**: in any finite poset, the minimum number of chains needed to cover it equals the size of the largest antichain. This is a clever duality used in scheduling proofs.

## Build It

Open `code/main.py`.

### Step 1: Axiom check

```python
def is_partial_order(R, A):
    return (
        all((a, a) in R for a in A) and
        not any((a, b) in R and (b, a) in R and a != b for a in A for b in A) and
        all((a, c) in R for (a, b) in R for (b2, c) in R if b == b2)
    )
```

### Step 2: Hasse cover relation

A pair `(a, b)` is a *cover* iff `a < b` AND there's no `c` with `a < c < b`:

```python
def covers(R, A):
    strict = {(a, b) for (a, b) in R if a != b}
    return {(a, b) for (a, b) in strict
            if not any((a, c) in strict and (c, b) in strict for c in A)}
```

### Step 3: Join and meet

```python
def upper_bounds(x, y, R, A):
    return {z for z in A if (x, z) in R and (y, z) in R}

def join(x, y, R, A):
    ubs = upper_bounds(x, y, R, A)
    candidates = [z for z in ubs if all((z, u) in R for u in ubs)]
    return candidates[0] if len(candidates) == 1 else None
```

Verify on the subset poset 𝒫({a, b, c}) with ⊆.

### Step 4: Topological sort

Kahn's algorithm in 15 lines. Use it to produce a linear extension of any DAG.

### Step 5: Divisibility lattice

```python
A = range(1, 31)
R = {(a, b) for a in A for b in A if b % a == 0}      # a divides b
# join = lcm, meet = gcd
import math
assert all(join(a, b, R, set(A)) == math.lcm(a, b)
           for a in A for b in A if a < 12 and b < 12)
```

## Use It

- **Make / Bazel / Cargo / npm:** Each computes a topo sort of the dependency DAG before running anything (Phase 00, Lesson 06).
- **Spreadsheets** like Excel: when you edit a cell, dependent cells recalc in topo order; cycles trigger #CIRC! errors.
- **Distributed systems' vector clocks** (Phase 11): a partial order on events; two events with incomparable vector clocks are "concurrent."
- **Static analyses** (dataflow lattices in Phase 08/15): the "join" of two abstract values is the least conservative common upper bound; analyses iterate until they reach a fixed point in a lattice.
- **Version selection** (npm / Cargo): choose latest version of each dep subject to upper bounds; the version graph is a poset.

## Read the Source

- *Enumerative Combinatorics* by Stanley, Vol 1, Ch. 3 — the canonical reference on posets.
- [Davey & Priestley — *Introduction to Lattices and Order*](https://www.cambridge.org/core/books/introduction-to-lattices-and-order/40C9F8C2F540B5D0E62FB87DD86E4D85) — short, clear, with CS examples.
- [Kahn's topological sort original paper (1962)](https://dl.acm.org/doi/10.1145/368996.369025) — half a page.

## Ship It

This lesson ships **`outputs/poset.py`** — a small library: `is_partial_order`, `covers`, `topological_sort`, `is_lattice`, `join`, `meet`. Reusable in any later lesson involving DAGs.

## Exercises

1. **Easy.** Draw the Hasse diagram of divisibility on {1, 2, 3, 4, 6, 12}. How many pairs are in the cover relation?
2. **Medium.** Implement Kahn's topo sort with cycle detection. Run on a 100-node random DAG; assert the output is a valid linear extension.
3. **Hard.** Implement Dilworth's theorem: take a finite poset and produce both the maximum antichain *and* a minimum chain cover; verify their sizes match.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Partial order | "Order with gaps" | Reflexive + antisymmetric + transitive relation |
| Total order | "A sort key" | Partial order in which every pair is comparable |
| Hasse diagram | "The picture" | The graph of cover relations — no implied edges drawn |
| Join (⊔) | "Smallest above" | Least upper bound of two elements; not always defined |
| Lattice | "A 'nice' poset" | A poset where every pair has both a join and a meet |
| Topo sort | "DAG order" | A linear extension of a poset; computed in O(V + E) |

## Further Reading

- *Concrete Mathematics* (Graham, Knuth, Patashnik), Ch. 7 — generating functions for poset enumeration.
- [The MIT 6.042J textbook](https://ocw.mit.edu/courses/6-042j-mathematics-for-computer-science-spring-2015/) — Chapter on partial orders has many CS-flavored examples.
- *Static Program Analysis* (Möller, Schwartzbach) — Chapter 4 on lattices in dataflow analysis.
