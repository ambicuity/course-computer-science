# Authenticated Encryption (AEAD)

> Encrypt without authenticate = mail a sealed envelope that anyone can open, re-seal, and re-send.

**Type:** Learn
**Languages:** Rust
**Prerequisites:** Phase 12 lessons 01–07
**Time:** ~60 minutes

## Learning Objectives

- Explain why combining encryption and MAC naively (Encrypt-and-MAC, MAC-then-Encrypt, Encrypt-then-MAC) has subtle ordering pitfalls, and prove that only Encrypt-then-MAC is generically secure.
- Describe how SSL/TLS 1.0's MAC-then-Encrypt choice led to padding oracle attacks (Lucky13) and why AEAD was created to eliminate this class of bug entirely.
- Define AEAD — Authenticated Encryption with Associated Data — and explain what AAD is for (authenticated but not encrypted header data like IP headers or sequence numbers).
- Implement ChaCha20-Poly1305 from scratch (ChaCha20 encryption + Poly1305 authentication) and verify against RFC 7539 test vectors.
- Compare AES-GCM (hardware-accelerated, nonce-misuse sensitive) with ChaCha20-Poly1305 (software-friendly, constant-time, no AES-NI needed) and know when to choose each.
- Manage nonces correctly: explain why nonce reuse in GCM is catastrophic (allows key recovery via GHASH), and contrast counter-based vs. random nonce generation.

## The Problem

You just encrypted a message with AES-CTR and attached an HMAC-SHA256 tag. You did the right thing — encryption for confidentiality, MAC for integrity. But did you encrypt-then-MAC, MAC-then-encrypt, or encrypt-and-MAC? Because the answer determines whether your system is actually secure.

**MAC-then-Encrypt (MtE):** You compute the MAC on the plaintext, then encrypt both the plaintext and the MAC tag. This is what SSL 3.0 and TLS 1.0 did. The decryptor must first decrypt (to get the MAC), then verify. A padding oracle attacker can manipulate the ciphertext, observe whether padding check fails before MAC verification, and recover plaintext byte by byte. The **Lucky13** attack (2013) exploited exactly this in TLS — it recovered plaintext using timing differences between valid and invalid padding.

**Encrypt-and-MAC (E&M):** You encrypt the plaintext and compute the MAC on the plaintext independently, then send both. SSH used this. The MAC may leak information about the plaintext because the tag is computed on unencrypted data. Worse, some MAC constructions can produce tags that validate on different keys for related plaintexts.

**Encrypt-then-MAC (EtM):** You encrypt first, then MAC the ciphertext. The decryptor verifies the MAC *before* decryption — a failed tag means the ciphertext is never decrypted at all, eliminating oracle attacks. This is the only composition provably secure for all underlying encryption and MAC schemes.

Here's the problem: even with EtM, you have two keys to manage, two operations to compose, two code paths to audit. Every implementation that got this wrong — and dozens did — looked correct at first glance. AEAD was invented to make the secure choice the *only* choice.

## The Concept

### The Composition Problem

```
Three ways to combine Encryption (E) and MAC (M)

  MAC-then-Encrypt (MtE)          Encrypt-and-MAC (E&M)         Encrypt-then-MAC (EtM)
  ─────────────────────           ─────────────────────         ─────────────────────
  tag = MAC(K2, P)                tag = MAC(K2, P)              C = E(K1, P)
  C  = E(K1, P || tag)            C  = E(K1, P)                 tag = MAC(K2, C)
                                   send C || tag                 send C || tag
  Decrypt first, then verify                                      Verify first, then decrypt
  ──padding oracles!──            ──MAC leaks plaintext!──       ──provably secure──
  (SSL/TLS 1.0, BEAST,            (SSH, some IPsec)
   Lucky13)
```

The attack surface difference is fundamental:

- **MtE:** Decryption happens before authentication. An attacker who can distinguish "bad padding" from "bad MAC" recovers plaintext. This is a *padding oracle*.
- **E&M:** The MAC is over plaintext, so the tag may leak information. Also, the receiver processes both encrypted and unencrypted data before authentication.
- **EtM:** Authentication gates decryption. Bad ciphertext is rejected before any cryptographic processing of the plaintext. No oracle possible.

### AEAD: One Primitive, All Three Properties

