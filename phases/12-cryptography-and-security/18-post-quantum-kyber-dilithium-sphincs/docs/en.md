# Post-Quantum — Kyber, Dilithium, SPHINCS+

> Quantum computers break RSA and ECC. These three algorithms don't rely on the same assumptions — and will be your fallback when they fall.

**Type:** Learn
**Languages:** Rust
**Prerequisites:** Phase 12 lessons 01–17
**Time:** ~60 minutes

## Learning Objectives

- Understand why Shor's algorithm breaks RSA, DSA, ECDH, and Ed25519, and what "harvest now, decrypt later" means.
- Explain the three NIST PQC families: Kyber (ML-KEM), Dilithium (ML-DSA), SPHINCS+ (SLH-DSA).
- Run working Rust demos of Kyber key encapsulation, Dilithium signatures, and SPHINCS+ signatures.
- Compare PQC schemes with classical schemes on key size, signature/ciphertext size, and security assumptions.

## The Problem

Shor's algorithm, running on a sufficiently large fault-tolerant quantum computer, solves the integer factorization problem and the discrete logarithm problem in polynomial time. A quantum computer with roughly 4000 logical qubits could factor a 2048-bit RSA modulus in hours. The same algorithm breaks DSA, ECDSA, Ed25519, Diffie-Hellman, and ECDH — essentially all deployed public-key cryptography.

