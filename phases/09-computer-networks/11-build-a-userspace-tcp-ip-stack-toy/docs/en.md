# Build a userspace TCP/IP stack (toy)

> Build a userspace TCP/IP stack (toy) — the part of CS you can't skip.

**Type:** Build
**Languages:** Rust, C
**Prerequisites:** Phase 09 lessons 01–10
**Time:** ~120 minutes

## Learning Objectives

- Understand how the kernel's TCP/IP stack processes packets at each layer.
- Implement a minimal TCP/IP stack in userspace that can complete a 3-way handshake and transfer data.
- Compare your toy stack against production implementations (Linux, gVisor, Fuchsia).
- Ship a working TCP/IP stack you can extend for the phase capstone.

## The Problem

You know the theory: Ethernet frames carry IP packets, IP packets carry TCP segments, TCP provides reliable byte streams (Lessons 01–07). But the kernel's TCP/IP stack is thousands of lines of C — opaque and intimidating.

Building a userspace TCP/IP stack from scratch is the fastest way to *internalize* how each layer works. You'll parse real wire-format headers, compute real checksums, and implement a real TCP state machine. This is how Google engineers built gVisor's network stack, and how the Fuchsia project bootstrapped networking.

## The Concept

### Architecture

```
┌─────────────────────────────────────────┐
│           Your Application              │
│  (reads/writes data on TCP connections) │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│         TCP State Machine               │
│  LISTEN → SYN_RCVD → ESTABLISHED       │
│  Seq/ACK numbers, window, reassembly    │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│         IP Packet Processing            │
│  Version, TTL, checksum, fragmentation  │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│         Ethernet Frame Processing       │
│  src/dst MAC, EtherType, CRC            │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│         TUN/TAP Device                  │
│  Kernel ↔ Userspace packet bridge       │
└─────────────────────────────────────────┘
```

### TUN/TAP devices

A **TAP device** is a virtual network interface. Packets sent to it appear as readable data in userspace. You write raw Ethernet frames to it, and the kernel treats them as if they arrived on a real NIC. This is how VPNs (OpenVPN, WireGuard) work.

```bash
# Linux: create a TAP device
sudo ip tuntap add dev tap0 mode tap
sudo ip addr add 10.0.0.1/24 dev tap0
sudo ip link set tap0 up

# macOS: use utun (TUN only) or install tuntaposx
```

### Layer 1: Ethernet frame parsing

An Ethernet frame has a 14-byte header:

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        Destination MAC (6 bytes)              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           Source MAC (6 bytes)                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|        EtherType (0x0800 = IPv4)        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

Followed by: Payload (46–1500 bytes) + FCS (4 bytes, usually stripped)
```

### Layer 2: IPv4 packet parsing

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|Version|  IHL  |    DSCP/ECN   |         Total Length          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|         Identification        |Flags|     Fragment Offset     |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|  TTL  | Protocol (6=TCP)  |        Header Checksum            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                       Source IP Address                       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    Destination IP Address                     |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

The **IP checksum** is a 16-bit one's complement sum of the header. Set checksum field to 0 before computing.

### Layer 3: TCP segment parsing

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|          Source Port          |       Destination Port        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        Sequence Number                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    Acknowledgment Number                      |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Offset|Reserved |U|A|P|R|S|F|            Window               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           Checksum            |        Urgent Pointer         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

The **TCP checksum** uses a pseudo-header (src IP, dst IP, protocol, TCP length) prepended to the TCP segment.

### TCP state machine (simplified)

```
CLOSED → (recv SYN) → SYN_RCVD → (recv ACK) → ESTABLISHED
                                                       │
                                            (recv FIN) → CLOSE_WAIT
```

## Build It

### Step 1: Data structures

```rust
use std::collections::HashMap;
use std::io;
use std::os::unix::io::AsRawFd;

#[derive(Debug, Clone, Copy)]
struct EthernetFrame {
    dst_mac: [u8; 6],
    src_mac: [u8; 6],
    ether_type: u16,
}

#[derive(Debug, Clone, Copy)]
struct Ipv4Packet {
    version_ihl: u8,
    dscp_ecn: u8,
    total_length: u16,
    identification: u16,
    flags_fragment: u16,
    ttl: u8,
    protocol: u8,
    checksum: u16,
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
}

#[derive(Debug, Clone, Copy)]
struct TcpSegment {
    src_port: u16,
    dst_port: u16,
    seq: u32,
    ack: u32,
    data_offset: u8,
    flags: u8,
    window: u16,
    checksum: u16,
    urgent: u16,
}

