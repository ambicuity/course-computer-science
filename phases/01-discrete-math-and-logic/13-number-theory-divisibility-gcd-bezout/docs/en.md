# Number Theory — Divisibility, GCD, Bezout

> The Euclidean algorithm is older than written algebra, still the fastest way to compute a GCD, and the foundation of every public-key cryptosystem in current use.

**Type:** Build
**Languages:** Python, Rust
**Prerequisites:** Phase 01, Lessons 03 (induction), 11 (recurrences)
**Time:** ~60 minutes

## Learning Objectives

- State the definition of divisibility (`a | b`) and the basic divisibility lemmas (transitivity, linearity).
- Compute `gcd(a, b)` via the Euclidean algorithm; explain why it terminates and runs in `O(log min(a, b))`.
- Apply the **extended Euclidean algorithm** to find Bezout coefficients `x, y` with `ax + by = gcd(a, b)`.
- Use Bezout's identity to (a) decide whether linear Diophantine equations have integer solutions, (b) compute modular inverses (Lesson 14), and (c) understand RSA's key generation (Phase 12).

## The Problem

Three problems that look unrelated:

1. "Reduce 462/1071 to lowest terms." (Need GCD.)
2. "Solve 11x + 13y = 1 in integers." (Linear Diophantine equation.)
3. "Find the modular inverse of 11 mod 13." (Used in RSA.)

All three are solved by the same algorithm — Euclid's, dating to ~300 BCE — extended to track Bezout coefficients. This lesson covers that algorithm and its three applications.

## The Concept

### Divisibility

For integers a, b: we say `a | b` ("a divides b") iff there exists an integer k with `b = a · k`. Notation gotcha: `a | b` reads "a divides b," NOT "b divides a." Vertical bar = divides; slash `b / a` is just division.

Basic lemmas:
- **Transitivity**: `a | b ∧ b | c ⇒ a | c`.
- **Linearity**: `a | b ∧ a | c ⇒ a | (mb + nc)` for any integers m, n.
- **a | 0** for every a (because 0 = a · 0). And **1 | a** for every a.

### Greatest common divisor

`gcd(a, b)` is the largest positive integer that divides both a and b. Conventions: `gcd(0, 0) = 0`, `gcd(a, 0) = |a|`. **Coprime** means `gcd(a, b) = 1`.

### The Euclidean algorithm

> **Key identity**: `gcd(a, b) = gcd(b, a mod b)` for b > 0.

Proof sketch: any common divisor of a and b divides `a - q·b = a mod b`. Conversely, any common divisor of b and a mod b divides `a = q·b + (a mod b)`. The two sets of common divisors are the same, so they share a max.

Algorithm:

```python
def gcd(a, b):
    a, b = abs(a), abs(b)
    while b:
        a, b = b, a % b
    return a
```

Terminates because `b` strictly decreases (and stays non-negative). Runtime: O(log min(a, b)) because each modulo operation at least *halves* the smaller argument after two steps (a fact related to Fibonacci — the worst case is `gcd(F_{n+1}, F_n)`).

### Bezout's identity

> For any integers a, b, there exist integers x, y with **`ax + by = gcd(a, b)`**.

Equivalently: gcd(a, b) is the smallest *positive* integer expressible as ax + by. Most striking consequence: `gcd(a, b) = 1` iff there exist integers x, y with `ax + by = 1` — coprimality has a *witness*.

The extended Euclidean algorithm computes x and y alongside the GCD:

```python
def extended_gcd(a, b):
    """Returns (g, x, y) with a·x + b·y = g = gcd(a, b)."""
    if b == 0:
        return a, 1, 0
    g, x1, y1 = extended_gcd(b, a % b)
    # gcd(b, a mod b) = b·x1 + (a mod b)·y1
    # a mod b = a - (a // b) * b, so substitute:
    return g, y1, x1 - (a // b) * y1
```

### Linear Diophantine equations

`ax + by = c` has integer solutions **iff `gcd(a, b) | c`**. If it does, every solution is:

```
x = x₀ + (b/g) · k
y = y₀ - (a/g) · k    for k ∈ ℤ
```

where (x₀, y₀) is any one solution (e.g., scaled from extended_gcd).

Example: `11x + 13y = 1`. gcd(11, 13) = 1. `extended_gcd(11, 13)` yields `(1, 6, -5)` — i.e., `11·6 + 13·(-5) = 66 - 65 = 1`. ✓

### Modular inverse (preview of Lesson 14)

