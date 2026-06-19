# Hash Functions — SHA-2, SHA-3, BLAKE

> A hash function turns any input into a fixed-size fingerprint. If the fingerprint is unpredictable and collision-free, you can build everything else: digital signatures, MACs, commitment schemes, Merkle trees.

**Type:** Learn
**Languages:** C, Python
**Prerequisites:** Phase 12 lessons 01–05
**Time:** ~75 minutes

## Learning Objectives

- Define the three security properties of a cryptographic hash function (preimage resistance, second preimage resistance, collision resistance) and explain why collision resistance implies the other two but not vice versa.
- Derive the birthday attack bound (2^(n/2) hash evaluations for an n-bit hash) and calculate the collision resistance of SHA-256 (128 bits).
- Explain the Merkle-Damgård construction, demonstrate the length extension attack, and describe how HMAC and the sponge construction defeat it.
- Walk through SHA-256's internals (padding, block processing, 64-round compression with Ch, Maj, Σ0, Σ1, K constants).
- Explain the Keccak sponge construction (absorb, squeeze) and its permutation-based design (θ, ρ, π, χ, ι on a 5×5×64 state).
- Compare BLAKE2's performance and security trade-offs against SHA-2 and SHA-3.
- Implement SHA-256 from scratch in C and verify against official test vectors.

## The Problem

You're building a system that stores user passwords, signs software updates, and verifies file integrity. For all three you need a function that maps arbitrary-length inputs to fixed-size outputs — but not just any function. If an attacker can find two inputs with the same output, they can forge a signature. If they can reverse the output, they can recover a password. If they can append data to a message and compute the new hash from the old hash alone, they can tamper with authenticated messages.

Cryptographic hash functions are the answer, but picking the wrong one (MD5, SHA-1) or using the right one incorrectly (raw SHA-256 without HMAC for message authentication) leads to catastrophic failures. The 2017 SHAttered attack on SHA-1 cost ~$110,000 in compute; MD5 collisions can be generated on a laptop in seconds. Understanding *how* these functions work — and *why* some designs are vulnerable — is essential for the TLS 1.3 capstone where hashes underpin every handshake step.

## The Concept

### Three Security Properties

A cryptographic hash function H maps arbitrary-length inputs to a fixed n-bit output:

```
H: {0,1}* → {0,1}^n
```

Three properties, in order of decreasing strength:

| Property | Definition | Security level |
|----------|-----------|---------------|
| **Collision resistance** | Hard to find any m₁ ≠ m₂ with H(m₁) = H(m₂) | 2^(n/2) effort (birthday bound) |
| **Second preimage resistance** | Given m₁, hard to find m₂ ≠ m₁ with H(m₁) = H(m₂) | 2^n effort |
| **Preimage resistance** | Given h, hard to find any m with H(m) = h | 2^n effort |

Collision resistance is the strongest property. If you can't find *any* collision, you certainly can't find a second preimage for a *given* input. But collision resistance is also the easiest to attack: the birthday attack means you only need 2^(n/2) evaluations, not 2^n.

```
Collision resistance     2^(n/2)  ←  birthday bound
    ┊ implies
Second preimage resist.  2^n
    ┊ implies
Preimage resistance      2^n
```

### The Birthday Attack

The name comes from the birthday paradox: in a room of 23 people, there's a >50% chance two share a birthday. Generalized: given a set of N possible values, you need only ~√N random samples before you expect a duplicate.

For an n-bit hash (2^n possible outputs), a collision is expected after ~2^(n/2) random evaluations:

- MD5 (128-bit): collisions in ~2^64 — feasible since 2004
- SHA-1 (160-bit): collisions in ~2^80 — demonstrated in SHAttered (2017)
- SHA-256 (256-bit): collisions in ~2^128 — infeasible
- SHA-512 (512-bit): collisions in ~2^256 — wildly infeasible

**SHA-256 has 128-bit collision resistance** (not 256-bit). This is fundamental. When you choose a hash, the birthday bound halves the bit security for collisions.

### Merkle-Damgård Construction

The construction used by MD5, SHA-1, and SHA-256. The idea: build a hash from a compression function f that takes a fixed-size input and produces a fixed-size output.

