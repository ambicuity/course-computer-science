# TLS 1.3 — Handshake, Records, 0-RTT

> One round trip to secure the world — and zero when you have been here before.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 12 lessons 01–13
**Time:** ~90 minutes

## Learning Objectives

- Explain the TLS 1.3 handshake: 1-RTT for new connections, 0-RTT for resumed sessions with PSK.
- Implement the TLS record layer: content-type framing, AES-128-GCM encryption, sequence-number nonce construction.
- Derive the full key schedule using HKDF-Extract/Expand with labeled secrets as specified in RFC 8446 §7.1.
- Demonstrate ECDHE key exchange with X25519, showing how forward secrecy is guaranteed even if the server's long-term key is later compromised.

## The Problem

When you type `https://example.com` into a browser, two things must happen before any encrypted bytes flow: the client and server must agree on a secret key that nobody else knows, and the client must verify that it is talking to the real example.com, not an impostor. TLS 1.2 handled these steps in two round trips, with most handshake messages sent in cleartext — including the server's certificate. Any eavesdropper could see which site you were visiting and how large your certificate chain was. Worse, TLS 1.2's cipher suite negotiation was fatally flexible: servers still supported RSA key exchange (no forward secrecy), CBC mode ciphers (vulnerable to padding oracle attacks like POODLE), and compression (CRIME attack). Misconfiguration was the norm, not the exception.

TLS 1.3 solved all of this with a single, mandatory handshake design. It reduced the full handshake to one round trip by sending the server's certificate immediately after the ServerHello, and encrypted the entire handshake — including the certificate — so passive observers learn nothing. It removed every cipher suite that did not provide both encryption and authentication (AEAD), every key exchange that lacked forward secrecy, and every extension that had proved fragile in practice. The result is a protocol that is both faster and safer by default, with no knobs for administrators to turn the wrong way.

The phase capstone (lesson 24) ties together every cryptographic primitive from this phase into a working TLS 1.3 library plus a mini-CTF toolkit. The record layer, the HKDF key schedule, the ECDHE key exchange, and the handshake state machine you build in this lesson are the same components that the capstone wires together into a full server implementation. If this lesson feels abstract, the capstone will make it concrete: you will see your own code negotiate keys and encrypt traffic.

### Key concepts to cover:

- TLS 1.3 handshake: 1-RTT for full handshake, 0-RTT for resumed sessions
- Ephemeral key exchange (ECDHE) — forward secrecy
- Encrypted handshake messages (no cleartext certificates)
- The downgrade protection mechanism
- Removal of non-AEAD ciphers, static RSA, CBC, compression
- Record layer: content type, encrypted records, sequence numbers
- Key schedule: HKDF-Extract/Expand, the 7 derived secrets
- PSK and 0-RTT: tradeoff between latency and replay protection

## The Concept

The TLS 1.3 handshake is a carefully orchestrated dance of key exchanges, transcript hashes, and labeled derivations. At its core, the handshake establishes a shared secret through ECDHE, then stretches that secret through a cascade of HKDF operations to produce separate traffic keys for each direction of communication.

```
Client                                    Server
  |                                         |
  |--- ClientHello ----------------------->|
  |    (key_share: X25519 pub,             |
  |     supported_versions: 0x0304,        |
  |     signature_algorithms)              |
  |                                         |--- generate keypair
  |                                         |--- compute ECDHE shared secret
  |                                         |--- derive handshake traffic keys
  |<-- ServerHello ------------------------|
  |    (key_share: X25519 pub)             |
  |<-- EncryptedExtensions ----------------|  (encrypted with HS keys)
  |<-- Certificate ------------------------|  (encrypted)
  |<-- CertificateVerify ------------------|  (encrypted, signs transcript)
  |<-- Finished ---------------------------|  (encrypted, HMAC of transcript)
  |                                         |
  |--- compute ECDHE shared secret         |
  |--- derive handshake traffic keys       |
  |--- verify CertificateVerify            |
  |--- verify Finished                      |
  |                                         |
  |--- Finished -------------------------->|  (encrypted with HS keys)
  |                                         |--- verify Client Finished
  |                                         |--- derive app traffic keys
  |=== Application Data (encrypted) =======|
```

