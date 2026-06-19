# Primes, Sieves, Primality Tests

> Find every prime up to N in O(N log log N). Decide whether a 1024-bit number is prime in milliseconds, with vanishing error probability.

**Type:** Build
**Languages:** Python, Rust
**Prerequisites:** Phase 01, Lessons 13–14
**Time:** ~75 minutes

## Learning Objectives

- Implement the Sieve of Eratosthenes; analyze its time and space complexity (O(N log log N) and O(N)).
- Implement trial-division primality testing; explain when to use it (small numbers) and when not (anything > 10^10).
- Implement Miller-Rabin probabilistic primality testing; explain its error bound and the small deterministic-witness sets.
- Use Pollard's rho or the Pratt certificate to factor or *certify* primality of medium-size integers.

## The Problem

Primes underpin cryptography (RSA, ECC, hash function design), number-theoretic algorithms (modular arithmetic, NTT), and pseudo-random number generation. Two recurring tasks:

1. **Enumerate primes up to N.** ("All primes < 10^6.") → Sieve.
2. **Decide whether a specific n is prime.** ("Is this 2048-bit number prime?") → Probabilistic primality test.

These are different problems with different best algorithms. The sieve is unbeatable for "all primes up to N." Trial division works for tiny n. Miller-Rabin is the workhorse for big n. AKS (2002) is deterministic polynomial-time but slower than Miller-Rabin in practice.

## The Concept

### Sieve of Eratosthenes

> Start with the list `[True] * (N+1)` representing "prime?" for each index.
> For each i from 2 to √N: if `is_prime[i]`, mark all multiples of i (starting from i²) as not prime.

```python
def sieve(N):
    is_prime = [True] * (N + 1)
    is_prime[0] = is_prime[1] = False
    for i in range(2, int(N**0.5) + 1):
        if is_prime[i]:
            for j in range(i*i, N + 1, i):
                is_prime[j] = False
    return [i for i, p in enumerate(is_prime) if p]
```

Complexity: O(N log log N) time, O(N) space. The log log comes from Mertens' theorem: Σ 1/p over primes p ≤ N is asymptotically log log N. For N = 10^7 the sieve fills in ~100 ms.

Variants:
- **Linear sieve**: O(N) by tracking each composite's smallest prime factor (slightly faster constant).
- **Segmented sieve**: process [L, R] without allocating [0, R]; useful for huge ranges.

### Counting primes

The prime-counting function π(N) counts primes ≤ N. The **Prime Number Theorem**:

```
π(N) ~ N / ln N        (asymptotically)
```

Implications: there are "lots" of primes — a random N-bit integer is prime with probability ~ 1/(ln 2^N) ≈ 1.44 / N. To find a 1024-bit prime, you try ~710 random candidates on average.

### Trial division

The naïve test: try every potential divisor up to √n. O(√n) per test, fine for n < 10^8 or so. Used as a quick screen before more expensive tests.

```python
def trial_division(n):
    if n < 2: return False
    if n % 2 == 0: return n == 2
    d = 3
    while d * d <= n:
        if n % d == 0: return False
        d += 2
    return True
```

### Fermat primality test (and why it fails)

Fermat: if n is prime and gcd(a, n) = 1, then `a^(n-1) ≡ 1 (mod n)`. So if you find an `a` with `a^(n-1) ≢ 1`, n is composite.

Problem: Carmichael numbers (Lesson 14) pass Fermat for every coprime witness. The smallest is 561.

### Miller-Rabin primality test

Miller-Rabin strengthens Fermat by additionally testing for *non-trivial square roots of 1*. The setup:

For an odd n, write `n - 1 = 2^s · d` where d is odd. For each random witness a in [2, n-2]:

```
x = a^d mod n
if x == 1 or x == n - 1: maybe-prime
else:
    for r in 1 .. s-1:
        x = x*x mod n
        if x == n - 1: maybe-prime; break
    else: COMPOSITE   (definitely composite)
```

If a passes, n is *probably prime*; the probability that a composite number passes is ≤ 1/4 per witness. With k random witnesses, error ≤ 4^(-k). With 20 witnesses, error ≤ 10^(-12). Production crypto uses 40+ witnesses.

