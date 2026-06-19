# Modular Arithmetic & Fermat / Euler

> Wrap-around arithmetic. Half the algorithms in cryptography and hashing live here.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 01, Lesson 13
**Time:** ~60 minutes

## Learning Objectives

- Compute modular addition, subtraction, multiplication, and exponentiation correctly without overflow.
- Apply **Fermat's little theorem** (`a^(p-1) ≡ 1 mod p` for prime p, gcd(a, p)=1) and **Euler's theorem** (`a^φ(n) ≡ 1 mod n` for gcd(a, n)=1) to simplify huge exponents.
- Compute Euler's totient `φ(n)` from prime factorization and from coprimality counts.
- Implement modular exponentiation via repeated squaring in O(log e); use it as the workhorse of RSA, ECC, and primality tests.

## The Problem

Modular arithmetic is what computers actually do (32-bit ints overflow modulo 2^32) and what cryptography is built on (every operation in RSA is `mod n`). Three concrete tasks:

1. Compute `7^200 mod 13`. (Naïve: huge number. Smart: Fermat → 7^200 = 7^(200 mod 12) = 7^8 mod 13.)
2. Solve `5x ≡ 1 mod 14`. (Modular inverse, Lesson 13.)
3. Verify that a 512-bit number is "probably prime." (Miller-Rabin, Lesson 15, uses modular exponentiation throughout.)

All three need fluent modular arithmetic.

## The Concept

### The basics

Working mod n means partitioning ℤ into n classes [0], [1], …, [n-1]. Two integers are equivalent (`a ≡ b mod n`) iff `n | (a - b)`. We work with *one representative* per class, usually in `{0, 1, …, n-1}` ("canonical residue").

Arithmetic is well-defined modulo n:

- `(a + b) mod n = ((a mod n) + (b mod n)) mod n`
- `(a · b) mod n = ((a mod n) · (b mod n)) mod n`
- `(-a) mod n = (n - (a mod n)) mod n`

Subtraction and division are subtler:

- Subtraction works the same as addition with negated b.
- **Division does not exist in general** — you need a *modular inverse*. `a / b mod n` becomes `a · b⁻¹ mod n`, which exists iff `gcd(b, n) = 1`.

### Repeated-squaring exponentiation

To compute `a^e mod n` for large e (e.g., e = 10^9), the naïve loop is O(e). Use binary exponentiation:

```python
def modpow(a, e, n):
    result = 1
    a %= n
    while e > 0:
        if e & 1:
            result = (result * a) % n
        a = (a * a) % n
        e >>= 1
    return result
```

`O(log e)` multiplications. Every cryptographic operation uses this. Python's built-in `pow(a, e, n)` is the same algorithm in C.

### Fermat's little theorem

> If p is prime and `gcd(a, p) = 1`, then `a^(p-1) ≡ 1 mod p`.

Equivalently: `a^p ≡ a mod p` for every a (no coprimality needed for this form).

Use case: compute `a^e mod p` by reducing the exponent mod (p - 1):

```
a^e mod p = a^(e mod (p-1)) mod p     (when gcd(a, p) = 1)
```

Example: `7^200 mod 13`. p = 13, p - 1 = 12. 200 mod 12 = 8. So `7^200 ≡ 7^8 (mod 13)`.

Cryptographic significance: Fermat's theorem is the basis of the Fermat primality test (used as a screen before Miller-Rabin) and a special case of RSA's correctness proof.

### Euler's theorem (generalization to composite moduli)

> If `gcd(a, n) = 1`, then `a^φ(n) ≡ 1 mod n`.

