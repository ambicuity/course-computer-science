# Toy TLS 1.3 Client

**Author:** Phase 12 — Cryptography & Security, Lesson 15

## What It Is

A working toy TLS 1.3 client that performs a full handshake over TCP with a local loopback server. Demonstrates every core TLS 1.3 cryptographic concept:

- X25519 ECDHE key exchange with ephemeral keypairs
- TLS 1.3 key schedule via HKDF-Extract/Expand (RFC 8446 §7.1) — 7 derived secrets
- AES-128-GCM record protection with sequence-number nonces
- Ed25519 CertificateVerify sign-and-verify over transcript hash
- Mutual Finished message verification (server and client)
- Encrypted application data exchange (HTTP request/response)

## How to Run

```bash
cd code
cargo run
```

The binary starts a server thread, then creates a client that connects, performs the full TLS 1.3 handshake, sends encrypted application data, and verifies the echo response. Every step prints diagnostic output with verification status.

## What It Demonstrates

The output shows each handshake step in sequence:

1. **ClientHello construction** — the binary ClientHello with extensions
2. **ServerHello response** — the server's key_share
3. **ECDHE shared secret** — both sides compute matching g^{ab}
4. **Key schedule** — early → handshake → master → traffic secrets with hex prefixes
5. **Encrypted handshake** — CertificateVerify signature + Finished verify_data
6. **Application data** — plaintext request is encrypted on the wire, decrypted by the peer
7. **Echo verification** — client sends data, server echoes it back encrypted, client verifies match

## Limitations

- Self-signed certificate pinned by public key (no CA chain validation)
- Single cipher suite (TLS_AES_128_GCM_SHA256 only)
- No session resumption or PSK
- No 0-RTT
- No key update
- No middlebox compatibility (no change_cipher_spec dummy record)
- Single-threaded server (one connection at a time)

## Where This Appears Later

The phase capstone (Phase 12, Lesson 24 — "A TLS 1.3 Library & A Mini-CTF") extends this client into a full TLS 1.3 library with a server implementation, session resumption, and a CTF toolkit where students exploit weaknesses in toy TLS implementations. The record layer, key schedule, and handshake state machine from this lesson are the foundation.