const FIN: u8 = 0x01;
const SYN: u8 = 0x02;
const RST: u8 = 0x04;
const PSH: u8 = 0x08;
const ACK: u8 = 0x10;
```

### Step 2: Checksum computation

IP checksum: 16-bit one's complement sum of the header.
TCP checksum: same algorithm over pseudo-header + TCP segment + data.

```rust
fn checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i + 1 < data.len() {
        sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
        i += 2;
    }
    if i < data.len() {
        sum += (data[i] as u32) << 8;
    }
    while (sum >> 16) > 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !sum as u16
}

fn tcp_checksum(
    src_ip: &[u8; 4],
    dst_ip: &[u8; 4],
    tcp_segment: &[u8],
) -> u16 {
    // Pseudo-header: src_ip(4) + dst_ip(4) + zero(1) + protocol(1) + tcp_len(2)
    let mut pseudo = Vec::with_capacity(12 + tcp_segment.len());
    pseudo.extend_from_slice(src_ip);
    pseudo.extend_from_slice(dst_ip);
    pseudo.push(0);
    pseudo.push(6); // TCP protocol
    let tcp_len = (tcp_segment.len() as u16).to_be_bytes();
    pseudo.extend_from_slice(&tcp_len);
    pseudo.extend_from_slice(tcp_segment);
    checksum(&pseudo)
}
```

### Step 3: Parsing packets

```rust
fn parse_ethernet(data: &[u8]) -> Option<(EthernetFrame, &[u8])> {
    if data.len() < 14 {
        return None;
    }
    let frame = EthernetFrame {
        dst_mac: data[0..6].try_into().ok()?,
        src_mac: data[6..12].try_into().ok()?,
        ether_type: u16::from_be_bytes([data[12], data[13]]),
    };
    Some((frame, &data[14..]))
}

fn parse_ipv4(data: &[u8]) -> Option<(Ipv4Packet, &[u8])> {
    if data.len() < 20 {
        return None;
    }
    let ihl = (data[0] & 0x0F) as usize * 4;
    if data.len() < ihl {
        return None;
    }
    let pkt = Ipv4Packet {
        version_ihl: data[0],
        dscp_ecn: data[1],
        total_length: u16::from_be_bytes([data[2], data[3]]),
        identification: u16::from_be_bytes([data[4], data[5]]),
        flags_fragment: u16::from_be_bytes([data[6], data[7]]),
        ttl: data[8],
        protocol: data[9],
        checksum: u16::from_be_bytes([data[10], data[11]]),
        src_ip: data[12..16].try_into().ok()?,
        dst_ip: data[16..20].try_into().ok()?,
    };
    Some((pkt, &data[ihl..]))
}

fn parse_tcp(data: &[u8]) -> Option<(TcpSegment, &[u8])> {
    if data.len() < 20 {
        return None;
    }
    let data_offset = (data[12] >> 4) as usize * 4;
    if data.len() < data_offset {
        return None;
    }
    let seg = TcpSegment {
        src_port: u16::from_be_bytes([data[0], data[1]]),
        dst_port: u16::from_be_bytes([data[2], data[3]]),
        seq: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
        ack: u32::from_be_bytes([data[8], data[9], data[10], data[11]]),
        data_offset: data[12] >> 4,
        flags: data[13],
        window: u16::from_be_bytes([data[14], data[15]]),
        checksum: u16::from_be_bytes([data[16], data[17]]),
        urgent: u16::from_be_bytes([data[18], data[19]]),
    };
    Some((seg, &data[data_offset..]))
}
```

### Step 4: TCP state machine

```rust
#[derive(Debug, Clone, PartialEq)]
enum TcpState {
    Closed,
    Listen,
    SynRcvd,
    Established,
}

#[derive(Debug)]
struct TcpConnection {
    state: TcpState,
    local_port: u16,
    remote_port: u16,
    remote_ip: [u8; 4],
    local_seq: u32,
    remote_seq: u32,
    recv_buffer: Vec<u8>,
}

#[derive(Debug)]
struct TcpStack {
    connections: HashMap<(u16, [u8; 4], u16), TcpConnection>,
}

impl TcpStack {
    fn new() -> Self {
        TcpStack {
            connections: HashMap::new(),
        }
    }

    fn listen(&mut self, port: u16) {
        let key = (port, [0, 0, 0, 0], 0);
        self.connections.insert(key, TcpConnection {
            state: TcpState::Listen,
            local_port: port,
            remote_port: 0,
            remote_ip: [0; 4],
            local_seq: rand::random::<u32>(),
            remote_seq: 0,
            recv_buffer: Vec::new(),
        });
    }

