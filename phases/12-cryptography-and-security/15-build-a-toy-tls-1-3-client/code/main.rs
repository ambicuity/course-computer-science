// Phase 12 — Cryptography & Security, Lesson 15
// Build a Toy TLS 1.3 Client
//
// A local loopback TLS 1.3 handshake over TCP demonstrating:
//   - X25519 ECDHE key exchange with ephemeral keypairs
//   - TLS 1.3 key schedule via HKDF-Extract/Expand (RFC 8446 §7.1)
//   - AES-128-GCM record protection with sequence-number nonces
//   - Ed25519 CertificateVerify sign-and-verify over transcript hash
//   - Mutual Finished message integrity verification
//   - Encrypted application data exchange

use aes_gcm::{Aes128Gcm, aead::{Aead, KeyInit, Nonce, Payload}};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use hmac::{Hmac, Mac};
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;
use x25519_dalek::{EphemeralSecret, PublicKey};

type HmacSha256 = Hmac<Sha256>;

const CT_HANDSHAKE: u8 = 22;
const CT_APP_DATA: u8 = 23;
const CT_ALERT: u8 = 21;
const RECORD_VERSION: u16 = 0x0301;
const HS_CLIENT_HELLO: u8 = 1;
const HS_SERVER_HELLO: u8 = 2;
const HS_ENCRYPTED_EXTENSIONS: u8 = 8;
const HS_CERTIFICATE: u8 = 11;
const HS_CERTIFICATE_VERIFY: u8 = 15;
const HS_FINISHED: u8 = 20;
const SERVER_PORT: u16 = 9843;

// ── Part 1: TLS Record Layer ─────────────────────────────────

struct TLSRecord {
    content_type: u8,
    payload: Vec<u8>,
}

impl TLSRecord {
    fn encode(&self) -> Vec<u8> {
        let mut buf = vec![self.content_type];
        buf.extend_from_slice(&RECORD_VERSION.to_be_bytes());
        buf.extend_from_slice(&(self.payload.len() as u16).to_be_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }
}

fn read_record(stream: &mut impl Read) -> TLSRecord {
    let mut header = [0u8; 5];
    stream.read_exact(&mut header).expect("read record header");
    let len = u16::from_be_bytes([header[3], header[4]]) as usize;
    let mut payload = vec![0u8; len];
    if len > 0 {
        stream.read_exact(&mut payload).expect("read record payload");
    }
    TLSRecord { content_type: header[0], payload }
}

// ── Part 2: Record Protection (AES-128-GCM) ──────────────────

struct RecordProtection {
    key: [u8; 16],
    iv: [u8; 12],
    seq: u64,
}

impl RecordProtection {
    fn new(key: [u8; 16], iv: [u8; 12]) -> Self {
        RecordProtection { key, iv, seq: 0 }
    }

    fn encrypt(&mut self, plaintext: &[u8], inner_ct: u8) -> Vec<u8> {
        let mut inner = plaintext.to_vec();
        inner.push(inner_ct);
        let mut nonce = self.iv;
        for (i, b) in self.seq.to_be_bytes().iter().enumerate() {
            nonce[4 + i] ^= b;
        }
        let ct_len = inner.len() + 16;
        let mut aad = vec![CT_APP_DATA];
        aad.extend_from_slice(&RECORD_VERSION.to_be_bytes());
        aad.extend_from_slice(&(ct_len as u16).to_be_bytes());
        let cipher = Aes128Gcm::new_from_slice(&self.key).unwrap();
        let encrypted = cipher
            .encrypt(Nonce::from_slice(&nonce),
                     Payload { msg: &inner, aad: &aad })
            .unwrap();
        self.seq += 1;
        let mut record = vec![CT_APP_DATA];
        record.extend_from_slice(&RECORD_VERSION.to_be_bytes());
        record.extend_from_slice(&(encrypted.len() as u16).to_be_bytes());
        record.extend_from_slice(&encrypted);
        record
    }

