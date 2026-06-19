//! main.rs — Toy userspace TCP/IP stack
//! Phase 09 — Computer Networks, Lesson 11
//!
//! Parses Ethernet → IPv4 → TCP, implements a minimal TCP state machine,
//! handles 3-way handshake (SYN → SYN-ACK → ACK) and data reception.
//! Uses TUN/TAP device for packet I/O, or falls back to demo mode.
//!
//! Build:  cargo build --release
//!         (or: rustc main.rs -o tcp_stack)
//! Run:    sudo ./target/release/tcp_stack
//!         (needs /dev/net/tun or /dev/tap0)
//! Demo:   ./target/release/tcp_stack  (no privileges needed)

use std::collections::HashMap;
use std::io::{self, Read, Write};

// ─── Ethernet ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct EthernetFrame {
    dst_mac: [u8; 6],
    src_mac: [u8; 6],
    ether_type: u16,
}

fn parse_ethernet(data: &[u8]) -> Option<(EthernetFrame, &[u8])> {
    if data.len() < 14 {
        return None;
    }
    Some((
        EthernetFrame {
            dst_mac: data[0..6].try_into().ok()?,
            src_mac: data[6..12].try_into().ok()?,
            ether_type: u16::from_be_bytes([data[12], data[13]]),
        },
        &data[14..],
    ))
}

// ─── IPv4 ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct Ipv4Packet {
    total_length: u16,
    ttl: u8,
    protocol: u8,
    checksum: u16,
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    header_len: usize,
}

fn parse_ipv4(data: &[u8]) -> Option<(Ipv4Packet, &[u8])> {
    if data.len() < 20 {
        return None;
    }
    let ihl = (data[0] & 0x0F) as usize * 4;
    if data.len() < ihl {
        return None;
    }
    Some((
        Ipv4Packet {
            total_length: u16::from_be_bytes([data[2], data[3]]),
            ttl: data[8],
            protocol: data[9],
            checksum: u16::from_be_bytes([data[10], data[11]]),
            src_ip: data[12..16].try_into().ok()?,
            dst_ip: data[16..20].try_into().ok()?,
            header_len: ihl,
        },
        &data[ihl..],
    ))
}