An AEAD scheme provides **confidentiality + integrity + authenticity** in a single operation with a single key (or key schedule):

```
AEAD.Encrypt(key, nonce, aad, plaintext) → (ciphertext, tag)

  ┌─────────────┐
  │   Key       │────► derive encryption subkey + authentication subkey
  └─────────────┘
         │
  ┌──────┴──────┐
  │    Nonce    │ (never reuse with same key)
  └──────┬──────┘
         │
  ┌──────┴──────────────────────┐
  │  Associated Data (AAD)     │──► authenticated, NOT encrypted
  │  (e.g., packet headers,     │    included in tag computation
  │   sequence numbers,         │
  │   protocol metadata)        │
  └──────┬──────────────────────┘
         │
  ┌──────┴──────────────────────┐
  │  Plaintext                  │──► encrypted AND authenticated
  └──────┬──────────────────────┘
         │
  ┌──────┴──────────────────────┐
  │  Ciphertext || Tag          │
  │                             │
  │  tag authenticates both      │
  │  AAD and ciphertext         │
  └─────────────────────────────┘

AEAD.Decrypt(key, nonce, aad, ciphertext, tag) → plaintext  OR  error

  If tag verification fails → reject immediately, do NOT release plaintext.
```

The tag covers *both* the AAD and the ciphertext. Any modification to either — a flipped bit in the header or a flipped bit in the encrypted body — causes verification to fail. The decryptor never processes unverified plaintext.

### AES-GCM

**GCM** (Galois/Counter Mode) combines AES in CTR mode for encryption with GHASH (a universal hash over GF(2¹²⁸)) for authentication:

```
AES-GCM Encryption:

  Counter 0:  E(K, 0²⁹‖nonce‖0³¹)  →  H  (hash subkey, 128-bit)
  Counter 1:  E(K, 1²⁹‖nonce‖0³¹)  →  E₁ (first keystream block)
  Counter 2:  E(K, 2²⁹‖nonce‖0³¹)  →  E₂ (second keystream block)
  ...

  Ciphertext: Cᵢ = Pᵢ ⊕ Eᵢ   (CTR mode encryption)

  Tag: GHASH(H, AAD, C) ⊕ E(K, 0²⁹‖nonce‖0³¹)

  GHASH computes:  Σ cᵢ·Hⁿ⁻ⁱ⁺¹  over GF(2¹²⁸)
  where cᵢ are blocks of: AAD || padding || C || padding || len(AAD)||len(C)
```

Key properties:
- **Single-pass encryption:** one AES call per 128-bit block for the keystream.
- **One additional pass** over the data for GHASH tag computation.
- **96-bit nonce** (IV). Must NEVER be reused with the same key. Nonce reuse allows an attacker to recover the authentication key H, enabling tag forgery and, in some cases, plaintext recovery.
- **Hardware acceleration:** AES-NI instructions make GCM extremely fast on x86 (~10 GB/s). ARM chips without AES-NI need table-based implementations that are slower and vulnerable to cache-timing attacks.

### ChaCha20-Poly1305

**ChaCha20** is a stream cipher (20 rounds of the ChaCha quarter-round). **Poly1305** is a one-time MAC that evaluates a polynomial in GF(2¹³⁰ - 5). Together they form an AEAD:

```
ChaCha20-Poly1305 Encryption:

  1. Derive Poly1305 one-time key:
     key_block = ChaCha20(K, nonce, counter=0)
     r = key_block[0:16]   (clamped: clear bits 4,5,6,7 of bytes 3,7,11,15)
     s = key_block[16:32]

  2. Encrypt plaintext:
     keystream = ChaCha20(K, nonce, counter=1), ChaCha20(K, nonce, counter=2), ...
     C = P ⊕ keystream

  3. Compute Poly1305 tag:
     mac_data = aad || pad(aad) || C || pad(C) || len(aad)_8B || len(C)_8B
     tag = Poly1305(mac_data, r, s)
          = (Σ mᵢ · rⁿ⁻ⁱ⁺¹  mod 2¹³⁰-5) + s  mod 2¹²⁸

  Counter 0 = key derivation for Poly1305
  Counter 1+ = keystream for encryption
```