    fn decrypt(&mut self, data: &[u8], outer_ct: u8) -> Option<(Vec<u8>, u8)> {
        let mut nonce = self.iv;
        for (i, b) in self.seq.to_be_bytes().iter().enumerate() {
            nonce[4 + i] ^= b;
        }
        let mut aad = vec![outer_ct];
        aad.extend_from_slice(&RECORD_VERSION.to_be_bytes());
        aad.extend_from_slice(&(data.len() as u16).to_be_bytes());
        let cipher = Aes128Gcm::new_from_slice(&self.key).unwrap();
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&nonce),
                     Payload { msg: data, aad: &aad })
            .ok()?;
        self.seq += 1;
        let inner_ct = *plaintext.last()?;
        let msg = plaintext[..plaintext.len() - 1].to_vec();
        Some((msg, inner_ct))
    }
}

// ── Part 3: HKDF Primitives ──────────────────────────────────

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

fn build_hkdf_label(length: u16, label: &[u8], context: &[u8]) -> Vec<u8> {
    let full_label: Vec<u8> = b"tls13 ".iter().chain(label).copied().collect();
    let mut out = Vec::new();
    out.extend_from_slice(&length.to_be_bytes());
    out.push(full_label.len() as u8);
    out.extend_from_slice(&full_label);
    out.push(context.len() as u8);
    out.extend_from_slice(context);
    out
}

fn derive_secret(secret: &[u8; 32], label: &[u8], context: &[u8]) -> [u8; 32] {
    let info = build_hkdf_label(32, label, context);
    let expanded = hkdf_expand(secret, &info, 32);
    let mut result = [0u8; 32];
    result.copy_from_slice(&expanded);
    result
}

fn derive_key(secret: &[u8; 32]) -> [u8; 16] {
    let info = build_hkdf_label(16, b"key", b"");
    let expanded = hkdf_expand(secret, &info, 16);
    let mut key = [0u8; 16];
    key.copy_from_slice(&expanded);
    key
}

fn derive_iv(secret: &[u8; 32]) -> [u8; 12] {
    let info = build_hkdf_label(12, b"iv", b"");
    let expanded = hkdf_expand(secret, &info, 12);
    let mut iv = [0u8; 12];
    iv.copy_from_slice(&expanded);
    iv
}

fn derive_finished_key(secret: &[u8; 32]) -> [u8; 32] {
    let info = build_hkdf_label(32, b"finished", b"");
    let expanded = hkdf_expand(secret, &info, 32);
    let mut key = [0u8; 32];
    key.copy_from_slice(&expanded);
    key
}

// ── Part 4: Transcript Hash ──────────────────────────────────

struct Transcript {
    hasher: Sha256,
}

impl Transcript {
    fn new() -> Self {
        Transcript { hasher: Sha256::new() }
    }

    fn append(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    fn snapshot(&self) -> [u8; 32] {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&self.hasher.clone().finalize());
        hash
    }
}

// ── Part 5: Finished Computation ─────────────────────────────

fn compute_finished(finished_key: &[u8; 32], transcript_hash: &[u8; 32]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(finished_key).unwrap();
    mac.update(transcript_hash);
    mac.finalize().into_bytes().to_vec()
}

// ── Part 6: Handshake Message Encoding ───────────────────────

fn encode_hs(msg_type: u8, body: &[u8]) -> Vec<u8> {
    let mut buf = vec![msg_type];
    let len = body.len() as u32;
    buf.extend_from_slice(&[(len >> 16) as u8, (len >> 8) as u8, len as u8]);
    buf.extend_from_slice(body);
    buf
}

fn encode_hs_record(msg_type: u8, body: &[u8]) -> Vec<u8> {
    TLSRecord { content_type: CT_HANDSHAKE, payload: encode_hs(msg_type, body) }.encode()
}

