# TLS in the Network Course (Handshake Overview)

> Encrypt the wire, authenticate the server — in one round trip.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 09 lessons 01–13 (especially TCP and HTTP), Phase 12 lessons 01–05 (symmetric crypto, AES, Diffie-Hellman)
**Time:** ~60 minutes

## Learning Objectives

- Understand the three security guarantees TLS provides: confidentiality, integrity, authentication.
- Trace the TLS 1.3 handshake: ClientHello, ServerHello, key derivation, certificate verification, Finished.
- Explain 0-RTT resumption and its replay attack risk.
- Compare TLS 1.3 with earlier TLS 1.2 in terms of round trips and cipher suite design.

## The Problem

Without TLS, all network data travels in plaintext. Any router, ISP, or Wi-Fi operator can read passwords, session tokens, and private content. Applications need a way to establish a secure channel over an insecure network — and they need it to work with the first data they send, not after a multi-second setup.

TLS solves this: it sits between TCP and the application layer, providing encryption, tamper detection, and server identity verification. HTTP over TLS is HTTPS — the S means the connection is protected by TLS before any HTTP request is sent.

## The Concept

### TLS 1.3 Handshake (1-RTT)

TLS 1.3 simplified the handshake to a single round trip (1-RTT), compared to 2-RTT in TLS 1.2.

**Step 1: ClientHello**

The client sends:
- **supported_versions**: TLS 1.3
- **cipher_suites**: list of supported cipher suites (e.g., TLS_AES_256_GCM_SHA384)
- **key_share**: client's ephemeral public key for key exchange (e.g., X25519)
- **random**: 32-byte random nonce
- **extensions**: SNI (server name), ALPN (application protocol negotiation), etc.

**Step 2: ServerHello + Encrypted Extensions + Certificate + Finished**

The server responds with:
- **server_hello**: chosen cipher suite + server's key share
- From this point, both sides derive encryption keys using HKDF
- **encrypted_extensions**: additional negotiation data (encrypted)
- **certificate**: server's X.509 certificate chain
- **certificate_verify**: signature proving possession of the certificate's private key
- **finished**: MAC of the entire handshake transcript

**Step 3: Client Finished**

The client verifies the certificate chain, checks the finished message, and sends its own finished. Application data can now flow in both directions.

Total: 1 round trip before data exchange.

### 0-RTT Resumption

If the client has a pre-shared key (PSK) from a previous session (stored as a session ticket), it can send encrypted data in the very first message (0-RTT). This comes with **replay attack risk** — 0-RTT data can be replayed by an attacker. Therefore, only idempotent requests (GET) should use 0-RTT.

### Certificate Chain and Verification

A server's certificate is signed by a Certificate Authority (CA). The chain works like this:

```
Server Certificate (example.com)
  → signed by Intermediate CA
    → signed by Root CA (trusted by your OS/browser)
```

Verification steps:
1. **Signature check**: verify each certificate's signature using its issuer's public key
2. **Validity period**: ensure current date falls within `notBefore` and `notAfter`
3. **Hostname match**: server name must match the certificate's CN or SAN fields
4. **Revocation**: check CRL or OCSP that the certificate has not been revoked
5. **Trust anchor**: the chain must terminate at a root CA in the trust store

### Cipher Suites

A TLS 1.3 cipher suite defines:
- **Encryption**: AES-256-GCM or ChaCha20-Poly1305 (authenticated encryption)
- **Hash**: SHA-256 or SHA-384 for key derivation and MAC

TLS 1.3 cipher suite format: `TLS_{ENC}_{HASH}`
Examples: `TLS_AES_128_GCM_SHA256`, `TLS_AES_256_GCM_SHA384`, `TLS_CHACHA20_POLY1305_SHA256`

Key exchange (X25519, P-256) and authentication (RSA, ECDSA) are negotiated separately via extensions, not baked into the cipher suite.

### Key Derivation (HKDF)

TLS 1.3 uses HKDF (HMAC-based Key Derivation Function) to derive encryption keys from the shared secret:

```
Early Secret = HKDF-Extract(PSK or 0)
Handshake Secret = HKDF-Extract(DHE(shared_secret))
Master Secret = HKDF-Extract(0)

Client Handshake Key = HKDF-Expand(Handshake Secret, "c hs traffic", length)
Server Handshake Key = HKDF-Expand(Handshake Secret, "s hs traffic", length)
Client App Key = HKDF-Expand(Master Secret, "c ap traffic", length)
Server App Key = HKDF-Expand(Master Secret, "s ap traffic", length)
```

### Session Resumption

- **Session Tickets**: the server encrypts session state and sends it to the client, who presents it on reconnect
- **Pre-Shared Keys (PSK)**: derived from previous sessions, enabling 0-RTT
- **Connection ID**: allows resumption across network changes (mobile devices)

## Build It

The code in `code/main.py` implements a TLS 1.3 handshake simulator. It is not a real TLS implementation — it uses plain Python integers for the math — but it faithfully models the handshake's structure and key derivation.

