# Lesson 13: HTTP/1.1, HTTP/2 — Wire Format and Multiplexing

## Overview

HTTP (HyperText Transfer Protocol) is the application-layer protocol that powers the web. Understanding its wire format and evolution from HTTP/1.1 to HTTP/2 reveals how protocol design directly impacts performance, latency, and scalability.

## HTTP/1.1: Text-Based Protocol

HTTP/1.1 communicates using human-readable text. Every message is either a **request** or a **response**.

### Request Format

```
METHOD /path HTTP/1.1\r\n
Header-Name: Header-Value\r\n
\r\n
optional body
```

Example:

```
GET /index.html HTTP/1.1\r\n
Host: www.example.com\r\n
User-Agent: Mozilla/5.0\r\n
Accept: text/html\r\n
Connection: keep-alive\r\n
\r\n
```

Key components:
- **Method**: GET, POST, PUT, DELETE, HEAD, OPTIONS
- **Request URI**: the resource path (`/index.html`)
- **HTTP version**: `HTTP/1.1`
- **Headers**: key-value pairs terminated by `\r\n`
- **Body**: optional, used with POST/PUT

### Response Format

```
HTTP/1.1 STATUS_CODE REASON_PHRASE\r\n
Header-Name: Header-Value\r\n
\r\n
response body
```

Example:

```
HTTP/1.1 200 OK\r\n
Content-Type: text/html\r\n
Content-Length: 1024\r\n
Connection: keep-alive\r\n
\r\n
<html>...</html>
```

Status codes follow a hierarchy:
- **1xx**: Informational (100 Continue, 101 Switching Protocols)
- **2xx**: Success (200 OK, 201 Created)
- **3xx**: Redirection (301 Moved Permanently, 304 Not Modified)
- **4xx**: Client Error (400 Bad Request, 404 Not Found)
- **5xx**: Server Error (500 Internal Server Error, 503 Service Unavailable)

## Persistent Connections and Pipelining

HTTP/1.0 opened a new TCP connection per request. HTTP/1.1 introduced **persistent connections** (`Connection: keep-alive`) — the TCP connection stays open for multiple request-response cycles.

**Pipelining** allows sending multiple requests without waiting for each response. However, responses must arrive in order — this creates **head-of-line blocking**. If the first response is slow, all subsequent responses are delayed even if they are ready. Browsers disabled pipelining due to this limitation.

### Chunked Transfer Encoding

When the server does not know the total content length upfront, it uses chunked transfer encoding:

```
HTTP/1.1 200 OK\r\n
Transfer-Encoding: chunked\r\n
\r\n
5\r\n
Hello\r\n
6\r\n
World!\r\n
0\r\n
\r\n
```

Each chunk is preceded by its size in hexadecimal. The final chunk has size `0`.

## HTTP/2: Binary Framing Layer

HTTP/2 fundamentally changes how data is transmitted while keeping the same HTTP semantics (methods, status codes, headers).

### Binary Frames

Every HTTP/2 message is encoded as binary **frames**. The frame header is 9 bytes:

```
+---+---+---+---+---+---+---+---+---+
| Length (24 bits) | Type (8) | Flags (8) | R | Stream ID (31 bits) |
+---+---+---+---+---+---+---+---+---+
```

- **Length**: payload size (2^14 default max, up to 2^24-1)
- **Type**: DATA (0x0), HEADERS (0x1), PRIORITY (0x2), RST_STREAM (0x3), SETTINGS (0x4), PUSH_PROMISE (0x5), PING (0x6), GOAWAY (0x7), WINDOW_UPDATE (0x8), CONTINUATION (0x9)
- **Flags**: type-specific flags
- **Stream ID**: identifies the logical stream

### Streams and Multiplexing

A **stream** is a bidirectional flow of frames within a connection, identified by a unique stream ID. Multiplexing means multiple streams share a single TCP connection simultaneously — no head-of-line blocking at the application layer.

Stream ID rules:
- Client-initiated streams use odd IDs
- Server-initiated streams use even IDs
- Stream 0 is reserved for connection-level control

### HPACK Header Compression

HTTP/2 compresses headers using **HPACK**, which maintains a dynamic table on both sides. Frequently used headers are indexed and referenced by number instead of being retransmitted. This reduces overhead significantly for requests with repetitive headers (cookies, user-agent, etc.).

### Server Push

The server can proactively send resources the client will need before the client requests them. The server sends a `PUSH_PROMISE` frame on an existing stream, then sends response frames on a new stream.

### Priority and Dependencies

Clients can assign priorities to streams and declare dependencies (stream B depends on stream A). The server uses this to allocate bandwidth optimally.

## Build It: Parsers

See the code files for:
- **main.rs**: HTTP/1.1 request parser, response builder, HTTP/2 frame parser
- **main.py**: HTTP/1.1 parser, HTTP/2 frame structure demo, HPACK simulation

## Use It

Modern browsers default to HTTP/2 when available. The `h2` ALPN identifier negotiates HTTP/2 during TLS handshake. HTTP/1.1 remains used for legacy servers and simple APIs.

## Ship It

Building a protocol parser teaches you exactly what bytes go on the wire. Tools like Wireshark let you capture and inspect real HTTP/2 frames.

## Exercises

### Level 1 — Recall

1. What is the size of an HTTP/2 frame header?
2. Name three HTTP/1.1 methods.
3. What does `Transfer-Encoding: chunked` indicate?

### Level 2 — Application

4. Write a function that parses an HTTP/1.1 request string and extracts the method, path, and headers.
5. Given a hex dump of an HTTP/2 HEADERS frame, decode the stream ID and flags.
6. Explain why HTTP/2 multiplexing eliminates head-of-line blocking at the application layer.

### Level 3 — Creation

7. Implement an HTTP/1.1 response builder that supports chunked transfer encoding.
8. Design a simplified HPACK encoder that replaces common header names with single-byte indices.
9. Build a connection coalescing detector: given two hostnames and their certificates, determine if they can share an HTTP/2 connection.