fn build_client_hello(key_share: &[u8; 32]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&[0x03, 0x03]);
    let mut random = [0u8; 32];
    let mut rng = OsRng;
    rng.fill_bytes(&mut random);
    body.extend_from_slice(&random);

    body.push(0x00);
    body.extend_from_slice(&[0x00, 0x02, 0x13, 0x01]);
    body.push(0x01);
    body.push(0x00);

    let mut exts = Vec::new();
    let sv = vec![0x02, 0x03, 0x04];
    exts.extend_from_slice(&[0x00, 0x2b]);
    exts.extend_from_slice(&(sv.len() as u16).to_be_bytes());
    exts.extend_from_slice(&sv);

    let mut ks_entry = Vec::new();
    ks_entry.extend_from_slice(&[0x00, 0x1d]);
    ks_entry.extend_from_slice(&[0x00, 0x20]);
    ks_entry.extend_from_slice(key_share);
    let mut ks_data = Vec::new();
    ks_data.extend_from_slice(&(ks_entry.len() as u16).to_be_bytes());
    ks_data.extend_from_slice(&ks_entry);
    exts.extend_from_slice(&[0x00, 0x33]);
    exts.extend_from_slice(&(ks_data.len() as u16).to_be_bytes());
    exts.extend_from_slice(&ks_data);

    let sigs: Vec<u8> = vec![0x00, 0x06, 0x08, 0x07, 0x04, 0x03, 0x08, 0x04];
    exts.extend_from_slice(&[0x00, 0x0d]);
    exts.extend_from_slice(&(sigs.len() as u16).to_be_bytes());
    exts.extend_from_slice(&sigs);

    let groups: Vec<u8> = vec![0x00, 0x02, 0x00, 0x1d];
    exts.extend_from_slice(&[0x00, 0x0a]);
    exts.extend_from_slice(&(groups.len() as u16).to_be_bytes());
    exts.extend_from_slice(&groups);

    body.extend_from_slice(&(exts.len() as u16).to_be_bytes());
    body.extend_from_slice(&exts);
    encode_hs(HS_CLIENT_HELLO, &body)
}

fn build_server_hello(key_share: &[u8; 32]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&[0x03, 0x03]);
    let mut random = [0u8; 32];
    let mut rng = OsRng;
    rng.fill_bytes(&mut random);
    body.extend_from_slice(&random);
    body.push(0x00);
    body.extend_from_slice(&[0x13, 0x01]);
    body.push(0x01);
    body.push(0x00);

    let mut exts = Vec::new();
    let mut ks = Vec::new();
    ks.extend_from_slice(&[0x00, 0x1d]);
    ks.extend_from_slice(&[0x00, 0x20]);
    ks.extend_from_slice(key_share);
    exts.extend_from_slice(&[0x00, 0x33]);
    exts.extend_from_slice(&(ks.len() as u16).to_be_bytes());
    exts.extend_from_slice(&ks);

    let sv: Vec<u8> = vec![0x03, 0x04];
    exts.extend_from_slice(&[0x00, 0x2b]);
    exts.extend_from_slice(&(sv.len() as u16).to_be_bytes());
    exts.extend_from_slice(&sv);

    body.extend_from_slice(&(exts.len() as u16).to_be_bytes());
    body.extend_from_slice(&exts);
    encode_hs(HS_SERVER_HELLO, &body)
}

fn build_encrypted_extensions() -> Vec<u8> {
    encode_hs(HS_ENCRYPTED_EXTENSIONS, &[0x00, 0x00])
}

fn build_certificate(verifying_key: &VerifyingKey) -> Vec<u8> {
    let raw = verifying_key.to_bytes();
    let mut entry = Vec::new();
    entry.extend_from_slice(&((raw.len() as u32).to_be_bytes()[1..])); // 3-byte length
    entry.extend_from_slice(&raw);
    entry.extend_from_slice(&[0x00, 0x00]); // no extensions

    let mut body = Vec::new();
    body.extend_from_slice(&((entry.len() as u32).to_be_bytes()[1..])); // 3-byte list length
    body.extend_from_slice(&entry);
    encode_hs(HS_CERTIFICATE, &body)
}

