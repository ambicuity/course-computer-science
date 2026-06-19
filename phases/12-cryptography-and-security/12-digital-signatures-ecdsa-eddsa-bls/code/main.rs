//! Digital Signatures — ECDSA, EdDSA, BLS
//! Phase 12 — Cryptography & Security
//!
//! Dependencies (add to Cargo.toml):
//! [dependencies]
//! sha2 = "0.10"
//! ed25519-dalek = "2"
//! rand = "0.8"
//! blst = "0.3"
//!
//! Run with: cargo run

use sha2::{Digest, Sha256};
use std::fmt;

// ====================================================================
// Step 1: ECDSA on a Toy Elliptic Curve
// Curve: y^2 = x^3 + 2x + 3 mod 97
// Generator: G = (3, 6), order n = 5
// ====================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
struct Point {
    x: Option<i64>,
    y: Option<i64>,
}

const INF: Point = Point { x: None, y: None };

struct Curve {
    a: i64,
    b: i64,
    p: i64,
    g: Point,
    /// Order of the generator G
    n: i64,
}

impl Curve {
    fn new() -> Self {
        Curve {
            a: 2,
            b: 3,
            p: 97,
            g: Point {
                x: Some(3),
                y: Some(6),
            },
            n: 5,
        }
    }

    fn is_on_curve(&self, x: i64, y: i64) -> bool {
        let lhs = ((y * y) % self.p + self.p) % self.p;
        let rhs = ((x * x * x + self.a * x + self.b) % self.p + self.p) % self.p;
        lhs == rhs
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.x, self.y) {
            (Some(x), Some(y)) => write!(f, "({}, {})", x, y),
            _ => write!(f, "O"),
        }
    }
}

fn extended_gcd(a: i64, b: i64) -> (i64, i64, i64) {
    if a == 0 {
        return (b, 0, 1);
    }
    let (g, x1, y1) = extended_gcd(b % a, a);
    (g, y1 - (b / a) * x1, x1)
}

fn inv_mod(k: i64, m: i64) -> i64 {
    assert_ne!(k, 0, "cannot invert zero");
    let k = ((k % m) + m) % m;
    let (g, x, _) = extended_gcd(k, m);
    assert_eq!(g, 1, "{} is not invertible mod {}", k, m);
    (x % m + m) % m
}

fn point_add(P: &Point, Q: &Point, curve: &Curve) -> Point {
    if P.x.is_none() {
        return *Q;
    }
    if Q.x.is_none() {
        return *P;
    }

    let px = P.x.unwrap();
    let py = P.y.unwrap();
    let qx = Q.x.unwrap();
    let qy = Q.y.unwrap();

    if px == qx && (py + qy) % curve.p == 0 {
        return INF;
    }

    let s: i64;
    if px == qx && py == qy {
        let num = ((3 * px * px + curve.a) % curve.p + curve.p) % curve.p;
        let den = (2 * py) % curve.p;
        s = num * inv_mod(den, curve.p) % curve.p;
    } else {
        let num = ((qy - py) % curve.p + curve.p) % curve.p;
        let den = ((qx - px) % curve.p + curve.p) % curve.p;
        s = num * inv_mod(den, curve.p) % curve.p;
    }

    let x3 = ((s * s - px - qx) % curve.p + curve.p) % curve.p;
    let y3 = ((s * (px - x3) - py) % curve.p + curve.p) % curve.p;
    Point {
        x: Some(x3),
        y: Some(y3),
    }
}

fn scalar_mult(k: i64, P: &Point, curve: &Curve) -> Point {
    if k == 0 || P.x.is_none() {
        return INF;
    }
    let mut result = INF;
    let mut addend = *P;
    let mut n = k.abs();
    while n > 0 {
        if n & 1 == 1 {
            result = point_add(&result, &addend, curve);
        }
        addend = point_add(&addend, &addend, curve);
        n >>= 1;
    }
    result
}

/// Hash a message to a scalar in [0, n-1].
fn hash_to_scalar(msg: &[u8], modulus: i64) -> i64 {
    let hash = Sha256::digest(msg);
    let bytes = hash.as_slice();
    let mut val: i64 = 0;
    for i in 0..8.min(bytes.len()) {
        val = (val << 8) | (bytes[i] as i64);
    }
    ((val % modulus) + modulus) % modulus
}

