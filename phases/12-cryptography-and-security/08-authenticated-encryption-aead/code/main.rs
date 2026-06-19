use std::fmt;

#[derive(Debug)]
struct AeadError(String);

impl fmt::Display for AeadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AEAD error: {}", self.0)
    }
}

trait Aead {
    fn encrypt(key: &[u8], nonce: &[u8], aad: &[u8], plaintext: &[u8]) -> (Vec<u8>, Vec<u8>);
    fn decrypt(key: &[u8], nonce: &[u8], aad: &[u8], ciphertext: &[u8], tag: &[u8]) -> Result<Vec<u8>, AeadError>;
}

fn xor_bytes(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b.iter()).map(|(x, y)| x ^ y).collect()
}

fn pad16(data: &[u8]) -> Vec<u8> {
    let rem = data.len() % 16;
    if rem == 0 { Vec::new() } else { vec![0u8; 16 - rem] }
}

fn le64(val: u64) -> [u8; 8] {
    val.to_le_bytes()
}

struct CtrAesHmac;

impl CtrAesHmac {
    const SBOX: [u8; 256] = [
        0x63,0x7c,0x77,0x7b,0xf2,0x6b,0x6f,0xc5,0x30,0x01,0x67,0x2b,0xfe,0xd7,0xab,0x76,
        0xca,0x82,0xc9,0x7d,0xfa,0x59,0x47,0xf0,0xad,0xd4,0xa2,0xaf,0x9c,0xa4,0x72,0xc0,
        0xb7,0xfd,0x93,0x26,0x36,0x3f,0xf7,0xcc,0x34,0xa5,0xe5,0xf1,0x71,0xd8,0x31,0x15,
        0x04,0xc7,0x23,0xc3,0x18,0x96,0x05,0x9a,0x07,0x12,0x80,0xe2,0xeb,0x27,0xb2,0x75,
        0x09,0x83,0x2c,0x1a,0x1b,0x6e,0x5a,0xa0,0x52,0x3b,0xd6,0xb3,0x29,0xe3,0x2f,0x84,
        0x53,0xd1,0x00,0xed,0x20,0xfc,0xb1,0x5b,0x6a,0xcb,0xbe,0x39,0x4a,0x4c,0x58,0xcf,
        0xd0,0xef,0xaa,0xfb,0x43,0x4d,0x33,0x85,0x45,0xf9,0x02,0x7f,0x50,0x3c,0x9f,0xa8,
        0x51,0xa3,0x40,0x8f,0x92,0x9d,0x38,0xf5,0xbc,0xb6,0xda,0x21,0x10,0xff,0xf3,0xd2,
        0xcd,0x0c,0x13,0xec,0x5f,0x97,0x44,0x17,0xc4,0xa7,0x7e,0x3d,0x64,0x5d,0x19,0x73,
        0x60,0x81,0x4f,0xdc,0x22,0x2a,0x90,0x88,0x46,0xee,0xb8,0x14,0xde,0x5e,0x0b,0xdb,
        0xe0,0x32,0x3a,0x0a,0x49,0x06,0x24,0x5c,0xc2,0xd3,0xac,0x62,0x91,0x95,0xe4,0x79,
        0xe7,0xc8,0x37,0x6d,0x8d,0xd5,0x4e,0xa9,0x6c,0x56,0xf4,0xea,0x65,0x7a,0xae,0x08,
        0xba,0x78,0x25,0x2e,0x1c,0xa6,0xb4,0xc6,0xe8,0xdd,0x74,0x1f,0x4b,0xbd,0x8b,0x8a,
        0x70,0x3e,0xb5,0x66,0x48,0x03,0xf6,0x0e,0x61,0x35,0x57,0xb9,0x86,0xc1,0x1d,0x9e,
        0xe1,0xf8,0x98,0x11,0x69,0xd9,0x8e,0x94,0x9b,0x1e,0x87,0xe9,0xce,0x55,0x28,0xdf,
        0x8c,0xa1,0x89,0x0d,0xbf,0xe6,0x42,0x68,0x41,0x99,0x2d,0x0f,0xb0,0x54,0xbb,0x16,
    ];

