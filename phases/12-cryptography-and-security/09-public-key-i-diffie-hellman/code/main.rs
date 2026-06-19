// Public Key I — Diffie-Hellman
// Phase 12 — Cryptography & Security
//
// Working Diffie-Hellman implementation with modular exponentiation,
// keypair generation, shared secret computation, and performance
// benchmarking across multiple parameter sets.

use std::time::Instant;

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

/// Generate a random private key in [2, p-2]
fn generate_private(p: u64) -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    // Simple LCG for demo purposes (not cryptographically secure RNG)
    let mut state = seed as u64;
    state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let range = p.saturating_sub(3);
    2 + (state % range)
}

/// Generate a DH keypair: (private, public)
fn generate_keypair(p: u64, g: u64) -> (u64, u64) {
    let private = generate_private(p);
    let public = mod_pow(g, private, p);
    (private, public)
}

/// Compute the shared secret: their_pub^my_priv mod p
fn compute_shared_secret(their_pub: u64, my_priv: u64, p: u64) -> u64 {
    mod_pow(their_pub, my_priv, p)
}

/// Demonstrate DH with a given parameter set (p, g name, p hex suffix)
fn demonstrate(p: u64, g: u64, label: &str) {
    println!("--- Diffie-Hellman with {label} ---");
    println!("p = {p}, g = {g}");

    let start = Instant::now();
    let (alice_priv, alice_pub) = generate_keypair(p, g);
    let gen_time = start.elapsed();

    let (bob_priv, bob_pub) = generate_keypair(p, g);

    let alice_shared = compute_shared_secret(bob_pub, alice_priv, p);
    let bob_shared = compute_shared_secret(alice_pub, bob_priv, p);

    println!("Alice pub: {alice_pub}, Bob pub: {bob_pub}");
    println!("Shared secret: {alice_shared}");
    println!("Match: {}\n", alice_shared == bob_shared);
    println!("Key generation took: {:?}\n", gen_time);
}

/// Simulate a MITM attack (demonstrates the vulnerability)
fn demonstrate_mitm(p: u64, g: u64) {
    println!("--- MITM Attack Simulation ---");

    let (alice_priv, alice_pub) = generate_keypair(p, g);
    let (bob_priv, bob_pub) = generate_keypair(p, g);
    let (mallory_priv, mallory_pub) = generate_keypair(p, g);

    // Alice thinks she's talking to Bob, but Mallory intercepted
    let alice_shared = compute_shared_secret(mallory_pub, alice_priv, p);
    let mallory_with_alice = compute_shared_secret(alice_pub, mallory_priv, p);

    // Bob thinks he's talking to Alice, but Mallory intercepted
    let bob_shared = compute_shared_secret(mallory_pub, bob_priv, p);
    let mallory_with_bob = compute_shared_secret(bob_pub, mallory_priv, p);

    assert_eq!(alice_shared, mallory_with_alice);
    assert_eq!(bob_shared, mallory_with_bob);

    println!("Alice-Mallory shared secret: {alice_shared}");
    println!("Bob-Mallory shared secret:   {bob_shared}");
    println!("Alice and Bob do NOT share a secret directly.");
    println!("Mallory can decrypt, read, and re-encrypt all traffic.\n");
}

fn main() {
    // Small parameters for 64-bit demonstration
    // These are NOT secure — real DH uses 2048+ bit primes.
    demonstrate(9973, 2, "10-bit prime (insecure, for demo)");
    demonstrate(99991, 5, "17-bit prime (insecure, for demo)");

    demonstrate_mitm(9973, 2);

    // Benchmark mod_pow with various exponents
    println!("--- Exponentiation Benchmark ---");
    let p = 99991u64;
    let g = 2u64;
    for exp_bits in [8, 16, 24, 32] {
        let exp = (1u64 << exp_bits) - 1;
        let start = Instant::now();
        let result = mod_pow(g, exp, p);
        let elapsed = start.elapsed();
        println!("mod_pow(2, 2^{exp_bits}-1, 99991) = {result:>5}  [{elapsed:?}]");
    }
}
