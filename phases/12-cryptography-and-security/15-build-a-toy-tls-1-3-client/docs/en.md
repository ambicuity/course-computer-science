# Build a Toy TLS 1.3 Client

> When your browser speaks TLS for you, you never see the handshake. But when you need to speak it yourself — from an IoT sensor, a security scanner, or a custom proxy — every byte matters.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 12 lessons 01–14 (especially lessons 11 + 14)
**Time:** ~120 minutes

## Learning Objectives

- Construct a TLS 1.3 ClientHello with required extensions (key_share, supported_versions, signature_algorithms) from raw bytes.
- Perform X25519 ECDHE key exchange and derive the full TLS 1.3 key schedule via HKDF-Extract/Expand.
- Implement AES-128-GCM record protection with sequence-number nonces.
- Verify both server and client Finished messages, proving handshake integrity.
- Exchange encrypted application data over a loopback TLS 1.3 connection.

## The Problem

Browsers do TLS transparently. You type `https://example.com`, the browser negotiates cipher suites, validates certificate chains, derives keys, and encrypts — all without you seeing a single handshake byte. This is a feature for everyday browsing. But what happens when you need to speak TLS from a constrained IoT device that has no TLS library? Or from a custom security audit tool that needs to inspect the handshake? Or from a load balancer that terminates TLS for thousands of backends?

You cannot use a browser. You cannot even use libcurl or rustls — those do TLS *for* you, hiding the details. If you need to understand what the handshake looks like on the wire, to debug a protocol-level issue, or to implement TLS on a platform that has no existing library, you need to build the handshake yourself from the socket up.

The phase capstone (lesson 24) asks you to build a full TLS 1.3 library paired with a mini-CTF toolkit where players exploit weaknesses in toy implementations. This lesson is the bridge: lesson 14 gave you the primitives (record layer, key schedule, 0-RTT theory). This lesson wires them into a working client that connects, handshakes, and exchanges encrypted data. The capstone extends this into a server and adds attack scenarios.

## The Concept

TLS 1.3 reduces the full handshake to a single round trip (plus one more flight from the client). The flow:

```
Client                                    Server
  |                                         |
  |--- ClientHello ----------------------->|
  |    key_share: X25519 public key,       |
  |    supported_versions: 0x0304          |
  |    signature_algorithms, groups         |
  |                                         |--- compute ECDHE shared secret
  |                                         |--- derive handshake traffic keys
  |<-- ServerHello ------------------------|
  |    key_share: X25519 public key        |
  |<-- EncryptedExtensions ----------------|
  |<-- Certificate ------------------------|  (encrypted with HS keys)
  |<-- CertificateVerify ------------------|  (Ed25519 over transcript)
  |<-- Finished ---------------------------|  (HMAC over transcript)
  |                                         |
  |--- compute ECDHE shared secret         |
  |--- derive handshake traffic keys       |
  |--- verify CertificateVerify            |
  |--- verify server Finished               |
  |--- Finished -------------------------->|  (encrypted with HS keys)
  |                                         |--- verify client Finished
  |                                         |--- derive app traffic keys
  |=== Encrypted Application Data =========|
```

**Key concept 1 — ECDHE forward secrecy.** Each side generates an ephemeral X25519 keypair that lives only for this session. The shared secret \(g^{ab}\) is computed from ephemeral keys, so even if the server's long-term signing key is later compromised, past session keys remain secret. This is forward secrecy.

**Key concept 2 — Transcript hash binds everything.** SHA-256 hashes every handshake message in order. The hash appears inside `derive_secret` for every traffic secret. If an attacker modifies, reorders, or replays any message, the transcript hash diverges and the derived keys will not match — the handshake fails immediately. This is why TLS 1.3's handshake is called "detached": the transcript is not transmitted separately but is implicitly agreed upon by both sides through the key derivation, making any tampering detectable before any application data is exchanged.

**Key concept 3 — Finished proves integrity.** After all handshake messages are exchanged, each side sends a Finished message containing `HMAC(finished_key, transcript_hash)`. The finished_key is derived from the side's handshake traffic secret. If both sides have seen the exact same transcript, the HMACs match, and both know the handshake was untampered.

**Key concept 4 — CertificateVerify proves identity.** The server signs the transcript hash with its long-term private key (Ed25519 in this lesson). The client verifies the signature against the server's public key. This proves the server controls the private key corresponding to the certificate — that it is who it claims to be.

