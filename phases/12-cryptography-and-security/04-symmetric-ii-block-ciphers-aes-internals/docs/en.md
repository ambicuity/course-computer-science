# Symmetric II — Block Ciphers, AES Internals

> AES doesn't scramble text — it applies algebra. Every byte is an element of GF(2⁸), every round is a matrix multiply, and the S-box is an inverted field element wrapped in an affine transform.

**Type:** Build
**Languages:** C, Rust
**Prerequisites:** Phase 12 lessons 01–03
**Time:** ~90 minutes

## Learning Objectives

- Explain the difference between block ciphers and stream ciphers and why AES is a block cipher.
- Define Shannon's confusion and diffusion properties and map each AES round operation to one or both.
- Trace a 128-bit plaintext through all 10 rounds of AES-128, showing how the state matrix transforms at each step.
- Implement GF(2⁸) arithmetic (addition, multiplication, inversion) using the AES irreducible polynomial.
- Build a complete AES-128 encrypt/decrypt in C and Rust, matching FIPS 197 test vectors.

## The Problem

You send a 16-byte secret to a friend. An attacker sees every byte on the wire. If you XOR those bytes with a key — as a stream cipher would — a single flipped ciphertext bit flips exactly one plaintext bit. The attacker can flip specific bits and know what they flipped. Worse: if you reuse the key, the attacker gets `c₁ ⊕ c₂ = p₁ ⊕ p₂`, leaking relationships between messages.

Block ciphers solve this by **mixing** every plaintext bit into every ciphertext bit. A 1-bit change in the input should, on average, flip 50% of the output bits. That property — **diffusion** — is what makes block ciphers essential. AES is the block cipher the world runs on: TLS, SSH, disk encryption, VPNs — all of it reduces to AES under the hood.

Without understanding AES internals, you can't reason about side-channel countermeasures, can't debug implementations that fail test vectors, and can't understand why certain design choices (like the S-box construction) matter for security.

## The Concept

### Block Ciphers vs Stream Ciphers

| Property | Block Cipher | Stream Cipher |
|----------|-------------|---------------|
| Operates on | Fixed-size block (AES: 128 bits) | Individual bits/bytes |
| Core mechanism | Repeated rounds of substitution + permutation | Keystream XOR |
| Key reuse | Different ciphertext for same plaintext (with IV/mode) | Fatal — reveals `p₁ ⊕ p₂` |
| Internal structure | SPN or Feistel network | LFSR, ARX, or sponge |

A stream cipher generates a pseudorandom keystream and XORs it with plaintext — essentially a one-time pad with a PRNG. A block cipher transforms a fixed-size block through multiple rounds of mixing so that every output bit depends on every input bit and every key bit.

### Confusion and Diffusion

Claude Shannon identified two properties a cipher needs:

**Confusion** — the relationship between key and ciphertext should be complex. Changing one key bit should change many ciphertext bits in an unpredictable way. Achieved by nonlinear substitution (S-boxes).

**Diffusion** — the statistical structure of the plaintext should be dissipated across the ciphertext. Changing one plaintext bit should affect many ciphertext bits. Achieved by permutation and mixing (ShiftRows, MixColumns).

Every round of AES applies both: SubBytes provides confusion, ShiftRows + MixColumns provide diffusion, and AddRoundKey mixes the key material in.

### Feistel Networks

Before AES, DES ruled. DES uses a **Feistel network**:

```
    L₀          R₀
     |            |
     |     ┌──────┘
     |     |  F(R₀, K₀)
     |     └──────┐
     |            |
    L₁=R₀    R₁=L₀⊕F(R₀,K₀)
     |            |
     |     ┌──────┘
     |     |  F(R₁, K₁)
     |     └──────┐
     |            |
    L₂=R₁    R₂=L₁⊕F(R₁,K₁)
```

The Feistel structure guarantees decryptability: `L₀ = R₁ ⊕ F(R₁, K₁)`. The F function doesn't need to be invertible. This is elegant but limits how much diffusion each round achieves — one half is unchanged each round.

