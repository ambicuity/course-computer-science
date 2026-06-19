# Build a User-Space TCP/IP Stack

> Network stacks are finite-state machines plus careful byte-level discipline.

**Type:** Build
**Languages:** Rust, C
**Prerequisites:** Phase 19 lessons 01-04
**Time:** ~720 minutes

## Learning Objectives

- Decompose packet parsing, connection state, and retransmission logic.
- Implement minimal header encode/decode workflow.
- Model a reduced TCP state progression for learning.
- Define validation checks for packet correctness and state safety.

## The Problem

Full TCP/IP is large. RFC 793 alone is 85 pages. Add IPv4 (RFC 791), ARP (RFC 826), Ethernet framing, checksums, retransmission, congestion control, and you have thousands of pages of specifications. Capstone success requires a reduced but coherent scope.

The key insight: a TCP/IP stack is layered, and each layer has a clear contract with the layer above and below. The Ethernet layer doesn't know about TCP connections. The IP layer doesn't know about byte streams. The TCP layer doesn't know about physical wires. This separation means you can build and test each layer independently.

A parser + state machine + checksum path gives concrete progress without full wire compatibility at first. The first milestone: parse and construct IPv4 and TCP headers from raw bytes. The second: implement the TCP handshake state machine (SYN, SYN-ACK, ACK). The third: send and receive data segments with sequence number tracking.

## The Concept

The TCP/IP stack is a layered architecture:

```
Application (HTTP, SSH, etc.)
      │
      ▼
┌──────────────┐
│  TCP layer   │  Connection state, reliability, ordering
│  (port-based)│  Segments: src_port, dst_port, seq, ack, flags
└──────────────┘
      │
      ▼
┌──────────────┐
│  IP layer    │  Addressing, routing
│  (host-based)│  Packets: src_ip, dst_ip, protocol, TTL
└──────────────┘
      │
      ▼
┌──────────────┐
│  Ethernet    │  Local network framing
│  (MAC-based) │  Frames: dst_mac, src_mac, ethertype, payload
└──────────────┘
```

Each layer adds a header, passes the result down, and reverses the process on the receive side. The TCP layer is the most complex: it maintains per-connection state (the state machine), ensures reliable delivery (retransmission), handles flow control (window sizing), and presents an ordered byte stream to the application.

The TCP state machine has 11 states. The most important transitions for a minimal stack:

```
CLOSED → SYN_SENT → ESTABLISHED → FIN_WAIT_1 → FIN_WAIT_2 → TIME_WAIT → CLOSED
                                       ↕
LISTEN → SYN_RCVD → ESTABLISHED → CLOSE_WAIT → LAST_ACK → CLOSED
```

A connection starts in CLOSED. The client sends SYN, transitions to SYN_SENT. The server receives SYN, sends SYN-ACK, transitions to SYN_RCVD. The client receives SYN-ACK, sends ACK, transitions to ESTABLISHED. The server receives ACK, transitions to ESTABLISHED. Both sides can now send data.

## Build It

We implement packet parsing, the TCP handshake state machine, and basic data transfer in Rust.

### Step 1: IPv4 and TCP Header Parsing

