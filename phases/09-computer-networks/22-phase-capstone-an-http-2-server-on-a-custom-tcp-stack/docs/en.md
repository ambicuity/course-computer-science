# Phase Capstone — An HTTP/2 Server on a Custom TCP Stack

> Build a complete HTTP/2 server from scratch on top of your own userspace TCP/IP stack — Ethernet frames to HTTP/2 frames.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 09 lessons 01–21
**Time:** ~150 minutes

## Learning Objectives

- Implement HTTP/2 frame encoding/decoding from the raw binary wire format
- Implement HTTP/2 stream multiplexing over a single TCP connection
- Implement a simplified HPACK header compressor/decompressor
- Integrate the HTTP/2 layer with the userspace TCP/IP stack from lesson 11
- Build a working HTTP/2 server that serves static content and echoes request bodies
- Compare your implementation against production HTTP/2 stacks (h2, nghttp2)

## The Problem

Lessons 01–20 each built one layer of the network stack: Ethernet frames, IPv4 packets, TCP segments, sockets, HTTP/1.1, TLS, DNS, and async I/O with tokio. You have all the pieces sitting on your workbench.

But knowing individual layers is not the same as understanding how they compose. An Ethernet frame carries an IP packet that carries a TCP segment that carries bytes that the other end interprets as HTTP/2 frames — but *how does that actually work in running code?* How does a `TcpStream::read()` return bytes that become a `HEADERS` frame with `:path: /index.html`?

This capstone ties every layer together. You will build a single binary that reads raw data from a TCP connection, walks up the protocol stack, and serves HTTP/2 responses. When you are done, `curl --http2-prior-knowledge http://localhost:8080/` will return a web page served by *your* code, from your Ethernet-frame parser up to your HPACK encoder.

## The Concept

### HTTP/2 Frame Wire Format

HTTP/2 is a **binary protocol**. Every message on the wire is a **frame** with a 9-byte header followed by a variable-length payload:

```
HTTP/2 Frame (9-byte header + payload):
┌──────────────────────────────┐
│ Length (24 bits)             │
├──────────────┬───────────────┤
│ Type (8)     │ Flags (8)     │
├──────────────┴───────────────┤
│ R│ Stream Identifier (31 bits)│
├──────────────────────────────┤
│ Payload (variable length)    │
└──────────────────────────────┘
```

**Length** (24 bits): The size of the frame payload in bytes. Does not include the 9-byte header. The maximum is 2^24 - 1 (16,777,215 bytes), but the default maximum frame size is 16,384 bytes (2^14). Both endpoints can increase this via the `SETTINGS_MAX_FRAME_SIZE` setting.

**Type** (8 bits): Determines how to interpret the payload.

| Type         | Code | Purpose                                      |
|--------------|------|----------------------------------------------|
| DATA         | 0x0  | Carries arbitrary bytes (request/response body) |
| HEADERS      | 0x1  | Carries header field block (request/response headers) |
| PRIORITY     | 0x2  | Specifies stream priority and dependencies   |
| RST_STREAM   | 0x3  | Terminates a stream immediately (error)      |
| SETTINGS     | 0x4  | Connection-level parameters                  |
| PUSH_PROMISE | 0x5  | Server promises a future stream              |
| PING         | 0x6  | Round-trip time measurement / keep-alive     |
| GOAWAY       | 0x7  | Graceful connection shutdown                 |
| WINDOW_UPDATE| 0x8  | Flow control credit update                   |
| CONTINUATION | 0x9  | Continuation of a HEADERS/PUSH_PROMISE block |

**Flags** (8 bits): Type-specific. The common ones:

| Flag             | Value | Applies to    | Meaning                                  |
|------------------|-------|---------------|------------------------------------------|
| END_STREAM       | 0x1   | DATA, HEADERS | Last frame of this stream                |
| END_HEADERS      | 0x4   | HEADERS, CONTINUATION, PUSH_PROMISE | Header block is complete |
| ACK              | 0x1   | SETTINGS, PING | Acknowledgement of received frame       |
| PADDED           | 0x8   | DATA, HEADERS, PUSH_PROMISE | Frame has padding bytes       |
| PRIORITY         | 0x20  | HEADERS        | Frame includes priority fields           |

