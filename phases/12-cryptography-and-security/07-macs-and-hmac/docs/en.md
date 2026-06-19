# MACs and HMAC

> A hash tells you the data arrived intact. A MAC tells you *who* sent it.

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 12 lessons 01–06 (especially 06 — Hash Functions)
**Time:** ~60 minutes

## Learning Objectives

- Distinguish a MAC from a hash: integrity alone vs. integrity + authenticity
- Explain why `H(key || message)` is vulnerable to the length extension attack
- Construct HMAC step by step and explain why it resists length extension
- Implement HMAC-SHA256 from scratch and verify against `hashlib`
- Describe CBC-MAC, its fixed-length requirement, and how CMAC fixes it
- Explain Poly1305's one-time authenticator model and its role in TLS
- Demonstrate a timing attack on tag comparison and apply constant-time verification

## The Problem

Alice sends Bob a message over an insecure channel. She appends a SHA-256 hash so Bob can verify integrity. But Mallory intercepts the message, modifies it, and appends *her own* hash. Bob checks the hash — it matches the modified message. The hash proved nothing, because anyone can compute a hash.

What Alice needs is a tag that only she can produce but anyone with the shared key can verify. That is a **Message Authentication Code**. Without MACs, every integrity check in TLS, JWT, SSH, and IPsec is theater. An attacker who can modify packets and recompute hashes owns the channel.

This lesson builds MACs from first principles: the broken naive construction, the HMAC standard, cipher-based MACs, and the Poly1305 authenticator that powers modern TLS.

## The Concept

### MAC Definition

A MAC is a function `MAC(K, M) → T` that takes a secret key `K` and a message `M` and produces a short tag `T`. Verification recomputes the tag and checks equality. Two security properties matter:

| Property | Meaning |
|----------|---------|
| **Computation resistance** | Given zero or more `(message, tag)` pairs, an adversary without `K` cannot compute a valid tag for any new message |
| **Tag forgery resistance** | An adversary cannot produce a valid `(message, tag')` pair for a message they haven't seen authenticated |

A hash has neither property — the key input is what makes a MAC a MAC.

### Naive MAC: `H(K || M)` — and Why It Breaks

The simplest MAC construction prepends the key to the message and hashes:

```
tag = SHA-256(key || message)
```

This is **vulnerable to the length extension attack**. SHA-256 (and all Merkle-Damgård hashes) process input in blocks. Given `H(X)`, an attacker can compute `H(X || padding || extension)` without knowing `X`, because the hash output *is* the internal state after processing `X`.

An attacker who sees `tag = H(K || M)` can compute a valid tag for `M || padding || attacker_data` without knowing `K`. That is a forgery.

### HMAC Construction

HMAC solves length extension by hashing *twice* with the key mixed in differently each time:

```
HMAC(K, m) = H((K ⊕ opad) || H((K ⊕ ipad) || m))
```

Where:
- `ipad` = `0x36` repeated to block size
- `opad` = `0x5C` repeated to block size
- If `K` is shorter than the block size (64 bytes for SHA-256), pad with zeros

```
┌─────────────────────────────────────────────┐
│              HMAC Flow                       │
│                                              │
│  K ──┬── ⊕ ipad ──┐                         │
│      │             │                         │
│      │         ┌───┴───┐                     │
│      │         │ concat │                     │
│      │         └───┬───┘                     │
│      │             │                         │
│      │         ┌───┴───┐                     │
│      │         │ H(·)  │  ← inner hash        │
│      │         └───┬───┘                     │
│      │             │                         │
│      ├── ⊕ opad ──┐│                         │
│      │             ││                         │
│      │         ┌───┴┴──┐                     │
│      │         │concat │                     │
│      │         └───┬───┘                     │
│      │         ┌───┴───┐                     │
│      │         │ H(·)  │  ← outer hash        │
│      │         └───┬───┘                     │
│      │             │                         │
│      └─────────► tag                         │
└─────────────────────────────────────────────┘
```

Why does this resist length extension? The inner hash `H((K ⊕ ipad) || m)` produces a digest. An attacker trying to extend this digest would be extending the *inner* hash, but the outer hash wraps it — the attacker would need to produce a valid outer hash, which requires `K ⊕ opad`, requiring knowledge of `K`.

### HMAC Security

- If `H` is collision-resistant, HMAC is a PRF (pseudo-random function) — proven by Bellare, Canetti, Krawczyk (1996)
- Even if `H` has collisions, HMAC may still be secure — the construction doesn't rely on collision resistance alone
- HMAC's security proof is one of the strongest in symmetric cryptography

### HKDF: HMAC-Based Key Derivation

HKDF (RFC 5869) uses HMAC for key derivation in two stages:

```
Extract:  PRK = HMAC-Hash(salt, IKM)     ← concentrate entropy
Expand:   OKM = HMAC-Hash(PRK, info || 0x01)
          OKM += HMAC-Hash(PRK, OKM[-N:] || info || 0x02)  ← extend to desired length
```