```
          ┌──────┐      ┌──────┐      ┌──────┐
  IV ────►│  f   │──┬──►│  f   │──┬──►│  f   │──┬──► H(m)
          └──┬───┘  │    └──┬───┘  │    └──┬───┘  │
             │      │       │      │       │      │
  m₁ ───────┘      │  m₂ ──┘      │  m₃ ──┘      │
                    │               │               │
                 pad(m₁)          pad(m₂)       pad(m₃)
```

Step by step:

1. **Pad the message**: append a 1-bit, then enough 0-bits, then the 64-bit message length so the total length is a multiple of the block size (512 bits for SHA-256).
2. **Split into blocks**: m₁, m₂, ..., m_k.
3. **Iterate**: h₀ = IV, hᵢ = f(hᵢ₋₁, mᵢ) for i = 1..k.
4. **Output**: H(m) = h_k.

The padding ensures that messages of different length hash differently. The chaining ensures each block's output depends on all previous blocks.

### Length Extension Attack

Merkle-Damgård has a structural flaw. Given H(m) and len(m), you can compute H(m ‖ pad(m) ‖ extension) *without knowing m*. Here's why:

```
H(m) = f(f(f(IV, m₁), m₂), m₃)

H(m ‖ pad ‖ extension) = f(f(f(f(IV, m₁), m₂), m₃), m₄)
                       = f(H(m), m₄)
```

The attacker sets IV to H(m), pads the extension, and continues compression. This breaks naive `H(secret ‖ message)` constructions for authentication.

**Fixes**:
- **HMAC**: H(key ‖ H(key ‖ message)) — inner and outer hashing
- **SHA-3 sponge**: the sponge construction makes length extension impossible

### SHA-256 Internals

SHA-256 processes messages in 512-bit blocks. The state is eight 32-bit words (a, b, c, d, e, f, g, h), initialized to specific constants:

```
Initial hash values (first 32 bits of the fractional parts of the square roots of the first 8 primes):
  h₀ = 6a09e667  h₁ = bb67ae85  h₂ = 3c6ef372  h₃ = a54ff53a
  h₄ = 510e527f  h₅ = 9b05688c  h₆ = 1f83d9ab  h₇ = 5be0cd19
```

**Message schedule**: expand 16 words (from the 512-bit block) into 64 words:

```
W[t] = M[t]                                          for 0 ≤ t ≤ 15
W[t] = σ₁(W[t-2]) + W[t-7] + σ₀(W[t-15]) + W[t-16]  for 16 ≤ t ≤ 63

where:
  σ₀(x) = ROTR(x,7) ⊕ ROTR(x,18) ⊕ SHR(x,3)
  σ₁(x) = ROTR(x,17) ⊕ ROTR(x,19) ⊕ SHR(x,10)
```

**Compression** — 64 rounds. Each round uses:

```
T₁ = h + Σ₁(e) + Ch(e,f,g) + K[t] + W[t]
T₂ = Σ₀(a) + Maj(a,b,c)

where:
  Ch(x,y,z)  = (x ∧ y) ⊕ (¬x ∧ z)
  Maj(x,y,z) = (x ∧ y) ⊕ (x ∧ z) ⊕ (y ∧ z)
  Σ₀(x)      = ROTR(x,2) ⊕ ROTR(x,13) ⊕ ROTR(x,22)
  Σ₁(x)      = ROTR(x,6) ⊕ ROTR(x,11) ⊕ ROTR(x,25)
```

Round update:

```
h = g
g = f
f = e
e = d + T₁
d = c
c = b
b = a
a = T₁ + T₂
```

After 64 rounds, add the working variables to the current hash state. The K constants are the first 64 bits of the fractional parts of the cube roots of the first 64 primes.

**Padding**:

```
message ‖ 0x80 ‖ 0x00...0x00 ‖ length_in_bits_as_64-bit_big_endian
```

Total padded length must be a multiple of 512 bits.

### SHA-3 (Keccak) — The Sponge Construction

SHA-3 uses a fundamentally different approach. Instead of a compression function, Keccak uses a **sponge** built on a permutation.

```
       ┌──────────────────────────────────┐
       │    state: 5×5×64 = 1600 bits     │
       │  ┌────────────┬─────────────────┐ │
       │  │   rate r   │  capacity c     │ │
       │  │  1088 bits │   512 bits      │ │
       │  └────────────┴─────────────────┘ │
       └──────────────────────────────────┘

Absorb:  state ⊕= message_block (padded), then apply Keccak-f[1600]
                       ↑ only the rate bits are XORed
Squeeze:  read r bits from state, apply Keccak-f[1600], repeat
```