fn build_certificate_verify(signing_key: &SigningKey, transcript_hash: &[u8; 32]) -> Vec<u8> {
    let context = b"TLS 1.3, server CertificateVerify\0";
    let mut signed_data = Vec::new();
    signed_data.extend_from_slice(context);
    signed_data.extend_from_slice(transcript_hash);
    let signature: Signature = signing_key.sign(&signed_data);
    let sig_bytes = signature.to_bytes();

    let mut body = Vec::new();
    body.extend_from_slice(&[0x08, 0x07]); // Ed25519
    body.extend_from_slice(&(sig_bytes.len() as u16).to_be_bytes());
    body.extend_from_slice(&sig_bytes);
    encode_hs(HS_CERTIFICATE_VERIFY, &body)
}

// ── Part 7: Extension Parsing ────────────────────────────────

fn find_extension(data: &[u8], ext_type: u16) -> Option<&[u8]> {
    let mut off = 0;
    while off + 4 <= data.len() {
        let t = u16::from_be_bytes([data[off], data[off + 1]]);
        let len = u16::from_be_bytes([data[off + 2], data[off + 3]]) as usize;
        if t == ext_type {
            return Some(&data[off + 4..off + 4 + len]);
        }
        off += 4 + len;
    }
    None
}

fn skip_handshake_body(data: &[u8]) -> Option<&[u8]> {
    if data.len() < 4 { return None; }
    let len = ((data[1] as usize) << 16) | ((data[2] as usize) << 8) | (data[3] as usize);
    if data.len() < 4 + len { return None; }
    Some(&data[4..4 + len])
}

fn extract_key_share_from_ch(ch: &[u8]) -> Option<[u8; 32]> {
    let body = skip_handshake_body(ch)?;
    let mut off: usize = 35;
    if body.len() <= off { return None; }
    off += 1 + body[off] as usize;
    if body.len() < off + 2 { return None; }
    off += 2 + u16::from_be_bytes([body[off], body[off + 1]]) as usize;
    if body.len() < off + 1 { return None; }
    off += 1 + body[off] as usize;
    if body.len() < off + 2 { return None; }
    let ext_len = u16::from_be_bytes([body[off], body[off + 1]]) as usize;
    off += 2;
    if body.len() < off + ext_len { return None; }
    let ext_data = find_extension(&body[off..off + ext_len], 0x0033)?;
    if ext_data.len() < 2 { return None; }
    let shares_len = u16::from_be_bytes([ext_data[0], ext_data[1]]) as usize;
    if ext_data.len() < 2 + shares_len { return None; }
    let entry = &ext_data[2..2 + shares_len];
    if entry.len() < 4 { return None; }
    let key_len = u16::from_be_bytes([entry[2], entry[3]]) as usize;
    if key_len != 32 || entry.len() < 4 + key_len { return None; }
    let mut key = [0u8; 32];
    key.copy_from_slice(&entry[4..4 + 32]);
    Some(key)
}

fn extract_key_share_from_sh(sh: &[u8]) -> Option<[u8; 32]> {
    let body = skip_handshake_body(sh)?;
    let mut off: usize = 35;
    if body.len() <= off { return None; }
    off += 1 + body[off] as usize;
    if body.len() < off + 2 { return None; }
    off += 2 + u16::from_be_bytes([body[off], body[off + 1]]) as usize;
    if body.len() < off + 1 { return None; }
    off += 1 + body[off] as usize;
    if body.len() < off + 2 { return None; }
    let ext_len = u16::from_be_bytes([body[off], body[off + 1]]) as usize;
    off += 2;
    if body.len() < off + ext_len { return None; }
    let ext_data = find_extension(&body[off..off + ext_len], 0x0033)?;
    if ext_data.len() < 4 { return None; }
    let key_len = u16::from_be_bytes([ext_data[2], ext_data[3]]) as usize;
    if key_len != 32 || ext_data.len() < 4 + key_len { return None; }
    let mut key = [0u8; 32];
    key.copy_from_slice(&ext_data[4..4 + 32]);
    Some(key)
}

