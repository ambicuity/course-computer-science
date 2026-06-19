# Certificate Inspection & Validation Toolkit

**Phase 12 — Cryptography & Security, Lesson 16: PKI, Certs, Transparency**

## What It Is

A Python toolkit for working with the Web PKI:

- **Certificate Parser** (`parse_certificate`) — Loads a PEM certificate, extracts every field including subject, issuer, validity, public key info, SANs, key usage, extended key usage, basic constraints, CRL distribution points, AIA, and embedded SCTs. Prints a formatted summary.
- **Chain Validator** (`build_chain_from_server`, `verify_cert_signature`) — Connects to a TLS server, fetches the peer certificate chain, verifies each link's signature (RSA and ECDSA), checks Basic Constraints and validity dates.
- **Merkle Tree** (`MerkleTree`, `verify_inclusion`) — An append-only binary Merkle tree as used in Certificate Transparency (RFC 6962). Supports leaf addition, inclusion proofs, and verification.

## How to Run

```bash
pip install cryptography certifi requests
python3 code/main.py
```

The program runs three demos:
1. Fetches and parses `google.com`'s TLS certificate
2. Fetches and validates `google.com`'s certificate chain
3. Demonstrates CT-style Merkle tree operations (inclusion proof, tamper detection, append-only property)

## Where This Appears Later

The phase capstone (A TLS 1.3 Implementation + Mini-CTF) requires certificate chain validation during the handshake. The parsing and verification logic here can be adapted for that purpose. The Merkle tree implementation also serves as a reference for understanding CT log auditing, which appears in the capstone's security analysis.

## Limitations

- Chain validation fetches the server-provided chain but does not verify against the OS trust store (no root store pinning).
- The Merkle tree is built from simulated certificate entries, not real CT log data.
- SCT parsing reads the embedded extension but does not verify the SCT signature against the log's public key.
- Network-dependent: requires internet access to fetch live certificates.
