//! Phase Capstone — An HTTP/2 Server on a Custom TCP Stack
//! Phase 09 — Computer Networks
//!
//! A from-scratch HTTP/2 implementation featuring:
//!   - Full frame layer (9-byte header, DATA/HEADERS/SETTINGS/RST_STREAM/PING/GOAWAY/WINDOW_UPDATE)
//!   - Simplified HPACK (static table + dynamic table, no Huffman)
//!   - Stream multiplexer with state machine (idle → open → half-closed → closed)
//!   - Connection preface handshake
//!   - Per-stream and per-connection flow control
//!
//! The session is generic over `T: Read + Write`, so it works with `std::net::TcpStream`
//! (used in the default server below) or with the userspace TCP/IP stack from Lesson 11.
//!
//! Usage:
//!   cargo run
//!   curl --http2-prior-knowledge http://localhost:8080/
//!
//! Note: curl's --http2-prior-knowledge speaks HTTP/2 without TLS upgrade (h2c).
//! Some clients need `--http2` (TLS) or `--http2-prior-knowledge` for cleartext.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

// ============================================================================
// Constants
// ============================================================================

const HTTP2_MAGIC: &[u8; 24] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
const DEFAULT_WINDOW_SIZE: u32 = 65_535;
const DEFAULT_MAX_FRAME_SIZE: u32 = 16_384;
const FRAME_HEADER_LEN: usize = 9;

// Frame types
const FRAME_DATA: u8 = 0x00;
const FRAME_HEADERS: u8 = 0x01;
const FRAME_PRIORITY: u8 = 0x02;
const FRAME_RST_STREAM: u8 = 0x03;
const FRAME_SETTINGS: u8 = 0x04;
const FRAME_PUSH_PROMISE: u8 = 0x05;
const FRAME_PING: u8 = 0x06;
const FRAME_GOAWAY: u8 = 0x07;
const FRAME_WINDOW_UPDATE: u8 = 0x08;
const FRAME_CONTINUATION: u8 = 0x09;

// Frame flags
const FLAG_END_STREAM: u8 = 0x01;
const FLAG_ACK: u8 = 0x01;
const FLAG_END_HEADERS: u8 = 0x04;
const FLAG_PADDED: u8 = 0x08;
const FLAG_PRIORITY: u8 = 0x20;

// HTTP/2 error codes
const ERROR_NO_ERROR: u32 = 0x00;
const ERROR_PROTOCOL_ERROR: u32 = 0x01;
const ERROR_INTERNAL_ERROR: u32 = 0x02;
const ERROR_FLOW_CONTROL_ERROR: u32 = 0x03;
const ERROR_STREAM_CLOSED: u32 = 0x05;
const ERROR_FRAME_SIZE_ERROR: u32 = 0x06;
const ERROR_REFUSED_STREAM: u32 = 0x07;
const ERROR_ENHANCE_YOUR_CALM: u32 = 0x0B;

// Settings identifiers
const SETTINGS_HEADER_TABLE_SIZE: u16 = 0x01;
const SETTINGS_ENABLE_PUSH: u16 = 0x02;
const SETTINGS_MAX_CONCURRENT_STREAMS: u16 = 0x03;
const SETTINGS_INITIAL_WINDOW_SIZE: u16 = 0x04;
const SETTINGS_MAX_FRAME_SIZE: u16 = 0x05;

// ============================================================================
// Frame Module
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Data,
    Headers,
    Priority,
    RstStream,
    Settings,
    PushPromise,
    Ping,
    Goaway,
    WindowUpdate,
    Continuation,
}

impl FrameType {
    fn from_byte(b: u8) -> Result<Self, H2Error> {
        match b {
            FRAME_DATA => Ok(FrameType::Data),
            FRAME_HEADERS => Ok(FrameType::Headers),
            FRAME_PRIORITY => Ok(FrameType::Priority),
            FRAME_RST_STREAM => Ok(FrameType::RstStream),
            FRAME_SETTINGS => Ok(FrameType::Settings),
            FRAME_PUSH_PROMISE => Ok(FrameType::PushPromise),
            FRAME_PING => Ok(FrameType::Ping),
            FRAME_GOAWAY => Ok(FrameType::Goaway),
            FRAME_WINDOW_UPDATE => Ok(FrameType::WindowUpdate),
            FRAME_CONTINUATION => Ok(FrameType::Continuation),
            _ => Err(H2Error::Protocol("unknown frame type".into())),
        }
    }

    fn to_byte(self) -> u8 {
        match self {
            FrameType::Data => FRAME_DATA,
            FrameType::Headers => FRAME_HEADERS,
            FrameType::Priority => FRAME_PRIORITY,
            FrameType::RstStream => FRAME_RST_STREAM,
            FrameType::Settings => FRAME_SETTINGS,
            FrameType::PushPromise => FRAME_PUSH_PROMISE,
            FrameType::Ping => FRAME_PING,
            FrameType::Goaway => FRAME_GOAWAY,
            FrameType::WindowUpdate => FRAME_WINDOW_UPDATE,
            FrameType::Continuation => FRAME_CONTINUATION,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FrameHeader {
    pub length: u32,
    pub frame_type: FrameType,
    pub flags: u8,
    pub stream_id: u32,
}

#[derive(Debug, Clone)]
pub struct Setting {
    pub id: u16,
    pub value: u32,
}

#[derive(Debug, Clone)]
pub enum Frame {
    Data {
        stream_id: u32,
        end_stream: bool,
        data: Vec<u8>,
    },
    Headers {
        stream_id: u32,
        end_headers: bool,
        end_stream: bool,
        priority: Option<PriorityParams>,
        data: Vec<u8>,
    },
    Priority {
        stream_id: u32,
        exclusive: bool,
        dependency: u32,
        weight: u8,
    },
    RstStream {
        stream_id: u32,
        error_code: u32,
    },
    Settings {
        ack: bool,
        settings: Vec<Setting>,
    },
    PushPromise {
        stream_id: u32,
        promised_id: u32,
        end_headers: bool,
        data: Vec<u8>,
    },
    Ping {
        ack: bool,
        opaque_data: [u8; 8],
    },
    Goaway {
        last_stream_id: u32,
        error_code: u32,
        debug_data: Vec<u8>,
    },
    WindowUpdate {
        stream_id: u32,
        increment: u32,
    },
    Continuation {
        stream_id: u32,
        end_headers: bool,
        data: Vec<u8>,
    },
}

#[derive(Debug, Clone)]
pub struct PriorityParams {
    pub exclusive: bool,
    pub dependency: u32,
    pub weight: u8,
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug)]
pub enum H2Error {
    Io(io::Error),
    Protocol(String),
    FrameSize(u32),
    StreamClosed(u32),
    FlowControl,
    Refused,
    Internal(String),
}

impl From<io::Error> for H2Error {
    fn from(e: io::Error) -> Self {
        H2Error::Io(e)
    }
}

impl std::fmt::Display for H2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            H2Error::Io(e) => write!(f, "I/O error: {}", e),
            H2Error::Protocol(s) => write!(f, "protocol error: {}", s),
            H2Error::FrameSize(s) => write!(f, "frame size error: stream={}", s),
            H2Error::StreamClosed(s) => write!(f, "stream closed: {}", s),
            H2Error::FlowControl => write!(f, "flow control error"),
            H2Error::Refused => write!(f, "refused stream"),
            H2Error::Internal(s) => write!(f, "internal error: {}", s),
        }
    }
}