In practice, every HTTPS connection your browser makes follows this exact flow. The difference is that your browser's TLS library handles the 5+ round trips of certificate chain validation, OCSP stapling, session tickets, and key update, while our toy focuses on the cryptographic core. Understanding that core — the four concepts above — is what separates someone who knows TLS exists from someone who could implement it.

## Build It

This lesson's implementation runs two threads connected over localhost TCP: a server that performs the TLS 1.3 server side of the handshake, and a client that performs the client side. Both use the same record layer, key schedule, and record protection code — the same crypto logic you would deploy on a real device.

The binary depends on `x25519-dalek` for X25519 key exchange, `sha2` for SHA-256, `hmac` for HMAC/HKDF, `aes-gcm` for AES-128-GCM record encryption, `rand` for randomness, `hex` for display, and `ed25519-dalek` for CertificateVerify signatures.

### Step 1: TLS Record Layer

Every TLS message travels inside a record. The wire format is minimal:

```
ContentType (1 byte) | ProtocolVersion (2 bytes) | Length (2 bytes) | Fragment
```

Before handshake keys are established, records are sent in cleartext with `ContentType = 22 (handshake)`. After keys are derived, records are encrypted with `ContentType = 23 (application_data)` — including handshake messages like Certificate and Finished. The inner content type (e.g., 22 for handshake) is appended to the plaintext as the last byte before encryption.

```
struct TLSRecord {
    content_type: u8,
    version: u16,
    payload: Vec<u8>,
}
```

The `encode` method writes the 5-byte header followed by the payload. The `decode` method reads the header, extracts the length, and returns the record — or `None` if the buffer is too short. This is used by both client and server to send and receive over TCP.

When you run the binary, it starts a server thread on localhost, then creates a client that connects over TCP. The output prints every step of the handshake: the ClientHello and ServerHello hex dumps, the transcript hash at each stage, the seven derived secrets (truncated for display), the CertificateVerify signature verification, the Finished verify_data, the encrypted ciphertext, and the decrypted application data. Every verification — ECDHE match, Finished HMAC, CertificateVerify signature, echo round-trip — is accompanied by a `✓ OK` or `✗ FAIL` indicator, making it easy to trace exactly where a handshake would break if any step went wrong.

### Step 2: ClientHello Construction

The ClientHello advertises what the client supports and provides its ephemeral public key. The body contains:

- **Legacy version**: `0x0303` (TLS 1.2, for middlebox compatibility)
- **Random**: 32 bytes from a CSPRNG
- **Session ID**: empty (legacy)
- **Cipher suites**: `TLS_AES_128_GCM_SHA256` (0x1301)
- **Extensions**:
  - `supported_versions` (0x002b): the client supports TLS 1.3 (0x0304)
  - `key_share` (0x0033): the client's X25519 public key (32 bytes), offered as the only key share
  - `signature_algorithms` (0x000d): Ed25519 (0x0807) and ECDSA secp256r1 (0x0403)
  - `supported_groups` (0x000a): x25519 (0x001d)

Each extension is a (type, length, value) triple. The key_share extension is the most important — it carries the raw 32-byte X25519 public key that the server uses to compute the ECDHE shared secret.

### Step 3: ServerHello and ECDHE Key Exchange

The server reads the ClientHello from the TCP socket, extracts the client's key_share, generates its own X25519 keypair, and responds with a ServerHello containing its own key_share.

Both sides then compute the ECDHE shared secret:

```rust
let client_shared = client_ephemeral.diffie_hellman(&server_public);
let server_shared = server_ephemeral.diffie_hellman(&client_public);
```

These produce identical 32-byte shared secrets (the X coordinate of \(g^{ab}\)). This shared secret is the root keying material for the entire session.

The transcript hash begins with `SHA-256(ClientHello || ServerHello)`, called the **HelloHash**. All subsequent handshake keys are bound to this hash.

### Step 4: Key Schedule

The TLS 1.3 key schedule follows a strict cascade of HKDF operations (RFC 8446 §7.1):

```
                0
                |
    HKDF-Extract -> early_secret
                |
    HKDF-Extract(early_secret, ECDHE) -> handshake_secret
                |
    Derive-Secret(., "c hs traffic", HelloHash) -> client_handshake_traffic_secret
    Derive-Secret(., "s hs traffic", HelloHash) -> server_handshake_traffic_secret
                |
    HKDF-Extract(handshake_secret, 0) -> master_secret
                |
    Derive-Secret(., "c ap traffic", HandshakeHash) -> client_app_traffic_secret
    Derive-Secret(., "s ap traffic", HandshakeHash) -> server_app_traffic_secret
```

