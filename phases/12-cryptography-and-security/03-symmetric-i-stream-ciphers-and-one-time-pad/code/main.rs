use std::fmt;

const CHACHA20_CONSTANT: [u32; 4] = [
    0x61707865, // "expa"
    0x3320646e, // "nd 3"
    0x79622d32, // "2-by"
    0x6b206574, // "te k"
];

#[inline(always)]
fn rotl32(v: u32, n: u32) -> u32 {
    (v << n) | (v >> (32 - n))
}

fn quarter_round(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    state[a] = state[a].wrapping_add(state[b]);
    state[d] ^= state[a];
    state[d] = rotl32(state[d], 16);

    state[c] = state[c].wrapping_add(state[d]);
    state[b] ^= state[c];
    state[b] = rotl32(state[b], 12);

    state[a] = state[a].wrapping_add(state[b]);
    state[d] ^= state[a];
    state[d] = rotl32(state[d], 8);

    state[c] = state[c].wrapping_add(state[d]);
    state[b] ^= state[c];
    state[b] = rotl32(state[b], 7);
}

fn double_round(state: &mut [u32; 16]) {
    quarter_round(state, 0, 4, 8, 12);
    quarter_round(state, 1, 5, 9, 13);
    quarter_round(state, 2, 6, 10, 14);
    quarter_round(state, 3, 7, 11, 15);
    quarter_round(state, 0, 5, 10, 15);
    quarter_round(state, 1, 6, 11, 12);
    quarter_round(state, 2, 7, 8, 13);
    quarter_round(state, 3, 4, 9, 14);
}

struct ChaCha20 {
    key: [u32; 8],
    nonce: [u32; 3],
    counter: u32,
}

impl ChaCha20 {
    fn new(key: &[u8; 32], nonce: &[u8; 12], counter: u32) -> Self {
        let key_words = [
            u32::from_le_bytes(key[0..4].try_into().unwrap()),
            u32::from_le_bytes(key[4..8].try_into().unwrap()),
            u32::from_le_bytes(key[8..12].try_into().unwrap()),
            u32::from_le_bytes(key[12..16].try_into().unwrap()),
            u32::from_le_bytes(key[16..20].try_into().unwrap()),
            u32::from_le_bytes(key[20..24].try_into().unwrap()),
            u32::from_le_bytes(key[24..28].try_into().unwrap()),
            u32::from_le_bytes(key[28..32].try_into().unwrap()),
        ];
        let nonce_words = [
            u32::from_le_bytes(nonce[0..4].try_into().unwrap()),
            u32::from_le_bytes(nonce[4..8].try_into().unwrap()),
            u32::from_le_bytes(nonce[8..12].try_into().unwrap()),
        ];
        ChaCha20 {
            key: key_words,
            nonce: nonce_words,
            counter,
        }
    }

    fn block(&self, counter: u32) -> [u32; 16] {
        let mut state: [u32; 16] = [
            CHACHA20_CONSTANT[0],
            CHACHA20_CONSTANT[1],
            CHACHA20_CONSTANT[2],
            CHACHA20_CONSTANT[3],
            self.key[0],
            self.key[1],
            self.key[2],
            self.key[3],
            self.key[4],
            self.key[5],
            self.key[6],
            self.key[7],
            counter,
            self.nonce[0],
            self.nonce[1],
            self.nonce[2],
        ];
        let initial = state;
        for _ in 0..10 {
            double_round(&mut state);
        }
        for i in 0..16 {
            state[i] = state[i].wrapping_add(initial[i]);
        }
        state
    }

    fn keystream_block(&self, counter: u32) -> [u8; 64] {
        let state = self.block(counter);
        let mut block = [0u8; 64];
        for i in 0..16 {
            block[i * 4..i * 4 + 4].copy_from_slice(&state[i].to_le_bytes());
        }
        block
    }

    fn encrypt(&self, plaintext: &[u8]) -> Vec<u8> {
        let mut ciphertext = Vec::with_capacity(plaintext.len());
        let mut offset = 0;
        let mut counter = self.counter;
        while offset < plaintext.len() {
            let ks = self.keystream_block(counter);
            let end = std::cmp::min(offset + 64, plaintext.len());
            let chunk_len = end - offset;
            for i in 0..chunk_len {
                ciphertext.push(plaintext[offset + i] ^ ks[i]);
            }
            offset += chunk_len;
            counter += 1;
        }
        ciphertext
    }

    fn decrypt(&self, ciphertext: &[u8]) -> Vec<u8> {
        self.encrypt(ciphertext)
    }
}

impl fmt::Debug for ChaCha20 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ChaCha20 {{ counter: {} }}", self.counter)
    }
}

