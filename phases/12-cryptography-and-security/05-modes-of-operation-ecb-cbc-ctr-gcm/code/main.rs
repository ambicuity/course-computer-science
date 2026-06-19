const BLOCK_SIZE: usize = 16;

type Block = [u8; 16];

fn xor_bytes(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b.iter()).map(|(&x, &y)| x ^ y).collect()
}

fn u128_to_bytes(val: u128) -> [u8; 16] {
    val.to_be_bytes()
}

fn bytes_to_u128(b: &[u8]) -> u128 {
    let mut arr = [0u8; 16];
    arr[..16].copy_from_slice(&b[..16]);
    u128::from_be_bytes(arr)
}

fn gf128_mul(x: u128, y: u128) -> u128 {
    let r: u128 = 0xe1000000000000000000000000000000;
    let mut z: u128 = 0;
    let mut v = y;
    for i in (0..128).rev() {
        if (x >> i) & 1 == 1 {
            z ^= v;
        }
        let carry = v & 1;
        v >>= 1;
        if carry != 0 {
            v ^= r;
        }
    }
    z
}

fn inc32(counter: &Block) -> Block {
    let mut result = *counter;
    let val = u32::from_be_bytes([result[12], result[13], result[14], result[15]]);
    let inc = val.wrapping_add(1);
    result[12] = ((inc >> 24) & 0xFF) as u8;
    result[13] = ((inc >> 16) & 0xFF) as u8;
    result[14] = ((inc >> 8) & 0xFF) as u8;
    result[15] = (inc & 0xFF) as u8;
    result
}

fn gh(h: u128, aad: &[u8], ciphertext: &[u8]) -> u128 {
    let mut y: u128 = 0;
    for chunk in aad.chunks(16) {
        let mut block = [0u8; 16];
        block[..chunk.len()].copy_from_slice(chunk);
        y = gf128_mul(y ^ bytes_to_u128(&block), h);
    }
    for chunk in ciphertext.chunks(16) {
        let mut block = [0u8; 16];
        block[..chunk.len()].copy_from_slice(chunk);
        y = gf128_mul(y ^ bytes_to_u128(&block), h);
    }
    let len_block = ((aad.len() as u128) * 8) << 64 | ((ciphertext.len() as u128) * 8);
    y = gf128_mul(y ^ len_block, h);
    y
}

const SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

const RCON: [u8; 10] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36];

fn sub_bytes(state: &mut [[u8; 4]; 4]) {
    for row in state.iter_mut() {
        for byte in row.iter_mut() {
            *byte = SBOX[*byte as usize];
        }
    }
}

fn shift_rows(state: &mut [[u8; 4]; 4]) {
    let tmp = state[1][0];
    state[1][0] = state[1][1]; state[1][1] = state[1][2]; state[1][2] = state[1][3]; state[1][3] = tmp;
    let tmp0 = state[2][0]; let tmp1 = state[2][1];
    state[2][0] = state[2][2]; state[2][1] = state[2][3]; state[2][2] = tmp0; state[2][3] = tmp1;
    let tmp = state[3][3];
    state[3][3] = state[3][2]; state[3][2] = state[3][1]; state[3][1] = state[3][0]; state[3][0] = tmp;
}

fn xtime(a: u8) -> u8 {
    if a & 0x80 != 0 { (a << 1) ^ 0x1b } else { a << 1 }
}

fn mix_columns(state: &mut [[u8; 4]; 4]) {
    for c in 0..4 {
        let s0 = state[0][c]; let s1 = state[1][c]; let s2 = state[2][c]; let s3 = state[3][c];
        state[0][c] = xtime(s0) ^ xtime(s1) ^ s1 ^ s2 ^ s3;
        state[1][c] = s0 ^ xtime(s1) ^ xtime(s2) ^ s2 ^ s3;
        state[2][c] = s0 ^ s1 ^ xtime(s2) ^ xtime(s3) ^ s3;
        state[3][c] = xtime(s0) ^ s0 ^ s1 ^ s2 ^ xtime(s3);
    }
}

fn add_round_key(state: &mut [[u8; 4]; 4], round_key: &[u8; 16]) {
    for r in 0..4 {
        for c in 0..4 {
            state[r][c] ^= round_key[r + c * 4];
        }
    }
}

fn key_expansion(key: &[u8; 16]) -> [[u8; 16]; 11] {
    let mut round_keys = [[0u8; 16]; 11];
    round_keys[0].copy_from_slice(key);
    for round in 0..10 {
        let prev = &round_keys[round];
        let mut w = [0u8; 16];
        w[0] = SBOX[prev[13] as usize] ^ RCON[round] ^ prev[0];
        w[1] = SBOX[prev[14] as usize] ^ prev[1];
        w[2] = SBOX[prev[15] as usize] ^ prev[2];
        w[3] = SBOX[prev[12] as usize] ^ prev[3];
        for i in 4..16 {
            w[i] = prev[i] ^ w[i - 4];
        }
        round_keys[round + 1] = w;
    }
    round_keys
}

