use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;

// HTTP/1.1 Request structure
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

// HTTP/1.1 Response structure
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub reason: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

/// Parse raw bytes into an HTTP/1.1 request.
/// Format: METHOD /path HTTP/1.1\r\nHeader: Value\r\n...\r\n\r\n<body>
pub fn parse_http_request(data: &[u8]) -> Result<HttpRequest, String> {
    let mut reader = BufReader::new(data);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|e| format!("Failed to read request line: {e}"))?;

    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() != 3 {
        return Err("Invalid request line".to_string());
    }

    let method = parts[0].to_string();
    let path = parts[1].to_string();
    let version = parts[2].to_string();

    let mut headers = HashMap::new();
    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|e| format!("Failed to read header: {e}"))?;
        let line = line.trim();
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_lowercase(), value.trim().to_string());
        }
    }

    let mut body = Vec::new();
    if let Some(content_length) = headers.get("content-length") {
        if let Ok(len) = content_length.parse::<usize>() {
            body.resize(len, 0);
            reader
                .read_exact(&mut body)
                .map_err(|e| format!("Failed to read body: {e}"))?;
        }
    }

    Ok(HttpRequest {
        method,
        path,
        version,
        headers,
        body,
    })
}

/// Serialize an HTTP response to bytes.
pub fn build_http_response(response: &HttpResponse) -> Vec<u8> {
    let mut buf = Vec::new();
    write!(
        &mut buf,
        "HTTP/1.1 {} {}\r\n",
        response.status, response.reason
    )
    .unwrap();

    for (key, value) in &response.headers {
        write!(&mut buf, "{key}: {value}\r\n").unwrap();
    }
    buf.extend_from_slice(b"\r\n");
    buf.extend_from_slice(&response.body);
    buf
}

// HTTP/2 frame types
pub const H2_FRAME_DATA: u8 = 0x0;
pub const H2_FRAME_HEADERS: u8 = 0x1;
pub const H2_FRAME_SETTINGS: u8 = 0x4;
pub const H2_FRAME_PING: u8 = 0x6;
pub const H2_FRAME_GOAWAY: u8 = 0x7;

/// Parsed HTTP/2 frame
#[derive(Debug, Clone)]
pub struct H2Frame {
    pub length: u32,
    pub frame_type: u8,
    pub flags: u8,
    pub stream_id: u32,
    pub payload: Vec<u8>,
}

impl H2Frame {
    pub fn frame_type_name(&self) -> &str {
        match self.frame_type {
            H2_FRAME_DATA => "DATA",
            H2_FRAME_HEADERS => "HEADERS",
            0x2 => "PRIORITY",
            0x3 => "RST_STREAM",
            H2_FRAME_SETTINGS => "SETTINGS",
            0x5 => "PUSH_PROMISE",
            H2_FRAME_PING => "PING",
            H2_FRAME_GOAWAY => "GOAWAY",
            0x8 => "WINDOW_UPDATE",
            0x9 => "CONTINUATION",
            _ => "UNKNOWN",
        }
    }
}

/// HTTP/2 connection preface (magic string sent by client)
pub const H2_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

/// Parse an HTTP/2 frame from raw bytes.
/// Frame header: 3 bytes length + 1 byte type + 1 byte flags + 4 bytes stream ID (1 bit reserved)
pub fn parse_h2_frame(data: &[u8]) -> Result<(H2Frame, usize), String> {
    if data.len() < 9 {
        return Err("Not enough data for frame header".to_string());
    }

    let length = u32::from_be_bytes([0, data[0], data[1], data[2]]);
    let frame_type = data[3];
    let flags = data[4];
    let stream_id = u32::from_be_bytes([data[5] & 0x7F, data[6], data[7], data[8]]);

    let total = 9 + length as usize;
    if data.len() < total {
        return Err(format!(
            "Need {total} bytes but only have {}",
            data.len()
        ));
    }

    let payload = data[9..total].to_vec();

    Ok((
        H2Frame {
            length,
            frame_type,
            flags,
            stream_id,
            payload,
        },
        total,
    ))
}

/// Build an HTTP/2 frame header + payload.
pub fn build_h2_frame(frame_type: u8, flags: u8, stream_id: u32, payload: &[u8]) -> Vec<u8> {
    let length = payload.len() as u32;
    let mut buf = Vec::with_capacity(9 + payload.len());

    buf.push(((length >> 16) & 0xFF) as u8);
    buf.push(((length >> 8) & 0xFF) as u8);
    buf.push((length & 0xFF) as u8);
    buf.push(frame_type);
    buf.push(flags);
    buf.extend_from_slice(&(stream_id & 0x7FFF_FFFF).to_be_bytes());
    buf.extend_from_slice(payload);
    buf
}