    const RCON: [u8; 10] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36];

    fn sub_bytes(state: u128) -> u128 {
        let b = state.to_be_bytes();
        let mut out = [0u8; 16];
        for i in 0..16 { out[i] = Self::SBOX[b[i] as usize]; }
        u128::from_be_bytes(out)
    }

    fn shift_rows(s: u128) -> u128 {
        let b = s.to_be_bytes();
        let mut o = [0u8; 16];
        o[0]=b[0]; o[1]=b[5]; o[2]=b[10]; o[3]=b[15];
        o[4]=b[4]; o[5]=b[9]; o[6]=b[14]; o[7]=b[3];
        o[8]=b[8]; o[9]=b[13]; o[10]=b[2]; o[11]=b[7];
        o[12]=b[12]; o[13]=b[1]; o[14]=b[6]; o[15]=b[11];
        u128::from_be_bytes(o)
    }

    fn mix_columns(s: u128) -> u128 {
        let b = s.to_be_bytes();
        let mut o = [0u8; 16];
        fn mul2(x: u8) -> u8 { if x & 0x80 != 0 { (x << 1) ^ 0x1b } else { x << 1 } }
        fn mul3(x: u8) -> u8 { mul2(x) ^ x }
        for c in 0..4 {
            let i = c * 4;
            let s0 = b[i]; let s1 = b[i+1]; let s2 = b[i+2]; let s3 = b[i+3];
            o[i]   = mul2(s0) ^ mul3(s1) ^ s2    ^ s3;
            o[i+1] = s0    ^ mul2(s1) ^ mul3(s2) ^ s3;
            o[i+2] = s0    ^ s1    ^ mul2(s2) ^ mul3(s3);
            o[i+3] = mul3(s0) ^ s1    ^ s2    ^ mul2(s3);
        }
        u128::from_be_bytes(o)
    }

    fn aes_key_schedule(key: u128) -> [u128; 11] {
        let mut keys = [0u128; 11];
        keys[0] = key;
        for i in 0..10 {
            let prev = keys[i].to_be_bytes();
            let w4 = u32::from_be_bytes([prev[0], prev[1], prev[2], prev[3]]);
            let w5 = u32::from_be_bytes([prev[4], prev[5], prev[6], prev[7]]);
            let w6 = u32::from_be_bytes([prev[8], prev[9], prev[10], prev[11]]);
            let w7 = u32::from_be_bytes([prev[12], prev[13], prev[14], prev[15]]);
            let rc = Self::RCON[i] as u32;
            let sub_word = u32::from_be_bytes([
                Self::SBOX[prev[13] as usize],
                Self::SBOX[prev[14] as usize],
                Self::SBOX[prev[15] as usize],
                Self::SBOX[prev[12] as usize],
            ]);
            let t = sub_word ^ (rc << 24);
            let n0 = w4 ^ t;
            let n1 = w5 ^ n0;
            let n2 = w6 ^ n1;
            let n3 = w7 ^ n2;
            keys[i+1] = u128::from_be_bytes([
                (n0>>24) as u8, (n0>>16) as u8, (n0>>8) as u8, n0 as u8,
                (n1>>24) as u8, (n1>>16) as u8, (n1>>8) as u8, n1 as u8,
                (n2>>24) as u8, (n2>>16) as u8, (n2>>8) as u8, n2 as u8,
                (n3>>24) as u8, (n3>>16) as u8, (n3>>8) as u8, n3 as u8,
            ]);
        }
        keys
    }

    fn aes_encrypt_block(key: &[u8; 16], block: &[u8; 16]) -> [u8; 16] {
        let subkeys = Self::aes_key_schedule(u128::from_be_bytes(*key));
        let mut s = u128::from_be_bytes(*block) ^ subkeys[0];
        for i in 1..10 {
            s = Self::sub_bytes(s);
            s = Self::shift_rows(s);
            s = Self::mix_columns(s);
            s ^= subkeys[i];
        }
        s = Self::sub_bytes(s);
        s = Self::shift_rows(s);
        s ^= subkeys[10];
        s.to_be_bytes()
    }

    fn sha256(data: &[u8]) -> [u8; 32] {
        let iv = [
            0x6a09e667u32, 0xbb67ae85u32, 0x3c6ef372u32, 0xa54ff53au32,
            0x510e527fu32, 0x9b05688cu32, 0x1f83d9abu32, 0x5be0cd19u32,
        ];
        let k: [u32; 64] = [
            0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
            0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
            0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
            0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
            0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c52e,0x92722c85,
            0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
            0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
            0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
        ];
        let msg_len = data.len();
        let mut padded = data.to_vec();
        padded.push(0x80);
        while padded.len() % 64 != 56 { padded.push(0); }
        padded.extend_from_slice(&((msg_len as u64) * 8).to_be_bytes());
        let mut h = iv;
        for chunk in padded.chunks(64) {
            let mut w = [0u32; 64];
            for i in 0..16 {
                w[i] = u32::from_be_bytes([chunk[i*4], chunk[i*4+1], chunk[i*4+2], chunk[i*4+3]]);
            }
            for i in 16..64 {
                let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
                let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
                w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
            }
            let mut a = h;
            for i in 0..64 {
                let s1 = a[0].rotate_right(2) ^ a[0].rotate_right(13) ^ a[0].rotate_right(22);
                let s2 = (a[0] & a[1]) ^ (a[0] & a[2]) ^ (a[1] & a[2]);
                let t2 = s1.wrapping_add(s2);
                let s3 = a[4].rotate_right(6) ^ a[4].rotate_right(11) ^ a[4].rotate_right(25);
                let s4 = (a[4] & a[5]) ^ ((!a[4]) & a[6]);
                let t1 = a[7].wrapping_add(s3).wrapping_add(s4).wrapping_add(k[i]).wrapping_add(w[i]);
                a[7] = a[6]; a[6] = a[5]; a[5] = a[4]; a[4] = a[3].wrapping_add(t1);
                a[3] = a[2]; a[2] = a[1]; a[1] = a[0]; a[0] = t1.wrapping_add(t2);
            }
            for i in 0..8 { h[i] = h[i].wrapping_add(a[i]); }
        }
        let mut result = [0u8; 32];
        for i in 0..8 { result[i*4..i*4+4].copy_from_slice(&h[i].to_be_bytes()); }
        result
    }

    fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
        let mut k_padded = [0u8; 64];
        let k = if key.len() > 64 { &Self::sha256(key)[..] } else { key };
        k_padded[..k.len()].copy_from_slice(k);
        let mut ipad = [0u8; 64];
        let mut opad = [0u8; 64];
        for i in 0..64 { ipad[i] = k_padded[i] ^ 0x36; opad[i] = k_padded[i] ^ 0x5c; }
        let mut inner = ipad.to_vec();
        inner.extend_from_slice(data);
        let inner_hash = Self::sha256(&inner);
        let mut outer = opad.to_vec();
        outer.extend_from_slice(&inner_hash);
        Self::sha256(&outer)
    }

    fn ctr_aes_keystream(key: &[u8; 16], nonce: &[u8], counter_start: u32, length: usize) -> Vec<u8> {
        let mut keystream = Vec::new();
        let mut counter = counter_start;
        while keystream.len() < length {
            let mut block = [0u8; 16];
            block[0..4].copy_from_slice(&nonce[0..4]);
            block[4..8].copy_from_slice(&nonce[4..8]);
            block[8..12].copy_from_slice(&nonce[8..12]);
            block[12..16].copy_from_slice(&counter.to_be_bytes());
            let encrypted = Self::aes_encrypt_block(key, &block);
            keystream.extend_from_slice(&encrypted);
            counter = counter.wrapping_add(1);
        }
        keystream.truncate(length);
        keystream
    }
}

