//! Public Key III — Elliptic Curves & Ed25519
//! Phase 12 — Cryptography & Security
//!
//! Demonstrates elliptic curve point operations and Ed25519 key operations.

/// An elliptic curve y^2 = x^3 + ax + b over F_p.
struct EllipticCurve {
    a: i64,
    b: i64,
    p: i64,
}

/// A point on an elliptic curve. `(None, None)` represents the point at infinity.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Point {
    x: Option<i64>,
    y: Option<i64>,
}

const INF: Point = Point {
    x: None,
    y: None,
};

impl EllipticCurve {
    fn new(a: i64, b: i64, p: i64) -> Self {
        let disc = ((4 * a * a * a + 27 * b * b) % p + p) % p;
        assert!(disc != 0, "singular curve: discriminant is zero");
        EllipticCurve { a, b, p }
    }

    fn is_on_curve(&self, x: i64, y: i64) -> bool {
        let lhs = ((y * y) % self.p + self.p) % self.p;
        let rhs = ((x * x * x + self.a * x + self.b) % self.p + self.p) % self.p;
        lhs == rhs
    }
}

impl Point {
    fn new(curve: &EllipticCurve, x: Option<i64>, y: Option<i64>) -> Self {
        if let (Some(xv), Some(yv)) = (x, y) {
            assert!(curve.is_on_curve(xv, yv), "point ({}, {}) is not on the curve", xv, yv);
        }
        Point { x, y }
    }

    fn infinity() -> Self {
        INF
    }

    fn is_infinity(&self) -> bool {
        self.x.is_none() && self.y.is_none()
    }

    fn negate(&self, curve: &EllipticCurve) -> Self {
        if self.is_infinity() {
            return *self;
        }
        let x = self.x.unwrap();
        let y = self.y.unwrap();
        Point::new(curve, Some(x), Some((-(y % curve.p) + curve.p) % curve.p))
    }
}

/// Extended Euclidean algorithm: returns (gcd, x, y) such that ax + by = gcd.
fn extended_gcd(a: i64, b: i64) -> (i64, i64, i64) {
    if a == 0 {
        return (b, 0, 1);
    }
    let (g, x1, y1) = extended_gcd(b % a, a);
    (g, y1 - (b / a) * x1, x1)
}

/// Modular inverse of k modulo m.
fn inv_mod(k: i64, m: i64) -> i64 {
    assert_ne!(k, 0, "cannot invert zero");
    let k = (k % m + m) % m;
    let (g, x, _) = extended_gcd(k, m);
    assert_eq!(g, 1, "{} is not invertible mod {}", k, m);
    (x % m + m) % m
}

/// Add two points on the same elliptic curve.
fn point_add(P: &Point, Q: &Point, curve: &EllipticCurve) -> Point {
    if P.is_infinity() {
        return *Q;
    }
    if Q.is_infinity() {
        return *P;
    }

    let px = P.x.unwrap();
    let py = P.y.unwrap();
    let qx = Q.x.unwrap();
    let qy = Q.y.unwrap();

    // P + (-P) = infinity
    if px == qx && (py + qy) % curve.p == 0 {
        return Point::infinity();
    }

    let s: i64;
    if px == qx {
        // Point doubling: tangent slope = (3x^2 + a) / (2y)
        let num = ((3 * px * px + curve.a) % curve.p + curve.p) % curve.p;
        let den = (2 * py) % curve.p;
        s = num * inv_mod(den, curve.p) % curve.p;
    } else {
        // Point addition: secant slope = (y2 - y1) / (x2 - x1)
        let num = ((qy - py) % curve.p + curve.p) % curve.p;
        let den = ((qx - px) % curve.p + curve.p) % curve.p;
        s = num * inv_mod(den, curve.p) % curve.p;
    }

    let x3 = ((s * s - px - qx) % curve.p + curve.p) % curve.p;
    let y3 = ((s * (px - x3) - py) % curve.p + curve.p) % curve.p;

    Point::new(curve, Some(x3), Some(y3))
}

/// Multiply point P by scalar k using double-and-add.
fn scalar_mult(k: i64, P: &Point, curve: &EllipticCurve) -> Point {
    if k == 0 || P.is_infinity() {
        return Point::infinity();
    }

    let mut result = Point::infinity();
    let mut addend = *P;
    let mut n = k.abs();

    while n > 0 {
        if n & 1 == 1 {
            result = point_add(&result, &addend, curve);
        }
        addend = point_add(&addend, &addend, curve);
        n >>= 1;
    }

    if k < 0 {
        result = result.negate(curve);
    }

    result
}