/// Build a SETTINGS frame with default values.
pub fn build_settings_frame() -> Vec<u8> {
    // SETTINGS: id(2) + value(4) for each setting
    let mut payload = Vec::new();
    // SETTINGS_MAX_CONCURRENT_STREAMS = 3
    payload.extend_from_slice(&0x0003u16.to_be_bytes());
    payload.extend_from_slice(&100u32.to_be_bytes());
    // SETTINGS_INITIAL_WINDOW_SIZE = 4
    payload.extend_from_slice(&0x0004u16.to_be_bytes());
    payload.extend_from_slice(&65535u32.to_be_bytes());
    // SETTINGS_MAX_FRAME_SIZE = 5
    payload.extend_from_slice(&0x0005u16.to_be_bytes());
    payload.extend_from_slice(&16384u32.to_be_bytes());

    build_h2_frame(H2_FRAME_SETTINGS, 0x00, 0, &payload)
}

/// Build a PING frame.
pub fn build_ping_frame(opaque_data: [u8; 8]) -> Vec<u8> {
    build_h2_frame(H2_FRAME_PING, 0x00, 0, &opaque_data)
}

/// Build a PONG (PING ACK) response.
pub fn build_pong_frame(opaque_data: [u8; 8]) -> Vec<u8> {
    build_h2_frame(H2_FRAME_PING, 0x01, 0, &opaque_data)
}

fn main() {
    println!("=== HTTP/1.1 and HTTP/2 Protocol Parser ===\n");

    // Demo 1: Parse an HTTP/1.1 request
    let raw_request = b"GET /api/users HTTP/1.1\r\nHost: localhost:8080\r\nAccept: application/json\r\nConnection: keep-alive\r\n\r\n";
    match parse_http_request(raw_request) {
        Ok(req) => {
            println!("Parsed HTTP/1.1 Request:");
            println!("  Method:  {}", req.method);
            println!("  Path:    {}", req.path);
            println!("  Version: {}", req.version);
            println!("  Headers:");
            for (k, v) in &req.headers {
                println!("    {k}: {v}");
            }
        }
        Err(e) => eprintln!("Parse error: {e}"),
    }

    // Demo 2: Build an HTTP/1.1 response
    let mut resp_headers = HashMap::new();
    resp_headers.insert("Content-Type".into(), "application/json".into());
    resp_headers.insert("Content-Length".into(), "27".into());
    let response = HttpResponse {
        status: 200,
        reason: "OK".into(),
        headers: resp_headers,
        body: b"{\"users\":[\"alice\",\"bob\"]}".to_vec(),
    };
    let resp_bytes = build_http_response(&response);
    println!("\nBuilt HTTP/1.1 Response:");
    println!("{}", String::from_utf8_lossy(&resp_bytes));

    // Demo 3: Parse HTTP/2 frames
    println!("HTTP/2 Frame Examples:");

    // Connection preface
    println!(
        "\n  Connection preface: {:?}",
        String::from_utf8_lossy(H2_PREFACE)
    );

    // SETTINGS frame
    let settings = build_settings_frame();
    match parse_h2_frame(&settings) {
        Ok((frame, _)) => {
            println!("\n  Parsed SETTINGS frame:");
            println!("    Length:     {}", frame.length);
            println!("    Type:      {} ({})", frame.frame_type, frame.frame_type_name());
            println!("    Flags:     0x{:02X}", frame.flags);
            println!("    Stream ID: {}", frame.stream_id);
        }
        Err(e) => eprintln!("  Frame parse error: {e}"),
    }

    // PING frame
    let ping_data: [u8; 8] = [0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04];
    let ping = build_ping_frame(ping_data);
    match parse_h2_frame(&ping) {
        Ok((frame, _)) => {
            println!("\n  Parsed PING frame:");
            println!("    Length:     {}", frame.length);
            println!("    Type:      {} ({})", frame.frame_type, frame.frame_type_name());
            println!("    Stream ID: {}", frame.stream_id);
            println!("    Payload:   {:02X?}", frame.payload);
        }
        Err(e) => eprintln!("  Frame parse error: {e}"),
    }

    // PONG response
    let pong = build_pong_frame(ping_data);
    match parse_h2_frame(&pong) {
        Ok((frame, _)) => {
            println!("\n  Parsed PING ACK frame:");
            println!("    Flags:     0x{:02X} (ACK={})", frame.flags, frame.flags & 0x01);
        }
        Err(e) => eprintln!("  Frame parse error: {e}"),
    }
}