```rust
use std::fmt;

// IPv4 header (20 bytes, no options)
#[derive(Debug, Clone)]
struct IPv4Header {
    version: u8,        // 4
    ihl: u8,            // Header length in 32-bit words (usually 5)
    total_length: u16,
    protocol: u8,       // 6 = TCP
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
}

impl IPv4Header {
    fn parse(data: &[u8]) -> Result<(Self, &[u8]), String> {
        if data.len() < 20 {
            return Err("IPv4 header too short".into());
        }
        let version_ihl = data[0];
        let version = version_ihl >> 4;
        let ihl = version_ihl & 0x0F;
        if version != 4 {
            return Err(format!("Not IPv4: version={}", version));
        }
        let header_len = (ihl as usize) * 4;
        if data.len() < header_len {
            return Err("IPv4 header truncated".into());
        }

        Ok((IPv4Header {
            version,
            ihl,
            total_length: u16::from_be_bytes([data[2], data[3]]),
            protocol: data[9],
            src_ip: [data[12], data[13], data[14], data[15]],
            dst_ip: [data[16], data[17], data[18], data[19]],
        }, &data[header_len..]))
    }

    fn serialize(&self, payload: &[u8]) -> Vec<u8> {
        let total_len = 20 + payload.len();
        let mut buf = vec![0u8; 20];
        buf[0] = (4 << 4) | 5; // version=4, ihl=5
        buf[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
        buf[8] = 64; // TTL
        buf[9] = self.protocol;
        buf[12..16].copy_from_slice(&self.src_ip);
        buf[16..20].copy_from_slice(&self.dst_ip);
        // Compute checksum over header
        let checksum = ipv4_checksum(&buf);
        buf[10..12].copy_from_slice(&checksum.to_be_bytes());
        buf.extend_from_slice(payload);
        buf
    }
}

fn ipv4_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    for chunk in data.chunks(2) {
        let word = if chunk.len() == 2 {
            u16::from_be_bytes([chunk[0], chunk[1]]) as u32
        } else {
            (chunk[0] as u32) << 8
        };
        sum += word;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !sum as u16
}

// TCP header (20 bytes, no options)
#[derive(Debug, Clone)]
struct TCPHeader {
    src_port: u16,
    dst_port: u16,
    seq_num: u32,
    ack_num: u32,
    data_offset: u8, // Header length in 32-bit words
    flags: u8,       // SYN, ACK, FIN, RST, etc.
    window: u16,
}

const TCP_SYN: u8 = 0x02;
const TCP_ACK: u8 = 0x10;
const TCP_FIN: u8 = 0x01;
const TCP_RST: u8 = 0x04;

impl TCPHeader {
    fn parse(data: &[u8]) -> Result<(Self, &[u8]), String> {
        if data.len() < 20 {
            return Err("TCP header too short".into());
        }
        let data_offset = (data[12] >> 4) & 0x0F;
        let header_len = (data_offset as usize) * 4;
        if data.len() < header_len {
            return Err("TCP header truncated".into());
        }

        Ok((TCPHeader {
            src_port: u16::from_be_bytes([data[0], data[1]]),
            dst_port: u16::from_be_bytes([data[2], data[3]]),
            seq_num: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
            ack_num: u32::from_be_bytes([data[8], data[9], data[10], data[11]]),
            data_offset,
            flags: data[13],
            window: u16::from_be_bytes([data[14], data[15]]),
        }, &data[header_len..]))
    }

    fn serialize(&self, payload: &[u8]) -> Vec<u8> {
        let header_len: usize = 20;
        let mut buf = vec![0u8; header_len];
        buf[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        buf[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        buf[4..8].copy_from_slice(&self.seq_num.to_be_bytes());
        buf[8..12].copy_from_slice(&self.ack_num.to_be_bytes());
        buf[12] = (5 << 4); // data offset = 5 words
        buf[13] = self.flags;
        buf[14..16].copy_from_slice(&self.window.to_be_bytes());
        buf.extend_from_slice(payload);
        buf
    }

    fn has_flag(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }
}
```

### Step 2: TCP Connection State Machine

