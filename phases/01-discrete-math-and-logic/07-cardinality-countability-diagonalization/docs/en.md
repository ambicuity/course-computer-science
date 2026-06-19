# Cardinality, Countability, Diagonalization

> There are more real numbers than integers. There are more functions than programs. Once you can prove that, you understand why the halting problem is unsolvable.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 01, Lessons 04
**Time:** ~60 minutes

## Learning Objectives

- Define cardinality precisely via bijections; explain why "same size" is a primitive notion for infinite sets.
- Prove ℕ × ℕ is countable (the pairing trick) and ℚ is countable; prove ℝ is uncountable (Cantor's diagonal).
- Apply diagonalization to show: there are uncountably many functions ℕ → {0,1}, but only countably many programs.
- Recognize diagonalization as the single technique behind: uncountability of reals, the halting problem, Russell's paradox, Gödel's incompleteness, and Turing's universal-machine argument.

## The Problem

Cardinality is the math of "how many." For finite sets the answer is an integer; for infinite sets the answer is one of a hierarchy of infinities. Whether two infinities are "the same size" or not turns out to determine:

- Whether you can index a data structure with a list of integers (yes for countable, no for uncountable).
- Whether a problem has a *computer program* solving it (only countably many programs exist).
- Whether you can route around the halting problem (no — diagonalization shows this).
- Whether type theory's "all functions" can be a set (no — Russell's paradox descended from diagonalization).

This lesson teaches the one technique behind all of those: **diagonalization**. Once you see it, you'll spot it in every uncountability proof for the rest of the course.

## The Concept

### Cardinality, defined

Two sets A and B have the **same cardinality** (`|A| = |B|`) iff there exists a bijection f: A → B. We say A is **at most as large** as B (`|A| ≤ |B|`) iff there's an injection A → B.

For finite sets this matches the integer count. For infinite sets, this is the *definition* — and it produces surprising consequences:

- |ℕ| = |2ℕ| (evens). Bijection: `n ↦ 2n`. There are "as many" evens as naturals.
- |ℕ| = |ℤ|. Bijection: `0 ↦ 0, 1 ↦ -1, 2 ↦ 1, 3 ↦ -2, …`.
- |ℕ| = |ℕ × ℕ|. See "pairing" below.
- |ℕ| = |ℚ|. Use the pairing on (numerator, denominator).

A set is **countable** iff it is finite or has the same cardinality as ℕ. Equivalently: countable iff there is a surjection ℕ → A (you can enumerate its elements as a₀, a₁, a₂, …, possibly with repeats).

### Pairing: ℕ × ℕ is countable

The **Cantor pairing function** zigzags through the lattice:

```
       0   1   2   3
   0 ─ 0 ─ 1   3   6
        ╲   ╲ ╱
   1   2   4   7
        ╲ ╱
   2   5   8
        ╲
   3   9 ...
```

A formula: `pair(x, y) = (x + y)(x + y + 1)/2 + y`. It is a bijection ℕ × ℕ ↔ ℕ. This is enough to show every finite-tuple product `ℕ^k` is countable.

### ℝ is uncountable: Cantor's diagonal argument

> **Theorem.** [0, 1) is uncountable.
> **Proof.** Suppose for contradiction that there's an enumeration r₀, r₁, r₂, … of [0, 1). Each rᵢ has a binary expansion `rᵢ = 0.bᵢ₁ bᵢ₂ bᵢ₃ …` (fix the form not ending in all 1's for rationals with two expansions). Build a new number `d = 0.d₁ d₂ d₃ …` where `dₖ = 1 - bₖₖ` — flip the kₜₕ digit of the kₜₕ row.
>
> By construction, d differs from rᵢ at the iₜₕ digit, so `d ≠ rᵢ` for every i. But d ∈ [0, 1), so the enumeration was incomplete. Contradiction. ∎

In code:

```
       r₀ = 0.[b₀₀] b₀₁ b₀₂ b₀₃ ...
       r₁ = 0.  b₁₀ [b₁₁] b₁₂ b₁₃ ...
       r₂ = 0.  b₂₀ b₂₁ [b₂₂] b₂₃ ...
       r₃ = 0.  b₃₀ b₃₁ b₃₂ [b₃₃] ...
   diagonal d  = 0. ¬b₀₀ ¬b₁₁ ¬b₂₂ ¬b₃₃ ...
```

`d` differs from every row on its diagonal digit → `d ∉ {r₀, r₁, …}`.

This is the **diagonal trick**. Every uncountability proof you'll meet is structurally this.

### Power-sets: |𝒫(A)| > |A| always

> **Cantor's theorem.** For any set A, |𝒫(A)| > |A|.
> **Proof.** Suppose f: A → 𝒫(A) is a surjection. Define D = { a ∈ A : a ∉ f(a) }. Since f is surjective, D = f(a₀) for some a₀. Then:
> - If a₀ ∈ f(a₀): a₀ satisfies the defining condition for D backwards, so a₀ ∉ f(a₀). Contradiction.
> - If a₀ ∉ f(a₀): a₀ satisfies the condition, so a₀ ∈ D = f(a₀). Contradiction.
>
> So no surjection exists; |𝒫(A)| strictly exceeds |A|. ∎

