# Equivalence Relations & Partitions

> Every "two things are essentially the same" you've ever written — modular arithmetic, type isomorphism, cache-key collision — is an equivalence relation. They're the formal mechanism for "treat-as-equal."

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 01, Lesson 04
**Time:** ~45 minutes

## Learning Objectives

- State the three axioms of an equivalence relation (reflexive, symmetric, transitive) and recognize each in real code.
- Compute equivalence *classes* and the *quotient set* from a relation; build a partition from an equivalence relation and vice versa.
- Compute the equivalence closure of an arbitrary relation by closing under R, S, T.
- Connect equivalence relations to concrete CS structures: union-find, hashing buckets, modular arithmetic, structural type equality.

## The Problem

"These two things are equal." That sentence is everywhere in CS, and it almost never means *literally* equal. It means equal *modulo some equivalence:*

- Two integers are equal mod 7 iff their difference is divisible by 7.
- Two graphs are equal up to relabeling iff one is isomorphic to the other.
- Two cache keys collide iff `hash(a) == hash(b)`.
- Two strings are equal under Unicode normalization NFC iff their NFC forms match.
- Two types are equal in structural type systems iff they expose the same fields with the same types (recursively).

Every one of those is an equivalence relation. This lesson is the algebra of "treat-as-equal" plus a Python implementation of the canonical algorithm for it: union-find.

## The Concept

### Definition

A binary relation R on a set A is an **equivalence relation** iff it is:

| Property | Formally | Reads as |
|----------|----------|----------|
| **Reflexive**  | ∀a ∈ A. a R a | Everything is related to itself |
| **Symmetric**  | ∀a, b. a R b ⇒ b R a | If a is like b, then b is like a |
| **Transitive** | ∀a, b, c. (a R b ∧ b R c) ⇒ a R c | Likeness chains |

We write `a ≡ b` (mod R) or `a ~ b`. The three axioms together are what `==` would mean if you allowed yourself to invent it.

### Equivalence classes

For `a ∈ A`, the **equivalence class** of a is `[a] = { x ∈ A : x ~ a }`. Two important consequences of the axioms:

1. Every element belongs to exactly one equivalence class (`a ∈ [a]` by reflexivity; and if a is in two classes [b] and [c] then b ~ a ~ c, so [b] = [c] by transitivity + symmetry).
2. The set of all equivalence classes is a **partition** of A: a collection of non-empty, pairwise disjoint subsets whose union is A.

The **quotient set** `A/~` is the set of equivalence classes — one element per "essentially same" group.

### Partition ↔ Equivalence relation

There is a **bijection** between equivalence relations on A and partitions of A:

- Given an equivalence relation, the classes form a partition.
- Given a partition, "in the same block" is an equivalence relation.

This is the formal version of "telling things apart" and "grouping things together" being two sides of the same operation.

### Concrete examples

| Set | Relation | Equivalence classes |
|-----|----------|---------------------|
| ℤ | `a ≡ b mod n` (same remainder mod n) | {[0], [1], …, [n-1]} — exactly n classes |
| Strings | `s ≡ t` iff `lower(s) == lower(t)` | each class = all case-variants of a fixed lowercased form |
| Database rows | `r₁ ≡ r₂` iff same primary key | each class = one logical row (possibly seen across versions) |
| Graphs | `G ≡ H` iff isomorphic | each class = one graph "shape" |
| Procedures | "compute the same function on every input" | each class = one observational behavior |

### Equivalence closure

If you have a relation R that isn't yet an equivalence relation, the **equivalence closure** is the smallest equivalence relation containing R:

```
EquivClosure(R) = TransitiveClosure(SymmetricClosure(ReflexiveClosure(R)))
```

You take reflexive closure first (cheap), then symmetric, then transitive. (Doing transitive before symmetric also works but with extra steps to reclose.)

This is what union-find computes incrementally: each `union(a, b)` adds `(a, b)` and re-closes under transitivity (by union-by-rank with path compression). The classes are tracked as a forest of trees, one per class.

### Union-Find (preview of Phase 03)

Naive: store classes as a list of lists. `find(a)` scans every list. `union(a, b)` finds both classes and merges.

Smart: store each element as a node with a "parent" pointer. `find(a)` walks up to the root (the class representative). `union(a, b)` joins the two roots. With *path compression* during find (point every walked node directly at the root) and *union by rank* (always join the shorter tree under the taller), the amortized cost per operation is O(α(n)) — inverse Ackermann, essentially constant.

