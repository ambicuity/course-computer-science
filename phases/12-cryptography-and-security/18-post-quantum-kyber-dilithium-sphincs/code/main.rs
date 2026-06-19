//! Post-Quantum Cryptography Demo — Kyber, Dilithium, SPHINCS+
//! Phase 12 — Cryptography & Security
//!
//! Demonstrates the three NIST-selected post-quantum cryptographic families:
//!   - Kyber (ML-KEM):   key encapsulation mechanism based on Module LWE
//!   - Dilithium (ML-DSA): digital signatures based on Module LWE
//!   - SPHINCS+ (SLH-DSA): stateless hash-based digital signatures

use std::time::Instant;

const SEP: &str = "────────────────────────────────────────────────────";

fn fmt_size(label: &str, bytes: usize) -> String {
    if bytes >= 1024 {
        format!("{}  {:>7}  ({:.1} KB)", label, bytes, bytes as f64 / 1024.0)
    } else {
        format!("{}  {:>7}  ({} B)", label, bytes, bytes)
    }
}

fn timing_us(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1_000_000.0
}

// ============================================================================
// Kyber (ML-KEM) — Key Encapsulation Mechanism
// ============================================================================

fn kyber_demo<KP, PK, SK, CT, SS>(
    label: &str,
    pk_expected: usize,
    sk_expected: usize,
    ct_expected: usize,
    ss_expected: usize,
    keypair: fn() -> (PK, SK),
    encapsulate: fn(&PK) -> (SS, CT),
    decapsulate: fn(&CT, &SK) -> SS,
) where
    PK: AsRef<[u8]>,
    SK: AsRef<[u8]>,
    CT: AsRef<[u8]>,
    SS: AsRef<[u8]> + PartialEq,
{
    println!("\n## {} (ML-KEM) — Key Encapsulation\n", label);
    println!("Security assumption: Module Learning With Errors (MLWE)");
    println!("{}", SEP);

    let t0 = Instant::now();
    let (pk, sk) = keypair();
    let keygen_us = timing_us(t0);

    assert_eq!(pk.as_ref().len(), pk_expected, "unexpected public key size");
    assert_eq!(sk.as_ref().len(), sk_expected, "unexpected secret key size");

    println!("{}", fmt_size("Public key:", pk.as_ref().len()));
    println!("{}", fmt_size("Secret key:", sk.as_ref().len()));
    println!("Public key prefix (hex): {}", hex::encode(&pk.as_ref()[..8]));
    println!("Key generation:  {:.0} µs", keygen_us);

    let t1 = Instant::now();
    let (ss_enc, ct) = encapsulate(&pk);
    let enc_us = timing_us(t1);

    assert_eq!(ct.as_ref().len(), ct_expected, "unexpected ciphertext size");
    assert_eq!(ss_enc.as_ref().len(), ss_expected, "unexpected shared secret size");

    println!("{}", fmt_size("Ciphertext:", ct.as_ref().len()));
    println!("{}", fmt_size("Shared secret:", ss_enc.as_ref().len()));
    println!("Ciphertext prefix (hex): {}", hex::encode(&ct.as_ref()[..8]));
    println!("Encapsulate:   {:.0} µs", enc_us);

    let t2 = Instant::now();
    let ss_dec = decapsulate(&ct, &sk);
    let dec_us = timing_us(t2);

    assert!(ss_enc == ss_dec, "shared secrets must match!");
    println!("Decapsulate:   {:.0} µs", dec_us);
    println!("✓ Shared secrets match ({} bytes)", ss_enc.as_ref().len());
    assert_ne!(ss_enc.as_ref(), &[0u8; 32], "shared secret must not be all-zero");
}

// ============================================================================
// Dilithium (ML-DSA) — Digital Signatures
// ============================================================================

