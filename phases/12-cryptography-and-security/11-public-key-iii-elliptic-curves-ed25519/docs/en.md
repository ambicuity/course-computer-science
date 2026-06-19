# Public Key III — Elliptic Curves & Ed25519

> Smaller keys, faster math, same security — ECC is the cryptography that fits in your pocket.

**Type:** Learn
**Languages:** Rust, Python
**Prerequisites:** Phase 12 lessons 09–10
**Time:** ~90 minutes

## Learning Objectives

- Understand how elliptic curves over finite fields enable public-key cryptography.
- Implement EC point addition, doubling, and scalar multiplication from scratch.
- Compare ECC with RSA/DH in terms of key size, performance, and security level.
- Explain how Ed25519 works and why it is preferred over ECDSA.

## The Problem

RSA needs 2048-bit keys for adequate security. A 2048-bit RSA public key is 256 bytes. A 3072-bit key — which provides the equivalent of 128-bit symmetric security — is 384 bytes. For every TLS connection, every SSH login, every code-signing operation, those large keys must be transmitted, stored, and processed. On a modern server this is fine. On an IoT sensor with 32 KB of RAM, a smart card, or a mobile phone doing hundreds of key exchanges per day, the cost adds up.

Elliptic curve cryptography (ECC) achieves the same 128-bit security level with 256-bit keys — 32 bytes. That is 12× smaller than RSA-3072. Signatures are smaller too: an Ed25519 signature is 64 bytes versus RSA-2048's 256 bytes. Operations are faster because field arithmetic on 256-bit numbers is cheaper than modular exponentiation with 2048-bit numbers. For constrained devices, ECC is not just better — it is the difference between feasible and impossible.

The phase capstone (a TLS 1.3 implementation with a mini-CTF toolkit) uses ECC everywhere: X25519 for key exchange, Ed25519 for certificate signatures, and ECDSA for some legacy compatibility. Without understanding ECC, you cannot build or break any of these.

## The Concept

An elliptic curve over a prime field \(\mathbb{F}_p\) is the set of points \((x, y)\) satisfying:

\[
y^2 \equiv x^3 + ax + b \pmod{p}
\]

where \(4a^3 + 27b^2 \not\equiv 0 \pmod{p}\) (the curve has no singularities). The points form an abelian group under an operation called **point addition**:

- **Point addition** \(P + Q\): draw the line through \(P\) and \(Q\); it intersects the curve at a third point \(-R\); reflect across the x-axis to get \(R\).
- **Point doubling** \(2P\): tangent line at \(P\) instead of secant; same reflection rule.
- **Point at infinity** \(O\): the identity element. \(P + O = P\).
- **Scalar multiplication** \(k \cdot P\): add \(P\) to itself \(k\) times using double-and-add.

The security of ECC rests on the **Elliptic Curve Discrete Logarithm Problem (ECDLP)**:

> Given a base point \(G\) and a point \(Q = k \cdot G\), find the scalar \(k\).

The best known attacks on a 256-bit elliptic curve require roughly \(2^{128}\) operations — the same work as breaking AES-128. For RSA, achieving 128-bit security requires a 3072-bit modulus. This efficiency gap is why ECC has replaced RSA in most modern protocols.

Two forms of Curve25519 dominate practice:

| Form | Equation | Use | Algorithm |
|------|----------|-----|-----------|
| Montgomery | \(y^2 = x^3 + 486662x^2 + x \mod 2^{255}-19\) | Key exchange | X25519 |
| Twisted Edwards | \(-x^2 + y^2 = 1 + 121665/121666 \cdot x^2y^2 \mod 2^{255}-19\) | Signatures | Ed25519 |

The Montgomery form enables a **Montgomery ladder** — a constant-time scalar multiplication that resists side-channel attacks. The Edwards form enables efficient, complete addition formulas that work for all inputs (no special cases for doubling or the identity). Both forms are birationally equivalent: they represent the same curve in different coordinate systems.

Ed25519 signing is **deterministic**: the nonce is derived as \(\text{SHA-512}(\text{seed} \parallel \text{message})\), so no RNG is needed. This eliminates an entire class of vulnerabilities that plagued ECDSA (e.g., the Sony PS3 private key recovery from repeated nonces, and the Android Bitcoin wallet nonce reuse bug).

## Build It

### Step 1: Elliptic Curve Point Operations in Python