Each `derive_secret` is an HKDF-Expand-Label operation that binds the derived key to a purpose label and the current transcript hash. The three primitives are:

- **HKDF-Extract**: `HMAC-SHA256(salt, ikm)` — turns a Diffie-Hellman shared secret into a uniformly random pseudorandom key (PRK).
- **HKDF-Expand**: iterated HMAC-SHA256 with a counter, producing arbitrary-length output from a PRK.
- **Derive-Secret**: wraps HKDF-Expand with the labeled construction `"tls13 " + label + context`.

From each traffic secret, the AEAD key (16 bytes) and IV (12 bytes) are derived by expanding with the labels `"key"` and `"iv"` respectively. The Finished key is derived by expanding with the label `"finished"`.

### Step 5: CertificateVerify and Finished

After deriving handshake traffic keys from the HelloHash, the server sends three encrypted messages:

1. **EncryptedExtensions** — carries extensions that don't belong in ServerHello (empty in our toy).
2. **Certificate** — the server's Ed25519 public key, sent as a bare "certificate" (no ASN.1 wrapping, since this is a toy).
3. **CertificateVerify** — an Ed25519 signature over `"TLS 1.3, server CertificateVerify\0" || transcript_hash`, proving the server controls the private key.

The client verifies the signature against the server's public key. If it does not match, the server is an impostor and the handshake is aborted.

4. **Finished** — `HMAC-SHA256(server_finished_key, transcript_hash)` where the transcript covers everything up to (but not including) the server Finished. The client recomputes the expected verify_data and checks it against the received value. If it matches, the client knows the server derived the same keys from the same transcript.

The client responds with its own Finished message (encrypted with the client handshake traffic key), whose verify_data covers the transcript including the server's Finished. The server verifies this to confirm the client derived the same keys.

### Step 6: Application Data and Closure

With the handshake complete, both sides derive application traffic keys from the master secret and the full handshake transcript. The client sends an encrypted HTTP request:

```rust
let request = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
let encrypted = client_app_protection.encrypt(request);
```

The server decrypts with its own server application protection, prints the plaintext, and echoes it back. The client decrypts and displays the response.

For connection closure, each side sends a `close_notify` alert:

```rust
// close_notify alert: content_type=21, level=1, description=0
let alert = vec![0x01, 0x00];  // warning, close_notify
let encrypted_alert = protection.encrypt(&alert);
```

This is the TLS 1.3 equivalent of a polite goodbye — it tells the peer that no more data will be sent, preventing truncation attacks.

## Use It

Production TLS 1.3 implementations like rustls, OpenSSL, and BoringSSL are 10–100× more complex than this toy. They handle:

- **Certificate chain validation**: verifying that the server's certificate chains to a trusted root CA, checking CRLs and OCSP for revocation, and enforcing name constraints.
- **Session resumption**: issuing and accepting PSK session tickets, enabling 0-RTT data on subsequent connections.
- **Key update**: re-keying within a long-lived session to limit ciphertext exposure.
- **Middlebox compatibility**: inserting `change_cipher_spec` dummy records that firewalls expect.
- **Multiple cipher suites**: supporting AES-256-GCM, ChaCha20-Poly1305, and other AEAD constructions.
- **Alert system**: a full taxonomy of alerts for every error condition (decrypt_error, certificate_expired, handshake_failure, etc.).

What your toy client gets right: the core crypto. The ECDHE exchange, the HKDF key schedule, the AEAD record protection with sequence-number nonces, and the Finished integrity check are identical to what rustls and OpenSSL do. If you understand your toy implementation, you understand the crypto core of every TLS 1.3 connection on the internet.

Compare with OpenSSL's `s_client`:

```bash
openssl s_client -connect example.com:443 -tls1_3
```

OpenSSL prints the full handshake transcript: the negotiated cipher suite, the server's certificate chain, the session ticket. Your toy does the same handshake in miniature — same key exchange, same key schedule, same AEAD encryption. The difference is that OpenSSL handles every edge case, every extension, every certificate format.

Compare with rustls (`rustls/src/tls13/key_schedule.rs`): the KeySchedule struct and derive_traffic_key function are structurally identical to your implementation. The HKDF-Extract/Expand loop, the labeled derivation, the 7-secret cascade — all the same.

## Read the Source

