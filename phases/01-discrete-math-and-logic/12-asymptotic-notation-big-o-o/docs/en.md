# Asymptotic Notation — Big-O, Θ, Ω, o, ω

> Big-O is a *set* of functions, not a single function. Once you internalize that, all the "rules of Big-O" become consequences of one definition.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 01, Lesson 11
**Time:** ~60 minutes

## Learning Objectives

- State the formal definitions of O, Θ, Ω, o, ω in terms of limits or quantifier statements.
- Compare functions asymptotically (which dominates, which are equivalent up to constants) using the limit-ratio test.
- Apply the algebra of asymptotic notation: O(f) + O(g) = O(max(f, g)), O(f) · O(g) = O(f · g), polynomial vs exponential vs log.
- Recognize and avoid common abuse: "is O(n) faster than O(n²)?" (not always, for small n), "O(n) ⊆ O(n²)" (yes, by definition).

## The Problem

Asymptotic notation is the language of algorithm analysis. Without it, statements like "merge sort is faster than insertion sort" are slippery: faster on what input size? With what constant factors? Asymptotic notation says: in the limit of large n, this is the dominant growth rate.

But the notation is also routinely abused. People write things like "the algorithm is O(n²)" when they mean "*at most* O(n²)" vs "*exactly* Θ(n²)." They claim "O(n) is faster than O(n²)" without noticing the constants. They forget that `O(...)` is a *set*, not a number.

This lesson is the formal language. After it, you'll read algorithm papers without flinching.

## The Concept

### The five notations

For functions f, g: ℕ → ℝ⁺:

| Notation | Reads as | Definition |
|----------|----------|------------|
| **f = O(g)**  | "f is at most g (up to constants)" | ∃ c, n₀ > 0. ∀ n ≥ n₀. `f(n) ≤ c · g(n)` |
| **f = Ω(g)**  | "f is at least g" | ∃ c, n₀ > 0. ∀ n ≥ n₀. `f(n) ≥ c · g(n)` |
| **f = Θ(g)**  | "f and g grow the same" | f = O(g) AND f = Ω(g) |
| **f = o(g)**  | "f is strictly less than g" | ∀ c > 0. ∃ n₀. ∀ n ≥ n₀. `f(n) ≤ c · g(n)`; equivalently `lim f/g = 0` |
| **f = ω(g)**  | "f strictly exceeds g" | ∀ c > 0. ∃ n₀. ∀ n ≥ n₀. `f(n) ≥ c · g(n)`; equivalently `lim f/g = ∞` |

Mnemonic: O ≈ ≤, Ω ≈ ≥, Θ ≈ =, o ≈ <, ω ≈ >.

`O(f)` is, formally, the *set* of all functions g with g = O(f). When you write `T(n) = O(n²)`, the `=` is really `∈`: T is *one element* of the set O(n²). That's why "O(n) ⊆ O(n²)" makes sense, but the reverse doesn't.

### The limit-ratio test

To compare f and g asymptotically:

```
            lim   f(n) / g(n)
           n→∞
       = 0  → f = o(g)
       = c (positive finite)  → f = Θ(g)
       = ∞  → f = ω(g)
       does not exist (oscillates) → use the inf/sup definitions directly
```

For most algorithm functions, this is the fastest path to comparison.

### The growth-rate hierarchy

Memorize this ladder (smaller is below):

```
   1                       constant
   log log n               very slow
   log n                   logarithmic
   log² n
   n^ε  (0 < ε < 1)        sublinear
   n                       linear
   n log n                 linearithmic
   n log² n
   n²                      quadratic
   n³                      cubic
   nᵏ                      polynomial of degree k
   n^log n
   2^n                     exponential
   3^n
   n!                      factorial
   n^n
   2^(2^n)                 doubly exponential
```

Each row is `o(` of every row below it. This single list answers most "is f better than g?" questions in algorithm analysis.

### Algebra of asymptotic notation

| Operation | Identity |
|-----------|----------|
| Sum | O(f) + O(g) = O(max(f, g)); when f, g positive, = O(f + g) |
| Product | O(f) · O(g) = O(f · g) |
| Constant multiple | O(c · f) = O(f) |
| Compose | O(f(g)) treats f and g algebraically; e.g., O(2 log n) = O(log n) since 2 is a constant |
| Power | (O(f))^k = O(f^k) |

**Polynomials**: O(aₖ nᵏ + … + a₀) = O(nᵏ). Lower-order terms vanish; the highest exponent wins.

**Logarithms**: O(log_a n) = O(log_b n) for any constant a, b (change of base is a constant factor). So `log` in `O(...)` doesn't need a base.

### Common abuses to watch for

1. **"O(n) is always faster than O(n²)"** — not in absolute terms for small n. A 1000n algorithm is slower than a 0.01n² algorithm for n < 100,000. Big-O describes the limit, not constant factors.

