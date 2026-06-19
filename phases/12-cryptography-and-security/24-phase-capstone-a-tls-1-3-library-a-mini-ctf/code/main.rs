//! Phase 12 Capstone — TLS 1.3 Library
//!
//! Modular Rust implementation of the core TLS 1.3 protocol.
//! Integrates: X25519 ECDHE, HKDF-SHA256 key schedule, AES-128-GCM,
//! Ed25519 signatures, and a full handshake state machine.
//!
//! Run with: cargo run

use aes_gcm::aead::{Aead, AeadCore, KeyInit};
use aes_gcm::{Aes128Gcm, Key, Nonce};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::{Digest, Sha256};
use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret};

type HmacSha256 = Hmac<Sha256>;

// ── Record Layer ──────────────────────────────────────────────────────────────

mod record {
    pub const CONTENT_CHANGE_CIPHER_SPEC: u8 = 20;
    pub const CONTENT_ALERT: u8 = 21;
    pub const CONTENT_HANDSHAKE: u8 = 22;
    pub const CONTENT_APPLICATION_DATA: u8 = 23;

    pub struct TlsRecord {
        pub content_type: u8,
        pub payload: Vec<u8>,
    }

    pub fn encode_record(content_type: u8, payload: &[u8]) -> Vec<u8> {
        let mut record = Vec::with_capacity(5 + payload.len());
        record.push(content_type);
        record.push(0x03);
        record.push(0x03);
        record.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        record.extend_from_slice(payload);
        record
    }

    pub fn decode_record(data: &[u8]) -> Option<TlsRecord> {
        if data.len() < 5 {
            return None;
        }
        let content_type = data[0];
        let length = u16::from_be_bytes([data[3], data[4]]) as usize;
        if data.len() < 5 + length {
            return None;
        }
        Some(TlsRecord {
            content_type,
            payload: data[5..5 + length].to_vec(),
        })
    }
}

// ── Transcript Hash ──────────────────────────────────────────────────────────

mod transcript {
    use sha2::{Digest, Sha256};

    pub struct Transcript {
        hasher: Sha256,
    }

    impl Transcript {
        pub fn new() -> Self {
            Transcript {
                hasher: Sha256::new(),
            }
        }

        pub fn update(&mut self, data: &[u8]) {
            self.hasher.update(data);
        }

        pub fn current_hash(&self) -> [u8; 32] {
            let mut hash = [0u8; 32];
            let result = self.hasher.clone().finalize();
            hash.copy_from_slice(&result);
            hash
        }

        pub fn update_and_hash(&mut self, data: &[u8]) -> [u8; 32] {
            self.update(data);
            self.current_hash()
        }
    }
}

// ── Key Schedule (HKDF-SHA256) ───────────────────────────────────────────────

mod key_schedule {
    use super::*;

    const LABEL_DERIVED: &[u8] = b"tls13 derived";
    const LABEL_C_HS_TRAFFIC: &[u8] = b"tls13 c hs traffic";
    const LABEL_S_HS_TRAFFIC: &[u8] = b"tls13 s hs traffic";
    const LABEL_C_AP_TRAFFIC: &[u8] = b"tls13 c ap traffic";
    const LABEL_S_AP_TRAFFIC: &[u8] = b"tls13 s ap traffic";
    const LABEL_KEY: &[u8] = b"tls13 key";
    const LABEL_IV: &[u8] = b"tls13 iv";
    const LABEL_FINISHED: &[u8] = b"tls13 finished";

    fn hkdf_extract(salt: &[u8], ikm: &[u8]) -> [u8; 32] {
        let mut mac = HmacSha256::new_from_slice(if salt.is_empty() {
            &[0u8; 32]
        } else {
            salt
        })
        .unwrap();
        mac.update(ikm);
        let result = mac.finalize().into_bytes();
        let mut prk = [0u8; 32];
        prk.copy_from_slice(&result);
        prk
    }