impl Aead for CtrAesHmac {
    fn encrypt(key: &[u8], nonce: &[u8], aad: &[u8], plaintext: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let enc_key: [u8; 16] = key[..16].try_into().expect("enc key 16 bytes");
        let mac_key = &key[16..32];
        let keystream = Self::ctr_aes_keystream(&enc_key, nonce, 0, plaintext.len());
        let ciphertext = xor_bytes(plaintext, &keystream);
        let mut mac_data = Vec::new();
        mac_data.extend_from_slice(aad);
        mac_data.extend_from_slice(&pad16(aad));
        mac_data.extend_from_slice(&ciphertext);
        mac_data.extend_from_slice(&pad16(&ciphertext));
        mac_data.extend_from_slice(&le64(aad.len() as u64));
        mac_data.extend_from_slice(&le64(ciphertext.len() as u64));
        let full_tag = Self::hmac_sha256(mac_key, &mac_data);
        (ciphertext, full_tag[..16].to_vec())
    }

    fn decrypt(key: &[u8], nonce: &[u8], aad: &[u8], ciphertext: &[u8], tag: &[u8]) -> Result<Vec<u8>, AeadError> {
        let enc_key: [u8; 16] = key[..16].try_into().expect("enc key 16 bytes");
        let mac_key = &key[16..32];
        let mut mac_data = Vec::new();
        mac_data.extend_from_slice(aad);
        mac_data.extend_from_slice(&pad16(aad));
        mac_data.extend_from_slice(ciphertext);
        mac_data.extend_from_slice(&pad16(ciphertext));
        mac_data.extend_from_slice(&le64(aad.len() as u64));
        mac_data.extend_from_slice(&le64(ciphertext.len() as u64));
        let expected_tag = Self::hmac_sha256(mac_key, &mac_data);
        let mut diff = 0u8;
        for (a, b) in expected_tag[..16].iter().zip(tag.iter()) { diff |= a ^ b; }
        if diff != 0 { return Err(AeadError("authentication tag verification failed".into())); }
        let keystream = Self::ctr_aes_keystream(&enc_key, nonce, 0, ciphertext.len());
        Ok(xor_bytes(ciphertext, &keystream))
    }
}