We implement an `EllipticCurve` class, a `Point` class with addition and doubling, and scalar multiplication via double-and-add.

```python
class EllipticCurve:
    """y^2 = x^3 + ax + b over F_p."""
    def __init__(self, a, b, p):
        self.a = a
        self.b = b
        self.p = p
        d = (4 * a**3 + 27 * b**2) % p
        assert d != 0, "singular curve"

    def is_on_curve(self, x, y):
        return (y * y - (x**3 + self.a * x + self.b)) % self.p == 0


class Point:
    def __init__(self, curve, x, y):
        self.curve = curve
        self.x = x
        self.y = y
        if x is not None and y is not None:
            assert curve.is_on_curve(x, y)

    def is_infinity(self):
        return self.x is None and self.y is None

    INF = None  # singleton set below


Point.INF = Point(None, None, None)  # type: ignore


def extended_gcd(a, b):
    if a == 0:
        return b, 0, 1
    g, x1, y1 = extended_gcd(b % a, a)
    return g, y1 - (b // a) * x1, x1


def inv_mod(k, p):
    g, x, _ = extended_gcd(k % p, p)
    if g != 1:
        raise ValueError("not invertible")
    return x % p


def point_add(P, Q):
    if P.is_infinity():
        return Q
    if Q.is_infinity():
        return P
    if P.x == Q.x and (P.y + Q.y) % P.curve.p == 0:
        return Point.INF

    curve = P.curve
    if P.x == Q.x and P.y == Q.y:
        s = (3 * P.x**2 + curve.a) * inv_mod(2 * P.y, curve.p) % curve.p
    else:
        s = (Q.y - P.y) * inv_mod(Q.x - P.x, curve.p) % curve.p

    x3 = (s * s - P.x - Q.x) % curve.p
    y3 = (s * (P.x - x3) - P.y) % curve.p
    return Point(curve, x3, y3)


def scalar_mult(k, P):
    if k == 0 or P.is_infinity():
        return Point.INF
    result = Point.INF
    addend = P
    while k:
        if k & 1:
            result = point_add(result, addend)
        addend = point_add(addend, addend)
        k >>= 1
    return result
```

### Step 2: Ed25519 Key Operations in Rust

This step demonstrates Ed25519 key generation and verification logic using the `ed25519-dalek` crate, with raw scalar multiplication on the Ed25519 curve for educational purposes.

```rust
use sha2::Sha512;
use curve25519_dalusek::{EdwardsPoint, Scalar};
use rand::rngs::OsRng;

/// Generate an Ed25519 key pair from a random seed.
/// Production Ed25519 seeds are 32 bytes; SHA-512 expands them.
fn generate_keypair() -> ([u8; 32], EdwardsPoint) {
    let mut seed = [0u8; 32];
    OsRng.fill_bytes(&mut seed);

    // SHA-512 the seed, then clamp
    let hash = Sha512::digest(&seed);
    let mut scalar_bytes = [0u8; 32];
    scalar_bytes.copy_from_slice(&hash[..32]);
    // Clamp: clear cofactor bits, set high bit
    scalar_bytes[0]  &= 248;
    scalar_bytes[31] &= 127;
    scalar_bytes[31] |= 64;

    let secret = Scalar::from_bits(scalar_bytes);
    let public = EdwardsPoint::mul_base(&secret);
    (seed, public)
}
```

The **clamping** step ensures the scalar is a multiple of the cofactor (8) and falls in the proper range. This prevents small-subgroup attacks.

## Use It

Ed25519 and X25519 are everywhere in modern cryptography:

- **SSH**: `ssh-ed25519` keys are the default in OpenSSH 9.x. Smaller than RSA, faster to authenticate.
- **TLS 1.3**: X25519 is the mandatory-to-implement key exchange. Ed25519 is used for TLS certificate signatures (though less common than ECDSA P-256 in practice).
- **Signal Protocol**: The X3DH key agreement protocol uses X25519 for initial key exchange and ratcheting.
- **Bitcoin/Ethereum**: Use secp256k1 (a different curve: \(y^2 = x^3 + 7\)) for ECDSA signatures. Ed25519 is not used due to the cofactor and the desire for standardized ECDSA.
- **DNSSEC**: Ed25519 is supported for zone-signing, providing smaller signatures than RSA.
- **OpenPGP**: Ed25519 keys are supported in the new crypto stack (Sequoia, GnuPG 2.3+).

