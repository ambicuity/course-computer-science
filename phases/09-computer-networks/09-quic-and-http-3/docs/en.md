# QUIC and HTTP/3

> QUIC and HTTP/3 — the part of CS you can't skip.

**Type:** Learn
**Languages:** Rust
**Prerequisites:** Phase 09 lessons 01–08
**Time:** ~75 minutes

## Learning Objectives

- Understand the core concept introduced in this lesson and why it matters.
- Implement the lesson's "Build It" artifact from scratch in one of: Rust.
- Compare your from-scratch implementation against the production tool used in industry.
- Ship the reusable artifact (see "Ship It") and add it to your toolbox.

## The Problem

This lesson sits in **Phase 09 — Computer Networks**. Without the concept it teaches, you cannot
build the phase's capstone (An HTTP/2 server on a custom userspace TCP/IP stack.). Concretely, *not* knowing this means you get stuck the
moment you try to build the stack: ethernet, ip, tcp, tls, http — by hand.

The next few sections walk through the smallest concrete scenario where this gap hurts, then build
the mental model, then the code, then the production equivalent.

## The Concept

### Problems with TCP

TCP has served the internet for 40+ years, but its design creates friction for modern applications:

1. **Head-of-line blocking**: TCP delivers bytes in order. If segment 5 is lost, segments 6–100
   sit in the receive buffer waiting. A single lost packet stalls the entire connection.
2. **Handshake latency**: TCP needs a 3-way handshake (1-RTT), then TLS adds another 1-2 RTTs.
   Total: 2–3 RTTs before any application data flows.
3. **No multiplexing**: TCP is a single byte stream. HTTP/2 multiplexes streams over one TCP
   connection, but a single TCP loss stalls all HTTP streams simultaneously.
4. **Connection migration**: TCP connections are identified by (src_ip, src_port, dst_ip, dst_port).
   When your phone switches from WiFi to cellular, the IP changes and the TCP connection breaks.

### QUIC: A New Transport Protocol

QUIC (RFC 9000, 2021) was designed by Google (2012) to solve all four problems. It runs over
**UDP** but provides reliable, ordered, encrypted transport.

Key properties:

```
┌─────────────────────────────────────────────────────┐
│  Application (HTTP/3)                               │
├─────────────────────────────────────────────────────┤
│  QUIC (streams, reliability, encryption)            │
├─────────────────────────────────────────────────────┤
│  UDP (datagrams)                                    │
├─────────────────────────────────────────────────────┤
│  IP                                                │
└─────────────────────────────────────────────────────┘
```

**0-RTT connection establishment**: If the client has connected to the server before, it can
send application data in the very first packet using cached TLS session keys. Even a fresh
connection requires only 1-RTT (combining transport and TLS handshake).

```
TCP+TLS:  Client ──SYN──────────→ Server     (1 RTT)
          Client ←──SYN+ACK────── Server
          Client ──ACK───────────→ Server
          Client ──ClientHello────→ Server    (1-2 RTTs)
          Client ←──ServerHello─── Server
          Client ──Finished───────→ Server
          Client ──HTTP GET───────→ Server    ← data finally flows

QUIC:     Client ──Initial (w/ ClientHello)→ Server (1 RTT)
          Client ←──Handshake────────────── Server
          Client ──HTTP GET─────────────────→ Server ← data flows

QUIC 0-RTT: Client ──0-RTT (w/ HTTP GET)→ Server  ← data in first packet!
```

**Independent streams**: QUIC multiplexes multiple streams within a single connection. Each
stream has its own flow control and ordering. Loss on stream 3 does NOT block stream 5.

```
TCP+HTTP/2:   [Stream1|Stream2|Stream3|Stream4] → single TCP byte stream
              Segment 3 lost → ALL streams wait

QUIC+HTTP/3:  Stream1 → [pkt1][pkt2][pkt3]  ← independent
              Stream2 → [pkt1][pkt2][pkt3]  ← independent
              Stream3 → [pkt1][pkt2][pkt3]  ← independent
              Stream 3 pkt2 lost → only Stream 3 waits
```

**Connection migration**: QUIC connections are identified by a **connection ID**, not by the
5-tuple. When the client's IP changes, it sends a packet with the same connection ID on the
new path. The server validates the new address and continues the connection seamlessly.

### HTTP/3

HTTP/3 is simply HTTP semantics (request/response, headers, status codes) running over QUIC
instead of TCP+TLS. The features are identical to HTTP/2:

- Request multiplexing (now over QUIC streams, not TCP segments)
- Header compression (QPACK instead of HPACK — adapted for out-of-order delivery)
- Server push
- Same binary framing semantics

The key difference: head-of-line blocking at the transport layer is eliminated. If one QUIC
stream's packet is lost, other streams continue unaffected.

## Build It

### Step 1: Simplified QUIC Connection

A simulation of QUIC's connection lifecycle — 0-RTT handshake, independent streams,
and connection migration.

```rust
// code/main.rs — see the full file for a runnable version
```

The simulation demonstrates:
- Creating a connection with a unique connection ID.
- 0-RTT handshake: sending application data in the first packet.
- Opening multiple independent streams.
- Migrating a connection to a new address.

## Use It

**In production:** Google Search, YouTube, and Google's CDN use QUIC/HTTP/3 for the majority
of their traffic. Cloudflare enables HTTP/3 by default. Facebook and Instagram use it extensively.

**QUIC libraries:**
- **quiche** (Cloudflare) — Rust/C: `github.com/cloudflare/quiche`
- **quinn** (Rust): `github.com/quinn-rs/quinn`
- **msquic** (Microsoft): C implementation used by Windows and .NET
- **quic-go** (Google): Go implementation

**RFC 9000** defines the QUIC transport protocol. **RFC 9114** defines HTTP/3.
**RFC 9001** defines the TLS 1.3 integration.

Your simulation captures the core abstraction: connection IDs, independent stream state,
and the 0-RTT optimization. The production version adds cryptographic handshake,
congestion control, packet number encryption, and loss recovery.

## Read the Source

- Cloudflare quiche `src/lib.rs` — `connect()` creates a QUIC connection; `stream_send()` writes to a stream.
- RFC 9000 Section 17 — Packet formats and connection establishment.
- RFC 9114 Section 4 — HTTP/3 connection lifecycle.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A QUIC demo** — connection simulation showing streams and migration.

## Exercises

1. **Easy** — Extend the `QuicConnection` simulation to track the number of bytes sent
   and received per stream. Print a summary table at the end showing per-stream statistics.
2. **Medium** — Simulate head-of-line blocking: model a TCP connection where 5 HTTP streams
   share one byte stream. When packet 10 (stream 2) is lost, show that streams 1, 3, 4, 5
   all stall until packet 10 is retransmitted. Compare this against the QUIC version where
   only stream 2 stalls.
3. **Hard** — Implement a simplified QPACK header compression: maintain a dynamic table of
   (name, value) pairs indexed by integer. Compress headers by replacing known entries with
   their index. Handle the case where streams arrive out of order and the dynamic table
   reference hasn't been synchronized yet (blocked decoding).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| QUIC | "HTTP/3 transport" | UDP-based transport protocol with reliability, encryption, and stream multiplexing (RFC 9000) |
| HTTP/3 | "HTTP over QUIC" | HTTP semantics (request/response, headers) running over QUIC instead of TCP+TLS |
| 0-RTT | "Zero round trip" | Sending application data in the first packet using cached session keys from a previous connection |
| Connection ID | "CID" | Opaque identifier in QUIC packets that uniquely identifies a connection, independent of IP/port |
| Connection migration | "Seamless handoff" | Client changes IP (WiFi→cellular) without breaking the QUIC connection, identified by connection ID |
| Head-of-line blocking | "HOL blocking" | A lost packet in an ordered stream blocks all data behind it; QUIC eliminates this at transport level |
| Stream | "Byte stream channel" | Independent, ordered byte sequence within a QUIC connection with its own flow control |
| QPACK | "Header compression for HTTP/3" | HPACK variant adapted for QUIC's out-of-order stream delivery |
| TLS 1.3 | "Modern encryption" | Cryptographic handshake integrated into QUIC; not layered separately like TCP+TLS |
| Multiplexing | "Multiple streams" | Running multiple logical data flows over a single connection without inter-stream interference |

## Further Reading

- RFC 9000 — QUIC: A UDP-Based Multiplexed and Secure Transport
- RFC 9114 — HTTP/3
- RFC 9001 — Using TLS to Secure QUIC
- "HTTP/3 Explained" by Daniel Stenberg (free online book)
- Google's QUIC paper: "QUIC: A UDP-Based Secure and Reliable Transport for HTTP/2" (2016)