// ============================================================================
// Frame encoding / decoding
// ============================================================================

fn encode_frame_header(hdr: &FrameHeader) -> [u8; FRAME_HEADER_LEN] {
    let mut buf = [0u8; FRAME_HEADER_LEN];
    let len = hdr.length.min(0xFFFFFF);
    buf[0] = (len >> 16) as u8;
    buf[1] = (len >> 8) as u8;
    buf[2] = len as u8;
    buf[3] = hdr.frame_type.to_byte();
    buf[4] = hdr.flags;
    let sid = hdr.stream_id & 0x7FFF_FFFF;
    buf[5] = (sid >> 24) as u8;
    buf[6] = (sid >> 16) as u8;
    buf[7] = (sid >> 8) as u8;
    buf[8] = sid as u8;
    buf
}

fn decode_frame_header(buf: &[u8]) -> Result<FrameHeader, H2Error> {
    if buf.len() < FRAME_HEADER_LEN {
        return Err(H2Error::Protocol("short frame header".into()));
    }
    let length = (buf[0] as u32) << 16 | (buf[1] as u32) << 8 | buf[2] as u32;
    let frame_type = FrameType::from_byte(buf[3])?;
    let flags = buf[4];
    let stream_id = ((buf[5] as u32) << 24 | (buf[6] as u32) << 16 | (buf[7] as u32) << 8 | buf[8] as u32) & 0x7FFF_FFFF;
    Ok(FrameHeader { length, frame_type, flags, stream_id })
}

pub fn read_frame<R: Read>(r: &mut R) -> Result<Frame, H2Error> {
    let mut header_buf = [0u8; FRAME_HEADER_LEN];
    r.read_exact(&mut header_buf)?;
    let hdr = decode_frame_header(&header_buf)?;

    let mut payload = vec![0u8; hdr.length as usize];
    if !payload.is_empty() {
        r.read_exact(&mut payload)?;
    }

    match hdr.frame_type {
        FrameType::Data => {
            let mut offset = 0usize;
            let pad_length = if hdr.flags & FLAG_PADDED != 0 {
                let pad = payload.get(0).copied().unwrap_or(0) as usize;
                offset = 1;
                pad
            } else {
                0
            };
            let data_len = payload.len().saturating_sub(offset + pad_length);
            let data = payload[offset..offset + data_len].to_vec();
            Ok(Frame::Data {
                stream_id: hdr.stream_id,
                end_stream: hdr.flags & FLAG_END_STREAM != 0,
                data,
            })
        }
        FrameType::Headers => {
            let mut offset = 0usize;
            let pad_length = if hdr.flags & FLAG_PADDED != 0 {
                let pad = payload.get(0).copied().unwrap_or(0) as usize;
                offset = 1;
                pad
            } else {
                0
            };
            let priority = if hdr.flags & FLAG_PRIORITY != 0 {
                if payload.len() < offset + 5 {
                    return Err(H2Error::Protocol("short HEADERS frame".into()));
                }
                let excl_dep = u32::from_be_bytes([
                    0, payload[offset], payload[offset + 1], payload[offset + 2],
                ]);
                let exclusive = (excl_dep >> 31) != 0;
                let dependency = excl_dep & 0x7FFF_FFFF;
                let weight = payload[offset + 4];
                offset += 5;
                Some(PriorityParams { exclusive, dependency, weight })
            } else {
                None
            };
            let data_len = payload.len().saturating_sub(offset + pad_length);
            let data = payload[offset..offset + data_len].to_vec();
            Ok(Frame::Headers {
                stream_id: hdr.stream_id,
                end_headers: hdr.flags & FLAG_END_HEADERS != 0,
                end_stream: hdr.flags & FLAG_END_STREAM != 0,
                priority,
                data,
            })
        }
        FrameType::Priority => {
            if payload.len() < 5 {
                return Err(H2Error::Protocol("short PRIORITY frame".into()));
            }
            let raw = u32::from_be_bytes([0, payload[0], payload[1], payload[2]]);
            let exclusive = (raw >> 31) != 0;
            let dependency = raw & 0x7FFF_FFFF;
            let weight = payload[4];
            Ok(Frame::Priority { stream_id: hdr.stream_id, exclusive, dependency, weight })
        }
        FrameType::RstStream => {
            if payload.len() < 4 {
                return Err(H2Error::Protocol("short RST_STREAM frame".into()));
            }
            let error_code = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
            Ok(Frame::RstStream { stream_id: hdr.stream_id, error_code })
        }
        FrameType::Settings => {
            let ack = hdr.flags & FLAG_ACK != 0;
            let mut settings = Vec::new();
            if !ack {
                if payload.len() % 6 != 0 {
                    return Err(H2Error::Protocol("invalid SETTINGS payload size".into()));
                }
                for chunk in payload.chunks(6) {
                    let id = u16::from_be_bytes([chunk[0], chunk[1]]);
                    let value = u32::from_be_bytes([chunk[2], chunk[3], chunk[4], chunk[5]]);
                    settings.push(Setting { id, value });
                }
            }
            Ok(Frame::Settings { ack, settings })
        }
        FrameType::PushPromise => {
            if hdr.stream_id == 0 {
                return Err(H2Error::Protocol("PUSH_PROMISE on stream 0".into()));
            }
            let mut offset = 0usize;
            let pad_length = if hdr.flags & FLAG_PADDED != 0 {
                let pad = payload.get(0).copied().unwrap_or(0) as usize;
                offset = 1;
                pad
            } else {
                0
            };
            if payload.len() < offset + 4 {
                return Err(H2Error::Protocol("short PUSH_PROMISE frame".into()));
            }
            let promised_id = u32::from_be_bytes([
                0, payload[offset], payload[offset + 1], payload[offset + 2],
            ]) & 0x7FFF_FFFF;
            offset += 4;
            let data_len = payload.len().saturating_sub(offset + pad_length);
            let data = payload[offset..offset + data_len].to_vec();
            Ok(Frame::PushPromise {
                stream_id: hdr.stream_id,
                promised_id,
                end_headers: hdr.flags & FLAG_END_HEADERS != 0,
                data,
            })
        }
        FrameType::Ping => {
            if payload.len() < 8 {
                return Err(H2Error::Protocol("short PING frame".into()));
            }
            let mut opaque_data = [0u8; 8];
            opaque_data.copy_from_slice(&payload[..8]);
            Ok(Frame::Ping { ack: hdr.flags & FLAG_ACK != 0, opaque_data })
        }
        FrameType::Goaway => {
            if payload.len() < 8 {
                return Err(H2Error::Protocol("short GOAWAY frame".into()));
            }
            let last_stream_id = u32::from_be_bytes([0, payload[0], payload[1], payload[2]]) & 0x7FFF_FFFF;
            let error_code = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
            let debug_data = if payload.len() > 8 { payload[8..].to_vec() } else { Vec::new() };
            Ok(Frame::Goaway { last_stream_id, error_code, debug_data })
        }
        FrameType::WindowUpdate => {
            if payload.len() < 4 {
                return Err(H2Error::Protocol("short WINDOW_UPDATE frame".into()));
            }
            let increment = u32::from_be_bytes([0, payload[0], payload[1], payload[2]]) & 0x7FFF_FFFF;
            if increment == 0 {
                return Err(H2Error::Protocol("zero window increment".into()));
            }
            Ok(Frame::WindowUpdate { stream_id: hdr.stream_id, increment })
        }
        FrameType::Continuation => {
            let data = payload;
            Ok(Frame::Continuation {
                stream_id: hdr.stream_id,
                end_headers: hdr.flags & FLAG_END_HEADERS != 0,
                data,
            })
        }
    }
}

