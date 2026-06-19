# Generating Functions

> Package an entire sequence as a power series. Algebra on the series translates to identities on the sequence — a code generator for combinatorial proofs.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 01, Lessons 08–09
**Time:** ~75 minutes

## Learning Objectives

- Write the ordinary generating function (OGF) of a sequence and read off coefficients to recover terms.
- Apply the four basic OGF operations — shift, derivative, integral, multiplication — and interpret each combinatorially.
- Use generating functions to solve linear recurrences in closed form (e.g., Fibonacci's golden-ratio formula).
- Recognize the catalog of well-known OGFs (geometric series, exponential, Catalan, partition function) and apply them when a recurrence has the right shape.

## The Problem

A sequence is just a list `(a₀, a₁, a₂, …)`. Many CS questions are "find aₙ in closed form" or "what does Σ aₙ converge to?" Doing this by hand for non-trivial recurrences (Fibonacci, Catalan, partitions) becomes painful fast.

Generating functions turn the problem into algebra. You pack the sequence into a formal power series:

```
A(x) = a₀ + a₁ x + a₂ x² + a₃ x³ + ...
```

Operations on A(x) — multiplication, differentiation, partial fractions — correspond to operations on the underlying sequence (convolution, shift, summation). With practice, you can derive closed forms for recurrences in 3–4 lines of algebra that would take pages of direct manipulation.

This lesson is the practical generating-function toolkit. In Phase 04 you'll use it to analyze algorithms; in Phase 17 the same machinery generalizes to probability generating functions for randomized algorithms.

## The Concept

### Ordinary generating function (OGF)

Given a sequence `(a₀, a₁, a₂, …)`:

```
A(x) = Σₙ₌₀ ∞  aₙ xⁿ
```

This is a **formal power series** — we don't ask whether it converges, only how its coefficients behave under algebraic operations. The coefficient of xⁿ is denoted `[xⁿ] A(x) = aₙ`.

### The standard catalog

| Sequence | OGF |
|----------|-----|
| `1, 1, 1, 1, …` | `1/(1 - x)` (geometric series) |
| `1, c, c², c³, …` | `1/(1 - cx)` |
| `0, 1, 2, 3, …` | `x/(1 - x)²` |
| `1, 2, 3, 4, …` | `1/(1 - x)²` |
| `C(n, k)` as n varies (k fixed) | `xᵏ / (1 - x)ᵏ⁺¹` |
| `C(n, k)` as k varies (n fixed) | `(1 + x)ⁿ` (the binomial theorem) |
| Catalan: `1, 1, 2, 5, 14, …` | `(1 - √(1 - 4x)) / (2x)` |
| Fibonacci: `0, 1, 1, 2, 3, 5, 8, …` | `x / (1 - x - x²)` |
| Partition function p(n) | `∏ₖ₌₁ ∞ 1/(1 - xᵏ)` |

Knowing this catalog turns many recurrences into pattern-matching.

### Four operations

Given A(x) = Σ aₙ xⁿ and B(x) = Σ bₙ xⁿ:

1. **Linearity**: `A(x) + B(x)` is the OGF of `(aₙ + bₙ)`.
2. **Shift**: `xᵏ · A(x)` is the OGF of the shifted sequence `(0, 0, …, 0, a₀, a₁, …)` — shift right by k.
3. **Multiplication = convolution**: `A(x) · B(x)` is the OGF of `cₙ = Σₖ₌₀ⁿ aₖ bₙ₋ₖ` (the *convolution*).
4. **Differentiation**: `A'(x) = Σ n aₙ x^(n-1)`; useful for "weighted by n" sequences.

### Worked example: Fibonacci

Fibonacci: `F₀ = 0, F₁ = 1, Fₙ = Fₙ₋₁ + Fₙ₋₂` for n ≥ 2.

Let `F(x) = Σ Fₙ xⁿ`. Then:

```
F(x) = F₀ + F₁ x + Σₙ≥₂ Fₙ xⁿ
     = 0 + x + Σₙ≥₂ (Fₙ₋₁ + Fₙ₋₂) xⁿ
     = x + x F(x) + x² F(x)
```

Solve:

```
F(x) (1 - x - x²) = x
F(x) = x / (1 - x - x²)
```

Partial fractions on `1 - x - x²` (roots `1/φ` and `1/ψ` where `φ = (1+√5)/2`, `ψ = (1-√5)/2`):

```
F(x) = (1/√5) · ( 1/(1 - φx) - 1/(1 - ψx) )
Fₙ   = (φⁿ - ψⁿ) / √5      (Binet's formula)
```

A four-line algebraic derivation of the closed form for Fibonacci. This is the entire point of generating functions.

### Worked example: Catalan numbers

Let `C(x) = Σₙ Cₙ xⁿ`. From the Catalan recurrence `Cₙ₊₁ = Σₖ₌₀ⁿ Cₖ Cₙ₋ₖ`:

```
C(x) = 1 + x · C(x)²
```

(The `1` is C₀; the rest is the convolution shifted by one for the "split at root" idea.) Solve the quadratic in C(x):

```
C(x) = (1 - √(1 - 4x)) / (2x)
```

Coefficient extraction (binomial series for `√(1 - 4x)`) gives `Cₙ = (1/(n+1)) · C(2n, n)`. Same result as Lesson 09's closed form.

### Partition numbers

The partition function `p(n)` counts the number of ways to write n as a sum of positive integers (order ignored). Its OGF is:

```
P(x) = Π_{k=1}^∞ 1/(1 - xᵏ)
```

Each factor `1/(1-xᵏ) = 1 + xᵏ + x^(2k) + …` represents "how many times do you use the part k?" Multiplying together packages every possible combination. There's no closed form for `p(n)`, but the generating function gives you efficient computation and Hardy-Ramanujan asymptotics.

## Build It

We'll implement OGFs as truncated polynomials (Python lists of coefficients) and verify each catalog entry.

### Step 1: Coefficient arithmetic

The lesson's `code/main.py` provides `add`, `mul` (truncated to N terms), and `inv` (power-series reciprocal via Newton iteration).

### Step 2: Geometric series

`1/(1-x)` truncated to N terms is `[1, 1, 1, ..., 1]`. Verify by multiplying by `[1, -1]` (= 1 - x) and seeing all higher coefficients vanish.

### Step 3: Fibonacci via OGF

Compute `x / (1 - x - x²)` by power-series division up to N terms. The coefficients are exactly the Fibonacci numbers.

### Step 4: Catalan via OGF

Solve `C(x) = 1 + x C(x)²` by power-series iteration: start with C(x) = [1], plug into the RHS, repeat until stable. The coefficients are the Catalan numbers — same as Lesson 09's closed form.

### Step 5: Stars-and-bars via OGF

`1/(1-x) = 1 + x + x² + x³ + …`. Multiplying k copies gives `(1/(1-x))ᵏ`, whose nₜₕ coefficient counts ordered tuples (x₁, …, xₖ) ≥ 0 with Σ = n — exactly `C(n + k - 1, k - 1)`.

## Use It

- **Algorithm analysis (Phase 04)**: solve master-theorem-shaped recurrences with OGFs when shapes don't fit the master theorem.
- **Probability**: probability generating functions (`E[z^X] = Σ P(X = n) zⁿ`) — same algebra; you read off probabilities, expectations, variances directly.
- **String algorithms (Phase 04)**: pattern-matching counts via convolutions; FFT (Phase 04 L21) computes them in O(n log n).
- **Compiler theory (Phase 05)**: unambiguous context-free grammars have OGFs derived directly from production rules; coefficient growth tells you about ambiguity.
- **Cryptography / coding theory**: weight enumerators of error-correcting codes are generating functions in disguise.

## Read the Source

- *generatingfunctionology* by Wilf — the canonical free PDF.
- *Analytic Combinatorics* by Flajolet & Sedgewick — comprehensive; the "symbolic method" makes the algebra mechanical.
- *Concrete Mathematics* (Graham/Knuth/Patashnik), Ch. 7 — clear development with many worked examples.

## Ship It

This lesson ships **`outputs/series.py`** — a small library: truncated power-series add/sub/mul, power-series reciprocal (Newton iteration), and constructors for the standard catalog (`geometric`, `fibonacci_ogf`, `catalan_ogf`).

## Exercises

1. **Easy.** Compute the first 10 coefficients of `1/(1 - x)²` using the library. Verify they equal `1, 2, 3, 4, …`.
2. **Medium.** Use the OGF approach to solve the recurrence `aₙ = 2aₙ₋₁ + 3` with `a₀ = 1`. Derive a closed form.
3. **Hard.** Compute `p(100)` via the partition generating function (no closed form needed). Verify against the OEIS A000041 entry (`p(100) = 190569292`).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Generating function | "Power series of a sequence" | A(x) = Σ aₙ xⁿ; algebra on A(x) corresponds to identities on (aₙ) |
| Formal power series | "Polynomial with infinite terms" | A series treated as a syntactic object; convergence is irrelevant |
| Convolution | "Multiply two sequences" | (a * b)ₙ = Σₖ aₖ bₙ₋ₖ; the coefficient-wise product of OGFs |
| OGF / EGF | "Two flavors" | Ordinary (Σ aₙ xⁿ) vs Exponential (Σ aₙ xⁿ / n!) — the EGF makes labeled-structure counting cleaner |
| Coefficient extraction | "[xⁿ] f(x)" | The coefficient of xⁿ in f(x) — the "decoder" from series back to sequence |

## Further Reading

- [Flajolet & Sedgewick's free PDF of *Analytic Combinatorics*](https://ac.cs.princeton.edu/home/) — chapters 1–3 are the foundation.
- *A = B* by Petkovšek, Wilf, Zeilberger — algorithms for proving combinatorial identities.
- [The OEIS](https://oeis.org/) — Online Encyclopedia of Integer Sequences. Search a generating function's coefficients; the OEIS will often name the sequence.
