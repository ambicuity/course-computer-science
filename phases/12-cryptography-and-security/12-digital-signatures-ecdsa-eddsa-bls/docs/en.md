# Digital Signatures — ECDSA, EdDSA, BLS

> Three signature schemes, one big idea: prove you wrote a message without sharing your secret.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 12 lessons 01–11
**Time:** ~75 minutes

## Learning Objectives

- Implement ECDSA signing and verification over a toy elliptic curve and demonstrate the catastrophic nonce-reuse vulnerability.
- Compare ECDSA with EdDSA (Ed25519) and explain why deterministic nonce derivation eliminates an entire class of vulnerabilities.
- Understand BLS signature aggregation via bilinear pairings and quantify the bandwidth savings it provides in multi-signature settings.
- Choose the right signature scheme for a given threat model (performance, security, aggregation requirements).

## The Problem

You receive a firmware update for your IoT device. How do you know it actually came from the manufacturer and not from an attacker who compromised the update server? The update is signed with the manufacturer's private key, and your device verifies the signature against a baked-in public key. If the signature scheme is broken — or misused — the attacker can forge updates and own your device.

On a blockchain with thousands of validators, every block needs signatures from a supermajority of validators. With ECDSA, that means appending ~64 bytes per validator. With 1000 validators, that's 64 KB of signature data per block — before you even store the transactions. With BLS signatures, all 1000 signatures compress to a single 48-byte group element. That is the difference between "fits in one block" and "needs a separate block for signatures alone."

The phase capstone (TLS 1.3 implementation + mini-CTF toolkit) uses signatures everywhere: certificate chains use ECDSA or Ed25519, the mini-CTF includes challenges that exploit nonce reuse, and the TLS handshake itself depends on signature-based authentication. You cannot secure or break any of these without understanding how ECDSA, EdDSA, and BLS actually work under the hood.

## The Concept

A digital signature scheme has three operations:

- **Key generation:** produce a private key \(d\) and a public key \(Q\).
- **Signing:** given a message \(m\) and private key \(d\), produce a signature \(\sigma\).
- **Verification:** given \(m\), \(\sigma\), and public key \(Q\), accept or reject.

The security requirement: without \(d\), an attacker cannot produce a valid signature for any message they haven't seen signed before.

### ECDSA

ECDSA works over an elliptic curve group of order \(n\). The signer:

1. Hashes the message: \(e = \text{SHA-256}(m)\).
2. Picks a random **nonce** \(k \in [1, n-1]\).
3. Computes \(R = k \cdot G\) and \(r = R_x \bmod n\).
4. Computes \(s = k^{-1}(e + d \cdot r) \bmod n\).

The signature is \((r, s)\). A verifier computes:

\[
u_1 = e \cdot s^{-1} \bmod n, \quad u_2 = r \cdot s^{-1} \bmod n
\]

and checks that \((u_1 \cdot G + u_2 \cdot Q)_x \equiv r \pmod{n}\).

The **nonce \(k\)** is the single point of failure. If the same \(k\) is ever used for two different messages, anyone who sees both signatures can compute:

\[
k = \frac{e_1 - e_2}{s_1 - s_2}, \quad d = \frac{s_1 \cdot k - e_1}{r}
\]

and steal the private key. This is not theoretical — it happened to Sony's PS3 (2010), to Android Bitcoin wallets (2013), and to numerous cryptocurrency projects since.

### EdDSA / Ed25519

Ed25519 is EdDSA over the twisted Edwards curve Curve25519. The critical innovation: **the nonce is deterministic**:

\[
k = \text{SHA-512}(\text{seed} \parallel m)
\]

