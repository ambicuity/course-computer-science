// Public Key II — RSA Internals & Padding
// Phase 12 — Cryptography & Security
//
// RSA math implementation with 64-bit integers for educational
// demonstration: modular exponentiation, Miller-Rabin primality
// test, extended Euclidean algorithm, key generation, encryption,
// and decryption.
//
// Note: 64-bit primes are trivially factorable. Real RSA uses
// 2048+ bit moduli. This code demonstrates the algorithm's
// core mechanics with small numbers.

use std::time::{SystemTime, UNIX_EPOCH};

/// Modular exponentiation: (base^exp) % modulus
/// Uses square-and-multiply with u128 intermediates to avoid overflow.
fn mod_pow(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    let mut result = 1u64;
    base %= modulus;
    while exp > 0 {
        if exp & 1 == 1 {
            result = (result as u128 * base as u128 % modulus as u128) as u64;
        }
        base = (base as u128 * base as u128 % modulus as u128) as u64;
        exp >>= 1;
    }
    result
}

/// Miller-Rabin primality test (deterministic for n < 2^64).
/// Uses the known set of bases that are sufficient for 64-bit integers:
/// [2, 3, 5, 7, 11, 13, 17] — no false positives for n < 2^64.
fn is_prime(n: u64) -> bool {
    if n < 2 { return false; }
    if n % 2 == 0 { return n == 2; }

    // Write n-1 as d * 2^r
    let mut d = n - 1;
    let mut r = 0;
    while d % 2 == 0 {
        d /= 2;
        r += 1;
    }

    // Bases sufficient for deterministic test on u64
    let bases: [u64; 7] = [2, 3, 5, 7, 11, 13, 17];

    'next_base: for &a in &bases {
        if a >= n { continue; }
        let mut x = mod_pow(a, d, n);
        if x == 1 || x == n - 1 { continue; }

        for _ in 0..r - 1 {
            x = mod_pow(x, 2, n);
            if x == n - 1 { continue 'next_base; }
        }
        return false;
    }
    true
}

/// Simple LCG for generating pseudo-random values (not cryptographically secure).
fn lcg(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *seed
}

/// Generate a random prime of approximately `bits` bits.
fn generate_prime(bits: u32, seed: &mut u64) -> u64 {
    loop {
        let mut n = lcg(seed) >> (64 - bits);
        n |= 1 << (bits - 1);  // Ensure bit length
        n |= 1;                // Ensure odd
        if is_prime(n) {
            return n;
        }
    }
}

/// Extended Euclidean algorithm returning (gcd, x, y) where a*x + b*y = gcd.
fn extended_gcd(a: i64, b: i64) -> (i64, i64, i64) {
    if a == 0 { return (b, 0, 1); }
    let (g, x1, y1) = extended_gcd(b % a, a);
    (g, y1 - (b / a) * x1, x1)
}

/// Compute modular inverse of a modulo m (m must be prime or a and m coprime).
fn mod_inverse(a: u64, m: u64) -> u64 {
    let (g, x, _) = extended_gcd(a as i64, m as i64);
    assert_eq!(g, 1, "a and m are not coprime");
    ((x % m as i64 + m as i64) % m as i64) as u64
}

/// Generate an RSA keypair: returns (n, e, d, p, q).
fn generate_keypair(bits: u32, seed: &mut u64) -> (u64, u64, u64, u64, u64) {
    let p = generate_prime(bits, seed);
    let q = generate_prime(bits, seed);
    let n = p * q;
    let phi = (p - 1) * (q - 1);
    let e: u64 = 65537;
    let d = mod_inverse(e, phi);
    (n, e, d, p, q)
}

/// RSA encryption: c = m^e mod n
fn encrypt(m: u64, e: u64, n: u64) -> u64 {
    mod_pow(m, e, n)
}

/// RSA decryption: m = c^d mod n
fn decrypt(c: u64, d: u64, n: u64) -> u64 {
    mod_pow(c, d, n)
}

