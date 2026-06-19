# What Cryptography Actually Promises

> Cryptography is math. Security is engineering. Know what the math gives you — and where only engineering can save you.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 11
**Time:** ~45 minutes

## Learning Objectives

- Name the four properties cryptography promises (confidentiality, integrity, authenticity, non-repudiation) and explain what each means with a concrete scenario.
- List what cryptography does **not** promise (availability, social-engineering resistance, implementation correctness, key-management safety) and explain why each gap matters.
- State Kerckhoffs' principle and argue why security through obscurity fails.
- Classify the five primitive categories (symmetric, asymmetric, hash, MAC, KDF) and draw a taxonomy diagram showing how they compose into protocols.
- Describe at least three historical cryptographic failures and identify which promise was violated and why.

## The Problem

You just encrypted your database backups with AES-256. You feel safe. Then:

- An insider leaks the decryption key they memorized from a sticky note.
- A timing side channel in your decryption routine leaks whether a padding byte is valid — an attacker recovers plaintext one byte at a time.
- Your random number generator seeds from `Math.random()` in JavaScript. The "256-bit" key has maybe 48 bits of entropy.
- The server holding the backups goes offline. Nobody can read them — not even you. That's not a privacy win; that's an availability loss.

Cryptography made none of these promises. **Encryption promises confidentiality — that only someone with the key can read the data.** It does not promise that the key stays secret, that the implementation is side-channel-free, that your randomness is random, or that the system stays online.

This lesson is about drawing that line precisely. Before we build any primitive (block ciphers, hash functions, RSA) or any protocol (TLS, Signal), we need to know: what is the primitive mathematically guaranteed to do, and where do we need engineering discipline to fill the gaps?

## The Concept

### The Four Promises

| Promise | Definition | Example |
|---------|-----------|---------|
| **Confidentiality** | Only intended recipients can read the plaintext | Encrypting a message with AES; only the holder of the key can decrypt |
| **Integrity** | The message has not been altered in transit | SHA-256 hash of a file detects a single flipped bit |
| **Authenticity** | The message comes from who it claims to be | An HMAC tag proves the sender holds the shared key |
| **Non-repudiation** | The sender cannot later deny sending the message | A digital signature binds the sender's private key to the message |

A protocol may provide several of these simultaneously. AES-GCM gives confidentiality + integrity + authenticity. An RSA signature gives authenticity + non-repudiation. A bare SHA-256 hash gives integrity only — anyone can recompute it.

### What Cryptography Does NOT Promise

| Non-promise | Why it matters |
|-------------|---------------|
| **Availability** | Encrypted data that nobody can decrypt (lost key, offline server) is useless. Crypto protects privacy, not uptime. |
| **Resistance to social engineering** | The math doesn't help when a human hands over their password. Phishing bypasses every cipher. |
| **Implementation correctness** | Heartbleed was a buffer over-read in OpenSSL — the RSA math was fine. The C code was broken. |
| **Key management** | Compromised keys = compromised crypto. No amount of AES key-size matters if the key is on a sticky note. |
| **Side-channel resistance** | Timing, power, cache attacks extract secrets without breaking the math. |

### Cryptography vs. Security

**Cryptography** is the mathematical study of algorithms that transform plaintext into ciphertext and back, under well-defined assumptions (e.g., "the adversary cannot factor large semiprimes in polynomial time"). A proof says: *under assumption X, no algorithm running in time Y can distinguish the ciphertext from random with advantage greater than Z.*

**Security** is an engineering discipline. It asks: *is the system resilient against real adversaries with real capabilities?* That includes the math, but also the implementation, the key management, the update cadence, the threat landscape, and the humans operating it.

A cryptosystem can be **mathematically perfect** and **practically broken**. The one-time pad is information-theoretically secure — no amount of computation can break it. But key distribution is a nightmare: the key must be as long as the message, shared in advance, and never reused. In practice, keys get reused, and the pad offers zero integrity — swapping two ciphertext bytes swaps the corresponding plaintext bytes undetectably.

