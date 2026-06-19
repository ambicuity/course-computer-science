# Zero-Knowledge Proof Library

This directory contains the reusable artifact for **Lesson 12.17 — Zero-Knowledge Proofs: Sigma, zk-SNARK overview**.

## What It Is

A self-contained zero-knowledge proof library implementing:

1. **Schnorr Sigma protocol** (interactive) — proves knowledge of a discrete log in 3 moves: commitment, challenge, response.
2. **Non-interactive Schnorr proof** (Fiat-Shamir) — same proof but publicly verifiable; the verifier's challenge is replaced by SHA-256 hash of the commitment.
3. **R1CS constraint system demo** — shows how a computation (`x^2 + 3x + 1 == 11` or `x^3 + x + 5 == 35`) gets flattened into rank-1 constraints `<A,w>·<B,w> = <C,w>`.
4. **Pedersen commitment** (Python) — hiding and binding commitment scheme using two generators.

## How to Run

### Python

```bash
cd code
python3 main.py
```

Requires: Python 3.8+, standard library only (hashlib, random).

Output: Full demonstration of interactive Schnorr, non-interactive Schnorr, invalid proof detection, Pedersen commitments, and R1CS constraint system.

### Rust

```bash
cd code
cargo run
```

Requires: Rust 1.65+, Cargo.

Dependencies: `sha2`, `rand`, `num-bigint`, `num-traits`, `hex`.

Output: Same demonstrations as Python, plus performance benchmarks (100 proofs timed).

## Proof Sizes (Rust)

For the 1024-bit safe prime used in this demonstration:
- Interactive proof: 2 messages (commitment t: 128 bytes, response s: 128 bytes) plus challenge (128 bytes) = 384 bytes total
- Non-interactive proof (Fiat-Shamir): t + s = 256 bytes
- Groth16 SNARK comparison: ~200 bytes (constant, independent of computation size)

The Fiat-Shamir transform eliminates the verifier's challenge message, since it's replaced by the hash. This is the same technique real SNARKs use to achieve non-interactive verification.

## Connection to Phase Capstone

The phase capstone (Lesson 12.24) implements a TLS 1.3 library and mini-CTF. While TLS 1.3 doesn't use zero-knowledge proofs directly, the ZK mindset — separating proof from revelation — applies to:

- **Anonymous credentials** for TLS client authentication (post-quantum TLS with ZK)
- **Private certificate verification** (prove a cert is valid without revealing which cert)
- **Side-channel-resistant protocols** where the prover and verifier roles are carefully separated

The Schnorr protocol also appears in Ed25519 signatures (Lesson 12.12) and BLS signatures, where the same "commit-challenge-respond" pattern underlies the signature scheme.
