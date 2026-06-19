# Public Key II — RSA Internals & Padding

> Public Key II — RSA Internals & Padding — the part of CS you can't skip.

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 12 lessons 01–09
**Time:** ~90 minutes

## Learning Objectives

- Generate RSA keypairs: select large primes p, q, compute modulus n = pq, Euler's totient φ(n) = (p-1)(q-1), and find e, d such that e·d ≡ 1 (mod φ(n)).
- Encrypt and decrypt using the textbook RSA operation: c = m^e mod n, m = c^d mod n.
- Explain why textbook RSA is insecure (deterministic, malleable, no semantic security).
- Implement OAEP padding and understand why PKCS#1 v1.5 padding is vulnerable to Bleichenbacher's padding oracle attack.
- Compare RSA with modern alternatives (ECDSA, EdDSA, ECDH) and understand RSA's declining role.

## The Problem

Diffie-Hellman solves key agreement, but it does not provide asymmetric encryption: you cannot send an encrypted message to someone without them participating in a protocol. What if you want to encrypt a file for a recipient who is offline? What if you need to sign a document to prove authorship?

RSA, invented in 1977 by Rivest, Shamir, and Adleman, was the first practical solution for both asymmetric encryption and digital signatures.

## The Concept

### Key Generation

1. Pick two large distinct primes **p** and **q** (each at least 1024 bits for security).
2. Compute modulus **n = p × q** (2048+ bits).
3. Compute Euler's totient **φ(n) = (p-1)(q-1)**.
4. Choose public exponent **e** (commonly 65537 = 2^16 + 1).
5. Compute private exponent **d ≡ e^(-1) (mod φ(n))** using the extended Euclidean algorithm.
6. **Public key:** (n, e). **Private key:** (n, d).

### Encryption and Decryption

- **Encryption:** c = m^e mod n (m is the plaintext as an integer, 0 ≤ m < n).
- **Decryption:** m = c^d mod n.

### Why It Works (Euler's Theorem)

For any integer m coprime to n: m^φ(n) ≡ 1 (mod n). Since e·d = 1 + k·φ(n), we have:

c^d = (m^e)^d = m^(e·d) = m^(1 + k·φ(n)) = m · (m^φ(n))^k ≡ m · 1^k = m (mod n).

### Security Foundation: The Factoring Problem

If an attacker could factor n into p and q, they could compute φ(n) and derive d from e. The security of RSA depends on the computational difficulty of factoring large composite numbers. The best known classical algorithm (General Number Field Sieve) is subexponential but still infeasible for 2048+ bit moduli.

### Textbook RSA Is Insecure

Textbook RSA (raw m^e mod n without padding) has fatal flaws:

1. **Deterministic:** Same plaintext always produces the same ciphertext — attacker can recognize repeats.
2. **Malleable:** Given c = m^e mod n, attacker can construct c' = (2m)^e mod n, which decrypts to 2m.
3. **Small message attack:** If m^e < n, the ciphertext is just m^e over the integers — trivial to take the e-th root.
4. **No semantic security:** Attacker can test guesses (does c encrypt "yes" or "no"?).

### Padding: PKCS#1 v1.5 and OAEP

**PKCS#1 v1.5** (RFC 2313, 1998): Pad message to k bytes as `0x00 || 0x02 || random_nonzero_bytes || 0x00 || message`. Vulnerable to **Bleichenbacher's padding oracle attack** (1998): if the server reveals whether padding is valid, the attacker can decrypt any ciphertext in ~2^20 queries.

**OAEP** (Optimal Asymmetric Encryption Padding, Bellare/Rogaway 1994): Provably secure (in the random oracle model). Uses a Feistel network with two hash functions to produce a padded message that is non-malleable. The padding ensures that any modification to the ciphertext randomizes the decrypted plaintext — preventing chosen-ciphertext attacks.

### Attacks on RSA

