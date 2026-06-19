# Post-Quantum Cryptography Demo

**Author:** Phase 12 — Cryptography & Security, Lesson 18

## What It Is

A Rust demonstration program exercising all three NIST-standardized post-quantum cryptographic families:

- **Kyber (ML-KEM)** — Module LWE key encapsulation (Kyber-512, Kyber-1024)
- **Dilithium (ML-DSA)** — Module LWE digital signatures (Dilithium2, Dilithium5)
- **SPHINCS+ (SLH-DSA)** — Stateless hash-based signatures (SHAKE-128s, SHA2-256s)

Each demo runs key generation, cryptographic operations, verification, and correctness assertions. A size comparison table contrasts PQC schemes with classical ones (Ed25519, RSA-3072, X25519).

## How to Run

```bash
cd code
cargo run --release
```

Requires Rust 2021 edition and a C compiler (for PQClean C bindings).

## What It Demonstrates

- Kyber key encapsulation: keygen → encapsulate → decapsulate → shared secret match
- Dilithium signatures: keygen → sign → verify → tampered message rejection
- SPHINCS+ signatures: keygen → sign → verify → timing comparison (sign vs verify)
- Size comparisons across all schemes and classical equivalents

## Limitations

- The `pqcrypto-*` crates wrap C implementations from PQClean — build times are long due to C compilation.
- Timing measurements include Go runtime overhead and are not suitable for security-critical benchmarking.
- Only "clean" (portable C) implementations are used; AVX2-optimized versions exist on supported platforms.
- The demo does not implement hybrid key exchange or protocol integration — only raw cryptographic primitives.

## Where This Appears Later

The Phase 12 TLS 1.3 capstone can be extended to support hybrid X25519Kyber768 key exchange. The SPHINCS+ and Dilithium code demonstrates signature schemes that could replace Ed25519 in the capstone's certificate chain.