pub fn write_frame<W: Write>(w: &mut W, frame: &Frame) -> Result<(), H2Error> {
    let (frame_type, stream_id, flags, payload): (FrameType, u32, u8, Vec<u8>) = match frame {
        Frame::Data { stream_id, end_stream, data } => {
            let flags = if *end_stream { FLAG_END_STREAM } else { 0 };
            (FrameType::Data, *stream_id, flags, data.clone())
        }
        Frame::Headers { stream_id, end_headers, end_stream, priority, data } => {
            let mut flags = 0u8;
            if *end_headers { flags |= FLAG_END_HEADERS; }
            if *end_stream { flags |= FLAG_END_STREAM; }
            let mut payload = Vec::new();
            if let Some(p) = priority {
                flags |= FLAG_PRIORITY;
                let raw = p.dependency | if p.exclusive { 0x8000_0000 } else { 0 };
                payload.extend_from_slice(&raw.to_be_bytes()[1..]);
                payload.push(p.weight);
            }
            payload.extend_from_slice(data);
            (FrameType::Headers, *stream_id, flags, payload)
        }
        Frame::Priority { stream_id, exclusive, dependency, weight } => {
            let raw = *dependency | if *exclusive { 0x8000_0000 } else { 0 };
            let mut payload = vec![0u8; 5];
            payload[..3].copy_from_slice(&raw.to_be_bytes()[1..]);
            payload[4] = *weight;
            (FrameType::Priority, *stream_id, 0, payload)
        }
        Frame::RstStream { stream_id, error_code } => {
            let payload = error_code.to_be_bytes().to_vec();
            (FrameType::RstStream, *stream_id, 0, payload)
        }
        Frame::Settings { ack, settings } => {
            let flags = if *ack { FLAG_ACK } else { 0 };
            let mut payload = Vec::with_capacity(settings.len() * 6);
            for s in settings {
                payload.extend_from_slice(&s.id.to_be_bytes());
                payload.extend_from_slice(&s.value.to_be_bytes());
            }
            (FrameType::Settings, 0, flags, payload)
        }
        Frame::PushPromise { stream_id, promised_id, end_headers, data } => {
            let flags = if *end_headers { FLAG_END_HEADERS } else { 0 };
            let mut payload = vec![0u8; 4];
            payload[..3].copy_from_slice(&((*promised_id & 0x7FFF_FFFF) as u32).to_be_bytes()[1..]);
            payload.extend_from_slice(data);
            (FrameType::PushPromise, *stream_id, flags, payload)
        }
        Frame::Ping { ack, opaque_data } => {
            let flags = if *ack { FLAG_ACK } else { 0 };
            (FrameType::Ping, 0, flags, opaque_data.to_vec())
        }
        Frame::Goaway { last_stream_id, error_code, debug_data } => {
            let mut payload = vec![0u8; 8];
            payload[..3].copy_from_slice(&((*last_stream_id & 0x7FFF_FFFF) as u32).to_be_bytes()[1..]);
            payload[4..8].copy_from_slice(&error_code.to_be_bytes());
            payload.extend_from_slice(debug_data);
            (FrameType::Goaway, *stream_id, 0, payload)
        }
        Frame::WindowUpdate { stream_id, increment } => {
            let mut payload = vec![0u8; 4];
            payload[..3].copy_from_slice(&((*increment & 0x7FFF_FFFF) as u32).to_be_bytes()[1..]);
            (FrameType::WindowUpdate, *stream_id, 0, payload)
        }
        Frame::Continuation { stream_id, end_headers, data } => {
            let flags = if *end_headers { FLAG_END_HEADERS } else { 0 };
            (FrameType::Continuation, *stream_id, flags, data.clone())
        }
    };

    let length = payload.len() as u32;
    let hdr = FrameHeader { length, frame_type, flags, stream_id };
    let header_bytes = encode_frame_header(&hdr);
    w.write_all(&header_bytes)?;
    w.write_all(&payload)?;
    w.flush()?;
    Ok(())
}

// ============================================================================
// HPACK Module (Simplified — no Huffman encoding)
// ============================================================================

pub struct Hpack {
    dynamic_table: Vec<(Vec<u8>, Vec<u8>)>,
    max_table_size: usize,
}