struct ChaCha20Poly1305;

impl ChaCha20Poly1305 {
    fn quarter_round(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
        s[a] = s[a].wrapping_add(s[b]); s[d] ^= s[a]; s[d] = s[d].rotate_left(16);
        s[c] = s[c].wrapping_add(s[d]); s[b] ^= s[c]; s[b] = s[b].rotate_left(12);
        s[a] = s[a].wrapping_add(s[b]); s[d] ^= s[a]; s[d] = s[d].rotate_left(8);
        s[c] = s[c].wrapping_add(s[d]); s[b] ^= s[c]; s[b] = s[b].rotate_left(7);
    }

    fn chacha20_block(key: &[u8; 32], counter: u32, nonce: &[u8; 12]) -> [u8; 64] {
        const CS: [u32; 4] = [0x61707865, 0x3320646e, 0x79622d32, 0x6b206574];
        let mut state = [0u32; 16];
        state[0] = CS[0]; state[1] = CS[1]; state[2] = CS[2]; state[3] = CS[3];
        for i in 0..8 { state[4+i] = u32::from_le_bytes([key[i*4], key[i*4+1], key[i*4+2], key[i*4+3]]); }
        state[12] = counter;
        for i in 0..3 { state[13+i] = u32::from_le_bytes([nonce[i*4], nonce[i*4+1], nonce[i*4+2], nonce[i*4+3]]); }
        let mut w = state;
        for _ in 0..10 {
            Self::quarter_round(&mut w, 0, 4, 8, 12);
            Self::quarter_round(&mut w, 1, 5, 9, 13);
            Self::quarter_round(&mut w, 2, 6, 10, 14);
            Self::quarter_round(&mut w, 3, 7, 11, 15);
            Self::quarter_round(&mut w, 0, 5, 10, 15);
            Self::quarter_round(&mut w, 1, 6, 11, 12);
            Self::quarter_round(&mut w, 2, 7, 8, 13);
            Self::quarter_round(&mut w, 3, 4, 9, 14);
        }
        let mut out = [0u8; 64];
        for i in 0..16 {
            let v = w[i].wrapping_add(state[i]);
            out[i*4..i*4+4].copy_from_slice(&v.to_le_bytes());
        }
        out
    }

