# EC Point Operations & Ed25519 Key Generation Library

**Author:** Phase 12 — Cryptography & Security, Lesson 11

## What It Is

A reusable elliptic curve library implementing:

- `EllipticCurve` / `Point` types with point addition, doubling, scalar multiplication
- Double-and-add scalar multiplication
- Point negation and identity handling
- Brute-force order finding (for small curves)
- ECDH-style key exchange demonstration

Available in **Python** (`code/main.py`) and **Rust** (`code/main.rs`).

## How to Run

### Python

```bash
python3 code/main.py
```

### Rust

```bash
cd code && rustc main.rs && ./main
```

## Where This Appears Later

The TLS 1.3 capstone (Phase 14+) needs both X25519 key exchange and Ed25519 signatures. The point operations implemented here are the mathematical foundation — in the capstone, you replace the double-and-add with a proper Montgomery ladder and use the actual Ed25519 curve parameters.

## Limitations

- Uses `i64` arithmetic — only works for small fields (`p < 2^31`). Production implementations use big-integer or limb-based arithmetic.
- Brute-force point counting and order finding — only practical for toy curves.
- No constant-time guarantees (the double-and-add has a data-dependent branch on the scalar bits).
