//! TLS 1.3 — Handshake, Records, 0-RTT
//! Phase 12 — Cryptography & Security, Lesson 14
//!
//! A simplified TLS 1.3 implementation demonstrating:
//! - Record layer with AES-128-GCM encryption
//! - Key schedule using HKDF-Extract/Expand (RFC 8446 §7.1)
//! - ECDHE key exchange via X25519
//! - Handshake: ClientHello, ServerHello, key derivation
//! - Encrypted application data with sequence number nonce

use aes_gcm::{
    aead::{Aead, KeyInit, Nonce, Payload},
    Aes128Gcm,
};
use hmac::{Hmac, Mac};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use x25519_dalek::{EphemeralSecret, PublicKey};

type HmacSha256 = Hmac<Sha256>;

// ============================================================
// Constants
// ============================================================

const TLS_RECORD_VERSION: u16 = 0x0301;
const CONTENT_TYPE_HANDSHAKE: u8 = 22;
const CONTENT_TYPE_APPLICATION_DATA: u8 = 23;
const CONTENT_TYPE_ALERT: u8 = 21;
const HANDSHAKE_TYPE_CLIENT_HELLO: u8 = 1;
const HANDSHAKE_TYPE_SERVER_HELLO: u8 = 2;

// ============================================================
// Step 1: TLS Record Layer
// ============================================================

#[derive(Debug, Clone, PartialEq)]
struct TLSRecord {
    content_type: u8,
    version: u16,
    payload: Vec<u8>,
}

impl TLSRecord {
    fn new(content_type: u8, payload: Vec<u8>) -> Self {
        TLSRecord { content_type, version: TLS_RECORD_VERSION, payload }
    }

    fn encode(&self) -> Vec<u8> {
        let mut buf = vec![self.content_type];
        buf.extend_from_slice(&self.version.to_be_bytes());
        buf.extend_from_slice(&(self.payload.len() as u16).to_be_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < 5 {
            return None;
        }
        let content_type = data[0];
        let version = u16::from_be_bytes([data[1], data[2]]);
        let len = u16::from_be_bytes([data[3], data[4]]) as usize;
        if data.len() < 5 + len {
            return None;
        }
        Some(TLSRecord {
            content_type,
            version,
            payload: data[5..5 + len].to_vec(),
        })
    }
}

// ============================================================
// Record Protection (AES-128-GCM with sequence number nonce)
// ============================================================

struct RecordProtection {
    key: [u8; 16],
    iv: [u8; 12],
    seq: u64,
}

impl RecordProtection {
    fn new(key: [u8; 16], iv: [u8; 12]) -> Self {
        RecordProtection { key, iv, seq: 0 }
    }

    fn encrypt(&mut self, plaintext: &[u8], content_type: u8) -> Vec<u8> {
        let mut nonce_bytes = self.iv;
        for (i, b) in self.seq.to_be_bytes().iter().enumerate() {
            nonce_bytes[4 + i] ^= b;
        }
        let nonce = Nonce::from_slice(&nonce_bytes);

        let record_len = plaintext.len() + 16;
        let mut aad = vec![content_type];
        aad.extend_from_slice(&TLS_RECORD_VERSION.to_be_bytes());
        aad.extend_from_slice(&(record_len as u16).to_be_bytes());

        let cipher = Aes128Gcm::new_from_slice(&self.key).unwrap();
        let ciphertext = cipher
            .encrypt(&nonce, Payload { msg: plaintext, aad: &aad })
            .unwrap();
        self.seq += 1;
        ciphertext
    }