**Stream Identifier** (31 bits): Identifies which stream this frame belongs to. Stream 0 (reserved) is for connection-level frames (SETTINGS, PING, GOAWAY). Client-initiated streams have odd IDs; server-initiated streams have even IDs.

### Stream Multiplexing

HTTP/2 multiplexes **multiple concurrent streams** over a single TCP connection. Each stream is an independent bidirectional flow of frames.

```
┌─────────────────────────────────────────────────────┐
│                  TCP Connection                      │
│                                                      │
│  Stream 1: ──── HEADERS ──── DATA ──┐               │
│                                      │               │
│  Stream 3: ──── HEADERS ────────────┼─── HEADERS ── │
│                                      │               │
│  Stream 5: ──── HEADERS ── DATA ════╧═════════════  │
│                                                      │
│  Frames from different streams are interleaved       │
│  on the wire. No head-of-line blocking at the        │
│  application layer.                                  │
└─────────────────────────────────────────────────────┘
```

**Stream state machine**:

```
                    ┌──────────┐
              PP    │   Idle   │
         ┌─────────►│          │◄─────────┐
         │          └──────────┘          │
         │               │H              PP
         │               ▼                │
         │          ┌──────────┐          │
         │          │  Open    │          │
         │          │          │          │
         │     ES   └────┬─────┘   ES     │
         │    ┌──────────┼──────────┐     │
         │    ▼          ▼          ▼     │
         │ ┌──────┐  ┌──────┐  ┌──────┐  │
         │ │H.Clsd│  │ Open │  │R.Clsd│  │
         │ │Local │  │      │  │Remote│  │
         │ └──┬───┘  └──┬───┘  └──┬───┘  │
         │    │         │         │      │
         │    └──────┬──┘    ┌────┘      │
         │           ▼       ▼           │
         │          ┌──────────┐          │
         └──────────│  Closed  │◄─────────┘
                    └──────────┘
  H  = HEADERS frame sent
  ES = END_STREAM flag
  PP = PUSH_PROMISE (server push)
```

Key insight: a stream is **half-closed** in one direction when that direction sends END_STREAM. The other direction can still send data. The stream is **closed** when both sides have sent END_STREAM or when either side sends RST_STREAM.

### Frame Types in Detail

**DATA frame** (Type 0x0):
```
+---------------+
|Pad Length? (8)|
+---------------+-----------------------------------------------+
|                            Data (*)                         ...
+---------------------------------------------------------------+
|                           Padding (*)                        ...
+---------------------------------------------------------------+
```
Flags: END_STREAM (0x1), PADDED (0x8)

**HEADERS frame** (Type 0x1):
```
+---------------+
|Pad Length? (8)|
+-+-------------+-----------------------------------------------+
|E|                 Stream Dependency? (31)                      |
+-+-------------+-----------------------------------------------+
|  Weight? (8)  |
+-+-------------+-----------------------------------------------+
|                   Header Block Fragment (*)                  ...
+---------------------------------------------------------------+
|                           Padding (*)                        ...
+---------------------------------------------------------------+
```
Flags: END_STREAM (0x1), END_HEADERS (0x4), PADDED (0x8), PRIORITY (0x20)

The Header Block Fragment is an HPACK-encoded block of header fields (explained below).

**SETTINGS frame** (Type 0x4):
```
+-------------------------------+
|       Identifier (16)         |
+-------------------------------+-------------------------------+
|                        Value (32)                             |
+---------------------------------------------------------------+
```
Each SETTINGS frame contains zero or more settings. Each setting is 6 bytes (2-byte ID + 4-byte value).

| Setting                | ID | Default | Description                          |
|------------------------|----|---------|--------------------------------------|
| SETTINGS_HEADER_TABLE_SIZE | 1 | 4096   | HPACK dynamic table max size         |
| SETTINGS_ENABLE_PUSH  | 2  | 1       | Enable/disable server push           |
| SETTINGS_MAX_CONCURRENT_STREAMS | 3 | unlimited | Max open streams          |
| SETTINGS_INITIAL_WINDOW_SIZE | 4 | 65535  | Initial stream flow control window |
| SETTINGS_MAX_FRAME_SIZE | 5 | 16384   | Max payload size for a single frame  |
| SETTINGS_MAX_HEADER_LIST_SIZE | 6 | unlimited | Max uncompressed header size |