    fn chacha20_encrypt(key: &[u8; 32], counter: u32, nonce: &[u8; 12], plaintext: &[u8]) -> Vec<u8> {
        let mut ct = Vec::with_capacity(plaintext.len());
        let mut ctr = counter;
        let mut off = 0;
        while off < plaintext.len() {
            let block = Self::chacha20_block(key, ctr, nonce);
            let take = (plaintext.len() - off).min(64);
            for i in 0..take { ct.push(plaintext[off+i] ^ block[i]); }
            ctr = ctr.wrapping_add(1);
            off += take;
        }
        ct
    }

    fn poly1305_clamp(r: &mut [u8; 16]) {
        r[3] &= 15; r[7] &= 15; r[11] &= 15; r[15] &= 15;
        r[4] &= 252; r[8] &= 252; r[12] &= 252;
    }

    fn poly1305_mac(msg: &[u8], key: &[u8; 32]) -> [u8; 16] {
        let mut r_bytes = [0u8; 16];
        r_bytes.copy_from_slice(&key[..16]);
        Self::poly1305_clamp(&mut r_bytes);

        let r = u128::from_le_bytes(r_bytes);
        let s = u128::from_le_bytes(key[16..32].try_into().unwrap());

        let p = (1u128 << 130) - 5;

        let r0 = r & ((1u128 << 44) - 1);
        let r1 = (r >> 44) & ((1u128 << 44) - 1);
        let r2 = (r >> 88) & ((1u128 << 42) - 1);

        let mut h0: u128 = 0;
        let mut h1: u128 = 0;
        let mut h2: u128 = 0;

        for chunk in msg.chunks(16) {
            let len = chunk.len();
            let mut n0: u128 = 0;
            let mut n1: u128 = 0;
            let mut n2: u128 = 0;

            let top = len.min(6);
            for i in 0..top { n0 |= (chunk[i] as u128) << (8 * i); }
            n0 += 1;
            if len > 6 {
                let top2 = len.min(12);
                for i in 6..top2 { n1 |= (chunk[i] as u128) << (8 * (i - 6)); }
                n1 += 1;
            }
            if len > 12 {
                for i in 12..len { n2 |= (chunk[i] as u128) << (8 * (i - 12)); }
                n2 += 1;
            }

            h0 += n0; h1 += n1; h2 += n2;

            let d0 = h0*r0 + h1*r2*5 + h2*r1*5;
            let d1 = h0*r1 + h1*r0      + h2*r2*5;
            let d2 = h0*r2 + h1*r1      + h2*r0;

            h2 = d2 & ((1u128 << 42) - 1);
            h1 = d1 & ((1u128 << 44) - 1);
            h0 = d0 & ((1u128 << 44) - 1);

            h1 += d0 >> 44;
            h2 += d1 >> 44;
            h0 += (d2 >> 42) * 5;

            h1 += h0 >> 44; h0 &= (1u128 << 44) - 1;
            h2 += h1 >> 44; h1 &= (1u128 << 44) - 1;
        }

        // Final reduction: carry from h2 into h0 using 2^130 ≡ 5 (mod p)
        h0 += (h2 >> 42) * 5;
        h2 &= (1u128 << 42) - 1;
        h1 += h0 >> 44; h0 &= (1u128 << 44) - 1;
        h2 += h1 >> 44; h1 &= (1u128 << 44) - 1;
        h0 += (h2 >> 42) * 5;
        h2 &= (1u128 << 42) - 1;
        h1 += h0 >> 44; h0 &= (1u128 << 44) - 1;
        h2 += h1 >> 44; h1 &= (1u128 << 44) - 1;

        // Subtract p if h >= p
        let full = h0 | (h1 << 44) | (h2 << 88);
        let final_val = if full >= p { full - p } else { full };

        let tag = (final_val + s) % (1u128 << 128);
        let mut result = [0u8; 16];
        result.copy_from_slice(&tag.to_le_bytes()[..16]);
        result
    }
}

