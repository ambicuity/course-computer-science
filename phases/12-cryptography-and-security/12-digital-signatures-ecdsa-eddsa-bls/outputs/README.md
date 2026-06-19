# Digital Signature Demo — ECDSA / Ed25519 / BLS

**Author:** Phase 12 — Cryptography & Security, Lesson 12

## What It Is

A self-contained Rust demonstration of three digital signature schemes:

- **ECDSA on a toy curve** (\(y^2 = x^3 + 2x + 3 \bmod 97\), \(G = (3,6)\), order 5). Full implementation of keygen, signing, and verification from scratch, plus the **nonce-reuse attack** that recovers the private key when the same \(k\) is used twice.
- **Ed25519** via the `ed25519-dalek` crate. Demonstrates deterministic signing (no RNG needed), tamper detection, and the invariant that the same key+message always yields the same signature.
- **BLS signatures** via the `blst` crate (BLS12-381). Demonstrates multi-key aggregation: 3 signatures (288 bytes) compress to 1 (96 bytes), with full verification.

## How to Run

```bash
# Create a new cargo project
cargo new sig-demo && cd sig-demo

# Add dependencies
cargo add sha2 ed25519-dalek rand blst

# Copy the source
cp ../path/to/code/main.rs src/main.rs

# Run
cargo run
```

Expected output: prints key generation, signing, verification, the nonce-reuse exploit, Ed25519 deterministic signing, BLS aggregation, and a comparison table. Ends with "All tests passed!".

## Where This Appears Later

- **TLS 1.3 capstone (Phase 14+):** The TLS handshake uses signature-based authentication (CertificateVerify messages). ECDSA and Ed25519 are the two most common signature algorithms in TLS 1.3.
- **Mini-CTF (Phase 14+):** The ECDSA nonce-reuse challenge is a direct port of Step 1 — students must write the recovery algorithm to extract a leaked private key.
- **Blockchain module (Phase 16):** BLS aggregation appears in the consensus layer section; this demo provides the intuition for why Ethereum 2.0 chose BLS over Ed25519.

## Limitations

- The toy curve ECDSA uses `i64` arithmetic and a group of order 5 — educational only, not secure.
- The Ed25519 and BLS demos use production crates; they are secure but not optimized for any specific use case.
- BLS aggregation in this demo is for the *same message*. Multi-message aggregation requires proof-of-possession (PoP) to prevent rogue-key attacks; see the IETF BLS standard for details.

## Example Output (abridged)

```
ECDSA Signing
  Message: "ECDSA needs a random nonce"
  Signature (r=1, s=2)
  ✓ Signature verified

Nonce Reuse Attack
  Same r = 1 — nonce was reused!
  Recovered d = 3 (original was 3)
  ✓ Private key stolen!

Ed25519
  ✓ Deterministic: same key + message = identical signature

BLS Aggregation
  Individual signature sizes: 96 bytes each
  Total: 288 bytes → 1 signature = 96 bytes
  ✓ Aggregated signature verified
```