Key properties:
- **No hardware dependency:** constant-time pure software implementation. No lookup tables, no cache-timing attacks.
- **96-bit nonce**, but XChaCha20-Poly1305 extends this to 192 bits via HChaCha20.
- **Preferred on mobile/ARM** where AES-NI is unavailable or slow. Used in TLS 1.3 (mandatory-to-implement cipher suite), WireGuard, SSH.
- **RFC 8439** (formerly 7539) defines the construction with test vectors.

### AES-GCM-SIV: Nonce-Misuse Resistance

Standard GCM is catastrophic on nonce reuse. **AES-GCM-SIV** (RFC 8452) is a nonce-misuse resistant variant:

- If the nonce is unique, SIV provides the same security as GCM.
- If the nonce is reused, the worst-case is that the attacker learns whether two plaintexts are identical. No key recovery, no tag forgery.
- Cost: two passes over the data (one to derive the SIV, one to encrypt).

### XChaCha20-Poly1305

The 96-bit nonce in ChaCha20-Poly1305 makes random nonce generation risky — the birthday bound gives a 2⁻³² collision probability at ~2³² messages. **XChaCha20-Poly1305** extends the nonce to 192 bits:

```
XChaCha20-Poly1305:

  1. HChaCha20(K, nonce[0:16])  →  subkey  (derives a new key from the first 16 nonce bytes)
  2. Use subkey with nonce[16:24] as the 96-bit nonce to standard ChaCha20-Poly1305
  3. 192-bit random nonce: birthday collision at 2⁹⁶ messages — safe for random nonces
```

Used by libsodium's `crypto_secretbox`, the Age encryption tool, and most modern libraries that default to AEAD.

### Nonce Management

| Strategy | Mechanism | Collision risk | When to use |
|----------|-----------|---------------|-------------|
| Counter | Sequential state, starts at 0 | Zero (if counter never resets) | Server-side sessions, deterministic protocols |
| Random 96-bit | `os.urandom(12)` | 2⁻³² at 2³² messages (birthday) | Short-lived keys, low-volume traffic |
| Random 192-bit (XChaCha20) | `os.urandom(24)` | Negligible | Long-lived keys, high-volume, any scenario |
| SIV | Derive nonce from plaintext | Zero (nonce is a function of message content) | When you cannot guarantee state |

**GCM nonce reuse consequences:** If you encrypt two messages with the same key and nonce, the attacker can:
1. XOR the two ciphertexts to get the XOR of the two plaintexts (CTR mode keystream is identical).
2. Recover the GHASH key H, enabling arbitrary tag forgery.
3. In some cases, recover the plaintext entirely.

**The rule:** For AES-GCM, never reuse a nonce with the same key. If you cannot guarantee this, use AES-GCM-SIV or XChaCha20-Poly1305.

### Comparison Table

| Property | AES-GCM | ChaCha20-Poly1305 | AES-GCM-SIV |
|----------|---------|-------------------|-------------|
| Encryption | AES-CTR | ChaCha20 stream | AES-CTR |
| Authentication | GHASH (GF(2¹²⁸)) | Poly1305 (GF(2¹³⁰-5)) | GHASH + SIV |
| Hardware acceleration | AES-NI (x86) | None needed | AES-NI (x86) |
| Software speed | Slow without AES-NI | Constant-time, fast | Slow without AES-NI |
| Nonce size | 96 bits | 96 bits | 96 bits (+ SIV) |
| Nonce misuse | Catastrophic | Catastrophic | Graceful degradation |
| Tag size | 128 bits (configurable) | 128 bits | 128 bits |
| Deployed in | TLS 1.2/1.3, IPsec, WPA3 | TLS 1.3, WireGuard, SSH | TLS 1.3 (optional) |
| Key size | 128/256 bits | 256 bits | 128/256 bits |

## Build It

`code/main.rs` implements AEAD from scratch in Rust. We build two constructions:

1. **CTR-AES-HMAC**: a teaching AEAD that uses AES-128 in CTR mode + HMAC-SHA256 as the tag. This mirrors the EtM composition and helps you understand what AEAD does internally.
2. **ChaCha20-Poly1305**: a real AEAD construction verified against RFC 7539 test vectors.

Both implement a common `Aead` trait so you can see the API is identical regardless of the underlying primitives.

### Step 1: AEAD Trait and Types