impl Aead for ChaCha20Poly1305 {
    fn encrypt(key: &[u8], nonce: &[u8], aad: &[u8], plaintext: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let key: [u8; 32] = key.try_into().expect("key must be 32 bytes");
        let nonce: [u8; 12] = nonce.try_into().expect("nonce must be 12 bytes");
        let block0 = Self::chacha20_block(&key, 0, &nonce);
        let mut poly_key = [0u8; 32];
        poly_key.copy_from_slice(&block0[..32]);
        let ciphertext = Self::chacha20_encrypt(&key, 1, &nonce, plaintext);
        let mut mac_input = Vec::new();
        mac_input.extend_from_slice(aad);
        mac_input.extend_from_slice(&pad16(aad));
        mac_input.extend_from_slice(&ciphertext);
        mac_input.extend_from_slice(&pad16(&ciphertext));
        mac_input.extend_from_slice(&le64(aad.len() as u64));
        mac_input.extend_from_slice(&le64(ciphertext.len() as u64));
        let tag = Self::poly1305_mac(&mac_input, &poly_key);
        (ciphertext, tag.to_vec())
    }

    fn decrypt(key: &[u8], nonce: &[u8], aad: &[u8], ciphertext: &[u8], tag: &[u8]) -> Result<Vec<u8>, AeadError> {
        let key: [u8; 32] = key.try_into().expect("key must be 32 bytes");
        let nonce: [u8; 12] = nonce.try_into().expect("nonce must be 12 bytes");
        let block0 = Self::chacha20_block(&key, 0, &nonce);
        let mut poly_key = [0u8; 32];
        poly_key.copy_from_slice(&block0[..32]);
        let mut mac_input = Vec::new();
        mac_input.extend_from_slice(aad);
        mac_input.extend_from_slice(&pad16(aad));
        mac_input.extend_from_slice(ciphertext);
        mac_input.extend_from_slice(&pad16(ciphertext));
        mac_input.extend_from_slice(&le64(aad.len() as u64));
        mac_input.extend_from_slice(&le64(ciphertext.len() as u64));
        let expected_tag = Self::poly1305_mac(&mac_input, &poly_key);
        let tag_bytes: [u8; 16] = tag.try_into().expect("tag must be 16 bytes");
        let mut diff = 0u8;
        for (a, b) in expected_tag.iter().zip(tag_bytes.iter()) { diff |= a ^ b; }
        if diff != 0 { return Err(AeadError("authentication tag verification failed".into())); }
        let plaintext = Self::chacha20_encrypt(&key, 1, &nonce, ciphertext);
        Ok(plaintext)
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn from_hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i+2], 16).unwrap()).collect()
}

