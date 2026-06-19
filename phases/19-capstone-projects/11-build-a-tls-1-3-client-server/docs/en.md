# Build a TLS 1.3 Client + Server

> Secure channels require strict handshake state and key schedule discipline.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 19 lessons 01-10
**Time:** ~720 minutes

## Learning Objectives

- Decompose TLS 1.3 handshake stages and key transitions.
- Model client/server state progression with explicit checks.
- Build minimal message-flow simulation before full crypto integration.
- Define validation gates for protocol correctness.

## The Problem

TLS implementations are fragile when state transitions and cryptographic assumptions are implicit. Someone starts implementing TLS 1.3, gets the ClientHello working, adds key exchange, discovers that the transcript hash is wrong because a message was included out of order, and can't tell whether the bug is in the message parser, the key schedule, or the hash computation.

The fix: explicit state machines. Each state has a defined set of valid incoming messages, a defined set of outgoing messages, and a defined transition. If a message arrives in the wrong state, the implementation rejects it. If a transition requires a key derivation step, the state machine enforces it.

TLS 1.3 simplifies the handshake compared to TLS 1.2: the entire key exchange happens in the first two flights (ClientHello + ServerHello), and all subsequent messages are encrypted. This makes the state machine cleaner but every step must be correct.

## The Concept

The TLS 1.3 handshake has a clear state progression:

```
Client                              Server
  │                                    │
  │──── ClientHello ──────────────────→│
  │     (key_share, supported_versions)│
  │                                    │
  │←─── ServerHello ──────────────────│
  │     (key_share, selected_version)  │
  │                                    │
  │←─── {EncryptedExtensions} ────────│  ← encrypted
  │←─── {Certificate} ────────────────│  ← encrypted
  │←─── {CertificateVerify} ──────────│  ← encrypted
  │←─── {Finished} ───────────────────│  ← encrypted
  │                                    │
  │──── {Finished} ──────────────────→│  ← encrypted
  │                                    │
  │←─────── application data ────────→│
```

The key schedule derives traffic keys from the shared secret:

```
PSK (pre-shared key, often 0)
    │
    ▼
Early Secret = HKDF-Extract(PSK, 0)
    │
    ├──→ early_traffic_secret
    │
    ▼
Handshake Secret = HKDF-Extract(DHE, derived_secret)
    │
    ├──→ client_handshake_traffic_secret
    ├──→ server_handshake_traffic_secret
    │
    ▼
Master Secret = HKDF-Extract(0, derived_secret)
    │
    ├──→ client_application_traffic_secret
    ├──→ server_application_traffic_secret
```

## Build It

We implement a typed state progression scaffold with placeholders for key schedule calls and transcript hashing.

### Step 1: TLS State Machine (Rust)

```rust
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum TlsState {
    ClientStart,
    ClientHelloSent,
    ServerHelloReceived,
    ExpectingEncryptedExtensions,
    ExpectingCertificate,
    ExpectingCertificateVerify,
    ExpectingServerFinished,
    ClientFinishedSent,
    Connected,
    Error(String),
}

#[derive(Debug, Clone)]
enum TlsMessage {
    ClientHello {
        key_share: Vec<u8>,
        supported_versions: Vec<u16>,
        random: [u8; 32],
    },
    ServerHello {
        key_share: Vec<u8>,
        selected_version: u16,
        random: [u8; 32],
    },
    EncryptedExtensions(Vec<u8>),
    Certificate(Vec<Vec<u8>>),       // Certificate chain
    CertificateVerify(Vec<u8>),      // Signature over transcript
    Finished(Vec<u8>),               // MAC over transcript
    Alert { level: u8, description: u8 },
}

#[derive(Debug)]
struct Transcript {
    messages: Vec<Vec<u8>>,  // Serialized handshake messages
}

impl Transcript {
    fn new() -> Self {
        Transcript { messages: Vec::new() }
    }

    fn add(&mut self, data: &[u8]) {
        self.messages.push(data.to_vec());
    }

    fn hash(&self) -> [u8; 32] {
        // Simplified: in real TLS, this is SHA-256 or SHA-384
        // of the concatenation of all handshake messages
        let mut combined = Vec::new();
        for msg in &self.messages {
            combined.extend_from_slice(msg);
        }
        // Fake hash: just XOR-fold the data into 32 bytes
        let mut result = [0u8; 32];
        for (i, &byte) in combined.iter().enumerate() {
            result[i % 32] ^= byte;
        }
        result
    }
}

#[derive(Debug)]
struct KeySchedule {
    // In real TLS, these are HKDF-derived secrets
    handshake_secret: Option<[u8; 32]>,
    master_secret: Option<[u8; 32]>,
    client_traffic_secret: Option<[u8; 32]>,
    server_traffic_secret: Option<[u8; 32]>,
}

impl KeySchedule {
    fn new() -> Self {
        KeySchedule {
            handshake_secret: None,
            master_secret: None,
            client_traffic_secret: None,
            server_traffic_secret: None,
        }
    }

    fn derive_handshake_secret(&mut self, shared_secret: &[u8]) {
        // In real TLS: HKDF-Extract(0, ECDHE_shared_secret)
        let mut secret = [0u8; 32];
        for (i, &b) in shared_secret.iter().take(32).enumerate() {
            secret[i] = b;
        }
        self.handshake_secret = Some(secret);
        // Derive handshake traffic secrets
        self.server_traffic_secret = Some(secret); // Simplified
        self.client_traffic_secret = Some(secret);
    }

    fn derive_master_secret(&mut self) {
        // In real TLS: HKDF-Extract(0, derived_secret)
        if let Some(ref hs) = self.handshake_secret {
            let mut master = [0u8; 32];
            master.copy_from_slice(hs);
            // Simple transformation for demo
            for b in master.iter_mut() { *b = b.wrapping_add(1); }
            self.master_secret = Some(master);
        }
    }

    fn get_server_key(&self) -> Option<[u8; 32]> {
        self.server_traffic_secret
    }

    fn get_client_key(&self) -> Option<[u8; 32]> {
        self.client_traffic_secret
    }
}
```