```rust
trait Aead {
    fn encrypt(
        key: &[u8],
        nonce: &[u8],
        aad: &[u8],
        plaintext: &[u8],
    ) -> (Vec<u8>, Vec<u8>);

    fn decrypt(
        key: &[u8],
        nonce: &[u8],
        aad: &[u8],
        ciphertext: &[u8],
        tag: &[u8],
    ) -> Result<Vec<u8>, AeadError>;
}
```

The trait enforces that encrypt takes (key, nonce, AAD, plaintext) and returns (ciphertext, tag); decrypt takes (key, nonce, AAD, ciphertext, tag) and returns either plaintext or an error. There is no path that returns unverified plaintext.

### Step 2: ChaCha20 Quarter-Round and Core

The ChaCha20 state is 16 × 32-bit words. The quarter-round operates on four words (a, b, c, d):

```
a += b; d ^= a; d <<<= 16
c += d; b ^= c; b <<<= 12
a += b; d ^= a; d <<<= 8
c += d; b ^= c; b <<<= 7
```

Twenty rounds (10 double-rounds) of 8 quarter-rounds per double-round, applied to the state initialized with constants, key, counter, and nonce, produce the keystream.

### Step 3: Poly1305 Authentication

Poly1305 computes a polynomial MAC in GF(2¹³⁰ - 5):

```
tag = (Σ mᵢ · rⁿ⁻ⁱ⁺¹  mod p) + s  mod 2¹²⁸
```

Where r is the one-time key (with certain bits clamped to zero), s is a one-time pad, and mᵢ are the message blocks. The "one-time" property is critical: a new (r, s) pair must be derived for every message via ChaCha20.

### Step 4: Composition into ChaCha20-Poly1305

Combine the ChaCha20 keystream (counter=1+ for encryption) with the Poly1305 tag (key derived from counter=0). The AAD is prepended (with padding to 16-byte boundary) before the ciphertext in the Poly1305 input. Lengths of AAD and ciphertext are appended as 8-byte little-endian integers.

See `code/main.rs` for the full implementation and RFC 7539 test vector verification.

## Use It

**Production libraries:**

- **Rust `aes-gcm` crate**: `Aes256Gcm` implements `aead::Aead` trait. Uses AES-NI when available. Returns `Result<Vec<u8>, aes_gcm::Error>` — you cannot accidentally decrypt without verifying the tag.
- **Rust `chacha20poly1305` crate**: `ChaCha20Poly1305` implements the same `aead::Aead` trait. Drop-in replacement for `Aes256Gcm`. The `XChaCha20Poly1305` variant uses 192-bit nonces.
- **Go `crypto/cipher`**: `NewGCM` returns a cipher.AEAD interface. The `Seal`/`Open` methods mirror our trait.
- **Python `cryptography`**: `AESGCM` class provides `encrypt`/`decrypt`. No path to unverified plaintext.

**What production does that our implementation doesn't:**

1. **Constant-time comparison** for tag verification. Our code uses `==` on the tag, which short-circuits on the first mismatched byte. Production uses `crypto::util::fixed_time_eq` to prevent timing side channels.
2. **SIMD/vectorized GHASH.** Production GCM implementations use PCLMULQDQ (x86) or PMULL (ARM) instructions to compute GF(2¹²⁸) multiplication in hardware. Our Poly1305 uses scalar operations.
3. **Key separation.** Production derives separate encryption and MAC keys from a master key via HKDF. Our code derives the Poly1305 key from the ChaCha20 keystream (which is the standard construction), but the CTR-AES-HMAC construction uses separate key halves — a simplification.
4. **Nonce misuse resistance.** Production libraries offer AES-GCM-SIV (Rust: `aes-gcm-siv` crate) for environments where nonce uniqueness cannot be guaranteed.

## Read the Source