This is diagonalization again — the "set of elements that disagree with their image" is the same construction as Cantor's diagonal number.

### Why the halting problem is unsolvable (preview)

The same trick:

- **Programs** are finite strings → there are countably many.
- **Functions ℕ → ℕ** are uncountable (by Cantor: 𝒫(ℕ) is uncountable, and functions ℕ → {0,1} are essentially 𝒫(ℕ)).
- So *some* function ℕ → ℕ has no program.

To pin down a *specific* such function (the halting decider), build a diagonal one: define `H(p) = 1 if program p halts on input p, 0 otherwise`. Suppose H itself were computable by a program. Construct D that runs `H(D)` and does the opposite. Run `D(D)`: contradiction either way.

The full halting-problem proof is in Phase 05 (Theory of Computation). This lesson plants the seed.

## Build It

Open `code/main.py`.

### Step 1: Cantor pairing in code

```python
def cantor_pair(x, y):
    return (x + y) * (x + y + 1) // 2 + y
```

The inverse can be computed by solving `w(w+1)/2 ≤ z < (w+1)(w+2)/2` for w (the "diagonal index"), then `y = z - w(w+1)/2`, `x = w - y`.

Verify `unpair(pair(x, y)) == (x, y)` for many (x, y).

### Step 2: Enumerate ℕ × ℕ

Print the first 20 unpaired values — perfect zigzag of the (x, y) lattice.

### Step 3: Enumerate ℚ⁺

Every positive rational `p/q` (in lowest terms) is a pair (p, q) with gcd(p, q) = 1. Walk Cantor pairs, skip non-coprime ones, output. Demonstrates ℚ is countable.

### Step 4: Computational diagonalization

For a *finite* simulated enumeration of "real numbers" (binary lists), compute the diagonal `d[i] = 1 - rows[i][i]`. Verify d differs from each row in the corresponding position.

### Step 5: 𝒫(ℕ) is uncountable — finite witness

Given any list of subsets `S₀, S₁, …, Sₙ₋₁` of ℕ, the diagonal subset `D = { i : i ∉ Sᵢ }` (for i in range n) is provably *not* in that list — exactly the construction in Cantor's theorem.

## Use It

- **Halting problem** (Phase 05): undecidable by exactly this diagonal trick.
- **Rice's theorem** (Phase 05): every non-trivial semantic property of programs is undecidable. Diagonalization corollary.
- **Type theory / Russell's paradox**: "the set of all sets that don't contain themselves" — same construction, applied to sets-of-sets.
- **Lower bounds via adversary arguments** (Phase 04): "any algorithm must take at least n log n comparisons" proofs use a related "build a bad input adversarially" idea.
- **Cryptography**: the existence of a one-way function requires countably-many efficient algorithms vs uncountably-many functions — used to argue what's plausibly hard.

## Read the Source

- *Naive Set Theory* (Halmos), §22 — cardinal arithmetic in 5 pages.
- *Computability and Logic* (Boolos, Burgess, Jeffrey) — diagonalization in computability + Gödel + Tarski.
- [Cantor's original 1891 paper (translated)](https://www.maths.tcd.ie/pub/HistMath/People/Cantor/Diagonal/Diagonal.html) — short, clear, beautiful.

## Ship It

This lesson ships **`outputs/pairing.py`** — bijective integer ↔ pair codecs (Cantor + Szudzik), used for serializing 2-tuples to bytes, sharding by composite key, and building dense indexes from sparse coordinates.

## Exercises

1. **Easy.** Verify `cantor_pair(x, y)` is a bijection on ℕ × ℕ ↔ ℕ for `x, y ∈ {0..9}`. Print all 100 pairs and their codes; assert no collisions.
2. **Medium.** Enumerate the first 50 positive rationals in lowest terms via Cantor pairs. Compare with the Stern-Brocot tree's first 50.
3. **Hard.** Implement a *computational* diagonal proof: given a Python list of `n` infinite-streams (lazy iterables of bits), produce a stream that differs from each at some position. Use this to argue, by analogy, that `Streams(bool) ≠ ℕ`-indexed enumerations.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Cardinality | "Size" | Equivalence class under "there exists a bijection" |
| Countable | "Listable" | Finite or in bijection with ℕ — equivalently, the image of some surjection ℕ → A |
| Diagonalization | "Cantor's trick" | A construction that produces an element disagreeing with every member of a candidate enumeration |
| Pairing function | "Bijection ℕ² ↔ ℕ" | A concrete map giving every pair a unique natural number; proves products of countable sets are countable |
| Cantor's theorem | "𝒫(A) is bigger than A" | For every set A, no surjection A → 𝒫(A) exists |

## Further Reading

- *Gödel, Escher, Bach* by Douglas Hofstadter — full-length narrative on diagonalization and self-reference.
- [The Stanford Encyclopedia: Cantor's Theorem](https://plato.stanford.edu/entries/cantor/) — historical + philosophical context.
- *The Annotated Turing* by Charles Petzold — the original "On Computable Numbers" walked through line by line.
