//! Transport — UDP, TCP State Machine
//! Phase 09 — Computer Networks
//!
//! TCP state machine in Rust with three-way handshake
//! and four-way teardown simulation.
//! Run: rustc tcp_state.rs -o tcp_state && ./tcp_state

use std::fmt;

/// All 11 TCP states as defined in RFC 793 / RFC 9293.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynRcvd,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

impl fmt::Display for TcpState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TcpState::Closed    => "CLOSED",
            TcpState::Listen    => "LISTEN",
            TcpState::SynSent   => "SYN_SENT",
            TcpState::SynRcvd   => "SYN_RCVD",
            TcpState::Established => "ESTABLISHED",
            TcpState::FinWait1  => "FIN_WAIT_1",
            TcpState::FinWait2  => "FIN_WAIT_2",
            TcpState::CloseWait => "CLOSE_WAIT",
            TcpState::Closing   => "CLOSING",
            TcpState::LastAck   => "LAST_ACK",
            TcpState::TimeWait  => "TIME_WAIT",
        };
        write!(f, "{}", s)
    }
}

/// TCP events that drive state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TcpEvent {
    PassiveOpen,    // Server: listen()
    ActiveOpen,     // Client: connect()
    SendSyn,        // Send SYN segment
    RecvSyn,        // Receive SYN segment
    SendSynAck,     // Send SYN+ACK
    RecvSynAck,     // Receive SYN+ACK
    SendAck,        // Send ACK
    RecvAck,        // Receive ACK
    SendFin,        // Send FIN
    RecvFin,        // Receive FIN
    Timeout,        // Timer expired (TIME_WAIT)
}

impl fmt::Display for TcpEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TcpEvent::PassiveOpen => "PASSIVE_OPEN",
            TcpEvent::ActiveOpen  => "ACTIVE_OPEN",
            TcpEvent::SendSyn     => "SEND_SYN",
            TcpEvent::RecvSyn     => "RECV_SYN",
            TcpEvent::SendSynAck  => "SEND_SYN_ACK",
            TcpEvent::RecvSynAck  => "RECV_SYN_ACK",
            TcpEvent::SendAck     => "SEND_ACK",
            TcpEvent::RecvAck     => "RECV_ACK",
            TcpEvent::SendFin     => "SEND_FIN",
            TcpEvent::RecvFin     => "RECV_FIN",
            TcpEvent::Timeout     => "TIMEOUT",
        };
        write!(f, "{}", s)
    }
}

/// A TCP connection tracking state, sequence numbers, and window.
#[derive(Debug)]
struct TcpConnection {
    state: TcpState,
    snd_una: u32,   // Oldest unacknowledged sequence number
    snd_nxt: u32,   // Next sequence number to send
    rcv_nxt: u32,   // Next sequence number expected
    window: u16,    // Receive window size
    role: Role,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Role {
    Client,
    Server,
}

impl TcpConnection {
    fn new(role: Role) -> Self {
        TcpConnection {
            state: TcpState::Closed,
            snd_una: 0,
            snd_nxt: 0,
            rcv_nxt: 0,
            window: 65535,
            role,
        }
    }

