//! QUIC and HTTP/3 Simulation
//! Phase 09 — Computer Networks
//!
//! Simplified QUIC connection demonstrating:
//! - 0-RTT connection establishment
//! - Independent stream multiplexing
//! - Connection migration
//! - Comparison with TCP behavior
//!
//! Run: rustc main.rs -o quic_demo && ./quic_demo

use std::collections::HashMap;
use std::fmt;
use std::net::SocketAddr;

/// Connection identifier — QUIC connections survive IP changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ConnectionId(u128);

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CID-{:016x}", self.0)
    }
}

/// Stream states within a QUIC connection.
#[derive(Debug, Clone, Copy, PartialEq)]
enum StreamState {
    Idle,
    Open,
    HalfClosedLocal,
    HalfClosedRemote,
    Closed,
}

impl fmt::Display for StreamState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            StreamState::Idle             => "IDLE",
            StreamState::Open             => "OPEN",
            StreamState::HalfClosedLocal  => "HALF_CLOSED_LOCAL",
            StreamState::HalfClosedRemote => "HALF_CLOSED_REMOTE",
            StreamState::Closed           => "CLOSED",
        };
        write!(f, "{}", s)
    }
}

/// A single bidirectional stream within a QUIC connection.
#[derive(Debug)]
struct Stream {
    stream_id: u64,
    state: StreamState,
    send_buffer: Vec<u8>,
    recv_buffer: Vec<u8>,
    bytes_sent: usize,
    bytes_recv: usize,
}

impl Stream {
    fn new(stream_id: u64) -> Self {
        Stream {
            stream_id,
            state: StreamState::Idle,
            send_buffer: Vec::new(),
            recv_buffer: Vec::new(),
            bytes_sent: 0,
            bytes_recv: 0,
        }
    }

    fn send(&mut self, data: &[u8]) {
        if self.state == StreamState::Idle {
            self.state = StreamState::Open;
        }
        self.send_buffer.extend_from_slice(data);
        self.bytes_sent += data.len();
        println!("    Stream {}: sent {} bytes (total: {})",
                 self.stream_id, data.len(), self.bytes_sent);
    }

    fn receive(&mut self, data: &[u8]) {
        self.recv_buffer.extend_from_slice(data);
        self.bytes_recv += data.len();
        println!("    Stream {}: received {} bytes (total: {})",
                 self.stream_id, data.len(), self.bytes_recv);
    }

    fn close_send(&mut self) {
        match self.state {
            StreamState::Open => self.state = StreamState::HalfClosedLocal,
            StreamState::HalfClosedRemote => self.state = StreamState::Closed,
            _ => {}
        }
    }

    fn close_recv(&mut self) {
        match self.state {
            StreamState::Open => self.state = StreamState::HalfClosedRemote,
            StreamState::HalfClosedLocal => self.state = StreamState::Closed,
            _ => {}
        }
    }
}

/// Represents a cached session for 0-RTT.
#[derive(Debug)]
struct SessionCache {
    server_name: String,
    keys: Vec<u8>,
    has_ticket: bool,
}

impl SessionCache {
    fn new(server_name: &str) -> Self {
        SessionCache {
            server_name: server_name.to_string(),
            keys: vec![0xDE, 0xAD, 0xBE, 0xEF], // simulated key material
            has_ticket: false,
        }
    }

    fn store(&mut self) {
        self.has_ticket = true;
        println!("  [TLS] Session ticket cached for {}", self.server_name);
    }
}

/// A QUIC connection.
#[derive(Debug)]
struct QuicConnection {
    connection_id: ConnectionId,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    streams: HashMap<u64, Stream>,
    next_stream_id: u64,
    is_established: bool,
    is_0rtt: bool,
    session_cache: Option<SessionCache>,
    total_bytes_sent: usize,
    total_bytes_recv: usize,
}

impl QuicConnection {
    fn new(
        local: SocketAddr,
        remote: SocketAddr,
        cache: Option<SessionCache>,
    ) -> Self {
        let cid = ConnectionId(rand_cid());
        let can_0rtt = cache.as_ref().map_or(false, |c| c.has_ticket);

        println!("  [QUIC] Creating connection {}", cid);
        println!("  [QUIC] {} → {}", local, remote);
        if can_0rtt {
            println!("  [QUIC] Cached session found — 0-RTT available");
        }

        QuicConnection {
            connection_id: cid,
            local_addr: local,
            remote_addr: remote,
            streams: HashMap::new(),
            next_stream_id: 0,
            is_established: false,
            is_0rtt: can_0rtt,
            session_cache: cache,
            total_bytes_sent: 0,
            total_bytes_recv: 0,
        }
    }