```rust
#[derive(Debug, Clone, PartialEq)]
enum TCPState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    LastAck,
    TimeWait,
}

struct TCPConnection {
    state: TCPState,
    local_port: u16,
    remote_port: u16,
    local_seq: u32,
    remote_seq: u32,
    recv_buffer: Vec<u8>,
    send_buffer: Vec<u8>,
}

impl TCPConnection {
    fn new(local_port: u16) -> Self {
        TCPConnection {
            state: TCPState::Listen,
            local_port,
            remote_port: 0,
            local_seq: 1000, // Initial sequence number
            remote_seq: 0,
            recv_buffer: Vec::new(),
            send_buffer: Vec::new(),
        }
    }

    // Process an incoming TCP segment
    fn process_segment(&mut self, header: &TCPHeader, payload: &[u8]) -> Option<TCPHeader> {
        match self.state {
            TCPState::Listen => {
                if header.has_flag(TCP_SYN) {
                    self.remote_port = header.src_port;
                    self.remote_seq = header.seq_num;
                    self.state = TCPState::SynReceived;
                    self.local_seq = 1000; // Our initial seq
                    println!("  [LISTEN → SYN_RCVD] Received SYN, seq={}", header.seq_num);
                    // Send SYN-ACK
                    Some(TCPHeader {
                        src_port: self.local_port,
                        dst_port: self.remote_port,
                        seq_num: self.local_seq,
                        ack_num: header.seq_num.wrapping_add(1),
                        data_offset: 5,
                        flags: TCP_SYN | TCP_ACK,
                        window: 65535,
                    })
                } else {
                    None
                }
            }
            TCPState::SynReceived => {
                if header.has_flag(TCP_ACK) {
                    self.state = TCPState::Established;
                    self.remote_seq = header.seq_num;
                    println!("  [SYN_RCVD → ESTABLISHED] Received ACK, handshake complete");
                    None
                } else {
                    None
                }
            }
            TCPState::Established => {
                if header.has_flag(TCP_FIN) {
                    self.state = TCPState::CloseWait;
                    self.remote_seq = header.seq_num;
                    println!("  [ESTABLISHED → CLOSE_WAIT] Received FIN");
                    // Send ACK for FIN
                    Some(TCPHeader {
                        src_port: self.local_port,
                        dst_port: self.remote_port,
                        seq_num: self.local_seq,
                        ack_num: header.seq_num.wrapping_add(1),
                        data_offset: 5,
                        flags: TCP_ACK,
                        window: 65535,
                    })
                } else if !payload.is_empty() {
                    self.recv_buffer.extend_from_slice(payload);
                    self.remote_seq = header.seq_num;
                    println!("  [ESTABLISHED] Received {} bytes, total buffered: {}",
                             payload.len(), self.recv_buffer.len());
                    // Send ACK
                    Some(TCPHeader {
                        src_port: self.local_port,
                        dst_port: self.remote_port,
                        seq_num: self.local_seq,
                        ack_num: header.seq_num.wrapping_add(payload.len() as u32),
                        data_offset: 5,
                        flags: TCP_ACK,
                        window: 65535,
                    })
                } else if header.has_flag(TCP_ACK) {
                    // Pure ACK, no action needed
                    None
                } else {
                    None
                }
            }
            _ => {
                println!("  [{}] Unhandled segment in state {:?}", self.local_port, self.state);
                None
            }
        }
    }
}
```

### Step 3: Handshake Simulation

```rust
fn main() {
    println!("=== TCP Handshake Simulation ===\n");

    // Server starts listening
    let mut server = TCPConnection::new(80);
    println!("Server: listening on port 80");

    // Client sends SYN
    let syn = TCPHeader {
        src_port: 54321,
        dst_port: 80,
        seq_num: 1000,
        ack_num: 0,
        data_offset: 5,
        flags: TCP_SYN,
        window: 65535,
    };
    println!("Client: sending SYN, seq={}", syn.seq_num);

    // Server processes SYN, responds with SYN-ACK
    let syn_ack = server.process_segment(&syn, &[]).unwrap();
    println!("Server: sending SYN-ACK, seq={}, ack={}", syn_ack.seq_num, syn_ack.ack_num);

    // Client processes SYN-ACK, sends ACK
    println!("Client: received SYN-ACK, sending ACK");
    let ack = TCPHeader {
        src_port: 54321,
        dst_port: 80,
        seq_num: 1001,
        ack_num: syn_ack.seq_num.wrapping_add(1),
        data_offset: 5,
        flags: TCP_ACK,
        window: 65535,
    };

    // Server processes ACK, connection established
    server.process_segment(&ack, &[]);
    println!("Server: state = {:?}", server.state);

    // Send data
    println!("\n=== Data Transfer ===\n");
    let data = b"Hello, TCP!";
    let data_seg = TCPHeader {
        src_port: 54321,
        dst_port: 80,
        seq_num: 1001,
        ack_num: syn_ack.seq_num.wrapping_add(1),
        data_offset: 5,
        flags: TCP_ACK,
        window: 65535,
    };
    println!("Client: sending {} bytes of data", data.len());
    let data_ack = server.process_segment(&data_seg, data).unwrap();
    println!("Server: ACK'd {} bytes, ack_num={}", data.len(), data_ack.ack_num);
    println!("Server: buffered data: {:?}", String::from_utf8_lossy(&server.recv_buffer));
}
```

Expected output:

```
=== TCP Handshake Simulation ===

Server: listening on port 80
Client: sending SYN, seq=1000
  [LISTEN → SYN_RCVD] Received SYN, seq=1000
Server: sending SYN-ACK, seq=1000, ack=1001
Client: received SYN-ACK, sending ACK
  [SYN_RCVD → ESTABLISHED] Received ACK, handshake complete
Server: state = Established

=== Data Transfer ===

Client: sending 11 bytes of data
  [ESTABLISHED] Received 11 bytes, total buffered: 11
Server: ACK'd 11 bytes, ack_num=1012
Server: buffered data: "Hello, TCP!"
```