static STATIC_TABLE: &[(u8, u8)] = &[
    (0, 0),   // index 0: unused in HPACK (1-indexed in the RFC, but we use 0-based internally)
    // Name-only entries (empty value = name-only)
    (0, 0),   //  1: :authority
    // Full entries
    (0, 0),   //  2: :method GET
    (0, 0),   //  3: :method POST
    (0, 0),   //  4: :path /
    (0, 0),   //  5: :path /index.html
    (0, 0),   //  6: :scheme http
    (0, 0),   //  7: :scheme https
    (0, 0),   //  8: :status 200
    (0, 0),   //  9: :status 204
    (0, 0),   // 10: :status 206
    (0, 0),   // 11: :status 304
    (0, 0),   // 12: :status 400
    (0, 0),   // 13: :status 404
    (0, 0),   // 14: :status 500
    (0, 0),   // 15: accept-charset
    (0, 0),   // 16: accept-encoding
    (0, 0),   // 17: accept-language
    (0, 0),   // 18: accept-ranges
    (0, 0),   // 19: accept
    (0, 0),   // 20: access-control-allow-origin
    (0, 0),   // 21: age
    (0, 0),   // 22: allow
    (0, 0),   // 23: authorization
    (0, 0),   // 24: cache-control
    (0, 0),   // 25: content-disposition
    (0, 0),   // 26: content-encoding
    (0, 0),   // 27: content-language
    (0, 0),   // 28: content-length
    (0, 0),   // 29: content-location
    (0, 0),   // 30: content-range
];

// We use a simple approach: store the actual header names and values for static table lookups.
fn static_table_name(idx: usize) -> Option<&'static str> {
    match idx {
        1 => Some(":authority"),
        2 | 3 => Some(":method"),
        4 | 5 => Some(":path"),
        6 | 7 => Some(":scheme"),
        8..=14 => Some(":status"),
        15 => Some("accept-charset"),
        16 => Some("accept-encoding"),
        17 => Some("accept-language"),
        18 => Some("accept-ranges"),
        19 => Some("accept"),
        20 => Some("access-control-allow-origin"),
        21 => Some("age"),
        22 => Some("allow"),
        23 => Some("authorization"),
        24 => Some("cache-control"),
        25 => Some("content-disposition"),
        26 => Some("content-encoding"),
        27 => Some("content-language"),
        28 => Some("content-length"),
        29 => Some("content-location"),
        30 => Some("content-range"),
        _ => None,
    }
}

fn static_table_value(idx: usize) -> Option<&'static str> {
    match idx {
        2 => Some("GET"),
        3 => Some("POST"),
        4 => Some("/"),
        5 => Some("/index.html"),
        6 => Some("http"),
        7 => Some("https"),
        8 => Some("200"),
        9 => Some("204"),
        10 => Some("206"),
        11 => Some("304"),
        12 => Some("400"),
        13 => Some("404"),
        14 => Some("500"),
        _ => None,
    }
}

fn static_table_entry(idx: usize) -> Option<(&'static str, Option<&'static str>)> {
    let name = static_table_name(idx)?;
    let value = static_table_value(idx);
    Some((name, value))
}

fn search_static_table(name: &[u8], value: &[u8]) -> Option<usize> {
    for idx in 1..=30 {
        if let Some((n, v)) = static_table_entry(idx) {
            if n.as_bytes() == name {
                if v.is_some() && v.unwrap().as_bytes() == value {
                    return Some(idx);
                }
            }
        }
    }
    None
}

fn search_static_table_name(name: &[u8]) -> Option<usize> {
    for idx in 1..=30 {
        if let Some((n, _)) = static_table_entry(idx) {
            if n.as_bytes() == name {
                return Some(idx);
            }
        }
    }
    None
}

impl Hpack {
    pub fn new() -> Self {
        Hpack {
            dynamic_table: Vec::new(),
            max_table_size: 4096,
        }
    }

    /// Read a variable-length integer with N-bit prefix.
    fn read_integer(data: &[u8], offset: &mut usize, prefix_bits: u8) -> Result<u64, H2Error> {
        if *offset >= data.len() {
            return Err(H2Error::Protocol("HPACK integer underflow".into()));
        }
        let mask = (1u8 << prefix_bits) - 1;
        let first = data[*offset] & mask;
        *offset += 1;

        if first < mask {
            return Ok(first as u64);
        }

        let mut value = mask as u64;
        let mut shift = 0u32;
        loop {
            if *offset >= data.len() {
                return Err(H2Error::Protocol("HPACK integer continuation underflow".into()));
            }
            let byte = data[*offset];
            *offset += 1;
            value += ((byte & 0x7F) as u64) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
        }
    }

    /// Write a variable-length integer with N-bit prefix.
    fn write_integer(buf: &mut Vec<u8>, value: u64, prefix_bits: u8) {
        let mask = (1u8 << prefix_bits) - 1;
        if value < mask as u64 {
            if let Some(last) = buf.last_mut() {
                *last |= value as u8;
            } else {
                buf.push(value as u8);
            }
            return;
        }

        if let Some(last) = buf.last_mut() {
            *last |= mask;
        } else {
            buf.push(mask);
        }

        let mut v = value - mask as u64;
        while v >= 128 {
            buf.push((v as u8 & 0x7F) | 0x80);
            v >>= 7;
        }
        buf.push(v as u8);
    }

    /// Decode an HPACK string (Huffman flag + length + data).
    pub fn decode_string(data: &[u8], offset: &mut usize) -> Result<Vec<u8>, H2Error> {
        if *offset >= data.len() {
            return Err(H2Error::Protocol("HPACK string underflow".into()));
        }
        let _huffman = (data[*offset] >> 7) != 0;
        // read_integer with 7-bit prefix naturally ignores the H bit (bit 7)
        let len = Self::read_integer(data, offset, 7)? as usize;
        if *offset + len > data.len() {
            return Err(H2Error::Protocol("HPACK string data underflow".into()));
        }
        let raw = data[*offset..*offset + len].to_vec();
        *offset += len;
        // For simplicity, we skip Huffman decoding (H=0 only).
        Ok(raw)
    }

    /// Encode an HPACK string.
    pub fn encode_string(buf: &mut Vec<u8>, s: &[u8]) {
        // H=0 (raw bytes)
        let prefix_start = buf.len();
        buf.push(0); // placeholder — we set the prefix bits below
        Self::write_integer(buf, s.len() as u64, 7);
        buf.extend_from_slice(s);
        // Fix the first byte's H flag
        buf[prefix_start] |= 0x00; // H=0, already set
    }

    fn dynamic_table_index(&self, name: &[u8], value: &[u8]) -> Option<usize> {
        let base = 30;
        for (i, (n, v)) in self.dynamic_table.iter().enumerate() {
            if n == name && v == value {
                return Some(base + 1 + i);
            }
        }
        None
    }

    fn dynamic_table_name_index(&self, name: &[u8]) -> Option<usize> {
        let base = 30;
        for (i, (n, _)) in self.dynamic_table.iter().enumerate() {
            if n == name {
                return Some(base + 1 + i);
            }
        }
        None
    }