- **Extract**: takesInput Keying Material (IKM) and a salt, produces a pseudorandom key (PRK)
- **Expand**: takes PRK and context info, produces output keying material (OKM) of any desired length

HKDF is used in TLS 1.3 for all key derivation (see lesson 14).

### CBC-MAC

CBC-MAC uses a block cipher in CBC mode with IV = 0:

```
┌────────┐   ┌────────┐       ┌────────┐
│  M_1   │   │  M_2   │  ...  │  M_n   │
└───┬────┘   └───┬────┘       └───┬────┘
    │            │                │
    ▼            │                │
┌───────┐        │                │
│E_K(0⊕)│        │                │
└───┬───┘        │                │
    │            ▼                │
    ├──────────►⊕        ...      │
    │        ┌───────┐            │
    │        │E_K(·) │            │
    │        └───┬───┘            │
    │            │                ▼
    │            ├──────────►⊕
    │            │        ┌───────┐
    │            │        │E_K(·) │
    │            │        └───┬───┘
    │            │            │
    └────────────┴────────────┘
                          │
                          ▼
                        tag
```

CBC-MAC is **secure only for fixed-length messages**. For variable-length messages, an attacker can perform a length attack:

Given `tag₁ = CBC-MAC(K, M₁)` and `tag₂ = CBC-MAC(K, M₂)`, the attacker can forge `CBC-MAC(K, M₁ || (M₂ ⊕ tag₁) || M₂[2..])` without knowing `K`.

### CMAC

CMAC (Cipher-based MAC, NIST SP 800-38B) fixes CBC-MAC's length vulnerability by deriving two subkeys `K₁` and `K₂` from the main key and XORing them into the final block before encryption:

- If the message length is a multiple of the block size, XOR `K₁` into the last block
- If padding is needed, XOR `K₂` into the last block

Subkey derivation:
1. `L = AES_K(0^128)` — encrypt an all-zero block
2. `K₁ = L << 1` (if MSB(L) = 0, else XOR with constant)
3. `K₂ = K₁ << 1` (same conditional XOR)

### Poly1305

Poly1305 is a **one-time authenticator** — the key must be unique per message. It takes a 32-byte key `(r, s)` where `r` is a 16-byte clamp, `s` is a 16-byte secret:

```
tag = ((Σ cᵢ · rⁱ) mod (2¹³⁰ − 5)) + s  mod 2¹²⁸
```

Where `cᵢ` are message chunks treated as coefficients of a polynomial evaluated at `r`.

- The `mod (2¹³⁰ − 5)` makes the modular arithmetic fast (the prime is close to 2¹³⁰)
- The `+ s mod 2¹²⁸` ensures that without knowing `s`, the polynomial evaluation alone doesn't leak the tag
- In TLS, Poly1305 is always paired with ChaCha20 (which provides the one-time key `r, s` per nonce) — this is ChaCha20-Poly1305 AEAD

### Comparison

| Scheme | Basis | Key Reuse | Variable Length | Speed | Standard |
|--------|-------|-----------|-----------------|-------|----------|
| HMAC | Hash | Yes | Yes | Fast | RFC 2104 / FIPS 198-1 |
| CBC-MAC | Block cipher | Yes | No (fixed only) | Medium | NIST SP 800-38A |
| CMAC | Block cipher | Yes | Yes | Medium | NIST SP 800-38B |
| Poly1305 | Polynomial | No (one-time) | Yes | Very fast | RFC 8439 |

- **HMAC**: Most versatile. Any hash function works. Used wherever a keyed hash is needed (API auth, JWT, TLS PRF).
- **CBC-MAC**: Simple but dangerous for variable-length messages.
- **CMAC**: The safe cipher-based MAC for variable-length messages.
- **Poly1305**: Blazing fast but requires a fresh key per message. Always paired with a stream cipher.

### Timing Attacks on Tag Comparison

When verifying a MAC, you must compare the received tag against the computed tag in **constant time**. A byte-by-byte comparison that returns on the first mismatch leaks timing information:

```
# VULNERABLE — leaks which byte differs
for a, b in zip(received, computed):
    if a != b:
        return False
return True

# SAFE — always scans the entire tag
result = 0
for a, b in zip(received, computed):
    result |= a ^ b
return result == 0
```

Python's `hmac.compare_digest()` provides constant-time comparison. In Rust, use `subtle::ConstantTimeEq`.

## Build It

### Step 1: HMAC-SHA256 from Scratch (Python)

See `code/main.py` — we implement the full HMAC construction:
- SHA-256 block size = 64 bytes, digest size = 32 bytes
- Key padding to block size
- Inner pad = key ⊕ 0x36, outer pad = key ⊕ 0x5C
- Two hash passes: `inner = H(ipad || m)`, `tag = H(opad || inner)`

### Step 2: HKDF Extract-and-Expand (Python)

See `code/main.py` — we implement HKDF using our HMAC:
- Extract: `PRK = HMAC-SHA256(salt, IKM)`
- Expand: iterated HMAC to produce output of desired length