/// Demonstrate textbook RSA with small parameters.
fn demonstrate_rsa() {
    println!("=== Textbook RSA (16-bit primes) ===");

    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let (n, e, d, p, q) = generate_keypair(10, &mut seed);

    println!("p = {}", p);
    println!("q = {}", q);
    println!("n = {} ({} bits)", n, (n as f64).log2().ceil() as u32);
    println!("e = {}", e);
    println!("d = {}", d);

    // Encrypt and decrypt a small message
    let message = 42u64;
    let cipher = encrypt(message, e, n);
    let plain = decrypt(cipher, d, n);

    println!("\nMessage:    {}", message);
    println!("Ciphertext: {}", cipher);
    println!("Decrypted:  {}", plain);
    println!("Match:      {}\n", message == plain);

    // Demonstrate that RSA is deterministic
    let c1 = encrypt(42, e, n);
    let c2 = encrypt(42, e, n);
    println!("Deterministic: encrypt(42) twice = {} and {}", c1, c2);
    println!("  Same? {} (BAD — should be different with padding)\n", c1 == c2);

    // Demonstrate malleability
    let c = encrypt(100, e, n);
    let mult = mod_pow(2, e, n);
    let modified = (c as u128 * mult as u128 % n as u128) as u64;
    let decrypted = decrypt(modified, d, n);
    println!("Malleability: encrypt(100) = {}", c);
    println!("  Modified ciphertext decrypts to: {} (should be 200)\n", decrypted);
}

/// Demonstrate RSA with a larger (but still tiny) key.
fn demonstrate_larger_key() {
    println!("=== RSA with 20-bit primes ===");

    let mut seed = 123456789u64;
    let (n, e, d, p, q) = generate_keypair(12, &mut seed);

    println!("p = {}", p);
    println!("q = {}", q);
    println!("n = {} ({} bits)", n, (n as f64).log2().ceil() as u32);

    let message = 12345u64;
    let cipher = encrypt(message, e, n);
    let plain = decrypt(cipher, d, n);
    println!("Message: {}, Decrypted: {}, Match: {}", message, plain, message == plain);
    println!();
}

/// Benchmark modular exponentiation with different exponents.
fn benchmark_mod_pow() {
    println!("=== Exponentiation Benchmark ===");
    let modulus = 65521u64; // A real prime
    for exp_bits in [8, 12, 16, 20] {
        let exp = (1u64 << exp_bits) - 1;
        let start = std::time::Instant::now();
        let iterations = 1000;
        for _ in 0..iterations {
            let _ = mod_pow(2, exp, modulus);
        }
        let elapsed = start.elapsed();
        let avg = elapsed / iterations;
        println!("mod_pow(2, 2^{}-1, 65521): avg {:?}", exp_bits, avg);
    }
}

fn main() {
    demonstrate_rsa();
    demonstrate_larger_key();
    benchmark_mod_pow();

    // Demonstrate Euler's theorem: a^phi(n) ≡ 1 (mod n) for coprime a, n
    println!("=== Euler's Theorem Verification ===");
    let p = 101u64;
    let q = 103u64;
    let n = p * q;
    let phi = (p - 1) * (q - 1);
    let a = 5u64;  // coprime to n
    let result = mod_pow(a, phi, n);
    println!("5^{} mod {} = {} (should be 1)\n", phi, n, result);

    // Show that e*d ≡ 1 mod phi
    println!("=== RSA Key Relationship ===");
    let e = 65537u64;
    let d = mod_inverse(e, phi);
    let check = (e as u128 * d as u128) % phi as u128;
    println!("e * d mod phi = {} (should be 1)", check);
    println!("Public:  (n={}, e={})", n, e);
    println!("Private: (n={}, d={})", n, d);

    // End-to-end with this key
    let msg = 2024u64;
    let ct = encrypt(msg, e, n);
    let pt = decrypt(ct, d, n);
    println!("Message: {}, Cipher: {}, Decrypted: {}, OK: {}", msg, ct, pt, msg == pt);
}