struct ECDSA;

impl ECDSA {
    fn keygen(curve: &Curve, private_key: i64) -> Point {
        scalar_mult(private_key, &curve.g, curve)
    }

    fn sign(curve: &Curve, private_key: i64, msg: &[u8], nonce: i64) -> (i64, i64) {
        let e = hash_to_scalar(msg, curve.n);
        let R = scalar_mult(nonce, &curve.g, curve);
        let r = R.x.unwrap_or(0) % curve.n;
        assert_ne!(r, 0, "r == 0: try a different nonce");
        let k_inv = inv_mod(nonce, curve.n);
        let s = (k_inv * ((e + private_key * r) % curve.n)) % curve.n;
        assert_ne!(s, 0, "s == 0: try a different nonce");
        (r, s)
    }

    fn verify(curve: &Curve, public_key: &Point, msg: &[u8], r: i64, s: i64) -> bool {
        if r < 1 || r >= curve.n || s < 1 || s >= curve.n {
            return false;
        }
        let e = hash_to_scalar(msg, curve.n);
        let s_inv = inv_mod(s, curve.n);
        let u1 = (e * s_inv) % curve.n;
        let u2 = (r * s_inv) % curve.n;
        let P = point_add(
            &scalar_mult(u1, &curve.g, curve),
            &scalar_mult(u2, public_key, curve),
            curve,
        );
        match P.x {
            Some(x) => x % curve.n == r,
            None => false,
        }
    }

    /// Given two signatures that reused the same nonce k, recover k and the
    /// private key d.  Both signatures must have the same r value.
    fn recover_from_nonce_reuse(
        curve: &Curve,
        msg1: &[u8],
        sig1: (i64, i64),
        msg2: &[u8],
        sig2: (i64, i64),
    ) -> (i64, i64) {
        let (r1, s1) = sig1;
        let (r2, s2) = sig2;
        assert_eq!(r1, r2, "nonce reuse: r values must match");

        let e1 = hash_to_scalar(msg1, curve.n);
        let e2 = hash_to_scalar(msg2, curve.n);

        // k = (e1 - e2) / (s1 - s2) mod n
        let num = ((e1 - e2) % curve.n + curve.n) % curve.n;
        let den = ((s1 - s2) % curve.n + curve.n) % curve.n;
        let k = (num * inv_mod(den, curve.n)) % curve.n;

        // d = (s1 * k - e1) / r1 mod n
        let d = ((s1 * k - e1) % curve.n + curve.n) % curve.n;
        let d = (d * inv_mod(r1, curve.n)) % curve.n;

        (k, d)
    }
}

// ====================================================================
// Step 2: Ed25519 Signatures
// ====================================================================

use ed25519_dalek::{Signature, SigningKey, Verifier};

fn ed25519_demo(signing_key: &SigningKey) {
    println!("\n{}", "-".repeat(60));
    println!("  Step 2: Ed25519 — Deterministic Signatures");
    println!("{}", "-".repeat(60));

    let verifying_key = signing_key.verifying_key();
    let msg = b"Ed25519 derives nonces deterministically from seed||message";

    // Sign — no RNG parameter; the nonce is derived from the seed and message.
    let sig: Signature = signing_key.sign(msg);

    // Verify
    assert!(verifying_key.verify(msg, &sig).is_ok());
    println!("  Message signed and verified successfully.");
    println!("  Public key (hex): {}", bytes_to_hex(&verifying_key.to_bytes()));
    println!("  Signature (hex):  {}", bytes_to_hex(&sig.to_bytes()));
    println!("  Signature size:   {} bytes", sig.to_bytes().len());
    println!("  Public key size:  {} bytes\n", verifying_key.to_bytes().len());

    // Tampered message should be rejected
    let wrong = b"tampered message";
    assert!(verifying_key.verify(wrong, &sig).is_err());
    println!("  ✓ Tampered message rejected.");

    // Deterministic property: same key + same message = same signature
    let sig2: Signature = signing_key.sign(msg);
    assert_eq!(sig.to_bytes(), sig2.to_bytes());
    println!("  ✓ Deterministic: same key + same message = identical signature.");

    // Different message produces different signature (obviously)
    let sig3: Signature = signing_key.sign(b"different message");
    assert_ne!(sig.to_bytes(), sig3.to_bytes());
    println!("  ✓ Different message → different signature.");
}

