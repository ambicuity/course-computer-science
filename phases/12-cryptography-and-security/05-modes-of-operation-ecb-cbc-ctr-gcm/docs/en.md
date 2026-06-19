# Modes of Operation — ECB, CBC, CTR, GCM

> A block cipher encrypts one block. A mode of operation tells it what to do with the next one.

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 12 lessons 01–04
**Time:** ~75 minutes

## Learning Objectives

- Explain why block ciphers need modes of operation and what goes wrong without them.
- Describe ECB mode and demonstrate why it is broken (deterministic encryption, pattern leakage, penguin attack).
- Describe CBC mode, explain the IV requirements (unpredictable, not just unique), and show why padding is necessary and how padding oracle attacks work.
- Describe CTR mode, explain how it turns a block cipher into a stream cipher, and demonstrate why nonce reuse is catastrophic.
- Describe GCM mode, explain how GHASH provides authentication, and show why AEAD (confidentiality + integrity + authenticity) is the minimum acceptable standard.
- Implement all four modes in Python and AES-128-GCM in Rust, and compare their security properties.

## The Problem

You learned in Lesson 04 that AES-128 encrypts a single 16-byte block. But real messages are rarely exactly 16 bytes. A database column is 200 bytes. A TLS record is up to 16 KiB. A video stream is effectively infinite.

What do you do with the second block? The third? The millionth?

**You need a mode of operation** — a rule for how to chain blocks together. The rule determines nearly everything about the security of the result:

- Does the same plaintext always produce the same ciphertext? (ECB: yes, and that's fatal.)
- Can you decrypt blocks in parallel, or must you wait for the previous block? (CBC: sequential encryption, parallel decryption. CTR: fully parallel.)
- Can an attacker flip bits in the plaintext by modifying the ciphertext? (ECB, CBC, CTR: yes. GCM: detection via tag.)
- Can an attacker recover plaintext without the key by querying a server? (CBC with padding oracle: yes.)

Choose the wrong mode and you lose confidentiality, integrity, or both — even though AES itself is unbroken.

## The Concept

### Block Ciphers Process Fixed-Size Blocks

AES operates on 128-bit (16-byte) blocks. A mode of operation defines:

1. How to split a message of arbitrary length into blocks.
2. How to combine each plaintext block with the cipher's output.
3. Whether and how to incorporate an initialization vector (IV) or nonce.
4. How to handle the last block (padding, truncation, or streaming).

```
Message: "Hello, world! This is a longer message than 16 bytes"
           ├──── Block 0 ──┤├──── Block 1 ──┤├──── Block 2 ──┤
           16 bytes         16 bytes         remainder (padded)
```

### ECB (Electronic Codebook)

Each 16-byte block is encrypted independently with the same key:

```
ECB Encrypt:
  P₀ ──AES──► C₀
  P₁ ──AES──► C₁
  P₂ ──AES──► C₂

ECB Decrypt:
  C₀ ──AES⁻¹──► P₀
  C₁ ──AES⁻¹──► P₁
  C₂ ──AES⁻¹──► P₂
```

**Properties:**

| Property | Value |
|----------|-------|
| Deterministic | Yes — same plaintext block + same key = same ciphertext block |
| Parallelizable | Yes — encrypt and decrypt |
| Needs IV | No |
| Needs padding | Yes (for messages not a multiple of 16 bytes) |
| Integrity | None |

**The penguin attack:** When you encrypt an image with ECB, identical pixel regions produce identical ciphertext. The image's structure is visible in the ciphertext:

```
Original Tux (8x8 pixel grid, each pixel = 1 block):
  B B B W W B B B
  B B W W W W B B
  B W W O O W W B
  B W O W W O W B
  B W W W W W W B
  B B W W W W B B
  B B B W W B B B
  B B B B B B B B

ECB-encrypted Tux (same structure visible):
  X X X Y Y X X X      ← white pixels map to Y, black to X
  X X Y Y Y Y X X        ← structural pattern fully preserved
  X Y Y Z Z Y Y X      ← orange pixels map to Z
  X Y Z Y Y Z Y X
  ...
```

ECB is broken. Never use it.

### CBC (Cipher Block Chaining)

Before encrypting each block, XOR it with the previous ciphertext block. The first block is XORed with an IV:

```
CBC Encrypt (sequential — each block depends on previous):
  IV
   │
   ▼
  P₀ ──XOR──► AES ──► C₀ ──┐
                              │
                              ▼
               P₁ ──XOR──► AES ──► C₁ ──┐
                                         │
                                         ▼
                          P₂ ──XOR──► AES ──► C₂

CBC Decrypt (parallelizable — each block depends only on one ciphertext block):
  C₀ ──AES⁻¹──► XOR ◄── IV  ──► P₀
  C₁ ──AES⁻¹──► XOR ◄── C₀ ──► P₁
  C₂ ──AES⁻¹──► XOR ◄── C₁ ──► P₂
```

**IV requirements:** The IV must be **unpredictable** (random or encrypted), not just unique. A predictable IV enables a chosen-plaintext attack: if the attacker can predict the IV before submitting plaintext, they can deduce relationships between plaintext blocks.

**Padding (PKCS#7):** If the message doesn't fill a complete block, pad it:

```
Message length 13 (3 bytes short):
  "Hello, world!\x03\x03\x03"

Message length 16 (exactly full — add a full padding block):
  "Hello, world!123\x10\x10\x10\x10\x10\x10\x10\x10\x10\x10\x10\x10\x10\x10\x10\x10"
```

**Properties:**

| Property | Value |
|----------|-------|
| Deterministic | No (with random IV) |
| Parallelizable | Encrypt: No. Decrypt: Yes. |
| Needs IV | Yes (must be unpredictable) |
| Needs padding | Yes |
| Integrity | None — vulnerable to bit-flipping |

### CTR (Counter Mode)

Encrypt a counter (nonce || block_count) with AES, then XOR the result with the plaintext. This turns AES into a stream cipher:

```
CTR Mode:
  nonce ──┬──► AES ──► XOR ◄── P₀ ──► C₀
          │
          1 ──► AES ──► XOR ◄── P₁ ──► C₁
          │
          2 ──► AES ──► XOR ◄── P₂ ──► C₂

Key stream: K₀ = AES(nonce || 0), K₁ = AES(nonce || 1), K₂ = AES(nonce || 2)
Ciphertext: C₀ = P₀ ⊕ K₀,  C₁ = P₁ ⊕ K₁,  C₂ = P₂ ⊕ K₂
```

**Properties:**

| Property | Value |
|----------|-------|
| Deterministic | No (with random nonce) |
| Parallelizable | Yes — encrypt and decrypt |
| Needs IV | Yes (nonce — must NEVER repeat with same key) |
| Needs padding | No — XOR is byte-by-byte |
| Integrity | None — bit-flipping is trivial |
| Seekable | Yes — decrypt block N without decrypting blocks 0..N-1 |

**Nonce reuse is catastrophic:** If you reuse the same nonce+key pair, you produce the same keystream. Two ciphertexts under the same keystream gives a two-time pad:

```
C₀ = P₀ ⊕ K    (first message)
C₁ = P₁ ⊕ K    (second message, same nonce)

C₀ ⊕ C₁ = P₀ ⊕ P₁    (keystream cancels!)
```

An attacker can XOR the two ciphertexts to get the XOR of the two plaintexts. From there, crib-dragging recovers both messages.

### GCM (Galois/Counter Mode)

GCM = CTR mode encryption + GHASH authentication tag. You get **AEAD** (Authenticated Encryption with Associated Data):

```
GCM Encrypt:
  1. H = AES(0^128)                              ← hash subkey
  2. Ciphertext via CTR: same as CTR mode above   ← confidentiality
  3. Tag = GHASH(H, AAD, Ciphertext) ⊕ E(J₀)   ← integrity + authenticity

GCM Decrypt:
  1. Recompute tag from (H, AAD, Ciphertext)
  2. Compare with received tag (constant-time!)
  3. If tag matches, decrypt via CTR             ← only decrypt after verifying

GHASH computation:
  Tag = ((((A₁ ⊗ H) ⊕ A₂) ⊗ H ⊕ ...) ⊕ (C₁ ⊗ H) ⊕ C₂) ⊗ H ... ⊕ [len(A)||len(C)] ⊗ H ⊕ E(J₀)
  where ⊗ = multiplication in GF(2^128)
```

**Why GHASH?** Multiplication in GF(2^128) is a one-way function under the hash key H. An attacker cannot forge a valid tag without knowing H, because finding x such that x ⊗ H = tag requires solving a discrete logarithm in GF(2^128).

**Properties:**

| Property | Value |
|----------|-------|
| Deterministic | No (with random nonce) |
| Parallelizable | Yes — both encryption and GHASH |
| Needs IV | Yes (nonce — must NEVER repeat) |
| Needs padding | No (CTR layer handles variable length) |
| Integrity | Yes — 128-bit authentication tag |
| Authenticated | Yes — forgeries detected before decryption |

### Comparison Table

| Mode | Confidentiality | Integrity | Padding | Parallel Encrypt | Parallel Decrypt | Verdict |
|------|----------------|-----------|---------|-----------------|------------------|---------|
| ECB | Broken (patterns leak) | None | Yes | Yes | Yes | **Never use** |
| CBC | Yes (with unpredictable IV) | None | Yes | No | Yes | Legacy only — no integrity |
| CTR | Yes (with unique nonce) | None | No | Yes | Yes | OK for confidentiality only |
| GCM | Yes (with unique nonce) | Yes (128-bit tag) | No | Yes | Yes | **Recommended** |

### Nonce Reuse Attacks

**CTR/GCM nonce reuse (two-time pad):**

```
Same key K, same nonce N:
  C₁ = P₁ ⊕ AES(N∥0) || P₁' ⊕ AES(N∥1) || ...
  C₂ = P₂ ⊕ AES(N∥0) || P₂' ⊕ AES(N∥1) || ...

  C₁ ⊕ C₂ = P₁ ⊕ P₂ || P₁' ⊕ P₂' || ...
```

Keystream cancels. Attacker gets plaintext XOR. Crib-dragging recovers both messages.

**CBC predictable IV (chosen-plaintext):**

If the attacker can predict IV before choosing plaintext P, they can test whether a guessed plaintext block P\* matches a previous ciphertext block C\*:

```
Submit P = P* ⊕ IV ⊕ C_prev_block
Server encrypts: AES(P ⊕ IV) = AES(P* ⊕ C_prev_block)
Compare result to C* — reveals whether P* matches the unknown block.
```

### Padding Oracle Attack (CBC)

When a server decrypts CBC ciphertext and returns different error messages for "invalid padding" vs "invalid MAC," an attacker can recover plaintext byte by byte:

```
Attacker sends:  R || C_i          (R = random block, C_i = target ciphertext block)
Server decrypts:  AES(C_i) ⊕ R

Attacker varies last byte of R:
  - When server says "valid padding," the last plaintext byte decrypted to 0x01
  - So: AES(C_i)[15] ⊕ R[15] = 0x01
  - So: AES(C_i)[15] = 0x01 ⊕ R[15]  →  P_i[15] = AES(C_i)[15] ⊕ C_{i-1}[15]

Repeat for each byte. The attacker never needs the key.
```

This attack broke SSL 3.0 and TLS 1.0. The fix: never reveal padding validity separately from MAC validity. AEAD modes (GCM) make this impossible because you verify the tag before any decryption.

## Build It

`code/main.py` implements all four modes and demonstrates the ECB penguin effect, padding oracle attack, and nonce-reuse catastrophe. `code/main.rs` implements AES-128-GCM from the ground up, including GF(2^128) multiplication for GHASH.

### Step 1: AES Primitive (Python)

We use the `cryptography` library's AES block cipher as our primitive and build the modes on top:

```python
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes
from cryptography.hazmat.primitives import padding
import os

def aes_encrypt_block(key: bytes, block: bytes) -> bytes:
    cipher = Cipher(algorithms.AES(key), modes.ECB())
    enc = cipher.encryptor()
    return enc.update(block) + enc.finalize()

def aes_decrypt_block(key: bytes, block: bytes) -> bytes:
    cipher = Cipher(algorithms.AES(key), modes.ECB())
    dec = cipher.decryptor()
    return dec.update(block) + dec.finalize()
```

### Step 2: ECB Mode

Each block encrypted independently. Same plaintext block → same ciphertext block.

```python
def ecb_encrypt(key: bytes, plaintext: bytes) -> bytes:
    padder = padding.PKCS7(128).padder()
    padded = padder.update(plaintext) + padder.finalize()
    return b"".join(aes_encrypt_block(key, padded[i:i+16]) for i in range(0, len(padded), 16))
```

### Step 3: CBC Mode

XOR each plaintext block with the previous ciphertext block before encryption.

```python
def cbc_encrypt(key: bytes, iv: bytes, plaintext: bytes) -> bytes:
    padder = padding.PKCS7(128).padder()
    padded = padder.update(plaintext) + padder.finalize()
    ciphertext = b""
    prev = iv
    for i in range(0, len(padded), 16):
        block = bytes(a ^ b for a, b in zip(padded[i:i+16], prev))
        encrypted = aes_encrypt_block(key, block)
        ciphertext += encrypted
        prev = encrypted
    return ciphertext
```

### Step 4: CTR Mode

Encrypt counter values, XOR with plaintext. No padding needed.

```python
def ctr_encrypt(key: bytes, nonce: bytes, plaintext: bytes) -> bytes:
    ciphertext = b""
    counter = 0
    for i in range(0, len(plaintext), 16):
        ctr_block = nonce + counter.to_bytes(8, "big")
        keystream = aes_encrypt_block(key, ctr_block)
        chunk = plaintext[i:i+16]
        ciphertext += bytes(a ^ b for a, b in zip(chunk, keystream[:len(chunk)]))
        counter += 1
    return ciphertext
```

### Step 5: GCM Mode

CTR encryption + GHASH authentication. The tag ensures both integrity and authenticity.

```python
def gcm_encrypt(key, nonce, plaintext, aad=b""):
    ciphertext = ctr_encrypt(key, nonce[:12], plaintext)
    h = aes_encrypt_block(key, b"\x00" * 16)
    tag = ghash(h, aad, ciphertext) ^ int.from_bytes(aes_encrypt_block(key, nonce[:12] + b"\x00\x00\x00\x01"), "big")
    return ciphertext, tag.to_bytes(16, "big")
```

### Step 6: Padding Oracle Attack

Simulate a server that leaks padding validity, then recover plaintext without the key.

```python
def padding_oracle_attack(key, iv, ciphertext):
    # Server: decrypt and check padding, return True/False
    def oracle(ct_block, prev_block):
        try:
            decrypted = aes_decrypt_block(key, ct_block)
            plaintext_block = bytes(a ^ b for a, b in zip(decrypted, prev_block))
            unpadder = padding.PKCS7(128).unpadder()
            unpadder.update(plaintext_block)
            unpadder.finalize()
            return True
        except Exception:
            return False
    # Attack: recover each byte from last to first
    ...
```

Full implementation in `code/main.py`.

### Step 7: AES-128-GCM in Rust (code/main.rs)

Implement GF(2^128) multiplication for GHASH, then build GCM encrypt/decrypt with NIST test vectors.

```rust
fn gf128_mul(x: u128, y: u128) -> u128 {
    let reduction = 0xe1 << 120; // x^128 + x^7 + x^2 + x + 1
    let mut z = 0u128;
    let mut v = y;
    for i in (0..128).rev() {
        if (x >> i) & 1 == 1 { z ^= v; }
        let carry = v & 1;
        v >>= 1;
        if carry != 0 { v ^= reduction; }
    }
    z
}
```

Full implementation including GCM encrypt, decrypt, and NIST test vectors in `code/main.rs`.

## Use It

In production, you should never implement these modes yourself. Use well-vetted libraries:

- **Python `cryptography`**: `Cipher(algorithms.AES(key), modes.GCM(nonce))` — the library handles GHASH, tag computation, and verification. You provide key, nonce, plaintext, and AAD; it returns ciphertext + tag.
- **Rust `aes-gcm` crate**: `Aes256Gcm::encrypt(&nonce, plaintext.as_ref())` — AEAD interface with compile-time key size enforcement.
- **OpenSSL**: `EVP_aes_128_gcm()` — the C interface used by virtually every TLS library. Handles GCM internally via hardware AES-NI instructions where available.

What the production versions do that ours doesn't:

1. **Hardware acceleration**: AES-NI instructions perform one AES round in a single CPU cycle. Our software implementation takes ~200 cycles per round.
2. **Constant-time GHASH**: Production implementations use carry-less multiplication instructions (`PCLMULQDQ` on x86) to avoid timing side channels in GF(2^128) multiplication.
3. **Tag verification in constant time**: `subtle::ConstantTimeEq` or equivalent — our Rust version uses ` == 0` which is vulnerable to timing attacks.
4. **Nonce misuse resistance**: AES-GCM-SIV delays the effects of nonce reuse by deriving the nonce from the plaintext, at the cost of two-pass encryption.

## Read the Source

- [OpenSSL `crypto/modes/gcm128.c`](https://github.com/openssl/openssl/blob/master/crypto/modes/gcm128.c) — production GCM implementation; look at `CRYPTO_gcm128_encrypt` to see CTR encryption interleaved with GHASH updates.
- [Rust `aes-gcm` crate source](https://github.com/RustCrypto/AEADs/tree/master/aes-gcm) — clean Rust AEAD implementation; note the `Aead` trait that enforces encrypt/decrypt-with-tag interface.
- [NIST SP 800-38D](https://csrc.nist.gov/publications/detail/sp/800-38d/final) — the official GCM specification. Sections 5.2.1 (GHASH) and 7.1 (encryption) are the authoritative reference.

## Ship It

This lesson ships two artifacts in `outputs/`:

- **`modes_demo.py`** — a self-contained Python script demonstrating all four modes, the ECB penguin effect, and the padding oracle attack. Reuse it in later phases when you need to test mode-specific behavior.
- **`aes128_gcm.rs`** — a standalone Rust implementation of AES-128-GCM with GF(2^128) multiplication. Reuse it as a reference when building the TLS 1.3 capstone.

## Exercises

1. **Easy.** Encrypt the same 48-byte message with ECB and CBC (random IV). Decrypt both. Encrypt again with ECB — notice the ciphertext is identical. Encrypt again with CBC — notice it's different. Explain why.
2. **Medium.** Implement CTR mode encryption from scratch (using AES-128 as the block cipher). Encrypt a message, flip a single bit in the ciphertext, and decrypt. Where does the flipped bit appear in the plaintext? Now do the same with GCM — what happens when tag verification fails?
3. **Hard.** Implement a full padding oracle attack against CBC mode. Given an oracle function `is_padding_valid(iv, ciphertext_block)` that returns True/False, recover the plaintext of an arbitrary ciphertext block without ever knowing the key. Estimate the number of oracle queries needed (average: 128 per byte, so ~2048 for a 16-byte block). Explain why GCM makes this attack class impossible.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Mode of operation | "How to encrypt multiple blocks" | A scheme defining how to chain block cipher operations across a message longer than one block — determines confidentiality, integrity, and parallelism properties |
| ECB | "Simple block encryption" | Electronic Codebook: each block encrypted independently. Deterministic, pattern-preserving, broken. Never use it. |
| CBC | "XOR with previous ciphertext" | Cipher Block Chaining: XOR each plaintext block with the previous ciphertext block before encryption. Requires unpredictable IV and padding. No integrity. |
| CTR | "Counter mode" | Counter mode: encrypt counter values as keystream, XOR with plaintext. Turns block cipher into stream cipher. No padding. Nonce must never repeat. No integrity. |
| GCM | "Authenticated encryption" | Galois/Counter Mode: CTR encryption + GHASH authentication tag. Provides confidentiality, integrity, and authenticity (AEAD). The recommended mode. |
| GHASH | "The GCM hash" | Multiplication in GF(2^128) using hash key H = AES(0^128). An attacker cannot forge a valid GHASH tag without knowing H. |
| IV / Nonce | "Random starting value" | IV (CBC): must be unpredictable. Nonce (CTR/GCM): must be unique per key, but need not be random. Reuse breaks security in all modes. |
| Padding oracle | "Server says padding error" | An oracle that reveals whether decrypted padding is valid. Enables plaintext recovery without the key. Killed SSL 3.0 and TLS 1.0. |
| AEAD | "Encrypt and authenticate" | Authenticated Encryption with Associated Data: one primitive providing confidentiality + integrity + authenticity. AES-GCM and ChaCha20-Poly1305 are AEAD. |
| Two-time pad | "Key reuse" | When a stream cipher keystream is reused (CTR/GCM nonce reuse), XORing the two ciphertexts cancels the keystream, revealing the XOR of the plaintexts. |
| PKCS#7 | "Padding scheme" | Pad each message to a multiple of the block size: if 3 bytes of padding are needed, append `\x03\x03\x03`. A full block of padding (`\x10` × 16) is added when the message is already block-aligned. |

## Further Reading

- [NIST SP 800-38A](https://csrc.nist.gov/publications/detail/sp/800-38a/final) — the official specification for ECB, CBC, CFB, OFB, and CTR modes. Section 6 shows test vectors.
- [NIST SP 800-38D](https://csrc.nist.gov/publications/detail/sp/800-38d/final) — the official GCM specification. Read Sections 5 (GHASH definition) and 7 (encryption/decryption algorithms).
- *Serious Cryptography* by Jean-Philippe Aumasson, Chapters 3 (Block Cipher Modes) and 4 — the best textbook treatment of modes, with clear diagrams and attack descriptions.
- [The Padding Oracle Attack](https://robertheaton.com/2013/07/29/padding-oracle-attack/) — Robert Heaton's visual walkthrough of how CBC padding oracles leak plaintext.
- [The ECB Penguin](https://blog.cryptographyengineering.com/2012/09/the-ecb-penguin-still-matters.html) — Matthew Green on why the canonical ECB attack (encrypting the Tux image) still matters in 2012 and beyond.