fn aes128_encrypt_block(key: [u8; 16], block: [u8; 16]) -> [u8; 16] {
    let round_keys = key_expansion(&key);
    let mut state = [[0u8; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            state[r][c] = block[r + c * 4];
        }
    }
    add_round_key(&mut state, &round_keys[0]);
    for round in 1..10 {
        sub_bytes(&mut state);
        shift_rows(&mut state);
        mix_columns(&mut state);
        add_round_key(&mut state, &round_keys[round]);
    }
    sub_bytes(&mut state);
    shift_rows(&mut state);
    add_round_key(&mut state, &round_keys[10]);
    let mut output = [0u8; 16];
    for r in 0..4 {
        for c in 0..4 {
            output[r + c * 4] = state[r][c];
        }
    }
    output
}

fn gcm_ctr_counter(nonce: &[u8]) -> Block {
    let mut counter = [0u8; 16];
    counter[..12].copy_from_slice(&nonce[..12]);
    counter[15] = 1;
    counter
}

fn gcm_encrypt(key: &[u8; 16], nonce: &[u8; 12], plaintext: &[u8], aad: &[u8]) -> (Vec<u8>, [u8; 16]) {
    let h = aes128_encrypt_block(*key, [0u8; 16]);
    let h_val = bytes_to_u128(&h);
    let j0 = gcm_ctr_counter(nonce);
    let e_j0 = aes128_encrypt_block(*key, j0);
    let mut ciphertext = Vec::with_capacity(plaintext.len());
    let mut counter = j0;
    counter[15] += 1;
    for chunk in plaintext.chunks(16) {
        let keystream = aes128_encrypt_block(*key, counter);
        let ct_chunk: Vec<u8> = chunk.iter().zip(keystream.iter()).map(|(&p, &k)| p ^ k).collect();
        ciphertext.extend_from_slice(&ct_chunk);
        counter = inc32(&counter);
    }
    let s = gh(h_val, aad, &ciphertext);
    let tag_val = s ^ bytes_to_u128(&e_j0);
    (ciphertext, u128_to_bytes(tag_val))
}

fn gcm_decrypt(key: &[u8; 16], nonce: &[u8; 12], ciphertext: &[u8], tag: &[u8; 16], aad: &[u8]) -> Option<Vec<u8>> {
    let h = aes128_encrypt_block(*key, [0u8; 16]);
    let h_val = bytes_to_u128(&h);
    let j0 = gcm_ctr_counter(nonce);
    let e_j0 = aes128_encrypt_block(*key, j0);
    let s = gh(h_val, aad, ciphertext);
    let computed_tag = u128_to_bytes(s ^ bytes_to_u128(&e_j0));
    let mut tag_match = true;
    for i in 0..16 {
        tag_match &= computed_tag[i] == tag[i];
    }
    if !tag_match {
        return None;
    }
    let mut plaintext = Vec::with_capacity(ciphertext.len());
    let mut counter = j0;
    counter[15] += 1;
    for chunk in ciphertext.chunks(16) {
        let keystream = aes128_encrypt_block(*key, counter);
        let pt_chunk: Vec<u8> = chunk.iter().zip(keystream.iter()).map(|(&c, &k)| c ^ k).collect();
        plaintext.extend_from_slice(&pt_chunk);
        counter = inc32(&counter);
    }
    Some(plaintext)
}

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn bytes_to_hex(b: &[u8]) -> String {
    b.iter().map(|byte| format!("{:02x}", byte)).collect()
}

fn test_aes128() {
    println!("AES-128 test (NIST FIPS 197 Appendix B):");
    let key: [u8; 16] = [
        0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6,
        0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c,
    ];
    let plaintext: [u8; 16] = [
        0x32, 0x43, 0xf6, 0xa8, 0x88, 0x5a, 0x30, 0x8d,
        0x31, 0x31, 0x98, 0xa2, 0xe0, 0x37, 0x07, 0x34,
    ];
    let expected: [u8; 16] = [
        0x39, 0x25, 0x84, 0x1d, 0x02, 0xdc, 0x09, 0xfb,
        0xdc, 0x11, 0x85, 0x97, 0x19, 0x6a, 0x0b, 0x32,
    ];
    let result = aes128_encrypt_block(key, plaintext);
    println!("  Input:    {}", bytes_to_hex(&plaintext));
    println!("  Expected: {}", bytes_to_hex(&expected));
    println!("  Got:      {}", bytes_to_hex(&result));
    assert_eq!(result, expected, "AES-128 encryption failed!");
    println!("  AES-128 encryption: OK\n");
}

