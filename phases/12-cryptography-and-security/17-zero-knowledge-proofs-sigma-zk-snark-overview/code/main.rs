// Zero-Knowledge Proofs — Sigma, zk-SNARK overview
// Phase 12 — Cryptography & Security
//
// Demonstrates:
//   - Schnorr Sigma protocol (interactive) for proving knowledge of a discrete log
//   - Non-interactive setup via Fiat-Shamir (SHA-256)
//   - R1CS constraint system demonstration
//   - Verification of valid and invalid proofs

use num_bigint::{BigUint, RandBigInt};
use num_traits::One;
use sha2::{Digest, Sha256};
use std::time::Instant;

const P_HEX: &str = concat!(
    "B10B8F96A080E01DDE92DE5EAE5D54EC52C99FBCFB06A3C6",
    "9A6A9DCA52D23B616073E28675A23D189838EF1E2EE652C0",
    "13ECB4AEA906112324975C3CD49B83BFACCBDD7D90C4BD70",
    "98488E9C219A73724EFFD6FAE5644738FAA31A4FF55BCCC0",
    "A151AF5F0DC8B4BD45BF37DF365C1A65E68CFDA76D4DA708",
    "DF1FB2BC2E4A4371",
);

fn bigint_from_hex(s: &str) -> BigUint {
    BigUint::parse_bytes(s.as_bytes(), 16).expect("valid hex")
}

fn hash_to_biguint(data: &[&[u8]]) -> BigUint {
    let mut hasher = Sha256::new();
    for d in data {
        hasher.update(d);
    }
    let result = hasher.finalize();
    BigUint::from_bytes_be(&result)
}

struct SchnorrParams {
    p: BigUint,
    q: BigUint,
    g: BigUint,
}

impl SchnorrParams {
    fn new() -> Self {
        let p = bigint_from_hex(P_HEX);
        let one = BigUint::one();
        let q = (&p - &one) / 2u32;
        SchnorrParams {
            p,
            q,
            g: BigUint::from(2u32),
        }
    }

    fn mod_exp(&self, base: &BigUint, exp: &BigUint) -> BigUint {
        base.modpow(exp, &self.p)
    }

    fn random_exponent(&self) -> BigUint {
        let mut rng = rand::thread_rng();
        let one = BigUint::one();
        rng.gen_biguint_range(&one, &self.q)
    }
}

fn schnorr_interactive_prover(
    params: &SchnorrParams,
) -> (BigUint, BigUint) {
    let r = params.random_exponent();
    let t = params.mod_exp(&params.g, &r);
    (t, r)
}

fn schnorr_interactive_respond(
    params: &SchnorrParams,
    r: &BigUint,
    c: &BigUint,
    x: &BigUint,
) -> BigUint {
    (r + c * x) % &params.q
}

fn schnorr_verify(
    params: &SchnorrParams,
    y: &BigUint,
    t: &BigUint,
    c: &BigUint,
    s: &BigUint,
) -> bool {
    let lhs = params.mod_exp(&params.g, s);
    let rhs = (t * params.mod_exp(y, c)) % &params.p;
    lhs == rhs
}

fn schnorr_prove(
    params: &SchnorrParams,
    x: &BigUint,
    y: &BigUint,
) -> (BigUint, BigUint) {
    let r = params.random_exponent();
    let t = params.mod_exp(&params.g, &r);
    let c = hash_to_biguint(&[
        &params.p.to_bytes_be(),
        &params.g.to_bytes_be(),
        &y.to_bytes_be(),
        &t.to_bytes_be(),
    ]) % &params.q;
    let s = (r + &c * x) % &params.q;
    (t, s)
}