    /// Drive the state machine: (current_state, event) → next_state.
    fn handle_event(&mut self, event: TcpEvent) {
        let prev = self.state;

        self.state = match (self.state, event) {
            // ── Connection establishment (client) ──
            (TcpState::Closed, TcpEvent::ActiveOpen) => {
                self.snd_una = 1000; // arbitrary ISN
                self.snd_nxt = 1001; // SYN consumes one sequence number
                TcpState::SynSent
            }
            (TcpState::SynSent, TcpEvent::RecvSynAck) => {
                self.rcv_nxt = 501; // peer's ISN + 1
                TcpState::Established
            }

            // ── Connection establishment (server) ──
            (TcpState::Closed, TcpEvent::PassiveOpen) => TcpState::Listen,
            (TcpState::Listen, TcpEvent::RecvSyn) => {
                self.rcv_nxt = 1001; // client's ISN + 1
                self.snd_una = 500;  // server's ISN
                self.snd_nxt = 501;  // SYN consumes one
                TcpState::SynRcvd
            }
            (TcpState::SynRcvd, TcpEvent::RecvAck) => {
                self.snd_una = self.snd_nxt;
                TcpState::Established
            }

            // ── Connection teardown (active close) ──
            (TcpState::Established, TcpEvent::SendFin) => {
                self.snd_nxt += 1; // FIN consumes one sequence number
                TcpState::FinWait1
            }
            (TcpState::FinWait1, TcpEvent::RecvAck) => TcpState::FinWait2,
            (TcpState::FinWait2, TcpEvent::RecvFin) => {
                self.rcv_nxt += 1;
                TcpState::TimeWait
            }
            (TcpState::TimeWait, TcpEvent::Timeout) => TcpState::Closed,

            // Simultaneous close: both sides send FIN
            (TcpState::FinWait1, TcpEvent::RecvFin) => {
                self.rcv_nxt += 1;
                TcpState::Closing
            }
            (TcpState::Closing, TcpEvent::RecvAck) => TcpState::TimeWait,

            // ── Connection teardown (passive close) ──
            (TcpState::Established, TcpEvent::RecvFin) => {
                self.rcv_nxt += 1;
                TcpState::CloseWait
            }
            (TcpState::CloseWait, TcpEvent::SendFin) => {
                self.snd_nxt += 1;
                TcpState::LastAck
            }
            (TcpState::LastAck, TcpEvent::RecvAck) => TcpState::Closed,

            // ── Invalid transitions ──
            (state, event) => {
                panic!(
                    "Invalid transition: state={}, event={}. \
                     See RFC 793 Section 3.4 for valid transitions.",
                    state, event
                );
            }
        };

        println!(
            "  [{:?}] {} + {} → {}  (snd_una={}, snd_nxt={}, rcv_nxt={})",
            self.role, prev, event, self.state,
            self.snd_una, self.snd_nxt, self.rcv_nxt
        );
    }
}

/// Simulate a full three-way handshake.
fn three_way_handshake() {
    println!("═══════════════════════════════════════");
    println!("  THREE-WAY HANDSHAKE SIMULATION");
    println!("═══════════════════════════════════════\n");

    let mut client = TcpConnection::new(Role::Client);
    let mut server = TcpConnection::new(Role::Server);

    println!("Initial state: client={}, server={}\n", client.state, server.state);

    // Step 1: Server does passive open
    println!("── Step 0: Server calls listen()");
    server.handle_event(TcpEvent::PassiveOpen);

    // Step 2: Client does active open (sends SYN)
    println!("\n── Step 1: Client calls connect() → sends SYN (seq=1000)");
    client.handle_event(TcpEvent::ActiveOpen);

    // Step 3: Server receives SYN, sends SYN+ACK
    println!("\n── Step 2: Server receives SYN, sends SYN+ACK (seq=500, ack=1001)");
    server.handle_event(TcpEvent::RecvSyn);

    // Step 4: Client receives SYN+ACK, sends ACK
    println!("\n── Step 3: Client receives SYN+ACK, sends ACK (ack=501)");
    client.handle_event(TcpEvent::RecvSynAck);

    // Step 5: Server receives ACK
    println!("\n── Step 4: Server receives ACK");
    server.handle_event(TcpEvent::RecvAck);

    println!("\nResult: client={}, server={}\n", client.state, server.state);
}

/// Simulate a full four-way teardown.
fn four_way_teardown() {
    println!("═══════════════════════════════════════");
    println!("  FOUR-WAY TEARDOWN SIMULATION");
    println!("═══════════════════════════════════════\n");

    // Start from ESTABLISHED
    let mut client = TcpConnection::new(Role::Client);
    let mut server = TcpConnection::new(Role::Server);

    // Fast-forward through handshake
    client.handle_event(TcpEvent::ActiveOpen);
    client.handle_event(TcpEvent::RecvSynAck);
    server.handle_event(TcpEvent::PassiveOpen);
    server.handle_event(TcpEvent::RecvSyn);
    server.handle_event(TcpEvent::RecvAck);

    println!("Starting teardown from ESTABLISHED...\n");

    // Step 1: Client sends FIN
    println!("── Step 1: Client sends FIN");
    client.handle_event(TcpEvent::SendFin);

    // Step 2: Server receives FIN, sends ACK
    println!("\n── Step 2: Server receives FIN, sends ACK");
    server.handle_event(TcpEvent::RecvFin);

    // Step 3: Client receives ACK for FIN
    println!("\n── Step 3: Client receives ACK for its FIN");
    client.handle_event(TcpEvent::RecvAck);

    // Step 4: Server sends FIN (application calls close)
    println!("\n── Step 4: Server sends FIN");
    server.handle_event(TcpEvent::SendFin);

    // Step 5: Client receives FIN, sends ACK
    println!("\n── Step 5: Client receives FIN, sends ACK");
    client.handle_event(TcpEvent::RecvFin);

    // Step 6: TIME_WAIT timeout
    println!("\n── Step 6: Client TIME_WAIT timeout (2×MSL)");
    client.handle_event(TcpEvent::Timeout);

    // Step 7: Server receives ACK
    println!("\n── Step 7: Server receives ACK for its FIN");
    server.handle_event(TcpEvent::RecvAck);

    println!("\nResult: client={}, server={}\n", client.state, server.state);
}

/// Print a visual state diagram.
fn print_state_diagram() {
    println!("═══════════════════════════════════════");
    println!("  TCP STATE DIAGRAM");
    println!("═══════════════════════════════════════\n");

    println!(
r#"  CLOSED
    │
    ├── passive open ──→ LISTEN ──recv SYN/send SYN+ACK──→ SYN_RCVD
    │                                                    └──recv ACK──→ ESTABLISHED
    │
    ├── active open ──→ SYN_SENT ──recv SYN+ACK/send ACK──→ ESTABLISHED
    │
    ESTABLISHED
    │
    ├── send FIN ──→ FIN_WAIT_1 ──recv ACK──→ FIN_WAIT_2
    │                                      └──recv FIN/send ACK──→ TIME_WAIT
    │                                                            └──timeout──→ CLOSED
    │
    ├── recv FIN/send ACK ──→ CLOSE_WAIT ──send FIN──→ LAST_ACK
    │                                               └──recv ACK──→ CLOSED
    │
    └── (simultaneous close)
        send FIN from FIN_WAIT_1 ──recv FIN──→ CLOSING ──recv ACK──→ TIME_WAIT
"#
    );
}

fn main() {
    print_state_diagram();
    three_way_handshake();
    four_way_teardown();
}