fn test_gcm_nist() {
    println!("AES-128-GCM NIST test vector (Test Case 1 - empty):");
    let key: [u8; 16] = [0x00; 16];
    let nonce: [u8; 12] = [0x00; 12];
    let plaintext: [u8; 0] = [];
    let aad: [u8; 0] = [];
    let (ciphertext, tag) = gcm_encrypt(&key, &nonce, &plaintext, &aad);
    let expected_tag = hex_to_bytes("58e2fccefa7e3061367f1d57a4e445a1");
    println!("  Ciphertext: {}", bytes_to_hex(&ciphertext));
    println!("  Tag:        {}", bytes_to_hex(&tag));
    println!("  Expected:   {}", bytes_to_hex(&expected_tag));
    assert_eq!(ciphertext.len(), 0);
    assert_eq!(&tag[..], &expected_tag[..]);
    println!("  Test Case 1 (empty): OK\n");

    println!("AES-128-GCM NIST test vector (Test Case 2 - 16 bytes):");
    let key: [u8; 16] = [0x00; 16];
    let nonce: [u8; 12] = [0x00; 12];
    let plaintext = [0u8; 16];
    let aad: [u8; 0] = [];
    let (ciphertext, tag) = gcm_encrypt(&key, &nonce, &plaintext, &aad);
    let expected_ct = hex_to_bytes("0388dace60b6a392f328c2b971b2fe78");
    let expected_tag = hex_to_bytes("ab6e47d42cec13bdf53c67e2d8fe5a1e");
    println!("  Ciphertext: {}", bytes_to_hex(&ciphertext));
    println!("  Expected:   {}", bytes_to_hex(&expected_ct));
    println!("  Tag:        {}", bytes_to_hex(&tag));
    println!("  Expected:   {}", bytes_to_hex(&expected_tag));
    assert_eq!(ciphertext, expected_ct);
    assert_eq!(&tag[..], &expected_tag[..]);
    println!("  Test Case 2 (16-byte plaintext): OK\n");
}

fn test_gcm_roundtrip() {
    println!("GCM encrypt/decrypt roundtrip:");
    let key: [u8; 16] = [
        0xfe, 0xff, 0xe9, 0x92, 0x86, 0x65, 0x73, 0x1c,
        0x6d, 0x56, 0xf6, 0x44, 0xda, 0x2c, 0x11, 0x0c,
    ];
    let nonce: [u8; 12] = [
        0xca, 0xfe, 0xba, 0xbe, 0xfa, 0xce, 0xdb, 0xad,
        0xde, 0xca, 0xf8, 0x88,
    ];
    let plaintext = b"Hello, GCM mode!";
    let aad = b"additional data";
    let (ciphertext, tag) = gcm_encrypt(&key, &nonce, plaintext, aad);
    println!("  Plaintext:  {}", String::from_utf8_lossy(plaintext));
    println!("  Ciphertext: {}", bytes_to_hex(&ciphertext));
    println!("  Tag:        {}", bytes_to_hex(&tag));
    let decrypted = gcm_decrypt(&key, &nonce, &ciphertext, &tag, aad).unwrap();
    println!("  Decrypted:  {}", String::from_utf8_lossy(&decrypted[..plaintext.len()]));
    assert_eq!(&decrypted[..plaintext.len()], plaintext);
    println!("  Roundtrip: OK");

    println!("\nGCM tag verification (tampered ciphertext):");
    let mut tampered = ciphertext.clone();
    tampered[0] ^= 0xFF;
    let result = gcm_decrypt(&key, &nonce, &tampered, &tag, aad);
    match result {
        None => println!("  Tampered ciphertext: REJECTED (tag mismatch)"),
        Some(_) => {
            eprintln!("  Tampered ciphertext: ACCEPTED (BUG!)");
            std::process::exit(1);
        }
    }
    println!("  Tag verification: OK\n");
}

fn test_gcm_with_aad() {
    println!("GCM with AAD (associated data not encrypted):");
    let key: [u8; 16] = [0x42; 16];
    let nonce: [u8; 12] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
    let plaintext = b"Secret payload";
    let aad = b"Public header";
    let (ciphertext, tag) = gcm_encrypt(&key, &nonce, plaintext, aad);
    println!("  AAD:        {}", String::from_utf8_lossy(aad));
    println!("  Plaintext:  {}", String::from_utf8_lossy(plaintext));
    println!("  Ciphertext: {}", bytes_to_hex(&ciphertext));
    println!("  Tag:        {}", bytes_to_hex(&tag));
    let decrypted = gcm_decrypt(&key, &nonce, &ciphertext, &tag, aad).unwrap();
    assert_eq!(&decrypted[..plaintext.len()], plaintext);
    println!("  Roundtrip with AAD: OK");

    println!("\nGCM with wrong AAD:");
    let result = gcm_decrypt(&key, &nonce, &ciphertext, &tag, b"Wrong header");
    match result {
        None => println!("  Wrong AAD: REJECTED (tag mismatch)"),
        Some(_) => {
            eprintln!("  Wrong AAD: ACCEPTED (BUG!)");
            std::process::exit(1);
        }
    }
    println!("  AAD verification: OK\n");
}

fn main() {
    println!("AES-128-GCM Implementation");
    println!("{}\n", "=".repeat(50));
    test_aes128();
    test_gcm_nist();
    test_gcm_roundtrip();
    test_gcm_with_aad();
    println!("All tests passed!");
}