    /// Simulate the QUIC handshake.
    fn handshake(&mut self) {
        if self.is_0rtt {
            println!("\n  [QUIC] 0-RTT Handshake:");
            println!("    Client → Server: Initial packet (ClientHello + 0-RTT data)");
            println!("    Server: validates 0-RTT keys, accepts early data");
            println!("    Server → Client: Handshake packet (ServerHello + Finished)");
            println!("    Client → Server: Handshake Finished");
            println!("  [QUIC] Connection established in 0-RTT (data sent with first packet)");
        } else {
            println!("\n  [QUIC] 1-RTT Handshake:");
            println!("    Client → Server: Initial packet (ClientHello)");
            println!("    Server → Client: Handshake packet (ServerHello + EncryptedExtensions)");
            println!("    Client → Server: Handshake Finished");
            println!("  [QUIC] Connection established in 1-RTT");
        }
        self.is_established = true;

        if let Some(ref mut cache) = self.session_cache {
            cache.store();
        }
    }

    /// Open a new bidirectional stream.
    fn open_stream(&mut self) -> u64 {
        let id = self.next_stream_id;
        self.next_stream_id += 1; // QUIC uses even IDs for client-initiated
        self.streams.insert(id, Stream::new(id));
        println!("  [QUIC] Opened stream {}", id);
        id
    }

    /// Send data on a specific stream.
    fn send_on_stream(&mut self, stream_id: u64, data: &[u8]) {
        if let Some(stream) = self.streams.get_mut(&stream_id) {
            stream.send(data);
            self.total_bytes_sent += data.len();
        } else {
            eprintln!("  [QUIC] Error: stream {} not found", stream_id);
        }
    }

    /// Simulate receiving data on a stream.
    fn receive_on_stream(&mut self, stream_id: u64, data: &[u8]) {
        if let Some(stream) = self.streams.get_mut(&stream_id) {
            stream.receive(data);
            self.total_bytes_recv += data.len();
        } else {
            // Auto-open stream for incoming data
            let mut stream = Stream::new(stream_id);
            stream.receive(data);
            self.total_bytes_recv += data.len();
            self.streams.insert(stream_id, stream);
        }
    }

    /// Migrate the connection to a new local address.
    fn migrate(&mut self, new_addr: SocketAddr) {
        println!("\n  [QUIC] Connection Migration:");
        println!("    Old address: {}", self.local_addr);
        println!("    New address: {}", new_addr);
        println!("    Connection ID: {} (unchanged)", self.connection_id);
        println!("    → Sending PATH_CHALLENGE on new path");
        println!("    → Server responds with PATH_RESPONSE");
        println!("    → Migration successful — all streams continue");
        self.local_addr = new_addr;
    }

    /// Close a stream.
    fn close_stream(&mut self, stream_id: u64) {
        if let Some(stream) = self.streams.get_mut(&stream_id) {
            stream.close_send();
            stream.close_recv();
            println!("  [QUIC] Closed stream {}", stream_id);
        }
    }

    /// Print connection summary.
    fn summary(&self) {
        println!("\n  ═══════════════════════════════════");
        println!("  Connection Summary: {}", self.connection_id);
        println!("  ═══════════════════════════════════");
        println!("  Local:  {}", self.local_addr);
        println!("  Remote: {}", self.remote_addr);
        println!("  Established: {}", self.is_established);
        println!("  0-RTT used: {}", self.is_0rtt);
        println!("  Total bytes sent: {}", self.total_bytes_sent);
        println!("  Total bytes recv: {}", self.total_bytes_recv);
        println!("  Streams:");
        for (id, stream) in &self.streams {
            println!("    Stream {:>2}: state={:<20} sent={:>6} recv={:>6}",
                     id, stream.state.to_string(), stream.bytes_sent, stream.bytes_recv);
        }
        println!("  ═══════════════════════════════════\n");
    }
}

fn rand_cid() -> u128 {
    // Simple deterministic "random" for demo purposes
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    t.as_nanos()
}

// ── Comparison demos ────────────────────────────────────────────────

fn demo_handshake_comparison() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  HANDSHAKE COMPARISON: TCP+TLS vs QUIC                  ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    println!("── TCP + TLS 1.2 ────────────────────────────────");
    println!("  Round 1: SYN                    (TCP handshake)");
    println!("  Round 2: SYN+ACK                (TCP handshake)");
    println!("  Round 3: ACK                    (TCP handshake)");
    println!("  Round 4: ClientHello            (TLS start)");
    println!("  Round 5: ServerHello + Cert     (TLS)");
    println!("  Round 6: Finished               (TLS done)");
    println!("  Round 7: HTTP GET               ← DATA FINALLY FLOWS");
    println!("  Total: 3 RTTs (TCP) + 2 RTTs (TLS) = 5 RTTs\n");

    println!("── TCP + TLS 1.3 ────────────────────────────────");
    println!("  Round 1: SYN                    (TCP handshake)");
    println!("  Round 2: SYN+ACK                (TCP handshake)");
    println!("  Round 3: ACK                    (TCP handshake)");
    println!("  Round 4: ClientHello            (TLS start)");
    println!("  Round 5: ServerHello + Finished (TLS done)");
    println!("  Round 6: HTTP GET               ← DATA FLOWS");
    println!("  Total: 3 RTTs (TCP) + 1 RTT (TLS) = 4 RTTs\n");

    println!("── QUIC 1-RTT (first connection) ────────────────");
    println!("  Round 1: Initial (ClientHello)  (transport + TLS combined)");
    println!("  Round 2: Handshake + Finished   (done)");
    println!("  Round 3: HTTP GET               ← DATA FLOWS");
    println!("  Total: 1 RTT (no separate TCP handshake)\n");

    println!("── QUIC 0-RTT (resumption) ──────────────────────");
    println!("  Round 1: 0-RTT (ClientHello + HTTP GET)");
    println!("           ← DATA FLOWS IN FIRST PACKET");
    println!("  Total: 0 RTTs (data arrives with handshake)\n");
}