Where `φ(n)` (Euler's totient) is the count of integers in [1, n] coprime to n. For prime p: `φ(p) = p - 1` (recovers Fermat). For prime powers: `φ(p^k) = p^k - p^(k-1) = p^(k-1)(p-1)`. For coprime products: `φ(ab) = φ(a) · φ(b)`.

Closed form via prime factorization:

```
φ(n) = n · ∏ (1 - 1/p)   for each prime p dividing n
```

Example: φ(12) = 12 · (1 - 1/2) · (1 - 1/3) = 12 · 1/2 · 2/3 = 4. Check: the coprime-to-12 residues in [1,12] are {1, 5, 7, 11} — four of them.

### RSA correctness sketch

RSA: public key (n, e), private (n, d) with n = pq, d = e⁻¹ mod φ(n). Encrypt: `c = m^e mod n`. Decrypt: `m' = c^d mod n`. Why m' = m?

```
c^d = m^(ed)        ed ≡ 1 mod φ(n)
    = m^(1 + k·φ(n))  for some integer k
    = m · (m^φ(n))^k
    ≡ m · 1^k         by Euler's theorem (when gcd(m, n) = 1)
    = m
```

Modular arithmetic + Euler = RSA decryption.

### Common pitfalls

1. **Negative results from `%`**: in C, `(-7) % 3 == -1`; in Python, `(-7) % 3 == 2`. Always normalize: `((a % n) + n) % n`.
2. **Order of operations**: do `(a * b) % n`, not `(a * (b % n))` alone — the intermediate `a * b` may overflow.
3. **Modular inverse**: doesn't always exist. Don't blindly divide.
4. **Carmichael numbers**: composite numbers that satisfy Fermat's theorem for *every* coprime a. The smallest is 561. They fool the Fermat primality test — Miller-Rabin (Lesson 15) doesn't have this weakness.

## Build It

Open `code/main.py`.

### Step 1: `modpow` via repeated squaring

The standard binary algorithm. Python's `pow(a, e, n)` does this internally for arbitrarily large integers.

### Step 2: Verify Fermat

For each prime p and a with gcd(a, p) = 1, confirm `pow(a, p-1, p) == 1`.

### Step 3: Compute Euler's totient

```python
def phi(n):
    out = n
    p = 2
    while p * p <= n:
        if n % p == 0:
            while n % p == 0: n //= p
            out -= out // p
        p += 1
    if n > 1: out -= out // n
    return out
```

Verify `phi(12) == 4` and `phi(36) == 12`.

### Step 4: Verify Euler

For each composite n and a coprime to n, confirm `pow(a, phi(n), n) == 1`.

### Step 5: Find a Carmichael number

561 = 3 · 11 · 17 is the smallest composite that passes Fermat for every a coprime to 561. Verify: `pow(a, 560, 561) == 1` for every a with gcd(a, 561) = 1.

### Step 6: A toy RSA

Pick small primes, set up keys, encrypt and decrypt a tiny message. (Real RSA uses 2048+-bit primes; this lesson's toy version is for understanding.)

## Use It

- **RSA / DSA / Diffie-Hellman**: every operation is modular exponentiation under Fermat / Euler (Phase 12).
- **Elliptic-curve crypto**: modular arithmetic over `GF(p)`; everything in this lesson generalizes.
- **Hashing**: many hash functions are modular polynomial computations.
- **Number-theoretic transform**: FFT-style algorithms over modular fields (Phase 04 L21).
- **Hashing distributed systems**: consistent hashing's coverage analysis uses modular pseudo-uniform distribution.

## Read the Source

- *A Computational Introduction to Number Theory and Algebra* (Shoup), Chapter 2-4.
- [Python `pow(a, e, n)` source](https://github.com/python/cpython/blob/main/Objects/longobject.c) — search for `long_pow`; ~150 lines of repeated squaring.
- Real-world: [Constant-time modpow in BoringSSL](https://github.com/google/boringssl/blob/master/crypto/fipsmodule/bn/exponentiation.c) — production-grade, side-channel-resistant.

## Ship It

This lesson ships **`outputs/modular.py`** — `modpow`, `phi`, `totient_table`, `is_fermat_witness`, `toy_rsa`. Used by Lesson 15 and Phase 12.

## Exercises

1. **Easy.** Compute `2^1000 mod 1000`. (Big exponent, small modulus — easy with modpow.)
2. **Medium.** Show: if p is an *odd* prime, then `(p-1)! ≡ -1 mod p` (Wilson's theorem). Verify for p ≤ 100.
3. **Hard.** Implement a Fermat primality test: pick random a, check `a^(n-1) ≡ 1 mod n`. Find a *Carmichael number* (a composite that passes Fermat for every coprime a). The smallest is 561; verify.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Modular arithmetic | "Wrap-around" | Arithmetic in ℤ/nℤ, where two integers are equivalent if they differ by a multiple of n |
| Fermat's little theorem | "a^(p-1) ≡ 1" | For prime p and gcd(a, p) = 1, a raised to p-1 reduces to 1 mod p |
| Euler's totient φ(n) | "Coprime count" | Number of integers in [1, n] that are coprime to n |
| Modular exponentiation | "modpow" | a^e mod n via repeated squaring in O(log e) multiplications |
| Carmichael number | "Fermat liar" | A composite n that satisfies Fermat's theorem for every a coprime to n; foils naïve primality testing |

## Further Reading

- *A First Course in Number Theory* by Niven, Zuckerman, Montgomery — classic, with thorough proofs.
- [Wikipedia: Fermat's little theorem](https://en.wikipedia.org/wiki/Fermat%27s_little_theorem) — multiple proofs; the combinatorial one (necklace counting) is beautiful.
- *Cryptography Engineering* by Ferguson, Schneier, Kohno — Chapter 10 on number-theoretic prelims for crypto.