// ====================================================================
// Step 3: BLS Signature Aggregation
// ====================================================================

use blst::min_pk::{
    AggregatePublicKey, AggregateSignature, PublicKey, SecretKey, Signature as BlsSignature,
};

fn bls_demo() {
    println!("\n{}", "-".repeat(60));
    println!("  Step 3: BLS Signature Aggregation (BLS12-381)");
    println!("{}", "-".repeat(60));

    // Domain separation tag — prevents cross-protocol reuse of signatures.
    let dst = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";
    let msg = b"BLS aggregation — one signature to rule them all";

    // Generate three key pairs with distinct IKM (input key material).
    let sk1 = SecretKey::key_gen(&[1u8; 32], &[]).expect("key_gen failed");
    let pk1 = sk1.sk_to_pk();
    let sk2 = SecretKey::key_gen(&[2u8; 32], &[]).expect("key_gen failed");
    let pk2 = sk2.sk_to_pk();
    let sk3 = SecretKey::key_gen(&[3u8; 32], &[]).expect("key_gen failed");
    let pk3 = sk3.sk_to_pk();

    // Each signer signs the common message.
    let sig1: BlsSignature = sk1.sign(msg, dst, &[]);
    let sig2: BlsSignature = sk2.sign(msg, dst, &[]);
    let sig3: BlsSignature = sk3.sign(msg, dst, &[]);

    println!("  Individual signature sizes: {} bytes each", 96);
    println!("  Total signature data (3 sigs): {} bytes\n", 96 * 3);

    // Verify each signature individually.
    assert!(pk1.verify(&sig1, msg, dst, &[]));
    assert!(pk2.verify(&sig2, msg, dst, &[]));
    assert!(pk3.verify(&sig3, msg, dst, &[]));
    println!("  ✓ All 3 individual signatures verified.");

    // Aggregate the three signatures into one.
    let agg_sig =
        AggregateSignature::aggregate(&[&sig1, &sig2, &sig3], false).expect("aggregation failed");
    let agg_sig_point = agg_sig.to_signature();

    println!("  Aggregated signature size: {} bytes", 96);
    println!("  Compression ratio: {}:1\n", 3);

    // Aggregate the corresponding public keys.
    let agg_pk =
        AggregatePublicKey::aggregate(&[&pk1, &pk2, &pk3], false).expect("pk aggregation failed");
    let agg_pk_point = agg_pk.to_public_key();

    // Verify the aggregate signature against the aggregate public key.
    assert!(agg_pk_point.verify(&agg_sig_point, msg, dst, &[]));
    println!("  ✓ Aggregated signature verified against aggregated public key.");

    // For 1000 validators, the savings are enormous.
    println!();
    println!("  --- What about 1000 validators? ---");
    println!("  Individual:   {} bytes  ({:.1} KB)", 96 * 1000, (96.0 * 1000.0) / 1024.0);
    println!("  Aggregated:   {} bytes\n", 96);
}

// ====================================================================
// Utility helpers
// ====================================================================

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn confirm_toy_curve(curve: &Curve) {
    println!("  Curve: y^2 = x^3 + {}x + {} mod {}", curve.a, curve.b, curve.p);
    println!("  Generator G = {}", curve.g);
    println!("  Order of G: {}", curve.n);
    assert!(curve.is_on_curve(3, 6));
    assert!(curve.is_on_curve(80, 10));
    assert!(curve.is_on_curve(80, 87));
    assert!(curve.is_on_curve(3, 91));
    // 5*G = O
    assert!(scalar_mult(5, &curve.g, curve).x.is_none());
    println!("  ✓ Generator confirmed: 5*G = O");
    // Verify the group structure
    let g2 = scalar_mult(2, &curve.g, curve);
    assert_eq!(g2, Point { x: Some(80), y: Some(10) });
    let g3 = scalar_mult(3, &curve.g, curve);
    assert_eq!(g3, Point { x: Some(80), y: Some(87) });
    let g4 = scalar_mult(4, &curve.g, curve);
    assert_eq!(g4, Point { x: Some(3), y: Some(91) });
    println!("  ✓ Group structure: 1×G=(3,6), 2×G=(80,10), 3×G=(80,87), 4×G=(3,91)");
    println!();
}