fn parse_hs_messages(data: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut msgs = Vec::new();
    let mut off = 0;
    while off + 4 <= data.len() {
        let t = data[off];
        let len = ((data[off + 1] as usize) << 16)
                | ((data[off + 2] as usize) << 8)
                | (data[off + 3] as usize);
        if off + 4 + len > data.len() { break; }
        msgs.push((t, data[off + 4..off + 4 + len].to_vec()));
        off += 4 + len;
    }
    msgs
}

// ── Part 8: Key Schedule ─────────────────────────────────────

fn derive_hs_keys(ecdhe: &[u8; 32], hello_hash: &[u8; 32]) -> ([u8; 32], [u8; 16], [u8; 12], [u8; 16], [u8; 12], [u8; 32], [u8; 32]) {
    let early = hkdf_extract(&[0u8; 32], &[0u8; 32]);
    let early_derived = derive_secret(&early, b"derived", b"");
    let hs = hkdf_extract(&early_derived, ecdhe);

    let c_hs = derive_secret(&hs, b"c hs traffic", hello_hash);
    let s_hs = derive_secret(&hs, b"s hs traffic", hello_hash);

    (hs,
     derive_key(&c_hs), derive_iv(&c_hs),
     derive_key(&s_hs), derive_iv(&s_hs),
     derive_finished_key(&c_hs), derive_finished_key(&s_hs))
}

fn derive_app_keys(hs: &[u8; 32], handshake_hash: &[u8; 32]) -> ([u8; 16], [u8; 12], [u8; 16], [u8; 12]) {
    let hs_derived = derive_secret(hs, b"derived", b"");
    let ms = hkdf_extract(&hs_derived, &[0u8; 32]);
    let c_app = derive_secret(&ms, b"c ap traffic", handshake_hash);
    let s_app = derive_secret(&ms, b"s ap traffic", handshake_hash);
    (derive_key(&c_app), derive_iv(&c_app), derive_key(&s_app), derive_iv(&s_app))
}

// ── Part 9: Printing Helpers ─────────────────────────────────

fn hex_prefix(data: &[u8], n: usize) -> String {
    let show = data.len().min(n);
    hex::encode(&data[..show])
}

fn print_verdict(ok: bool, label: &str) {
    println!("  {}  {}", if ok { "✓" } else { "✗" }, label);
}

// ── Part 10: Server ──────────────────────────────────────────