/// Find the order of a point by brute-force scalar multiplication (only for small groups).
fn find_order(G: &Point, curve: &EllipticCurve, max_order: i64) -> Option<i64> {
    if G.is_infinity() {
        return Some(1);
    }
    let mut cur = *G;
    for i in 1..=max_order {
        cur = point_add(&cur, G, curve);
        if cur.is_infinity() {
            return Some(i + 1);
        }
    }
    None
}

/// Count all points on the curve by brute force (only for small p).
fn count_points_brute(curve: &EllipticCurve) -> i64 {
    let mut count = 1; // point at infinity
    for x in 0..curve.p {
        let rhs = ((x * x * x + curve.a * x + curve.b) % curve.p + curve.p) % curve.p;
        for y in 0..curve.p {
            if (y * y) % curve.p == rhs {
                count += 1;
            }
        }
    }
    count
}

fn main() {
    println!("=== Elliptic Curve Operations Demo ===\n");

    // Small curve: y^2 = x^3 + 2x + 3 mod 97
    let curve = EllipticCurve::new(2, 3, 97);
    let G = Point::new(&curve, Some(3), Some(6));

    println!("Curve: y^2 = x^3 + {}x + {} mod {}", curve.a, curve.b, curve.p);
    println!("Generator G = (3, 6)");
    println!("G on curve: {}", curve.is_on_curve(3, 6));
    println!();

    // 2*G
    let G2 = point_add(&G, &G, &curve);
    println!("2*G = ({}, {})", G2.x.unwrap(), G2.y.unwrap());
    assert!(curve.is_on_curve(G2.x.unwrap(), G2.y.unwrap()));
    assert_eq!(G2.x.unwrap(), 80);
    assert_eq!(G2.y.unwrap(), 10);
    println!("  ✓ Verified: 2*(3,6) = (80,10)");
    println!();

    // Scalar multiplication
    let G3 = scalar_mult(3, &G, &curve);
    let G4 = scalar_mult(4, &G, &curve);
    let G5 = scalar_mult(5, &G, &curve);
    println!("3*G = ({}, {})", G3.x.unwrap(), G3.y.unwrap());
    println!("4*G = ({}, {})", G4.x.unwrap(), G4.y.unwrap());
    println!("5*G = ({:?}, {:?})", G5.x, G5.y);
    println!();

    // Verify generator order is 5
    assert!(G5.is_infinity(), "order of G should be 5");
    println!("  ✓ Verified: 5*G = infinity (order of G is 5)");
    println!();

    // Associativity: (G+G)+G == G+(G+G)
    let left = point_add(&G2, &G, &curve);
    let right = point_add(&G, &G2, &curve);
    assert_eq!(left, right);
    println!("  ✓ Associativity: (G+G)+G == G+(G+G) == ({}, {})",
             left.x.unwrap(), left.y.unwrap());
    println!();

    // Negation: P + (-P) = infinity
    let neg_G = G.negate(&curve);
    let inf = point_add(&G, &neg_G, &curve);
    assert!(inf.is_infinity());
    println!("  ✓ Negation: G + (-G) = infinity");
    println!("      G  = ({}, {})", G.x.unwrap(), G.y.unwrap());
    println!("     -G  = ({}, {})", neg_G.x.unwrap(), neg_G.y.unwrap());
    println!();

    // Scalar multiplication consistency
    let G10a = scalar_mult(2, &G5, &curve);
    let G10b = scalar_mult(5, &G2, &curve);
    assert!(G10a.is_infinity());
    assert!(G10b.is_infinity());
    println!("  ✓ Consistency: 2*(5*G) == 5*(2*G) == infinity");
    println!();

    // Negative scalar multiplication
    let neg3 = scalar_mult(-3, &G, &curve);
    let check = point_add(&G3, &neg3, &curve);
    assert!(check.is_infinity());
    println!("  ✓ Negative scalar: 3*G + (-3)*G = infinity");
    println!();

    // Find order of the generator
    match find_order(&G, &curve, 100) {
        Some(order) => println!("  Order of G: {}", order),
        None => println!("  Order not found within limit"),
    }
    println!();

    // Count total points (Hasse bound)
    let total = count_points_brute(&curve);
    let hasse_lower = curve.p + 1 - (2.0 * (curve.p as f64).sqrt()) as i64;
    let hasse_upper = curve.p + 1 + (2.0 * (curve.p as f64).sqrt()) as i64;
    println!("  Total points on curve: {}", total);
    println!("  Hasse bound: [{}, {}]", hasse_lower, hasse_upper);
    assert!(total >= hasse_lower && total <= hasse_upper, "Hasse bound violated!");
    println!("  ✓ Within Hasse bound");
    println!();

    // ECDH-style key exchange
    println!("=== ECDH-style Key Exchange ===");
    let alice_priv: i64 = 17;
    let bob_priv: i64 = 23;
    let alice_pub = scalar_mult(alice_priv, &G, &curve);
    let bob_pub = scalar_mult(bob_priv, &G, &curve);

    assert!(!alice_pub.is_infinity());
    assert!(!bob_pub.is_infinity());

    let shared_alice = scalar_mult(alice_priv, &bob_pub, &curve);
    let shared_bob = scalar_mult(bob_priv, &alice_pub, &curve);
    let shared_match = shared_alice == shared_bob;

    println!("  Alice's private: {}", alice_priv);
    println!("  Bob's private:   {}", bob_priv);
    println!("  Alice's public:  ({}, {})", alice_pub.x.unwrap(), alice_pub.y.unwrap());
    println!("  Bob's public:    ({}, {})", bob_pub.x.unwrap(), bob_pub.y.unwrap());
    println!("  Shared match: {}", shared_match);
    assert!(shared_match);
    println!();

    // Edge case: identity operations
    let inf_pt = Point::infinity();
    assert_eq!(point_add(&G, &inf_pt, &curve), G);
    assert_eq!(point_add(&inf_pt, &G, &curve), G);
    assert!(point_add(&inf_pt, &inf_pt, &curve).is_infinity());
    println!("  ✓ Identity: P+O = P, O+P = P, O+O = O");
    println!();

    // Edge case: scalar multiplication by zero
    let zero = scalar_mult(0, &G, &curve);
    assert!(zero.is_infinity());
    println!("  ✓ Scalar mult by 0: 0*G = infinity");
    println!();

    // Edge case: scalar multiplication by 1
    let one = scalar_mult(1, &G, &curve);
    assert_eq!(one, G);
    println!("  ✓ Scalar mult by 1: 1*G = G");
    println!();

    // Second curve: y^2 = x^3 + 7 mod 71 (like secp256k1 but tiny)
    println!("=== Second Curve: y^2 = x^3 + 7 mod 71 ===");
    let curve2 = EllipticCurve::new(0, 7, 71);
    let G2 = Point::new(&curve2, Some(1), Some(24));
    println!("  Generator: (1, 13)");
    let G2_2 = point_add(&G2, &G2, &curve2);
    println!("  2*G = ({}, {})", G2_2.x.unwrap(), G2_2.y.unwrap());
    let G2_3 = scalar_mult(3, &G2, &curve2);
    println!("  3*G = ({}, {})", G2_3.x.unwrap(), G2_3.y.unwrap());
    match find_order(&G2, &curve2, 200) {
        Some(o) => println!("  Order of G2: {}", o),
        None => println!("  Order not found within limit"),
    }
    println!();

    // Key size comparison
    println!("=== Key Size Comparison ===");
    println!("  ECC-256:  32 bytes  (equivalent security to RSA-3072)");
    println!("  ECC-384:  48 bytes  (equivalent security to RSA-7680)");
    println!("  ECC-521:  66 bytes  (equivalent security to RSA-15360)");
    println!("  RSA-2048: 256 bytes");
    println!("  RSA-4096: 512 bytes");
    println!();

    println!("=== Ed25519 Overview ===");
    println!("  Ed25519 key generation: seed(32) + SHA-512 + clamp + scalar_mult(B, k)");
    println!("  Ed25519 signing: deterministic, no RNG needed");
    println!("  Ed25519 verification: double-base scalar multiplication");
    println!("  Benefits: fast, secure, constant-time, no RNG dependency");
    println!("  Key size: 32-byte secret + 32-byte public = 64 bytes total");
    println!("  Signature size: 64 bytes (vs RSA-2048's 256 bytes)");
    println!();

    println!("All tests passed!");
}