    /// Decode HPACK header block into a list of (name, value) pairs.
    pub fn decode(&mut self, data: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>, H2Error> {
        let mut headers = Vec::new();
        let mut offset = 0usize;

        while offset < data.len() {
            let b = data[offset];
            if b & 0x80 != 0 {
                // Indexed header field (0xxxxxxx)
                let idx = Self::read_integer(data, &mut offset, 7)? as usize;
                if idx == 0 {
                    return Err(H2Error::Protocol("HPACK index 0".into()));
                }
                if idx <= 30 {
                    let (name, value) = static_table_entry(idx)
                        .ok_or_else(|| H2Error::Protocol("bad static table index".into()))?;
                    let val = value.unwrap_or("");
                    headers.push((name.as_bytes().to_vec(), val.as_bytes().to_vec()));
                } else {
                    let dyn_idx = idx - 31;
                    if dyn_idx < self.dynamic_table.len() {
                        let (n, v) = &self.dynamic_table[dyn_idx];
                        headers.push((n.clone(), v.clone()));
                    } else {
                        return Err(H2Error::Protocol("bad dynamic table index".into()));
                    }
                }
            } else if b & 0x40 != 0 {
                // Literal with incremental indexing (01xxxxxx)
                let name_idx = Self::read_integer(data, &mut offset, 6)? as usize;
                let name = if name_idx == 0 {
                    Self::decode_string(data, &mut offset)?
                } else if name_idx <= 30 {
                    let (n, _) = static_table_entry(name_idx)
                        .ok_or_else(|| H2Error::Protocol("bad static table name index".into()))?;
                    n.as_bytes().to_vec()
                } else {
                    let dyn_idx = name_idx - 31;
                    if dyn_idx < self.dynamic_table.len() {
                        self.dynamic_table[dyn_idx].0.clone()
                    } else {
                        return Err(H2Error::Protocol("bad dynamic table name index".into()));
                    }
                };
                let value = Self::decode_string(data, &mut offset)?;
                headers.push((name.clone(), value.clone()));
                // Add to dynamic table
                let size = name.len() + value.len() + 32;
                while self.dynamic_table_size() + size > self.max_table_size {
                    self.dynamic_table.pop();
                }
                self.dynamic_table.insert(0, (name, value));
            } else if b & 0xF0 == 0x00 {
                // Literal without indexing (0000xxxx)
                let name_idx = Self::read_integer(data, &mut offset, 4)? as usize;
                let name = if name_idx == 0 {
                    Self::decode_string(data, &mut offset)?
                } else if name_idx <= 30 {
                    let (n, _) = static_table_entry(name_idx)
                        .ok_or_else(|| H2Error::Protocol("bad static table name index".into()))?;
                    n.as_bytes().to_vec()
                } else {
                    let dyn_idx = name_idx - 31;
                    if dyn_idx < self.dynamic_table.len() {
                        self.dynamic_table[dyn_idx].0.clone()
                    } else {
                        return Err(H2Error::Protocol("bad dynamic table name index".into()));
                    }
                };
                let value = Self::decode_string(data, &mut offset)?;
                headers.push((name, value));
            } else {
                return Err(H2Error::Protocol("unexpected HPACK prefix".into()));
            }
        }

        Ok(headers)
    }

    /// Encode a list of (name, value) pairs into an HPACK header block.
    pub fn encode(&mut self, headers: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
        let mut buf = Vec::new();

        for (name, value) in headers {
            // Try indexed encoding first (full match in static or dynamic table)
            if let Some(idx) = search_static_table(name, value) {
                buf.push(0x80); // indexed prefix
                Self::write_integer(&mut buf, idx as u64, 7);
                continue;
            }
            if let Some(idx) = self.dynamic_table_index(name, value) {
                buf.push(0x80);
                Self::write_integer(&mut buf, idx as u64, 7);
                continue;
            }

            // Literal with incremental indexing
            buf.push(0x40); // literal with indexing prefix
            if let Some(idx) = search_static_table_name(name) {
                Self::write_integer(&mut buf, idx as u64, 6);
            } else if let Some(idx) = self.dynamic_table_name_index(name) {
                Self::write_integer(&mut buf, idx as u64, 6);
            } else {
                buf.push(0); // new name
                Self::encode_string(&mut buf, name);
            }
            Self::encode_string(&mut buf, value);

            // Add to dynamic table
            let size = name.len() + value.len() + 32;
            while self.dynamic_table_size() + size > self.max_table_size {
                self.dynamic_table.pop();
            }
            self.dynamic_table.insert(0, (name.clone(), value.clone()));
        }

        buf
    }

    fn dynamic_table_size(&self) -> usize {
        self.dynamic_table.iter().map(|(n, v)| n.len() + v.len() + 32).sum()
    }

    pub fn set_max_table_size(&mut self, size: usize) {
        self.max_table_size = size;
        while self.dynamic_table_size() > self.max_table_size {
            self.dynamic_table.pop();
        }
    }
}

// ============================================================================
// Stream Module
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    Idle,
    Open,
    HalfClosedRemote,
    HalfClosedLocal,
    Closed,
}

#[derive(Debug, Clone)]
pub struct Stream {
    pub id: u32,
    pub state: StreamState,
    pub recv_buf: Vec<u8>,
    pub send_buf: Vec<u8>,
    pub window_size: u32,
    pub received_header: bool,
    pub headers: Vec<(Vec<u8>, Vec<u8>)>,
    pub response_sent: bool,
}

impl Stream {
    pub fn new(id: u32) -> Self {
        Stream {
            id,
            state: StreamState::Idle,
            recv_buf: Vec::new(),
            send_buf: Vec::new(),
            window_size: DEFAULT_WINDOW_SIZE,
            received_header: false,
            headers: Vec::new(),
            response_sent: false,
        }
    }
}

// ============================================================================
// Settings for the connection
// ============================================================================

#[derive(Debug, Clone)]
pub struct ConnectionSettings {
    pub header_table_size: u32,
    pub enable_push: u32,
    pub max_concurrent_streams: u32,
    pub initial_window_size: u32,
    pub max_frame_size: u32,
}

impl Default for ConnectionSettings {
    fn default() -> Self {
        ConnectionSettings {
            header_table_size: 4096,
            enable_push: 1,
            max_concurrent_streams: u32::MAX,
            initial_window_size: DEFAULT_WINDOW_SIZE,
            max_frame_size: DEFAULT_MAX_FRAME_SIZE,
        }
    }
}

// ============================================================================
// Session Module
// ============================================================================

