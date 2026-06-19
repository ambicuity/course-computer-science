# Pigeonhole, Inclusion-Exclusion, Catalan

> Three counting tools that turn impossible-looking problems into one-liners.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 01, Lessons 04, 08
**Time:** ~60 minutes

## Learning Objectives

- Apply the pigeonhole principle (simple and generalized) to prove existence and find collisions in hashing, DB sharding, and graph problems.
- Use inclusion-exclusion to count elements satisfying *at least one* of a set of properties; recognize the formula as the algebra of overlapping events.
- Compute Catalan numbers `Cₙ = C(2n, n) / (n+1)` and recognize the objects they count: balanced parentheses, binary trees, monotonic lattice paths, triangulations.
- Apply bijective proofs: show two seemingly different counts are equal by exhibiting a bijection between them.

## The Problem

Some counts look daunting until you find the right framing:

- "Given 13 people, must at least two share a birth month?" Yes — pigeonhole.
- "How many integers in [1, 100] are divisible by 2, 3, or 5?" Naïvely 50+33+20=103, but you double-count. Inclusion-exclusion fixes it.
- "How many distinct binary search trees with n nodes?" Looks recursive — it's the Catalan number Cₙ.

Each has a one-line answer if you know the tool. This lesson collects three of the most reused.

## The Concept

### Pigeonhole principle

> If you place `n + 1` pigeons into n boxes, some box must contain at least two pigeons.

Generalization:

> If you place `n` pigeons into `k` boxes, some box must contain at least `⌈n/k⌉` pigeons.

Examples:
- Among 13 people, two share a birth month.
- Hashing n items into m buckets forces a collision when n > m.
- In any n+1 integers chosen from {1, …, 2n}, two are coprime (classical olympiad problem).
- The Erdős–Ko–Rado theorem in extremal combinatorics is a pigeonhole-style refinement.

Pigeonhole gives you *existence* without telling you which pigeon — perfect for non-constructive proofs.

### Inclusion-exclusion

For two overlapping properties P and Q on a set S:

```
|P ∪ Q| = |P| + |Q| - |P ∩ Q|
```

For three:

```
|P ∪ Q ∪ R| = |P| + |Q| + |R| - |P∩Q| - |P∩R| - |Q∩R| + |P∩Q∩R|
```

In general (n properties):

```
|⋃ Aᵢ| = Σ|Aᵢ| - Σ|Aᵢ ∩ Aⱼ| + Σ|Aᵢ ∩ Aⱼ ∩ Aₖ| - ...
```

Alternating signs.

Worked example: integers in [1, 100] divisible by 2, 3, or 5.

```
|div 2|       = 50
|div 3|       = 33
|div 5|       = 20
|div 6|       = 16    (intersection of div 2 and div 3)
|div 10|      = 10    (intersection of div 2 and div 5)
|div 15|      = 6     (intersection of div 3 and div 5)
|div 30|      = 3     (intersection of all three)

|div 2 or 3 or 5| = 50 + 33 + 20 - 16 - 10 - 6 + 3 = 74
```

A handy special case: **derangements** (permutations with no fixed points): `Dₙ = n! · Σ (-1)ᵏ / k!`. Comes from inclusion-exclusion on "element i is fixed."

### Catalan numbers

```
Cₙ = (1/(n+1)) · C(2n, n)
   = C(2n, n) - C(2n, n+1)         (reflection-principle form)
```

Sequence: 1, 1, 2, 5, 14, 42, 132, 429, …

Cₙ counts a *startling* variety of objects:

| Object | Why Cₙ |
|--------|--------|
| Balanced strings of n pairs of parens | recursive split + bijection |
| Binary trees with n internal nodes | recursive split at root |
| Triangulations of a convex (n+2)-gon | recursive split on an edge |
| Lattice paths from (0,0) to (n,n) that don't cross above the diagonal | reflection principle |
| Number of ways to multiply n+1 matrices (parenthesizations) | binary-tree bijection |

That all these count the same number is the classic **bijective proof**: each pair of objects is in bijection via a constructible map.

Recurrence: `Cₙ = Σₖ₌₀ⁿ⁻¹ Cₖ · Cₙ₋₁₋ₖ`. (Split a parenthesis sequence at the matching `(`.) Solves in O(n²); the closed-form `(1/(n+1)) C(2n, n)` is asymptotically much cheaper.

### Bijective proofs — the technique

Two formulas count the same thing → exhibit a bijection between the two sets they count. Examples:

- `C(n, k) = C(n, n-k)`: bijection is "take the complement of the subset."
- Catalan = balanced parens = binary trees: bijection is "the recursive decomposition."
- `Σ C(n, k) = 2ⁿ`: bijection is "each subset to its indicator function."

When two formulas look "equal by coincidence," look for a bijection. Once you find it, the equality is structural.