fn verify_rfc8439() {
    println!("=== RFC 8439 Test Vectors ===\n");
    let key = from_hex("808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f");
    let nonce = from_hex("070000004041424344454647");
    let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
    let aad = from_hex("50515253c0c1c2c3c4c5c6c7");

    let (ct, tag) = ChaCha20Poly1305::encrypt(&key, &nonce, &aad, plaintext);
    let expected_ct_prefix = from_hex("d31a8d34648e60db0785322007644a67b8ac3a6b9e6972db1a3a8f49d8f54e0b36b0946d2d1a7428c0a570b6f8303f72a3f1c0f2d0a5e1d0db5b8ce16");
    let expected_tag = from_hex("1ae10b594f2e75bf215a342a10b1b1a8");

    let ct_match = ct[..expected_ct_prefix.len()].to_vec() == expected_ct_prefix;
    let tag_match = tag == expected_tag;
    println!("ChaCha20-Poly1305 AEAD (RFC 8439 SS2.8.2):");
    println!("  Ciphertext prefix match: {}", ct_match);
    println!("  Tag match:               {}", tag_match);
    println!("  Ciphertext: {}", hex(&ct));
    println!("  Tag:        {}", hex(&tag));

    let decrypted = ChaCha20Poly1305::decrypt(&key, &nonce, &aad, &ct, &tag).unwrap();
    println!("  Decrypted:  {}", String::from_utf8_lossy(&decrypted));
    println!();

    let b1_key = from_hex("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f");
    let b1_nonce = from_hex("000000090000004a00000000");
    let block1 = ChaCha20Poly1305::chacha20_block(&b1_key.try_into().unwrap(), 1, &b1_nonce.try_into().unwrap());
    println!("ChaCha20 Block 1 (RFC 8439 SS2.3.2):");
    println!("  First 32 bytes: {}", hex(&block1[..32]));
    println!();
}

fn demo_aead() {
    println!("=== AEAD Demo: Encrypt, Decrypt, Tamper ===\n");
    let key: [u8; 32] = [0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d, 0x8e, 0x8f,
                          0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b, 0x9c, 0x9d, 0x9e, 0x9f];
    let nonce: [u8; 12] = [0x07, 0x00, 0x00, 0x00, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47];
    let aad = [0x50, 0x51, 0x52, 0x53, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7];
    let message = b"Hello, AEAD world!";

    println!("Plaintext:  {}", String::from_utf8_lossy(message));
    println!("AAD:        {}", hex(&aad));

    let (ct, tag) = ChaCha20Poly1305::encrypt(&key, &nonce, &aad, message);
    println!("\nChaCha20-Poly1305:");
    println!("  Ciphertext: {}", hex(&ct));
    println!("  Tag:        {}", hex(&tag));

    let pt = ChaCha20Poly1305::decrypt(&key, &nonce, &aad, &ct, &tag).unwrap();
    println!("  Decrypted:  {}", String::from_utf8_lossy(&pt));

    println!("\n--- Tampered ciphertext (1 bit flip) ---");
    let mut bad_ct = ct.clone(); bad_ct[0] ^= 0x01;
    match ChaCha20Poly1305::decrypt(&key, &nonce, &aad, &bad_ct, &tag) {
        Ok(_) => println!("  ERROR: should have rejected!"),
        Err(e) => println!("  Rejected: {}", e),
    }

    println!("--- Tampered AAD (1 bit flip) ---");
    let mut bad_aad = aad; bad_aad[0] ^= 0x01;
    match ChaCha20Poly1305::decrypt(&key, &nonce, &bad_aad, &ct, &tag) {
        Ok(_) => println!("  ERROR: should have rejected!"),
        Err(e) => println!("  Rejected: {}", e),
    }

    println!("--- Tampered tag (1 bit flip) ---");
    let mut bad_tag = tag.clone(); bad_tag[0] ^= 0x01;
    match ChaCha20Poly1305::decrypt(&key, &nonce, &aad, &ct, &bad_tag) {
        Ok(_) => println!("  ERROR: should have rejected!"),
        Err(e) => println!("  Rejected: {}", e),
    }
    println!();
}