**WINDOW_UPDATE frame** (Type 0x8):
```
+-+-------------------------------------------------------------+
|R|              Window Size Increment (31)                     |
+-+-------------------------------------------------------------+
```
Used for flow control. Each side maintains a window of how many bytes the other side can send. Every DATA frame consumes window. WINDOW_UPDATE replenishes it.

**GOAWAY frame** (Type 0x7):
```
+-+-------------------------------------------------------------+
|R|                  Last-Stream-ID (31)                        |
+-+-------------------------------------------------------------+
|                      Error Code (32)                          |
+---------------------------------------------------------------+
|                  Additional Debug Data (*)                    |
+---------------------------------------------------------------+
```
Used for graceful shutdown. The sender will not process frames with stream ID > Last-Stream-ID.

**PING frame** (Type 0x6):
```
+---------------------------------------------------------------+
|                                                               |
|                      Opaque Data (64)                         |
|                                                               |
+---------------------------------------------------------------+
```
Heartbeat / RTT measurement. Receivers must respond with a PING + ACK containing the same 8 bytes.

### HPACK Header Compression

HTTP/2 compresses headers using **HPACK** (RFC 7541). HPACK eliminates redundancy in two ways:

1. **Static table**: 61 predefined header table entries (common headers like `:method: GET`, `:status: 200`, `content-type`).

2. **Dynamic table**: Entries the connection adds over time (recently seen headers).

**Indexed encoding**: If a header matches a table entry, send just the index.

```
Indexed Header Field (1 byte for common headers):
┌─┬─────────────────────┐
│0│      Index (7+)     │
└─┴─────────────────────┘
Example: `:method: GET` is at index 2 → byte 0x82
```

**Literal encoding**: If the header is not in the table, encode the name and value inline:

```
Literal with Incremental Indexing:
┌─┬─┬───────────────────┐
│0│1│     Index (6+)     │  ← name index (or 0 for new name)
├─┼─┴───────────────────┤
│  Name String (if index=0)│
├────────────────────────┤
│  Value String          │
└────────────────────────┘
```