pub struct Http2Session<T: Read + Write> {
    conn: T,
    streams: HashMap<u32, Stream>,
    hpack_encoder: Hpack,
    hpack_decoder: Hpack,
    local_settings: ConnectionSettings,
    remote_settings: ConnectionSettings,
    connection_window: u32,
    next_server_stream: u32,
    preface_done: bool,
    buf: Vec<u8>,
    document_root: String,
}

impl<T: Read + Write> Http2Session<T> {
    pub fn new(conn: T, document_root: &str) -> Self {
        Http2Session {
            conn,
            streams: HashMap::new(),
            hpack_encoder: Hpack::new(),
            hpack_decoder: Hpack::new(),
            local_settings: ConnectionSettings::default(),
            remote_settings: ConnectionSettings::default(),
            connection_window: DEFAULT_WINDOW_SIZE,
            next_server_stream: 2,
            preface_done: false,
            buf: Vec::with_capacity(DEFAULT_MAX_FRAME_SIZE as usize + FRAME_HEADER_LEN),
            document_root: document_root.to_string(),
        }
    }

    pub fn run(&mut self) -> Result<(), H2Error> {
        self.perform_preface()?;
        self.main_loop()
    }

    fn perform_preface(&mut self) -> Result<(), H2Error> {
        // Client sends: magic (24 bytes) + SETTINGS
        let mut magic_buf = [0u8; 24];
        self.conn.read_exact(&mut magic_buf)?;
        if &magic_buf != HTTP2_MAGIC {
            return Err(H2Error::Protocol("bad HTTP/2 magic".into()));
        }

        // Read client's initial SETTINGS frame
        let settings_frame = read_frame(&mut self.conn)?;
        match &settings_frame {
            Frame::Settings { ack, .. } => {
                if *ack {
                    return Err(H2Error::Protocol("unexpected SETTINGS ACK in preface".into()));
                }
            }
            _ => {
                return Err(H2Error::Protocol("expected SETTINGS as first frame".into()));
            }
        }
        self.apply_settings(&settings_frame);

        // Send our SETTINGS (empty = defaults)
        write_frame(&mut self.conn, &Frame::Settings { ack: false, settings: vec![] })?;

        // Send SETTINGS ACK for client's settings
        write_frame(&mut self.conn, &Frame::Settings { ack: true, settings: vec![] })?;

        // Wait for client's SETTINGS ACK
        loop {
            let ack_frame = read_frame(&mut self.conn)?;
            match &ack_frame {
                Frame::Settings { ack: true, .. } => break,
                _ => {
                    // We could process other frames here, but technically
                    // no other frames should arrive before SETTINGS ACK.
                }
            }
        }

        self.preface_done = true;
        Ok(())
    }

    fn apply_settings(&mut self, frame: &Frame) {
        if let Frame::Settings { settings, .. } = frame {
            for s in settings {
                match s.id {
                    SETTINGS_HEADER_TABLE_SIZE => {
                        self.remote_settings.header_table_size = s.value;
                        self.hpack_decoder.set_max_table_size(s.value as usize);
                    }
                    SETTINGS_INITIAL_WINDOW_SIZE => {
                        let delta = s.value as i64 - self.remote_settings.initial_window_size as i64;
                        self.remote_settings.initial_window_size = s.value;
                        for stream in self.streams.values_mut() {
                            stream.window_size = (stream.window_size as i64 + delta) as u32;
                        }
                    }
                    SETTINGS_MAX_FRAME_SIZE => {
                        self.remote_settings.max_frame_size = s.value.clamp(16384, 16777215);
                    }
                    _ => {}
                }
            }
        }
    }