### Substitution-Permutation Networks (SPN)

AES uses an **SPN** instead of a Feistel network. In an SPN, the entire block is transformed every round:

```
Plaintext (128 bits)
    │
    ├─ AddRoundKey(key₀)
    │
    ├─ Round 1: SubBytes → ShiftRows → MixColumns → AddRoundKey(key₁)
    ├─ Round 2: SubBytes → ShiftRows → MixColumns → AddRoundKey(key₂)
    ├─ ...
    ├─ Round 9: SubBytes → ShiftRows → MixColumns → AddRoundKey(key₉)
    └─ Round 10: SubBytes → ShiftRows → AddRoundKey(key₁₀)  ← no MixColumns!
         │
    Ciphertext (128 bits)
```

Every round touches the full state. There's no "unchanged half." This gives faster diffusion than Feistel but requires every operation to be invertible — decryption applies the inverse of each operation in reverse order.

### AES State Matrix

AES operates on a 4×4 byte matrix called the **state**. The 128-bit input is filled column-by-column:

```
Input bytes:  b0  b1  b2  b3  b4  b5  b6  b7  b8  b9  b10 b11 b12 b13 b14 b15

State matrix (column-major):
 ┌────┬────┬────┬────┐
 │ b0 │ b4 │ b8 │b12 │   row 0
 │ b1 │ b5 │ b9 │b13 │   row 1
 │ b2 │ b6 │b10 │b14 │   row 2
 │ b3 │ b7 │b11 │b15 │   row 3
 └────┴────┴────┴────┘
  col0  col1  col2  col3
```

Row indices matter for ShiftRows. Column indices matter for MixColumns.

### AES Round Operations

**SubBytes** — Replace each byte with its S-box lookup. The S-box is computed as: invert the byte in GF(2⁸) (with `0x00` mapping to itself), then apply an affine transformation `b' = Ab ⊕ 0x63`. This gives a nonlinear mapping that resists linear and differential cryptanalysis.

**ShiftRows** — Cyclically shift row *i* left by *i* positions:
```
Row 0: no shift
Row 1: shift left by 1
Row 2: shift left by 2
Row 3: shift left by 3
```
This ensures that each column's bytes come from different columns of the previous step, creating inter-column diffusion.

**MixColumns** — Multiply each column by a fixed 4×4 matrix in GF(2⁸):

```
┌         ┐ ┌    ┐   ┌    ┐
│ 2  3  1  1 │ │ s0 │   │ s'0│
│ 1  2  3  1 │ │ s1 │ = │ s'1│
│ 1  1  2  3 │ │ s2 │   │ s'2│
│ 3  1  1  2 │ │ s3 │   │ s'3│
└         ┘ └    ┘   └    ┘
```

The constants 1, 2, 3 are in GF(2⁸). This creates intra-column diffusion: each output byte depends on all four input bytes of that column.

**AddRoundKey** — XOR the state with the 128-bit round key. This is the only operation that introduces key material.

### AES Key Schedule

AES-128 has a 128-bit (16-byte) key, expanded into 11 round keys (176 bytes total). The key schedule uses:

1. **RotWord** — rotate a 4-byte word left by one byte
2. **SubWord** — apply S-box to each byte of the word
3. **Rcon** — round constant: `[rc, 0, 0, 0]` where `rc` follows the GF(2⁸) powers of 2: `0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1B, 0x36`

```
For i from 4 to 43 (each word W[i]):
  if i % 4 == 0:
    W[i] = W[i-4] ⊕ SubWord(RotWord(W[i-1])) ⊕ Rcon[i/4]
  else:
    W[i] = W[i-4] ⊕ W[i-1]

Round key r = W[4r] || W[4r+1] || W[4r+2] || W[4r+3]
```

### AES Key Sizes and Rounds