**Step 1: Handshake State Machine**

The `TLSHandshake` class models the client and server roles. The `client_hello()` method creates a ClientHello message with supported cipher suites and an ephemeral X25519 public key. The `server_hello()` method selects a cipher suite, generates the server's key share, and sends back the ServerHello.

**Step 2: Key Derivation**

The `derive_handshake_keys()` method implements the two-stage HKDF-Extract/HKDF-Expand pattern. First it extracts an intermediate secret from the ECDHE shared secret, then it expands that secret into separate client and server handshake traffic keys using the labeled strings from RFC 8446.

**Step 3: Certificate Chain**

The `CertificateAuthority` class generates a root CA certificate, signs an intermediate CA, which then signs a server certificate. The `verify_certificate_chain()` method walks the chain from leaf to root, checking each signature with the next certificate's public key, and confirming the chain terminates at a trusted root.

**Step 4: Verification**

The `verify_certificate()` method checks five things: the cryptographic signature, the certificate validity window, that the hostname matches the CN or SAN, that the certificate has not been revoked (simulated OCSP check), and that the issuer is trusted.

The simulator can run a full handshake in about 50 lines of calls. Run it to see the complete flow printed step by step.

## Use It

Every HTTPS connection uses TLS. When you visit `https://example.com`, your browser performs a TLS handshake with the server, verifies its certificate, derives symmetric keys, and then sends HTTP requests encrypted with AES-GCM. The same handshake runs inside your SSH client (for tunneling), your VPN, and your database driver.

## Read the Source

- **RFC 8446** — The TLS 1.3 specification. Sections 2 (protocol overview), 4 (handshake protocol), and 7 (key schedule) are essential reading.
- **OpenSSL `ssl/` directory** — `ssl_lib.c`, `tls13_enc.c`, `statem_clnt.c` — the production implementation of the handshake state machine and key schedule.
- **Rustls `rustls/src/tls13/`** — A pure-Rust TLS 1.3 implementation; `key_schedule.rs` implements the same HKDF derivation, `handshake.rs` drives the state machine.

## Ship It

The reusable artifact is the TLS 1.3 handshake simulator in `code/main.py`. It can be used to trace a complete TLS handshake, inspect the key schedule outputs at each stage, and verify certificate chains — all without real cryptographic operations. Add it to your debugging toolkit for understanding TLS errors.

## Exercises

### Level 1 — Recall

1. What are the three security properties TLS provides?
2. How many round trips does a TLS 1.3 handshake require?
3. What is the purpose of the `finished` message?

### Level 2 — Application

4. Trace the key derivation steps from ECDHE shared secret to application traffic keys.
5. Explain why 0-RTT data is vulnerable to replay attacks and give an example of when it is safe to use.
6. Given a certificate chain with 3 certificates, describe the verification steps.

### Level 3 — Creation

7. Implement a simplified HKDF function that derives a client key and server key from a shared secret.
8. Design a session resumption mechanism: define what the server stores, what the client presents, and how 0-RTT keys are derived.
9. Build a certificate validity checker that validates notBefore, notAfter, and hostname matching.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| TLS | "Transport Layer Security" | Cryptographic protocol that provides encryption, authentication, and integrity over TCP |
| 1-RTT | "One round trip" | The TLS 1.3 handshake completes in a single network round trip before data flows |
| 0-RTT | "Zero round trip" | Early data sent with the first ClientHello using a PSK from a prior session; vulnerable to replay |
| Cipher suite | "The encryption algorithm" | In TLS 1.3, defines only the AEAD cipher and HKDF hash; key exchange and auth are separate |
| HKDF | "HMAC-based KDF" | Two-stage key derivation (extract then expand) used to derive all TLS 1.3 traffic keys |
| PSK | "Pre-shared key" | A key established during a prior session, enabling session resumption and 0-RTT data |
| Certificate chain | "Server sends its cert" | An ordered list of X.509 certificates from leaf to root, each signing the next |
| SNI | "Server Name Indication" | TLS extension that lets the client announce which hostname it wants, enabling virtual hosting |
| AEAD | "Authenticated encryption" | Encryption mode (AES-GCM or ChaCha20-Poly1305) that provides both confidentiality and integrity |
| Forward secrecy | "Ephemeral keys" | Compromise of the long-term key does not reveal past session keys, because session keys are derived from ephemeral ECDHE shares |

## Further Reading

- RFC 8446 — The TLS 1.3 Protocol (Mandatory reading; the spec is remarkably readable)
- "The Design and Implementation of TLS 1.3" by Christopher Patton and Brian Smith — A deep-dive into the protocol's evolution from TLS 1.2
- OpenSSL source: `ssl/statem/statem_clnt.c` — The client-side handshake state machine
- Rustls source: `rustls/src/tls13/key_schedule.rs` — A clean, minimal TLS 1.3 key schedule implementation
- "Bulletproof TLS and PKI" by Ivan Ristić — Practical TLS deployment and debugging guide