fn build_ipv4_packet(src_ip: [u8; 4], dst_ip: [u8; 4], protocol: u8, payload: &[u8]) -> Vec<u8> {
    let header_len = 20usize;
    let total_len = header_len + payload.len();
    let mut pkt = vec![0u8; total_len];
    pkt[0] = 0x45; // version=4, ihl=5
    pkt[1] = 0x00;
    pkt[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
    pkt[6..8].copy_from_slice(&0x4000u16.to_be_bytes()); // flags: DF
    pkt[8] = 64; // TTL
    pkt[9] = protocol;
    pkt[12..16].copy_from_slice(&src_ip);
    pkt[16..20].copy_from_slice(&dst_ip);
    pkt[header_len..].copy_from_slice(payload);
    let cksum = ip_checksum(&pkt[..header_len]);
    pkt[10..12].copy_from_slice(&cksum.to_be_bytes());
    pkt
}

// ─── TCP ─────────────────────────────────────────────────────────────────────

const FIN: u8 = 0x01;
const SYN: u8 = 0x02;
const RST: u8 = 0x04;
const ACK: u8 = 0x10;

#[derive(Debug, Clone, Copy)]
struct TcpSegment {
    src_port: u16,
    dst_port: u16,
    seq: u32,
    ack: u32,
    data_offset: u8,
    flags: u8,
    window: u16,
}

fn parse_tcp(data: &[u8]) -> Option<(TcpSegment, &[u8])> {
    if data.len() < 20 {
        return None;
    }
    let doff = (data[12] >> 4) as usize * 4;
    if data.len() < doff {
        return None;
    }
    Some((
        TcpSegment {
            src_port: u16::from_be_bytes([data[0], data[1]]),
            dst_port: u16::from_be_bytes([data[2], data[3]]),
            seq: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
            ack: u32::from_be_bytes([data[8], data[9], data[10], data[11]]),
            data_offset: data[12] >> 4,
            flags: data[13],
            window: u16::from_be_bytes([data[14], data[15]]),
        },
        &data[doff..],
    ))
}

// ─── Checksums ───────────────────────────────────────────────────────────────

fn ip_checksum(data: &[u8]) -> u16 {
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

fn tcp_checksum(src_ip: &[u8; 4], dst_ip: &[u8; 4], tcp_segment: &[u8]) -> u16 {
    let mut pseudo = Vec::with_capacity(12 + tcp_segment.len());
    pseudo.extend_from_slice(src_ip);
    pseudo.extend_from_slice(dst_ip);
    pseudo.push(0);
    pseudo.push(6); // TCP
    let tcp_len = (tcp_segment.len() as u16).to_be_bytes();
    pseudo.extend_from_slice(&tcp_len);
    pseudo.extend_from_slice(tcp_segment);
    ip_checksum(&pseudo)
}

// ─── Packet construction ─────────────────────────────────────────────────────

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
    seg[header_len as usize..].copy_from_slice(payload);
    seg
}

fn wrap_ip_tcp(
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    src_port: u16,
    dst_port: u16,
    seq: u32,
    ack: u32,
    flags: u8,
    payload: &[u8],
) -> Vec<u8> {
    let mut tcp = build_tcp_segment(src_port, dst_port, seq, ack, flags, 65535, payload);
    let cksum = tcp_checksum(&src_ip, &dst_ip, &tcp);
    tcp[16..18].copy_from_slice(&cksum.to_be_bytes());
    build_ipv4_packet(src_ip, dst_ip, 6, &tcp)
}

// ─── TCP State Machine ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum TcpState {
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

struct TcpStack {
    connections: HashMap<(u16, [u8; 4], u16), TcpConnection>,
    listeners: HashMap<u16, TcpConnection>,
}

impl TcpStack {
    fn new() -> Self {
        TcpStack {
            connections: HashMap::new(),
            listeners: HashMap::new(),
        }
    }

    fn listen(&mut self, port: u16) {
        self.listeners.insert(port, TcpConnection {
            state: TcpState::Listen,
            local_port: port,
            remote_port: 0,
            remote_ip: [0; 4],
            local_seq: pseudo_random(),
            remote_seq: 0,
            recv_buffer: Vec::new(),
        });
        eprintln!("[*] Listening on port {}", port);
    }

    fn handle_segment(
        &mut self,
        ip: &Ipv4Packet,
        seg: &TcpSegment,
        data: &[u8],
    ) -> Option<Vec<u8>> {
        let conn_key = (seg.dst_port, ip.src_ip, seg.src_port);

        // Check existing connection first
        if let Some(conn) = self.connections.get_mut(&conn_key) {
            return self.handle_established(conn, ip, seg, data);
        }

        // Check if there's a listener for this port
        if let Some(listener) = self.listeners.get(&seg.dst_port) {
            if listener.state == TcpState::Listen && (seg.flags & SYN) != 0 {
                return self.handle_syn(ip, seg);
            }
        }

        // Unknown connection, non-SYN → RST
        if seg.flags & SYN == 0 && seg.flags & RST == 0 {
            return Some(wrap_ip_tcp(
                ip.dst_ip, ip.src_ip,
                seg.dst_port, seg.src_port,
                0, seg.seq.wrapping_add(1),
                RST | ACK, &[],
            ));
        }

        None
    }

    fn handle_syn(&mut self, ip: &Ipv4Packet, seg: &TcpSegment) -> Option<Vec<u8>> {
        let local_seq = pseudo_random();
        let remote_seq = seg.seq.wrapping_add(1);
        let conn_key = (seg.dst_port, ip.src_ip, seg.src_port);

        self.connections.insert(conn_key, TcpConnection {
            state: TcpState::SynRcvd,
            local_port: seg.dst_port,
            remote_port: seg.src_port,
            remote_ip: ip.src_ip,
            local_seq,
            remote_seq,
            recv_buffer: Vec::new(),
        });

        eprintln!("[+] SYN from {}:{}", format_ip(&ip.src_ip), seg.src_port);
        eprintln!("[>] Sending SYN-ACK seq={} ack={}", local_seq, remote_seq);

        Some(wrap_ip_tcp(
            ip.dst_ip, ip.src_ip,
            seg.dst_port, seg.src_port,
            local_seq, remote_seq,
            SYN | ACK, &[],
        ))
    }

    fn handle_established(
        &mut self,
        conn: &mut TcpConnection,
        ip: &Ipv4Packet,
        seg: &TcpSegment,
        data: &[u8],
    ) -> Option<Vec<u8>> {
        match conn.state {
            TcpState::SynRcvd => {
                if seg.flags & ACK != 0 {
                    conn.state = TcpState::Established;
                    eprintln!("[*] ESTABLISHED {}:{}", 
                              format_ip(&conn.remote_ip), conn.remote_port);
                }
                None
            }
            TcpState::Established => {
                if seg.flags & FIN != 0 {
                    conn.remote_seq = seg.seq.wrapping_add(1);
                    eprintln!("[<] FIN from {}:{}, closing",
                              format_ip(&conn.remote_ip), conn.remote_port);
                    // Send ACK for FIN
                    let ack = wrap_ip_tcp(
                        ip.dst_ip, ip.src_ip,
                        conn.local_port, conn.remote_port,
                        conn.local_seq, conn.remote_seq,
                        ACK, &[],
                    );
                    return Some(ack);
                }
                if !data.is_empty() {
                    conn.recv_buffer.extend_from_slice(data);
                    conn.remote_seq = seg.seq.wrapping_add(data.len() as u32);
                    eprintln!("[<] Received {} bytes from {}:{}, ACK sent",
                              data.len(), format_ip(&conn.remote_ip), conn.remote_port);
                    return Some(wrap_ip_tcp(
                        ip.dst_ip, ip.src_ip,
                        conn.local_port, conn.remote_port,
                        conn.local_seq, conn.remote_seq,
                        ACK, &[],
                    ));
                }
                if seg.flags & ACK != 0 {
                    // Update local_seq if this ACK advances it
                    return None;
                }
                None
            }
            _ => None,
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn format_ip(ip: &[u8; 4]) -> String {
    format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])
}

fn pseudo_random() -> u32 {
    // Simple PRNG for seq numbers (not cryptographically secure)
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let seed = t.as_secs() ^ t.subsec_nanos() as u64;
    (seed.wrapping_mul(6364136223846793005).wrapping_add(1)) as u32
}

// ─── Ethernet wrapping ──────────────────────────────────────────────────────

fn wrap_ethernet(
    src_mac: [u8; 6],
    dst_mac: [u8; 6],
    ip_payload: &[u8],
) -> Vec<u8> {
    let mut frame = Vec::with_capacity(14 + ip_payload.len());
    frame.extend_from_slice(&dst_mac);
    frame.extend_from_slice(&src_mac);
    frame.extend_from_slice(&0x0800u16.to_be_bytes()); // EtherType: IPv4
    frame.extend_from_slice(ip_payload);
    frame
}

// ─── Main ────────────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    let mut stack = TcpStack::new();
    stack.listen(8080);

    // Try to open a TAP device
    let tap_path = "/dev/net/tun";
    let _tap = match std::fs::OpenOptions::new().read(true).write(true).open(tap_path) {
        Ok(f) => f,
        Err(ref e) if e.kind() == io::ErrorKind::PermissionDenied => {
            eprintln!("Permission denied for {}. Run with sudo.", tap_path);
            eprintln!();
            eprintln!("Running in demo mode...");
            demo_run(&mut stack);
            return Ok(());
        }
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
            eprintln!("{} not found.", tap_path);
            eprintln!("Linux: sudo ip tuntap add dev tap0 mode tap && sudo ip link set tap0 up");
            eprintln!();
            eprintln!("Running in demo mode...");
            demo_run(&mut stack);
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    eprintln!("TCP/IP stack running on {}. Listening on port 8080.", tap_path);

    // Production: read frames from TAP device in a loop
    let mut buf = [0u8; 1518];
    loop {
        let n = match io::stdin().read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };

        if let Some(response) = process_frame(&mut stack, &buf[..n]) {
            let _ = io::stdout().write_all(&response);
        }
    }

    Ok(())
}

fn process_frame(stack: &mut TcpStack, data: &[u8]) -> Option<Vec<u8>> {
    let (eth, payload) = parse_ethernet(data)?;
    if eth.ether_type != 0x0800 {
        return None;
    }
    let (ip, ip_payload) = parse_ipv4(payload)?;
    if ip.protocol != 6 {
        return None;
    }
    let (tcp, tcp_data) = parse_tcp(ip_payload)?;
    let response = stack.handle_segment(&ip, &tcp, tcp_data)?;

    // Wrap response in Ethernet (swap MACs)
    let src_mac: [u8; 6] = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
    let dst_mac = eth.src_mac;
    Some(wrap_ethernet(src_mac, dst_mac, &response))
}

fn demo_run(stack: &mut TcpStack) {
    eprintln!("=== Demo: Simulating TCP 3-way handshake ===\n");

    // Simulate: client 10.0.0.2:54321 → server 10.0.0.1:8080
    let src_ip = [10, 0, 0, 2];
    let dst_ip = [10, 0, 0, 1];

    // Step 1: Client sends SYN
    eprintln!("Step 1: Client sends SYN");
    let syn_tcp = build_tcp_segment(54321, 8080, 1000, 0, SYN, 65535, &[]);
    let syn_tcp_with_cksum = {
        let mut t = syn_tcp.clone();
        let ck = tcp_checksum(&src_ip, &dst_ip, &t);
        t[16..18].copy_from_slice(&ck.to_be_bytes());
        t
    };
    let syn_ip = Ipv4Packet {
        total_length: 40,
        ttl: 64,
        protocol: 6,
        checksum: 0,
        src_ip,
        dst_ip,
        header_len: 20,
    };
    let syn_seg = parse_tcp(&syn_ip_with_payload(&syn_tcp_with_cksum).0).unwrap().0;

    if let Some(response) = stack.handle_segment(&syn_ip, &syn_seg, &[]) {
        let resp_tcp = parse_tcp(&response).unwrap().0;
        eprintln!("  Server responds with: SYN-ACK (seq={}, ack={})\n", resp_tcp.seq, resp_tcp.ack);

        // Step 2: Client sends ACK
        eprintln!("Step 2: Client sends ACK");
        let ack_tcp = build_tcp_segment(54321, 8080, 1001, resp_tcp.seq.wrapping_add(1), ACK, 65535, &[]);
        let ack_tcp_with_cksum = {
            let mut t = ack_tcp;
            let ck = tcp_checksum(&src_ip, &dst_ip, &t);
            t[16..18].copy_from_slice(&ck.to_be_bytes());
            t
        };
        let ack_seg = parse_tcp(&syn_ip_with_payload(&ack_tcp_with_cksum).0).unwrap().0;
        stack.handle_segment(&syn_ip, &ack_seg, &[]);
    }

    // Step 3: Client sends data
    eprintln!("\nStep 3: Client sends data");
    let payload = b"Hello from TCP client!";
    let data_tcp = build_tcp_segment(54321, 8080, 1001, 0, PSH | ACK, 65535, payload);
    let data_tcp_with_cksum = {
        let mut t = data_tcp;
        let ck = tcp_checksum(&src_ip, &dst_ip, &t);
        t[16..18].copy_from_slice(&ck.to_be_bytes());
        t
    };
    let (data_seg, seg_data) = parse_tcp(&data_tcp_with_cksum).unwrap();
    if let Some(_response) = stack.handle_segment(&syn_ip, &data_seg, seg_data) {
        eprintln!("  Server ACKed the data");
    }

    eprintln!("\n=== Demo complete ===");
}

fn syn_ip_with_payload(tcp: &[u8]) -> (Vec<u8>, &[u8]) {
    // Simple helper: build a fake IP packet wrapping TCP
    let pkt = build_ipv4_packet([10, 0, 0, 2], [10, 0, 0, 1], 6, tcp);
    let remaining = &pkt[20..]; // after IP header
    (pkt, remaining)
}