| Variant   | Key Size | Rounds | Key Words | Expanded Key Bytes |
|-----------|----------|--------|-----------|-------------------|
| AES-128   | 128 bits | 10     | 4         | 176               |
| AES-192   | 192 bits | 12     | 6         | 208               |
| AES-256   | 256 bits | 14     | 8         | 240               |

More rounds = more mixing = harder cryptanalysis. The last round omits MixColumns to make decryption symmetric with encryption structure (same involution pattern), and omitting it doesn't weaken security because MixColumns is a linear operation — it adds no key-dependent confusion.

### GF(2⁸) Arithmetic

AES does all its math in **Galois Field GF(2⁸)** with the irreducible polynomial `p(x) = x⁸ + x⁴ + x³ + x + 1` (binary `0x11B`).

**Addition** = XOR. No carries in GF(2).

**Multiplication** = polynomial multiplication modulo `p(x)`. The function `xtimes(a)` multiplies by `x` (i.e., shifts left by 1, and XORs with `0x1B` if the high bit was set to reduce modulo `p(x)`).

```
xtimes(a):
  if a & 0x80:
    return ((a << 1) ^ 0x1B) & 0xFF
  else:
    return (a << 1) & 0xFF
```

General multiplication: `mul(a, b)` — use the "peasant multiplication" approach: shift-and-add (XOR) using xtimes.

**Inversion** — finding `b` such that `a × b = 1` in GF(2⁸). Used to build the S-box. Can be computed via exponentiation (`a⁻¹ = a²⁵⁴` since `a²⁵⁵ = 1` for all nonzero a) or via extended Euclidean algorithm.

### The S-Box Construction

```
For each byte b (0x00 to 0xFF):
  1. Compute b⁻¹ in GF(2⁸) (0x00 maps to 0x00)
  2. Apply affine transformation: each bit is a linear combination
     of bits of b⁻¹, plus a constant 0x63

Affine transform (matrix form):
  ┌1 0 0 0 1 1 1 1┐ ┌b7⁻¹┐   ┌1┐
  │1 1 0 0 0 1 1 1│ │b6⁻¹│   │1│
  │1 1 1 0 0 0 1 1│ │b5⁻¹│   │0│
  │1 1 1 1 0 0 0 1│ │b4⁻¹│ ⊕ │0│
  │1 1 1 1 1 0 0 0│ │b3⁻¹│   │0│
  │0 1 1 1 1 1 0 0│ │b2⁻¹│   │0│
  │0 0 1 1 1 1 1 0│ │b1⁻¹│   │1│
  └0 0 0 1 1 1 1 1┘ └b0⁻¹┘   └1┘
```

This two-step construction (invert, then affine) makes the S-box resistant to both linear and differential cryptanalysis. A pure inversion would have a simple algebraic description (vulnerable to interpolation attacks). The affine transform breaks that structure.

## Build It

### Step 1: GF(2⁸) Arithmetic (C)

The foundation: all AES operations reduce to XOR and GF(2⁸) multiplication.

```c
uint8_t xtime(uint8_t x) {
    return (x & 0x80) ? ((x << 1) ^ 0x1B) : (x << 1);
}

uint8_t gf_mul(uint8_t a, uint8_t b) {
    uint8_t result = 0;
    uint8_t hi;
    for (int i = 0; i < 8; i++) {
        if (b & 1) result ^= a;
        hi = a & 0x80;
        a <<= 1;
        if (hi) a ^= 0x1B;
        b >>= 1;
    }
    return result;
}
```

Test: `gf_mul(0x57, 0x83) == 0xC1` (FIPS 197 Section 4.2.1).

### Step 2: S-Box and Key Expansion (C)

The S-box is precomputed from the GF(2⁸) inverse + affine transform. We'll use a lookup table (the standard one from FIPS 197). Key expansion turns 16 key bytes into 176 bytes (11 round keys).

