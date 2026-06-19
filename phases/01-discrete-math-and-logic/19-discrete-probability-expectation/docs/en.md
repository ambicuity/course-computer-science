# Discrete Probability & Expectation

> Linearity of expectation is the single most useful trick in randomized algorithm analysis. You can prove things about expected behavior without ever computing the distribution.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 01, Lessons 04, 08
**Time:** ~60 minutes

## Learning Objectives

- Define a (discrete) probability space, event, and random variable; compute probabilities of events.
- Compute expectations using the definition and using **linearity of expectation**; apply linearity even when variables are dependent.
- Use Markov's inequality, Chebyshev's inequality, and the union bound to derive probabilistic guarantees.
- Apply the **birthday paradox** and the **coupon collector** analyses — two foundational templates for randomized algorithm analysis.

## The Problem

Randomized algorithms (quicksort with random pivots, treaps, bloom filters, Monte-Carlo methods, hash tables) all need probability analysis. Three recurring questions:

1. "What's the expected runtime?" — linearity of expectation.
2. "What's the probability of catastrophic behavior?" — Markov, Chebyshev, Chernoff.
3. "What's the expected time to see all n distinct items in a random stream?" — coupon collector.

This lesson is the toolbox.

## The Concept

### Probability space

A **discrete probability space** is `(Ω, P)` where Ω is a finite (or countable) sample space and P assigns each outcome `ω ∈ Ω` a probability with `Σ P(ω) = 1`. An **event** A ⊆ Ω has probability `P(A) = Σ_{ω ∈ A} P(ω)`.

Two events A, B:
- **Union**: P(A ∪ B) = P(A) + P(B) - P(A ∩ B). (Inclusion-exclusion from L09.)
- **Independent**: P(A ∩ B) = P(A) · P(B). NOT the same as disjoint.
- **Conditional**: P(A | B) = P(A ∩ B) / P(B).

### Random variables

A **random variable** X: Ω → ℝ assigns a number to each outcome. The **expectation**:

```
E[X] = Σ_ω P(ω) · X(ω) = Σ_x x · P(X = x)
```

### Linearity of expectation

```
E[X + Y] = E[X] + E[Y]
```

This holds **even when X and Y are dependent**. That's why it's the workhorse of probabilistic analysis.

**Example (linearity, dependent variables).** A randomly permuted array of n distinct elements. What's the expected number of "ascents" (positions i where `a[i] < a[i+1]`)?

Let `Xᵢ = 1` if position i is an ascent, 0 otherwise. The Xᵢ are NOT independent (an ascent at i constrains i+1). But:

```
E[Xᵢ] = P(a[i] < a[i+1]) = 1/2     (symmetric)
E[ascents] = Σ E[Xᵢ] = (n - 1)/2
```

No distribution math needed. That's linearity.

### Indicator variables

The above trick is the most-used pattern in probabilistic analysis. For ANY event A:

```
E[1_A] = P(A)
```

So counting events is summing indicator expectations. Reframe "how many X happen?" as "Σ_i Xᵢ" with each Xᵢ an indicator, then sum the probabilities.

### Conditional expectation

```
E[X] = Σ_y E[X | Y = y] · P(Y = y)
```

The **law of total expectation** ("tower property"). Useful when X depends on Y.

### Three useful inequalities

| Inequality | Statement | Use case |
|------------|-----------|----------|
| **Markov** | P(X ≥ a) ≤ E[X]/a for X ≥ 0 | crude upper bound on rare events |
| **Chebyshev** | P(|X − μ| ≥ k σ) ≤ 1/k² | tail bound when you know variance |
| **Chernoff** | P(X ≥ (1 + δ)μ) ≤ exp(−δ² μ / 3) (one form) | exponential tail; sharpest for sums of independent vars |

The **union bound** `P(A₁ ∪ … ∪ Aₙ) ≤ Σ P(Aᵢ)` is technically not an inequality on expectations but is foundational: bound "at least one bad thing happens" by the sum of individual probabilities.

### Birthday paradox

> With n people in a room, what's the probability some pair shares a birthday?