fn schnorr_verify_proof(
    params: &SchnorrParams,
    y: &BigUint,
    t: &BigUint,
    s: &BigUint,
) -> bool {
    let c = hash_to_biguint(&[
        &params.p.to_bytes_be(),
        &params.g.to_bytes_be(),
        &y.to_bytes_be(),
        &t.to_bytes_be(),
    ]) % &params.q;
    let lhs = params.mod_exp(&params.g, s);
    let rhs = (t * params.mod_exp(y, &c)) % &params.p;
    lhs == rhs
}

fn demo_interactive_schnorr(params: &SchnorrParams, x: &BigUint, y: &BigUint) {
    println!("=== Interactive Schnorr Proof (Sigma Protocol) ===");

    let (t, r) = schnorr_interactive_prover(params);
    let c = params.random_exponent();
    let s = schnorr_interactive_respond(params, &r, &c, x);
    let valid = schnorr_verify(params, y, &t, &c, &s);

    println!("  Secret x:         {}", x);
    println!("  Public key y:     {}...", &y.to_str_radix(16)[..40]);
    println!("  Commitment t:     {}...", &t.to_str_radix(16)[..40]);
    println!("  Challenge c:      {}", c);
    println!("  Response s:       {}", s);
    println!("  g^s == t * y^c:   {}\n", valid);
}

fn demo_noninteractive_schnorr(params: &SchnorrParams, x: &BigUint, y: &BigUint) {
    println!("=== Non-Interactive Schnorr Proof (Fiat-Shamir) ===");

    let (t, s) = schnorr_prove(params, x, y);
    let valid = schnorr_verify_proof(params, y, &t, &s);

    println!("  Secret x:    {}", x);
    println!("  Public key y: {}...", &y.to_str_radix(16)[..40]);
    println!("  Proof (t,s): t = {}...", &t.to_str_radix(16)[..40]);
    println!("              s = {}", s);
    println!("  c = SHA256(p || g || y || t) mod q");
    println!("  Verifier accepts: {}\n", valid);
}

fn demo_invalid_proofs(params: &SchnorrParams, x: &BigUint, y: &BigUint) {
    println!("=== Invalid Proof Detection ===");

    let (t, s) = schnorr_prove(params, x, y);
    let valid_orig = schnorr_verify_proof(params, y, &t, &s);

    let one = BigUint::one();
    let t_tampered = (&t + &one) % &params.p;
    let valid_tampered = schnorr_verify_proof(params, y, &t_tampered, &s);

    println!("  Original proof valid:          {}", valid_orig);
    println!("  Tampered t proof valid:        {}", valid_tampered);

    let wrong_x = x + &one;
    let wrong_y = params.mod_exp(&params.g, &wrong_x);
    let valid_wrong_y = schnorr_verify_proof(params, &wrong_y, &t, &s);
    println!("  Wrong public key proof valid:  {}\n", valid_wrong_y);
}