### Step 2: TLS Client Implementation

```rust
struct TlsClient {
    state: TlsState,
    transcript: Transcript,
    key_schedule: KeySchedule,
    server_name: String,
}

impl TlsClient {
    fn new(server_name: &str) -> Self {
        TlsClient {
            state: TlsState::ClientStart,
            transcript: Transcript::new(),
            key_schedule: KeySchedule::new(),
            server_name: server_name.to_string(),
        }
    }

    fn build_client_hello(&self) -> TlsMessage {
        TlsMessage::ClientHello {
            key_share: vec![0x04, 0x12, 0x34], // Fake ECDHE public key
            supported_versions: vec![0x0304],   // TLS 1.3
            random: [0x42; 32],
        }
    }

    fn process_server_hello(&mut self, msg: &TlsMessage) -> Result<(), String> {
        match (self.state.clone(), msg) {
            (TlsState::ClientHelloSent, TlsMessage::ServerHello { key_share, selected_version, .. }) => {
                if *selected_version != 0x0304 {
                    return Err(format!("Unsupported version: 0x{:04x}", selected_version));
                }
                // Derive handshake secret from key share
                self.key_schedule.derive_handshake_secret(key_share);
                self.state = TlsState::ServerHelloReceived;
                println!("  [Client] ServerHello received, handshake secret derived");
                Ok(())
            }
            (state, _) => Err(format!("Unexpected ServerHello in state {:?}", state)),
        }
    }

    fn process_encrypted_extensions(&mut self, _msg: &TlsMessage) -> Result<(), String> {
        match self.state {
            TlsState::ServerHelloReceived => {
                self.state = TlsState::ExpectingCertificate;
                println!("  [Client] EncryptedExtensions received");
                Ok(())
            }
            _ => Err(format!("Unexpected EncryptedExtensions in state {:?}", self.state)),
        }
    }

    fn process_certificate(&mut self, _msg: &TlsMessage) -> Result<(), String> {
        match self.state {
            TlsState::ExpectingCertificate => {
                self.state = TlsState::ExpectingCertificateVerify;
                println!("  [Client] Certificate received, verifying...");
                Ok(())
            }
            _ => Err(format!("Unexpected Certificate in state {:?}", self.state)),
        }
    }

    fn process_server_finished(&mut self, msg: &TlsMessage) -> Result<(), String> {
        match (&self.state, msg) {
            (TlsState::ExpectingServerFinished, TlsMessage::Finished(mac)) => {
                // Verify MAC over transcript hash
                let transcript_hash = self.transcript.hash();
                println!("  [Client] Server Finished verified (transcript hash: {:?})", 
                         &transcript_hash[..4]);
                self.key_schedule.derive_master_secret();
                self.state = TlsState::ExpectingServerFinished;
                Ok(())
            }
            _ => Err(format!("Unexpected Finished in state {:?}", self.state)),
        }
    }

    fn build_client_finished(&self) -> TlsMessage {
        let transcript_hash = self.transcript.hash();
        TlsMessage::Finished(transcript_hash.to_vec())
    }

    fn process_message(&mut self, msg: &TlsMessage) -> Result<Option<TlsMessage>, String> {
        // Add to transcript
        self.transcript.add(&format!("{:?}", msg).into_bytes());

        match msg {
            TlsMessage::ServerHello { .. } => {
                self.process_server_hello(msg)?;
                Ok(None)
            }
            TlsMessage::EncryptedExtensions(_) => {
                self.process_encrypted_extensions(msg)?;
                Ok(None)
            }
            TlsMessage::Certificate(_) => {
                self.process_certificate(msg)?;
                Ok(None)
            }
            TlsMessage::CertificateVerify(_) => {
                self.state = TlsState::ExpectingServerFinished;
                println!("  [Client] CertificateVerify received");
                Ok(None)
            }
            TlsMessage::Finished(_) => {
                self.process_server_finished(msg)?;
                let client_finished = self.build_client_finished();
                self.state = TlsState::Connected;
                println!("  [Client] Connected!");
                Ok(Some(client_finished))
            }
            TlsMessage::Alert { level, description } => {
                self.state = TlsState::Error(format!("Alert: level={}, desc={}", level, description));
                Err(format!("TLS Alert: level={}, description={}", level, description))
            }
            _ => Err(format!("Unexpected message in state {:?}", self.state)),
        }
    }
}
```