**Why it resists length extension**: After absorption, the capacity bits (which the attacker never sees) are indistinguishable from random. To extend the hash, the attacker would need to guess the capacity — which requires 2^c work. SHA-3-256 uses c = 512, so this is infeasible.

The Keccak-f[1600] permutation operates on a 5×5×64-bit state and consists of 24 rounds, each applying five steps:

| Step | Name | Operation |
|------|------|-----------|
| θ (theta) | Column parity mixing | XOR each bit with parities of two columns |
| ρ (rho) | Bit rotation | Rotate each lane by a fixed offset |
| π (pi) | Lane permutation | Rearrange lane positions |
| χ (chi) | Nonlinear mixing | The only nonlinear step: `x ⊕ (¬y ∧ z)` |
| ι (iota) | Round constant XOR | Break symmetry |

The π step rearranges lanes: `A'[y, 2x+3y] = A[x, y]`. The ρ step rotates lane (x,y) by `r(x,y)` where the offsets follow a specific pattern starting from (0,1) offset 1. The only nonlinear operation is χ, which is why Keccak needs all five steps — χ alone isn't enough for diffusion.

### BLAKE2

BLAKE2 (2012) is a hash function inspired by the ChaCha stream cipher. Key advantages:

- **Faster than MD5** on modern CPUs (uses SIMD-friendly ARX operations)
- **More secure** than SHA-2 (no length extension, larger internal state)
- **Two variants**: BLAKE2s (256-bit, 32-bit words) and BLAKE2b (512-bit, 64-bit words)

The core of BLAKE2 is a **ChaCha quarter round**:

```
a = a + b;  d ^= a;  d >>>= 16
c = c + d;  b ^= c;  b >>>= 12
a = a + b;  d ^= a;  d >>>= 8
c = c + d;  b ^= c;  b >>>= 7
```

BLAKE2 also supports **tree-hashing mode** for parallel computation — you can hash subtrees independently and combine them, making it efficient for multi-core systems and large files.

### Comparison Table

| Hash | Output | Collision resistance | Speed | Status |
|------|--------|---------------------|-------|--------|
| MD5 | 128-bit | ~2^64 (broken in practice) | Fast | **Broken** — practical collisions |
| SHA-1 | 160-bit | ~2^80 (broken in 2017) | Medium | **Broken** — SHAttered attack |
| SHA-256 | 256-bit | 2^128 | Medium | **Recommended** — current standard |
| SHA-512 | 512-bit | 2^256 | Fast on 64-bit | Recommended for 64-bit platforms |
| SHA-3-256 | 256-bit | 2^128 | Slower than SHA-2 | **Alternative** — no length extension |
| BLAKE2b | 512-bit | 2^256 | Very fast | **Fast + secure** — modern choice |
| BLAKE2s | 256-bit | 2^128 | Very fast | Fast + secure for 32-bit platforms |

## Build It

### Step 1: SHA-256 in C

Open `code/main.c`. This implements the complete SHA-256 algorithm from scratch — padding, message schedule, 64-round compression, and test vector verification.

The key data structures:

```
State:      uint32_t H[8]     — eight 32-bit working variables
Block:      uint8_t block[64] — one 512-bit message block
Schedule:   uint32_t W[64]   — expanded message schedule
```