    fn derive_secret(prk: &[u8], label: &[u8], context: &[u8]) -> [u8; 32] {
        let hkdf = Hkdf::<Sha256>::from_prk(prk).unwrap();
        let label_len = label.len() as u8;
        let context_len = context.len() as u8;
        let mut info = Vec::new();
        info.extend_from_slice(&32u16.to_be_bytes());
        info.push(label_len);
        info.extend_from_slice(label);
        info.push(context_len);
        info.extend_from_slice(context);
        let mut okm = [0u8; 32];
        hkdf.expand(&info, &mut okm).unwrap();
        okm
    }

    fn derive_derive_secret(prk: &[u8]) -> [u8; 32] {
        derive_secret(prk, LABEL_DERIVED, b"")
    }

    pub fn derive_early_secret(psk: &[u8]) -> [u8; 32] {
        hkdf_extract(&[0u8; 32], psk)
    }

    pub fn derive_handshake_secret(early_secret: &[u8], shared_secret: &[u8]) -> [u8; 32] {
        let derived = derive_derive_secret(early_secret);
        hkdf_extract(&derived, shared_secret)
    }

    pub fn derive_master_secret(handshake_secret: &[u8]) -> [u8; 32] {
        let derived = derive_derive_secret(handshake_secret);
        hkdf_extract(&derived, b"")
    }

    pub fn derive_client_handshake_traffic_secret(
        handshake_secret: &[u8],
        transcript_hash: &[u8],
    ) -> [u8; 32] {
        derive_secret(handshake_secret, LABEL_C_HS_TRAFFIC, transcript_hash)
    }

    pub fn derive_server_handshake_traffic_secret(
        handshake_secret: &[u8],
        transcript_hash: &[u8],
    ) -> [u8; 32] {
        derive_secret(handshake_secret, LABEL_S_HS_TRAFFIC, transcript_hash)
    }

    pub fn derive_client_application_traffic_secret(
        master_secret: &[u8],
        transcript_hash: &[u8],
    ) -> [u8; 32] {
        derive_secret(master_secret, LABEL_C_AP_TRAFFIC, transcript_hash)
    }

    pub fn derive_server_application_traffic_secret(
        master_secret: &[u8],
        transcript_hash: &[u8],
    ) -> [u8; 32] {
        derive_secret(master_secret, LABEL_S_AP_TRAFFIC, transcript_hash)
    }

    pub fn derive_traffic_keys(traffic_secret: &[u8]) -> ([u8; 16], [u8; 12]) {
        let key_bytes = derive_secret(traffic_secret, LABEL_KEY, b"");
        let iv_bytes = derive_secret(traffic_secret, LABEL_IV, b"");
        let mut key = [0u8; 16];
        let mut iv = [0u8; 12];
        key.copy_from_slice(&key_bytes[..16]);
        iv.copy_from_slice(&iv_bytes[..12]);
        (key, iv)
    }

    pub fn derive_verify_data(traffic_secret: &[u8], transcript_hash: &[u8]) -> Vec<u8> {
        let finished_key = derive_secret(traffic_secret, LABEL_FINISHED, b"");
        let mut mac = HmacSha256::new_from_slice(&finished_key[..]).unwrap();
        mac.update(transcript_hash);
        mac.finalize().into_bytes().to_vec()
    }

    pub fn compute_shared_secret(
        secret: EphemeralSecret,
        peer_public: &PublicKey,
    ) -> [u8; 32] {
        let shared: SharedSecret = secret.diffie_hellman(peer_public);
        let mut result = [0u8; 32];
        result.copy_from_slice(shared.as_bytes());
        result
    }
}

// ── Cipher Operations (AES-128-GCM) ─────────────────────────────────────────

mod cipher {
    use super::*;
    use aes_gcm::aead::AeadMutInPlace;
    use aes_gcm::Aes128Gcm;
    use aes_gcm::{Key, Nonce};