For n < a few specific thresholds, *deterministic* small witness sets suffice. For n < 3,317,044,064,679,887,385,961,981 (the verified bound), the witnesses {2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37} are sufficient — making Miller-Rabin a deterministic test in practice for any 64-bit number.

### Pollard's rho factoring

When you need *factors* (not just "is this prime?"), Pollard's rho is the standard sub-exponential algorithm:

```
x = 2, y = 2, d = 1
f(x) = (x*x + 1) mod n
while d == 1:
    x = f(x)
    y = f(f(y))
    d = gcd(|x - y|, n)
if d == n: try a different f
else: d is a non-trivial factor
```

Expected runtime ~ O(n^(1/4)). For 64-bit composites, this is ~10^5 steps — fast.

## Build It

Open `code/main.py`.

### Step 1: Sieve of Eratosthenes

The classic. Verify: 25 primes below 100; 168 below 1000; 78,498 below 10^6.

### Step 2: Prime-counting check (PNT)

For increasing N, compute π(N) and N / ln(N); the ratio converges to 1.

### Step 3: Trial division

The naïve test. Compare against the sieve on n ≤ 10000.

### Step 4: Miller-Rabin

Build the full probabilistic primality test. Verify: it identifies 561 as composite (where Fermat fails), the Mersenne prime 2^31 - 1 as prime, and 2^61 - 1 (a known Mersenne prime) as prime.

### Step 5: Find big primes

A random 64-bit prime via Miller-Rabin: pick odd candidates, test.

### Step 6: Rust version

`code/main.rs` reimplements the sieve and Miller-Rabin for u128.

## Use It

- **RSA key generation** (Phase 12): generate two 1024-bit primes via Miller-Rabin.
- **Hash table sizing**: prime moduli reduce clustering in linear-probing hash tables.
- **Number-theoretic transform**: requires a prime modulus with the right structure.
- **Cryptographic PRNGs**: many require generation of "safe primes" (p prime, (p-1)/2 also prime).
- **Project Euler / competitive programming**: the sieve is the bread-and-butter precomputation.

## Read the Source

- *The Art of Computer Programming, Vol 2* (Knuth), §4.5.4 — exhaustive primality / factoring.
- [Crandall & Pomerance — *Prime Numbers: A Computational Perspective*](https://www.springer.com/gp/book/9780387252827) — full reference on practical algorithms.
- [GMP's mpz_nextprime](https://gmplib.org/manual/Number-Theoretic-Functions.html) — production-grade prime search.

## Ship It

This lesson ships **`outputs/primes.py`** — `sieve(N)`, `is_prime(n)` (Miller-Rabin), `next_prime(n)`, `pollard_rho(n)`. Used in Lesson 17 (graph theory has prime-shuffling for hashing) and Phase 12 (cryptography).

## Exercises

1. **Easy.** Use the sieve to count primes below 10^7. Compare with N/ln(N) — the ratio should be ~1.0.
2. **Medium.** Verify that 561 fools Fermat but not Miller-Rabin: run Fermat with random a ≤ 100 (always passes), then Miller-Rabin (returns composite). Identify which witness exposes the compositeness.
3. **Hard.** Implement a segmented sieve over [10^18, 10^18 + 10^6] without allocating the full range. Use small primes from a base sieve to mark composites in the window.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Sieve of Eratosthenes | "Mark multiples" | O(N log log N) algorithm enumerating all primes ≤ N |
| Prime-counting function π(N) | "How many primes" | π(N) ~ N / ln N by the Prime Number Theorem |
| Miller-Rabin | "Strong Fermat test" | Probabilistic primality test with error ≤ 4^(-k) per k random witnesses |
| Witness | "Test value" | An a that exposes n as composite via failing a^(n-1) ≡ 1 or a stronger Miller-Rabin check |
| Carmichael number | "Fermat liar" | A composite that passes Fermat for every coprime witness; doesn't fool Miller-Rabin |

## Further Reading

- [The AKS primality test (2002)](https://annals.math.princeton.edu/2004/160-2/p12) — the first unconditional deterministic polynomial-time primality test. Beautiful but slower in practice than Miller-Rabin.
- [Pollard's original rho paper](https://link.springer.com/article/10.1007/BF01933667) — short and elegant.
- [Project Nayuki — Miller-Rabin walkthrough](https://www.nayuki.io/page/miller-rabin-primality-test) — clean code with proof sketches.