fn dilithium_demo<PK, SK, SIG>(
    label: &str,
    pk_expected: usize,
    sk_expected: usize,
    sig_expected: usize,
    keypair: fn() -> (PK, SK),
    detached_sign: fn(&[u8], &SK) -> SIG,
    verify_detached_signature: fn(&SIG, &[u8], &PK) -> Result<(), ()>,
) where
    PK: AsRef<[u8]>,
    SK: AsRef<[u8]>,
    SIG: AsRef<[u8]>,
{
    println!("\n## {} (ML-DSA) — Digital Signatures\n", label);
    println!("Security assumption: Module Learning With Errors (MLWE)");
    println!("{}", SEP);

    let t0 = Instant::now();
    let (pk, sk) = keypair();
    let keygen_us = timing_us(t0);

    assert_eq!(pk.as_ref().len(), pk_expected, "unexpected public key size");
    assert_eq!(sk.as_ref().len(), sk_expected, "unexpected secret key size");

    println!("{}", fmt_size("Public key:", pk.as_ref().len()));
    println!("{}", fmt_size("Secret key:", sk.as_ref().len()));
    println!("Key generation:  {:.0} µs", keygen_us);

    let message = b"Post-quantum signatures are here — Dilithium leads the way.";
    println!("\nMessage:  \"{}\"", String::from_utf8_lossy(message));

    let t1 = Instant::now();
    let sig = detached_sign(message, &sk);
    let sign_us = timing_us(t1);

    assert_eq!(sig.as_ref().len(), sig_expected, "unexpected signature size");
    println!("{}", fmt_size("Signature:", sig.as_ref().len()));
    println!("Signature prefix (hex): {}", hex::encode(&sig.as_ref()[..8]));
    println!("Sign:          {:.0} µs", sign_us);

    let t2 = Instant::now();
    let verify_ok = verify_detached_signature(&sig, message, &pk).is_ok();
    let verify_us = timing_us(t2);

    println!("Verify:        {:.0} µs", verify_us);
    assert!(verify_ok, "signature verification must succeed!");
    println!("✓ Signature verified successfully");

    let tampered = b"Tampered message that should NOT verify.";
    let tamper_rejected = verify_detached_signature(&sig, tampered, &pk).is_err();
    assert!(tamper_rejected, "tampered message must be rejected!");
    println!("✓ Tampered message correctly rejected");
}

// ============================================================================
// SPHINCS+ (SLH-DSA) — Stateless Hash-Based Signatures
// ============================================================================

fn sphincs_demo<PK, SK, SIG>(
    label: &str,
    pk_expected: usize,
    sk_expected: usize,
    sig_expected: usize,
    hash_name: &str,
    keypair: fn() -> (PK, SK),
    detached_sign: fn(&[u8], &SK) -> SIG,
    verify_detached_signature: fn(&SIG, &[u8], &PK) -> Result<(), ()>,
) where
    PK: AsRef<[u8]>,
    SK: AsRef<[u8]>,
    SIG: AsRef<[u8]>,
{
    println!("\n## {} (SLH-DSA) — Stateless Hash-Based Signatures\n", label);
    println!("Security assumption: {} hash function security", hash_name);
    println!("No lattice assumptions — conservative security only.");
    println!("{}", SEP);

    let t0 = Instant::now();
    let (pk, sk) = keypair();
    let keygen_us = timing_us(t0);

    assert_eq!(pk.as_ref().len(), pk_expected, "unexpected public key size");
    assert_eq!(sk.as_ref().len(), sk_expected, "unexpected secret key size");

    println!("{}", fmt_size("Public key:", pk.as_ref().len()));
    println!("{}", fmt_size("Secret key:", sk.as_ref().len()));
    println!("Public key (hex): {}", hex::encode(pk.as_ref()));
    println!("Key generation:  {:.0} µs", keygen_us);

    let message = b"SPHINCS+ proves that hash functions alone are enough for signatures.";

    let t1 = Instant::now();
    let sig = detached_sign(message, &sk);
    let sign_us = timing_us(t1);

    assert_eq!(sig.as_ref().len(), sig_expected, "unexpected signature size");
    println!("{}", fmt_size("Signature:", sig.as_ref().len()));
    println!("Sign:          {:.0} µs ({:.2} ms)", sign_us, sign_us / 1000.0);

    let t2 = Instant::now();
    let verify_ok = verify_detached_signature(&sig, message, &pk).is_ok();
    let verify_us = timing_us(t2);

    println!("Verify:        {:.0} µs", verify_us);
    assert!(verify_ok, "signature verification must succeed!");
    println!("✓ Signature verified successfully");
    println!(
        "Note: Signing is {:.0}× slower than verification due to\n      many Merkle tree traversals.",
        sign_us / verify_us.max(1.0)
    );
}

// ============================================================================
// Size Comparison
// ============================================================================