- **RFC 8446 §4 (Handshake), §5 (Record Protocol), §7 (Key Schedule)** — The definitive specification. Read §7.1 for the labeled derivation this lesson implements verbatim.
- **rustls `src/tls13/key_schedule.rs`** — The KeySchedule struct and derive_traffic_key function. Compare the 7-secret cascade with your own. rustls's version handles PSK and early export, but the core is identical.
- **rustls `src/tls13/handshake.rs`** — The full TLS 1.3 handshake state machine. Search for `emit_server_hello` and `expect_client_hello` to see how rustls tracks handshake state.
- **OpenSSL `ssl/statem/statem_clnt.c`** — The client-side state machine for OpenSSL. Massive switch statements handling hundreds of edge cases. Useful contrast with your focused implementation.
- **Cloudflare TLS 1.3 Implementation Guide** — A blog-post series walking through implementing TLS 1.3 from scratch in Go. The handshake diagrams in Part 2 match this lesson's flow.

## Ship It

The reusable artifact is a working TLS 1.3 client that performs a full handshake (ECDHE + HKDF + AEAD + CertificateVerify + Finished) over TCP and exchanges encrypted application data. It lives in `outputs/` and directly feeds into the phase capstone (lesson 24): the capstone wires these components into a full TLS 1.3 library and pairs it with a mini-CTF toolkit where players exploit weaknesses in toy implementations.

## Exercises

1. **Easy** — Trace the key schedule: after the handshake completes, print every derived secret (early, handshake, master, all traffic secrets, all keys, all IVs) alongside the HelloHash and HandshakeHash. Verify that changing a single byte of the ClientHello causes every subsequent secret to diverge.

2. **Medium** — Add PSK session resumption: after the handshake, have the server issue a 32-byte session ticket (encrypted with a server-held key). On the next connection, the client includes the ticket in its ClientHello as a `pre_shared_key` extension skips the ECDHE exchange, and derives keys directly from the PSK. Compare the handshake latency with and without resumption.

3. **Hard** — Implement a downgrade attack: have a man-in-the-middle strip the `supported_versions` extension from the ClientHello, causing the server to fall back to TLS 1.2. The client then receives a TLS 1.2 ServerHello. Show that the server's downgrade protection (a sentinel value in the ServerHello random bytes: `44 4F 57 4E 47 52 44` for TLS 1.2) causes the client to detect the downgrade and abort the connection. Then extend your implementation to detect this sentinel and reject the handshake with a `protocol_version` alert.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| ClientHello | The first message in a TLS handshake | Advertises supported versions, cipher suites, key shares, and signature algorithms; contains the client's ephemeral X25519 public key |
| key_share | An extension carrying public key material | Contains the NamedGroup (e.g., x25519 = 0x001d) and the raw 32-byte public key for ECDHE |
| ECDHE | Elliptic curve Diffie-Hellman ephemeral | Each side generates a fresh X25519 keypair per-session; the shared secret \(g^{ab}\) is never reused, guaranteeing forward secrecy |
| Transcript hash | A running SHA-256 of all handshake messages | Binds every derived key to the exact sequence of messages seen; tampering with any message causes all subsequent keys to diverge |
| HKDF | HMAC-based key derivation function | A two-step primitive (Extract then Expand) that turns a non-uniform Diffie-Hellman shared secret into uniformly random traffic keys |
| CertificateVerify | Proof the server owns its certificate | An Ed25519 signature over the transcript hash, proving the server controls the private key corresponding to its certificate |
| Finished | The handshake integrity check | HMAC-SHA256(finished_key, transcript_hash); proves both sides derived the same keys from the same transcript |
| close_notify | A TLS alert that signals connection closure | A polite goodbye that tells the peer no more data will follow; prevents truncation attacks where an attacker cuts off the ciphertext |

## Further Reading

- **RFC 8446** — "The Transport Layer Security (TLS) Protocol Version 1.3." Read §2 (protocol overview), §4 (handshake), §5 (record protocol), §7 (key schedule). The definitive reference.
- **"TLS 1.3 in Practice" by David Wong** — A practical book covering the wire format, every extension, and common implementation mistakes.
- **Cloudflare TLS 1.3 Blog Series** — "An Overview of TLS 1.3" and "TLS 1.3 in Go" — accessible explanations with interactive handshake visualizations.
- **rustls source code** — `src/tls13/key_schedule.rs` and `src/tls13/handshake.rs`. A clean, modern, safe implementation that mirrors the RFC structure.
- **"The Security of TLS 1.3" by Krawczyk, Paterson, and Wee** — The formal security analysis proving session independence, forward secrecy, and key privacy.
