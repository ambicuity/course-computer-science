//! KDFs, PBKDF2, scrypt, Argon2
//! Phase 12 — Cryptography & Security
//!
//! Implements PBKDF2-HMAC-SHA256 and scrypt from scratch, verifies
//! against RFC 6070 and RFC 7914 test vectors, then benchmarks them.

use sha2::{Digest, Sha256};
use std::time::Instant;

// ---------------------------------------------------------------------------
// HMAC-SHA256
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// PBKDF2-HMAC-SHA256
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Salsa20/8 core (scrypt's mixing primitive)
// ---------------------------------------------------------------------------

fn salsa20_8(input: &[u8; 64]) -> [u8; 64] {
    let mut x = [0u32; 16];
    for i in 0..16 {
        x[i] = u32::from_le_bytes(input[4 * i..4 * i + 4].try_into().unwrap());
    }

    for _ in 0..4 {
        // Column round
        x[4] ^= (x[0].wrapping_add(x[12])).rotate_left(7);
        x[8] ^= (x[4].wrapping_add(x[0])).rotate_left(9);
        x[12] ^= (x[8].wrapping_add(x[4])).rotate_left(13);
        x[0] ^= (x[12].wrapping_add(x[8])).rotate_left(18);

        x[9] ^= (x[5].wrapping_add(x[1])).rotate_left(7);
        x[13] ^= (x[9].wrapping_add(x[5])).rotate_left(9);
        x[1] ^= (x[13].wrapping_add(x[9])).rotate_left(13);
        x[5] ^= (x[1].wrapping_add(x[13])).rotate_left(18);

        x[14] ^= (x[10].wrapping_add(x[6])).rotate_left(7);
        x[2] ^= (x[14].wrapping_add(x[10])).rotate_left(9);
        x[6] ^= (x[2].wrapping_add(x[14])).rotate_left(13);
        x[10] ^= (x[6].wrapping_add(x[2])).rotate_left(18);

        x[3] ^= (x[15].wrapping_add(x[11])).rotate_left(7);
        x[7] ^= (x[3].wrapping_add(x[15])).rotate_left(9);
        x[11] ^= (x[7].wrapping_add(x[3])).rotate_left(13);
        x[15] ^= (x[11].wrapping_add(x[7])).rotate_left(18);

        // Row round
        x[1] ^= (x[0].wrapping_add(x[3])).rotate_left(7);
        x[2] ^= (x[1].wrapping_add(x[0])).rotate_left(9);
        x[3] ^= (x[2].wrapping_add(x[1])).rotate_left(13);
        x[0] ^= (x[3].wrapping_add(x[2])).rotate_left(18);

        x[6] ^= (x[5].wrapping_add(x[4])).rotate_left(7);
        x[7] ^= (x[6].wrapping_add(x[5])).rotate_left(9);
        x[4] ^= (x[7].wrapping_add(x[6])).rotate_left(13);
        x[5] ^= (x[4].wrapping_add(x[7])).rotate_left(18);

        x[11] ^= (x[10].wrapping_add(x[9])).rotate_left(7);
        x[8] ^= (x[11].wrapping_add(x[10])).rotate_left(9);
        x[9] ^= (x[8].wrapping_add(x[11])).rotate_left(13);
        x[10] ^= (x[9].wrapping_add(x[8])).rotate_left(18);

        x[12] ^= (x[15].wrapping_add(x[14])).rotate_left(7);
        x[13] ^= (x[12].wrapping_add(x[15])).rotate_left(9);
        x[14] ^= (x[13].wrapping_add(x[12])).rotate_left(13);
        x[15] ^= (x[14].wrapping_add(x[13])).rotate_left(18);
    }

    let mut out = [0u8; 64];
    for i in 0..16 {
        let orig = u32::from_le_bytes(input[4 * i..4 * i + 4].try_into().unwrap());
        out[4 * i..4 * i + 4].copy_from_slice(&(x[i].wrapping_add(orig)).to_le_bytes());
    }
    out
}

fn blockmix_salsa8(b: &[u8], r: usize) -> Vec<u8> {
    let mut x = [0u8; 64];
    x.copy_from_slice(&b[(2 * r - 1) * 64..2 * r * 64]);

    let mut out = Vec::with_capacity(2 * r * 64);
    for i in 0..2 * r {
        for j in 0..64 {
            x[j] ^= b[i * 64 + j];
        }
        x = salsa20_8(&x);
        out.extend_from_slice(&x);
    }
    out
}

fn romix(b: &[u8], n: usize, r: usize) -> Vec<u8> {
    let v_len = n;
    let b_len = 2 * r * 64;
    let mut v: Vec<Vec<u8>> = Vec::with_capacity(v_len);

    let mut x = b.to_vec();
    for i in 0..v_len {
        v.push(x.clone());
        x = blockmix_salsa8(&x, r);
    }

    for _ in 0..n {
        let j = usize::try_from(u32::from_le_bytes(
            x[(2 * r - 1) * 64..2 * r * 64][..4].try_into().unwrap(),
        ))
        .unwrap()
            & (n - 1);
        for k in 0..b_len {
            x[k] ^= v[j][k];
        }
        x = blockmix_salsa8(&x, r);
    }
    x
}

// ---------------------------------------------------------------------------
// scrypt (RFC 7914)
// ---------------------------------------------------------------------------

fn scrypt(password: &[u8], salt: &[u8], n: u32, r: u32, p: u32, dk_len: usize) -> Vec<u8> {
    let b = pbkdf2(password, salt, 1, (p as usize) * 128 * (r as usize));

    let mut b_mixed = b.clone();
    for i in 0..p as usize {
        let start = i * 128 * (r as usize);
        let end = start + 128 * (r as usize);
        let mixed = romix(&b[start..end], n as usize, r as usize);
        b_mixed[start..end].copy_from_slice(&mixed);
    }

    pbkdf2(password, &b_mixed, 1, dk_len)
}

// ---------------------------------------------------------------------------
// Test vectors
// ---------------------------------------------------------------------------

fn test_pbkdf2_rfc6070() {
    println!("=== PBKDF2-HMAC-SHA256: RFC 6070 Test Vectors ===\n");

    // RFC 6070 doesn't have SHA-256 vectors directly (it covers SHA-1).
    // We use the IETF test vectors from draft-ietf-kitten-password-storage-01
    // (PBKDF2 with HMAC-SHA-256).

    struct TestVector {
        password: &'static [u8],
        salt: &'static [u8],
        iterations: u32,
        dk_len: usize,
        expected: &'static str,
    }

    let vectors = [
        TestVector {
            password: b"password",
            salt: b"salt",
            iterations: 1,
            dk_len: 32,
            expected: "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b",
        },
        TestVector {
            password: b"password",
            salt: b"salt",
            iterations: 2,
            dk_len: 32,
            expected: "ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6fc2e4a1ad9b957",
        },
        TestVector {
            password: b"passwordPASSWORDpassword",
            salt: b"saltSALTsaltSALTsaltSALTsaltSALTsalt",
            iterations: 4096,
            dk_len: 40,
            expected: "348c89dbcbd32b2f32d814b8116e84cf2b17347ebc1800181c4e2a1fb8dd53e1c635518c7dac47e9",
        },
    ];

    for (i, tv) in vectors.iter().enumerate() {
        let dk = pbkdf2(tv.password, tv.salt, tv.iterations, tv.dk_len);
        let got = hex::encode(&dk);
        let pass = got == tv.expected;
        println!("  Vector {} ({} iterations)", i + 1, tv.iterations);
        println!("    Expected: {}", tv.expected);
        println!("    Got:      {}", got);
        println!("    Result:   {}\n", if pass { "✅ PASS" } else { "❌ FAIL" });
        assert!(pass, "PBKDF2 test vector {} failed", i + 1);
    }
}

fn test_scrypt_rfc7914() {
    println!("=== scrypt: RFC 7914 Test Vectors ===\n");

    struct TestVector {
        password: &'static [u8],
        salt: &'static [u8],
        n: u32,
        r: u32,
        p: u32,
        dk_len: usize,
        expected: &'static str,
    }

    let vectors = [
        TestVector {
            password: b"",
            salt: b"",
            n: 16,
            r: 1,
            p: 1,
            dk_len: 64,
            expected: "77d6576238657b203b19ca42c18a0497f16b48444e3374cf8ce5d64c3fba2bb0384b9c3df286213b1c5c82c969adb057f0d19279dcb6bd1c9f26b4cce56c7d88",
        },
        TestVector {
            password: b"password",
            salt: b"NaCl",
            n: 1024,
            r: 8,
            p: 16,
            dk_len: 64,
            expected: "fdbabe1c9d3472007856e7190d01e9fe7c6ad7cbc8237830e77376634b3731622eaf30d92e22a3886ff109279d9830dac727afb94a83ee6d8360cbdfa2cc0640",
        },
    ];

    for (i, tv) in vectors.iter().enumerate() {
        let dk = scrypt(tv.password, tv.salt, tv.n, tv.r, tv.p, tv.dk_len);
        let got = hex::encode(&dk);
        let pass = got == tv.expected;
        println!("  Vector {} (n={}, r={}, p={})", i + 1, tv.n, tv.r, tv.p);
        println!("    Expected: {}", tv.expected);
        println!("    Got:      {}", got);
        println!("    Result:   {}\n", if pass { "✅ PASS" } else { "❌ FAIL" });
        assert!(pass, "scrypt test vector {} failed", i + 1);
    }
}

// ---------------------------------------------------------------------------
// Timing benchmarks
// ---------------------------------------------------------------------------

fn benchmark_pbkdf2() {
    println!("=== PBKDF2 Timing Benchmarks ===\n");

    let password = b"correct horse battery staple";
    let salt = b"random-salt-value";

    for iterations in [1000u32, 10000, 100000] {
        let start = Instant::now();
        let dk = pbkdf2(password, salt, iterations, 32);
        let elapsed = start.elapsed();
        println!("  {:<8} iterations → {:>8?}  (DK: {}...{})",
                 iterations, elapsed,
                 &hex::encode(&dk[..4]), &hex::encode(&dk[28..]));
    }
    println!();
}

fn benchmark_scrypt() {
    println!("=== scrypt Timing & Memory Benchmarks ===\n");

    let password = b"correct horse battery staple";
    let salt = b"random-salt-value";
    let r = 8u32;

    for n in [1024u32, 2048, 4096] {
        let mem_kb = 128.0 * (r as f64) * (n as f64) / 1024.0;
        let start = Instant::now();
        let dk = scrypt(password, salt, n, r, 1, 32);
        let elapsed = start.elapsed();
        println!("  n={:<5} (memory: ~{:<6.0} KB) → {:>8?}  (DK: {}...{})",
                 n, mem_kb, elapsed,
                 &hex::encode(&dk[..4]), &hex::encode(&dk[28..]));
    }
    println!();
}

// ---------------------------------------------------------------------------
// Password hashing demo
// ---------------------------------------------------------------------------

fn password_hashing_demo() {
    println!("=== Password Hashing Demo ===\n");

    // Different salts → different hashes for the same password
    let password = b"let-me-in";
    let salts = [b"salt-001", b"salt-002"];

    for salt in &salts {
        let hash = pbkdf2(password, salt, 100_000, 32);
        println!("  Password: {:?}  Salt: {:?}", std::str::from_utf8(password).unwrap(),
                 std::str::from_utf8(salt).unwrap());
        println!("    Hash: {}\n", hex::encode(&hash));
    }

    // Key stretching: derive a 256-bit key from a low-entropy password
    println!("  Key stretching: deriving 256-bit AES key from password");
    let weak_password = b"1234";
    let strong_key = pbkdf2(weak_password, b"unique-salt-per-user", 100_000, 32);
    println!("    Weak password: '{}'", std::str::from_utf8(weak_password).unwrap());
    println!("    Derived 256-bit key: {}\n", hex::encode(&strong_key));
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    println!();
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║   KDFs: PBKDF2 & scrypt From Scratch               ║");
    println!("║   Phase 12 — Cryptography & Security               ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();

    test_pbkdf2_rfc6070();
    test_scrypt_rfc7914();
    benchmark_pbkdf2();
    benchmark_scrypt();
    password_hashing_demo();

    println!("=== Summary ===");
    println!("  PBKDF2: iteration count controls cost (CPU-bound, no memory-hardness)");
    println!("  scrypt: adds memory-hardness via ROMix + Salsa20/8 mixing");
    println!("  Argon2id: see Python demo (state-of-the-art, memory-hard + side-channel resistant)");
    println!();
    println!("All tests passed!");
}