2. **"This is O(n²)"** — strictly, this only says "≤ n²" up to constants. It might *also* be O(n log n). To say "exactly n²," use Θ(n²).

3. **Mixing up f = O(g) and f ∈ O(g)** — they mean the same thing; the abuse-of-`=` is conventional. But it's NOT symmetric: O(n) = O(n²) does NOT mean O(n²) = O(n).

4. **"O(log n)" without a base** — fine, because of change-of-base. But "O(2^n)" cares about the base: 2^n is strictly smaller than 3^n in the o(·) sense.

5. **Hidden polylogs**: "O(n)" might quietly be hiding "O(n log n)" if you didn't analyze a sort step. Always check.

### Master-theorem terms in asymptotic notation

| Recurrence | Master case | Result in Big-Θ |
|------------|-------------|-----------------|
| T(n) = 2T(n/2) + n | 2 | Θ(n log n) |
| T(n) = T(n/2) + 1 | 2 | Θ(log n) |
| T(n) = 3T(n/2) + n | 1 | Θ(n^log₂3) |
| T(n) = T(n/2) + n | 3 | Θ(n) |
| T(n) = T(n-1) + n | (linear recursion) | Θ(n²) |

## Build It

Open `code/main.py`.

### Step 1: Limit-ratio comparison

Run `f(n) / g(n)` for ramping n; observe whether it goes to 0 (o), a constant (Θ), or ∞ (ω).

### Step 2: Hierarchy table

Tabulate `1, log n, n, n log n, n², n³, 2ⁿ, n!` at n = 10, 30, 100. Confirm that 2ⁿ explodes past n²·n³ as n grows.

### Step 3: Polynomial / logarithm identities

- `3n² + 5n + 7` divided by `n²` converges to 3 (constant) — so the polynomial is Θ(n²).
- `log₂(n) / log₁₀(n)` is constant ≈ 3.322 for every n — so logs at different bases are interchangeable inside O(·).

### Step 4: Constant-factor crossover

Plot empirical runtime of "1000n" vs "0.01n²". For n < 100000, the quadratic is faster. The crossover is where asymptotic dominance kicks in — and it can be far past production input sizes.

## Use It

- **Algorithm analysis**: every algorithm in Phase 04 gets an O/Θ analysis.
- **Performance engineering** (Phase 15): asymptotic bound + measured constant factors guide micro-optimizations.
- **Cryptography**: security claims are "no polynomial-time adversary can break X with non-negligible probability" — direct asymptotic statements.
- **Complexity theory** (Phase 05): P, NP, EXP, BPP are defined via Big-O of resources.
- **Compiler optimization**: passes are bounded by O(IR-size) or O(IR-size · log) to remain practical.

## Read the Source

- *CLRS*, Chapter 3 (Growth of Functions) — the canonical asymptotic-notation chapter.
- [Big-O cheat sheet](https://www.bigocheatsheet.com/) — common operations' costs, useful for interview prep and sanity checks.
- *Concrete Mathematics* §9.2 — Knuth's careful treatment of O, Θ, Ω notation, including subtle issues.

## Ship It

This lesson ships **`outputs/growth.py`** — a small library with `compare_growth(f, g, ns)`, a hierarchy table, and a `crossover_n(f, g, c_f, c_g)` helper that finds the smallest n where c_f·f(n) ≥ c_g·g(n).

## Exercises

1. **Easy.** Compare: is `n^2.5` in O(n²)? In Ω(n²)? In Θ(n²)? Justify each.
2. **Medium.** Show `log(n!) = Θ(n log n)`. (Hint: Stirling's approximation gives log(n!) ≈ n log n - n + O(log n).)
3. **Hard.** Two algorithms have runtimes O(n^1.585) and O(n²) respectively. For what n does the first beat the second if their constants are c₁ = 1000 and c₂ = 1? At what n does the asymptotic dominance "kick in"?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Big-O | "Upper bound" | The set of functions bounded above by f, up to constants, for sufficiently large n |
| Big-Θ | "Tight bound" | Both O and Ω — same growth rate up to constants |
| little-o | "Strictly slower" | Limit ratio is 0; strictly dominated |
| Asymptotic | "In the limit" | Behavior as n → ∞; ignores constants and lower-order terms |
| Polynomial | "Manageable" | Of the form O(n^k) for some constant k; computer-science usually treats P as 'tractable' |

## Further Reading

- [Big-O / asymptotic notation, Wikipedia](https://en.wikipedia.org/wiki/Big_O_notation) — surprisingly precise; good cross-reference.
- *Introduction to the Theory of Computation* by Sipser — Chapter 7's treatment of complexity classes.
- [Algorithms (Sedgewick & Wayne) — analysis section](https://algs4.cs.princeton.edu/14analysis/) — empirical-meets-theoretical view of growth rates.
