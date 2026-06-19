# Diffie-Hellman Key Exchange — Artifact

## What This Is

A self-contained DH key exchange implementation in Python and Rust, suitable for reuse in any project needing asymmetric key agreement over an insecure channel.

## Contents

- `code/main.py` — Python implementation with `mod_exp`, `generate_keypair`, `compute_shared_secret`, MITM attack simulation, and key derivation via SHA-256.
- `code/main.rs` — Rust implementation with `mod_pow` (square-and-multiply, u128 intermediates), keypair generation, shared secret computation, and exponentiation benchmarks.

## Usage

```python
from main import generate_keypair, compute_shared_secret

p = 0xFFFFFFFFFFFFFFFFC90FDAA22168C234...  # RFC 7919 2048-bit safe prime
g = 2

alice_priv, alice_pub = generate_keypair(p, g)
bob_priv, bob_pub = generate_keypair(p, g)

shared = compute_shared_secret(bob_pub, alice_priv, p)
```

```rust
use mod_pow;

let p = 99991u64;
let g = 5u64;
let (priv_a, pub_a) = generate_keypair(p, g);
let (priv_b, pub_b) = generate_keypair(p, g);
let shared = compute_shared_secret(pub_b, priv_a, p);
```

## Security Notes

- The Rust implementation uses small primes (≤ 17 bits) for educational demonstration only — real DH requires primes of at least 2048 bits.
- The "PRNG" in the Rust code is a simple LCG for portability; never use this in production.
- The Python implementation uses `os.urandom` for private key generation but lacks timing-attack countermeasures.

## Exercises

See `docs/en.md` for Easy (verify shared secret), Medium (MITM simulation), and Hard (DHE with ephemeral keys) exercises.