fn run_server() {
    println!("── Server starting on 127.0.0.1:{} ──", SERVER_PORT);

    let server_eph = EphemeralSecret::random_from_rng(OsRng);
    let server_pub = PublicKey::from(&server_eph);
    let mut csprng = OsRng;
    let server_signing = SigningKey::generate(&mut csprng);
    let server_verifying = server_signing.verifying_key();

    let listener = TcpListener::bind(("127.0.0.1", SERVER_PORT)).expect("bind");
    let (mut stream, _) = listener.accept().expect("accept");

    println!("  Connection accepted");

    // Read ClientHello
    let ch_rec = read_record(&mut stream);
    let ch_encoded = ch_rec.payload.clone();
    let client_key = extract_key_share_from_ch(&ch_encoded).expect("parse CH key_share");
    println!("  Client key_share extracted: {}..", hex_prefix(&client_key, 8));

    let client_pub = PublicKey::from(client_key);
    let shared = server_eph.diffie_hellman(&client_pub);
    let mut ecdhe = [0u8; 32];
    ecdhe.copy_from_slice(shared.as_bytes());
    println!("  ECDHE shared secret computed");

    // Build and send ServerHello
    let sh = build_server_hello(server_pub.as_bytes());
    let sh_encoded = sh.clone();
    stream.write_all(&encode_hs_record(HS_SERVER_HELLO, &skip_handshake_body(&sh).unwrap())).unwrap();

    let mut tx = Transcript::new();
    tx.append(&ch_encoded);
    tx.append(&sh_encoded);
    let hello_hash = tx.snapshot();

    let (hs, c_hs_key, c_hs_iv, s_hs_key, s_hs_iv, c_hs_fk, s_hs_fk) =
        derive_hs_keys(&ecdhe, &hello_hash);
    let mut swrite = RecordProtection::new(s_hs_key, s_hs_iv);
    let mut cwrite = RecordProtection::new(c_hs_key, c_hs_iv);

    println!("  Handshake keys derived from HelloHash");

    // Build EncryptedExtensions, Certificate, CertificateVerify
    let ee = build_encrypted_extensions();
    let cert = build_certificate(&server_verifying);
    tx.append(&ee);
    tx.append(&cert);
    let cv_transcript = tx.snapshot();
    let cv = build_certificate_verify(&server_signing, &cv_transcript);
    tx.append(&cv);

    // Build server Finished
    let sf_transcript = tx.snapshot();
    let sf_verify = compute_finished(&s_hs_fk, &sf_transcript);
    let sf = encode_hs(HS_FINISHED, &sf_verify);
    tx.append(&sf);

    // Send encrypted flight: EE || Cert || CV || SF
    let mut combined = Vec::new();
    combined.extend_from_slice(&ee);
    combined.extend_from_slice(&cert);
    combined.extend_from_slice(&cv);
    combined.extend_from_slice(&sf);
    let flight = swrite.encrypt(&combined, CT_HANDSHAKE);
    stream.write_all(&flight).unwrap();
    println!("  EncryptedExtensions + Certificate + CertificateVerify + Finished sent");

    // Read client Finished
    let cf_rec = read_record(&mut stream);
    let cf_encrypted = cf_rec.payload;
    let (cf_plain, cf_inner) = cwrite.decrypt(&cf_encrypted, CT_APP_DATA).expect("decrypt CF");
    assert_eq!(cf_inner, CT_HANDSHAKE);
    let cf_body = skip_handshake_body(&cf_plain).expect("CF body");

    let expected_cf = compute_finished(&c_hs_fk, &tx.snapshot());
    let cf_ok = cf_body == expected_cf.as_slice();
    print_verdict(cf_ok, "Client Finished verified");

    // Derive app keys from full handshake hash
    let cf_encoded = encode_hs(HS_FINISHED, cf_body);
    tx.append(&cf_encoded);
    let handshake_hash = tx.snapshot();
    let (c_ak, c_ai, s_ak, s_ai) = derive_app_keys(&hs, &handshake_hash);
    let mut sapp = RecordProtection::new(s_ak, s_ai);
    let mut capp = RecordProtection::new(c_ak, c_ai);
    println!("  Application keys derived from HandshakeHash");

    // Read and echo application data
    let app_rec = read_record(&mut stream);
    let (app_data, _) = capp.decrypt(&app_rec.payload, CT_APP_DATA).expect("decrypt app data");
    println!("  Received application data ({} bytes): {}", app_data.len(),
             String::from_utf8_lossy(&app_data));

    let echo = sapp.encrypt(&app_data, CT_APP_DATA);
    stream.write_all(&echo).unwrap();
    println!("  Echoed back");

    // Send close_notify
    let alert = vec![0x01, 0x00];
    let close = sapp.encrypt(&alert, CT_ALERT);
    let _ = stream.write_all(&close);
    println!("  Connection closed");
}

// ── Part 11: Client ──────────────────────────────────────────