**Integer encoding** (HPACK's building block): Variable-length encoding using N-bit prefixes. If the value fits in N bits, done. Otherwise, set all N bits to 1, then encode remaining value in 7-bit chunks with continuation bit.

```
Example: encode 42 with 5-bit prefix:
  42 < 31? No. Write 31 (0x1F), then encode 42-31=11
  11 < 128? Yes. Write 11 as 0x0B
  Result: 0x1F 0x0B
```

**String encoding**:
```
┌────┬────────────────────┐
│ H  │  Length (7+)       │
├────┴────────────────────┤
│  String Data (Length bytes)│
└─────────────────────────┘
H = 1 means Huffman encoded, H = 0 means raw bytes.
```

For simplicity, our implementation uses H=0 (raw bytes) only.

### Worked Example: Compressing `:path: /index.html`

1. Look up `:path` in static table: index 4 (value is `/index.html`).
2. This is an exact match (both name and value exist in the table).
3. Encode as indexed header: `0x80 | 4 = 0x84`.
4. Result: 1 byte instead of 21 bytes.

Worked example: compressing `content-type: application/json`:

1. `content-type` has no exact match. Look up name: static table index 31.
2. Encode as literal with incremental indexing: name from table (index 31), value inline.
3. Name: `01` prefix + 6-bit index 31 = `0x5F`.
4. Value string: `application/json` — need to encode length (16) and data.
5. Encode 16 with 7-bit prefix: 16 < 127, single byte `0x10`.
6. Value data: `application/json`.
7. Result: `0x5F 0x10 "application/json"` = 20 bytes.

Without HPACK, the raw headers would be 50+ bytes. With the dynamic table, subsequent requests on the same connection might encode `content-type: application/json` as a single indexed byte.

### Flow Control

HTTP/2 provides **per-stream** and **per-connection** flow control. Each stream starts with an initial window size (default 65,535 bytes). A sender cannot send more DATA bytes than the receiver's current window. The receiver grants more credit via WINDOW_UPDATE frames.

```
Connection window:     [████████████░░░░░░░░░░]  35,535 / 65,535
Stream 1 window:      [████████████████░░░░░░]  45,000 / 65,535
Stream 3 window:      [██░░░░░░░░░░░░░░░░░░░░]   5,000 / 65,535
```

This prevents a fast sender from overwhelming a slow receiver — the receiver controls its own buffer occupancy.

Note: HTTP/2 flow control prevents **application-layer head-of-line blocking**, which solves the HTTP/1.1 pipelining problem. However, HTTP/2 still suffers from **TCP-level head-of-line blocking**: if a TCP packet is lost, all streams stall until it is retransmitted. This is the problem QUIC solves (Lesson 09).

### Integration with the Custom TCP Stack

The userspace TCP/IP stack from Lesson 11 provides a `Stream`-like interface: you accept TCP connections, read payload bytes from the receive buffer, and write response bytes to the send buffer. Our HTTP/2 session wraps this layer:

```
┌───────────────────────────────────────────────────────┐
│                    HTTP/2 Server                       │
│  ┌─────────────────────────────────────────────┐       │
│  │           Http2Session<T: Read+Write>        │       │
│  │  ┌──────┐  ┌──────┐  ┌──────┐  ┌─────────┐ │       │
│  │  │Frame │→ │HPACK │→ │Stream│→ │Handler  │ │       │
│  │  │Parser│  │Decode│  │Mgr   │  │Router   │ │       │
│  │  └──────┘  └──────┘  └──────┘  └─────────┘ │       │
│  └─────────────────────────────────────────────┘       │
│                         │                              │
│  ┌──────────────────────▼────────────────────────┐     │
│  │  TcpStream (std) or UserspaceTcpStream (toy)  │     │
│  │  Implements Read + Write                      │     │
│  └───────────────────────────────────────────────┘     │
└───────────────────────────────────────────────────────┘
```

The key design: `Http2Session` is generic over `T: Read + Write`. At this point you can plug in either `std::net::TcpStream` (for testing) or the userspace TCP stack's connection type (for the full stack). The HTTP/2 layer does not care what provides the byte stream — it just reads frames and writes frames.

## Build It

### Step 1: Frame Parser

Write a module that reads the 9-byte frame header and decodes the payload into typed structs.

**Frame header decoding:**
```
Read 9 bytes from TCP stream:
  length   = (buf[0] << 16) | (buf[1] << 8) | buf[2]
  type     = buf[3]
  flags    = buf[4]
  stream_id = buf[5..9] & 0x7FFFFFFF

Read `length` more bytes as payload.
Decode payload based on `type`.
```

Key edge cases:
- Frame length must not exceed `SETTINGS_MAX_FRAME_SIZE` (default 16,384)
- Stream ID must not be 0x80000000 or higher (31-bit limit)
- Fragmentation: HEADERS can span multiple CONTINUATION frames (use the END_HEADERS flag)

**Frame header encoding:**
```
Write 9 bytes:
  length as 3 bytes big-endian
  type byte
  flags byte
  stream_id as 4 bytes big-endian with top bit cleared
Write payload bytes.
```

### Step 2: Connection Preface and SETTINGS Exchange

When an HTTP/2 connection opens, the client sends a **connection preface**:

```
Magic bytes (24 bytes):
  PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n

Followed immediately by a SETTINGS frame (with at least 0 settings).
```

The server must:
1. Read and verify the magic bytes
2. Read the client's initial SETTINGS frame
3. Apply the settings
4. Send its own SETTINGS frame
5. Send a SETTINGS ACK for the client's settings

```
Client                              Server
  │                                    │
  ├── Magic (24 bytes) ───────────────►│
  ├── SETTINGS (empty) ───────────────►│
  │                                    ├── Verify magic
  │                                    ├── Apply settings
  │◄── SETTINGS (empty) ───────────────┤
  │◄── SETTINGS ACK ───────────────────┤
  ├── SETTINGS ACK ──────────────────►│
  │                                    │
  │◄══════ Normal frame exchange ═════►│
```

The server must wait for the client's SETTINGS ACK before sending DATA frames for pushed streams (this ensures the client's initial window is known). For simplicity, our implementation will send settings immediately and proceed.

