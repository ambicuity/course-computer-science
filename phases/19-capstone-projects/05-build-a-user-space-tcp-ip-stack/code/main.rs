// Build a User-Space TCP/IP Stack
// Run: rustc main.rs && ./main
//
// Architecture:
//   Application → TCP (connection state, reliability) → IP (addressing) → Ethernet (framing)
//
// Implements IPv4/TCP header parsing/serialization, a TCP state machine
// with handshake simulation, and data transfer.

// =============================================================================
// Step 1: IPv4 and TCP Header Parsing
// =============================================================================

#[derive(Debug, Clone)]
struct IPv4Header {
    version: u8,
    ihl: u8,
    total_length: u16,
    protocol: u8,
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
            version, ihl,
            total_length: u16::from_be_bytes([data[2], data[3]]),
            protocol: data[9],
            src_ip: [data[12], data[13], data[14], data[15]],
            dst_ip: [data[16], data[17], data[18], data[19]],
        }, &data[header_len..]))
    }

    fn serialize(&self, payload: &[u8]) -> Vec<u8> {
        let total_len = 20 + payload.len();
        let mut buf = vec![0u8; 20];
        buf[0] = (4 << 4) | 5;
        buf[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
        buf[8] = 64; // TTL
        buf[9] = self.protocol;
        buf[12..16].copy_from_slice(&self.src_ip);
        buf[16..20].copy_from_slice(&self.dst_ip);
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

#[derive(Debug, Clone)]
struct TCPHeader {
    src_port: u16,
    dst_port: u16,
    seq_num: u32,
    ack_num: u32,
    data_offset: u8,
    flags: u8,
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
        let mut buf = vec![0u8; 20];
        buf[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        buf[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        buf[4..8].copy_from_slice(&self.seq_num.to_be_bytes());
        buf[8..12].copy_from_slice(&self.ack_num.to_be_bytes());
        buf[12] = 5 << 4;
        buf[13] = self.flags;
        buf[14..16].copy_from_slice(&self.window.to_be_bytes());
        buf.extend_from_slice(payload);
        buf
    }

    fn has_flag(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }
}

// =============================================================================
// Step 2: TCP Connection State Machine
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
enum TCPState {
    Closed, Listen, SynSent, SynReceived, Established,
    FinWait1, FinWait2, CloseWait, LastAck, TimeWait,
}

struct TCPConnection {
    state: TCPState,
    local_port: u16,
    remote_port: u16,
    local_seq: u32,
    remote_seq: u32,
    recv_buffer: Vec<u8>,
}

impl TCPConnection {
    fn new(local_port: u16) -> Self {
        TCPConnection {
            state: TCPState::Listen,
            local_port, remote_port: 0,
            local_seq: 1000, remote_seq: 0,
            recv_buffer: Vec::new(),
        }
    }

    fn process_segment(&mut self, header: &TCPHeader, payload: &[u8]) -> Option<TCPHeader> {
        match self.state {
            TCPState::Listen => {
                if header.has_flag(TCP_SYN) {
                    self.remote_port = header.src_port;
                    self.remote_seq = header.seq_num;
                    self.state = TCPState::SynReceived;
                    self.local_seq = 1000;
                    println!("  [LISTEN → SYN_RCVD] Received SYN, seq={}", header.seq_num);
                    Some(TCPHeader {
                        src_port: self.local_port, dst_port: self.remote_port,
                        seq_num: self.local_seq, ack_num: header.seq_num.wrapping_add(1),
                        data_offset: 5, flags: TCP_SYN | TCP_ACK, window: 65535,
                    })
                } else { None }
            }
            TCPState::SynReceived => {
                if header.has_flag(TCP_ACK) {
                    self.state = TCPState::Established;
                    self.remote_seq = header.seq_num;
                    println!("  [SYN_RCVD → ESTABLISHED] Received ACK, handshake complete");
                    None
                } else { None }
            }
            TCPState::Established => {
                if header.has_flag(TCP_FIN) {
                    self.state = TCPState::CloseWait;
                    self.remote_seq = header.seq_num;
                    println!("  [ESTABLISHED → CLOSE_WAIT] Received FIN");
                    Some(TCPHeader {
                        src_port: self.local_port, dst_port: self.remote_port,
                        seq_num: self.local_seq, ack_num: header.seq_num.wrapping_add(1),
                        data_offset: 5, flags: TCP_ACK, window: 65535,
                    })
                } else if !payload.is_empty() {
                    self.recv_buffer.extend_from_slice(payload);
                    self.remote_seq = header.seq_num;
                    println!("  [ESTABLISHED] Received {} bytes, total buffered: {}",
                             payload.len(), self.recv_buffer.len());
                    Some(TCPHeader {
                        src_port: self.local_port, dst_port: self.remote_port,
                        seq_num: self.local_seq,
                        ack_num: header.seq_num.wrapping_add(payload.len() as u32),
                        data_offset: 5, flags: TCP_ACK, window: 65535,
                    })
                } else { None }
            }
            _ => {
                println!("  [{}] Unhandled in state {:?}", self.local_port, self.state);
                None
            }
        }
    }
}

// =============================================================================
// Step 3: Handshake Simulation
// =============================================================================

fn main() {
    println!("=== TCP Handshake Simulation ===\n");

    let mut server = TCPConnection::new(80);
    println!("Server: listening on port 80");

    let syn = TCPHeader {
        src_port: 54321, dst_port: 80, seq_num: 1000, ack_num: 0,
        data_offset: 5, flags: TCP_SYN, window: 65535,
    };
    println!("Client: sending SYN, seq={}", syn.seq_num);

    let syn_ack = server.process_segment(&syn, &[]).unwrap();
    println!("Server: sending SYN-ACK, seq={}, ack={}", syn_ack.seq_num, syn_ack.ack_num);

    println!("Client: received SYN-ACK, sending ACK");
    let ack = TCPHeader {
        src_port: 54321, dst_port: 80, seq_num: 1001,
        ack_num: syn_ack.seq_num.wrapping_add(1),
        data_offset: 5, flags: TCP_ACK, window: 65535,
    };
    server.process_segment(&ack, &[]);
    println!("Server: state = {:?}", server.state);

    println!("\n=== Data Transfer ===\n");
    let data = b"Hello, TCP!";
    let data_seg = TCPHeader {
        src_port: 54321, dst_port: 80, seq_num: 1001,
        ack_num: syn_ack.seq_num.wrapping_add(1),
        data_offset: 5, flags: TCP_ACK, window: 65535,
    };
    println!("Client: sending {} bytes of data", data.len());
    let data_ack = server.process_segment(&data_seg, data).unwrap();
    println!("Server: ACK'd {} bytes, ack_num={}", data.len(), data_ack.ack_num);
    println!("Server: buffered data: {:?}", String::from_utf8_lossy(&server.recv_buffer));
}