fn run_client() {
    println!("\n── Client connecting to 127.0.0.1:{} ──", SERVER_PORT);
    thread::sleep(Duration::from_millis(100));
    let mut stream = TcpStream::connect(("127.0.0.1", SERVER_PORT)).expect("connect");

    // Generate keypair and send ClientHello
    let client_eph = EphemeralSecret::random_from_rng(OsRng);
    let client_pub = PublicKey::from(&client_eph);
    let ch = build_client_hello(client_pub.as_bytes());
    let ch_encoded = ch.clone();
    stream.write_all(&encode_hs_record(HS_CLIENT_HELLO, skip_handshake_body(&ch).unwrap())).unwrap();
    println!("  ClientHello sent");

    // Read ServerHello
    let sh_rec = read_record(&mut stream);
    let sh_encoded = sh_rec.payload.clone();
    let server_key = extract_key_share_from_sh(&sh_encoded).expect("parse SH key_share");
    println!("  ServerHello received, key_share: {}..", hex_prefix(&server_key, 8));

    let server_pub = PublicKey::from(server_key);
    let shared = client_eph.diffie_hellman(&server_pub);
    let mut ecdhe = [0u8; 32];
    ecdhe.copy_from_slice(shared.as_bytes());
    println!("  ECDHE shared secret computed");

    // Derive handshake keys from HelloHash
    let mut tx = Transcript::new();
    tx.append(&ch_encoded);
    tx.append(&sh_encoded);
    let hello_hash = tx.snapshot();
    let (hs, c_hs_key, c_hs_iv, s_hs_key, s_hs_iv, c_hs_fk, s_hs_fk) =
        derive_hs_keys(&ecdhe, &hello_hash);
    let mut cwrite = RecordProtection::new(c_hs_key, c_hs_iv);
    let mut swrite = RecordProtection::new(s_hs_key, s_hs_iv);
    println!("  Handshake keys derived from HelloHash");

    // Read encrypted handshake flight
    let hs_rec = read_record(&mut stream);
    let (hs_plain, hs_inner) = swrite.decrypt(&hs_rec.payload, CT_APP_DATA).expect("decrypt HS flight");
    assert_eq!(hs_inner, CT_HANDSHAKE);

    let msgs = parse_hs_messages(&hs_plain);
    println!("  Received {} encrypted handshake messages", msgs.len());

    assert_eq!(msgs[0].0, HS_ENCRYPTED_EXTENSIONS);
    tx.append(&encode_hs(HS_ENCRYPTED_EXTENSIONS, &msgs[0].1));

    assert_eq!(msgs[1].0, HS_CERTIFICATE);
    tx.append(&encode_hs(HS_CERTIFICATE, &msgs[1].1));

    assert_eq!(msgs[2].0, HS_CERTIFICATE_VERIFY);
    let cv_transcript = tx.snapshot();
    tx.append(&encode_hs(HS_CERTIFICATE_VERIFY, &msgs[2].1));

    // Verify CertificateVerify
    let cv_body = &msgs[2].1;
    let scheme = u16::from_be_bytes([cv_body[0], cv_body[1]]);
    assert_eq!(scheme, 0x0807, "expected Ed25519");
    let sig_len = u16::from_be_bytes([cv_body[2], cv_body[3]]) as usize;
    let sig_bytes = &cv_body[4..4 + sig_len];
    let sig_arr: [u8; 64] = sig_bytes.try_into().expect("Ed25519 signature must be 64 bytes");
    let signature = Signature::from_bytes(&sig_arr).expect("valid signature");

    // Recover server public key from certificate
    let cert_body = &msgs[1].1;
    let cert_list_len = ((cert_body[0] as usize) << 16) | ((cert_body[1] as usize) << 8) | cert_body[2] as usize;
    let cert_entry = &cert_body[3..3 + cert_list_len];
    let cert_data_len = ((cert_entry[0] as usize) << 16) | ((cert_entry[1] as usize) << 8) | cert_entry[2] as usize;
    let cert_pub_bytes = &cert_entry[3..3 + cert_data_len];
    let server_verifying = VerifyingKey::from_bytes(cert_pub_bytes.try_into().unwrap()).expect("parse pubkey");

    let context = b"TLS 1.3, server CertificateVerify\0";
    let mut signed_data = Vec::new();
    signed_data.extend_from_slice(context);
    signed_data.extend_from_slice(&cv_transcript);
    let cv_ok = server_verifying.verify(&signed_data, &signature).is_ok();
    print_verdict(cv_ok, "CertificateVerify signature verified");
    if !cv_ok { return; }

    // Verify server Finished
    assert_eq!(msgs[3].0, HS_FINISHED);
    let sf_body = &msgs[3].1;
    let sf_transcript = tx.snapshot();
    let expected_sf = compute_finished(&s_hs_fk, &sf_transcript);
    let sf_ok = sf_body == expected_sf.as_slice();
    print_verdict(sf_ok, "Server Finished verified");
    if !sf_ok { return; }

    tx.append(&encode_hs(HS_FINISHED, sf_body));
    println!("  Handshake messages verified");

    // Build and send client Finished
    let cf_transcript = tx.snapshot();
    let cf_verify = compute_finished(&c_hs_fk, &cf_transcript);
    let cf = encode_hs(HS_FINISHED, &cf_verify);
    let cf_encrypted = cwrite.encrypt(&cf, CT_HANDSHAKE);
    stream.write_all(&cf_encrypted).unwrap();
    let cf_encoded = encode_hs(HS_FINISHED, &cf_verify);
    tx.append(&cf_encoded);
    println!("  Client Finished sent");

    // Derive app keys
    let handshake_hash = tx.snapshot();
    let (c_ak, c_ai, s_ak, s_ai) = derive_app_keys(&hs, &handshake_hash);
    let mut capp = RecordProtection::new(c_ak, c_ai);
    let mut sapp = RecordProtection::new(s_ak, s_ai);
    println!("  Application keys derived");

    // Send encrypted application data
    let request = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\nHello TLS 1.3!";
    let encrypted_req = capp.encrypt(request, CT_APP_DATA);
    println!("  Plaintext request ({} bytes): {}", request.len(), String::from_utf8_lossy(request));
    println!("  Encrypted request: {}..", hex_prefix(&encrypted_req, 16));
    stream.write_all(&encrypted_req).unwrap();

    // Read encrypted response
    let resp_rec = read_record(&mut stream);
    let (resp_data, _) = sapp.decrypt(&resp_rec.payload, CT_APP_DATA).expect("decrypt response");
    let echo_ok = resp_data == request;
    print_verdict(echo_ok, "Echo verified (response matches request)");
    if echo_ok {
        println!("  Decrypted response: {}", String::from_utf8_lossy(&resp_data));
    }

    // Read close_notify
    let close_rec = read_record(&mut stream);
    let (alert, alert_ct) = sapp.decrypt(&close_rec.payload, CT_APP_DATA).expect("decrypt alert");
    assert_eq!(alert_ct, CT_ALERT);
    println!("  close_notify received (level={}, desc={})", alert[0], alert[1]);

    println!("\n── TLS 1.3 handshake complete ✓ ──");
}

// ── Part 12: Main ────────────────────────────────────────────

fn main() {
    println!("{}", "=".repeat(68));
    println!("  Build a Toy TLS 1.3 Client");
    println!("  Phase 12 — Cryptography & Security, Lesson 15");
    println!("{}", "=".repeat(68));

    let server = thread::spawn(|| {
        run_server();
    });

    thread::sleep(Duration::from_millis(200));
    run_client();

    server.join().expect("server join");

    println!("\n  All handshake steps completed successfully.");
    println!("  The toy TLS 1.3 client negotiated keys, verified");
    println!("  the server's identity, and exchanged encrypted data.");
}