    fn build_nonce(iv: &[u8; 12], seq: u64) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        nonce[4..].copy_from_slice(&seq.to_be_bytes());
        for i in 0..12 {
            nonce[i] ^= iv[i];
        }
        nonce
    }

    fn build_aad(content_type: u8, payload_len: usize) -> Vec<u8> {
        let mut aad = Vec::with_capacity(5);
        aad.push(content_type);
        aad.push(0x03);
        aad.push(0x03);
        aad.extend_from_slice(&(payload_len as u16).to_be_bytes());
        aad
    }

    pub fn encrypt(
        key: &[u8; 16],
        iv: &[u8; 12],
        seq: u64,
        content_type: u8,
        plaintext: &[u8],
    ) -> Vec<u8> {
        let aes_key = Key::<Aes128Gcm>::from_slice(key);
        let mut cipher = Aes128Gcm::new(aes_key);
        let nonce_bytes = build_nonce(iv, seq);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let aad = build_aad(content_type, plaintext.len());
        let mut buffer = plaintext.to_vec();
        cipher
            .encrypt_in_place(nonce, &aad, &mut buffer)
            .unwrap();
        buffer
    }

    pub fn decrypt(
        key: &[u8; 16],
        iv: &[u8; 12],
        seq: u64,
        content_type: u8,
        ciphertext: &[u8],
    ) -> Option<Vec<u8>> {
        let aes_key = Key::<Aes128Gcm>::from_slice(key);
        let mut cipher = Aes128Gcm::new(aes_key);
        let nonce_bytes = build_nonce(iv, seq);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let payload_len = ciphertext.len().saturating_sub(16);
        let aad = build_aad(content_type, payload_len);
        let mut buffer = ciphertext.to_vec();
        cipher.decrypt_in_place(nonce, &aad, &mut buffer).ok()?;
        Some(buffer)
    }
}

// ── Handshake Messages ───────────────────────────────────────────────────────

mod handshake {
    use super::*;

    const HANDSHAKE_CLIENT_HELLO: u8 = 1;
    const HANDSHAKE_SERVER_HELLO: u8 = 2;
    const HANDSHAKE_ENCRYPTED_EXTENSIONS: u8 = 8;
    const HANDSHAKE_CERTIFICATE: u8 = 11;
    const HANDSHAKE_CERTIFICATE_VERIFY: u8 = 15;
    const HANDSHAKE_FINISHED: u8 = 20;

    fn encode_handshake_message(msg_type: u8, body: &[u8]) -> Vec<u8> {
        let mut msg = Vec::with_capacity(4 + body.len());
        msg.push(msg_type);
        msg.extend_from_slice(&(body.len() as u24).to_be_bytes());
        msg.extend_from_slice(body);
        msg
    }

    struct u24([u8; 3]);

    impl u24 {
        fn to_be_bytes(self) -> [u8; 3] {
            self.0
        }
    }

    impl From<usize> for u24 {
        fn from(n: usize) -> Self {
            let n = n as u32;
            u24([(n >> 16) as u8, (n >> 8) as u8, n as u8])
        }
    }