fn demo_stream_multiplexing() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  STREAM MULTIPLEXING DEMO                                ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let local: SocketAddr = "192.168.1.100:4433".parse().unwrap();
    let remote: SocketAddr = "93.184.216.34:443".parse().unwrap();

    let mut cache = SessionCache::new("example.com");
    cache.store();

    let mut conn = QuicConnection::new(local, remote, Some(cache));
    conn.handshake();

    // Open 3 independent streams
    println!("\n── Opening 3 independent streams ──");
    let s0 = conn.open_stream(); // HTML
    let s1 = conn.open_stream(); // CSS
    let s2 = conn.open_stream(); // JS

    // Send data on each stream
    println!("\n── Sending requests on each stream ──");
    conn.send_on_stream(s0, b"GET /index.html HTTP/3");
    conn.send_on_stream(s1, b"GET /style.css HTTP/3");
    conn.send_on_stream(s2, b"GET /app.js HTTP/3");

    // Simulate receiving responses
    println!("\n── Receiving responses (interleaved, independent) ──");
    conn.receive_on_stream(s0, b"<html>...</html>");
    conn.receive_on_stream(s1, b"body { color: black; }");
    conn.receive_on_stream(s2, b"console.log('hello');");
    conn.receive_on_stream(s0, b"<body>more html</body>");

    println!("\n  Key insight: Stream {} loss does NOT block streams {} or {}!",
             s1, s0, s2);

    // Close streams
    println!("\n── Closing streams ──");
    conn.close_stream(s0);
    conn.close_stream(s1);
    conn.close_stream(s2);

    conn.summary();
}

fn demo_connection_migration() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  CONNECTION MIGRATION DEMO                               ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // Start on WiFi
    let wifi: SocketAddr = "192.168.1.100:4433".parse().unwrap();
    let server: SocketAddr = "93.184.216.34:443".parse().unwrap();

    let mut conn = QuicConnection::new(wifi, server, None);
    conn.handshake();

    // Open a stream and send data
    let s0 = conn.open_stream();
    conn.send_on_stream(s0, b"GET /video HTTP/3");
    conn.receive_on_stream(s0, b"video chunk 1...");
    conn.receive_on_stream(s0, b"video chunk 2...");

    // User walks out of WiFi range → switches to cellular
    let cellular: SocketAddr = "10.0.0.50:4433".parse().unwrap();
    conn.migrate(cellular);

    // Connection continues seamlessly
    println!("\n── After migration, streams continue uninterrupted ──");
    conn.receive_on_stream(s0, b"video chunk 3...");
    conn.receive_on_stream(s0, b"video chunk 4...");

    println!("\n  TCP comparison: the connection would BREAK on IP change");
    println!("  TCP 5-tuple: (src_ip, src_port, dst_ip, dst_port, proto)");
    println!("  QUIC connection ID: {} — IP is irrelevant!", conn.connection_id);

    conn.summary();
}

fn demo_hol_blocking_comparison() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  HEAD-OF-LINE BLOCKING COMPARISON                        ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    println!("── TCP + HTTP/2 (single byte stream) ────────────");
    println!("  Segments: [S1:1][S1:2][S2:1][S1:3][S2:2][S3:1]");
    println!("  Packet [S2:1] LOST");
    println!("  TCP receive buffer stalls waiting for [S2:1]");
    println!("  → Stream 1 data [S1:3] BLOCKED (can't deliver out of order)");
    println!("  → Stream 3 data [S3:1] BLOCKED");
    println!("  → ALL streams affected by ONE lost packet\n");

    println!("── QUIC + HTTP/3 (independent streams) ───────────");
    println!("  Stream 1: [pkt1] [pkt2] [pkt3]");
    println!("  Stream 2: [pkt1] [pkt2-lost!] [pkt3]");
    println!("  Stream 3: [pkt1] [pkt2] [pkt3]");
    println!("  Stream 2 packet lost → only Stream 2 stalls");
    println!("  → Stream 1: UNAFFECTED (delivers pkt1, pkt2, pkt3)");
    println!("  → Stream 3: UNAFFECTED (delivers pkt1, pkt2, pkt3)");
    println!("  → Only the affected stream pays the retransmission cost\n");
}

fn main() {
    println!("QUIC and HTTP/3 Demonstration");
    println!("==============================\n");

    demo_handshake_comparison();
    println!();
    demo_stream_multiplexing();
    println!();
    demo_connection_migration();
    println!();
    demo_hol_blocking_comparison();
}