fn test_rfc7539_block() {
    let key: [u8; 32] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
        0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
        0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
    ];
    let nonce: [u8; 12] = [
        0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x4a, 0x00, 0x00, 0x00, 0x00,
    ];
    let c = ChaCha20::new(&key, &nonce, 1);

    let state = c.block(1);
    let expected: [u32; 16] = [
        0xe4e7f110, 0x15593bd1, 0x1fdd0f50, 0xc47120a3,
        0xc7f4d1c7, 0x0368c033, 0x9aaa2204, 0x4e6cd4c3,
        0x466482d2, 0x09aa9f07, 0x05d7c214, 0xa2028bd9,
        0xd19c12b5, 0xb94e16de, 0xe883d0cb, 0x4e3c50a2,
    ];

    for i in 0..16 {
        assert_eq!(state[i], expected[i], "State word {} mismatch", i);
    }
    println!("RFC 7539 Section 2.3.2 block test: PASS");
}

fn test_rfc7539_quarter_round() {
    let mut a: u32 = 0x11111111;
    let mut b: u32 = 0x01020304;
    let mut c: u32 = 0x9b8d6f43;
    let mut d: u32 = 0x01234567;

    a = a.wrapping_add(b); d ^= a; d = rotl32(d, 16);
    c = c.wrapping_add(d); b ^= c; b = rotl32(b, 12);
    a = a.wrapping_add(b); d ^= a; d = rotl32(d, 8);
    c = c.wrapping_add(d); b ^= c; b = rotl32(b, 7);

    assert_eq!(a, 0xea2a92f4, "QR a mismatch");
    assert_eq!(b, 0xcb1cf8ce, "QR b mismatch");
    assert_eq!(c, 0x4581472e, "QR c mismatch");
    assert_eq!(d, 0x5881c4bb, "QR d mismatch");
    println!("RFC 7539 Section 2.1.1 quarter round test: PASS");
}

fn test_rfc7539_encryption() {
    let key: [u8; 32] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
        0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
        0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
    ];
    let nonce: [u8; 12] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4a, 0x00, 0x00, 0x00, 0x00,
    ];
    let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
    let c = ChaCha20::new(&key, &nonce, 1);
    let ciphertext = c.encrypt(plaintext);
    let c2 = ChaCha20::new(&key, &nonce, 1);
    let decrypted = c2.decrypt(&ciphertext);
    assert_eq!(&decrypted[..], plaintext, "Decryption failed");
    println!("RFC 7539 Section 2.4.2 encryption/decryption test: PASS");
    println!("  Plaintext:  {}", String::from_utf8_lossy(plaintext));
    println!("  Ciphertext: {}...{}",
        hex::encode(&ciphertext[..8]),
        hex::encode(&ciphertext[ciphertext.len()-8..])
    );
    println!("  Decrypted:  {}", String::from_utf8_lossy(&decrypted));
}

mod hex {
    const CHARS: &[u8; 16] = b"0123456789abcdef";
    pub fn encode(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            s.push(CHARS[(b >> 4) as usize] as char);
            s.push(CHARS[(b & 0xf) as usize] as char);
        }
        s
    }
}

fn demo_quarter_round() {
    println!("\n=== CHACHA20 QUARTER ROUND DEMO ===\n");
    let a: u32 = 0x11111111;
    let b: u32 = 0x01020304;
    let c: u32 = 0x9b8d6f43;
    let d: u32 = 0x01234567;

    println!("Input:  a=0x{:08X}  b=0x{:08X}  c=0x{:08X}  d=0x{:08X}", a, b, c, d);

    let mut sa = a;
    let mut sb = b;
    let mut sc = c;
    let mut sd = d;

    sa = sa.wrapping_add(sb); sd ^= sa; sd = rotl32(sd, 16);
    println!("After a+=b; d^=a; d<<<16: a=0x{:08X}  b=0x{:08X}  c=0x{:08X}  d=0x{:08X}", sa, sb, sc, sd);

    sc = sc.wrapping_add(sd); sb ^= sc; sb = rotl32(sb, 12);
    println!("After c+=d; b^=c; b<<<12: a=0x{:08X}  b=0x{:08X}  c=0x{:08X}  d=0x{:08X}", sa, sb, sc, sd);

    sa = sa.wrapping_add(sb); sd ^= sa; sd = rotl32(sd, 8);
    println!("After a+=b; d^=a; d<<<8:  a=0x{:08X}  b=0x{:08X}  c=0x{:08X}  d=0x{:08X}", sa, sb, sc, sd);

    sc = sc.wrapping_add(sd); sb ^= sc; sb = rotl32(sb, 7);
    println!("After c+=d; b^=c; b<<<7:  a=0x{:08X}  b=0x{:08X}  c=0x{:08X}  d=0x{:08X}", sa, sb, sc, sd);
}

fn main() {
    println!("ChaCha20 — Stream Ciphers Implementation\n");

    demo_quarter_round();

    println!("\n=== RFC 7539 TEST VECTORS ===\n");
    test_rfc7539_quarter_round();
    test_rfc7539_block();
    test_rfc7539_encryption();

    println!("\nAll tests passed.");
}