    pub fn build_client_hello(key_share: &[u8; 32]) -> Vec<u8> {
        let mut body = Vec::new();
        let mut random = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut random);
        body.extend_from_slice(&random);
        body.extend_from_slice(&0u16.to_be_bytes());
        body.push(0x02);
        body.extend_from_slice(&[0x13, 0x01]);
        body.extend_from_slice(&[0x01, 0x00]);
        body.extend_from_slice(&[0x00, 0x2b, 0x00, 0x02, 0x03, 0x04]);
        body.extend_from_slice(&[0x00, 0x0d, 0x00, 0x14, 0x00, 0x12]);
        body.push(0x04);
        body.extend_from_slice(&[0x00, 0x1d, 0x00, 0x20]);
        body.extend_from_slice(key_share);
        let mut ext_len = body.len() as u16;
        let mut msg = Vec::new();
        msg.push(HANDSHAKE_CLIENT_HELLO);
        msg.extend_from_slice(&u24::from(body.len() + 2).to_be_bytes());
        msg.extend_from_slice(&(body.len() as u16).to_be_bytes());
        msg.extend_from_slice(&body);
        record::encode_record(record::CONTENT_HANDSHAKE, &msg)
    }

    pub fn parse_client_hello(data: &[u8]) -> ([u8; 32], [u8; 32]) {
        let mut random = [0u8; 32];
        random.copy_from_slice(&data[4..36]);
        if let Some(pos) = data.windows(4).position(|w| w == [0x00, 0x1d, 0x00, 0x20]) {
            let start = pos + 4;
            let mut key_share = [0u8; 32];
            key_share.copy_from_slice(&data[start..start + 32]);
            (random, key_share)
        } else {
            (random, [0u8; 32])
        }
    }

    pub fn build_server_hello(key_share: &[u8; 32]) -> Vec<u8> {
        let mut body = Vec::new();
        let mut random = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut random);
        body.extend_from_slice(&random);
        body.extend_from_slice(&[0x13, 0x01]);
        body.extend_from_slice(&[0x00, 0x2b, 0x00, 0x02, 0x03, 0x04]);
        body.push(0x04);
        body.extend_from_slice(&[0x00, 0x1d, 0x00, 0x20]);
        body.extend_from_slice(key_share);
        let mut msg = Vec::new();
        msg.push(HANDSHAKE_SERVER_HELLO);
        msg.extend_from_slice(&u24::from(body.len()).to_be_bytes());
        msg.extend_from_slice(&body);
        record::encode_record(record::CONTENT_HANDSHAKE, &msg)
    }

    pub fn parse_server_hello(data: &[u8]) -> ([u8; 32], [u8; 32]) {
        let mut random = [0u8; 32];
        random.copy_from_slice(&data[4..36]);
        if let Some(pos) = data.windows(4).position(|w| w == [0x00, 0x1d, 0x00, 0x20]) {
            let start = pos + 4;
            let mut key_share = [0u8; 32];
            key_share.copy_from_slice(&data[start..start + 32]);
            (random, key_share)
        } else {
            (random, [0u8; 32])
        }
    }

    pub fn build_encrypted_extensions() -> Vec<u8> {
        let body = vec![0x00, 0x00];
        let msg = encode_handshake_message(HANDSHAKE_ENCRYPTED_EXTENSIONS, &body);
        record::encode_record(record::CONTENT_HANDSHAKE, &msg)
    }

    pub fn build_certificate(signing_key: &VerifyingKey) -> Vec<u8> {
        let cert_pk = signing_key.to_bytes();
        let mut body = Vec::new();
        body.extend_from_slice(&[0x00, 0x00, 0x00]);
        let cert_body_len = 3 + 1 + 3 + cert_pk.len();
        body.extend_from_slice(&u24::from(cert_body_len).to_be_bytes());
        body.push(0x00);
        body.extend_from_slice(&u24::from(cert_pk.len() + 4).to_be_bytes());
        body.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        body.extend_from_slice(&cert_pk);
        let msg = encode_handshake_message(HANDSHAKE_CERTIFICATE, &body);
        record::encode_record(record::CONTENT_HANDSHAKE, &msg)
    }

    pub fn build_certificate_verify(
        signing_key: &SigningKey,
        transcript_hash: &[u8],
    ) -> Vec<u8> {
        let mut sig_data = Vec::new();
        sig_data.extend_from_slice(b" ");
        sig_data.extend_from_slice(b"TLS 1.3, server CertificateVerify");
        sig_data.extend_from_slice(&[0x00]);
        sig_data.extend_from_slice(transcript_hash);

        let signature: Signature = signing_key.sign(&sig_data);

        let mut body = Vec::new();
        body.extend_from_slice(&[0x08, 0x04, 0x00, 0x02]);
        body.extend_from_slice(&[0x08, 0x04]);
        let sig_bytes = signature.to_bytes();
        body.extend_from_slice(&u24::from(sig_bytes.len()).to_be_bytes());
        body.extend_from_slice(&sig_bytes);

        let msg = encode_handshake_message(HANDSHAKE_CERTIFICATE_VERIFY, &body);
        record::encode_record(record::CONTENT_HANDSHAKE, &msg)
    }

    pub fn parse_certificate_verify(
        verifying_key: &VerifyingKey,
        data: &[u8],
        transcript_hash: &[u8],
    ) -> bool {
        let hs_body = if data.len() > 4 { &data[4..] } else { return false };
        let mut pos = 4;
        let sig_scheme = u16::from_be_bytes([hs_body.get(pos).copied().unwrap_or(0), hs_body.get(pos + 1).copied().unwrap_or(0)]);
        pos += 2;
        if sig_scheme != 0x0804 {
            return false;
        }
        let sig_len = ((hs_body[pos] as usize) << 16) | (hs_body[pos + 1] as usize) << 8 | hs_body[pos + 2] as usize;
        pos += 3;
        if pos + sig_len > hs_body.len() {
            return false;
        }
        let sig_bytes = &hs_body[pos..pos + sig_len];
        let signature = match Signature::from_slice(sig_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let mut sig_data = Vec::new();
        sig_data.extend_from_slice(b" ");
        sig_data.extend_from_slice(b"TLS 1.3, server CertificateVerify");
        sig_data.extend_from_slice(&[0x00]);
        sig_data.extend_from_slice(transcript_hash);

        verifying_key.verify(&sig_data, &signature).is_ok()
    }

    pub fn build_finished(verify_data: &[u8]) -> Vec<u8> {
        let msg = encode_handshake_message(HANDSHAKE_FINISHED, verify_data);
        record::encode_record(record::CONTENT_HANDSHAKE, &msg)
    }

    pub fn parse_finished(data: &[u8]) -> Vec<u8> {
        data[4..].to_vec()
    }

    pub fn encode_handshake_for_transcript(data: &[u8]) -> Vec<u8> {
        if data.len() >= 5 {
            data[5..].to_vec()
        } else {
            data.to_vec()
        }
    }
}

