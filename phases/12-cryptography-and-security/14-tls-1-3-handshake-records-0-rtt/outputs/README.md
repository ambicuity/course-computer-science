# TLS 1.3 Core Library — Record Layer, Key Schedule, Handshake

**Phase:** 12 — Cryptography & Security, Lesson 14

## What It Is

A modular TLS 1.3 implementation in Rust demonstrating the three foundational layers of the protocol:

### `TLSRecord` — Record Layer (Step 1)
Encodes and decodes TLS records with the standard wire format: content type byte (22=handshake, 23=application data, 21=alert), 2-byte protocol version, 2-byte length, and variable-length payload. Supports round-trip serialization verification.

### `RecordProtection` — AEAD Encryption (Step 1)
Encrypts and decrypts TLS records using AES-128-GCM. The 12-byte nonce is constructed by XORing the 64-bit per-direction sequence number into the derived IV, ensuring a unique nonce per record without transmission overhead. The additional authenticated data (AAD) binds each ciphertext to its content type, preventing record-type substitution attacks.

### `KeySchedule` — HKDF Key Derivation (Step 2)
Implements the full TLS 1.3 key schedule per RFC 8446 §7.1. The seven-secret cascade:
- `early_secret` — derived from PSK (zero-padded for full handshake)
- `handshake_secret` — derived by mixing the ECDHE shared secret into the early secret
- `master_secret` — derived from the handshake secret with zero salt
- 4 traffic secrets (client/server × handshake/application) — derived with labeled HKDF-Expand-Label, each bound to the transcript hash

Traffic keys (16-byte AES key, 12-byte IV) are derived from each traffic secret using the labels `"tls13 key"` and `"tls13 iv"`.

### Handshake Messages — ClientHello / ServerHello (Step 3)
Constructs and parses the first two handshake messages, including the X25519 key share, supported versions (0x0304), cipher suite (TLS_AES_128_GCM_SHA256), and signature algorithms extension. Transcript hashing uses an incremental SHA-256 over the concatenated handshake message encodings.

### Application Data — Full-Duplex Encryption (Step 4)
Demonstrates a complete round-trip: client encrypts an HTTP request, server decrypts it, server encrypts an HTTP response, client decrypts it. Each direction uses independent `RecordProtection` instances with separate sequence numbers.

### 0-RTT Analysis (Step 6)
Explains the latency/replay tradeoff of zero round-trip time resumption, listing the anti-replay mitigations required (freshness checks, replay cache, idempotency restrictions).

## How to Run

```bash
cd code && cargo run
```

Requires Rust 2021 edition and the following crates (auto-downloaded by Cargo):
- `x25519-dalek` 2.x — ECDHE key exchange
- `sha2` 0.10 — SHA-256 for transcript hashing
- `hmac` 0.12 — HMAC-SHA256 for HKDF
- `aes-gcm` 0.10 — AES-128-GCM for record encryption
- `rand` 0.8 — cryptographic randomness

## Where This Appears Later

This is the direct prerequisite for **Lesson 24 — Phase Capstone: TLS 1.3 Library + Mini-CTF**. The capstone wires these components into a full TLS 1.3 state machine that handles the complete handshake (EncryptedExtensions, Certificate, CertificateVerify, both Finished messages), adds session resumption via PSK tickets, and pairs the implementation with a mini-CTF where players exploit:

- **Downgrade attacks** — tricking a client into accepting TLS 1.2 by stripping extensions
- **Replay attacks** — replaying 0-RTT data against a naive server
- **Nonce reuse** — observing what happens when sequence numbers wrap or are reset
- **Key confusion** — exploiting the difference between handshake and application traffic keys

## Limitations

- Single-process simulation (both client and server roles in one binary); no actual network I/O
- No session resumption or PSK ticket handling
- No Certificate or CertificateVerify message construction (Ed25519 signing of transcript)
- No alert system or error recovery
- No key update or post-handshake messages
- No middlebox compatibility mode (change_cipher_spec dummy)
- Fixed cipher suite (TLS_AES_128_GCM_SHA256 only)
