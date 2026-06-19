# Combinatorics — Counting, Permutations, Combinations

> Knowing *how many things there are* is the difference between a feasible algorithm and a hopeless one. Combinatorics is how you count without listing.

**Type:** Build
**Languages:** Python, Rust
**Prerequisites:** Phase 01, Lessons 04
**Time:** ~75 minutes

## Learning Objectives

- Apply the four basic counting principles: sum rule, product rule, permutations, combinations.
- Compute permutations `P(n, k) = n!/(n-k)!` and combinations `C(n, k) = n!/(k!(n-k)!)`; recognize when each applies.
- Solve "stars and bars" problems: number of non-negative integer solutions to x₁ + x₂ + … + xₖ = n.
- Estimate `C(n, k)` quickly with Stirling's approximation; read Pascal's triangle and its identities.

## The Problem

How many keys in a 256-bit space? (2^256.) How many distinct shuffles of a 52-card deck? (52! ≈ 8 × 10^67.) How many ways can a hash table of size 100 hold 50 distinct items? (C(100, 50) ≈ 10^29.) Every one of those numbers is a combinatorial count.

In algorithm analysis you reach for these constantly:
- Search-tree sizes: how many nodes in a backtracking tree of depth d, branching factor b? (b^d.)
- Dynamic-programming state spaces: how many subsets of n items? (2^n.)
- Probabilistic guarantees: probability that two distinct items collide in a hash table of size m? (~ n^2 / 2m by birthday paradox — a counting argument.)
- Lower bounds: any sorting algorithm needs ≥ log₂(n!) comparisons (because n! permutations × 1 yes/no answer each).

This lesson is the counting toolbox you'll reach for in every algorithm analysis.

## The Concept

### The four basic rules

| Rule | When | Formula |
|------|------|---------|
| **Sum rule** | Counting *disjoint* alternatives | `\|A ∪ B\| = \|A\| + \|B\|` (when disjoint) |
| **Product rule** | Sequence of independent choices | If you make k choices in turn with nᵢ options each: `n₁ · n₂ · … · nₖ` |
| **Permutations** | Ordered selections from a pool, no repeats | `P(n, k) = n!/(n-k)!` |
| **Combinations** | Unordered selections from a pool, no repeats | `C(n, k) = n! / (k!(n-k)!)` |

### When does order matter?

If different orderings count as different outcomes → permutations.
If different orderings count as the same outcome → combinations.

| Question | Order matters? | Formula |
|----------|----------------|---------|
| Pick a president, VP, treasurer from 10 people | Yes (different roles) | P(10, 3) = 720 |
| Pick a 3-person committee from 10 | No | C(10, 3) = 120 |
| 5-card hands from a 52-card deck | No | C(52, 5) = 2,598,960 |
| Lottery: 6 numbers from 49 | No | C(49, 6) = 13,983,816 |

### Pascal's triangle

```
                1
              1   1
            1   2   1
          1   3   3   1
        1   4   6   4   1
      1   5  10  10   5   1
    1   6  15  20  15   6   1
```

Row n column k is `C(n, k)`. Pascal's rule:

`C(n, k) = C(n-1, k-1) + C(n-1, k)`

Interpretation: to choose k items from n, either include the nₜₕ item (C(n-1, k-1) ways) or exclude it (C(n-1, k) ways). This is the cleanest example of a *combinatorial proof*: two formulas equal because they count the same thing two ways.

Symmetry: `C(n, k) = C(n, n-k)`. Sum: `Σₖ C(n, k) = 2ⁿ` (the number of subsets of an n-element set).

### Stars and bars

> How many non-negative integer solutions are there to `x₁ + x₂ + … + xₖ = n`?

Imagine n stars (`*`) and k-1 bars (`|`). Each arrangement of the n + k - 1 symbols corresponds to one solution: the bars divide the stars into k groups, and xᵢ = number of stars in group i. So:

`C(n + k - 1, k - 1)` solutions.

This solves a huge class of "distribute n items into k buckets" problems. Variants:
- "At least one in each bucket" (`xᵢ ≥ 1`): substitute `yᵢ = xᵢ - 1`, becomes Σ yᵢ = n - k → `C(n - 1, k - 1)`.

### Multinomial coefficient

Number of distinct arrangements of a multiset (e.g., the letters in "MISSISSIPPI"):

```
n! / (n₁! · n₂! · … · nₖ!)
```

where nᵢ is the number of repeats of each distinct item.

### Stirling's approximation

For large n:

```
n! ≈ √(2πn) · (n/e)ⁿ
```

Useful when you need to estimate `C(n, k)`:

```
C(n, n/2) ≈ 2ⁿ / √(πn/2)
```

`C(n, n/2)` grows like 2ⁿ divided by a logarithmic factor — the central column of Pascal's triangle dominates the sum.

## Build It

### Step 1: A combinatorial library

The lesson's `code/main.py` provides `factorial`, `permutations`, `combinations` (with the `k = min(k, n-k)` symmetry optimization), `multinomial`, `stars_and_bars`.

Verify `combinations(52, 5) == 2_598_960`.

### Step 2: Pascal's triangle

```python
def pascal(rows):
    out = [[1]]
    for _ in range(rows - 1):
        prev = out[-1]
        new = [1] + [prev[i] + prev[i+1] for i in range(len(prev)-1)] + [1]
        out.append(new)
    return out
```

Verify `pascal(10)[5] == [1, 5, 10, 10, 5, 1]`.

### Step 3: Multinomial coefficient

"MISSISSIPPI" has letters M=1, I=4, S=4, P=2 (total 11). Distinct arrangements: `multinomial(1, 4, 4, 2) = 34_650`.

### Step 4: Stars and bars

Distribute 10 candies among 4 kids: `stars_and_bars(10, 4) = C(13, 3) = 286`.

### Step 5: Verification by brute force

```python
from itertools import combinations as it_combs
n, k = 8, 3
assert len(list(it_combs(range(n), k))) == combinations(n, k)
```

### Step 6: Pascal in Rust

`code/main.rs` re-implements Pascal's triangle and prints the first 15 rows.

## Use It

- **Probability**: most basic probability problems start as combinatorial counts (events, sample space).
- **Hashing**: birthday paradox is a counting argument on collisions in m bins.
- **Cryptography**: 2^256 keyspace, brute-force lower bounds for n-bit hashes.
- **Algorithm analysis**: enumeration of subsets/permutations in backtracking, DP, sorting.
- **Combinatorial optimization**: counting feasible solutions; cardinality of NP problem instance spaces.

## Read the Source

- *Concrete Mathematics* by Graham, Knuth, Patashnik — Chapters 5–7 are the textbook.
- *generatingfunctionology* by Herbert Wilf — free PDF; brilliant on the bigger machinery.
- [Project Euler problems](https://projecteuler.net/) — many are combinatorial; solving them sharpens this muscle.

## Ship It

This lesson ships **`outputs/combo.py`** — a tested combinatorial library with `factorial`, `nCr`, `nPr`, `multinomial`, `stars_and_bars`, `stirling_approx_factorial`. Reused in Lesson 09 (pigeonhole/IE/Catalan), Lesson 10 (generating functions), and throughout Phase 04 (algorithms).

## Exercises

1. **Easy.** How many distinct 5-letter passwords of distinct lowercase letters? (P(26, 5).) How many if letters can repeat? (26⁵.) Verify both with the library.
2. **Medium.** Use stars-and-bars to count: how many ways to distribute 12 indistinguishable candies among 4 children if each child gets at least 1? (Hint: substitute y = x - 1.)
3. **Hard.** Implement a function that enumerates all C(n, k) subsets of size k from {0..n-1} in *lexicographic order*, in O(C(n,k) · k) time using "Gosper's hack" or the standard combination-iterator pattern (without recursion).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Permutation | "Ordered arrangement" | P(n, k) = n!/(n-k)! — ordered selection of k from n without repeats |
| Combination | "Unordered choice" | C(n, k) = n!/(k!(n-k)!) — k-subset of an n-set |
| Pascal's rule | "C(n,k) recursion" | C(n, k) = C(n-1, k-1) + C(n-1, k); the basis of Pascal's triangle |
| Stars and bars | "Distribute into buckets" | C(n + k - 1, k - 1) ways to write n = x₁ + … + xₖ with xᵢ ≥ 0 |
| Stirling's approximation | "Factorial ≈ formula" | n! ≈ √(2πn)·(n/e)ⁿ; lets you reason about huge factorials asymptotically |

## Further Reading

- *Enumerative Combinatorics* by Stanley — the graduate-level reference.
- [Donald Knuth — TAOCP, Vol 4A](https://www-cs-faculty.stanford.edu/~knuth/taocp.html), §7.2.1 — *Combinatorial Algorithms*; how to enumerate, not just count.
- *A Walk Through Combinatorics* by Bóna — undergrad textbook with clean exposition.