We'll build the smart version in Phase 03; this lesson uses a simpler explicit version.

## Build It

Open `code/main.py`.

### Step 1: Check the three axioms

```python
def is_equivalence(R, A):
    return (
        all((a, a) in R for a in A) and                                # reflexive
        all((b, a) in R for (a, b) in R) and                           # symmetric
        all((a, c) in R                                                # transitive
            for (a, b1) in R for (b2, c) in R if b1 == b2)
    )
```

### Step 2: Build classes from an equivalence relation

A simple union-by-rep approach: assign each element a representative; for each pair `(a, b) ∈ R`, unify their representatives. Group elements by final representative.

### Step 3: Partition ↔ relation round-trip

Given a partition, build the equivalence relation as "(a, b) for each pair in the same block." The round-trip `from_partition(classes(R)) == R` holds for any equivalence relation R.

### Step 4: Equivalence closure of arbitrary R

```python
equivalence_closure(R, A) = transitive_closure(symmetric_closure(reflexive_closure(R, A)))
```

Verify: applying it to an arbitrary R gives back a relation that satisfies all three axioms.

### Step 5: Modular arithmetic

`x ≡ y (mod n) ⇔ (x - y) is divisible by n`. There are exactly n classes mod n, named `[0], [1], …, [n-1]`. Run the code — it produces exactly n.

## Use It

- **Union-Find / Disjoint Set Union (Phase 03 L16):** the canonical algorithm for maintaining equivalence classes dynamically. Used in Kruskal's MST (Phase 04), in incremental SAT solvers, in static analyses.
- **Type equality:** structural type equality is an equivalence; nominal type equality is a strict-but-correct subset. Phase 18.
- **Database normalization:** functional dependencies induce equivalence classes on tuples; canonical-form rewriting is computing the quotient. Phase 10.
- **Cache coalescing in compilers (CSE, GVN):** expressions are partitioned by "same value at runtime"; equivalent expressions share a single stored result. Phase 08.
- **Hashing:** `hash(a) == hash(b)` is an equivalence relation; collisions are "the same class for distinct elements." Lessons 03 L05–06.

## Read the Source

- *Naive Set Theory* (Halmos), §5 — exact treatment of partitions and quotients.
- [Tarjan's 1975 paper on Union-Find with path compression](https://dl.acm.org/doi/10.1145/321879.321884) — original analysis of the inverse Ackermann bound.
- [TAOCP Vol 1, §2.3.3](https://www-cs-faculty.stanford.edu/~knuth/taocp.html) — Knuth on equivalence classes and union-find.

## Ship It

This lesson ships **`outputs/dsu.py`** — a small disjoint-set-union implementation with union-by-rank + path compression, the structure that makes equivalence-relation maintenance O(α(n)) amortized.

## Exercises

1. **Easy.** Show that "has the same age" is an equivalence relation on people, but "is taller than" is not.
2. **Medium.** Implement `mod_classes(n)` from the lesson and verify there are exactly n classes for n ∈ {2, 3, 5, 7}.
3. **Hard.** Given an arbitrary directed graph G, compute its strongly connected components (SCCs). The SCCs partition the vertices; the relation "u and v are in the same SCC" is an equivalence relation. Verify the partition property.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Equivalence relation | "Treat-as-equal" | A binary relation that is reflexive, symmetric, and transitive |
| Equivalence class | "A group of equal things" | All elements related to a given representative under the relation |
| Partition | "A grouping" | A collection of non-empty, pairwise-disjoint sets that cover A |
| Quotient set A/~ | "What's left after merging" | The set of equivalence classes — one element per "essentially the same" |
| Equivalence closure | "The R you get by enforcing the three axioms" | Smallest equivalence relation containing R: reflexive + symmetric + transitive closure |

## Further Reading

- *Concrete Mathematics* (Graham, Knuth, Patashnik), §5.2 — modular equivalence with full proofs.
- [CLRS](https://mitpress.mit.edu/9780262046305/introduction-to-algorithms/) — Chapter 21 on disjoint-set data structures.
- [Mac Lane — *Categories for the Working Mathematician*](https://link.springer.com/book/10.1007/978-1-4757-4721-8) — the category-theoretic view: quotients as coequalizers.