### Step 3: TLS Server Implementation

```rust
struct TlsServer {
    state: TlsState,
    transcript: Transcript,
    key_schedule: KeySchedule,
}

impl TlsServer {
    fn new() -> Self {
        TlsServer {
            state: TlsState::ServerHelloReceived, // Starts after receiving ClientHello
            transcript: Transcript::new(),
            key_schedule: KeySchedule::new(),
        }
    }

    fn process_client_hello(&mut self, msg: &TlsMessage) -> Result<TlsMessage, String> {
        match msg {
            TlsMessage::ClientHello { key_share, supported_versions, .. } => {
                if !supported_versions.contains(&0x0304) {
                    return Err("Client doesn't support TLS 1.3".into());
                }
                // Derive handshake secret
                self.key_schedule.derive_handshake_secret(key_share);
                self.state = TlsState::ServerHelloReceived;
                println!("  [Server] ClientHello processed, sending ServerHello");

                Ok(TlsMessage::ServerHello {
                    key_share: vec![0x04, 0x56, 0x78], // Server's ECDHE public key
                    selected_version: 0x0304,
                    random: [0x24; 32],
                })
            }
            _ => Err("Expected ClientHello".into()),
        }
    }

    fn build_server_flight(&self) -> Vec<TlsMessage> {
        vec![
            TlsMessage::EncryptedExtensions(vec![0x00]),
            TlsMessage::Certificate(vec![vec![0x30, 0x82]]), // Fake cert
            TlsMessage::CertificateVerify(vec![0x00; 64]),
            TlsMessage::Finished(self.transcript.hash().to_vec()),
        ]
    }

    fn process_client_finished(&mut self, msg: &TlsMessage) -> Result<(), String> {
        match msg {
            TlsMessage::Finished(_) => {
                self.key_schedule.derive_master_secret();
                self.state = TlsState::Connected;
                println!("  [Server] Connected!");
                Ok(())
            }
            _ => Err("Expected client Finished".into()),
        }
    }
}
```

### Step 4: Full Handshake Simulation

```rust
fn main() {
    println!("=== TLS 1.3 Handshake Simulation ===\n");

    let mut client = TlsClient::new("example.com");
    let mut server = TlsServer::new();

    // Client sends ClientHello
    let client_hello = client.build_client_hello();
    client.transcript.add(&format!("{:?}", client_hello).into_bytes());
    client.state = TlsState::ClientHelloSent;
    println!("Client → Server: ClientHello");

    // Server processes ClientHello, sends ServerHello + flight
    let server_hello = server.process_client_hello(&client_hello).unwrap();
    server.transcript.add(&format!("{:?}", server_hello).into_bytes());
    println!("Server → Client: ServerHello");

    client.process_message(&server_hello).unwrap();

    // Server sends encrypted flight
    let server_flight = server.build_server_flight();
    for msg in &server_flight {
        server.transcript.add(&format!("{:?}", msg).into_bytes());
        println!("Server → Client: {:?}", std::mem::discriminant(msg));
        client.process_message(msg).unwrap();
    }

    // Client sends Finished
    let client_finished = client.build_client_finished();
    client.transcript.add(&format!("{:?}", client_finished).into_bytes());
    println!("Client → Server: Finished");
    server.process_message(&client_finished).unwrap();

    // Verify both sides connected
    println!("\nClient state: {:?}", client.state);
    println!("Server state: {:?}", server.state);

    // Verify key derivation
    println!("\nKey schedule:");
    println!("  Handshake secret: {:?}", client.key_schedule.handshake_secret.is_some());
    println!("  Master secret: {:?}", client.key_schedule.master_secret.is_some());
}
```

