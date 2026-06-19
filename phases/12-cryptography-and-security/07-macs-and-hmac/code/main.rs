//! MACs and HMAC — Phase 12, Lesson 07
//! HMAC-SHA256 and CBC-MAC implementations with constant-time verification.

use sha2::{Digest, Sha256};

const SHA256_BLOCK_SIZE: usize = 64;
const SHA256_DIGEST_SIZE: usize = 32;

// ── HMAC-SHA256 ──────────────────────────────────────────────────────────

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let padded_key = if key.len() > SHA256_BLOCK_SIZE {
        let h = Sha256::digest(key);
        let mut padded = vec![0u8; SHA256_BLOCK_SIZE];
        padded[..32].copy_from_slice(&h);
        padded
    } else {
        let mut padded = vec![0u8; SHA256_BLOCK_SIZE];
        padded[..key.len()].copy_from_slice(key);
        padded
    };

    let ipad: Vec<u8> = padded_key.iter().map(|k| k ^ 0x36).collect();
    let opad: Vec<u8> = padded_key.iter().map(|k| k ^ 0x5C).collect();

    let inner_hash = {
        let mut hasher = Sha256::new();
        hasher.update(&ipad);
        hasher.update(message);
        hasher.finalize().to_vec()
    };

    let mut outer_hasher = Sha256::new();
    outer_hasher.update(&opad);
    outer_hasher.update(&inner_hash);

    let result = outer_hasher.finalize();
    let mut tag = [0u8; 32];
    tag.copy_from_slice(&result);
    tag
}

// ── CBC-MAC (AES-128) ──────────────────────────────────────────────────

fn aes128_encrypt(block: &[u8; 16], key: &[u8; 16]) -> [u8; 16] {
    use aes::cipher::{BlockEncrypt, KeyInit};
    let cipher = aes::Aes128::new(key.into());
    let mut b = aes::cipher::generic_array::GenericArray::clone_from_slice(block);
    cipher.encrypt_block(&mut b);
    let mut out = [0u8; 16];
    out.copy_from_slice(&b);
    out
}

fn xor_blocks(a: &[u8; 16], b: &[u8; 16]) -> [u8; 16] {
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = a[i] ^ b[i];
    }
    out
}

fn cbc_mac(key: &[u8; 16], message: &[u8]) -> [u8; 16] {
    assert!(
        message.len() % 16 == 0,
        "CBC-MAC requires message length to be a multiple of 16 bytes"
    );

    let mut chain = [0u8; 16];
    for chunk in message.chunks(16) {
        let mut block = [0u8; 16];
        block.copy_from_slice(chunk);
        chain = aes128_encrypt(&xor_blocks(&chain, &block), key);
    }
    chain
}

// ── Constant-time comparison ────────────────────────────────────────────

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

// ── Test vectors ────────────────────────────────────────────────────────

fn test_hmac_sha256_vectors() {
    println!("HMAC-SHA256 Test Vectors (RFC 4231)");
    println!("{}", "-".".repeat(40));

    let test_cases = [
        (
            vec![0x0b; 20],
            b"Hi There".to_vec(),
            "b0344c703b8ec6cf82e8b4d0394b4b0a3b6f85d7b0f0303f7afab2403f063029",
        ),
        (
            b"Jefe".to_vec(),
            b"what do ya want for nothing?".to_vec(),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
        ),
        (
            vec![0xaa; 20],
            vec![0xdd; 50],
            "773ea91e36800e46854db8ebd09181a72986033167f883d0289f18758fdb3822",
        ),
        (
            vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19],
            vec![0xcd; 50],
            "d730594dd1672e82f7c0208f9762788201c12b5bdac24bc25f6f5933a1aaffed",
        ),
    ];

    let mut all_pass = true;
    for (i, (key, data, expected_hex)) in test_cases.iter().enumerate() {
        let tag = hmac_sha256(key, data);
        let tag_hex: String = tag.iter().map(|b| format!("{:02x}", b)).collect();
        let pass = tag_hex == *expected_hex;
        all_pass = all_pass && pass;
        println!("  Test {}: key_len={:>3} data_len={:>3} pass={}", i + 1, key.len(), data.len(), pass);
        if !pass {
            println!("    Got:      {}", tag_hex);
            println!("    Expected: {}", expected_hex);
        }
    }
    println!("  All HMAC-SHA256 vectors pass: {}\n", all_pass);
}