Production implementations like `libsodium`, `openssl`, and `curve25519-dalek` use **assembly-optimized field arithmetic** for the 255-bit prime \(2^{255} - 19\). The reference implementation uses a 10-limb representation (each limb is 25.5 bits) to fit in 64-bit registers without overflow. Constant-time operation is mandatory — any data-dependent branch or memory access leaks key material through timing.

## Read the Source

- **RFC 8032** — EdDSA (Ed25519 and Ed448): the specification that defines key generation, signing, and verification.
- **SUPERCOP `crypto_sign/ed25519/`** — Reference implementations of Ed25519; look at `ref10/` for the reference implementation by Bernstein et al.
- **libsodium `src/libsodium/crypto_sign/ed25519/`** — Production implementation used by millions; shows how to handle cofactor, batch verification, and side-channel resistance.
- **OpenSSL `crypto/ec/ecp_oct.c`** — EC point octet (byte) conversion; shows point serialization/deserialization.
- **Curve25519 paper** — "Curve25519: New Diffie-Hellman Speed Records" by Daniel J. Bernstein; explains the Montgomery ladder and the choice of the specific curve parameters.

## Ship It

The reusable artifact is an elliptic curve library implementing point operations and Ed25519 key generation. It lives in `outputs/` as a reference implementation you can slot into later phases (the TLS 1.3 capstone needs both X25519 key exchange and Ed25519 signatures).

## Exercises

1. **Easy** — On the curve \(y^2 = x^3 + 2x + 3 \mod 97\), the point \(G = (3, 6)\) has order 5. Verify that \(5 \cdot G = O\) by computing \(G, 2G, 3G, 4G, 5G\). Confirm that \(2G = (80, 10)\).
2. **Medium** — Extend the Python code to find the order of any point on a small curve by brute force. On the curve \(y^2 = x^3 + 2x + 3 \mod 97\), how many points does the curve have? (Use Hasse's bound as a sanity check.)
3. **Hard** — Implement Ed25519 key generation and signing from scratch in Python without using any crypto library. Use the `ed25519.py` reference implementation as a guide. Verify your signatures against a known test vector from RFC 8032.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Elliptic curve | A curve used for cryptography | The set of points satisfying \(y^2 = x^3 + ax + b\) over a finite field, forming a group under point addition |
| Point addition | Adding two curve points | The group operation: line through P and Q finds a third intersection point, reflected across the x-axis |
| Scalar multiplication | Multiplying a point by a number | Repeated point addition via double-and-add: \(kP = P + P + \dots + P\) |
| ECDLP | The hard problem behind ECC | Given P and kP, find k. Believed to be harder than integer factorization for equivalent sizes |
| Curve25519 | A specific elliptic curve | \(y^2 = x^3 + 486662x^2 + x\) over \(\mathbb{F}_{2^{255}-19}\), chosen for twist security, constant-time performance |
| Ed25519 | A signature algorithm | EdDSA using the Edwards form of Curve25519; deterministic, fast, batch-verifiable |
| Montgomery ladder | A way to do scalar multiplication | A constant-time algorithm that always does both an addition and a doubling per bit, regardless of the bit value |
| Clamping | Processing a scalar before use | Bit operations on the scalar to clear cofactor bits and set the high bit, preventing small-subgroup attacks |
| Twisted Edwards curve | A curve form with complete addition formulas | \(-x^2 + y^2 = 1 + d \cdot x^2y^2\); all additions work for all inputs, no special cases |
| Cofactor | A small divisor of the curve order | Ed25519's cofactor is 8; implementations must handle it to avoid small-subgroup attacks |

## Further Reading

- "Curve25519: New Diffie-Hellman Speed Records" by Daniel J. Bernstein — the original paper introducing the curve.
- "Ed25519: High-speed high-security signatures" by Bernstein, Duif, Lange, Schwabe, and Yang — the paper describing the signature scheme.
- RFC 7748 (Elliptic Curves for Security) and RFC 8032 (EdDSA) — the IETF standards for X25519, X448, Ed25519, and Ed448.
- "A (Relatively Easy To Understand) Primer on Elliptic Curve Cryptography" (Cloudflare blog) — a gentle introduction with interactive visualizations.
- "Implementing Curve25519/X25519: A Tutorial" by Martin Kleppmann — a step-by-step walkthrough of field arithmetic, the Montgomery ladder, and constant-time implementation.