### Step 3: Length Extension Attack Demo (Python)

See `code/main.py` — we show that `H(K || M)` is forgeable using Merkle-Damgård length extension, and that `HMAC(K, M)` is not.

### Step 4: CBC-MAC (Rust)

See `code/main.rs` — we implement CBC-MAC using AES-128 for fixed-length messages and demonstrate the variable-length vulnerability.

### Step 5: Timing Attack Demo (Python)

See `code/main.py` — we show that comparing tags byte-by-byte leaks information proportional to the number of matching prefix bytes, and that `hmac.compare_digest()` does not.

## Use It

### Production HMAC

Python's `hashlib` and `hmac` modules implement HMAC natively:

```python
import hmac, hashlib
tag = hmac.new(key, message, hashlib.sha256).digest()
hmac.compare_digest(tag, received_tag)
```

TLS 1.3 uses HMAC-SHA256 for the HKDF that derives all session keys (RFC 8446 §5.1). Every modern TLS session starts with HMAC.

### Production CBC-MAC / CMAC

The `cryptography` library provides CMAC:

```python
from cryptography.hazmat.primitives.cmac import CMAC
from cryptography.hazmat.primitives.ciphers import algorithms
cmac = CMAC(algorithms.AES(key))
cmac.update(message)
tag = cmac.finalize()
```

### Production Poly1305

ChaCha20-Poly1305 AEAD is exposed by most crypto libraries:

```python
from cryptography.hazmat.primitives.ciphers.aead import ChaCha20Poly1305
aead = ChaCha20Poly1305(key)
ciphertext = aead.encrypt(nonce, plaintext, aad)
plaintext = aead.decrypt(nonce, ciphertext, aad)
```

## Read the Source

- **OpenSSL HMAC**: `crypto/hmac/hmac.c` — production HMAC implementation supporting multiple hash functions
- **libsodium Poly1305**: `src/libsodium/crypto_onetimeauth/poly1305/onetimeauth_poly1305.c` — the reference Poly1305 implementation used in TLS
- **Linux kernel CMAC**: `crypto/cmac.c` — kernel-level CMAC for IPsec and other in-kernel crypto
- **RFC 2104**: Section 2 — the original HMAC specification
- **RFC 8439**: Section 2.5 — Poly1305 definition and security analysis

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **HMAC-SHA256 and CBC-MAC reference implementations** with test vectors, usable as building blocks for the TLS 1.3 capstone in lesson 24.

## Exercises

1. **Easy** — Modify the Python HMAC implementation to use SHA-512 instead of SHA-256 (change block size to 128 bytes, digest size to 64).
2. **Medium** — Implement CMAC in Rust by deriving subkeys `K₁` and `K₂` from the AES key and XORing them into the final CBC-MAC block.
3. **Hard** — Implement Poly1305 from RFC 8439 §2.5 and compose it with ChaCha20 to create a working ChaCha20-Poly1305 AEAD encrypt-then-MAC construction.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| MAC | "Like a hash with a key" | A function MAC(K,M)→T where only key-holders can produce or verify tags, providing both integrity and authenticity |
| HMAC | "The MAC standard" | A specific MAC construction: H((K⊕opad)‖H((K⊕ipad)‖m)), proven secure if H is a PRF |
| Length extension | "You can extend the hash" | For Merkle-Damgård hashes, given H(X), one can compute H(X‖padding‖Y) without knowing X — breaks H(K‖M) as a MAC |
| CBC-MAC | "CBC as a MAC" | Using cipher block chaining with IV=0 to produce a tag; secure only for fixed-length messages |
| CMAC | "CBC-MAC but safe" | CBC-MAC with subkey derivation to prevent length-variable attacks (NIST SP 800-38B) |
| Poly1305 | "A fast MAC" | A one-time polynomial authenticator; key must be unique per message; always paired with ChaCha20 in practice |
| HKDF | "Key derivation from HMAC" | HMAC-based Extract-then-Expand KDF (RFC 5869); the basis of TLS 1.3 key schedule |
| Timing attack | "Don't leak the tag" | If tag comparison short-circuits on first mismatch, attackers extract the tag byte-by-byte via timing |

## Further Reading

- [RFC 2104 — HMAC: Keyed-Hashing for Message Authentication](https://www.rfc-editor.org/rfc/rfc2104) — the HMAC specification
- [RFC 5869 — HMAC-based Extract-and-Expand Key Derivation Function (HKDF)](https://www.rfc-editor.org/rfc/rfc5869) — HKDF standard
- [RFC 8439 — ChaCha20 and Poly1305](https://www.rfc-editor.org/rfc/rfc8439) — Poly1305 definition and composition with ChaCha20
- [NIST SP 800-38B — Recommendation for Block Cipher Modes of Operation: CMAC](https://csrc.nist.gov/publications/detail/sp/800-38b/final) — CMAC specification
- [Bellare, Canetti, Krawczyk 1996 — Keyed Hash Functions for Message Authentication](https://www.iacr.org/archive/crypto1996/11040401.pdf) — HMAC security proof