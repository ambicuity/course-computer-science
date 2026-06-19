use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

// --- WebSocket Frame ---

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WsOpcode {
    Continuation = 0x0,
    Text = 0x1,
    Binary = 0x2,
    Close = 0x8,
    Ping = 0x9,
    Pong = 0xA,
}

impl WsOpcode {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x0 => Some(Self::Continuation),
            0x1 => Some(Self::Text),
            0x2 => Some(Self::Binary),
            0x8 => Some(Self::Close),
            0x9 => Some(Self::Ping),
            0xA => Some(Self::Pong),
            _ => None,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Continuation => "Continuation",
            Self::Text => "Text",
            Self::Binary => "Binary",
            Self::Close => "Close",
            Self::Ping => "Ping",
            Self::Pong => "Pong",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WebSocketFrame {
    pub fin: bool,
    pub opcode: WsOpcode,
    pub masked: bool,
    pub mask_key: Option<[u8; 4]>,
    pub payload: Vec<u8>,
}

/// Parse a WebSocket frame from raw bytes.
/// Returns the frame and the number of bytes consumed.
pub fn parse_ws_frame(data: &[u8]) -> Result<(WebSocketFrame, usize), String> {
    if data.len() < 2 {
        return Err("Need at least 2 bytes".to_string());
    }

    let fin = data[0] & 0x80 != 0;
    let opcode_val = data[0] & 0x0F;
    let opcode = WsOpcode::from_u8(opcode_val).ok_or(format!("Unknown opcode: 0x{opcode_val:X}"))?;

    let masked = data[1] & 0x80 != 0;
    let mut payload_len = (data[1] & 0x7F) as u64;
    let mut offset = 2;

    if payload_len == 126 {
        if data.len() < 4 {
            return Err("Need 4 bytes for extended 16-bit length".to_string());
        }
        payload_len = u16::from_be_bytes([data[2], data[3]]) as u64;
        offset = 4;
    } else if payload_len == 127 {
        if data.len() < 10 {
            return Err("Need 10 bytes for extended 64-bit length".to_string());
        }
        payload_len = u64::from_be_bytes(data[2..10].try_into().unwrap());
        offset = 10;
    }

    let mask_key = if masked {
        if data.len() < offset + 4 {
            return Err("Need 4 bytes for masking key".to_string());
        }
        let key: [u8; 4] = data[offset..offset + 4].try_into().unwrap();
        offset += 4;
        Some(key)
    } else {
        None
    };

    let total = offset + payload_len as usize;
    if data.len() < total {
        return Err(format!("Need {total} bytes, have {}", data.len()));
    }

    let mut payload = data[offset..total].to_vec();
    if let Some(key) = mask_key {
        apply_mask(&mut payload, key);
    }

    Ok((
        WebSocketFrame {
            fin,
            opcode,
            masked,
            mask_key,
            payload,
        },
        total,
    ))
}

/// Encode a WebSocket frame to bytes (server-side, unmasked).
pub fn build_ws_frame(opcode: WsOpcode, payload: &[u8], fin: bool) -> Vec<u8> {
    let mut buf = Vec::new();
    let first_byte = if fin { 0x80 } else { 0x00 } | (opcode as u8);
    buf.push(first_byte);

    let len = payload.len();
    if len < 126 {
        buf.push(len as u8);
    } else if len < 65536 {
        buf.push(126);
        buf.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        buf.push(127);
        buf.extend_from_slice(&(len as u64).to_be_bytes());
    }

    buf.extend_from_slice(payload);
    buf
}

/// XOR mask/unmask a payload with a 4-byte key.
pub fn apply_mask(data: &mut Vec<u8>, key: [u8; 4]) {
    for (i, byte) in data.iter_mut().enumerate() {
        *byte ^= key[i % 4];
    }
}

/// Build the Sec-WebSocket-Accept header value.
pub fn compute_ws_accept(key: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Simplified: real implementation uses SHA-1
    let magic = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let combined = format!("{key}{magic}");
    let mut hasher = DefaultHasher::new();
    combined.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

// --- SSE Event Parser ---

#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event_type: String,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
}

/// Parse a single SSE event block from lines.
pub fn parse_sse_event(lines: &[&str]) -> SseEvent {
    let mut event_type = "message".to_string();
    let mut data_parts = Vec::new();
    let mut id = None;
    let mut retry = None;

    for line in lines {
        if let Some(rest) = line.strip_prefix("event: ") {
            event_type = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("data: ") {
            data_parts.push(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("id: ") {
            id = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("retry: ") {
            retry = rest.parse().ok();
        }
    }

    SseEvent {
        event_type,
        data: data_parts.join("\n"),
        id,
        retry,
    }
}

/// Parse multiple SSE events from a text block.
pub fn parse_sse_stream(text: &str) -> Vec<SseEvent> {
    let mut events = Vec::new();
    let mut current_block = Vec::new();

    for line in text.lines() {
        if line.is_empty() {
            if !current_block.is_empty() {
                let refs: Vec<&str> = current_block.iter().map(|s| s.as_str()).collect();
                events.push(parse_sse_event(&refs));
                current_block.clear();
            }
        } else {
            current_block.push(line.to_string());
        }
    }

    if !current_block.is_empty() {
        let refs: Vec<&str> = current_block.iter().map(|s| s.as_str()).collect();
        events.push(parse_sse_event(&refs));
    }

    events
}

// --- gRPC Concepts ---

/// gRPC frame header (simplified).
/// HTTP/2 DATA frame with gRPC prefix: 1 byte compressed flag + 4 bytes message length.
#[derive(Debug)]
pub struct GrpcFrame {
    pub compressed: bool,
    pub message_length: u32,
    pub message: Vec<u8>,
}

/// Parse a gRPC message from an HTTP/2 DATA frame payload.
pub fn parse_grpc_frame(payload: &[u8]) -> Result<GrpcFrame, String> {
    if payload.len() < 5 {
        return Err("gRPC frame needs at least 5 bytes".to_string());
    }

    let compressed = payload[0] != 0;
    let message_length = u32::from_be_bytes([payload[1], payload[2], payload[3], payload[4]]);

    if payload.len() < 5 + message_length as usize {
        return Err(format!(
            "Need {} bytes for message, have {}",
            5 + message_length,
            payload.len()
        ));
    }

    let message = payload[5..5 + message_length as usize].to_vec();

    Ok(GrpcFrame {
        compressed,
        message_length,
        message,
    })
}

/// Build a gRPC message frame.
pub fn build_grpc_frame(message: &[u8], compressed: bool) -> Vec<u8> {
    let mut buf = Vec::with_capacity(5 + message.len());
    buf.push(if compressed { 1 } else { 0 });
    buf.extend_from_slice(&(message.len() as u32).to_be_bytes());
    buf.extend_from_slice(message);
    buf
}

fn main() {
    println!("=== WebSockets, SSE, gRPC ===\n");

    // --- WebSocket Demo ---
    println!("--- WebSocket Frame Parser ---");

    // Build a text frame (server-side, unmasked)
    let message = b"Hello, WebSocket!";
    let frame_bytes = build_ws_frame(WsOpcode::Text, message, true);
    println!("Built Text frame: {} bytes", frame_bytes.len());

    match parse_ws_frame(&frame_bytes) {
        Ok((frame, consumed)) => {
            println!("  FIN:     {}", frame.fin);
            println!("  Opcode:  {} ({})", frame.opcode as u8, frame.opcode.name());
            println!("  Masked:  {}", frame.masked);
            println!("  Length:   {}", frame.payload.len());
            println!("  Payload: {}", String::from_utf8_lossy(&frame.payload));
            println!("  Consumed: {consumed} bytes");
        }
        Err(e) => eprintln!("  Error: {e}"),
    }

    // Ping/Pong
    let ping = build_ws_frame(WsOpcode::Ping, b"ping-data", true);
    match parse_ws_frame(&ping) {
        Ok((frame, _)) => {
            println!("\n  Ping frame: opcode={}, payload={:?}", frame.opcode.name(), String::from_utf8_lossy(&frame.payload));
        }
        Err(e) => eprintln!("  Error: {e}"),
    }

    // Masking demo
    let mut data = b"mask me".to_vec();
    let key: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
    println!("\n  Masking demo:");
    println!("    Original: {:02X?}", data);
    apply_mask(&mut data, key);
    println!("    Masked:   {:02X?}", data);
    apply_mask(&mut data, key);
    println!("    Unmasked: {:02X?}", data);

    // --- SSE Demo ---
    println!("\n--- SSE Event Parser ---");
    let sse_text = "event: stock\ndata: {\"symbol\":\"AAPL\",\"price\":185.5}\nid: 42\n\nevent: stock\ndata: {\"symbol\":\"GOOG\",\"price\":141.2}\nid: 43\n\nevent: alert\ndata: Market closing soon\nid: 44\nretry: 5000\n";

    let events = parse_sse_stream(sse_text);
    for event in &events {
        println!("  Event: {}", event.event_type);
        println!("    Data: {}", event.data);
        if let Some(id) = &event.id {
            println!("    ID: {id}");
        }
        if let Some(retry) = &event.retry {
            println!("    Retry: {retry}ms");
        }
    }

    // --- gRPC Demo ---
    println!("\n--- gRPC Frame Parser ---");
    let message = b"grpc-service-call-data";
    let grpc_frame = build_grpc_frame(message, false);
    println!("Built gRPC frame: {} bytes", grpc_frame.len());

    match parse_grpc_frame(&grpc_frame) {
        Ok(frame) => {
            println!("  Compressed: {}", frame.compressed);
            println!("  Length:      {}", frame.message_length);
            println!("  Message:     {:?}", String::from_utf8_lossy(&frame.message));
        }
        Err(e) => eprintln!("  Error: {e}"),
    }

    println!("\n--- Protocol Comparison ---");
    println!("  WebSocket:  Full-duplex, binary + text, ping/pong keep-alive");
    println!("  SSE:        Server-to-client, text only, auto-reconnect");
    println!("  gRPC:       RPC over HTTP/2, protobuf, 4 streaming modes");
}