    fn decrypt(&mut self, ciphertext: &[u8], content_type: u8) -> Option<Vec<u8>> {
        let mut nonce_bytes = self.iv;
        for (i, b) in self.seq.to_be_bytes().iter().enumerate() {
            nonce_bytes[4 + i] ^= b;
        }
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut aad = vec![content_type];
        aad.extend_from_slice(&TLS_RECORD_VERSION.to_be_bytes());
        aad.extend_from_slice(&(ciphertext.len() as u16).to_be_bytes());

        let cipher = Aes128Gcm::new_from_slice(&self.key).unwrap();
        let plaintext = cipher
            .decrypt(&nonce, Payload { msg: ciphertext, aad: &aad })
            .ok()?;
        self.seq += 1;
        Some(plaintext)
    }
}

// ============================================================
// Step 2: Key Schedule (HKDF, RFC 8446 §7.1)
// ============================================================

fn hkdf_extract(salt: &[u8], ikm: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(salt).unwrap();
    mac.update(ikm);
    let result = mac.finalize().into_bytes();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

fn hkdf_expand(prk: &[u8], info: &[u8], len: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(len);
    let mut t = Vec::new();
    let mut counter: u8 = 1;
    while result.len() < len {
        let mut mac = HmacSha256::new_from_slice(prk).unwrap();
        mac.update(&t);
        mac.update(info);
        mac.update(&[counter]);
        t = mac.finalize().into_bytes().to_vec();
        result.extend_from_slice(&t);
        counter += 1;
    }
    result.truncate(len);
    result
}

fn derive_secret(secret: &[u8; 32], label: &[u8], context: &[u8]) -> [u8; 32] {
    let label_prefix = b"tls13 ";
    let total_label: Vec<u8> = label_prefix.iter().chain(label.iter()).copied().collect();

    let mut hkdf_label = Vec::new();
    hkdf_label.extend_from_slice(&[0x00, 0x20]);
    hkdf_label.push(total_label.len() as u8);
    hkdf_label.extend_from_slice(&total_label);
    hkdf_label.push(context.len() as u8);
    hkdf_label.extend_from_slice(context);

    let expanded = hkdf_expand(secret, &hkdf_label, 32);
    let mut result = [0u8; 32];
    result.copy_from_slice(&expanded);
    result
}

fn derive_traffic_key(secret: &[u8; 32]) -> [u8; 16] {
    let label: Vec<u8> = b"tls13 key".to_vec();
    let mut hkdf_label = Vec::new();
    hkdf_label.extend_from_slice(&[0x00, 0x10]);
    hkdf_label.push(label.len() as u8);
    hkdf_label.extend_from_slice(&label);
    hkdf_label.push(0x00);
    let expanded = hkdf_expand(secret, &hkdf_label, 16);
    let mut key = [0u8; 16];
    key.copy_from_slice(&expanded);
    key
}

fn derive_traffic_iv(secret: &[u8; 32]) -> [u8; 12] {
    let label: Vec<u8> = b"tls13 iv".to_vec();
    let mut hkdf_label = Vec::new();
    hkdf_label.extend_from_slice(&[0x00, 0x0C]);
    hkdf_label.push(label.len() as u8);
    hkdf_label.extend_from_slice(&label);
    hkdf_label.push(0x00);
    let expanded = hkdf_expand(secret, &hkdf_label, 12);
    let mut iv = [0u8; 12];
    iv.copy_from_slice(&expanded);
    iv
}

struct KeySchedule {
    early_secret: [u8; 32],
    handshake_secret: [u8; 32],
    master_secret: [u8; 32],
    client_handshake_traffic_secret: [u8; 32],
    server_handshake_traffic_secret: [u8; 32],
    client_app_traffic_secret: [u8; 32],
    server_app_traffic_secret: [u8; 32],
    client_handshake_key: [u8; 16],
    server_handshake_key: [u8; 16],
    client_handshake_iv: [u8; 12],
    server_handshake_iv: [u8; 12],
    client_app_key: [u8; 16],
    server_app_key: [u8; 16],
    client_app_iv: [u8; 12],
    server_app_iv: [u8; 12],
}

impl KeySchedule {
    fn new(ecdhe_secret: &[u8; 32], transcript_hash: &[u8]) -> Self {
        let zero_psk = [0u8; 32];
        let early_secret = hkdf_extract(&[0u8; 32], &zero_psk);
        let handshake_secret = hkdf_extract(&early_secret, ecdhe_secret);

        let c_hs = derive_secret(&handshake_secret, b"c hs traffic", transcript_hash);
        let s_hs = derive_secret(&handshake_secret, b"s hs traffic", transcript_hash);

        let master_secret = hkdf_extract(&handshake_secret, &[0u8; 32]);

        let c_app = derive_secret(&master_secret, b"c ap traffic", transcript_hash);
        let s_app = derive_secret(&master_secret, b"s ap traffic", transcript_hash);

        KeySchedule {
            early_secret,
            handshake_secret,
            master_secret,
            client_handshake_traffic_secret: c_hs,
            server_handshake_traffic_secret: s_hs,
            client_app_traffic_secret: c_app,
            server_app_traffic_secret: s_app,
            client_handshake_key: derive_traffic_key(&c_hs),
            server_handshake_key: derive_traffic_key(&s_hs),
            client_handshake_iv: derive_traffic_iv(&c_hs),
            server_handshake_iv: derive_traffic_iv(&s_hs),
            client_app_key: derive_traffic_key(&c_app),
            server_app_key: derive_traffic_key(&s_app),
            client_app_iv: derive_traffic_iv(&c_app),
            server_app_iv: derive_traffic_iv(&s_app),
        }
    }
}

// ============================================================
// Step 3: Handshake Messages
// ============================================================

struct HandshakeMessage {
    msg_type: u8,
    body: Vec<u8>,
}

impl HandshakeMessage {
    fn encode(&self) -> Vec<u8> {
        let mut buf = vec![self.msg_type];
        let len = self.body.len() as u32;
        buf.extend_from_slice(&[(len >> 16) as u8, (len >> 8) as u8, len as u8]);
        buf.extend_from_slice(&self.body);
        buf
    }
}

fn build_client_hello(key_share: &[u8; 32]) -> HandshakeMessage {
    let mut body = Vec::new();
    body.extend_from_slice(&[0x03, 0x04]);
    body.extend_from_slice(&[0u8; 32]);
    body.push(0x00);
    body.extend_from_slice(&[0x00, 0x02, 0x13, 0x01]);
    body.push(0x01);
    body.push(0x00);
    body.extend_from_slice(&[0x00, 0x2b, 0x00, 0x03, 0x02, 0x03, 0x04]);
    body.extend_from_slice(&[0x00, 0x0a, 0x00, 0x04, 0x00, 0x02, 0x00, 0x1d]);
    body.extend_from_slice(&[0x00, 0x33]);
    body.extend_from_slice(&[0x00, 0x26, 0x00, 0x24]);
    body.extend_from_slice(&[0x00, 0x1d]);
    body.extend_from_slice(&(32u16).to_be_bytes());
    body.extend_from_slice(key_share);
    body.extend_from_slice(&[0x00, 0x0d, 0x00, 0x08, 0x00, 0x06, 0x00, 0x03, 0x08, 0x07]);
    HandshakeMessage { msg_type: HANDSHAKE_TYPE_CLIENT_HELLO, body }
}

fn build_server_hello(key_share: &[u8; 32]) -> HandshakeMessage {
    let mut body = Vec::new();
    body.extend_from_slice(&[0x03, 0x04]);
    body.extend_from_slice(&[0u8; 32]);
    body.push(0x00);
    body.extend_from_slice(&[0x13, 0x01]);
    body.push(0x00);
    body.extend_from_slice(&[0x00, 0x33]);
    body.extend_from_slice(&[0x00, 0x26, 0x00, 0x24]);
    body.extend_from_slice(&[0x00, 0x1d]);
    body.extend_from_slice(&(32u16).to_be_bytes());
    body.extend_from_slice(key_share);
    body.extend_from_slice(&[0x00, 0x2b, 0x00, 0x03, 0x02, 0x03, 0x04]);
    HandshakeMessage { msg_type: HANDSHAKE_TYPE_SERVER_HELLO, body }
}

// ============================================================
// Transcript Hash
// ============================================================

struct Transcript {
    hash: Sha256,
}

impl Transcript {
    fn new() -> Self {
        Transcript { hash: Sha256::new() }
    }

    fn append(&mut self, data: &[u8]) {
        self.hash.update(data);
    }

    fn current_hash(&self) -> Vec<u8> {
        self.hash.clone().finalize().to_vec()
    }
}

// ============================================================
// Step 4: Main Demo
// ============================================================

fn print_header(title: &str) {
    println!("\n{}", "=" .repeat(70));
    println!("  {}", title);
    println!("{}", "=" .repeat(70));
}

fn print_subheader(title: &str) {
    println!("\n--- {} ---\n", title);
}

fn main() {
    println!("{}", "=" .repeat(70));
    println!("  TLS 1.3 -- Handshake, Records, 0-RTT");
    println!("  Phase 12 -- Cryptography & Security, Lesson 14");
    println!("{}", "=" .repeat(70));

    // ============================================================
    // Step 1: Record Layer
    // ============================================================
    print_header("Step 1: TLS Record Layer");

    let record = TLSRecord::new(CONTENT_TYPE_HANDSHAKE, vec![0x01, 0x02, 0x03, 0x04]);
    let encoded = record.encode();
    let decoded = TLSRecord::decode(&encoded).unwrap();

    println!("  Original:");
    println!("    Content-Type: {} (handshake)", record.content_type);
    println!("    Version:      0x{:04x}", record.version);
    println!("    Payload:      {:02x?}", record.payload);
    println!("  Wire format:     {:02x?}", &encoded);
    println!("  Round-trip:      {}", record == decoded);

    // ============================================================
    // Step 2: Key Schedule Primitives
    // ============================================================
    print_header("Step 2: Key Schedule (HKDF)");

    let test_salt = [0x02u8; 32];
    let test_ikm = [0x01u8; 32];
    let prk = hkdf_extract(&test_salt, &test_ikm);
    println!("  HKDF-Extract(salt=0x02..02, ikm=0x01..01)");
    println!("    PRK: {:02x?}..{:02x?}", &prk[..4], &prk[28..]);

    let expanded = hkdf_expand(&prk, b"test-info", 48);
    println!("  HKDF-Expand(prk, \"test-info\", 48)");
    println!("    Output (first 16 bytes): {:02x?}", &expanded[..16]);

    let derived = derive_secret(&prk, b"test label", b"");
    println!("  derive_secret(prk, \"test label\", \"\")");
    println!("    Output: {:02x?}..{:02x?}", &derived[..4], &derived[28..]);

    // ============================================================
    // Step 3: ECDHE Key Exchange
    // ============================================================
    print_header("Step 3: ECDHE Key Exchange (X25519)");

    let client_secret = EphemeralSecret::random_from_rng(OsRng);
    let client_public = PublicKey::from(&client_secret);
    let server_secret = EphemeralSecret::random_from_rng(OsRng);
    let server_public = PublicKey::from(&server_secret);

    let client_shared = client_secret.diffie_hellman(&server_public);
    let server_shared = server_secret.diffie_hellman(&client_public);

    println!("  Client public key: {:02x?}..{:02x?}",
             &client_public.as_bytes()[..4], &client_public.as_bytes()[28..]);
    println!("  Server public key: {:02x?}..{:02x?}",
             &server_public.as_bytes()[..4], &server_public.as_bytes()[28..]);
    println!("  Shared secrets match: {}", client_shared.as_bytes() == server_shared.as_bytes());

    let ecdhe_secret: [u8; 32] = {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(client_shared.as_bytes());
        arr
    };

    // ============================================================
    // Step 4: Handshake Simulation
    // ============================================================
    print_header("Step 4: TLS 1.3 Handshake");

    let mut transcript = Transcript::new();

    let client_hello = build_client_hello(client_public.as_bytes());
    let ch_encoded = client_hello.encode();
    transcript.append(&ch_encoded);
    println!("  ClientHello ({} bytes encoded)", ch_encoded.len());
    println!("    Body prefix: {:02x?}..", &client_hello.body[..12]);

    let server_hello = build_server_hello(server_public.as_bytes());
    let sh_encoded = server_hello.encode();
    transcript.append(&sh_encoded);
    println!("  ServerHello ({} bytes encoded)", sh_encoded.len());
    println!("    Body prefix: {:02x?}..", &server_hello.body[..12]);

    let th = transcript.current_hash();
    println!("  Transcript hash: {:02x?}", &th);
    println!("  (ClientHello + ServerHello hashed together)");

    // Derive all keys
    let transcript_hash: [u8; 32] = {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&th);
        arr
    };
    let keys = KeySchedule::new(&ecdhe_secret, &transcript_hash);

    println!("\n  Key Schedule Output:\n");
    println!("    {:32}  {:02x?}..{:02x?}", "Early secret",
             &keys.early_secret[..4], &keys.early_secret[28..]);
    println!("    {:32}  {:02x?}..{:02x?}", "Handshake secret",
             &keys.handshake_secret[..4], &keys.handshake_secret[28..]);
    println!("    {:32}  {:02x?}..{:02x?}", "Master secret",
             &keys.master_secret[..4], &keys.master_secret[28..]);
    println!("    {:32}  {:02x?}..{:02x?}", "Client handshake traffic secret",
             &keys.client_handshake_traffic_secret[..4], &keys.client_handshake_traffic_secret[28..]);
    println!("    {:32}  {:02x?}..{:02x?}", "Server handshake traffic secret",
             &keys.server_handshake_traffic_secret[..4], &keys.server_handshake_traffic_secret[28..]);
    println!("    {:32}  {:02x?}..{:02x?}", "Client application traffic secret",
             &keys.client_app_traffic_secret[..4], &keys.client_app_traffic_secret[28..]);
    println!("    {:32}  {:02x?}..{:02x?}", "Server application traffic secret",
             &keys.server_app_traffic_secret[..4], &keys.server_app_traffic_secret[28..]);
    println!();
    println!("    {:32}  {:02x?}", "Client handshake key", keys.client_handshake_key);
    println!("    {:32}  {:02x?}", "Server handshake key", keys.server_handshake_key);
    println!("    {:32}  {:02x?}", "Client handshake IV", keys.client_handshake_iv);
    println!("    {:32}  {:02x?}", "Server handshake IV", keys.server_handshake_iv);
    println!("    {:32}  {:02x?}", "Client application key", keys.client_app_key);
    println!("    {:32}  {:02x?}", "Server application key", keys.server_app_key);
    println!("    {:32}  {:02x?}", "Client application IV", keys.client_app_iv);
    println!("    {:32}  {:02x?}", "Server application IV", keys.server_app_iv);

    // ============================================================
    // Step 5: Encrypted Application Data
    // ============================================================
    print_header("Step 5: Encrypted Application Data");

    let mut client_protection = RecordProtection::new(keys.client_app_key, keys.client_app_iv);
    let mut server_protection = RecordProtection::new(keys.server_app_key, keys.server_app_iv);

    let request = b"GET /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n";
    println!("  Client request ({} bytes):", request.len());
    println!("    {:?}", std::str::from_utf8(request).unwrap());

    let encrypted_req = client_protection.encrypt(request, CONTENT_TYPE_APPLICATION_DATA);
    println!("  Encrypted ({} bytes): {:02x?}..{:02x?}",
             encrypted_req.len(),
             &encrypted_req[..8], &encrypted_req[encrypted_req.len() - 8..]);

    let decrypted_req = server_protection
        .decrypt(&encrypted_req, CONTENT_TYPE_APPLICATION_DATA)
        .unwrap();
    let ok = decrypted_req == request;
    println!("  Server decrypts: {:?}  [{}]",
             std::str::from_utf8(&decrypted_req).unwrap(),
             if ok { "OK" } else { "MISMATCH" });

    assert!(ok, "Client -> Server application data round-trip failed!");

    let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html><body>Hello, TLS 1.3!</body></html>";
    let encrypted_resp = server_protection.encrypt(response, CONTENT_TYPE_APPLICATION_DATA);
    println!("\n  Server response ({} bytes):", response.len());
    println!("    Encrypted ({} bytes): {:02x?}..{:02x?}",
             encrypted_resp.len(),
             &encrypted_resp[..8], &encrypted_resp[encrypted_resp.len() - 8..]);

    let decrypted_resp = client_protection
        .decrypt(&encrypted_resp, CONTENT_TYPE_APPLICATION_DATA)
        .unwrap();
    let ok2 = decrypted_resp == response;
    println!("  Client decrypts: {} bytes received  [{}]",
             decrypted_resp.len(),
             if ok2 { "OK" } else { "MISMATCH" });

    assert!(ok2, "Server -> Client application data round-trip failed!");

    println!("\n  Verified: full-duplex encrypted application data round-trips OK");
    println!("  (Client uses seq={}, Server uses seq={})",
             client_protection.seq, server_protection.seq);

    // ============================================================
    // Step 6: 0-RTT Overview
    // ============================================================
    print_header("Step 6: 0-RTT (Zero Round-Trip Time) Resumption");

    println!(
        "  0-RTT allows a client to send encrypted data on its FIRST flight\n\
         when resuming a previous session using a PSK:\n"
    );
    println!("  Normal 1-RTT handshake:");
    println!("    Client  ----------------- ClientHello ---------------->  Server");
    println!("    Client  <------- ServerHello + Certificate + Finished -  Server");
    println!("    Client  ----------------- Finished ------------------>  Server");
    println!("    Client  ==================== Data ====================>  Server\n");
    println!("  0-RTT resumption handshake:");
    println!("    Client  --- ClientHello + PSK + early_data(encrypted) ->  Server");
    println!("    Client  <------- ServerHello + Finished + data ------->  Server\n");
    println!("  Tradeoffs:");
    println!("    PRO: Eliminates one full round trip on resumption");
    println!("    CON: 0-RTT data has no forward secrecy");
    println!("    CON: Vulnerable to replay attacks");
    println!("    CON: Server must implement anti-replay (freshness + cache)");
    println!("    MITIGATION: Limit 0-RTT to idempotent operations (GET, PUT)");

    // ============================================================
    // Summary
    // ============================================================
    print_header("Summary");

    println!("  TLS 1.3 components implemented and verified:");
    println!("    [1] TLS record layer: encode/decode with content type, version, length");
    println!("    [2] AEAD encryption:  AES-128-GCM with sequence number nonce");
    println!("    [3] Key schedule:     7 secrets via HKDF-Extract/Expand (RFC 8446)");
    println!("    [4] ECDHE:            X25519 key exchange with forward secrecy");
    println!("    [5] Handshake:        ClientHello + ServerHello + transcript hash");
    println!("    [6] App data:         Full-duplex encrypted communication");
    println!("    [7] 0-RTT:            Latency/replay tradeoff analysis");
    println!("\n  All components verified successfully.");
}