### Threat Models

Before choosing any cryptographic primitive, you must answer: **who are you protecting against?**

| Threat actor | Capabilities | Example defenses |
|-------------|-------------|-----------------|
| **Script kiddie** | Runs publicly available tools, no custom exploits | TLS, basic input validation |
| **Organized criminal** | Buys 0-days, can mount side-channel attacks | Constant-time implementations, HSMs |
| **Nation-state** | Unlimited budget, can infiltrate hardware supply chains | Air-gaps, hardware verification, post-quantum crypto |
| **Insider** | Already has legitimate access | Least-privilege, audit logs, multi-party authorization |

Different threat models demand different defenses. Encrypting against a script kiddie is different from encrypting against the NSA — and encrypting against a malicious insider is a different problem entirely (they may already hold the key).

### Kerckhoffs' Principle

Auguste Kerckhoffs stated in 1883:

> *The system must not require secrecy and can be stolen by the enemy without causing trouble.*

In modern form: **the security of a cryptosystem should depend only on the secrecy of the key, not on the secrecy of the algorithm.**

Why?

1. **Algorithms get reverse-engineered.** If your security depends on the attacker not knowing the algorithm, a single reverse-engineering effort breaks everything.
2. **Public algorithms get peer-reviewed.** AES, SHA-256, ChaCha20 — all public, all extensively analyzed. A secret algorithm has zero external scrutiny.
3. **Key rotation is easy; algorithm rotation is catastrophic.** Compromised key? Generate a new one. Compromised algorithm? Redesign the entire system.

**Security through obscurity is not security.** It is, at best, delay.

### The Primitives Taxonomy

```
                    ┌─────────────────────────────────────────┐
                    │          Cryptographic Primitives       │
                    └─────────────────┬───────────────────────┘
                                      │
            ┌─────────────────────────┼─────────────────────────┐
            │                         │                         │
   ┌────────┴────────┐     ┌─────────┴──────────┐    ┌───────┴────────┐
   │   Symmetric      │     │   Asymmetric        │    │  Unkeyed        │
   │  (same key       │     │  (key pair:         │    │  (no key)       │
   │   encrypt/decrypt)│     │   public + private) │    │                 │
   └────────┬─────────┘     └─────────┬──────────┘    └───────┬────────┘
            │                         │                        │
      ┌─────┴──────┐          ┌───────┴───────┐        ┌──────┴──────┐
      │ Block      │          │ Encryption    │        │ Hash        │
      │ ciphers    │          │ (RSA, ElGamal)│        │ (SHA-256,   │
      │ (AES,      │          └───────┬───────┘        │  BLAKE3)    │
      │  ChaCha20) │                  │                └──────┬──────┘
      └─────┬──────┘          ┌───────┴───────┐               │
            │                 │ Signatures    │        ┌──────┴──────┐
     ┌──────┴──────┐          │ (RSA-PSS,     │        │ Keyed Hash  │
     │ Modes of    │          │  ECDSA,       │        │ (MAC/HMAC)  │
     │ Operation   │          │  EdDSA)       │        └──────┬──────┘
     │ (CBC, CTR,  │          └───────┬───────┘               │
     │  GCM)       │                  │                ┌──────┴──────┐
     └──────┬──────┘          ┌───────┴───────┐        │ KDF         │
            │                 │ Key Exchange  │        │ (PBKDF2,    │
            │                 │ (DH, ECDH,   │        │  scrypt,    │
            │                 │  X25519)     │        │  Argon2)    │
            │                 └──────────────┘        └─────────────┘
            │
     ┌──────┴──────┐
     │ AEAD        │
     │ (AES-GCM,  │
     │  ChaCha20- │
     │  Poly1305) │
     └─────────────┘
```

How the primitives compose into protocols:

```
Symmetric encryption  ──┐
                         │──► AEAD (AES-GCM) ──► Confidentiality + Integrity + Authenticity
MAC (HMAC)            ──┘

Asymmetric encryption ──► Key exchange (ECDH) ──► Shared secret ──► Symmetric encryption
Digital signatures    ──► Authenticity + Non-repudiation
KDF                   ──► Password ──► Symmetric key (for encryption or MAC)

Full protocol (e.g., TLS 1.3):
  ECDH key exchange  ──► shared secret ──► KDF (HKDF) ──► symmetric keys ──► AEAD
  + digital signatures on the handshake ──► authentication
```

### Historical Failures

| Failure | What broke | Which promise(s) violated | Root cause |
|---------|-----------|--------------------------|------------|
| **Enigma** (WWII) | Repeated key settings, operator conventions ("weather report" cribs), key sheets distributed on paper | Confidentiality | Key management failures — the math was reasonable for its era; the operational practice was not |
| **DES** (1977–1998) | 56-bit keys brute-forced by EFF's "Deep Crack" machine in 56 hours | Confidentiality | Key size too small for advancing compute power |
| **MD5** (2004–2008) | Collision attacks: two different inputs produce the same hash. Used in SSL certs, software Integrity | Integrity | Mathematical weakness in the compression function; collisions found in hours on a laptop |
| **RSA PKCS#1 v1.5 padding oracle** (1998–2014) | Attacker sends modified ciphertexts, observes server's error messages ("padding correct" vs "padding incorrect"), recovers plaintext byte-by-byte | Confidentiality | Implementation error: the protocol leaked information through error messages (side channel) |

The pattern: **the math is rarely the first thing to break.** Enigma fell to key reuse and operator error. DES fell to Moore's law. MD5 fell to analytic advances on the compression function. RSA padding oracle fell to a protocol-level mistake. Only MD5 involves a mathematical break of the primitive itself.

## Build It

`code/main.py` demonstrates each primitive category. Each demo shows what the primitive promises and what it doesn't.

### Step 1: Minimal Version — XOR Symmetric Encryption

The simplest symmetric cipher: XOR the plaintext with a key stream. One key encrypts and decrypts.

```python
def xor_encrypt(plaintext: bytes, key: bytes) -> bytes:
    return bytes(p ^ k for p, k in zip(plaintext, key))

ciphertext = xor_encrypt(b"hello", b"\x17\x0a\x1c\x1e\x0b")
plaintext  = xor_encrypt(ciphertext, b"\x17\x0a\x1c\x1e\x0b")
```

**Promises:** Confidentiality (if key is random and never reused).
**Does not promise:** Integrity (flip any bit in ciphertext → flipped bit in plaintext, undetected), authenticity (anyone can produce ciphertext).

### Step 2: Realistic Version — Production-Grade Primitives

Replace XOR with `cryptography` library calls that use AES-GCM (symmetric + integrity + authenticity), HMAC (keyed integrity + authenticity), and PBKDF2 (key derivation from passwords).

## Use It

- **Python `cryptography` library**: `Fernet` provides AES-128 in CBC mode with HMAC-SHA256 for authenticated encryption. It's the library's "easy" API — use it when you need "encrypt and authenticate this blob."
- **OpenSSL**: The `openssl enc` command-line tool performs symmetric encryption with AES. The `openssl dgst` command computes message digests. The `openssl genpkey` / `openssl pkeyutl` commands handle asymmetric key generation and encryption/signing.
- **Signal Protocol**: Combines X3DH (extended triple Diffie-Hellman) key exchange + Double Ratchet algorithm + Curve25519 + AES-256-CBC + HMAC-SHA256. This is what "primitives composing into a protocol" looks like in production.