This is not a hypothetical concern. The number of qubits in quantum processors roughly doubles every two years (similar to classical Moore's Law). While current systems have a few hundred physical qubits with high error rates, logical qubit technology and error correction are advancing rapidly. NIST's PQC standardization timeline (2016–2024) was driven by the need to have standards ready well before the quantum threat materializes.

**"Harvest now, decrypt later" attacks** change the calculus further. An adversary records encrypted TLS traffic today, stores it on cheap long-term storage, and decrypts it retroactively once a quantum computer exists. Data encrypted with classical public-key crypto in 2025 could be readable in 2040. Governments, financial institutions, and healthcare organizations routinely handle data that must remain confidential for 20+ years. For these use cases, the migration to post-quantum cryptography needs to happen *before* the quantum computer arrives, not after.

Every protocol you've built in this phase — TLS handshakes, certificate chains, authenticated encryption — depends on public-key primitives that Shor's algorithm breaks. The Phase 12 capstone (TLS 1.3 implementation) must support PQC key exchange to be future-proof.

## The Concept

**Post-quantum cryptography (PQC)** refers to cryptographic algorithms believed secure against both classical and quantum computers. After an 8-year competition (2016–2024), NIST selected three families for standardization:

### Kyber (ML-KEM — FIPS 203)

A **key encapsulation mechanism (KEM)** based on the Module Learning With Errors (MLWE) problem. The sender encapsulates a shared secret using the recipient's public key; the recipient decapsulates using their secret key. Security rests on the hardness of recovering a secret vector from noisy linear equations over a module lattice.

- Ciphertext: ~800 bytes (Kyber-512) to ~1.6 KB (Kyber-1024)
- Public key: ~800 bytes to ~1.6 KB
- Compared to X25519 (32-byte public key, 32-byte shared secret), Kyber is 25–50× larger

### Dilithium (ML-DSA — FIPS 204)

A **digital signature scheme** also based on MLWE — the same lattice assumption as Kyber. This design reuse means a cryptanalytic break of MLWE would affect both, but it also means implementations can share engineering effort.

- Signature: ~2.4 KB (Dilithium2) to ~4.6 KB (Dilithium5)
- Public key: ~1.3 KB to ~2.6 KB
- Compared to Ed25519 (64-byte signature), Dilithium signatures are 38–72× larger

### SPHINCS+ (SLH-DSA — FIPS 205)

A **stateless hash-based signature scheme**. Its security depends *only* on the security of the underlying hash function (SHA-256 or SHAKE) — no lattice assumptions, no number-theoretic assumptions. This makes it the most conservative choice: as long as hash functions are secure, SPHINCS+ is secure. The cost is large signatures and slow signing.

- Signature: 8 KB (128s) to 50 KB (256f)
- Public key: 32 bytes (same as Ed25519!)
- Signing is 10–100× slower than verification due to many Merkle tree traversals

### How LWE Works (High-Level)

Learning With Errors starts with a matrix **A** of random values, a secret vector **s**, and a small error vector **e**. The public key is (**A**, **b = A·s + e**). Recovering **s** from (**A**, **b**) is easy if you know **e** — just solve linear equations. But without knowing which entries of **b** are noisy, the system looks uniformly random. Module-LWE extends this: instead of working with scalars over ℤ_q, it works with vectors over a ring ℤ_q[x]/(xⁿ+1), giving better efficiency for the same security level.

### Comparison Table

| Scheme | Type | Public key | Signature/Ciphertext | Security assumption | NIST level |
|--------|------|-----------|---------------------|-------------------|------------|
| Kyber-512 | KEM | 800 B | 768 B (ct) | MLWE | 1 (AES-128) |
| Kyber-1024 | KEM | 1.6 KB | 1.6 KB (ct) | MLWE | 5 (AES-256) |
| Dilithium2 | Signature | 1.3 KB | 2.4 KB (sig) | MLWE | 2 (SHA-256) |
| Dilithium5 | Signature | 2.6 KB | 4.6 KB (sig) | MLWE | 5 (AES-256) |
| SPHINCS+-128s | Signature | 32 B | 8 KB (sig) | Hash (SHAKE) | 1 (AES-128) |
| SPHINCS+-256s | Signature | 64 B | 30 KB (sig) | Hash (SHA-256) | 5 (AES-256) |
| Ed25519 (classical) | Signature | 32 B | 64 B | ECDLP | — |
| RSA-3072 (classical) | Signature | 384 B | 384 B | Factoring | — |

NIST security levels map to the work factor of breaking the corresponding symmetric primitive: Level 1 = AES-128, Level 3 = AES-192, Level 5 = AES-256.

## Build It

The `code/main.rs` program demonstrates all three PQC families using the `pqcrypto-*` Rust crates (wrappers around PQClean C implementations). Each demo runs key generation, cryptographic operations, verification, and size reporting.

### Step 1: Kyber Key Encapsulation

The Kyber demo generates a keypair, encapsulates a shared secret to the public key, and decapsulates it with the secret key. It verifies both parties arrive at the same 32-byte shared secret. The code runs two parameter sets: Kyber-512 (NIST level 1) and Kyber-1024 (NIST level 5).

```rust
fn kyber_demo<PK, SK, CT, SS>(...) {
    let (pk, sk) = keypair();
    let (ss_enc, ct) = encapsulate(&pk);
    let ss_dec = decapsulate(&ct, &sk);
    assert!(ss_enc == ss_dec, "shared secrets must match!");
    // Print sizes via as_ref().len()
}
```

**Contrast with classical ECDH:** X25519 shared secrets are 32 bytes transmitted implicitly (no ciphertext, both sides compute the same point). Kyber must transmit the ciphertext explicitly — 768–1568 bytes overhead. This is the cost of PQC.

### Step 2: Dilithium Signatures

The Dilithium demo generates a keypair, signs a message, and verifies the signature. It also tests that a tampered message is correctly rejected.

```rust
fn dilithium_demo<PK, SK, SIG>(...) {
    let (pk, sk) = keypair();
    let sig = detached_sign(message, &sk);
    assert!(verify_detached_signature(&sig, message, &pk).is_ok());
    assert!(verify_detached_signature(&sig, tampered, &pk).is_err());
}
```

The code runs Dilithium2 (~2.4 KB signatures) and Dilithium5 (~4.6 KB signatures). Compare with Ed25519's 64-byte signatures: PQC signatures are 38–72× larger.

### Step 3: SPHINCS+ Signatures

SPHINCS+ demonstrates the hash-based alternative. The code runs two variants: SPHINCS+-SHAKE-128s (8 KB signatures, SHAKE-based) and SPHINCS+-SHA2-256s (30 KB signatures, SHA-256-based).

SPHINCS+ has the smallest public keys of any PQC scheme (32–64 bytes, same as Ed25519), but the largest signatures (8–50 KB). Signing is significantly slower than verification because SPHINCS+ constructs authentication paths through a hyper-tree of Merkle trees at signing time.

```rust
fn sphincs_demo<PK, SK, SIG>(...) {
    let (pk, sk) = keypair();
    let sig = detached_sign(message, &sk);
    assert!(verify_detached_signature(&sig, message, &pk).is_ok());
}
```

**Why is SPHINCS+ signing so slow?** SPHINCS+ uses a "hyper-tree" structure: a top-level Merkle tree whose leaves are themselves Merkle trees (and so on, for multiple layers). To sign one message, SPHINCS+ generates a one-time signature key pair, signs the message with it, then generates all the authentication paths up the tree layers. This involves thousands of hash computations. Verification, by contrast, only needs one hash path from leaf to root — a few hundred hashes at most. The "s" (slow) variants prioritize small signatures; the "f" (fast) variants accept larger signatures for ~10× faster signing.

### Generic Approach — Understanding the Code Structure

All three demo functions in `code/main.rs` are generic over the key/signature types, sharing a single implementation for each operation type. This avoids code duplication and demonstrates Rust's trait-based generics in a cryptographic context.

The function signatures accept closures for the scheme-specific operations:

```rust
fn kyber_demo<PK, SK, CT, SS>(
    label: &str,
    pk_expected: usize, sk_expected: usize,
    ct_expected: usize, ss_expected: usize,
    keypair: fn() -> (PK, SK),
    encapsulate: fn(&PK) -> (SS, CT),
    decapsulate: fn(&CT, &SK) -> SS,
) where
    PK: AsRef<[u8]>, SK: AsRef<[u8]>,
    CT: AsRef<[u8]>, SS: AsRef<[u8]> + PartialEq,
{ ... }
```

The `AsRef<[u8]>` bound allows accessing the underlying byte representation for size measurement and hex printing, without knowing the concrete type. The `PartialEq` bound on `SharedSecret` enables the correctness check. This pattern makes it trivial to add new parameter sets: just pass the module's functions.

### What to Watch For in the Output

The program prints:
- Key generation, encapsulate/sign, and decapsulate/verify timing in microseconds
- Hex dumps of key prefixes and ciphertext prefixes for visual comparison
- Size breakdown for every key and output
- A tampered-message rejection test for signatures
- A summary comparison table

If the output shows all operations passing with correct sizes, the PQC primitives are working correctly.

## Use It

Post-quantum cryptography is already being deployed in production:

- **TLS 1.3 hybrid key exchange**: The IETF standardized X25519Kyber768 (RFC draft), combining classical X25519 with Kyber-768. Google Chrome, Cloudflare, and Firefox have deployed hybrid PQC key exchange. The hybrid ensures security if *either* scheme holds.
- **OpenSSL 3.2+**: Supports the OQS (Open Quantum Safe) provider for PQC algorithms including Kyber, Dilithium, and SPHINCS+.
- **Apple PQ3**: Apple's iMessage protocol uses post-quantum cryptography (a variant of Kyber) for key establishment, protecting against harvest-now-decrypt-later attacks.
- **SSH**: OpenSSH supports hybrid key exchange (sntrup761x25519) combining Streamlined NTRU Prime with X25519.
- **Signal Protocol**: Signal is researching PQC extensions to the X3DH key agreement protocol, planning to add Kyber-based KEM for post-quantum deniability.
- **NIST SP 800-227**: The NIST special publication on PQC migration guidance for US government systems, mandating the transition to PQC algorithms by 2035.

### Why Hybrid?

Pure PQC deployments carry risk: if the MLWE assumption is weakened by future cryptanalysis, systems that switched to Kyber-only would be compromised. Hybrid key exchange (X25519 + Kyber-768) ensures that the session key is secure as long as *either* assumption holds. The TLS 1.3 key schedule combines both shared secrets:

    ss = HKDF-Extract(HKDF-Extract(PSK, ss_x25519), ss_kyber)

This means an attacker would need to break both ECDLP *and* MLWE simultaneously. Hybrid migration is the recommended approach for all production deployments through at least 2035.

## Read the Source

- **FIPS 203** (ML-KEM) — The NIST standard for Kyber: module-lattice-based key encapsulation mechanism. Read the specification for exact parameter selection, error distributions, and encoding. Look at the `K-PKE` (key encapsulation) protocol definition and how the Fujisaki-Okamoto transform converts a weakly-secure PKE into a CCA-secure KEM.
- **FIPS 204** (ML-DSA) — The NIST standard for Dilithium: module-lattice-based digital signature. Note the shared MLWE foundation with Kyber. Focus on the "Fiat-Shamir with aborts" paradigm that converts a sigma protocol into a signature scheme.
- **FIPS 205** (SLH-DSA) — The NIST standard for SPHINCS+: stateless hash-based digital signature. The only PQC standard not based on lattices. Pay attention to the hyper-tree parameterization: how `h` (height), `d` (layers), and `w` (Winternitz parameter) trade off signature size against signing time.
- **pqcrypto crate source** (`github.com/rustpq/pqcrypto/src/*.rs`) — Rust bindings to PQClean C implementations. The `build.rs` compiles C source from PQClean; the Rust modules expose `keypair()` / `encapsulate()` / `decapsulate()` / `sign()` / `open()` functions that call the C FFI. Look at how `pqcrypto-internals` handles the build glue.
- **OpenSSL OQS Provider** (`github.com/open-quantum-safe/oqs-provider/`) — Production OpenSSL provider for PQC algorithms. See `oqsprov/oqsprov.c` for the EVP_PKEY method table registration and `oqsprov/oqs_kmgmt.c` for key management. This is the reference for how PQC integrates into TLS in practice.
- **RFC draft-ietf-tls-hybrid-design** — The IETF specification for hybrid key exchange in TLS 1.3. Defines the X25519Kyber768 codepoint, the shared-secret composition, and the extension encoding.

### Dependencies and Build Notes

The `code/Cargo.toml` requires:
- `pqcrypto-kyber = "0.8"` — Kyber-512/768/1024 bindings to PQClean
- `pqcrypto-dilithium = "0.5"` — Dilithium2/3/5 bindings to PQClean
- `pqcrypto-sphincsplus = "0.7"` — SPHINCS+ SHA-2/SHAKE bindings to PQClean
- `hex = "0.4"` — Hex encoding for key/signature display

These crates compile C source from the PQClean project during build, requiring a C compiler (gcc/clang). Build times are longer than typical Rust-only crates because of this C compilation step. The `--release` flag is recommended for realistic timing measurements.

## Ship It

The reusable artifact is a post-quantum cryptography demonstration library in `outputs/`. It exercises all three NIST-standardized PQC families with timing measurements, size reporting, and correctness verification. Use it as a reference when adding PQC support to the Phase 12 TLS 1.3 capstone or any security-sensitive project.

## Exercises

1. **Easy** — Run `cargo run` and record the sizes for each PQC scheme. Verify the output matches the comparison table in the lesson. Why is SPHINCS+ signing so much slower than verification?

2. **Medium** — Read about the X25519Kyber768 hybrid key exchange (RFC draft). Sketch how you would modify the TLS 1.3 handshake from Lesson 15 to support both classical and PQC key exchange. Which messages carry the Kyber ciphertext? How does the hybrid shared secret combine both outputs?

3. **Hard** — Add a third parameter set to each demo: Kyber-768, Dilithium3, SPHINCS+-SHAKE-192s. Measure the performance and compare with the existing variants. For Dilithium, try timing signature verification with batch verification — how much faster is it to verify N signatures together?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| PQC | Post-quantum cryptography | Cryptographic algorithms believed secure against both classical and quantum computers; does not require a quantum computer to run |
| Kyber / ML-KEM | NIST's chosen KEM | Key encapsulation mechanism based on Module LWE; FIPS 203 standard for shared-secret establishment |
| Dilithium / ML-DSA | NIST's primary signature | Digital signature scheme based on Module LWE; FIPS 204; fast verification, larger signatures than Ed25519 |
| SPHINCS+ / SLH-DSA | NIST's backup signature | Stateless hash-based signature; FIPS 205; security depends only on hash functions, not lattices |
| MLWE | Module Learning With Errors | The lattice problem underlying Kyber and Dilithium: given A, b = As + e, find s where e is small noise |
| LWE | Learning With Errors | Foundational lattice problem: recover a secret from noisy linear equations; basis of most PQC |
| Lattice | A regular grid of points in n-dimensional space | An additive subgroup of ℝⁿ; cryptographic lattices are defined over ℤ_qⁿ and used for their hard worst-case problems |
| Hybrid key exchange | Combining classical + PQC KEM | Running both X25519 and Kyber and combining outputs (e.g., via HKDF) so security holds if at least one survives |
| Harvest-now-decrypt-later | Store encrypted data today, decrypt later | Attack strategy: record TLS traffic now, decrypt it after a quantum computer exists; motivates early PQC migration |
| FIPS 203/204/205 | The NIST PQC standard numbers | 203 = ML-KEM (Kyber), 204 = ML-DSA (Dilithium), 205 = SLH-DSA (SPHINCS+); published August 2024 |
| Fujisaki-Okamoto transform | Converts a weak PKE into a CCA-secure KEM | Re-encryption and hash-based confirmation turns a one-way encryption into a KEM that resists chosen-ciphertext attacks |
| Fiat-Shamir with aborts | The transform used by Dilithium | Converts an interactive sigma protocol into a non-interactive signature; "aborts" mean the signer may need multiple tries to produce a valid signature |
| Merkle hyper-tree | A tree of Merkle trees | SPHINCS+ uses multiple layers of Merkle trees where each leaf in an upper layer signs the root of a lower layer; enables many signatures from a single key |

## Further Reading

- NIST PQC Standardization (csrc.nist.gov/projects/post-quantum-cryptography) — The official NIST page tracking the standardization process, including the selection rationale and timeline. Read the finalist and alternate candidate reports.
- "Post-Quantum Cryptography" by Bernstein, Buchmann, and Dahmen (Springer, 2009) — The comprehensive textbook covering lattice-based, code-based, hash-based, and multivariate PQC. Still the best single-volume reference.
- Cloudflare PQC Blog Series (blog.cloudflare.com/tag/post-quantum/) — Cloudflare's research blog posts on deploying PQC at scale, including real-world traffic analysis and hybrid key exchange performance measurements.
- pqcrypto Rust crate documentation (docs.rs/pqcrypto-*) — API reference for the Rust bindings used in this lesson's code. See `pqcrypto-kyber`, `pqcrypto-dilithium`, and `pqcrypto-sphincsplus` for the specific module APIs.
- "SoK: Hybrid Key Exchange" by Stebila, Fluhrer, and Gueron — A survey paper on hybrid KEM constructions, covering the security models and composition approaches used in TLS 1.3 PQC migration.
- "The PQClean Project" (github.com/pqclean/pqclean) — The C reference implementations underlying the pqcrypto Rust crates. Each scheme has a clean C implementation and optionally optimized (AVX2, Neon) variants.