Each arrow represents a TLS record on the wire. The record layer wraps every message with a content type byte, a version field, a length, and (after the handshake is established) an AEAD-authenticated ciphertext. The key insight of TLS 1.3 is that encryption starts *during* the handshake: after the ServerHello, all remaining messages are encrypted with keys derived from the handshake secret.

The key schedule follows a strict cascade:

```
0 -> HKDF-Extract(salt=0, ikm=PSK) -> early_secret
                                           |
early_secret + ECDHE -> HKDF-Extract -> handshake_secret
                                           |
handshake_secret + "c hs traffic" + hash -> client_handshake_traffic_secret
handshake_secret + "s hs traffic" + hash -> server_handshake_traffic_secret
                                           |
handshake_secret -> HKDF-Extract(salt=0) -> master_secret
                                           |
master_secret + "c ap traffic" + hash -> client_app_traffic_secret
master_secret + "s ap traffic" + hash -> server_app_traffic_secret
```

Each `derive_secret(secret, label, transcript_hash)` call is an HKDF-Expand-Label operation that binds the derived key to the exact handshake transcript seen so far. If an attacker tries to replay or tamper with any message, the transcript hash changes and all subsequent derived keys diverge — the handshake fails.

## Build It

### Step 1: Record Layer

The TLS record layer is the framing protocol. Every TLS message is wrapped in a `TLSPlaintext` structure:

```
struct {
    ContentType type;        // 22=handshake, 23=app_data, 21=alert
    ProtocolVersion version; // 0x0301 for TLS 1.3 record layer
    uint16 length;           // payload length
    opaque fragment[length]; // the payload
} TLSPlaintext;
```

After the handshake produces traffic keys, records are encrypted using AES-128-GCM. The 12-byte nonce is constructed by XORing a 64-bit sequence number (per-direction, starting at 0) into the IV derived from the traffic secret:

```
nonce = IV XOR (sequence_number padded to 8 bytes, zero-extended to 12)
```

This ensures a unique nonce for every record without needing to transmit one. The additional authenticated data (AAD) is the content type byte plus the record version and length — this binds each ciphertext to its context so records cannot be replayed across different content types.

```rust
struct TLSRecord {
    content_type: u8,
    version: u16,
    payload: Vec<u8>,
}

impl TLSRecord {
    fn encode(&self) -> Vec<u8> {
        let mut buf = vec![self.content_type];
        buf.extend_from_slice(&self.version.to_be_bytes());
        buf.extend_from_slice(&(self.payload.len() as u16).to_be_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < 5 { return None; }
        let content_type = data[0];
        let version = u16::from_be_bytes([data[1], data[2]]);
        let len = u16::from_be_bytes([data[3], data[4]]) as usize;
        if data.len() < 5 + len { return None; }
        Some(TLSRecord {
            content_type,
            version,
            payload: data[5..5 + len].to_vec(),
        })
    }
}
```

### Step 2: Key Schedule (HKDF)

TLS 1.3's key schedule is built from two primitives: HKDF-Extract (HMAC-SHA256 where the salt keys the HMAC) and HKDF-Expand (an iterated HMAC that produces arbitrary-length output). These are combined in `derive_secret` which constructs an `HkdfLabel` struct containing a "tls13 " prefix, the purpose label, and the current transcript hash:

```rust
fn hkdf_extract(salt: &[u8], ikm: &[u8]) -> [u8; 32] {
    // HMAC-SHA256(salt, ikm)
    let mut mac = HmacSha256::new_from_slice(salt).unwrap();
    mac.update(ikm);
    let result = mac.finalize().into_bytes();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

fn hkdf_expand(prk: &[u8], info: &[u8], len: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(len);
    let mut t: Vec<u8> = Vec::new();
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
```

The `derive_secret` function wraps HKDF-Expand with the labeled construction:

```
HkdfLabel = (2-byte length) + (1-byte label_len) + "tls13 " + label
            + (1-byte ctx_len) + context
output = HKDF-Expand(secret, HkdfLabel, 32)
```

The seven secrets in the cascade are: `early_secret`, `handshake_secret`, `master_secret`, `client_handshake_traffic_secret`, `server_handshake_traffic_secret`, `client_application_traffic_secret`, `server_application_traffic_secret`. From the traffic secrets, we derive the AEAD key (first 16 bytes) and IV (next 12 bytes) by expanding with the labels "key" and "iv" respectively.

### Step 3: Handshake Messages

