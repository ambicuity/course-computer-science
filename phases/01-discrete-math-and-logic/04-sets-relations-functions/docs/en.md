# Sets, Relations, Functions

> Sets are the substrate for *everything* in mathematics and most of CS. Once you have sets, relations are subsets of products, and functions are special relations. Three concepts, infinite reach.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 01, Lessons 01–03
**Time:** ~60 minutes

## Learning Objectives

- Manipulate sets with union, intersection, difference, complement, Cartesian product, and powerset; compute their cardinalities.
- Define a *relation* as a subset of A × B, and a *function* as a relation with the "single-valued" property.
- Classify a function as injective, surjective, bijective — and prove which class a given function belongs to.
- Compose functions, find inverses (when they exist), and connect this to programming concepts (database joins, dictionaries, type isomorphisms).

## The Problem

Every database table is a relation. Every dictionary in code is a function (key → value). Every type isomorphism in functional programming is a bijection. The same three primitives — sets, relations, functions — show up in:

- SQL: a table is a relation; JOIN is relational composition.
- Type theory: `A → B` is the set of functions; `A × B` is the Cartesian product.
- Algorithms: a graph is a relation E ⊆ V × V.
- Cryptography: a secure hash is a "one-way" function (no efficient inverse).

If your mental model of these objects is fuzzy, every later lesson will be fuzzy too. This lesson sharpens it.

## The Concept

### Sets

A **set** is an unordered collection of distinct elements. We write `S = {1, 2, 3}` or `S = {x ∈ ℤ : x > 0}` (set-builder notation).

| Operation | Symbol | Definition |
|-----------|--------|-----------|
| Membership | `x ∈ S` | x is an element of S |
| Subset | `A ⊆ B` | every element of A is in B |
| Equality | `A = B` | A ⊆ B and B ⊆ A |
| Union | `A ∪ B` | { x : x ∈ A or x ∈ B } |
| Intersection | `A ∩ B` | { x : x ∈ A and x ∈ B } |
| Difference | `A \ B` (or A - B) | { x : x ∈ A and x ∉ B } |
| Symmetric difference | `A △ B` | (A \ B) ∪ (B \ A) |
| Complement | `A^c` or `Ā` | universe \ A (only meaningful with a fixed universe) |
| Cartesian product | `A × B` | { (a, b) : a ∈ A, b ∈ B } |
| Powerset | `𝒫(A)` (or `2^A`) | the set of all subsets of A |

Two important sets:
- `∅` — the empty set. Subset of every set.
- `|A|` — the **cardinality** (size) of A.

Cardinality rules:
- `|A ∪ B| = |A| + |B| - |A ∩ B|` (inclusion-exclusion).
- `|A × B| = |A| · |B|`.
- `|𝒫(A)| = 2^|A|` (this is why powerset explodes — and why finding "any subset matching a constraint" is NP-hard).

### Relations

A **(binary) relation** R from A to B is a subset of A × B. We write `a R b` to mean `(a, b) ∈ R`.

Examples:
- `<` on integers: R = `{(a, b) ∈ ℤ × ℤ : a < b}`.
- "is parent of": `R = {(p, c) : p is a parent of c}`.
- A database table with two columns is exactly a relation.

Composition: if R ⊆ A × B and S ⊆ B × C, then `S ∘ R ⊆ A × C` is `{(a, c) : ∃ b ∈ B. (a, b) ∈ R and (b, c) ∈ S}`. This is exactly a *join* in SQL.

### Functions

A **function** `f : A → B` is a relation `R ⊆ A × B` with two properties:
1. **Total** — for every a ∈ A, there exists at least one b ∈ B with `(a, b) ∈ R`.
2. **Single-valued** — for every a ∈ A, there exists at most one such b.

Combined: for every input there is *exactly one* output. We write `f(a) = b` when `(a, b) ∈ f`.

A **partial function** drops the totality requirement: some inputs map to "undefined." Many CS objects are partial (dictionary lookup, exception-throwing function, file I/O).

### Injective, surjective, bijective

For `f : A → B`:

| Property | Definition | Intuition |
|----------|------------|-----------|
| **Injective** (one-to-one) | `f(a₁) = f(a₂) ⇒ a₁ = a₂` | distinct inputs map to distinct outputs (no collisions) |
| **Surjective** (onto) | `∀b ∈ B. ∃a ∈ A. f(a) = b` | every output is hit |
| **Bijective** | injective and surjective | a perfect pairing between A and B |

Bijections are the "isomorphisms" of sets: they witness `|A| = |B|`. They're also the only functions with two-sided inverses.

### Inverses

If `f : A → B` is bijective, the **inverse** `f⁻¹ : B → A` is the unique function with `f⁻¹(f(a)) = a` and `f(f⁻¹(b)) = b`. For non-bijective functions:
- A *left inverse* (g with g ∘ f = id_A) exists iff f is injective.
- A *right inverse* (g with f ∘ g = id_B) exists iff f is surjective (assuming the axiom of choice in infinite settings).