fn demo_ctr_aes_hmac() {
    println!("=== CTR-AES-HMAC (Teaching AEAD) Demo ===\n");
    let key = [0u8; 32];
    let nonce = [0u8; 12];
    let aad = b"header";
    let message = b"CTR-AES-HMAC authenticated encryption";

    let (ct, tag) = CtrAesHmac::encrypt(&key, &nonce, aad, message);
    println!("CTR-AES-HMAC EtM:");
    println!("  Ciphertext: {}", hex(&ct));
    println!("  Tag (16 of 32 bytes): {}", hex(&tag));

    let pt = CtrAesHmac::decrypt(&key, &nonce, aad, &ct, &tag).unwrap();
    println!("  Decrypted:  {}", String::from_utf8_lossy(&pt));

    println!("\n--- Tampered ciphertext ---");
    let mut bad_ct = ct.clone(); bad_ct[0] ^= 0xff;
    match CtrAesHmac::decrypt(&key, &nonce, aad, &bad_ct, &tag) {
        Ok(_) => println!("  ERROR: should have rejected!"),
        Err(e) => println!("  Rejected: {}", e),
    }

    println!("--- Tampered AAD ---");
    let bad_aad = b" HEADER";
    match CtrAesHmac::decrypt(&key, &nonce, bad_aad, &ct, &tag) {
        Ok(_) => println!("  ERROR: should have rejected!"),
        Err(e) => println!("  Rejected: {}", e),
    }
    println!();
}

fn demo_nonce_reuse_warning() {
    println!("=== Nonce Reuse: Why It's Catastrophic ===\n");
    println!("If you reuse the same nonce+key pair in ChaCha20 or GCM:");
    println!("  C1 XOR C2 = P1 XOR P2  (keystream is identical, so XOR cancels)\n");
    let key: [u8; 32] = [0x42; 32];
    let nonce: [u8; 12] = [0; 12];
    let msg1 = b"Attack at dawn!";
    let msg2 = b"Defend at dusk!";

    let (ct1, _) = ChaCha20Poly1305::encrypt(&key, &nonce, b"", msg1);
    let (ct2, _) = ChaCha20Poly1305::encrypt(&key, &nonce, b"", msg2);
    let xor_ct = xor_bytes(&ct1, &ct2);
    let xor_pt = xor_bytes(msg1, msg2);
    println!("  C1 XOR C2 = {}", hex(&xor_ct));
    println!("  P1 XOR P2  = {}", hex(&xor_pt));
    println!("  Match:      {}", xor_ct == xor_pt);
    println!();
    println!("  Mitigation:");
    println!("    - Counter-based nonces (deterministic, unique per session)");
    println!("    - XChaCha20-Poly1305 (192-bit nonce, safe for random generation)");
    println!("    - AES-GCM-SIV (nonce-misuse resistant, degrades gracefully)");
}

fn main() {
    println!("Authenticated Encryption (AEAD) - Phase 12, Lesson 08\n");
    println!("Three ways to combine encryption + MAC:\n");
    println!("  MAC-then-Encrypt (MtE):  tag=MAC(P), C=E(P||tag)");
    println!("    Decrypt before verify -> padding oracles (TLS 1.0, Lucky13)");
    println!("  Encrypt-and-MAC (E&M):  C=E(P), tag=MAC(P)");
    println!("    MAC over plaintext may leak info (SSH)");
    println!("  Encrypt-then-MAC (EtM):  C=E(P), tag=MAC(C)");
    println!("    Verify before decrypt -> provably secure (AEAD is EtM)\n");
    println!("{}\n", "-".repeat(60));

    demo_aead();
    demo_ctr_aes_hmac();
    verify_rfc8439();
    demo_nonce_reuse_warning();

    println!("\n{}", "-".repeat(60));
    println!("Key takeaways:");
    println!("  1. Always use AEAD (AES-GCM, ChaCha20-Poly1305) - never compose encrypt+MAC yourself");
    println!("  2. AEAD authenticates both ciphertext AND AAD - header tampering is caught");
    println!("  3. Never reuse a nonce with the same key (GCM: key recovery; ChaCha: plaintext XOR)");
    println!("  4. Prefer ChaCha20-Poly1305 on systems without AES-NI");
    println!("  5. Use AES-GCM-SIV or XChaCha20-Poly1305 for nonce-misuse resistance");
}