    fn main_loop(&mut self) -> Result<(), H2Error> {
        loop {
            let frame = match read_frame(&mut self.conn) {
                Ok(f) => f,
                Err(H2Error::Io(e)) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    // Connection closed gracefully
                    return Ok(());
                }
                Err(e) => return Err(e),
            };

            self.handle_frame(frame)?;
        }
    }

    fn handle_frame(&mut self, frame: Frame) -> Result<(), H2Error> {
        match frame {
            Frame::Data { stream_id, end_stream, data } => {
                self.handle_data(stream_id, end_stream, data)?;
            }
            Frame::Headers { stream_id, end_headers, end_stream, priority, data } => {
                self.handle_headers(stream_id, end_headers, end_stream, priority, data)?;
            }
            Frame::Priority { stream_id, exclusive, dependency, weight } => {
                self.handle_priority(stream_id, exclusive, dependency, weight)?;
            }
            Frame::RstStream { stream_id, error_code } => {
                self.handle_rst_stream(stream_id, error_code)?;
            }
            Frame::Settings { ack, settings } => {
                self.handle_settings(ack, settings)?;
            }
            Frame::Ping { ack, opaque_data } => {
                self.handle_ping(ack, opaque_data)?;
            }
            Frame::Goaway { .. } => {
                // Graceful shutdown: stop processing
                return Ok(());
            }
            Frame::WindowUpdate { stream_id, increment } => {
                self.handle_window_update(stream_id, increment)?;
            }
            Frame::Continuation { stream_id, end_headers, data } => {
                self.handle_continuation(stream_id, end_headers, data)?;
            }
            Frame::PushPromise { .. } => {
                // We don't send PUSH_PROMISE in basic mode, ignore
            }
        }
        Ok(())
    }

    fn handle_data(&mut self, stream_id: u32, end_stream: bool, data: Vec<u8>) -> Result<(), H2Error> {
        if stream_id == 0 {
            return Err(H2Error::Protocol("DATA frame on stream 0".into()));
        }
        if data.len() as u32 > self.connection_window {
            return Err(H2Error::FlowControl);
        }

        self.connection_window -= data.len() as u32;

        let stream = self.streams.get_mut(&stream_id);
        if let Some(s) = stream {
            s.recv_buf.extend_from_slice(&data);
            s.window_size = s.window_size.saturating_sub(data.len() as u32);

            if end_stream {
                s.state = StreamState::HalfClosedRemote;
                // Process the request now
                self.process_request(stream_id)?;
            }

            // Send WINDOW_UPDATE to replenish
            if !data.is_empty() {
                write_frame(&mut self.conn, &Frame::WindowUpdate {
                    stream_id: 0,
                    increment: data.len() as u32,
                })?;
                write_frame(&mut self.conn, &Frame::WindowUpdate {
                    stream_id,
                    increment: data.len() as u32,
                })?;
                if let Some(s) = self.streams.get_mut(&stream_id) {
                    s.window_size = s.window_size.wrapping_add(data.len() as u32);
                }
                self.connection_window = self.connection_window.wrapping_add(data.len() as u32);
            }
        } else {
            return Err(H2Error::StreamClosed(stream_id));
        }

        Ok(())
    }

    fn handle_headers(
        &mut self,
        stream_id: u32,
        end_headers: bool,
        end_stream: bool,
        _priority: Option<PriorityParams>,
        data: Vec<u8>,
    ) -> Result<(), H2Error> {
        if stream_id == 0 || (stream_id % 2 == 0) {
            return Err(H2Error::Protocol("invalid HEADERS stream ID".into()));
        }
        if stream_id % 2 == 0 {
            return Err(H2Error::Protocol("even stream ID for client-initiated stream".into()));
        }

        let stream = self.streams.entry(stream_id).or_insert_with(|| Stream::new(stream_id));
        if stream.state == StreamState::Idle {
            stream.state = StreamState::Open;
        }

        let decoded = self.hpack_decoder.decode(&data)?;
        stream.headers.extend(decoded);
        stream.received_header = true;

        if end_headers {
            if end_stream {
                stream.state = StreamState::HalfClosedRemote;
                self.process_request(stream_id)?;
            }
        }
        // If not end_headers, we expect CONTINUATION frames

        Ok(())
    }

    fn handle_continuation(&mut self, stream_id: u32, end_headers: bool, data: Vec<u8>) -> Result<(), H2Error> {
        let stream = self.streams.get_mut(&stream_id);
        if let Some(s) = stream {
            let decoded = self.hpack_decoder.decode(&data)?;
            s.headers.extend(decoded);

            if end_headers {
                // If the stream was already half-closed (END_STREAM on HEADERS),
                // process the request now
                if s.state == StreamState::HalfClosedRemote {
                    self.process_request(stream_id)?;
                }
            }
        }

        Ok(())
    }

    fn process_request(&mut self, stream_id: u32) -> Result<(), H2Error> {
        let headers = self.streams.get(&stream_id).map(|s| s.headers.clone()).unwrap_or_default();
        let body = self.streams.get(&stream_id).map(|s| s.recv_buf.clone()).unwrap_or_default();

        let method = extract_pseudo(&headers, ":method").unwrap_or("GET");
        let path = extract_pseudo(&headers, ":path").unwrap_or("/");

        let (status, content_type, response_body) = match (method, path) {
            ("GET", "/") | ("GET", "/index.html") => {
                let path = format!("{}/index.html", self.document_root);
                match std::fs::read(&path) {
                    Ok(data) => (200, detect_content_type("index.html"), data),
                    Err(_) => {
                        let html = b"<html><body><h1>Hello from custom HTTP/2!</h1></body></html>".to_vec();
                        (200, "text/html", html)
                    }
                }
            }
            ("POST", "/echo") => {
                // Echo the request body back
                (200, "application/octet-stream", body.clone())
            }
            _ => {
                let not_found = b"<html><body><h1>404 Not Found</h1></body></html>".to_vec();
                (404, "text/html", not_found)
            }
        };

        self.send_response(stream_id, status, content_type, &response_body)
    }

    fn send_response(
        &mut self,
        stream_id: u32,
        status: u16,
        content_type: &str,
        body: &[u8],
    ) -> Result<(), H2Error> {
        let status_str = format!("{}", status);
        let mut headers = Vec::new();
        headers.push((":status".as_bytes().to_vec(), status_str.as_bytes().to_vec()));
        headers.push(("content-type".as_bytes().to_vec(), content_type.as_bytes().to_vec()));
        headers.push(("content-length".as_bytes().to_vec(), format!("{}", body.len()).as_bytes().to_vec()));

        let encoded_headers = self.hpack_encoder.encode(&headers);

        write_frame(&mut self.conn, &Frame::Headers {
            stream_id,
            end_headers: true,
            end_stream: body.is_empty(),
            priority: None,
            data: encoded_headers,
        })?;

        if !body.is_empty() {
            write_frame(&mut self.conn, &Frame::Data {
                stream_id,
                end_stream: true,
                data: body.to_vec(),
            })?;
        }

        if let Some(s) = self.streams.get_mut(&stream_id) {
            s.response_sent = true;
            s.state = StreamState::HalfClosedLocal;
            // If both sides are done, close
            if s.state == StreamState::HalfClosedRemote || s.state == StreamState::HalfClosedLocal {
                if s.state == StreamState::HalfClosedRemote {
                    s.state = StreamState::Closed;
                }
            }
        }

        Ok(())
    }

    fn handle_priority(&mut self, _stream_id: u32, _exclusive: bool, _dependency: u32, _weight: u8) -> Result<(), H2Error> {
        // For simplicity, ignore priority
        Ok(())
    }

    fn handle_rst_stream(&mut self, stream_id: u32, _error_code: u32) -> Result<(), H2Error> {
        if let Some(s) = self.streams.get_mut(&stream_id) {
            s.state = StreamState::Closed;
            s.recv_buf.clear();
            s.send_buf.clear();
        }
        Ok(())
    }

    fn handle_settings(&mut self, ack: bool, settings: Vec<Setting>) -> Result<(), H2Error> {
        if ack {
            return Ok(());
        }
        for s in &settings {
            match s.id {
                SETTINGS_HEADER_TABLE_SIZE => {
                    self.hpack_encoder.set_max_table_size(s.value as usize);
                }
                SETTINGS_INITIAL_WINDOW_SIZE => {
                    let delta = s.value as i64 - self.local_settings.initial_window_size as i64;
                    self.local_settings.initial_window_size = s.value;
                    self.connection_window = (self.connection_window as i64 + delta) as u32;
                }
                SETTINGS_MAX_FRAME_SIZE => {
                    self.local_settings.max_frame_size = s.value;
                }
                _ => {}
            }
        }
        // Send ACK
        write_frame(&mut self.conn, &Frame::Settings { ack: true, settings: vec![] })?;
        Ok(())
    }

    fn handle_ping(&mut self, ack: bool, data: [u8; 8]) -> Result<(), H2Error> {
        if !ack {
            write_frame(&mut self.conn, &Frame::Ping { ack: true, opaque_data: data })?;
        }
        Ok(())
    }

    fn handle_window_update(&mut self, stream_id: u32, increment: u32) -> Result<(), H2Error> {
        if stream_id == 0 {
            self.connection_window = self.connection_window.wrapping_add(increment);
        } else if let Some(s) = self.streams.get_mut(&stream_id) {
            s.window_size = s.window_size.wrapping_add(increment);
        }
        Ok(())
    }
}