The implementation verifies against two official NIST test vectors:
- `SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
- `SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad`

Compile and run:

```bash
gcc -Wall -Wextra -o sha256 code/main.c
./sha256
```

### Step 2: SHA-3 Sponge, BLAKE2, Birthday Attack, Length Extension in Python

Open `code/main.py`. This covers:

1. **SHA-256 from scratch** (Python, for educational clarity)
2. **SHA-3/Keccak sponge** — absorb message blocks, apply Keccak-f permutation, squeeze output
3. **BLAKE2b demo** — using hashlib
4. **Birthday attack simulation** — find collisions in a truncated 24-bit SHA-256 hash
5. **Length extension attack demo** — given H(m) and len(m), compute H(m ‖ pad ‖ extension) without knowing m
6. **Benchmark comparison** — SHA-256 vs SHA-3-256 vs BLAKE2b on 1MB of data

Run:

```bash
python3 code/main.py
```

## Use It

### Production Usage

In Python, use `hashlib` — it wraps OpenSSL's highly optimized implementations:

```python
import hashlib
h = hashlib.sha256(b"message").hexdigest()
```

In C (Linux), use OpenSSL's EVP interface:

```c
#include <openssl/evp.h>
EVP_MD_CTX *ctx = EVP_MD_CTX_new();
EVP_DigestInit_ex(ctx, EVP_sha256(), NULL);
EVP_DigestUpdate(ctx, data, len);
EVP_DigestFinal_ex(ctx, hash, &hash_len);
```

In Go, use `crypto/sha256`. In Rust, use the `sha2` crate. Every standard library implements SHA-256.

### What Production Does Differently

Your C implementation is correct but ~100× slower than OpenSSL. Production implementations use:

- **SIMD** (AVX2, NEON): process 4–8 blocks in parallel
- **Loop unrolling**: all 64 rounds unrolled
- **Assembly**: hand-written SHA-NI instructions on modern Intel/AMD CPUs (SHA-256 computation in hardware, ~3 cycles/byte)

OpenSSL's `crypto/sha/sha256.c` is the reference. For BLAKE2, the reference implementation at `https://github.com/BLAKE2/BLAKE2` includes optimized SIMD paths.

For SHA-3, the NIST FIPS 202 document is the definitive reference. The Keccak team's implementation at `https://keccak.team` includes optimized variants.

## Read the Source

- **Linux kernel SHA-256**: `lib/crypto/sha256.c` — minimal, clean C implementation used for module signing and integrity checks
- **OpenSSL SHA-256**: `crypto/sha/sha256.c` — production-grade with assembly acceleration
- **Keccak reference**: `https://keccak.team/files/KeccakReference-3.0.pdf` — the specification
- **BLAKE2 reference**: `https://github.com/BLAKE2/BLAKE2/blob/ref10/blake2b-ref.c` — stand-alone reference C implementation

## Ship It

The reusable artifact from this lesson:

- **`outputs/sha256.c`** — A self-contained, dependency-free SHA-256 implementation you can drop into any C project. Compiles with `gcc -Wall -Wextra -o sha256 sha256.c`.

## Exercises

1. **Easy** — Modify the SHA-256 implementation to produce SHA-224 (truncate to 224 bits, use different initial hash values per the spec).
2. **Medium** — Implement the birthday attack: find a collision in a 40-bit truncated SHA-256 hash. Time how many hash evaluations it takes on average.
3. **Hard** — Implement the Keccak-f[1600] permutation from scratch. Verify your implementation produces correct SHA-3-256 test vectors. Then implement parallel tree-hashing with BLAKE2bp.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Hash function | "It encrypts data" | A deterministic function mapping arbitrary input to fixed-size output; it's not encryption — no key, no decryption |
| Collision | "Two hashes happen to match" | Two distinct inputs producing the same output; statistically inevitable by the birthday bound, but must be computationally infeasible to find |
| Birthday attack | "Attack on my birthday" | Exploiting the birthday paradox to find collisions in ~2^(n/2) rather than 2^n evaluations |
| Merkle-Damgård | "The hash chain thing" | An iterative construction that processes message blocks through a compression function, chaining state from block to block |
| Length extension | "Appending data to a hash" | Given H(m) and len(m), an attacker can compute H(m ‖ pad(m) ‖ x) without knowing m — a structural flaw of Merkle-Damgård |
| Sponge | "The absorb-squeeze thing" | A permutation-based construction where message blocks are XORed into the state (absorb), then output is read (squeeze); Keccak/SHA-3 |
| Preimage resistance | "Can't invert the hash" | Given h, it's computationally infeasible to find any m with H(m) = h; requires ~2^n work for an n-bit hash |

## Further Reading

- [FIPS 180-4: SHA-256 Standard](https://csrc.nist.gov/publications/detail/fips/180/4/final) — The definitive specification for SHA-256
- [FIPS 202: SHA-3 Standard](https://csrc.nist.gov/publications/detail/fips/202/final) — The SHA-3 / Keccak specification
- [Keccak Team Reference](https://keccak.team/) — Official site with reference implementations and analysis
- [BLAKE2 Specification](https://www.blake2.net/) — Official BLAKE2 site with benchmarks and implementations
- [SHAttered Collision Attack](https://shattered.io/) — The first practical SHA-1 collision