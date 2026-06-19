# Recurrence Relations & the Master Theorem

> Divide a problem of size n into a pieces of size n/b. The Master theorem tells you the runtime — by pattern matching, in one line.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 01, Lessons 08–10
**Time:** ~75 minutes

## Learning Objectives

- Distinguish *linear recurrences* (homogeneous and non-homogeneous) from *divide-and-conquer recurrences*, and solve each with the right tool.
- Solve linear homogeneous recurrences via the characteristic equation; recover closed forms.
- Apply the Master theorem (CLRS form) to recurrences of shape `T(n) = a T(n/b) + f(n)`; recognize the three cases and the regularity condition.
- Solve recurrences with the recursion-tree method when the Master theorem doesn't apply (e.g., Akra-Bazzi territory).

## The Problem

Almost every algorithm analysis ends at a recurrence:

- Merge sort: `T(n) = 2 T(n/2) + n`
- Binary search: `T(n) = T(n/2) + 1`
- Karatsuba multiplication: `T(n) = 3 T(n/2) + n`
- Strassen's matrix multiply: `T(n) = 7 T(n/2) + n²`
- Quicksort (average): `T(n) = T(n/2) + T(n/2) + n` (in expectation)
- A loop that scans linearly and recurses: `T(n) = T(n-1) + n`
- Recursive Fibonacci: `T(n) = T(n-1) + T(n-2) + 1`

Solving each from scratch — substituting, expanding, summing — is tedious. The Master theorem and the characteristic-equation method are the two cookbooks for the common shapes. Get fluent with them and you read algorithm runtimes off the recurrence in seconds.

## The Concept

### Linear homogeneous recurrences

`aₙ = c₁ aₙ₋₁ + c₂ aₙ₋₂ + … + cₖ aₙ₋ₖ` with constant coefficients.

**Solution recipe** (characteristic equation):
1. Write the **characteristic polynomial**: `xᵏ - c₁ xᵏ⁻¹ - … - cₖ = 0`.
2. Find its roots r₁, …, rₖ (with multiplicities).
3. The general solution is a linear combination:
   - If all roots distinct: `aₙ = α₁ r₁ⁿ + α₂ r₂ⁿ + … + αₖ rₖⁿ`.
   - If a root r has multiplicity m: include `(α + βn + γn² + … + δ n^(m-1)) rⁿ`.
4. Solve for the αᵢ using the initial conditions.

**Worked example: Fibonacci.**
Recurrence: `Fₙ = Fₙ₋₁ + Fₙ₋₂`. Characteristic: `x² - x - 1 = 0`. Roots: `φ = (1+√5)/2, ψ = (1-√5)/2`. General solution: `Fₙ = αφⁿ + βψⁿ`. Initial conditions F₀=0, F₁=1 give α = 1/√5, β = -1/√5. **Binet's formula** falls out.

### Non-homogeneous: just add a particular solution

`aₙ = c₁ aₙ₋₁ + … + cₖ aₙ₋ₖ + g(n)`.

Solve the homogeneous part as above, then guess a particular solution that matches `g(n)`:

| `g(n)` shape | Particular solution to try |
|--------------|----------------------------|
| polynomial of degree d | polynomial of degree d (or d + m if the homogeneous solution already contains it) |
| `cⁿ` (c not a homogeneous root) | `α · cⁿ` |
| `cⁿ` (c IS a homogeneous root) | `α · n · cⁿ` |
| `nᵈ · cⁿ` | similar with extra polynomial factor |

### Divide-and-conquer recurrences: the Master theorem

> `T(n) = a · T(n/b) + f(n)`,  with a ≥ 1, b > 1.

Compare `f(n)` against `n^(log_b a)`:

| Case | Condition | Solution |
|------|-----------|----------|
| **1** | `f(n) = O(n^(log_b a - ε))` for some ε > 0 (leaves dominate) | `T(n) = Θ(n^(log_b a))` |
| **2** | `f(n) = Θ(n^(log_b a) · log^k n)` (balanced) | `T(n) = Θ(n^(log_b a) · log^(k+1) n)` |
| **3** | `f(n) = Ω(n^(log_b a + ε))` AND `a · f(n/b) ≤ c · f(n)` for some c < 1 (root dominates) | `T(n) = Θ(f(n))` |

The intuition: there are `a^k` subproblems at depth k, each of size `n / b^k`. Total work at depth k is `a^k · f(n / b^k)`. The recursion has `log_b n` levels. Sum of work over levels is either dominated by the leaves (case 1), spread evenly (case 2), or dominated by the root (case 3).

**Examples:**

| Recurrence | a | b | log_b a | f(n) | Case | T(n) |
|------------|---|---|---------|------|------|------|
| Merge sort: `2T(n/2) + n` | 2 | 2 | 1 | n | 2 | `Θ(n log n)` |
| Binary search: `T(n/2) + 1` | 1 | 2 | 0 | 1 | 2 | `Θ(log n)` |
| Karatsuba: `3T(n/2) + n` | 3 | 2 | log₂3 ≈ 1.585 | n | 1 | `Θ(n^1.585)` |
| Strassen: `7T(n/2) + n²` | 7 | 2 | log₂7 ≈ 2.807 | n² | 1 | `Θ(n^2.807)` |
| Trivial recursion: `T(n/2) + n` | 1 | 2 | 0 | n | 3 | `Θ(n)` |

### Where the Master theorem fails