- **Bleichenbacher's attack** (PKCS#1 v1.5 padding oracle): ~2^20 chosen-ciphertext queries.
- **Manger's attack** (OAEP timing oracle): ~2^20 queries if implementation leaks whether a decrypted value is ≤ n - 2^(k-1).
- **Common modulus attack:** If the same n is shared with different (e1, e2) where gcd(e1, e2) = 1, an attacker can decrypt without factoring.
- **Small e attack (Coppersmith):** If the same message is encrypted with small e to multiple recipients, CRT recovers the message.
- **Timing attacks:** The time to compute m^d mod n can leak d bit by bit (Kocher's attack). Countermeasure: **blinding** — multiply ciphertext by r^e before decryption, then divide by r.

## Build It

### Python

```python
import random, hashlib, os

def is_prime(n: int, k: int = 10) -> bool:
    if n < 2: return False
    if n < 4: return True
    if n % 2 == 0: return False
    r, d = 0, n - 1
    while d % 2 == 0: r += 1; d //= 2
    for _ in range(k):
        a = random.randrange(2, n - 2)
        x = pow(a, d, n)
        if x == 1 or x == n - 1: continue
        for _ in range(r - 1):
            x = pow(x, 2, n)
            if x == n - 1: break
        else: return False
    return True

def generate_prime(bits: int) -> int:
    while True:
        n = int.from_bytes(os.urandom(bits // 8), "big") | (1 << (bits - 1)) | 1
        if is_prime(n): return n

def generate_keypair(bits: int = 512):
    p, q = generate_prime(bits // 2), generate_prime(bits // 2)
    n = p * q
    phi = (p - 1) * (q - 1)
    e = 65537
    d = pow(e, -1, phi)  # Python 3.8+ extended gcd
    return (n, e), (n, d)
```

### OAEP Padding

OAEP encodes the message using two hash functions (or one with domain separation). The padded message is k bytes (the RSA modulus size):

1. **Length check:** Message must be ≤ k - 2·hLen - 2 bytes.
2. **Data block:** `DB = Hash(L) || 0x00...0x00 || 0x01 || M`.
3. **Masked DB:** `maskedDB = DB ⊕ MGF1(seed, k - hLen - 1)`.
4. **Masked seed:** `maskedSeed = seed ⊕ MGF1(maskedDB, hLen)`.
5. **Encoded:** `EM = 0x00 || maskedSeed || maskedDB`.

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

### Miller-Rabin and Extended Euclidean

The Miller-Rabin test is probabilistic: with k = 10 iterations, the probability of falsely identifying a composite as prime is < 4^(-10) ≈ 10^(-6). For cryptographic use, k = 40-64 is standard. The extended Euclidean algorithm (`xgcd`) returns (g, x, y) such that a·x + b·y = g = gcd(a, b). When a and b are coprime, x is the modular inverse of a modulo b.

## Use It

RSA was once ubiquitous but is increasingly replaced by elliptic-curve algorithms:

- **TLS 1.2** supports RSA key exchange and RSA signatures. **TLS 1.3** removed RSA key transport entirely — only (EC)DHE key agreement with (EC)DSA or EdDSA signatures is allowed.
- **PGP/GPG** still uses RSA for both encryption and signing by default.
- **SSH** supports RSA (legacy) but prefers Ed25519.
- **Signatures:** ECDSA and EdDSA produce smaller signatures (64 bytes for Ed25519 vs 256 bytes for 2048-bit RSA) and are faster to verify.

## Read the Source

- **OpenSSL `crypto/rsa/rsa_ossl.c`**: The core RSA implementation — see `rsa_ossl_public_encrypt`, `rsa_ossl_private_decrypt`, and the blinding logic in `rsa_blinding_invert`.
- **Rust `rsa` crate (`src/` on crates.io)**: Clean Rust implementation with OAEP, PSS, and PKCS#1 v1.5 support.
- **RFC 8017**: PKCS#1 v2.2 — the definitive specification for RSA encryption (OAEP) and signatures (PSS).

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **An RSA encryption/decryption library with OAEP padding**, usable for educational projects and as a reference implementation.

## Exercises

1. **Easy** — Generate a keypair, encrypt and decrypt a message end-to-end. Verify that modified ciphertexts fail to decrypt or produce garbage.
2. **Medium** — Implement Bleichenbacher's padding oracle attack simulation: create an oracle that validates PKCS#1 v1.5 padding, then recover a ciphertext's plaintext using chosen-ciphertext queries.
3. **Hard** — Implement CRT optimization for RSA decryption: compute m_p = c^(d mod (p-1)) mod p and m_q = c^(d mod (q-1)) mod q, then combine using Garner's formula.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| RSA | Rivest-Shamir-Adleman | First practical asymmetric cryptosystem; security from factoring |
| Modulus (n) | n = p × q | The public composite — its factorization is the secret |
| Euler's totient (φ) | φ(n) = (p-1)(q-1) | Count of integers 1..n coprime to n; used to find d |
| PKCS#1 | RSA padding standard | Family of encoding methods for RSA encryption and signatures |
| OAEP | Optimal Asymmetric Encryption Padding | Provably secure Feistel-network padding for RSA encryption |
| Blinding | r^e · c mod n | Multiplicative masking to prevent timing attacks on decryption |
| CRT | Chinese Remainder Theorem | Faster RSA decryption by working mod p and mod q separately |
| Factoring | Breaking n into p, q | The hard problem underlying RSA security |
| Padding oracle | Side channel that leaks padding validity | Enables Bleichenbacher's attack on PKCS#1 v1.5 |
| Carmichael function (λ) | lcm(p-1, q-1) | Alternative to φ; e·d ≡ 1 (mod λ) is sufficient for RSA |

## Further Reading

- Rivest, R., Shamir, A., and Adleman, L. (1978). *A Method for Obtaining Digital Signatures and Public-Key Cryptosystems*. Communications of the ACM, 21(2), 120-126. The original RSA paper.
- Bellare, M. and Rogaway, P. (1994). *Optimal Asymmetric Encryption Padding*. Eurocrypt 1994. The paper that introduced OAEP with a security proof.
- Bleichenbacher, D. (1998). *Chosen Ciphertext Attacks Against Protocols Based on the RSA Encryption Standard PKCS#1*. Crypto 1998. The classic padding oracle attack.
- RFC 8017 — PKCS#1: RSA Cryptography Specifications v2.2.
- Manger, J. (2001). *A Chosen Ciphertext Attack on RSA Optimal Asymmetric Encryption Padding (OAEP) as Standardized in PKCS#1 v2.0*. Crypto 2001. Timing attack on OAEP.