where the seed is part of the private key. Every signature is a deterministic function of the message and key. No RNG needed. No nonce reuse possible. (Unless you clone the private key — but that's a different threat model.)

Ed25519 signatures are 64 bytes; public keys are 32 bytes. Verification is roughly 3× faster than ECDSA over a 256-bit curve with comparable security.

### BLS Signatures

BLS (Boneh–Lynn–Shacham) uses **bilinear pairings** on pairing-friendly curves like BLS12-381. A pairing is a map:

\[
e: G_1 \times G_2 \rightarrow G_T
\]

with the bilinear property:

\[
e(g_1^a, g_2^b) = e(g_1, g_2)^{ab}
\]

A BLS signature is simply \(\sigma = H(m)^d \in G_2\) (or \(G_1\), depending on the mode). Verification checks:

\[
e(\sigma, g_2) \stackrel{?}{=} e(H(m), Q)
\]

The magic is **aggregation**: given signatures \(\sigma_1, \sigma_2, \dots, \sigma_n\) on messages \(m_1, m_2, \dots, m_n\) under keys \(Q_1, Q_2, \dots, Q_n\), anyone can compute:

\[
\sigma_{\text{agg}} = \sigma_1 + \sigma_2 + \cdots + \sigma_n
\]

a single group element. Verification of the aggregate requires all \((m_i, Q_i)\) pairs, but the signature data shrinks from \(n \times 96\) bytes to just 96 bytes — or 48 bytes in the min-sig mode.

| Scheme | Public key | Signature | Deterministic? | Aggregatable? |
|--------|-----------|-----------|----------------|---------------|
| ECDSA-256 | 32 B | 64 B | No (needs RNG) | No |
| Ed25519 | 32 B | 64 B | Yes | No |
| BLS12-381 | 48 B | 96 B (or 48) | Yes | Yes |

## Build It

### Step 1: ECDSA — Signing and Verification on a Toy Curve

We implement ECDSA over the curve \(y^2 = x^3 + 2x + 3 \bmod 97\), generator \(G = (3, 6)\) of order \(n = 5\). A 5-element group is tiny — insecure by a factor of \(2^{128}\) — but it lets us trace every operation by hand.

```rust
// Full ECDSA implementation on the toy curve
// See code/main.rs for the complete program.

struct Curve {
    a: i64, b: i64, p: i64,
    g: Point, n: i64,
}

impl Curve {
    fn new() -> Self {
        // y^2 = x^3 + 2x + 3 mod 97, G = (3,6), order n = 5
        Curve { a: 2, b: 3, p: 97,
                g: Point { x: Some(3), y: Some(6) }, n: 5 }
    }
}
```

We implement point addition, scalar multiplication (double-and-add), and ECDSA keygen/sign/verify. Then we demonstrate the nonce reuse attack:

- Sign two different messages with the same \(k\).
- Both signatures have identical \(r\) (because \(r = (k \cdot G)_x\)).
- Recover \(k = (e_1 - e_2)(s_1 - s_2)^{-1} \bmod n\).
- Recover \(d = (s_1 \cdot k - e_1) \cdot r^{-1} \bmod n\).

The output shows the stolen private key matches the original — the attacker now controls the signing key.

### Step 2: Ed25519 — Deterministic Signatures

Using the `ed25519-dalek` crate:

```rust
use ed25519_dalek::{SigningKey, Signature, Verifier};

let mut csprng = OsRng;
let signing_key = SigningKey::generate(&mut csprng);
let verifying_key = signing_key.verifying_key();

// Signing: no RNG parameter — the nonce is derived from seed || message
let signature: Signature = signing_key.sign(b"Ed25519 is deterministic");
assert!(verifying_key.verify(b"Ed25519 is deterministic", &signature).is_ok());
```

The private key is a 32-byte seed. Signing hashes the seed with the message through SHA-512 to produce the nonce. Signing the same message twice produces identical signatures — no RNG, no nonce reuse, no Sony PS3 scenario.

### Step 3: BLS — Signature Aggregation

Using the `blst` crate (BLS12-381 implementation):

```rust
use blst::min_pk::*;

// Generate three key pairs
let sk = SecretKey::key_gen(&ikm, &[]).unwrap();
let pk = sk.sk_to_pk();
let sig = sk.sign(msg, dst, &[]);
assert!(pk.verify(&sig, msg, dst, &[]));

// Aggregate all three signatures into one
let agg_sig = AggregateSignature::aggregate(&[&sig1, &sig2, &sig3], false).unwrap();
let agg_pk = AggregatePublicKey::aggregate(&[&pk1, &pk2, &pk3], false).unwrap();
assert!(agg_pk.to_public_key().verify(&agg_sig.to_signature(), msg, dst, &[]));
```

Three signatures (3 × 96 = 288 bytes) compress to one (96 bytes). For 1000 validators on a blockchain, that is 96 KB → 96 bytes. The trade-off: the verifier must know all 1000 public keys and messages, but the signature data itself shrinks by 1000×.

## Use It

- **ECDSA** — Bitcoin, Ethereum, TLS certificates (P-256), DNSSEC, code signing. Everywhere. But the nonce fragility means implementations must use either hardware RNG or deterministic nonces (RFC 6979).
- **Ed25519** — OpenSSH (default since 9.x), Signal Protocol, OpenPGP (GnuPG 2.3+), Tor, WireGuard. Preferred whenever you control both ends and want fast, safe signatures.
- **BLS** — Ethereum 2.0 (consensus layer), Filecoin, Chia, Dfinity. Any blockchain with a proof-of-stake consensus where validators sign blocks and those signatures must be aggregated.

Production implementations differ from our toy code in important ways:
- **Constant-time math.** Our double-and-add has a branch on the scalar bit. A real implementation uses a Montgomery ladder or similar to prevent timing attacks.
- **Big-integer arithmetic.** Our toy uses `i64` — production uses limb-based representations (e.g., `u64[4]` for 256-bit fields or `u64[6]` for 381-bit fields).
- **Side-channel resistance.** Real implementations mask the private key, randomize projective coordinates, and avoid secret-dependent memory access.
- **Batch verification.** Ed25519 and BLS both support verifying many signatures at once with sub-linear cost via Strauss's algorithm or multi-pairing checks.

## Read the Source

- **RFC 6979** — Deterministic usage of ECDSA (the fix for the nonce vulnerability).
- **RFC 8032** — EdDSA (Ed25519 and Ed448): key generation, signing, verification, and test vectors.
- **BLS标准草案 (IETF)** — `draft-irtf-cfrg-bls-signature-05`: the standard for BLS signatures over BLS12-381.
- **libsodium `src/libsodium/crypto_sign/ed25519/`** — Production Ed25519 with proper cofactor handling and side-channel resistance.
- **blst crate `src/`** — The BLS12-381 library used in Ethereum 2.0; look at `blst.h` for the C API and `min_pk.rs` for the high-level Rust bindings.

## Ship It

The reusable artifact is a digital signature demo that implements ECDSA on a toy curve and demonstrates Ed25519 and BLS with production crates. It lives in `outputs/` as a reference you can reuse in the TLS 1.3 capstone (for signing handshake messages) and the mini-CTF (the nonce-reuse challenge is straight from Step 1).

## Exercises

1. **Easy** — On the toy curve (\(y^2 = x^3 + 2x + 3 \bmod 97\)), generate a key pair with private key \(d = 3\). Sign the message `"hello"` with nonce \(k = 2\). Verify the signature by hand. What are \((r, s)\)?
2. **Medium** — Modify the Rust ECDSA implementation to use deterministic nonces (RFC 6979): derive \(k = \text{HMAC-SHA-256}(d, m)\) instead of using a random \(k\). Verify that signing the same message twice now produces identical signatures.
3. **Hard** — Implement multi-message BLS aggregation: generate three key pairs, sign *different* messages with each, aggregate the signatures, and verify the aggregate using the proof-of-possession scheme (see the IETF BLS standard, Section 3.3). Compare the verification cost against verifying all three signatures individually.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Nonce | A random number used once in signing | The value \(k\) in ECDSA; if reused, the private key is leaked. EdDSA derives it deterministically from seed+messagae. |
| ECDSA | Elliptic Curve Digital Signature Algorithm | A signature scheme using elliptic curves with a random nonce; vulnerable if RNG is broken or nonces are reused. |
| EdDSA | Edwards-curve Digital Signature Algorithm | A family of deterministic signature schemes (Ed25519, Ed448) that derive the nonce from key material, eliminating RNG dependence. |
| BLS | Boneh–Lynn–Shacham signatures | A signature scheme using bilinear pairings that supports aggregation: \(n\) signatures compress to 1 group element. |
| Bilinear pairing | A map \(e: G_1 \times G_2 \to G_T\) | A bilinear map where \(e(g_1^a, g_2^b) = e(g_1, g_2)^{ab}\), enabling BLS verification and aggregation. |
| Aggregation | Combining many signatures into one | In BLS, adding group elements: \(\sigma_{\text{agg}} = \sum \sigma_i\). Verification checks all messages against all keys in one pairing equation. |
| Double-and-add | Algorithm for scalar multiplication | Repeated doubling (add point to self) with conditional addition based on bits of the scalar. Not constant-time. |
| Deterministic signature | A signature that depends only on key and message | No randomness needed. EdDSA and RFC 6979 ECDSA are deterministic; vanilla ECDSA is not. |
| Clamping | Bit manipulation of scalar before use | Clearing low bits (cofactor), setting high bit (constant-time range). Prevents small-subgroup attacks in Ed25519. |
| DST | Domain Separation Tag | A string that binds a signature to a specific protocol context, preventing cross-protocol signature reuse. |

## Further Reading

- "The Insecurity of the Digital Signature Algorithm" (NIST, SP 800-186) — Explains the nonce generation requirements for DSA/ECDSA and why deterministic nonces are recommended.
- RFC 8032 (EdDSA) — The authoritative spec for Ed25519 and Ed448; includes test vectors you can verify against your own implementation.
- "BLS Signatures for Aggregation" (Ethereum 2.0 spec) — How Ethereum uses BLS to aggregate validator signatures in the beacon chain.
- "Why ECDSA has a nonce, and why EdDSA doesn't" (Blog, Neil Madden) — A clear, non-mathematical explanation of the key difference between the two schemes.
- "Pairings for Beginners" (Craig Costello, Microsoft Research) — The best introduction to bilinear pairings for cryptography, from first principles to applications.