### Step 3: Stream Multiplexer

Maintain a `HashMap<u32, Stream>` mapping stream IDs to active stream state.

```rust
struct Stream {
    id: u32,
    state: StreamState,
    recv_buf: Vec<u8>,    // Accumulated DATA payloads
    send_buf: Vec<u8>,    // Queued response data
    window_size: u32,     // Flow control window
    headers: Vec<Header>, // Parsed request headers
    content_length: u64,  // Expected body length
}

enum StreamState {
    Idle,
    Open,
    HalfClosedRemote, // Client sent END_STREAM
    HalfClosedLocal,  // Server sent END_STREAM
    Closed,
}
```

When a HEADERS frame arrives for a new stream ID:
1. Create a new `Stream` in the `Idle` state
2. Transition to `Open`
3. Decode HPACK block → extract `:method`, `:path`, `:scheme`, `:authority`, and other headers
4. If the frame has `END_STREAM`, transition to `HalfClosedRemote`

When a DATA frame arrives for an existing stream:
1. Append payload to `recv_buf`
2. Update flow control window
3. If the frame has `END_STREAM`, transition to `HalfClosedRemote`

When the server has a response ready:
1. Encode response headers with HPACK
2. Send HEADERS frame (with END_HEADERS)
3. Send DATA frame(s) (with END_STREAM)
4. Transition to `HalfClosedLocal` (or `Closed` if both sides are done)

**Stream prioritization**: For simplicity, use round-robin. When multiple streams have data to send, alternate between them. The production version uses the PRIORITY frame and dependency tree.

### Step 4: HPACK (Simplified)

Implement a minimal HPACK module:

**Static table**: Store the first 31 entries from RFC 7541. Each entry has an index, a name, and an optional value.

```rust
struct HeaderField {
    name: &'static str,
    value: &'static str,
}

static STATIC_TABLE: &[HeaderField] = &[
    HeaderField { name: ":authority",     value: "" },
    HeaderField { name: ":method",        value: "GET" },
    HeaderField { name: ":method",        value: "POST" },
    HeaderField { name: ":path",          value: "/" },
    HeaderField { name: ":path",          value: "/index.html" },
    HeaderField { name: ":scheme",        value: "http" },
    HeaderField { name: ":scheme",        value: "https" },
    HeaderField { name: ":status",        value: "200" },
    // ... more entries
];
```

**Dynamic table**: A `Vec<(Vec<u8>, Vec<u8>)>` that entries are pushed to the front. Max size is controlled by SETTINGS_HEADER_TABLE_SIZE (default 4,096).

**Integer encoding** (5-bit and 7-bit prefix variants):
- 5-bit prefix: used for indexed header fields and literal name indices
- 7-bit prefix: used for string lengths

**String encoding**: For simplicity, skip Huffman encoding. A string is:
```
H (1 bit) = 0 (raw)
Length (7+ bits, variable-length integer)
Data (Length bytes)
```

**Header encoding**:
- Indexed: `0` prefix + 7-bit index
- Literal with incremental indexing: `01` prefix + 6-bit name index + value string
- Literal without indexing: `0000` prefix + 4-bit name index + value string

**Decoder**: Given a byte slice, produce a `Vec<(Vec<u8>, Vec<u8>)>` of decoded headers.

**Encoder**: Given a list of `(name, value)` pairs, produce a byte slice. Check static + dynamic table for existing entries. Use indexed encoding for hits, literal with indexing for new entries.

### Step 5: Request Router and Response

When a complete request is received (all HEADERS + CONTINUATION frames, possibly with DATA):

