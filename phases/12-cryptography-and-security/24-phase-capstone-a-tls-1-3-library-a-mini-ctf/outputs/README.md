# Phase 12 Capstone Deliverables

This directory contains the two capstone deliverables for Phase 12 (Cryptography & Security).

## Deliverable 1: TLS 1.3 Library (`tls13-lib/`)

A modular Rust implementation of the TLS 1.3 core protocol.

**Source:** `../code/main.rs` + `../code/Cargo.toml`

**What it demonstrates:**
- Record layer: TLS record encoding/decoding with content type, version, and length
- Key schedule: HKDF-SHA256 chain (early secret → handshake secret → master secret → traffic keys)
- Cipher operations: AES-128-GCM AEAD encryption with sequence-number-based nonce
- Transcript hash: Rolling SHA-256 hash of all handshake messages
- Handshake: ClientHello, ServerHello, EncryptedExtensions, Certificate, CertificateVerify, Finished
- Signatures: Ed25519-based CertificateVerify
- Connection state machine: Handshake → Connected → Closed

**Build and run:**
```bash
cd tls13-lib/
cargo build --release
cargo run
```

**Dependencies:** x25519-dalek, sha2, hmac, hkdf, aes-gcm, ed25519-dalek, rand, hex

## Deliverable 2: Mini-CTF Framework (`mini-ctf/`)

A Python capture-the-flag framework with 5 cryptography/security challenges.

**Source:** `../code/main.py`

**Challenges:**

| # | Name | Vulnerability | Concept |
|---|------|---------------|---------|
| 1 | ECB Byte-at-a-Time | Deterministic ECB encryption allows plaintext recovery by comparing ciphertext blocks | AES block cipher modes |
| 2 | Nonce Reuse | Identical keystream from reused CTR nonce reveals plaintext via XOR | CTR mode, stream ciphers |
| 3 | Weak RSA | Small exponent (e=3) with no padding allows message recovery via integer cube root | RSA, padding |
| 4 | Timing Oracle | Variable-time comparison leaks password character-by-character via response timing | Side-channel attacks |
| 5 | Padding Oracle | CBC padding validation oracle enables full ciphertext decryption via block manipulation | CBC mode, padding |

**Run:**
```bash
cd mini-ctf/
python3 main.py
```

**Dependencies:** pycryptodome (`pip install pycryptodome`)

## Grading

The mini-CTF includes a built-in grader. Run all 5 challenges and check the grader report:

```
Options:
  [1-5]  Run a specific challenge
  [a]    Run all challenges
  [g]    Show grader report
  [q]    Quit
```

Each solved challenge is marked PASS. Score 5/5 to complete the capstone.