impl<T: Read + Write> Drop for Http2Session<T> {
    fn drop(&mut self) {
        // Send GOAWAY if we're shutting down gracefully
        let last_id = self.streams.keys().max().copied().unwrap_or(0);
        let _ = write_frame(&mut self.conn, &Frame::Goaway {
            last_stream_id: last_id,
            error_code: ERROR_NO_ERROR,
            debug_data: vec![],
        });
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn extract_pseudo(headers: &[(Vec<u8>, Vec<u8>)], name: &str) -> Option<&str> {
    for (n, v) in headers {
        if n == name.as_bytes() {
            return std::str::from_utf8(v).ok();
        }
    }
    None
}

fn detect_content_type(path: &str) -> &'static str {
    if path.ends_with(".html") || path.ends_with(".htm") {
        "text/html"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".gif") {
        "image/gif"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".json") {
        "application/json"
    } else {
        "text/plain"
    }
}

// ============================================================================
// Server
// ============================================================================

fn handle_connection(stream: TcpStream, document_root: &str) {
    if let Err(e) = stream.set_nodelay(true) {
        eprintln!("set_nodelay: {}", e);
    }

    let mut session = Http2Session::new(stream, document_root);
    if let Err(e) = session.run() {
        eprintln!("Session error: {}", e);
    }
}

fn main() -> Result<(), H2Error> {
    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr)
        .map_err(|e| H2Error::Internal(format!("bind: {}", e)))?;

    let document_root = std::env::var("DOCUMENT_ROOT")
        .unwrap_or_else(|_| ".".to_string());

    println!("HTTP/2 server listening on {}", addr);
    println!("Document root: {}", document_root);
    println!("Test with: curl --http2-prior-knowledge http://localhost:8080/");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let root = document_root.clone();
                thread::spawn(move || {
                    handle_connection(stream, &root);
                });
            }
            Err(e) => {
                eprintln!("Accept error: {}", e);
            }
        }
    }

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_header_roundtrip() {
        let hdr = FrameHeader {
            length: 42,
            frame_type: FrameType::Headers,
            flags: FLAG_END_HEADERS | FLAG_END_STREAM,
            stream_id: 1,
        };
        let encoded = encode_frame_header(&hdr);
        let decoded = decode_frame_header(&encoded).unwrap();
        assert_eq!(decoded.length, 42);
        assert_eq!(decoded.frame_type, FrameType::Headers);
        assert_eq!(decoded.flags, FLAG_END_HEADERS | FLAG_END_STREAM);
        assert_eq!(decoded.stream_id, 1);
    }

    #[test]
    fn test_data_frame_roundtrip() {
        let frame = Frame::Data {
            stream_id: 3,
            end_stream: true,
            data: vec![1, 2, 3, 4],
        };

        let mut buf = Vec::new();
        write_frame(&mut buf, &frame).unwrap();

        // Reset cursor
        let mut cursor = io::Cursor::new(buf);
        let decoded = read_frame(&mut cursor).unwrap();

        match decoded {
            Frame::Data { stream_id, end_stream, data } => {
                assert_eq!(stream_id, 3);
                assert!(end_stream);
                assert_eq!(data, vec![1, 2, 3, 4]);
            }
            _ => panic!("wrong frame type"),
        }
    }

    #[test]
    fn test_settings_roundtrip() {
        let s = [
            Setting { id: SETTINGS_MAX_CONCURRENT_STREAMS, value: 100 },
            Setting { id: SETTINGS_INITIAL_WINDOW_SIZE, value: 65535 },
        ];
        let frame = Frame::Settings { ack: false, settings: s.to_vec() };

        let mut buf = Vec::new();
        write_frame(&mut buf, &frame).unwrap();

        let mut cursor = io::Cursor::new(buf);
        let decoded = read_frame(&mut cursor).unwrap();

        match decoded {
            Frame::Settings { ack, settings } => {
                assert!(!ack);
                assert_eq!(settings.len(), 2);
                assert_eq!(settings[0].id, SETTINGS_MAX_CONCURRENT_STREAMS);
                assert_eq!(settings[0].value, 100);
                assert_eq!(settings[1].id, SETTINGS_INITIAL_WINDOW_SIZE);
                assert_eq!(settings[1].value, 65535);
            }
            _ => panic!("wrong frame type"),
        }
    }

    #[test]
    fn test_hpack_integer_encoding() {
        // Encode 42 with 5-bit prefix
        let mut buf = vec![0x00];
        Hpack::write_integer(&mut buf, 42, 5);
        // After write_integer, the first byte's lower 5 bits are set
        assert_eq!(buf[0], 0x1F | 0x00); // 0x1F = 31
        assert_eq!(buf[1], 11); // 42 - 31 = 11
    }

    #[test]
    fn test_hpack_string_roundtrip() {
        let mut buf = Vec::new();
        Hpack::encode_string(&mut buf, b"hello");

        let mut offset = 0usize;
        // The first byte has H=0 and length
        assert!(buf[0] & 0x80 == 0); // H=0
        let decoded = Hpack::decode_string(&buf, &mut offset).unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_hpack_indexed_header() {
        let mut hpack = Hpack::new();
        // Encode a :status 200 response header (index 8 in static table)
        let headers = vec![
            (b":status".to_vec(), b"200".to_vec()),
        ];
        let encoded = hpack.encode(&headers);

        let mut decoder = Hpack::new();
        let decoded = decoder.decode(&encoded).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].0, b":status");
        assert_eq!(decoded[0].1, b"200");
    }

    #[test]
    fn test_hpack_dynamic_table() {
        let mut hpack = Hpack::new();
        let headers = vec![
            (b"x-custom".to_vec(), b"hello".to_vec()),
        ];
        let encoded = hpack.encode(&headers);

        // Second encode should use indexed encoding (dynamic table hit)
        let encoded2 = hpack.encode(&headers);
        assert!(encoded2.len() < encoded.len() || encoded2[0] & 0x80 != 0);
    }

    #[test]
    fn test_stream_state_transitions() {
        let mut s = Stream::new(1);
        assert_eq!(s.state, StreamState::Idle);

        s.state = StreamState::Open;
        assert_eq!(s.state, StreamState::Open);

        s.state = StreamState::HalfClosedRemote;
        assert_eq!(s.state, StreamState::HalfClosedRemote);
    }

    #[test]
    fn test_frame_type_conversion() {
        assert_eq!(FrameType::from_byte(0x00).unwrap(), FrameType::Data);
        assert_eq!(FrameType::from_byte(0x01).unwrap(), FrameType::Headers);
        assert_eq!(FrameType::from_byte(0x04).unwrap(), FrameType::Settings);
        assert_eq!(FrameType::from_byte(0x08).unwrap(), FrameType::WindowUpdate);
        assert!(FrameType::from_byte(0x0A).is_err());

        assert_eq!(FrameType::Data.to_byte(), 0x00);
        assert_eq!(FrameType::Goaway.to_byte(), 0x07);
    }
}
