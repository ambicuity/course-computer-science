# Public Key I — Diffie-Hellman

> Public Key I — Diffie-Hellman — the part of CS you can't skip.

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 12 lessons 01–08
**Time:** ~75 minutes

## Learning Objectives

- Understand the symmetric key distribution problem: two parties need a shared secret but only have an insecure channel.
- Model the Diffie-Hellman key exchange using color mixing as an analogy, then map it to modular arithmetic.
- Write a from-scratch DH implementation using modular exponentiation, random private keys, and shared secret derivation.
- Explain why the discrete logarithm problem makes DH secure and why ephemeral DH (DHE) provides forward secrecy.
- Identify the man-in-the-middle (MITM) vulnerability that exists because DH itself provides no authentication.
- Compare classic DH (modular exponentiation over safe primes) with modern elliptic-curve DH (X25519).

## The Problem

Symmetric-key ciphers (AES, ChaCha20) are fast and secure, but they require both parties to hold the same secret key. How do Alice and Bob agree on a key when the only channel available to them is insecure — an eavesdropper (Eve) can read every message?

The naive approach: Alice picks a key and sends it. Eve copies it. Game over.

This is the **key distribution problem**, and it looks like a chicken-and-egg paradox: to establish a secure channel, you first need a secure channel to exchange the key.