The modular inverse of a mod m is an integer x with `ax ≡ 1 (mod m)`. It exists iff `gcd(a, m) = 1`. Found by extended Euclidean:

```python
def modinv(a, m):
    g, x, _ = extended_gcd(a, m)
    if g != 1:
        raise ValueError(f"no inverse: gcd({a},{m})={g}")
    return x % m
```

This single function is the workhorse of RSA, ECC, and Diffie-Hellman.

### Worst case is Fibonacci

The Euclidean algorithm takes the most steps when the inputs are consecutive Fibonacci numbers. `gcd(F_{n+1}, F_n)` requires n - 1 steps. This proves the O(log) bound: F_n grows exponentially in n, so n is logarithmic in the input.

## Build It

Open `code/main.py`.

### Step 1: Plain Euclidean GCD

```python
def gcd(a, b):
    a, b = abs(a), abs(b)
    while b: a, b = b, a % b
    return a
```

Verify `gcd(462, 1071) == 21` and `462/1071 == 22/51`.

### Step 2: Extended Euclidean

The recursive version above. Verify the Bezout identity:

```python
g, x, y = extended_gcd(11, 13)
assert 11*x + 13*y == g
```

### Step 3: Linear Diophantine solver

Use extended_gcd to find a base solution, then scale.

### Step 4: Modular inverse

```python
def modinv(a, m):
    g, x, _ = extended_gcd(a, m)
    if g != 1: raise ValueError("gcd != 1")
    return x % m
```

Verify `(3 · modinv(3, 7)) % 7 == 1`.

### Step 5: Verify worst case is Fibonacci

Track number of mod operations: `gcd(F_{n+1}, F_n)` should take exactly n-1 steps, matching the theoretical worst case.

### Step 6: Rust version

`code/main.rs` implements the same algorithms in Rust with `i128` for large inputs and panics on overflow.

## Use It

- **RSA key generation** (Phase 12): pick primes p, q; modulus n = pq; choose public exponent e coprime to (p-1)(q-1); compute private exponent `d = modinv(e, (p-1)(q-1))` via extended Euclidean. The entire scheme rests on this lesson's algorithm.
- **Linear Diophantine equations**: change-making, coin-problem variants.
- **Sieve enumeration**: counting coprime integers in [1, n].
- **CRT (Chinese Remainder Theorem)**: reconstruct an integer from its residues; combinations use modinv.
- **Computer-algebra systems**: GCD over polynomials uses the same algorithm (and a generalization called the *subresultant* algorithm).

## Read the Source

- Euclid's *Elements*, Book VII, Propositions 1–2 — the original algorithm. Still readable.
- *Concrete Mathematics*, §4.5 — clean derivation with proofs.
- *A Computational Introduction to Number Theory and Algebra* by Victor Shoup — free PDF; the cleanest CS-flavored treatment.

## Ship It

This lesson ships **`outputs/euclid.py`** — `gcd`, `lcm`, `extended_gcd`, `modinv`, `diophantine`. Used directly in Lesson 14 (modular arithmetic) and Phase 12 (cryptography).

## Exercises

1. **Easy.** Compute gcd(462, 1071) by hand, showing each step of the Euclidean algorithm. Verify with the lesson library.
2. **Medium.** Find ALL non-negative integer solutions to `7x + 11y = 100`. (Use the diophantine solver to get the general form; pick out the non-negative ones.)
3. **Hard.** Prove (with induction): `gcd(F_{n+1}, F_n) = 1` for every n. (Hint: F_{n+1} = F_n + F_{n-1}.) Conclude that consecutive Fibonacci numbers are coprime — this is what makes Fibonacci the worst-case input.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Divides | "a \| b" | ∃ integer k with b = a·k |
| GCD | "Greatest common divisor" | The largest positive integer that divides both a and b |
| Coprime | "Relatively prime" | gcd(a, b) = 1 — no common factor other than 1 |
| Bezout's identity | "ax + by = gcd" | The GCD can always be written as an integer linear combination of a and b |
| Modular inverse | "1/a mod m" | The integer x with ax ≡ 1 (mod m); exists iff gcd(a, m) = 1 |

## Further Reading

- *The Art of Computer Programming, Vol 2* by Knuth — §4.5.2, "The Greatest Common Divisor," is exhaustive.
- [The Stein binary GCD algorithm](https://en.wikipedia.org/wiki/Binary_GCD_algorithm) — replaces modulo with shifts/subtractions; faster on hardware without fast mod.
- *Cryptography Engineering* by Ferguson, Schneier, Kohno — Chapter 12 walks through RSA key gen step by step.