    fn handle_segment(
        &mut self,
        ip: &Ipv4Packet,
        seg: &TcpSegment,
        data: &[u8],
    ) -> Option<Vec<u8>> {
        // Find matching connection: exact match first, then listen socket
        let exact_key = (seg.dst_port, ip.src_ip, seg.src_port);
        let listen_key = (seg.dst_port, [0, 0, 0, 0], 0);

        let is_listen = self.connections.get(&listen_key)
            .map(|c| c.state == TcpState::Listen)
            .unwrap_or(false);

        let key = if self.connections.contains_key(&exact_key) {
            exact_key
        } else if is_listen {
            listen_key
        } else {
            return None;
        };

        let conn = self.connections.get(&key)?;
        match conn.state {
            TcpState::Listen => {
                if seg.flags & SYN == 0 {
                    return None;
                }
                // Create new connection for this client
                let local_seq = rand::random::<u32>();
                let new_key = (seg.dst_port, ip.src_ip, seg.src_port);
                self.connections.insert(new_key, TcpConnection {
                    state: TcpState::SynRcvd,
                    local_port: seg.dst_port,
                    remote_port: seg.src_port,
                    remote_ip: ip.src_ip,
                    local_seq,
                    remote_seq: seg.seq.wrapping_add(1),
                    recv_buffer: Vec::new(),
                });
                // Build SYN-ACK response
                Some(build_syn_ack(
                    ip.dst_ip, ip.src_ip,
                    seg.dst_port, seg.src_port,
                    local_seq, seg.seq.wrapping_add(1),
                ))
            }
            TcpState::SynRcvd => {
                if seg.flags & ACK == 0 {
                    return None;
                }
                if seg.ack == self.connections[&key].local_seq.wrapping_add(1) {
                    let conn = self.connections.get_mut(&key).unwrap();
                    conn.state = TcpState::Established;
                    eprintln!("Connection ESTABLISHED: {}:{}", 
                              format_ip(&ip.src_ip), seg.src_port);
                }
                None
            }
            TcpState::Established => {
                if seg.flags & FIN != 0 {
                    // Remote side is closing
                    let conn = self.connections.get_mut(&key).unwrap();
                    conn.remote_seq = seg.seq.wrapping_add(1);
                    let ack = build_ack(
                        ip.dst_ip, ip.src_ip,
                        seg.dst_port, seg.src_port,
                        conn.local_seq, conn.remote_seq,
                    );
                    self.connections.remove(&key);
                    eprintln!("Connection CLOSED: {}:{}", 
                              format_ip(&ip.src_ip), seg.src_port);
                    return Some(ack);
                }
                if !data.is_empty() {
                    let conn = self.connections.get_mut(&key).unwrap();
                    conn.recv_buffer.extend_from_slice(data);
                    conn.remote_seq = seg.seq.wrapping_add(data.len() as u32);
                    // Send ACK
                    Some(build_ack(
                        ip.dst_ip, ip.src_ip,
                        seg.dst_port, seg.src_port,
                        conn.local_seq, conn.remote_seq,
                    ))
                } else if seg.flags & ACK != 0 {
                    None
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
```

### Step 5: Building response packets

```rust
fn format_ip(ip: &[u8; 4]) -> String {
    format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])
}

fn build_tcp_segment(
    src_port: u16,
    dst_port: u16,
    seq: u32,
    ack: u32,
    flags: u8,
    window: u16,
    payload: &[u8],
) -> Vec<u8> {
    let header_len: u8 = 20;
    let mut seg = vec![0u8; header_len as usize + payload.len()];
    seg[0..2].copy_from_slice(&src_port.to_be_bytes());
    seg[2..4].copy_from_slice(&dst_port.to_be_bytes());
    seg[4..8].copy_from_slice(&seq.to_be_bytes());
    seg[8..12].copy_from_slice(&ack.to_be_bytes());
    seg[12] = (header_len / 4) << 4;
    seg[13] = flags;
    seg[14..16].copy_from_slice(&window.to_be_bytes());
    // checksum placeholder (index 16-17) — computed later
    seg[18..20].copy_from_slice(&0u16.to_be_bytes());
    seg[header_len as usize..].copy_from_slice(payload);
    seg
}

fn build_ipv4_packet(
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    protocol: u8,
    payload: &[u8],
) -> Vec<u8> {
    let header_len = 20usize;
    let total_len = header_len + payload.len();
    let mut pkt = vec![0u8; total_len];
    pkt[0] = 0x45; // version=4, ihl=5
    pkt[1] = 0x00; // DSCP/ECN
    pkt[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
    pkt[4..6].copy_from_slice(&0u16.to_be_bytes()); // identification
    pkt[6..8].copy_from_slice(&0x4000u16.to_be_bytes()); // flags: DF
    pkt[8] = 64; // TTL
    pkt[9] = protocol;
    // pkt[10..12] = checksum (computed below)
    pkt[12..16].copy_from_slice(&src_ip);
    pkt[16..20].copy_from_slice(&dst_ip);
    pkt[header_len..].copy_from_slice(payload);
    // Compute IP header checksum
    let cksum = checksum(&pkt[..header_len]);
    pkt[10..12].copy_from_slice(&cksum.to_be_bytes());
    pkt
}

fn build_syn_ack(
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    src_port: u16,
    dst_port: u16,
    seq: u32,
    ack: u32,
) -> Vec<u8> {
    let mut seg = build_tcp_segment(src_port, dst_port, seq, ack, SYN | ACK, 65535, &[]);
    let cksum = tcp_checksum(&src_ip, &dst_ip, &seg);
    seg[16..18].copy_from_slice(&cksum.to_be_bytes());
    build_ipv4_packet(src_ip, dst_ip, 6, &seg)
}

fn build_ack(
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    src_port: u16,
    dst_port: u16,
    seq: u32,
    ack: u32,
) -> Vec<u8> {
    let mut seg = build_tcp_segment(src_port, dst_port, seq, ack, ACK, 65535, &[]);
    let cksum = tcp_checksum(&src_ip, &dst_ip, &seg);
    seg[16..18].copy_from_slice(&cksum.to_be_bytes());
    build_ipv4_packet(src_ip, dst_ip, 6, &seg)
}
```

### Step 6: Main loop — TAP device I/O

```rust
use std::fs::OpenOptions;

fn open_tap_device(path: &str) -> io::Result<std::fs::File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
}

fn main() -> io::Result<()> {
    let mut stack = TcpStack::new();
    stack.listen(8080);

    let tap_path = if cfg!(target_os = "linux") {
        "/dev/net/tun"
    } else {
        // macOS uses utun; simplified demo reads from a raw socket
        "/dev/tap0"
    };

    let mut tap = match open_tap_device(tap_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Cannot open {}: {}", tap_path, e);
            eprintln!("Run with: sudo ip tuntap add dev tap0 mode tap");
            eprintln!("          sudo ip link set tap0 up");
            eprintln!("          sudo ./main (or cargo run)");
            eprintln!();
            eprintln!("Running in demo mode (simulating packets)...");

            // Demo mode: simulate a SYN packet
            let syn_packet = build_syn_packet();
            demo_process(&mut stack, &syn_packet);
            return Ok(());
        }
    };

    eprintln!("TCP/IP stack listening on port 8080. Waiting for packets...");
    let mut buf = [0u8; 1518]; // max Ethernet frame

    loop {
        use std::io::Read;
        let n = match tap.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };

        if let Some(response) = process_frame(&mut stack, &buf[..n]) {
            use std::io::Write;
            let _ = tap.write_all(&response);
        }
    }
    Ok(())
}

fn process_frame(stack: &mut TcpStack, data: &[u8]) -> Option<Vec<u8>> {
    let (eth, payload) = parse_ethernet(data)?;
    if eth.ether_type != 0x0800 {
        return None; // Not IPv4
    }
    let (ip, ip_payload) = parse_ipv4(payload)?;
    if ip.protocol != 6 {
        return None; // Not TCP
    }
    let (tcp, tcp_data) = parse_tcp(ip_payload)?;
    let response_payload = stack.handle_segment(&ip, &tcp, tcp_data)?;

    // Wrap in Ethernet frame
    let mut frame = Vec::with_capacity(14 + response_payload.len());
    frame.extend_from_slice(&eth.src_mac); // reply to sender
    frame.extend_from_slice(&eth.dst_mac); // our MAC (swap)
    frame.extend_from_slice(&0x0800u16.to_be_bytes()); // EtherType IPv4
    frame.extend_from_slice(&response_payload);
    Some(frame)
}

fn demo_process(stack: &mut TcpStack, data: &[u8]) {
    let (eth, payload) = parse_ethernet(data).unwrap();
    let (ip, ip_payload) = parse_ipv4(payload).unwrap();
    let (tcp, tcp_data) = parse_tcp(ip_payload).unwrap();

    eprintln!("Got packet: {}:{} → {}:{} flags={:02b}",
        format_ip(&ip.src_ip), tcp.src_port,
        format_ip(&ip.dst_ip), tcp.dst_port,
        tcp.flags);

    if let Some(response) = stack.handle_segment(&ip, &tcp, tcp_data) {
        let (_, resp_payload) = parse_ethernet(&response).unwrap();
        let (resp_ip, resp_ip_payload) = parse_ipv4(resp_payload).unwrap();
        let (resp_tcp, _) = parse_tcp(resp_ip_payload).unwrap();
        eprintln!("Sent response: {}:{} → {}:{} flags={:02b}",
            format_ip(&resp_ip.src_ip), resp_tcp.src_port,
            format_ip(&resp_ip.dst_ip), resp_tcp.dst_port,
            resp_tcp.flags);
    }
}

fn build_syn_packet() -> Vec<u8> {
    // Simulate: client 10.0.0.2:54321 → server 10.0.0.1:8080, SYN
    let src_ip = [10, 0, 0, 2];
    let dst_ip = [10, 0, 0, 1];
    let mut tcp = build_tcp_segment(54321, 8080, 1000, 0, SYN, 65535, &[]);
    let cksum = tcp_checksum(&src_ip, &dst_ip, &tcp);
    tcp[16..18].copy_from_slice(&cksum.to_be_bytes());
    let ip_pkt = build_ipv4_packet(src_ip, dst_ip, 6, &tcp);

    let mut frame = Vec::with_capacity(14 + ip_pkt.len());
    frame.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]); // dst
    frame.extend_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]); // src
    frame.extend_from_slice(&0x0800u16.to_be_bytes());
    frame.extend_from_slice(&ip_pkt);
    frame
}
```

## Use It

Production TCP/IP stacks:

- **Linux kernel `net/ipv4/tcp_input.c`** — the reference TCP implementation. Handles millions of connections. Look at `tcp_v4_do_rcv()` for the main receive path.
- **gVisor (`pkg/tcpip/`)** — Google's Go userspace TCP/IP stack. Runs in containers. Uses the same layer separation (Ethernet → IP → TCP) but with Go's type safety.
- **Fuchsia netstack (`src/connectivity/network/`)** — Rust userspace stack. Similar architecture to what we built, with async I/O.

Your toy stack handles SYN → SYN-ACK → ACK and receives data. A production stack adds: retransmission timers, congestion control (Reno/Cubic/BBR), window scaling, SACK, MTU discovery, and connection teardown (FIN).

## Read the Source

- Linux `net/ipv4/tcp_input.c` — `tcp_v4_do_rcv()`: the core TCP receive handler showing how the real kernel processes segments.
- gVisor `pkg/tcpip/transport/tcp/` — Go userspace TCP with comparable state machine logic.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **An HTTP/2 server on a custom userspace TCP/IP stack** — the foundation for the phase capstone (Lesson 22).

## Exercises

1. **Easy** — Add an `RST` handler: when the stack receives a segment for a port with no listener, respond with RST. This is what the kernel does for closed ports.

2. **Medium** — Implement a simple HTTP responder: when a connection reaches ESTABLISHED and data arrives, parse it as an HTTP GET and respond with a fixed HTML page. You need to send data on the connection (not just ACKs).

3. **Hard** — Add a retransmission timer: store each outgoing segment with a timestamp. If no ACK arrives within 500ms, retransmit. Implement exponential backoff (double the timeout on each retransmission, up to 64 seconds). This is the core of TCP reliability.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| TUN/TAP | "TAP device" | Virtual network interface: TAP delivers Ethernet frames, TUN delivers IP packets |
| Pseudo-header | "TCP pseudo header" | Temporary header (src IP, dst IP, protocol, length) prepended for TCP checksum computation |
| Sequence number | "Seq number" | Byte offset of the first byte in this segment within the connection's byte stream |
| Acknowledgment number | "ACK number" | Next expected byte from the other side — everything before this has been received |
| SYN-ACK | "Synack" | TCP segment with both SYN and ACK flags set — the server's reply in the 3-way handshake |
| State machine | "TCP states" | The set of states (LISTEN, SYN_RCVD, ESTABLISHED, etc.) a connection transitions through |

## Further Reading

- [Cloudflare Blog: A Linux TCP stack in userspace](https://blog.cloudflare.com/) — real-world motivation for userspace stacks.
- [Jon Gjengset — Implementing TCP in Rust](https://www.youtube.com/watch?v=bzja9fQWzdA) — live-coding a TCP stack from scratch.
- RFC 793 — the TCP specification. Section 3: "Functional Specification" defines the state machine.
