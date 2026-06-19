# KDF Library — PBKDF2, scrypt & Argon2id

## What It Is

A self-contained demonstration of three key derivation functions (KDFs) for
password hashing, with implementations spanning Rust and Python:

- **PBKDF2-HMAC-SHA256** — implemented from scratch in Rust. Shows the core
  iteration loop, HMAC construction, and RFC 6070 test vector verification.
- **scrypt** — implemented from scratch in Rust with a full Salsa20/8 core,
  ROMix memory-hard mixing, and RFC 7914 test vector verification.
- **Argon2id** — parameter exploration in Python using `argon2-cffi`. Shows
  timing and parameter trade-offs for interactive vs. sensitive use cases.

## How to Run

### Rust (PBKDF2 + scrypt from scratch)

```bash
cd code
cargo run --release
```

Requires: Rust toolchain (rustc + cargo). Dependencies: `sha2`, `hex`.

### Python (Argon2id + benchmarks)

```bash
pip install argon2-cffi
python3 code/main.py
```

## Parameter Recommendations

| Use Case | Algorithm | Parameters |
|----------|-----------|------------|
| Interactive login | Argon2id | t=2, m=32 MiB, p=2 |
| OWASP baseline | Argon2id | t=3, m=64 MiB, p=4 |
| Sensitive / password manager | Argon2id | t=4, m=128 MiB, p=4 |
| Legacy / embedded (no Argon2) | scrypt | N=2^14, r=8, p=1 |
| Legacy / embedded (no scrypt) | PBKDF2 | 600,000 iterations |

## Where It's Reused

This KDF artifact is used in the TLS 1.3 capstone (Phase 12, Lesson 24-25)
when deriving a pre-shared key (PSK) from a shared secret during session
resumption. The PBKDF2 implementation also serves as a reference for
password-to-key derivation in the mini-CTF's password-cracking challenge.

## Files

- `code/main.rs` — PBKDF2 and scrypt from scratch implementation
- `code/main.py` — Argon2id demo, password verification, and KDF benchmarks
- `code/Cargo.toml` — Rust dependencies