### Composition

For `f : A → B` and `g : B → C`, `g ∘ f : A → C` is defined by `(g ∘ f)(a) = g(f(a))`. Composition is associative but not commutative. Identity functions act as identity for composition.

```
A ──f──> B ──g──> C
       g ∘ f
```

This is the same diagram as a database query plan, a type-class instance chain, a pipeline.

### Useful identities

- De Morgan for sets: `(A ∪ B)^c = A^c ∩ B^c`, `(A ∩ B)^c = A^c ∪ B^c`.
- Distributivity: `A ∩ (B ∪ C) = (A ∩ B) ∪ (A ∩ C)`.
- `f` injective ⇒ `f(A ∩ B) = f(A) ∩ f(B)` (for all A, B in the domain).

## Build It

### Step 1: Set ops in Python

Python's `set` already implements these efficiently:

```python
A = {1, 2, 3, 4}
B = {3, 4, 5, 6}
A | B        # {1, 2, 3, 4, 5, 6}
A & B        # {3, 4}
A - B        # {1, 2}
A ^ B        # {1, 2, 5, 6}     symmetric difference
A <= B       # False             subset
```

Cartesian product via `itertools.product`. Powerset via a small combinatorial function (in the lesson code).

### Step 2: Relations as sets of pairs

```python
R = {(1, 'a'), (1, 'b'), (2, 'c')}
# Not a function — input 1 has two outputs.
def is_function(R, A):
    return len({a for a, _ in R}) == len(A) and \
           all(sum(1 for x, _ in R if x == a) == 1 for a in A)
```

### Step 3: Compose relations

```python
def compose(R, S):
    """(S ∘ R)(a, c) iff ∃b. (a, b) ∈ R and (b, c) ∈ S"""
    return {(a, c) for (a, b) in R for (b2, c) in S if b == b2}
```

Compare to SQL: `SELECT R.a, S.c FROM R JOIN S ON R.b = S.b` — same operation.

### Step 4: Classify a function

```python
def classify(f_pairs, codomain):
    domain = {a for a, _ in f_pairs}
    image  = {b for _, b in f_pairs}
    injective  = len(image) == len(f_pairs)
    surjective = image == codomain
    return injective, surjective, injective and surjective
```

### Step 5: Invert a bijection

```python
def invert(f_pairs):
    return {(b, a) for (a, b) in f_pairs}
```

`(f⁻¹)⁻¹ = f` — verify on a few examples.

## Use It

- **SQL JOIN, GROUP BY, projection** — all are operations on relations. Set-theoretic algebra is the formal foundation of the relational model (Phase 10).
- **Type isomorphisms** (Phase 18) — `(A × B) → C ≅ A → (B → C)`. Both sides are sets of functions; the isomorphism is a bijection (currying).
- **Cryptographic hash functions** — designed to be efficient one-way *functions* — easy to compute, computationally infeasible to invert. "One-way" isn't a math property; it's a complexity-theoretic one (Phase 12).
- **Type theory** — many type constructors are functorial mappings of sets: Option is `A ↦ A + 1`, List is `A ↦ μX. 1 + A × X`.

## Read the Source

- *Naive Set Theory* by Paul Halmos — short, classic intro; brutally clean.
- *How to Prove It* (Velleman), Chapters 4–5 — sets, relations, functions with proof exercises.
- [SQL relational algebra reference](https://www.cs.utexas.edu/users/kanellakis/sigact-ra.pdf) — the formal mapping between SQL and set theory.

## Ship It

This lesson ships **`outputs/relation_ops.py`** — a small reusable library with `compose`, `inverse`, `is_function`, `classify`, and a `closure` function (used in Lesson 05 for equivalence closures).

## Exercises

1. **Easy.** For A = {1, 2, 3}, list every element of 𝒫(A). Then count: should be 2³ = 8 subsets.
2. **Medium.** Let `f : ℤ → ℤ` be `f(x) = 2x`. Is it injective? Surjective? Bijective? Prove your answers from the definitions.
3. **Hard.** Implement `transitive_closure(R)` for a binary relation R ⊆ A × A using repeated composition (Warshall's algorithm). Verify on a small graph.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Set | "A collection" | An unordered collection of distinct elements; the substrate of math |
| Relation | "How two things relate" | A subset of A × B; a table with two columns |
| Function | "f(x) = y" | A relation that's total and single-valued — every input has exactly one output |
| Bijection | "Perfect pairing" | An injective + surjective function; witnesses equality of cardinality |
| Composition | "f ∘ g" | (f ∘ g)(x) = f(g(x)); the relational join operation |

## Further Reading

- [Halmos — Naive Set Theory](https://link.springer.com/book/10.1007/978-1-4757-1645-0) — under 100 pages and worth every one.
- [Bartosz Milewski — Category Theory for Programmers](https://github.com/hmemcpy/milewski-ctfp-pdf) — sets-and-functions are the entry point into a much bigger story.
- *Relational Database Design* — sections covering relational algebra are the formal complement to this lesson.