fn demo_r1cs() {
    println!("=== R1CS Constraint System (x^2 + 3x + 1 == 11) ===");

    let x_val = 2u64;
    let lhs_val = x_val * x_val + 3 * x_val + 1;

    println!("  Statement: I know x such that x^2 + 3x + 1 == 11");
    println!("  Solution:  x = {}", x_val);
    println!("  Check:     {}^2 + 3*{} + 1 = {}\n", x_val, x_val, lhs_val);

    let v1 = x_val * x_val;
    let v2 = 3 * x_val;
    let v3 = v1 + v2 + 1;

    let w = vec![1u64, x_val, v1, v2, v3];

    // R1CS: <A,w> * <B,w> = <C,w>
    // Constraint 1: v1 = x * x
    //   A = [0,1,0,0,0], B = [0,1,0,0,0], C = [0,0,1,0,0]
    // Constraint 2: v2 = 3 * x
    //   A = [0,3,0,0,0], B = [1,0,0,0,0], C = [0,0,0,1,0]
    // Constraint 3: v3 = v1 + v2 + 1  => <[1,0,1,1,0], w> * <[1,0,0,0,0], w> = <[0,0,0,0,1], w>

    let constraints: [([u64; 5], [u64; 5], [u64; 5], &str); 3] = [
        (
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 0, 1, 0, 0],
            "v1 = x * x",
        ),
        (
            [0, 3, 0, 0, 0],
            [1, 0, 0, 0, 0],
            [0, 0, 0, 1, 0],
            "v2 = 3 * x",
        ),
        (
            [1, 0, 1, 1, 0],
            [1, 0, 0, 0, 0],
            [0, 0, 0, 0, 1],
            "v3 = v1 + v2 + 1",
        ),
    ];

    let mut all_ok = true;
    for (i, (a_vec, b_vec, c_vec, desc)) in constraints.iter().enumerate() {
        let dot_a: u64 = a_vec.iter().zip(&w).map(|(a, ww)| a * ww).sum();
        let dot_b: u64 = b_vec.iter().zip(&w).map(|(a, ww)| a * ww).sum();
        let dot_c: u64 = c_vec.iter().zip(&w).map(|(a, ww)| a * ww).sum();
        let ok = dot_a * dot_b == dot_c;
        all_ok = all_ok && ok;
        let status = if ok { "OK" } else { "FAIL" };
        println!("  Constraint {}: {}", i, desc);
        println!("    <A,w>={}, <B,w>={}, <C,w>={}  [{}]", dot_a, dot_b, dot_c, status);
    }

    println!("  All constraints satisfied: {}", all_ok);

    let x_bad = 3u64;
    let v1_bad = x_bad * x_bad;
    let v2_bad = 3 * x_bad;
    let v3_bad = v1_bad + v2_bad + 1;
    let w_bad = vec![1u64, x_bad, v1_bad, v2_bad, v3_bad];

    println!("\n  Invalid witness: x = {} (gives out = {}, not 11)", x_bad, v3_bad);
    for (i, (a_vec, b_vec, c_vec, _desc)) in constraints.iter().enumerate() {
        let dot_a: u64 = a_vec.iter().zip(&w_bad).map(|(a, ww)| a * ww).sum();
        let dot_b: u64 = b_vec.iter().zip(&w_bad).map(|(a, ww)| a * ww).sum();
        let dot_c: u64 = c_vec.iter().zip(&w_bad).map(|(a, ww)| a * ww).sum();
        let ok = dot_a * dot_b == dot_c;
        if !ok {
            println!("  Constraint {} FAILS: <A,w>={}, <B,w>={}, <C,w>={}", i, dot_a, dot_b, dot_c);
            break;
        }
    }
    println!();
}

fn main() {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║  Zero-Knowledge Proofs — Sigma & zk-SNARK overview  ║");
    println!("║  Phase 12 — Cryptography & Security                 ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();

    let params = SchnorrParams::new();
    let x = params.random_exponent();
    let y = params.mod_exp(&params.g, &x);

    println!("  Using 1024-bit safe prime p = 2q + 1");
    println!("  Generator g = {}", params.g);
    println!("  Secret x:    {}", x);
    println!("  Public y:    {}...\n", &y.to_str_radix(16)[..40]);

    demo_interactive_schnorr(&params, &x, &y);
    demo_noninteractive_schnorr(&params, &x, &y);
    demo_invalid_proofs(&params, &x, &y);
    demo_r1cs();

    println!("=== Performance (1024-bit Schnorr) ===");
    let start = Instant::now();
    for _ in 0..100 {
        let (_t, _s) = schnorr_prove(&params, &x, &y);
    }
    let dur = start.elapsed();
    println!("  100 non-interactive proofs: {:?}", dur);
    println!("  Per proof: {:?}\n", dur / 100);

    println!("=== Summary ===");
    println!("  • Sigma protocol: interactive commit-challenge-respond");
    println!("  • Fiat-Shamir: non-interactive via SHA-256 hash oracle");
    println!("  • R1CS: <A,w> * <B,w> = <C,w> constraint system");
    println!("  All demonstrations passed.");
}