## Use It

User-space stacks appear in high-performance networking, testing harnesses, and specialized dataplane systems:

- **Linux kernel TCP**: the definitive reference implementation. `net/ipv4/tcp_input.c` handles segment processing, `net/ipv4/tcp_output.c` handles segment construction, and `net/ipv4/tcp.c` handles the socket API. The state machine is in `tcp_input.c` with the `tcp_rcv_state_process` function.
- **DPDK and io_uring**: high-performance user-space networking frameworks that bypass the kernel. Applications implement their own TCP stacks on top of raw packet I/O for latency-sensitive workloads.
- **smoltcp**: a Rust user-space TCP/IP stack designed for embedded systems. No heap allocation, no threads, deterministic behavior. It's a production-quality version of what we're building.
- **lwIP**: a lightweight TCP/IP stack for embedded systems. Used in FreeRTOS, Zephyr, and many IoT devices. Implements the full TCP state machine with configurable features (timestamps, window scaling, etc.).

The key production lesson: **the TCP state machine is the hardest part to get right**. Every RFC clarification, every edge case (simultaneous open, simultaneous close, RST during handshake, TIME_WAIT assassination) adds states and transitions. Production stacks have thousands of lines dedicated to handling these edge cases.

## Read the Source

- [RFC 793](https://www.rfc-editor.org/rfc/rfc793) — The TCP specification. Section 3 (sequence numbers) and section 3.9 (event processing) define the state machine.
- [TCP/IP Illustrated, Volume 1](https://www.kohala.com/start/tcpipiv1.html) — Stevens. The definitive reference for understanding TCP on the wire. Chapter 18 (TCP connection establishment) is directly relevant.
- [smoltcp source](https://github.com/smoltcp-rs/smoltcp) — A production Rust user-space TCP/IP stack. Compare its socket API and state machine with ours.

## Ship It

- `code/main.rs`: TCP header parsing, IPv4 header parsing, TCP state machine with handshake simulation and data transfer.
- `code/main.c`: equivalent C implementation with struct-based header parsing.
- `outputs/README.md`: stack milestone checklist covering packet parsing, state machine, checksums, and data transfer.

## Exercises

1. **Easy** — Add retransmission timeout handling. Track the time since the last ACK for outstanding data. If no ACK arrives within a timeout (e.g., 1 second), retransmit the unacknowledged segments. Use a simple exponential backoff (double the timeout on each retransmission).
2. **Medium** — Add receive-window bookkeeping. Track the advertised receive window (how much buffer space the receiver has). The sender must not send more data than the receiver's window allows. Update the window in ACK segments as the application consumes data from the buffer.
3. **Hard** — Add malformed packet rejection tests. Write test cases for packets with invalid checksums, out-of-window sequence numbers, incorrect flag combinations (SYN+FIN), and header truncation. Each test should verify that the stack rejects the packet with the appropriate RST or silent discard.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| TCP state | "connection phase" | The finite-state position of a connection in its lifecycle (SYN_SENT, ESTABLISHED, etc.). Each state has specific valid incoming segments and valid outgoing responses. |
| Checksum | "integrity field" | A 16-bit value computed over the TCP pseudo-header, header, and payload. The receiver recomputes and compares. Mismatched checksums mean the segment was corrupted in transit. |
| MSS | "segment size cap" | Maximum Segment Size: the largest payload a TCP segment can carry. Negotiated during the handshake (SYN option). Typically 1460 bytes (1500 MTU minus 20 IP minus 20 TCP). |
| Retransmission | "resend" | When an ACK doesn't arrive within the retransmission timeout (RTO), the sender resends the unacknowledged data. The RTO adapts based on measured round-trip time (RTT). |
| Sequence number | "byte position" | A 32-bit counter that identifies each byte in the TCP byte stream. The receiver uses sequence numbers to reassemble data in order and detect duplicates. |

## Further Reading

- [RFC 793](https://www.rfc-editor.org/rfc/rfc793) — The original TCP specification.
- [TCP/IP Illustrated](https://www.kohala.com/start/tcpipiv1.html) — Stevens. Volume 1 covers the protocol on the wire; Volume 2 covers the implementation.
- [smoltcp](https://github.com/smoltcp-rs/smoltcp) — A Rust user-space TCP/IP stack for embedded systems.