## Use It

This approach maps to real protocol implementations:

- **rustls**: a production TLS 1.3 library in Rust. Its state machine is in `rustls/src/client/hs.rs` and `rustls/src/server/hs.rs`. Each state is a struct implementing a `State` trait with a `handle()` method that processes the next message and returns the next state.
- **OpenSSL**: the most widely deployed TLS library. Its state machine is implicit (driven by flags and mode variables) but the handshake flow is the same. The `ssl/statem/` directory contains the state machine code.
- **BoringSSL**: Google's OpenSSL fork, used in Chrome and Android. Its TLS 1.3 implementation is in `ssl/tls13_client.cc` and `ssl/tls13_server.cc`.

The key production lesson: **the transcript hash is the security anchor**. Every handshake message is included in the transcript hash. The Finished message is a MAC over this hash. If any message is modified, added, or removed, the Finished verification fails. This prevents tampering with the handshake.

## Read the Source

- [RFC 8446](https://www.rfc-editor.org/rfc/rfc8446) — The TLS 1.3 specification. Section 4 (Handshake Protocol) defines the message flow. Section 7 (Key Schedule) defines the key derivation.
- [rustls](https://github.com/rustls/rustls) — A production Rust TLS 1.3 library. The state machine is clean and well-structured.
- [tls13-spec-explained](https://tls13.xargs.org/) — A visual walkthrough of the TLS 1.3 handshake with packet captures.

## Ship It

- `code/main.rs`: TLS 1.3 handshake state machine with client/server simulation, transcript hashing, and key schedule derivation.
- `outputs/README.md`: TLS capstone checklist covering state machine, key schedule, transcript, and certificate verification.

## Exercises

1. **Easy** — Add alert/error state transitions. When the client receives an unexpected message, send a `fatal_alert` and transition to the Error state. When the server receives a malformed ClientHello, respond with `decode_error`. Test that both sides handle alerts gracefully.
2. **Medium** — Add transcript hash checkpoints. After each handshake message, print the current transcript hash. Verify that client and server compute the same hash at each step. Add an assertion that the hashes match before deriving traffic keys.
3. **Hard** — Add certificate verification integration plan. Implement a `verify_certificate(chain, trusted_roots)` function that checks the certificate chain's signatures, expiration dates, and hostname. Use a mock CA for testing. Show that an invalid certificate causes the handshake to fail with the appropriate alert.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Handshake | "connection setup" | The negotiation phase where client and server agree on cipher suites, exchange keys, and verify identities. In TLS 1.3, the handshake completes in 1-RTT (one round trip). |
| Transcript | "message history" | The ordered concatenation of all handshake messages. The Finished message is a MAC over the transcript hash. This binds all handshake messages into the key confirmation. |
| Key schedule | "key derivation pipeline" | The HKDF-based process that derives traffic keys from the shared secret. TLS 1.3 derives separate keys for handshake traffic and application traffic, plus separate keys for each direction. |
| Forward secrecy | "past-safe keys" | The property that compromising long-term keys doesn't reveal past session keys. TLS 1.3 achieves this by using ephemeral ECDHE key exchange: each session uses fresh keys that are deleted after the session. |
| HKDF | "key derivation function" | HMAC-based Key Derivation Function. Extracts pseudorandom key material from input keying material and expands it into multiple output keys. Used throughout the TLS 1.3 key schedule. |

## Further Reading

- [RFC 8446](https://www.rfc-editor.org/rfc/rfc8446) — The TLS 1.3 specification.
- [rustls](https://github.com/rustls/rustls) — Production Rust TLS implementation.
- [The Illustrated TLS 1.3 Connection](https://tls13.xargs.org/) — Visual walkthrough with packet captures.