Before 1976, the only solutions were physical (courier, face-to-face meeting) or relied on a trusted third party (e.g., a bank's key distribution center). Neither scales to the open internet.

## The Concept

### Color-Mixing Analogy

Imagine Alice and Bob share a public color (say, yellow). Each picks a private color (Alice: red, Bob: blue). They mix their private color with the public color and send the result to each other. Alice gets (yellow + blue), Bob gets (yellow + red). Now Alice adds her private red to what she received: red + (yellow + blue) = yellow + red + blue. Bob adds his private blue: blue + (yellow + red) = yellow + blue + red. Both end up with the same mixture. Eve sees only the public yellow and the two mixed paints — she never sees the private colors, so she cannot reconstruct the final shared mixture.

### The Math

Replace "mixing colors" with modular exponentiation:

- Choose a large prime **p** and a generator **g** (both are public).
- Alice picks private key **a** (random), computes public key **A = g^a mod p**.
- Bob picks private key **b** (random), computes public key **B = g^b mod p**.
- They exchange **A** and **B** over the insecure channel.
- Alice computes **s = B^a mod p** = (g^b)^a = g^(ab) mod p.
- Bob computes **s = A^b mod p** = (g^a)^b = g^(ab) mod p.
- Both now share secret **s**. Eve sees p, g, A, B but cannot compute **s** because she does not know **a** or **b**.

### Security Foundation: The Discrete Log Problem

Given **g^a mod p**, finding **a** is computationally infeasible when **p** is a large safe prime (p = 2q + 1 where q is also prime) and **a** is chosen uniformly at random. This is the **discrete logarithm problem** — no known efficient classical algorithm exists.

### Man-in-the-Middle (MITM)

DH on its own provides **no authentication**. Mallory the active attacker can:
1. Intercept Alice's **A**, replace it with her own **A'**.
2. Intercept Bob's **B**, replace it with her own **B'**.
3. Establish shared secret **s_AM** with Alice and **s_BM** with Bob.
4. Every message Alice sends, Mallory decrypts, reads, re-encrypts with **s_BM**, and forwards to Bob (and vice versa).

Neither Alice nor Bob detects the interception. This is why real protocols combine DH with digital signatures (e.g., TLS uses DH key exchange signed by the server's certificate).

### Ephemeral DH (DHE) and Forward Secrecy

In **DHE**, both parties generate fresh private keys for each session and discard them after the session ends. Even if an attacker records all encrypted traffic and later compromises the long-term keys, they cannot decrypt past sessions because the ephemeral private keys no longer exist. This property is called **forward secrecy**.

TLS 1.3 mandates DHE (or ECDHE) for all key exchanges — no static DH allowed.

## Build It

### Python

```python
import random

def mod_exp(base: int, exp: int, mod: int) -> int:
    result = 1
    base = base % mod
    while exp > 0:
        if exp & 1:
            result = (result * base) % mod
        base = (base * base) % mod
        exp >>= 1
    return result

def generate_keypair(p: int, g: int) -> tuple[int, int]:
    private = random.randrange(2, p - 1)
    public = mod_exp(g, private, p)
    return private, public

def compute_shared_secret(their_pub: int, my_priv: int, p: int) -> int:
    return mod_exp(their_pub, my_priv, p)
```

The `mod_exp` function uses square-and-multiply (O(log exp) multiplications instead of O(exp)). The private key is chosen uniformly from [2, p-2]. The shared secret is `their_pub^my_priv mod p`, which by the properties of modular exponentiation equals `g^(a*b) mod p`.

### Rust

```rust
fn mod_pow(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    let mut result = 1u64;
    base %= modulus;
    while exp > 0 {
        if exp & 1 == 1 {
            result = (result as u128 * base as u128 % modulus as u128) as u64;
        }
        base = (base as u128 * base as u128 % modulus as u128) as u64;
        exp >>= 1;
    }
    result
}
```

Rust's implementation uses `u128` intermediates to avoid overflow during multiplication — a real concern for 64-bit arithmetic with large moduli.

## Use It

Modern DH has moved from the original modular-exponentiation form to **elliptic-curve DH (ECDH)**. The most widely deployed implementation is **X25519** (Curve25519), designed by Daniel J. Bernstein:

- Uses the Curve25519 elliptic curve (y^2 = x^3 + 486662x^2 + x over GF(2^255 - 19)).
- Private keys are 32 random bytes; public keys are 32 bytes.
- Fast, constant-time, no side-channel leakage.
- Adopted by TLS 1.3, SSH, Signal, WireGuard, and most modern crypto libraries.

TLS 1.3 uses **ECDHE** (Elliptic-Curve Diffie-Hellman Ephemeral) exclusively for key agreement. The server sends its ephemeral ECDHE public key in the ServerKeyExchange message, signed by its certificate for authentication.

## Read the Source

- **RFC 7919**: Negotiated Finite Field Diffie-Hellman Ephemeral Parameters for TLS — defines the safe-prime groups used in TLS DHE.
- **OpenSSL `crypto/dh/dh_key.c`**: Implementation of DH key generation and shared secret computation (see `dh_generate_key` and `dh_compute_key`).
- **Rust `x25519-dalek` crate**: Clean, audited X25519 implementation — study `x25519.rs` for the scalar multiplication internals.
- **Cloudflare `circl` library**: Go implementations of X25519 and other modern DH primitives.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained DH key exchange library** (Python class or Rust module) that you can drop into any project needing asymmetric key agreement.

## Exercises

1. **Easy** — Run `simulate_dh()` and verify that Alice and Bob compute identical shared secrets. Try different primes and generators from RFC 7919.
2. **Medium** — Implement `simulate_mitm()` where Mallory intercepts the public keys, substitutes her own, and decrypts/ re-encrypts a message without detection.
3. **Hard** — Implement DHE: generate fresh ephemeral keypairs per session, perform the exchange, then discard the private keys. Add a `recover_session(recorded_traffic)` function that tries to break a past session — and confirm it fails because the ephemeral key was discarded.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Diffie-Hellman (DH) | Key exchange over insecure channel | Two parties agree on a shared secret without ever transmitting it |
| Discrete log | Hard math problem | Given g^x mod p, finding x is computationally infeasible for large p |
| Modular exponentiation | g^x mod p | Repeated squaring with modulus to compute large exponents efficiently |
| Generator (g) | Base element | An integer whose powers modulo p generate a large cyclic subgroup |
| Shared secret | g^(ab) mod p | The value both parties independently compute, used as symmetric key material |
| Man-in-the-Middle (MITM) | Active interception | Attacker replaces public keys, establishing separate secrets with each party |
| Forward secrecy | Past is safe | Ephemeral keys discarded after session prevent retroactive decryption |
| Ephemeral (DHE) | Fresh keys each time | New private key per session; keys are not stored |
| Safe prime | p = 2q + 1 | A prime p where (p-1)/2 is also prime, preventing certain discrete log attacks |

## Further Reading

- Diffie, W. and Hellman, M. (1976). *New Directions in Cryptography*. IEEE Transactions on Information Theory, 22(6), 644-654. The paper that invented public-key cryptography.
- RFC 2631 — Diffie-Hellman Key Agreement Method.
- RFC 7919 — Negotiated Finite Field Diffie-Hellman Ephemeral Parameters for TLS.
- Bernstein, D.J. (2006). *Curve25519: New Diffie-Hellman Speed Records*. PKC 2006.
- Boneh, D. and Shoup, V. (2023). *A Graduate Course in Applied Cryptography*. Chapters 10-11 cover DH and discrete-log security proofs.