// ── TLS Connection ───────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum TlsState {
    Handshake,
    Connected,
    Closed,
}

pub struct TlsConnection {
    state: TlsState,
    is_server: bool,
    client_hs_key: [u8; 16],
    client_hs_iv: [u8; 12],
    server_hs_key: [u8; 16],
    server_hs_iv: [u8; 12],
    client_ap_key: [u8; 16],
    client_ap_iv: [u8; 12],
    server_ap_key: [u8; 16],
    server_ap_iv: [u8; 12],
    shared_secret: [u8; 32],
    client_seq: u64,
    server_seq: u64,
    transcript: transcript::Transcript,
    signing_key: Option<SigningKey>,
    verifying_key: Option<VerifyingKey>,
}

impl TlsConnection {
    pub fn new(is_server: bool) -> Self {
        let zero16 = [0u8; 16];
        let zero12 = [0u8; 12];
        let zero32 = [0u8; 32];
        TlsConnection {
            state: TlsState::Handshake,
            is_server,
            client_hs_key: zero16,
            client_hs_iv: zero12,
            server_hs_key: zero16,
            server_hs_iv: zero12,
            client_ap_key: zero16,
            client_ap_iv: zero12,
            server_ap_key: zero16,
            server_ap_iv: zero12,
            shared_secret: zero32,
            client_seq: 0,
            server_seq: 0,
            transcript: transcript::Transcript::new(),
            signing_key: None,
            verifying_key: None,
        }
    }