fn print_comparison_table() {
    println!("\n\n## Size Comparison: PQC vs Classical");
    println!("{}", SEP);
    println!(
        "{:30} {:>10} {:>10} {:>18}",
        "Scheme", "Public key", "Secret", "Signature/Ciphertext"
    );
    println!("{}", SEP);
    println!("{:30} {:>6} B {:>6} B {:>6} B    (KEM)", "Kyber-512", 800, 1632, 768);
    println!("{:30} {:>6} B {:>6} B {:>6} B    (KEM)", "Kyber-1024", 1568, 3168, 1568);
    println!("{:30} {:>6} B {:>6} B {:>6} B    (Sig)", "Dilithium2", 1312, 2528, 2420);
    println!("{:30} {:>6} B {:>6} B {:>6} B    (Sig)", "Dilithium5", 2592, 4864, 4595);
    println!("{:30} {:>6} B {:>6} B {:>6} B    (Sig)", "SPHINCS+-128s (SHAKE)", 32, 64, 7856);
    println!("{:30} {:>6} B {:>6} B {:>6} B    (classical Sig)", "Ed25519", 32, 64, 64);
    println!("{:30} {:>6} B {:>6} B {:>6} B    (classical Sig)", "RSA-3072", 384, 512, 384);
    println!("{}", SEP);
    println!(
        "PQC signatures are {:>4}×–{:>4}× larger than Ed25519.",
        2420 / 64,
        7856 / 64
    );
    println!(
        "Kyber ciphertexts are {:>4}×–{:>4}× larger than X25519 (32 B).",
        768 / 32,
        1568 / 32
    );
    println!(
        "\nHybrid key exchange (X25519 + Kyber-768): {} B total.\n\
         Single X25519: 32 B handshake.",
        32 + 1088
    );
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    println!("══════════════════════════════════════════════════════");
    println!("  Post-Quantum Cryptography Demo");
    println!("  Kyber · Dilithium · SPHINCS+");
    println!("══════════════════════════════════════════════════════");

    // --- Kyber-512 ---
    kyber_demo(
        "Kyber-512",
        800, 1632, 768, 32,
        pqcrypto_kyber::kyber512::keypair,
        pqcrypto_kyber::kyber512::encapsulate,
        pqcrypto_kyber::kyber512::decapsulate,
    );

    // --- Kyber-1024 ---
    kyber_demo(
        "Kyber-1024",
        1568, 3168, 1568, 32,
        pqcrypto_kyber::kyber1024::keypair,
        pqcrypto_kyber::kyber1024::encapsulate,
        pqcrypto_kyber::kyber1024::decapsulate,
    );

    // --- Dilithium2 ---
    dilithium_demo(
        "Dilithium2",
        1312, 2528, 2420,
        pqcrypto_dilithium::dilithium2::keypair,
        pqcrypto_dilithium::dilithium2::detached_sign,
        |sig, msg, pk| {
            pqcrypto_dilithium::dilithium2::verify_detached_signature(sig, msg, pk)
                .map_err(|_| ())
        },
    );

    // --- Dilithium5 ---
    dilithium_demo(
        "Dilithium5",
        2592, 4864, 4595,
        pqcrypto_dilithium::dilithium5::keypair,
        pqcrypto_dilithium::dilithium5::detached_sign,
        |sig, msg, pk| {
            pqcrypto_dilithium::dilithium5::verify_detached_signature(sig, msg, pk)
                .map_err(|_| ())
        },
    );

    // --- SPHINCS+-SHAKE-128s ---
    sphincs_demo(
        "SPHINCS+-SHAKE-128s",
        32, 64, 7856,
        "SHAKE-256",
        pqcrypto_sphincsplus::sphincsshake128ssimple::keypair,
        pqcrypto_sphincsplus::sphincsshake128ssimple::detached_sign,
        |sig, msg, pk| {
            pqcrypto_sphincsplus::sphincsshake128ssimple::verify_detached_signature(sig, msg, pk)
                .map_err(|_| ())
        },
    );

    // --- SPHINCS+-SHA2-256s ---
    sphincs_demo(
        "SPHINCS+-SHA2-256s",
        64, 128, 29792,
        "SHA-256",
        pqcrypto_sphincsplus::sphincssha2256ssimple::keypair,
        pqcrypto_sphincsplus::sphincssha2256ssimple::detached_sign,
        |sig, msg, pk| {
            pqcrypto_sphincsplus::sphincssha2256ssimple::verify_detached_signature(sig, msg, pk)
                .map_err(|_| ())
        },
    );

    print_comparison_table();

    println!("\n{}", SEP);
    println!("All post-quantum operations verified successfully.");
}
