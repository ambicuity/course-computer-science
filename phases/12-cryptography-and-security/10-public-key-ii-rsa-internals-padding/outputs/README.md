# RSA Internals & Padding — Artifact

## What This Is

A working RSA implementation demonstrating key generation (Miller-Rabin primality testing), textbook encryption/decryption, OAEP padding, and known weaknesses (determinism, malleability).

## Contents

- `code/main.py` — Full implementation: Miller-Rabin primality test, prime generation, RSA key generation, textbook RSA encryption/decryption, OAEP padding with MGF1, RSA-OAEP encryption, and tamper detection.
- `code/main.rs` — Rust implementation with mod_pow (square-and-multiply), deterministic Miller-Rabin for u64, extended Euclidean algorithm, RSA key generation with small primes, and Euler's theorem verification.

## Usage

```python
from main import generate_keypair, encrypt_oaep, decrypt_oaep

pub_key, priv_key = generate_keypair(bits=512)
ct = encrypt_oaep(pub_key, b"secret message")
pt = decrypt_oaep(priv_key, ct)
```

```rust
let (n, e, d, p, q) = generate_keypair(10, &mut seed);
let ct = encrypt(message, e, n);
let pt = decrypt(ct, d, n);
```

## Security Notes

- The Rust implementation uses 64-bit integers (trivially factorable). Real RSA requires at least 2048-bit moduli.
- The Python LCG in the Rust code is not cryptographically secure — it is used only for portability and reproducibility.
- OAEP implementation in Python uses SHA-256 for both the hash function and MGF1, following PKCS#1 v2.2.

## Exercises

See `docs/en.md` for Easy (end-to-end verification), Medium (Bleichenbacher's attack simulation), and Hard (CRT optimization) exercises.