```c
static const uint8_t SBOX[256] = { /* standard FIPS 197 S-box */ };
static const uint8_t RCON[11] = {0x00,0x01,0x02,0x04,0x08,0x10,0x20,0x40,0x80,0x1B,0x36};

void key_expansion(const uint8_t key[16], uint8_t w[176]) {
    memcpy(w, key, 16);
    for (int i = 4; i < 44; i++) {
        uint8_t temp[4];
        memcpy(temp, w + 4*(i-1), 4);
        if (i % 4 == 0) {
            uint8_t t = temp[0]; temp[0] = SBOX[temp[1]] ^ RCON[i/4];
            temp[1] = SBOX[temp[2]]; temp[2] = SBOX[temp[3]];
            temp[3] = SBOX[t];
        }
        for (int j = 0; j < 4; j++) w[4*i+j] = w[4*(i-4)+j] ^ temp[j];
    }
}
```

### Step 3: AES-128 Encrypt (C)

The main loop: SubBytes → ShiftRows → MixColumns → AddRoundKey for rounds 1–9, then SubBytes → ShiftRows → AddRoundKey for round 10.

```c
void aes128_encrypt(const uint8_t in[16], uint8_t out[16], const uint8_t w[176]) {
    uint8_t state[16];
    memcpy(state, in, 16);
    add_round_key(state, w);

    for (int r = 1; r <= 9; r++) {
        sub_bytes(state);
        shift_rows(state);
        mix_columns(state);
        add_round_key(state, w + 16*r);
    }
    sub_bytes(state);
    shift_rows(state);
    add_round_key(state, w + 160);
    memcpy(out, state, 16);
}
```

### Step 4: AES-128 Decrypt (C)

Decryption reverses each operation: InvSubBytes → InvShiftRows → InvMixColumns → AddRoundKey (with keys in reverse order). The last round (first in decryption) omits InvMixColumns.

```c
void aes128_decrypt(const uint8_t in[16], uint8_t out[16], const uint8_t w[176]) {
    uint8_t state[16];
    memcpy(state, in, 16);
    add_round_key(state, w + 160);

    for (int r = 9; r >= 1; r--) {
        inv_shift_rows(state);
        inv_sub_bytes(state);
        add_round_key(state, w + 16*r);
        inv_mix_columns(state);
    }
    inv_shift_rows(state);
    inv_sub_bytes(state);
    add_round_key(state, w);
    memcpy(out, state, 16);
}
```

### Step 5: Full Rust Implementation

The Rust version provides type-safe wrappers and validates against FIPS 197 Appendix B test vectors:

```
Key:       2b7e151628aed2a6abf7158809cf4f3c
Plaintext: 3243f6a8885a308d313198a2e0370734
Ciphertext: 3925841d02dc09fbdc118597196a0b32
```

The Rust code uses `u32` words internally for cleaner column operations but the logic is identical: SubBytes → ShiftRows → MixColumns → AddRoundKey.

See `code/main.rs` for the complete implementation.

## Use It

In production, you don't implement AES yourself — you use hardware-accelerated intrinsics:

**Linux kernel** (`arch/x86/crypto/aesni-intel_glue.c`) — uses AES-NI instructions. The kernel's AES doesn't compute S-boxes at runtime; it uses `AESENC`/`AESDEC` CPU instructions that perform an entire round in one cycle. The same kernel also has a pure-software fallback (`crypto/aes_generic.c`) for platforms without AES-NI.

**OpenSSL** (`crypto/aes/aes_core.c`) — the reference implementation uses the same lookup tables we built, plus T-tables that fuse SubBytes + ShiftRows + MixColumns into four 32-bit table lookups per column for speed. Each T-table has 256 entries of 32 bits (4 tables × 256 × 4 bytes = 4 KB). This is faster than computing each round operation individually, but it's vulnerable to cache-timing attacks — hence the preference for AES-NI.

**Ring** (Rust, used by BoringSSL) — uses AES-NI on x86 and the ARM equivalent (AES extension) on ARMv8. On platforms without hardware AES, it falls back to a constant-time bitsliced implementation that avoids lookup tables entirely.

Key differences from our implementation:

1. **T-tables**: Production AES fuses SubBytes + ShiftRows + MixColumns into precomputed 4 KB of tables. We compute each operation separately — correct but slow.
2. **Bitsliced implementations**: Some software AES runs 4 blocks in parallel using only XOR, AND, and shifts — no table lookups, no cache side channels.
3. **Hardware acceleration**: AES-NI makes the entire cipher ~4 cycles/block. Our software implementation takes hundreds of cycles.
4. **Constant-time**: Our key expansion and round functions leak timing through early exits in xtime/gf_mul. Production code must be constant-time to resist side channels.

## Read the Source

- `arch/x86/crypto/aesni-intel_glue.c` in the Linux kernel — see how `_aesni_encrypt` dispatches to the `AESENC` instruction. The glue code handles key scheduling and multi-block modes.
- `crypto/aes/aes_core.c` in OpenSSL — the `aes_encrypt` function shows the T-table optimization. Each call to `TE0[]` through `TE3[]` fuses four round operations.
- FIPS 197 (NIST SP 800-38A) — the authoritative specification. Sections 4–5 define every operation. Appendix B has the test vectors we validate against. Available at <https://csrc.nist.gov/pubs/fips/197/final>.

## Ship It

The reusable artifact for this lesson is in `outputs/`:

- **A standalone AES-128 library** — `aes128.h` / `aes128.c` that you can drop into any C project. Includes encrypt, decrypt, key expansion, and GF(2⁸) utilities. This will be reused in Lessons 05 (Modes of Operation) and 08 (AEAD).

## Exercises

1. **Easy** — Implement `gf_mul` from scratch without looking at the lesson code. Verify: `gf_mul(0x57, 0x83) == 0xC1` and compute `gf_mul(0x57, 0x13)`.
2. **Medium** — Modify your AES-128 implementation to support AES-256 (14 rounds, 256-bit key). This requires changing the key schedule and adding 4 extra rounds. Verify against FIPS 197 Appendix C.3.
3. **Hard** — Implement AES-128 using bitsliced representation (4 blocks in parallel, only XOR/AND/shift operations, no lookup tables). Benchmark against the T-table version. This is how constant-time software AES works in production.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Block cipher | "encrypts blocks" | A keyed permutation on a fixed-size block; every input maps to exactly one output and vice versa |
| Confusion | "mixing things up" | Nonlinear key-ciphertext relationship; S-boxes provide this |
| Diffusion | "spreading information" | Each plaintext bit affects many ciphertext bits; ShiftRows+MixColumns provide this |
| SPN | "like DES" | Substitution-Permutation Network — the structure AES uses; DES uses a Feistel network, not an SPN |
| S-box | "the lookup table" | Substitution box — nonlinear mapping built from GF(2⁸) inversion + affine transform |
| State | "the data" | The 4×4 byte matrix AES operates on; evolves through rounds |
| GF(2⁸) | "weird math" | Finite field of 256 elements where addition is XOR and multiplication is polynomial arithmetic modulo x⁸+x⁴+x³+x+1 |
| Key schedule | "key stretching" | Expanding the cipher key into round keys using RotWord, SubWord, and Rcon |
| Round key | "part of the key" | A 128-bit value XORed into the state at each round; derived from the key schedule |

## Further Reading

- [FIPS 197 — Advanced Encryption Standard](https://csrc.nist.gov/pubs/fips/197/final) — the definitive specification. Read Sections 4 and 5 first, then Appendix B for test vectors.
- [The Design of Rijndael](https://link.springer.com/book/10.1007/978-3-662-06561-3) by Joan Daemen and Vincent Rijmen — the creators explain every design choice.
- [A Stick Figure Guide to AES](https://www.moserware.com/2009/06/stick-figure-guide-to-advanced-encryption-standard-aes.html) — visual walkthrough of the full algorithm.
- [GF(2⁸) arithmetic](https://www.samiam.org/galois.html) — extensive worked examples of finite field operations.