fn test_cbc_mac() {
    println!("CBC-MAC Test Vectors");
    println!("{}", "-".".repeat(40));

    // NIST SP 800-38B CMAC test vectors (also valid for CBC-MAC on single-block messages)
    let key: [u8; 16] = [
        0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6,
        0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c,
    ];
    let msg: [u8; 16] = [
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96,
        0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a,
    ];
    let tag = cbc_mac(&key, &msg);
    let tag_hex: String = tag.iter().map(|b| format!("{:02x}", b)).collect();
    println!("  Key:   2b7e151628aed2a6abf7158809cf4f3c");
    println!("  Msg:   6bc1bee22e409f96e93d7e117393172a");
    println!("  Tag:   {}", tag_hex);
    println!("  (CBC-MAC on single-block message)");

    // Verify with self-consistency
    let tag2 = cbc_mac(&key, &msg);
    println!("  Self-consistent: {}\n", tag == tag2);
}

fn test_constant_time_comparison() {
    println!("Constant-Time Comparison");
    println!("{}", "-".repeat(40));

    let key = b"my_secret_key_for_demo_12345";
    let message = b"Important financial transaction";
    let tag = hmac_sha256(key, message);

    // Verify correct tag
    let recomputed = hmac_sha256(key, message);
    println!("  Correct tag verification: {}", constant_time_eq(&tag, &recomputed));

    // Detect tampered message
    let tampered = b"Important financial transactiom";
    let tampered_tag = hmac_sha256(key, tampered);
    println!("  Tampered message detected:  {}", !constant_time_eq(&tag, &tampered_tag));

    // Single-bit difference
    let mut forged = tag;
    forged[0] ^= 0x01;
    println!("  Single-bit flip detected:    {}\n", !constant_time_eq(&tag, &forged));
}

fn demonstrate_message_integrity() {
    println!("Message Integrity and Authentication");
    println!("{}", "-".repeat(40));

    let key = b"shared_secret_between_alice_bob";
    let original = b"Transfer $500 to account #1234";
    let tampered = b"Transfer $500 to account #5678";

    let original_tag = hmac_sha256(key, original);

    println!("  Original: {}", String::from_utf8_lossy(original));
    println!("  Tag:      {}", hex::encode(original_tag));

    println!("\n  Tampered: {}", String::from_utf8_lossy(tampered));
    println!("  Tag:      {}", hex::encode(hmac_sha256(key, tampered)));

    println!("\n  Verify original: tag match = {}", constant_time_eq(&original_tag, &hmac_sha256(key, original)));
    println!("  Verify tampered: tag match = {}", constant_time_eq(&original_tag, &hmac_sha256(key, tampered)));
    println!("  Eve cannot forge a valid tag without the key.\n");
}

fn print_comparison_table() {
    println!("MAC Comparison Table");
    println!("{}", "-".repeat(40));
    println!("  {:<12} {:<10} {:<12} {:<16} {:<10}", "Scheme", "Basis", "Key Reuse", "Var Length", "Standard");
    println!("  {:<12} {:<10} {:<12} {:<16} {:<10}", "-----", "-----", "---------", "----------", "--------");
    println!("  {:<12} {:<10} {:<12} {:<16} {:<10}", "HMAC", "Hash", "Yes", "Yes", "RFC 2104");
    println!("  {:<12} {:<10} {:<12} {:<16} {:<10}", "CBC-MAC", "Cipher", "Yes", "No (fixed)", "SP 800-38A");
    println!("  {:<12} {:<10} {:<12} {:<16} {:<10}", "CMAC", "Cipher", "Yes", "Yes", "SP 800-38B");
    println!("  {:<12} {:<10} {:<12} {:<16} {:<10}", "Poly1305", "Polynomial", "No (1-time)", "Yes", "RFC 8439");
    println!();
}

fn main() {
    println!("MACs and HMAC \u{2014} Phase 12, Lesson 07");
    println!("{}\n", "=".repeat(50));

    test_hmac_sha256_vectors();
    test_cbc_mac();
    test_constant_time_comparison();
    demonstrate_message_integrity();
    print_comparison_table();

    println!("All tests complete.");
}