## Build It

Open `code/main.py`.

### Step 1: Generalized pigeonhole

```python
def pigeonhole_witness(items, n_boxes):
    from collections import defaultdict
    box = defaultdict(list)
    for it in items:
        box[hash(it) % n_boxes].append(it)
    for b, vals in box.items():
        if len(vals) >= 2:
            return vals[:2]
    return None
```

### Step 2: Inclusion-exclusion on a finite ground set

```python
def inclusion_exclusion(sets):
    """|⋃ Aᵢ| via the alternating sum over all non-empty subset intersections."""
    from itertools import combinations as it_combs
    n = len(sets)
    total = 0
    for size in range(1, n + 1):
        sign = 1 if size % 2 == 1 else -1
        for combo in it_combs(range(n), size):
            inter = set(sets[combo[0]])
            for i in combo[1:]:
                inter &= sets[i]
            total += sign * len(inter)
    return total
```

Verify on integers in [1, 100] divisible by 2, 3, 5 → 74.

### Step 3: Catalan numbers two ways

```python
from math import comb
def catalan_closed(n): return comb(2*n, n) // (n + 1)

cache = {0: 1}
def catalan_rec(n):
    if n in cache: return cache[n]
    cache[n] = sum(catalan_rec(k) * catalan_rec(n - 1 - k) for k in range(n))
    return cache[n]
```

Verify they agree for n ≤ 10.

### Step 4: Enumerate balanced parens; count = Cₙ

```python
def balanced_parens(n):
    if n == 0: yield ""; return
    for k in range(n):
        for left in balanced_parens(k):
            for right in balanced_parens(n - 1 - k):
                yield "(" + left + ")" + right
```

`len(list(balanced_parens(n)))` matches Cₙ.

### Step 5: Derangements via inclusion-exclusion

```python
def derangements(n):
    from math import factorial
    return round(factorial(n) * sum((-1)**k / factorial(k) for k in range(n+1)))

assert derangements(4) == 9     # !4
assert derangements(5) == 44    # !5
```

## Use It

- **Pigeonhole in CS:** hash collisions, sharding bound proofs, birthday paradox, "any 5 points in a unit square place two within distance √2/2."
- **Inclusion-exclusion:** counting graphs with no isolated vertex; surjective functions; probability of "at least one of several events."
- **Catalan numbers in CS:** counting parse trees of a grammar, valid bracket sequences in code, matrix-chain associations (Phase 04 DP), lattice paths.
- **Bijective proofs:** combinatorial proof of identities that would otherwise need induction or generating functions.

## Read the Source

- *Concrete Mathematics* (Graham, Knuth, Patashnik), §7.5 and §5.4.
- *Catalan Numbers* by Richard Stanley — full book on objects counted by Cₙ; over 200 of them.
- [Wikipedia's pigeonhole-principle examples](https://en.wikipedia.org/wiki/Pigeonhole_principle) — surprisingly diverse list of one-line proofs.

## Ship It

This lesson ships **`outputs/catalan.py`** with `catalan_closed(n)`, `catalan_recurrence(n)` (memoized), `balanced_parens(n)` (lazy iterator), and **`outputs/inclusion_exclusion.py`** with a general `inclusion_exclusion(sets)`.

## Exercises

1. **Easy.** How many integers in [1, 1000] are not divisible by 2, 3, 5, or 7? Use inclusion-exclusion. (Hint: subtract the "at least one of" count from 1000.)
2. **Medium.** Prove with pigeonhole: in any group of 6 people, either 3 mutually know each other or 3 are mutual strangers. (Ramsey R(3, 3) = 6; pigeonhole on each person's edges into "knows" or "doesn't know.")
3. **Hard.** Find the bijection between balanced parens of n pairs and binary trees with n internal nodes. Implement both directions; verify on n = 5 (42 of each).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Pigeonhole principle | "Collision must happen" | Placing n items into k < n boxes forces some box to hold ≥ 2 items |
| Inclusion-exclusion | "Add overlaps back" | Formula for \|⋃ Aᵢ\| as alternating sum over non-empty subset intersections |
| Catalan number | "The bracket-count sequence" | Cₙ = C(2n, n)/(n+1); counts dozens of recursively-defined CS objects |
| Bijective proof | "Show they pair up" | Prove two counts are equal by exhibiting a bijection between the sets they count |
| Derangement | "Permutation with no fixed points" | A permutation where no element is in its original position; counted by !n |

## Further Reading

- *Bijective Combinatorics* by Loehr — bijective-proof style, deep examples.
- *A Combinatorial Miscellany* by Björner & Stanley — short, beautiful tour.
- [Catalan number bijections (OEIS A000108)](https://oeis.org/A000108) — Stanley's list of 200+ objects, with references.