Production crypto differs from our demos in three ways:
1. **Authenticated encryption** (GCM/ChaCha20-Poly1305) is the default, not "encrypt then MAC." The authentication tag is mandatory — you cannot decrypt without verifying integrity.
2. **Key derivation** uses memory-hard functions (Argon2, scrypt) instead of PBKDF2 when protecting passwords, because GPUs make iterated hashes cheap.
3. **Random number generation** uses OS-provided CSPRNGs (`/dev/urandom`, `CryptGenRandom`), not `Math.random()` or user-space PRNGs.

## Read the Source

- [OpenSSL `crypto/aes/aes_core.c`](https://github.com/openssl/openssl/blob/master/crypto/aes/aes_core.c) — the AES round function implementation; see how T-tables (precomputed lookup tables) accelerate encryption.
- [libsodium `crypto_secretbox/xsalsa20poly1305/ref/api.h`](https://github.com/jedisct1/libsodium/blob/master/src/libsodium/crypto_secretbox/xsalsa20poly1305/ref/api.h) — production authenticated encryption API. Note the combined encrypt+authenticate interface — you never get one without the other.

## Ship It

This lesson ships **`outputs/crypto_primitives_demo.py`** — a self-contained Python script that demonstrates all five primitive categories with clear annotations showing what each promises and what it doesn't. Reuse it in later phases when you need a quick reference for "which primitive do I reach for?"

## Exercises

1. **Easy.** Encrypt a message with the XOR cipher from Step 1. Modify one byte of the ciphertext, then decrypt. What changed? Why didn't decryption fail? Explain which promise was violated.

2. **Medium.** Generate two different inputs that produce the same MD5 hash using an existing collision tool (e.g., `hashclash`). Verify with `md5sum`. Explain in terms of the promises table: which promises does a broken hash function no longer make?

3. **Hard.** Implement a simplified padding oracle attack: encrypt a message with AES-CBC (no authentication tag), then write an attacker function that, given only the ciphertext and an oracle that returns "valid padding" / "invalid padding," recovers the plaintext. Explain why AES-GCM makes this attack impossible.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Confidentiality | "Encryption keeps data private" | Only holders of the decryption key can recover plaintext — no guarantee about availability, integrity, or authenticity |
| Integrity | "The data hasn't been changed" | A forgery attempt will be detected — no guarantee about who sent the data (that's authenticity) |
| Authenticity | "We know who sent it" | The sender possessed a key or private key — no guarantee the sender is unique (key sharing) or that the data is private |
| Non-repudiation | "They can't deny sending it" | Only the holder of one specific private key could have produced the signature — requires a PKI or web-of-trust to bind key to identity |
| Kerckhoffs' principle | "Don't keep the algorithm secret" | The system must stay secure even if everything about it is public except the key; obscurity can delay, never replace, real security |
| Threat model | "Who are we defending against?" | A precise enumeration of the adversary's capabilities, motivations, and access level — determines which defenses are necessary and which are overkill |
| Security through obscurity | "Hiding it makes it safe" | Relying on the secrecy of the design rather than the key; any system that breaks when the design is public is insecure by definition |
| AEAD | "Encrypt with a tag" | Authenticated Encryption with Associated Data: confidentiality + integrity + authenticity in one primitive, with optional non-encrypted header data |
| Side channel | "Leakage" | Information revealed by the implementation's physical behavior (timing, power, cache) rather than by breaking the mathematical algorithm |

## Further Reading

- [Kerckhoffs' original 1883 paper](https://www.petitcolas.net/kerckhoffs/la_cryptographie_militaire.htm) — the six principles; principle #2 is the one everyone quotes.
- [Cryptographic Righter's Guide, 2015](https://golang.design/crypto-righter/) — Peter Gutmann's survey of what's actually secure vs. what people think is secure.
- *Serious Cryptography* by Jean-Philippe Aumasson (No Starch Press, 2018) — Chapters 1 and 2 cover the promises and threat models in depth.
- [The Padding Oracle Attack](https://robertheaton.com/2013/07/29/padding-oracle-attack/) — Robert Heaton's walkthrough of how a side channel in error messages breaks AES-CBC.