- **Non-polynomial gaps** between f(n) and n^(log_b a): e.g., `T(n) = 2T(n/2) + n/log n`. Use **Akra-Bazzi** (generalization).
- **Unbalanced splits**: `T(n) = T(n/3) + T(2n/3) + n`. Akra-Bazzi handles this too (T(n) = Θ(n log n)).
- **Non-power-of-b sizes**: usually OK if you ignore floors/ceilings (the answer is asymptotically the same).
- **f(n) negative or oscillating**: rare in algorithm analysis; the regularity condition in Case 3 breaks.

### The recursion-tree method (always works)

Draw the recursion as a tree. At depth k, there are `a^k` nodes of size `n/b^k`. Total work at depth k is `a^k · f(n/b^k)`. Sum over all levels (depth 0 through log_b n) to get T(n). This is **how** the Master theorem cases are derived; falling back to it when MT doesn't fit is always safe (though sometimes algebraic).

## Build It

Open `code/main.py`.

### Step 1: Solve linear homogeneous recurrences

```python
import numpy as np
def linear_homogeneous(coeffs, initial, n):
    """Compute aₙ for a linear homogeneous recurrence aₙ = c₁aₙ₋₁ + ... + cₖaₙ₋ₖ
    with given initial conditions, by stepping."""
    a = list(initial)
    while len(a) <= n:
        a.append(sum(coeffs[i] * a[-1-i] for i in range(len(coeffs))))
    return a[n]
```

Use it to verify F_30 = 832040 and the Padovan sequence's growth.

### Step 2: Characteristic-equation closed form (for small recurrences)

For Fibonacci specifically:

```python
import math
phi = (1 + math.sqrt(5)) / 2
psi = (1 - math.sqrt(5)) / 2
def binet(n):
    return round((phi**n - psi**n) / math.sqrt(5))
```

Verify against the iterative version for n up to 70 (after that, float precision degrades).

### Step 3: Master theorem classifier

```python
def master_theorem(a, b, f_degree, f_log_factor=0):
    """Classify T(n) = a T(n/b) + n^f_degree · log^f_log_factor n.
    Returns (case, asymptotic_string)."""
    import math
    crit = math.log(a, b)
    if f_degree < crit:
        return 1, f"Θ(n^{crit:.3f})"
    if f_degree == crit:
        return 2, f"Θ(n^{crit:.3f} log^{f_log_factor + 1} n)"
    return 3, f"Θ(n^{f_degree} log^{f_log_factor} n)"
```

Run on the table above; verify all five answers.

### Step 4: Recursion-tree summation

For any (a, b, f), sum `a^k · f(n/b^k)` over k = 0, …, ⌊log_b n⌋. Compare against the Master theorem result for matching examples; observe the discrepancy when MT doesn't apply.

### Step 5: Verify by direct simulation

For T(n) = 2T(n/2) + n, simulate the actual function:

```python
def T(n):
    if n <= 1: return 1
    return 2 * T(n // 2) + n

# Check growth rate
ns = [2**i for i in range(8, 18)]
ratios = [T(n) / (n * math.log2(n)) for n in ns]
# Should converge to a constant near 1
```

## Use It

- **Phase 04 (Algorithms)**: every divide-and-conquer algorithm in this course has an MT-derivable runtime.
- **DP / memoized recursion**: even when not exactly D&C, the recursion-tree method computes the work.
- **Network / distributed systems**: protocols often have logarithmic depth (Chord, B-trees); MT explains the cost of a query.
- **Cryptography**: modular exponentiation via repeated squaring is T(n) = T(n/2) + O(1) → O(log n). Trivial via MT.

## Read the Source

- *CLRS*, Chapters 2 (recursion) and 4 (Master theorem with full proof).
- *Akra-Bazzi (1998)* — the generalization that handles unbalanced splits and log factors.
- *Concrete Mathematics* §1.2 (the tower of Hanoi recurrence) and Ch. 6 (linear recurrences).

## Ship It

This lesson ships **`outputs/recurrence.py`** — a small library: `solve_linear(coeffs, initial, n)`, `master(a, b, f_degree, f_log_factor)`, `recursion_tree_sum(a, b, f, n)`. Reused throughout Phase 04.

## Exercises

1. **Easy.** Classify each via Master theorem: (a) `T(n) = 4T(n/2) + n`, (b) `T(n) = 2T(n/2) + n²`, (c) `T(n) = T(n - 1) + log n`.
2. **Medium.** Solve `aₙ = 2aₙ₋₁ - aₙ₋₂` with a₀ = 1, a₁ = 2 via the characteristic equation. Verify your closed form for n up to 10.
3. **Hard.** Use the recursion-tree method (not the Master theorem) to derive T(n) = O(n log n) for `T(n) = T(n/3) + T(2n/3) + n`. Bonus: confirm with Akra-Bazzi.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Recurrence | "T(n) = ... formula" | An equation defining `aₙ` in terms of earlier values, plus initial conditions |
| Characteristic equation | "The polynomial" | For a linear recurrence, the polynomial whose roots determine the closed form |
| Master theorem | "The cookbook" | A 3-case classifier for `T(n) = aT(n/b) + f(n)` recurrences |
| Recursion tree | "Draw the calls" | A tree where each node is a subproblem; summing work over levels gives total runtime |
| Akra-Bazzi | "Generalized MT" | A more general method that handles unbalanced splits and irregular f(n) |

## Further Reading

- *Algorithm Design* by Kleinberg & Tardos, Chapter 5 — divide-and-conquer worked from first principles.
- [Akra-Bazzi original paper](https://link.springer.com/article/10.1023/A:1018299006954) — the generalization, ~10 pages.
- [Sedgewick & Wayne lecture notes](https://algs4.cs.princeton.edu/lectures/) on analysis of algorithms — visualizations of recursion trees.
