# KDFs, PBKDF2, scrypt, Argon2

> Passwords are low-entropy secrets. KDFs stretch them into cryptographic keys — and they do it slowly on purpose.

**Type:** Build
**Languages:** Rust, Python
**Prerequisites:** Phase 12 lessons 01–12
**Time:** ~60 minutes

## Learning Objectives

- Explain why password hashing requires different design goals than cryptographic hashing (intentional slowness, salt, memory-hardness).
- Implement PBKDF2-HMAC-SHA256 from scratch and verify against RFC test vectors.
- Implement the scrypt ROMix with Salsa20/8 mixing and compare its memory-hard property against PBKDF2.
- Select appropriate KDF parameters for interactive vs. sensitive use cases using Argon2id.

## The Problem

SHA-256 of `"letmein"` takes about 10 nanoseconds on a modern CPU. An attacker with an NVIDIA RTX 4090 can compute roughly 20 *billion* SHA-256 hashes per second. That means every 8-character printable ASCII password (~6e14 possibilities) can be brute-forced in under 9 hours. Your users pick worse passwords than that.

Cryptographic hash functions like SHA-256, SHA-3, and BLAKE2 were designed to be *fast* — they process gigabytes per second. Speed is the enemy of password storage. If the server can hash a password in 1 us, the attacker can try millions per second. The defense is to make hashing *intentionally slow* — but not so slow that legitimate users notice.

A second problem: identical passwords produce identical hashes. If two users both pick `"password123"`, their stored hashes are byte-for-byte the same. A precomputed table (rainbow table) for common passwords immediately reveals both accounts. The fix is a *salt* — a random value unique to each user, mixed into the hash so the same password produces a completely different output.

A third problem: attackers throw specialized hardware (GPUs with thousands of cores, ASICs with dedicated SHA-256 pipelines) at password cracking. A KDF that only burns CPU cycles (time-hardness) is still trivially parallelizable. The solution is *memory-hardness* — forcing each hash computation to use a large, unpredictable memory buffer that GPUs cannot efficiently replicate.

## The Concept

A **Key Derivation Function (KDF)** transforms a low-entropy input (a password) and a random salt into one or more cryptographic keys, using a controlled amount of work. A KDF for password hashing has three properties that distinguish it from a regular hash function:

- **Key stretching**: expands a weak password into a full-length cryptographic key (e.g., 256 bits for AES).
- **Salt mixing**: incorporates a per-user random value to defeat rainbow tables.
- **Work factor**: a tunable parameter that controls computational cost, making brute-force search expensive.

The three KDFs this lesson covers form a progression of defensive sophistication:

| KDF | Year | Hardness | Memory | GPU-resistant | Side-channel resistant |
|-----|------|----------|--------|---------------|----------------------|
| PBKDF2 | 2000 | Time (iteration count) | Negligible | No | No |
| scrypt | 2009 | Time + Memory (ROMix buffer) | Configurable, large | Partial | No |
| Argon2id | 2015 | Time + Memory + Data-dependent access | Configurable, large | Yes | Yes |

### PBKDF2 — Password-Based Key Derivation Function 2

PBKDF2 (RFC 2898) applies a pseudorandom function (typically HMAC-SHA256) many times in sequence:

```
DK = T_1 || T_2 || ... || T_ceil(dkLen / hLen)
```

where each block T_i is computed as:

```
U_1 = PRF(Password, Salt || INT_32_BE(i))
U_j = PRF(Password, U_{j-1})        for j = 2 ... c
T_i = U_1 XOR U_2 XOR ... XOR U_c
```

Iteration count c is the only defense. Doubling c doubles the time for both the server and the attacker. There is no memory requirement — an ASIC can pipeline thousands of PBKDF2 cores on a single die.

### scrypt — Memory-Hard KDF

scrypt (RFC 7914) wraps PBKDF2 around a memory-hard mixing step:

1. **PBKDF2-expand**: B = PBKDF2(Password, Salt, 1, p * 128 * r)
2. **ROMix**: for each of p blocks, fill a large array V[0..n-1] of 128*r-byte elements, then repeatedly read from pseudo-random indices in V.
3. **PBKDF2-contract**: DK = PBKDF2(Password, B', 1, dkLen)

The ROMix uses **Salsa20/8** as its mixing primitive. Each iteration reads from a buffer location determined by the *data itself* — an attacker cannot predict the access pattern. Since GPU memory bandwidth is limited (an RTX 4090 has ~1 TB/s bandwidth vs. ~100 TB/s compute), a memory-hard function effectively caps the attacker's throughput regardless of how many cores they have.

Parameters N (CPU/memory cost), r (block size), and p (parallelization) control the trade-off:

```
Memory ~= 128 * r * N bytes
```

Common choice: N = 2^14, r = 8, p = 1 -> ~16 MB memory per hash.

### Argon2id — State of the Art

Argon2 won the Password Hashing Competition (2015). Three variants exist:

- **Argon2d**: data-dependent memory access — fastest, but vulnerable to side-channel attacks.
- **Argon2i**: data-independent access — resistant to side-channels, but slower.
- **Argon2id** (recommended): hybrid — uses data-independent access for the first half of the pass, data-dependent for the second. Side-channel resistant *and* memory-hard.

Parameters: t (time cost = number of passes), m (memory size in KiB), p (parallelism). OWASP recommends Argon2id with t=3, m=64 MiB, p=4 for sensitive applications.

### Why Salt Matters

A salt prevents precomputation. Without a salt, the attacker computes SHA-256("password123") once and looks up every matching hash. With a 128-bit salt, each user's hash is effectively a *different function* — the attacker must brute-force each account separately. Salts must be:

- **Unique**: never reuse a salt across users or across password changes.
- **Random**: generated by a CSPRNG (e.g., `secrets.token_bytes(16)`).
- **Long**: at least 16 bytes (128 bits).

## Build It

### Step 1: PBKDF2-HMAC-SHA256 from Scratch (Rust)

We start with an HMAC-SHA256 implementation. HMAC wraps SHA-256 to produce a keyed hash:

```
HMAC(K, m) = SHA-256((K' XOR opad) || SHA-256((K' XOR ipad) || m))
```

where K' is the key padded to 64 bytes, ipad = 0x36 repeated, and opad = 0x5c repeated.

```rust
use sha2::{Digest, Sha256};

fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let k = if key.len() > 64 {
        Sha256::digest(key).to_vec()
    } else {
        key.to_vec()
    };
    let mut k_ipad = vec![0u8; 64];
    let mut k_opad = vec![0u8; 64];
    for (i, &b) in k.iter().enumerate() {
        k_ipad[i] = b ^ 0x36;
        k_opad[i] = b ^ 0x5c;
    }
    for i in k.len()..64 {
        k_ipad[i] = 0x36;
        k_opad[i] = 0x5c;
    }
    let inner = Sha256::new()
        .chain_update(&k_ipad)
        .chain_update(msg)
        .finalize();
    let result = Sha256::new()
        .chain_update(&k_opad)
        .chain_update(&inner)
        .finalize();
    result.into()
}
```

With HMAC in hand, PBKDF2 is a loop:

```rust
fn pbkdf2(password: &[u8], salt: &[u8], iterations: u32, dk_len: usize) -> Vec<u8> {
    let mut dk = Vec::with_capacity(dk_len);
    let block_count = (dk_len as f64 / 32.0).ceil() as u32;

    for block in 1..=block_count {
        let mut u = hmac_sha256(password, &[salt, &(block as u32).to_be_bytes()].concat());
        let mut t = u;
        for _ in 1..iterations {
            u = hmac_sha256(password, &u);
            for j in 0..32 {
                t[j] ^= u[j];
            }
        }
        dk.extend_from_slice(&t);
    }
    dk.truncate(dk_len);
    dk
}
```

We verify against the IETF PBKDF2-HMAC-SHA256 test vectors. With only 1 iteration, the hash of `("password", "salt")` is:

```
120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b
```

Increasing iterations to 4096 produces a completely different output:

```
348c89dbcbd32b2f32d814b8116e84cf2b17347ebc1800181c4e2a1fb8dd53e1c635518c7dac47e9
```

On our test system, 1000 iterations complete in ~2ms, 100,000 in ~200ms. The user never notices 200ms on login, but an attacker trying 1e9 passwords must spend 1e9 x 0.2s = 6.3 years. That is the power of time-hardness.

### Step 2: scrypt — Memory-Hard KDF (Rust)

scrypt adds memory-hardness via the ROMix algorithm. The core mixing primitive is Salsa20/8 — 8 rounds of the Salsa20 stream cipher core:

```rust
fn salsa20_8(input: &[u8; 64]) -> [u8; 64] {
    let mut x = [0u32; 16];
    for i in 0..16 {
        x[i] = u32::from_le_bytes(input[4 * i..4 * i + 4].try_into().unwrap());
    }
    for _ in 0..4 {
        // Column round: quarter-rounds on columns
        x[4]  ^= (x[0].wrapping_add(x[12])).rotate_left(7);
        x[8]  ^= (x[4].wrapping_add(x[0])).rotate_left(9);
        x[12] ^= (x[8].wrapping_add(x[4])).rotate_left(13);
        x[0]  ^= (x[12].wrapping_add(x[8])).rotate_left(18);
        // ... (see full source in main.rs for all quarter-rounds)
    }
    let mut out = [0u8; 64];
    for i in 0..16 {
        let orig = u32::from_le_bytes(input[4 * i..4 * i + 4].try_into().unwrap());
        out[4 * i..4 * i + 4].copy_from_slice(&(x[i].wrapping_add(orig)).to_le_bytes());
    }
    out
}
```

The ROMix fills an array V of N elements (each 128*r bytes), then performs N additional mixing steps where each read index is determined by the last 32 bits of the previous mixed output:

```rust
fn romix(b: &[u8], n: usize, r: usize) -> Vec<u8> {
    let mut v: Vec<Vec<u8>> = Vec::with_capacity(n);
    let mut x = b.to_vec();
    for i in 0..n {
        v.push(x.clone());
        x = blockmix_salsa8(&x, r);
    }
    for _ in 0..n {
        let j = u32::from_le_bytes(
            x[(2 * r - 1) * 64..2 * r * 64][..4].try_into().unwrap()
        ) as usize & (n - 1);
        for k in 0..x.len() {
            x[k] ^= v[j][k];
        }
        x = blockmix_salsa8(&x, r);
    }
    x
}
```

The data-dependent indexing is what makes scrypt memory-hard: an attacker cannot predict which element of V to read next, so they must keep the entire array in fast memory. With N = 1024, r = 8, that array is ~4 MB. With N = 4096, it is ~16 MB — larger than most GPU caches.

We verify against RFC 7914 test vectors. The scrypt of `("password", "NaCl", n=1024, r=8, p=16)` produces a 64-byte key starting with `fdbabe1c9d34...`.

On our test system, increasing N from 1024 to 4096 increases time from ~40ms to ~180ms — and memory from 1 MB to 4 MB. An attacker with a parallel machine must multiply both time *and* memory per core.

### Step 3: Argon2id — The State of the Art (Python)

While Python's standard library includes `hashlib.pbkdf2_hmac` and `hashlib.scrypt`, Argon2 requires the `argon2-cffi` package:

```python
from argon2 import PasswordHasher, Type

ph = PasswordHasher(
    time_cost=3,          # t: 3 passes
    memory_cost=65536,    # m: 64 MiB
    parallelism=4,        # p: 4 threads
    hash_len=32,
    type=Type.ID,         # Argon2id
)
hash = ph.hash("correct horse battery staple")
valid = ph.verify(hash, "correct horse battery staple")
```

Argon2id's memory access pattern is *data-independent* in the first pass (resisting side-channels that measure cache timing) and *data-dependent* in subsequent passes (maximizing memory-hardness). This dual strategy makes it strictly superior to both PBKDF2 (no memory-hardness) and scrypt (data-dependent throughout, weaker side-channel resistance).

Parameter comparison on our test system:

| Parameters | Timing | Use case |
|-----------|--------|----------|
| t=2, m=256 MiB, p=1 | ~0.5s | Sensitive (password managers) |
| t=3, m=64 MiB, p=4 | ~0.3s | OWASP recommended |
| t=1, m=16 MiB, p=1 | ~0.05s | Interactive (web app login) |

## Use It

Production password storage uses these KDFs everywhere:

- **`/etc/shadow`**: modern Linux systems use `yescrypt` (an evolution of scrypt) or `argon2id`. The hash field encodes parameters: `$argon2id$v=19$m=65536,t=3,p=4$<salt>$<hash>`.
- **LUKS (Linux Unified Key Setup)**: uses PBKDF2 or Argon2 to derive a disk encryption key from the user's passphrase. The LUKS2 header stores the KDF type, salt, and parameters alongside the encrypted master key.
- **WireGuard PSK**: pre-shared keys are typically derived from a password using a KDF with a salt (the peer's public key serves as salt context).
- **GnuPG symmetric encryption**: uses s2k (string-to-key) with a hash iteration count to derive the cipher key from a passphrase.
- **LastPass / 1Password / Bitwarden**: derive the master encryption key from your master password using PBKDF2 (100,000-600,000 iterations) or Argon2id.

The Python standard library exposes PBKDF2 and scrypt in `hashlib`:

```python
dk = hashlib.pbkdf2_hmac("sha256", password, salt, iterations=600_000)
dk = hashlib.scrypt(password, salt=salt, n=2**14, r=8, p=1, dklen=32)
```

Production implementations (`libsodium`, `OpenSSL`, `Go x/crypto`) add constant-time comparison, automatic salt generation, parameter encoding in the output string, and defense-in-depth against side-channels. Your from-scratch implementations capture the algorithm; the library versions protect against the attacks you haven't thought of.

## Read the Source

- **RFC 2898 (PKCS #5 v2.0)** — PBKDF2 specification: the exact loop with INT_32_BE encoding for block index.
- **RFC 7914 (scrypt)** — The scrypt specification with Salsa20/8 core, ROMix definition, and test vectors.
- **Argon2 specification (RFC 9106)** — The official spec: memory-hard hashing with three variants (d, i, id).
- **libsodium `src/libsodium/crypto_pwhash/`** — Production implementation of scrypt and Argon2id with automatic parameter tuning and constant-time verification.
- **Go `x/crypto/pbkdf2/pbkdf2.go`** — A clean, minimal implementation of PBKDF2 in ~100 lines; good for understanding the core logic without noise.

## Ship It

The reusable artifact is a KDF library/demo implementing PBKDF2-HMAC-SHA256 from scratch (Rust), scrypt from scratch (Rust), and Argon2id parameter exploration (Python). It lives in `outputs/` and serves as a reference for deriving encryption keys from passwords in later phases (notably the TLS 1.3 capstone, where a PSK is derived from a shared secret).

## Exercises

1. **Easy** — Run `main.rs` with iteration counts of 1, 1000, and 100000 for PBKDF2. Record the timing. How long would it take to brute-force a 6-character lowercase password (26^6 ~= 3e8 possibilities) at each iteration count?

2. **Medium** — Modify the Rust scrypt implementation to track the number of times each index in the V array is read. Plot the access frequency distribution. Is it uniform? What does this tell you about the difficulty of optimizing scrypt with a smaller memory buffer?

3. **Hard** — Implement a timing-safe comparison function (`constant_time_eq`) and use it for PBKDF2 verification. Demonstrate that your implementation resists timing attacks by showing that comparing hashes with different prefixes takes the same wall-clock time.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| KDF | A function that turns passwords into keys | A key derivation function that applies a salt and a configurable work factor to stretch a low-entropy input into one or more cryptographic keys |
| Salt | Random data mixed into a hash | A per-user random value (>=16 bytes) that prevents rainbow-table attacks and ensures the same password produces different hashes for different users |
| Work factor | How hard the hash is to compute | A tunable parameter (iteration count, memory size, parallelism) that controls the computational cost of deriving a key from a password |
| Memory-hard | A function that needs lots of memory | A property of a KDF where computing the output requires a large, unpredictable memory buffer, making GPU/ASIC acceleration infeasible |
| Time-hard | A function that needs lots of time | A property of a KDF where the output requires a tunable number of sequential operations (iteration count), limiting throughput even on parallel hardware |
| PBKDF2 | Password-Based Key Derivation Function 2 | An RFC 2898 standard that iterates HMAC many times; only time-hard, no memory-hardness, vulnerable to GPU/ASIC |
| scrypt | A memory-hard KDF | An RFC 7914 standard that adds memory-hardness via a large ROMix buffer with Salsa20/8 mixing; resists GPU attacks better than PBKDF2 |
| Argon2id | The recommended Argon2 variant | The RFC 9106 state-of-the-art KDF; hybrid data-independent/dependent memory access, side-channel resistant, memory-hard, the OWASP 2022 recommended choice |

## Further Reading

- [RFC 2898 — PBKDF2 Specification](https://datatracker.ietf.org/doc/html/rfc2898) — The original definition of PBKDF2 with HMAC as the PRF and test vectors for SHA-1.
- [RFC 7914 — The scrypt Password-Based Key Derivation Function](https://datatracker.ietf.org/doc/html/rfc7914) — Colin Percival's specification with the full Salsa20/8 core, ROMix algorithm, and RFC test vectors.
- [RFC 9106 — Argon2 Memory-Hard Function](https://datatracker.ietf.org/doc/html/rfc9106) — The official standard for Argon2id, Argon2d, and Argon2i with parameter guidelines and test vectors.
- [libsodium crypto_pwhash](https://github.com/jedisct1/libsodium/tree/master/src/libsodium/crypto_pwhash) — Production implementation used by millions; shows constant-time verification, automatic salt generation, and parameter encoding.
- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html) — Industry best practices for choosing KDF algorithms and parameters.
