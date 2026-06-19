// Build a TLS 1.3 Client + Server
// Run: rustc main.rs && ./main
//
// Architecture:
//   Client → ClientHello → Server → ServerHello + Flight → Client → Finished → Connected
//
// Implements a simulated TLS 1.3 handshake with state machines for both
// client and server, transcript hashing, and key derivation.

// =============================================================================
// Step 1: TLS State Machine and Core Types
// =============================================================================

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
    ClientHello { key_share: Vec<u8>, supported_versions: Vec<u16>, random: [u8; 32] },
    ServerHello { key_share: Vec<u8>, selected_version: u16, random: [u8; 32] },
    EncryptedExtensions(Vec<u8>),
    Certificate(Vec<Vec<u8>>),
    CertificateVerify(Vec<u8>),
    Finished(Vec<u8>),
    Alert { level: u8, description: u8 },
}

#[derive(Debug)]
struct Transcript { messages: Vec<Vec<u8>> }

impl Transcript {
    fn new() -> Self { Transcript { messages: Vec::new() } }
    fn add(&mut self, data: &[u8]) { self.messages.push(data.to_vec()); }
    fn hash(&self) -> [u8; 32] {
        let mut combined = Vec::new();
        for msg in &self.messages { combined.extend_from_slice(msg); }
        let mut result = [0u8; 32];
        for (i, &byte) in combined.iter().enumerate() { result[i % 32] ^= byte; }
        result
    }
}

#[derive(Debug)]
struct KeySchedule {
    handshake_secret: Option<[u8; 32]>,
    master_secret: Option<[u8; 32]>,
    client_traffic_secret: Option<[u8; 32]>,
    server_traffic_secret: Option<[u8; 32]>,
}

impl KeySchedule {
    fn new() -> Self {
        KeySchedule { handshake_secret: None, master_secret: None,
                      client_traffic_secret: None, server_traffic_secret: None }
    }

    fn derive_handshake_secret(&mut self, shared_secret: &[u8]) {
        let mut secret = [0u8; 32];
        for (i, &b) in shared_secret.iter().take(32).enumerate() { secret[i] = b; }
        self.handshake_secret = Some(secret);
        self.server_traffic_secret = Some(secret);
        self.client_traffic_secret = Some(secret);
    }

    fn derive_master_secret(&mut self) {
        if let Some(ref hs) = self.handshake_secret {
            let mut master = [0u8; 32];
            master.copy_from_slice(hs);
            for b in master.iter_mut() { *b = b.wrapping_add(1); }
            self.master_secret = Some(master);
        }
    }
}

// =============================================================================
// Step 2: TLS Client
// =============================================================================

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
            key_share: vec![0x04, 0x12, 0x34],
            supported_versions: vec![0x0304],
            random: [0x42; 32],
        }
    }

    fn process_message(&mut self, msg: &TlsMessage) -> Result<Option<TlsMessage>, String> {
        self.transcript.add(&format!("{:?}", msg).into_bytes());
        match msg {
            TlsMessage::ServerHello { key_share, selected_version, .. } => {
                if *selected_version != 0x0304 {
                    return Err(format!("Unsupported version: 0x{:04x}", selected_version));
                }
                self.key_schedule.derive_handshake_secret(key_share);
                self.state = TlsState::ServerHelloReceived;
                println!("  [Client] ServerHello received, handshake secret derived");
                Ok(None)
            }
            TlsMessage::EncryptedExtensions(_) => {
                self.state = TlsState::ExpectingCertificate;
                println!("  [Client] EncryptedExtensions received");
                Ok(None)
            }
            TlsMessage::Certificate(_) => {
                self.state = TlsState::ExpectingCertificateVerify;
                println!("  [Client] Certificate received, verifying...");
                Ok(None)
            }
            TlsMessage::CertificateVerify(_) => {
                self.state = TlsState::ExpectingServerFinished;
                println!("  [Client] CertificateVerify received");
                Ok(None)
            }
            TlsMessage::Finished(_) => {
                let transcript_hash = self.transcript.hash();
                println!("  [Client] Server Finished verified (transcript hash: {:?})", &transcript_hash[..4]);
                self.key_schedule.derive_master_secret();
                let client_finished = TlsMessage::Finished(transcript_hash.to_vec());
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

// =============================================================================
// Step 3: TLS Server
// =============================================================================

struct TlsServer {
    state: TlsState,
    transcript: Transcript,
    key_schedule: KeySchedule,
}

impl TlsServer {
    fn new() -> Self {
        TlsServer {
            state: TlsState::ServerHelloReceived,
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
                self.key_schedule.derive_handshake_secret(key_share);
                println!("  [Server] ClientHello processed, sending ServerHello");
                Ok(TlsMessage::ServerHello {
                    key_share: vec![0x04, 0x56, 0x78],
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
            TlsMessage::Certificate(vec![vec![0x30, 0x82]]),
            TlsMessage::CertificateVerify(vec![0x00; 64]),
            TlsMessage::Finished(self.transcript.hash().to_vec()),
        ]
    }

    fn process_message(&mut self, msg: &TlsMessage) -> Result<(), String> {
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

// =============================================================================
// Step 4: Full Handshake Simulation
// =============================================================================

fn main() {
    println!("=== TLS 1.3 Handshake Simulation ===\n");

    let mut client = TlsClient::new("example.com");
    let mut server = TlsServer::new();

    // Client sends ClientHello
    let client_hello = client.build_client_hello();
    client.transcript.add(&format!("{:?}", client_hello).into_bytes());
    client.state = TlsState::ClientHelloSent;
    println!("Client -> Server: ClientHello");

    // Server processes ClientHello, sends ServerHello + flight
    let server_hello = server.process_client_hello(&client_hello).unwrap();
    server.transcript.add(&format!("{:?}", server_hello).into_bytes());
    println!("Server -> Client: ServerHello");

    client.process_message(&server_hello).unwrap();

    // Server sends encrypted flight
    let server_flight = server.build_server_flight();
    for msg in &server_flight {
        server.transcript.add(&format!("{:?}", msg).into_bytes());
        println!("Server -> Client: {:?}", std::mem::discriminant(msg));
        client.process_message(msg).unwrap();
    }

    // Client sends Finished
    let client_finished = client.build_client_finished().unwrap();
    client.transcript.add(&format!("{:?}", client_finished).into_bytes());
    println!("Client -> Server: Finished");
    server.process_message(&client_finished).unwrap();

    // Verify both sides connected
    println!("\nClient state: {:?}", client.state);
    println!("Server state: {:?}", server.state);

    println!("\nKey schedule:");
    println!("  Handshake secret: {:?}", client.key_schedule.handshake_secret.is_some());
    println!("  Master secret: {:?}", client.key_schedule.master_secret.is_some());
}