    pub fn connect(
        client_conn: &mut TlsConnection,
        server_conn: &mut TlsConnection,
    ) -> Result<(), String> {
        if client_conn.is_server || !server_conn.is_server {
            return Err("connect(): first arg must be client, second must be server".into());
        }

        println!("\n  Step 1: Client generates X25519 keypair");
        let client_secret = EphemeralSecret::random_from_rng(rand::thread_rng());
        let client_public = PublicKey::from(&client_secret);

        println!("  Step 2: Server generates X25519 keypair");
        let server_secret = EphemeralSecret::random_from_rng(rand::thread_rng());
        let server_public = PublicKey::from(&server_secret);

        println!("  Step 3: Server Ed25519 key generation");
        let mut seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        server_conn.signing_key = Some(signing_key);
        server_conn.verifying_key = Some(verifying_key);

        println!("  Step 4: ClientHello exchange");
        let client_hello = handshake::build_client_hello(client_public.as_bytes());
        let ch_body = handshake::encode_handshake_for_transcript(&client_hello);
        let ch_hash = client_conn.transcript.update_and_hash(&ch_body);
        server_conn.transcript.update(&ch_body);

        let (_, server_pk_bytes) = handshake::parse_client_hello(
            &record::decode_record(&client_hello).unwrap().payload,
        );
        let server_pk = PublicKey::from(server_pk_bytes);

        println!("  Step 5: ServerHello exchange");
        let server_hello = handshake::build_server_hello(server_public.as_bytes());
        let sh_body = handshake::encode_handshake_for_transcript(&server_hello);
        client_conn.transcript.update(&sh_body);
        let sh_hash = server_conn.transcript.update_and_hash(&sh_body);

        let (_, client_pk_bytes) = handshake::parse_server_hello(
            &record::decode_record(&server_hello).unwrap().payload,
        );
        let client_pk = PublicKey::from(client_pk_bytes);

        println!("  Step 6: Shared secret computation (ECDHE)");
        let client_shared = key_schedule::compute_shared_secret(client_secret, &server_pk);
        let server_shared = key_schedule::compute_shared_secret(server_secret, &client_pk);
        assert_eq!(
            hex::encode(client_shared),
            hex::encode(server_shared),
            "Shared secrets must match"
        );
        client_conn.shared_secret = client_shared;
        server_conn.shared_secret = server_shared;
        println!("      Shared secret: {}", hex::encode(&client_shared[..8]));

        println!("  Step 7: Handshake key derivation");
        let early_secret = key_schedule::derive_early_secret(&[0u8; 32]);
        let hs_secret_client =
            key_schedule::derive_handshake_secret(&early_secret, &client_conn.shared_secret);
        let hs_secret_server =
            key_schedule::derive_handshake_secret(&early_secret, &server_conn.shared_secret);

        let c_hs_traffic =
            key_schedule::derive_client_handshake_traffic_secret(&hs_secret_client, &ch_hash);
        let s_hs_traffic =
            key_schedule::derive_server_handshake_traffic_secret(&hs_secret_server, &sh_hash);

        let (c_hs_key, c_hs_iv) = key_schedule::derive_traffic_keys(&c_hs_traffic);
        let (s_hs_key, s_hs_iv) = key_schedule::derive_traffic_keys(&s_hs_traffic);
        client_conn.client_hs_key = c_hs_key;
        client_conn.client_hs_iv = c_hs_iv;
        client_conn.server_hs_key = s_hs_key;
        client_conn.server_hs_iv = s_hs_iv;
        server_conn.client_hs_key = c_hs_key;
        server_conn.client_hs_iv = c_hs_iv;
        server_conn.server_hs_key = s_hs_key;
        server_conn.server_hs_iv = s_hs_iv;

        let hello_hash = client_conn.transcript.current_hash();

        println!("  Step 8: Server sends EncryptedExtensions, Certificate, CertificateVerify");
        let ee = handshake::build_encrypted_extensions();
        let ee_body = handshake::encode_handshake_for_transcript(&ee);
        server_conn.transcript.update(&ee_body);
        client_conn.transcript.update(&ee_body);

        let cert = handshake::build_certificate(server_conn.verifying_key.as_ref().unwrap());
        let cert_body = handshake::encode_handshake_for_transcript(&cert);
        server_conn.transcript.update(&cert_body);
        client_conn.transcript.update(&cert_body);

        let s_cert_hash = server_conn.transcript.current_hash();
        let cv = handshake::build_certificate_verify(
            server_conn.signing_key.as_ref().unwrap(),
            &s_cert_hash,
        );
        let cv_body = handshake::encode_handshake_for_transcript(&cv);
        let s_cv_hash = server_conn.transcript.update_and_hash(&cv_body);
        client_conn.transcript.update(&cv_body);
        let c_cv_hash = client_conn.transcript.current_hash();

        println!("  Step 9: Client verifies CertificateVerify signature");
        let cv_record = record::decode_record(&cv).unwrap();
        let verified = handshake::parse_certificate_verify(
            server_conn.verifying_key.as_ref().unwrap(),
            &cv_record.payload,
            &c_cv_hash,
        );
        assert!(verified, "CertificateVerify signature must verify");
        println!("      ✓ Ed25519 signature verified");

        println!("  Step 10: Server Finished and Client Finished");
        let s_hs_traffic_finish =
            key_schedule::derive_server_handshake_traffic_secret(&hs_secret_server, &hello_hash);
        let s_verify_data =
            key_schedule::derive_verify_data(&s_hs_traffic_finish, &s_cv_hash);
        let _sf = handshake::build_finished(&s_verify_data);

        let c_hs_traffic_finish =
            key_schedule::derive_client_handshake_traffic_secret(&hs_secret_client, &hello_hash);
        let c_verify_data =
            key_schedule::derive_verify_data(&c_hs_traffic_finish, &c_cv_hash);
        let cf = handshake::build_finished(&c_verify_data);
        let cf_body = handshake::encode_handshake_for_transcript(&cf);
        let _c_finish_hash = client_conn.transcript.update_and_hash(&cf_body);
        server_conn.transcript.update(&cf_body);
        let s_finish_hash = server_conn.transcript.current_hash();

        let s_verify_check = key_schedule::derive_verify_data(
            &key_schedule::derive_server_handshake_traffic_secret(&hs_secret_server, &hello_hash),
            &s_finish_hash,
        );
        let client_finished_data = handshake::parse_finished(
            &record::decode_record(&cf).unwrap().payload,
        );
        assert_eq!(
            hex::encode(&client_finished_data),
            hex::encode(&s_verify_check),
            "Server must verify Client Finished"
        );
        println!("      ✓ Finished messages verified");

        println!("  Step 11: Application traffic key derivation");
        let master_secret_client =
            key_schedule::derive_master_secret(&hs_secret_client);
        let master_secret_server =
            key_schedule::derive_master_secret(&hs_secret_server);

        let final_hash = client_conn.transcript.current_hash();
        let c_ap_traffic =
            key_schedule::derive_client_application_traffic_secret(
                &master_secret_client,
                &final_hash,
            );
        let s_ap_traffic =
            key_schedule::derive_server_application_traffic_secret(
                &master_secret_server,
                &final_hash,
            );

        let (c_ap_key, c_ap_iv) = key_schedule::derive_traffic_keys(&c_ap_traffic);
        let (s_ap_key, s_ap_iv) = key_schedule::derive_traffic_keys(&s_ap_traffic);
        client_conn.client_ap_key = c_ap_key;
        client_conn.client_ap_iv = c_ap_iv;
        client_conn.server_ap_key = s_ap_key;
        client_conn.server_ap_iv = s_ap_iv;
        server_conn.client_ap_key = c_ap_key;
        server_conn.client_ap_iv = c_ap_iv;
        server_conn.server_ap_key = s_ap_key;
        server_conn.server_ap_iv = s_ap_iv;

        client_conn.state = TlsState::Connected;
        server_conn.state = TlsState::Connected;

        println!("  ✓ TLS 1.3 handshake complete");
        Ok(())
    }