- [RFC 8439 §2.3–2.6](https://datatracker.ietf.org/doc/html/rfc8439) — the ChaCha20-Poly1305 construction, defined with test vectors.
- [RFC 5116](https://datatracker.ietf.org/doc/html/rfc5116) — the AEAD interface definition. Section 2 defines the encrypt/decrypt API we used.
- [Rust `aes-gcm` source](https://github.com/RustCrypto/AEADs/tree/master/aes-gcm) — production GCM with AES-NI and PCLMULQDQ.
- [RFC 8452](https://datatracker.ietf.org/doc/html/rfc8452) — AES-GCM-SIV: nonce-misuse resistant AEAD.

## Ship It

This lesson ships **`code/main.rs`** — a self-contained Rust implementation of AEAD encryption and decryption with both CTR-AES-HMAC (teaching) and ChaCha20-Poly1305 (production), verified against RFC 7539 test vectors. Reuse the trait and poly1305 module in the phase capstone (TLS 1.3 library).

## Exercises

1. **Easy.** Encrypt a message with ChaCha20-Poly1305, flip a single bit in the ciphertext, and attempt decryption. Verify that decryption fails. Explain which property of AEAD prevents the forgery.

2. **Medium.** Modify the CTR-AES-HMAC construction to use MAC-then-Encrypt (compute HMAC on plaintext, then encrypt plaintext || tag). Demonstrate a padding oracle-style scenario: show that the decryptor must process the ciphertext before it can reject, and explain why this is weaker than EtM.

3. **Hard.** Implement XChaCha20-Poly1305: the HChaCha20 key derivation step that takes a 256-bit key and 128-bit nonce prefix to produce a sub-key, then use that sub-key with the remaining 64-bit nonce suffix in standard ChaCha20-Poly1305. Verify that random 192-bit nonces can be used safely (birthday bound at 2⁹⁶).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| AEAD | "Authenticated encryption" | Authenticated Encryption with Associated Data: a single primitive providing confidentiality + integrity + authenticity, with optional unencrypted-but-authenticated header data (AAD) |
| AAD | "Extra data you send with the message" | Associated Data: data that is authenticated but not encrypted — packet headers, sequence numbers, protocol fields that must be integrity-protected but visible to intermediaries |
| Encrypt-then-MAC (EtM) | "Encrypt then tag" | The only provably secure composition: MAC the ciphertext, verify before decrypt. AEAD constructions are implicitly EtM |
| MAC-then-Encrypt (MtE) | "Tag the plaintext then encrypt" | An insecure composition used by SSL/TLS 1.0: decryption before verification enables padding oracle attacks (BEAST, Lucky13) |
| GHASH | "GCM's MAC" | A universal hash over GF(2¹²⁸) that authenticates both AAD and ciphertext in GCM. Fast with PCLMULQDQ hardware; vulnerable to timing attacks in software |
| Poly1305 | "The other MAC" | A one-time MAC that evaluates a polynomial in GF(2¹³⁰ - 5). "One-time" means a new key (r, s) for every message — derived from the cipher's keystream |
| Nonce | "A random number" | Number used ONCE. GCM requires a unique 96-bit nonce per key. Nonce reuse in GCM enables key recovery and tag forgery. Not random — just unique |
| AES-GCM-SIV | "Misuse-resistant GCM" | A variant where the nonce is derived from the plaintext via a PRF. If the nonce repeats with a different plaintext, it produces a different derived nonce. Misuse degrades to leaking whether plaintexts are equal |
| XChaCha20-Poly1305 | "ChaCha with a bigger nonce" | Extended-nonce variant: HChaCha20 derives a subkey from the first 128 nonce bits, then standard ChaCha20-Poly1305 uses the subkey with the remaining 64 nonce bits. 192-bit random nonces are safe |

## Further Reading

- [RFC 8439](https://datatracker.ietf.org/doc/html/rfc8439) — ChaCha20-Poly1305 construction and test vectors (formerly RFC 7539).
- [RFC 5116](https://datatracker.ietf.org/doc/html/rfc5116) — An Interface and Algorithms for Authenticated Encryption. The definitive AEAD API specification.
- [RFC 8452](https://datatracker.ietf.org/doc/html/rfc8452) — AES-GCM-SIV: Nonce-Misuse-Resistant Authenticated Encryption.
- [The Lucky13 Attack](https://www.imperialviolet.org/2013/02/04/lucky thirteen.html) — Al Fardan and Paterson's padding oracle attack on TLS CBC cipher suites. Demonstrates why MtE composition is fatal.
- [*Dangerous Usage of AEADs: Nonce Reuse*](https://soatok.blog/2021/10/11/nonce-reuse-is-a-format-oracle/) — Soatok's analysis of what happens when GCM nonces collide.
- *Serious Cryptography* by Jean-Philippe Aumasson — Chapter 12 (AEAD) covers GCM, ChaCha20-Poly1305, and the composition problem in depth.