1. Parse pseud headers (`:method`, `:path`, `:scheme`, `:authority`)
2. Route based on `:path`:
   - `/` → serve `index.html` from document root
   - `/echo` → echo back request body
   - `/file/*` → serve static file
   - Other → 404 Not Found

3. Build response headers:
   - `:status: 200` (or appropriate status)
   - `content-type: text/html` (or from file extension)
   - `content-length: N` (body size)

4. Encode response headers with HPACK, send as HEADERS frame

5. Send response body in one or more DATA frames

6. If `:method` is HEAD, send only the HEADERS, no DATA

### Step 6: Tie Everything to the Custom TCP Stack

The `Http2Session` struct is generic over `T: Read + Write`:

```rust
struct Http2Session<T: Read + Write> {
    conn: T,
    streams: HashMap<u32, Stream>,
    hpack_encoder: HpackEncoder,
    hpack_decoder: HpackDecoder,
    next_stream_id: u32,
    settings: Settings,
    connection_window: u32,
}

impl<T: Read + Write> Http2Session<T> {
    fn new(conn: T) -> Self { /* ... */ }
    async fn run(&mut self) -> Result<()> { /* ... */ }
}
```

The server main loop:

```rust
fn main() -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;

    for stream in listener.incoming() {
        let stream = stream?;
        stream.set_nodelay(true)?;
        std::thread::spawn(move || {
            let mut session = Http2Session::new(stream);
            if let Err(e) = session.run() {
                eprintln!("Session error: {}", e);
            }
        });
    }
    Ok(())
}
```

To adapt to the userspace TCP stack (Lesson 11), replace `TcpListener` and `TcpStream` with the stack's equivalents. The stack provides a `TcpListener::accept()` that returns a `TcpStream`-like object implementing `Read + Write`. The `Http2Session` works unchanged.

## Use It

Compare your implementation with production HTTP/2 stacks:

**h2 crate** (Rust, used by hyper/warp):
- Full HPACK with Huffman encoding
- Complete state machine with PRIORITY frames
- Flow control with connection and stream windows
- Graceful shutdown with GOAWAY
- ~30,000 lines of Rust
- Your implementation: ~800 lines, same concepts, minimal edge cases

**nghttp2** (C, reference implementation):
- Used by curl, Apache httpd, nginx
- Asynchronous I/O, event-driven
- Complete HPACK, including Huffman encoding
- Stream priority tree (not just round-robin)
- Your implementation follows the same architecture: frame → stream → HPACK

**Go's HTTP/2**:
- Built into `net/http` since Go 1.6
- Goroutine-per-stream model (similar to your thread-per-connection but lighter)
- Same frame types, same HPACK, same state machine
- Go's `golang.org/x/net/http2/hpack` package mirrors your Hpack struct

**Key differences your toy implementation skips**:
- Huffman encoding in HPACK (reduces header size ~30%)
- Stream dependencies and priorities (PRIORITY frames)
- Server push (PUSH_PROMISE frames)
- Connection-level flow control accounting
- CONTINUATION frame fragmentation of large header blocks
- Many SETTINGS parameters (max concurrent streams, max header list size)
- GOAWAY with graceful drain

## Read the Source

- **RFC 7540** — the HTTP/2 specification. Section 4: "Frame Format", Section 5: "Streams and Multiplexing", Section 6: "Frame Definitions".
- **RFC 7541** — HPACK: Header Compression for HTTP/2. Section 5: "Header Table", Section 6: "Binary Format", Appendix A: "Static Table".
- **h2 crate** (`github.com/hyperium/h2`): `src/frame/` for frame parsing, `src/hpack/` for HPACK, `src/proto/streams/` for the stream state machine.
- **nghttp2** (`github.com/nghttp2/nghttp2`): `lib/nghttp2_frame.c` for frame building, `lib/nghttp2_hd.c` for HPACK, `lib/nghttp2_stream.c` for stream management.
- **golang.org/x/net/http2** — Go's HTTP/2. `framer.go` for frame I/O, `hpack/` for HPACK.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **An HTTP/2 server on a custom userspace TCP/IP stack** — a complete, working server that combines Ethernet-frame parsing (Lesson 11), TCP state machine (Lesson 11), and HTTP/2 framing (this lesson) into a single runnable binary.