    pub fn send_data(&mut self, data: &[u8]) -> Vec<u8> {
        assert_eq!(self.state, TlsState::Connected);
        let (key, iv) = if self.is_server {
            (self.server_ap_key, self.server_ap_iv)
        } else {
            (self.client_ap_key, self.client_ap_iv)
        };
        let seq = if self.is_server {
            self.server_seq
        } else {
            self.client_seq
        };

        let encrypted = cipher::encrypt(&key, &iv, seq, record::CONTENT_APPLICATION_DATA, data);
        let record = record::encode_record(record::CONTENT_APPLICATION_DATA, &encrypted);

        if self.is_server {
            self.server_seq += 1;
        } else {
            self.client_seq += 1;
        }
        record
    }

    pub fn receive_data(&mut self, record_bytes: &[u8]) -> Vec<u8> {
        assert_eq!(self.state, TlsState::Connected);
        let rec = record::decode_record(record_bytes).unwrap();
        let (key, iv) = if self.is_server {
            (self.client_ap_key, self.client_ap_iv)
        } else {
            (self.server_ap_key, self.server_ap_iv)
        };
        let seq = if self.is_server {
            self.server_seq
        } else {
            self.client_seq
        };

        let plaintext = cipher::decrypt(
            &key,
            &iv,
            seq,
            rec.content_type,
            &rec.payload,
        )
        .expect("Decryption failed");

        if self.is_server {
            self.server_seq += 1;
        } else {
            self.client_seq += 1;
        }
        plaintext
    }