fn ecdsa_demo(curve: &Curve) {
    println!("  --- ECDSA Key Generation ---");
    let d = 3_i64;
    let Q = ECDSA::keygen(curve, d);
    println!("  Private key d = {}", d);
    println!("  Public key  Q = {} × G = {}", d, Q);
    assert!(!Q.x.is_none());
    assert!(curve.is_on_curve(Q.x.unwrap(), Q.y.unwrap()));
    println!("  ✓ Q is on the curve\n");

    println!("  --- ECDSA Signing & Verification ---");
    let msg = b"ECDSA needs a random nonce";
    let k = 2_i64;
    let (r, s) = ECDSA::sign(curve, d, msg, k);
    println!("  Message: \"{}\"", String::from_utf8_lossy(msg));
    println!("  Nonce k = {}", k);
    println!("  Signature (r={}, s={})", r, s);
    assert!(ECDSA::verify(curve, &Q, msg, r, s));
    println!("  ✓ Signature verified\n");

    println!("  --- Wrong Signature Rejected ---");
    assert!(!ECDSA::verify(curve, &Q, b"wrong message", r, s));
    println!("  ✓ Wrong message rejected\n");

    println!("  --- Nonce Reuse Attack ---");
    let msg1 = b"message one";
    let msg2 = b"message two";
    // Evil: sign both messages with the SAME nonce k = 2
    let sig1 = ECDSA::sign(curve, d, msg1, 2);
    let sig2 = ECDSA::sign(curve, d, msg2, 2);
    println!("  msg1 \"{}\" → (r={}, s={})", String::from_utf8_lossy(msg1), sig1.0, sig1.1);
    println!("  msg2 \"{}\" → (r={}, s={})", String::from_utf8_lossy(msg2), sig2.0, sig2.1);
    assert_eq!(sig1.0, sig2.0, "nonce reuse: r values are identical");
    println!("  ⚠  Same r = {} — nonce was reused!\n", sig1.0);

    // Recover k and d from the two signatures
    let (recovered_k, recovered_d) = ECDSA::recover_from_nonce_reuse(curve, msg1, sig1, msg2, sig2);
    println!("  Recovered k = {} (original was {})", recovered_k, k);
    println!("  Recovered d = {} (original was {})", recovered_d, d);
    assert_eq!(recovered_k, k);
    assert_eq!(recovered_d, d);
    println!("  ✓ Private key stolen! The attacker can now sign anything.");
    println!();
}

fn main() {
    println!("{}", "=".repeat(60));
    println!("  Digital Signatures — ECDSA, EdDSA, BLS");
    println!("  Phase 12 — Cryptography & Security, Lesson 12");
    println!("{}", "=".repeat(60));

    let curve = Curve::new();

    // ==================================================================
    // Step 1: ECDSA
    // ==================================================================
    println!("\n{}", "-".repeat(60));
    println!("  Step 1: ECDSA on a Toy Curve");
    println!("{}", "-".repeat(60));

    confirm_toy_curve(&curve);
    ecdsa_demo(&curve);

    // ==================================================================
    // Step 2: Ed25519
    // ==================================================================
    use rand::rngs::OsRng;
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    ed25519_demo(&signing_key);

    // ==================================================================
    // Step 3: BLS
    // ==================================================================
    bls_demo();

    // ==================================================================
    // Summary comparison
    // ==================================================================
    println!("\n{}", "-".repeat(60));
    println!("  Summary: Signature Scheme Comparison");
    println!("{}", "-".repeat(60));
    println!("  ┌──────────┬──────────────┬──────────────┬────────────────┬───────────────┐");
    println!("  │ Scheme   │ Public Key   │ Signature    │ Aggregatable?  │ Deterministic │");
    println!("  ├──────────┼──────────────┼──────────────┼────────────────┼───────────────┤");
    println!("  │ ECDSA    │ 32 B         │ 64 B         │ No             │ No            │");
    println!("  │ Ed25519  │ 32 B         │ 64 B         │ No             │ Yes           │");
    println!("  │ BLS12-381│ 48 B         │ 96 B (48 B)  │ Yes            │ Yes           │");
    println!("  └──────────┴──────────────┴──────────────┴────────────────┴───────────────┘");
    println!();

    println!("All tests passed!");
}