The handshake begins with the ClientHello. This message advertises the client's supported versions (TLS 1.3 = 0x0304), its X25519 public key share, the cipher suites it accepts (TLS_AES_128_GCM_SHA256), and its supported signature algorithms (Ed25519). The server responds with a ServerHello carrying its chosen version, cipher suite, and its own X25519 public key share.

Both sides now compute the ECDHE shared secret using X25519 Diffie-Hellman:

```rust
// Client side
let client_secret = EphemeralSecret::random_from_rng(OsRng);
let client_public = PublicKey::from(&client_secret);

// Server side
let server_secret = EphemeralSecret::random_from_rng(OsRng);
let server_public = PublicKey::from(&server_secret);

// Both compute the same shared secret
let client_shared = client_secret.diffie_hellman(&server_public);
let server_shared = server_secret.diffie_hellman(&client_public);
// client_shared.as_bytes() == server_shared.as_bytes()
```

The shared secret feeds into the key schedule to produce handshake traffic keys. The handshake transcript hash (SHA-256 of all messages sent so far) is used in `derive_secret` to bind the keys to the exact exchange. Because the transcript is hashed incrementally — starting with ClientHello, then ServerHello — any tampering is immediately detected.

After the handshake keys are derived, the server sends its EncryptedExtensions, Certificate, CertificateVerify, and Finished — all encrypted. The CertificateVerify contains an Ed25519 signature over the handshake transcript, proving the server owns the private key corresponding to its certificate. The Finished message is an HMAC over the transcript, confirming that both sides saw the same messages.

The client then sends its own Finished, and both sides derive application traffic keys from the master secret.

### Step 4: Putting It Together

The complete demo runs in a single process, simulating both the client and server roles. It:

1. Creates TLS record layer structures and demonstrates encoding/decoding round-trips.
2. Generates X25519 keypairs for both client and server and computes the shared ECDHE secret.
3. Constructs ClientHello and ServerHello messages and builds a running transcript hash.
4. Runs the full key schedule: early → handshake → master → application traffic keys.
5. Encrypts application data ("GET /index.html") with the client's application traffic key and decrypts it with the server's traffic key.
6. Verifies the round-trip and prints all intermediate values.

The output shows every derived key, every encrypted payload, and confirms that forward secrecy is achieved: the shared secret depends on ephemeral keys that exist only for the duration of this session.

## Use It

Production TLS 1.3 implementations — rustls, OpenSSL, BoringSSL — are substantially more complex than this toy implementation:

- **rustls** (`rustls/src/tls13/`) is the gold standard for a clean, safe implementation. It uses the same primitives (X25519, AES-128-GCM, HKDF-SHA256) but handles session resumption, PSK provisioning, key update, middlebox compatibility mode (the change_cipher_spec dummy message), and certificate chain validation with CRL/OCSP checking. The rustls `Tls13CipherSuite` struct directly maps to the cipher suite constants we used.
- **OpenSSL** (`ssl/statem/statem_clnt.c` and `ssl/statem/statem_srvr.c`) implements the handshake state machines as massive switch statements with hundreds of edge cases for compatibility with buggy peers. OpenSSL's implementation predates the finalized RFC and carries workarounds for draft versions.
- **BoringSSL** (Google's fork of OpenSSL, `ssl/handshake_client.cc` and `ssl/handshake_server.cc`) prioritizes simplicity. BoringSSL was the first production implementation of TLS 1.3 and its code reflects a cleaner architecture than OpenSSL's.

What your implementation is missing: session resumption via PSK (ticket-based and external), 0-RTT data, key update (re-keying within a session), middlebox compatibility (the useless change_cipher_spec that firewalls expect), certificate chain validation against trust stores, CRL/OCSP revocation checking, and the entire alert system. These are all important — but they extend the basic architecture, they do not change it. If you understand this lesson's handshake flow, you understand the core of TLS 1.3.

## Read the Source

- **RFC 8446** — The TLS 1.3 specification. Read §2 (protocol overview), §4 (handshake), §5 (record protocol), §7 (key schedule). The labeled derivation in §7.1 is what this lesson implements verbatim.
- **rustls `src/tls13/key_schedule.rs`** — The `KeySchedule` struct and `derive_traffic_key` function. Compare with our implementation — rustls handles PSK and early export, but the core HKDF-Extract/Expand loop is identical.
- **OpenSSL `ssl/t1_lib.c`** — The `tls_construct_client_hello` and `tls_construct_server_hello` functions. See how a production implementation handles extensions and compatibility.
- **Cloudflare's TLS 1.3 Implementation Guide** — A blog-post series that walks through implementing TLS 1.3 from scratch in Go. The handshake diagrams in Part 2 are excellent.
- **draft-ietf-tls-tls13-28** — The final draft before RFC publication. Reading the diff between draft 28 and RFC 8446 reveals what changed during the standardization process (mostly editorial).

## Ship It

The reusable artifact is a TLS 1.3 core library implementing the record layer, the HKDF key schedule, and ECDHE-based handshake key derivation. It lives in `outputs/` and directly feeds into the phase capstone (lesson 24): the capstone wires these components into a full TLS 1.3 server and pairs it with a mini-CTF toolkit where players exploit weaknesses in toy implementations.

## Exercises

1. **Easy** — Trace the key schedule: Starting with a known PSK of `0x00..00` and an ECDHE shared secret of `0x01..01`, compute the early secret, handshake secret, and master secret manually or with a script. Verify your output matches the demo output for the same inputs.
2. **Medium** — Add session resumption: After a handshake completes, have the server issue a PSK ticket (a randomly generated 32-byte value encrypted with a server-held key). On the next connection, the client sends the PSK in its ClientHello and the handshake completes without ECDHE, using the PSK to derive the early secret directly. Show that the handshake takes 0-RTT for application data.
3. **Hard** — Implement CertificateVerify verification: Use `ed25519-dalek` to have the server sign the transcript hash with its Ed25519 private key, and have the client verify the signature. Then implement a downgrade attack: trick the client into accepting a TLS 1.2 ServerHello by stripping the supported_versions extension. Show that the server's downgrade protection (a sentinel value in the ServerHello random) causes the client to reject the downgrade.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| 0-RTT | Zero round trip time resumption | Client sends encrypted data in the first flight using a previously established PSK; no round trip needed before data starts flowing |
| PSK | Pre-shared key | A symmetric key established during a previous handshake (or out of band) that allows resumption without a full ECDHE exchange |
| HKDF | HMAC-based key derivation function | A two-step key derivation (Extract then Expand) that turns a Diffie-Hellman shared secret into cryptographically strong traffic keys |
| ECDHE | Elliptic curve Diffie-Hellman ephemeral | A key exchange where each side generates a fresh keypair per session; the shared secret is never reused, guaranteeing forward secrecy |
| Handshake secret | The intermediate secret after ECDHE | Derived as HKDF-Extract(early_secret, ecdhe_shared_secret); from it, both sides derive their handshake traffic keys |
| Record layer | The TLS framing protocol | A thin wrapper: content type + version + length + payload; once keys are established, the payload is AEAD-encrypted with the content type in the AAD |
| Finished | The handshake integrity check | An HMAC over the full handshake transcript, proving both sides saw identical messages; the first message encrypted with the derived traffic keys |
| CertificateVerify | Proof of certificate ownership | An Ed25519 (or other) signature over the handshake transcript hash, proving the server controls the private key for its certificate |
| Forward secrecy | Compromising the long-term key does not reveal past sessions | Because each session uses ephemeral ECDHE keys that are discarded after the handshake, an attacker who later learns the server's private key cannot decrypt old recorded sessions |
| AEAD | Authenticated encryption with associated data | A cipher mode (like AES-128-GCM) that simultaneously provides confidentiality, integrity, and authenticity of the plaintext and unencrypted metadata |

## Further Reading

- RFC 8446 — "The Transport Layer Security (TLS) Protocol Version 1.3." The definitive specification. https://datatracker.ietf.org/doc/html/rfc8446
- "TLS 1.3 in Practice" by David Wong — A practical book that covers the protocol wire format, every extension, and common implementation mistakes.
- Cloudflare TLS 1.3 Blog Series — "An Overview of TLS 1.3" and "TLS 1.3 in Go" — accessible explanations with interactive handshake visualizations.
- "The Security of TLS 1.3" by Krawczyk, Paterson, and Wee — The formal security analysis proving the TLS 1.3 handshake provides session independence, forward secrecy, and key privacy. Read this to understand *why* the protocol is structured as it is.
- rustls source code — `src/tls13/` and `src/msgs/handshake.rs` — A clean, modern, safe implementation that mirrors the RFC structure closely. https://github.com/rustls/rustls