    pub fn close(&mut self) -> Vec<u8> {
        self.state = TlsState::Closed;
        let alert = vec![0x01, 0x00];
        let (key, iv) = if self.is_server {
            (self.server_ap_key, self.server_ap_iv)
        } else {
            (self.client_ap_key, self.client_ap_iv)
        };
        let seq = if self.is_server {
            self.server_seq
        } else {
            self.client_seq
        };
        let encrypted = cipher::encrypt(&key, &iv, seq, record::CONTENT_ALERT, &alert);
        record::encode_record(record::CONTENT_ALERT, &encrypted)
    }
}

// ── Main: Echo Server Demo ───────────────────────────────────────────────────

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║     Phase 12 Capstone — TLS 1.3 Library (Rust)         ║");
    println!("║     In-Process Handshake Simulation                    ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!("Integrating: X25519 ECDHE, HKDF-SHA256, AES-128-GCM, Ed25519");
    println!();

    let mut client = TlsConnection::new(false);
    let mut server = TlsConnection::new(true);

    TlsConnection::connect(&mut client, &mut server).unwrap();

    println!("\n  Step 12: Encrypted application data exchange");
    let message = b"Hello, TLS 1.3! This is encrypted application data.";
    println!("      Client sending: \"{}\"", String::from_utf8_lossy(message));

    let encrypted_record = client.send_data(message);
    println!(
        "      Encrypted record ({} bytes): {}...",
        encrypted_record.len(),
        hex::encode(&encrypted_record[..10])
    );

    let decrypted = server.receive_data(&encrypted_record);
    println!(
        "      Server received: \"{}\"",
        String::from_utf8_lossy(&decrypted)
    );

    let response = b"Echo from server: message received securely.";
    println!("      Server sending: \"{}\"", String::from_utf8_lossy(response));

    let server_encrypted = server.send_data(response);
    let client_decrypted = client.receive_data(&server_encrypted);
    println!(
        "      Client received: \"{}\"",
        String::from_utf8_lossy(&client_decrypted)
    );

    assert_eq!(decrypted, message, "Server must receive client's message exactly");
    assert_eq!(
        client_decrypted, response,
        "Client must receive server's response exactly"
    );

    println!("\n  Step 13: close_notify");
    client.close();
    server.close();

    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║     ✓ TLS 1.3 Echo Demo Complete                        ║");
    println!("║     ✓ Record layer encoding/decoding                    ║");
    println!("║     ✓ X25519 ECDHE key agreement                        ║");
    println!("║     ✓ HKDF-SHA256 key schedule                          ║");
    println!("║     ✓ AES-128-GCM AEAD encryption                       ║");
    println!("║     ✓ Ed25519 signature verification                    ║");
    println!("║     ✓ Full handshake + application data                 ║");
    println!("╚══════════════════════════════════════════════════════════╝");
}