P(no collision) = ∏ᵢ₌₀ⁿ⁻¹ (1 - i/365). For n = 23, P(collision) ≈ 0.507. The threshold scales as √m (where m is the number of possible birthdays/buckets). This is why a 128-bit hash provides ~64 bits of *collision resistance*, not 128.

### Coupon collector

> A stream of items, each independently uniform from {1, …, n}. Expected time to collect all n distinct items?

Let Tᵢ be the time to find the iₜₕ new coupon after having i − 1. P(new) = (n − i + 1)/n, so E[Tᵢ] = n/(n − i + 1). Sum:

```
E[T] = Σᵢ₌₁ⁿ n/(n − i + 1) = n · Hₙ
```

where Hₙ is the nₜₕ harmonic number ≈ ln n. So E[T] ≈ n · ln n. For n = 365, you need ≈ 365 · 6 ≈ 2,365 days to collect all birthdays.

## Build It

Open `code/main.py`. We'll simulate and analytically verify.

### Step 1: Linearity-of-expectation simulation (ascents in a permutation)

Generate many random permutations of length 20; count ascents; verify the mean is ≈ 19/2 = 9.5.

### Step 2: Birthday paradox

For n = 5, 10, 20, 23, 30, 50: simulate the probability of a collision in a year of 365 days; compare with the closed form.

### Step 3: Coupon collector

Simulate; average time to collect all 100 distinct items. Compare with `100 · H₁₀₀ ≈ 518.7`.

### Step 4: Two coin tosses sanity check

For X = number of heads in 1000 fair coin tosses (μ = 500, σ² = 250). Simulate, compare with Markov and Chebyshev bounds — see how loose Markov is.

## Use It

- **Randomized algorithms** (Phase 04): quicksort, randomized selection, Karger's min-cut all need expectation analysis.
- **Hash tables**: load factor, chain length, birthday-paradox-based collision analysis.
- **Bloom filters**: false-positive probability is a straight union-bound calculation.
- **Randomized rounding** for approximation algorithms: expected approximation ratio.
- **Crypto / security**: probability of guessing a key, generating two equal hashes (birthday attacks).

## Read the Source

- *Introduction to Probability* by Blitzstein & Hwang — clear, free Harvard textbook.
- *Probability and Computing* by Mitzenmacher & Upfal — best book on randomized algorithms.
- *CLRS* Chapter 5 (Probabilistic Analysis & Randomized Algorithms).

## Ship It

This lesson ships **`outputs/prob.py`** — `expectation_estimate(rv, trials)`, `birthday_prob(n, year)`, `coupon_collector_sim(n)`, plus Markov / Chebyshev computation.

## Exercises

1. **Easy.** Roll two fair 6-sided dice. What's the expected sum? Verify with linearity (= 7) and with brute-force enumeration.
2. **Medium.** Randomized quicksort on a random array of length n picks a uniform pivot. Show, using linearity over comparisons, that the expected number of comparisons is `2(n+1)Hₙ - 4n = O(n log n)`.
3. **Hard.** A hash table of m bins, n items inserted uniform-at-random. Find the expected number of empty bins (linearity of indicator variables), the maximum bin load (Chernoff), and the probability of a collision (birthday).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Random variable | "A variable with random value" | A function X: Ω → ℝ mapping each outcome to a number |
| Expectation | "Average" | E[X] = Σ x · P(X=x); the long-run mean if you sampled X many times |
| Linearity of expectation | "Sum of expectations" | E[X + Y] = E[X] + E[Y], even if X and Y are dependent |
| Indicator | "0/1 variable" | 1_A equals 1 on event A, 0 otherwise; E[1_A] = P(A) |
| Birthday paradox | "Square-root collision" | In m bins, the expected first collision arrives after ~√m insertions |

## Further Reading

- *Probability with Martingales* by D. Williams — Chapter 11+ for advanced randomized-algorithm analysis.
- *The Probabilistic Method* by Alon & Spencer — every theorem proved by an expectation argument.
- [Stanford CS265 notes on randomized algorithms](https://web.stanford.edu/class/cs265/) — clear and CS-focused.