## Exercises

### Easy: Add HTTP/2 Server Push

When a client requests `GET /`, check if the document root contains `index.html` and/or `style.css`. If so:
1. Send a PUSH_PROMISE frame on stream 1 (the index request) promising stream 2 (the CSS)
2. Use stream ID 2 (even = server-initiated)
3. Send HEADERS + DATA for stream 2 with the CSS content
4. Verify the client receives both resources

Server push is disabled by default in modern browsers due to complexity, but implementing it teaches the PUSH_PROMISE frame format.

### Medium: Implement Per-Stream Flow Control

Currently the server ignores WINDOW_UPDATE frames. Implement flow control:
1. Track the connection-level window (initially 65,535 bytes)
2. Track per-stream windows
3. Before sending DATA, check the window for that stream AND the connection
4. Only send min(window_remaining, min(MTU, 16384)) bytes per DATA frame
5. Process incoming WINDOW_UPDATE frames: add the increment to the appropriate window
6. The server should also send WINDOW_UPDATE frames when it reads DATA from the client (every N bytes, grant N more credit)

Test: send a large POST body and verify the server pauses/resumes based on window availability.

### Hard: Implement HPACK Huffman Encoding

RFC 7541 Appendix B defines a Huffman code table for 256 symbols. Implement:
1. Build the Huffman encoding table (symbol → (code_bitstring, code_length))
2. Implement `huffman_encode(data: &[u8]) -> Vec<u8>` that encodes a byte string using the Huffman table
3. Implement `huffman_decode(data: &[u8]) -> Result<Vec<u8>>` that decodes back
4. In the HPACK encoder, set H=1 for strings and use Huffman encoding when it reduces size
5. In the HPACK decoder, check H and decode accordingly

Huffman encoding typically reduces header size by 30-40%. Validate by comparing raw vs encoded `content-type: application/json` or a typical cookie header.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Frame | "HTTP/2 frame" | The smallest unit of communication — a 9-byte header + typed payload |
| Stream | "HTTP/2 stream" | An independent bidirectional flow of frames, identified by a 31-bit integer |
| Multiplexing | "Stream multiplexing" | Sending frames from multiple streams interleaved on a single TCP connection |
| HPACK | "Header compression" | Compression scheme using static + dynamic tables that eliminates redundant headers |
| Preface | "Connection preface" | The mandatory 24-byte magic string + initial SETTINGS sent at connection start |
| Stream identifier | "Stream ID" | 31-bit odd/even number identifying a stream; client uses odd, server uses even |
| Flow control | "Per-stream flow control" | Mechanism preventing one stream from consuming all connection buffer space |
| Server push | "HTTP/2 push" | Server speculatively sends resources the client will need (PUSH_PROMISE + response) |
| Connection window | "Connection-level flow control" | Total bytes the receiver is willing to accept across all streams |
| WINDOW_UPDATE | "Window update" | Frame that grants additional flow control credit to the sender |
| GOAWAY | "Go away" | Frame signaling graceful connection shutdown — tells client which streams were processed |
| SETTINGS | "Settings frame" | Connection-level parameters exchanged at the start of a connection |
| END_STREAM | "End stream flag" | Flag indicating no more data will be sent on this stream (half-close) |
| HEADERS | "Headers frame" | Frame type carrying HPACK-encoded header fields |
| DATA | "Data frame" | Frame type carrying arbitrary bytes (the body) |

## Further Reading

- RFC 7540 — HTTP/2
- RFC 7541 — HPACK: Header Compression for HTTP/2
- [http2 explained](https://http2-explained.haxx.se/) — Daniel Stenberg's free book on HTTP/2
- *HTTP/2 in Action* by Barry Pollard — comprehensive guide to HTTP/2 with examples
- [h2 crate documentation](https://docs.rs/h2/latest/h2/) — Rust HTTP/2 implementation docs
- [nghttp2 documentation](https://nghttp2.org/documentation/) — C reference implementation docs
- [HTTP/2 spec visualizer](https://http2.akamai.com/) — Akamai's HTTP/2 frame inspector
