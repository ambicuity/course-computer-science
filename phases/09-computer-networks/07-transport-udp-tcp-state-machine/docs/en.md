# Transport — UDP, TCP State Machine

> Transport — UDP, TCP State Machine — the part of CS you can't skip.

**Type:** Learn
**Languages:** C, Rust
**Prerequisites:** Phase 09 lessons 01–06
**Time:** ~90 minutes

## Learning Objectives

- Understand the core concept introduced in this lesson and why it matters.
- Implement the lesson's "Build It" artifact from scratch in one of: C, Rust.
- Compare your from-scratch implementation against the production tool used in industry.
- Ship the reusable artifact (see "Ship It") and add it to your toolbox.

## The Problem

This lesson sits in **Phase 09 — Computer Networks**. Without the concept it teaches, you cannot
build the phase's capstone (An HTTP/2 server on a custom userspace TCP/IP stack.). Concretely, *not* knowing this means you get stuck the
moment you try to build the stack: ethernet, ip, tcp, tls, http — by hand.

The next few sections walk through the smallest concrete scenario where this gap hurts, then build
the mental model, then the code, then the production equivalent.

## The Concept

The transport layer sits between the network layer (IP) and the application layer (HTTP, DNS, etc.).
It provides **end-to-end communication** between processes on different hosts. Two protocols dominate:
UDP (User Datagram Protocol) and TCP (Transmission Control Protocol).

### UDP — Fire and Forget

UDP is connectionless and unreliable. It adds **four fields** to the payload:

| Field | Size | Purpose |
|-------|------|---------|
| Source Port | 16 bits | Sending process identifier |
| Destination Port | 16 bits | Receiving process identifier |
| Length | 16 bits | Total header + data length |
| Checksum | 16 bits | Optional error detection |

No connection setup. No acknowledgments. No ordering guarantees. Packets may arrive out of order,
be duplicated, or disappear entirely. The application must handle reliability if needed.

**Use cases:** DNS queries (512 bytes, one round trip), video streaming (late packets are worthless),
real-time gaming (old state is stale), QUIC (built on UDP — see lesson 09).

### TCP — Reliable Stream

TCP is connection-oriented. Every byte gets a **sequence number**. The receiver sends **acknowledgments**.
The TCP header adds these critical fields:

| Field | Size | Purpose |
|-------|------|---------|
| Source Port | 16 bits | Sending process |
| Destination Port | 16 bits | Receiving process |
| Sequence Number | 32 bits | First byte in this segment |
| Acknowledgment Number | 32 bits | Next byte expected |
| Data Offset | 4 bits | Header length |
| Flags | 6 bits | URG, ACK, PSH, RST, SYN, FIN |
| Window Size | 16 bits | Receive buffer space |
| Checksum | 16 bits | Mandatory error detection |

### The TCP State Machine

A TCP connection transitions through a fixed set of states. Here is the complete state machine:

```
CLOSED
  │
  ├── Client calls connect() ──→ SYN_SENT ──(recv SYN+ACK, send ACK)──→ ESTABLISHED
  │
  ├── Server calls listen() ──→ LISTEN ──(recv SYN, send SYN+ACK)──→ SYN_RCVD
  │                                    └──(recv ACK)──→ ESTABLISHED
  │
  ESTABLISHED
  │
  ├── call close() ──→ FIN_WAIT_1 ──(recv ACK)──→ FIN_WAIT_2
  │                                            └──(recv FIN, send ACK)──→ TIME_WAIT
  │                                                                     └──(2*MSL)──→ CLOSED
  │
  ├── recv FIN, send ACK ──→ CLOSE_WAIT ──(call close(), send FIN)──→ LAST_ACK
  │                                                                    └──(recv ACK)──→ CLOSED
```

### Three-Way Handshake

```
Client                          Server
  │                                │
  │──── SYN (seq=x) ─────────────→│
  │                                │
  │←─── SYN+ACK (seq=y, ack=x+1) ─│
  │                                │
  │──── ACK (ack=y+1) ───────────→│
  │                                │
  │      ESTABLISHED               │ ESTABLISHED
```

1. **SYN**: Client sends initial sequence number `x`.
2. **SYN+ACK**: Server acknowledges `x+1`, sends its own sequence number `y`.
3. **ACK**: Client acknowledges `y+1`. Both sides are now synchronized.

### Four-Way Teardown

```
Client                          Server
  │                                │
  │──── FIN (seq=u) ─────────────→│
  │←─── ACK (ack=u+1) ───────────│
  │                                │
  │            (server finishes)   │
  │                                │
  │←─── FIN (seq=v) ─────────────│
  │──── ACK (ack=v+1) ───────────→│
  │                                │
  │      TIME_WAIT                 │ CLOSED
```

Each direction closes independently. The initiator enters **TIME_WAIT** (typically 2×MSL = 60s)
to catch late packets from the old connection.

## Build It

### Step 1: Minimal UDP Echo (C)

A bare-bones UDP echo server and client. The server receives a datagram and sends it back.

```c
// code/main.c — see the full file for a runnable version
```

Key steps:
- `socket(AF_INET, SOCK_DGRAM, 0)` — create a UDP socket.
- `bind()` — attach to a port.
- `recvfrom()` / `sendto()` — read and write datagrams with address info.

### Step 2: TCP State Machine (Rust)

An enum-driven TCP state machine that simulates the full lifecycle:

```rust
// code/tcp_state.rs — see the full file for a runnable version
```

The `handle_event()` method maps `(current_state, event) → next_state`. Invalid transitions
panic or return an error. The `three_way_handshake()` and `four_way_teardown()` functions
drive the state machine through the complete lifecycle.

## Use It

**In production:** Linux implements TCP in `net/ipv4/tcp_input.c` and `net/ipv4/tcp_output.c`.
The state machine lives in `tcp_set_state()` — grep for it and you will find the exact
transition table. The UDP implementation is much simpler: `net/ipv4/udp.c` is roughly 2000 lines
compared to TCP's 50,000+.

**RFC 793** defines TCP. Section 3.2 ("Sequence Numbers") and Section 3.4 ("Establishing a
Connection") contain the state machine diagram. Section 3.5 ("Closing a Connection") covers
the teardown.

Your hand-built version follows the same state transitions. The production version adds:
- Timers for retransmission (RTO exponential backoff).
- Window scaling and SACK (selective acknowledgment).
- Nagle's algorithm for small-packet coalescing.

## Read the Source

- Linux `net/ipv4/tcp_input.c` — `tcp_rcv_state_process()` handles incoming segments in each state.
- Linux `net/ipv4/udp.c` — compare the simplicity of UDP vs TCP.
- RFC 793, Sections 3.2–3.5 — the authoritative state machine specification.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A transport layer library** — UDP echo server + TCP state machine you can embed in later lessons.

## Exercises

1. **Easy** — Write a UDP client that sends "hello" to a server and prints the echo response. Use the
   C code from `code/main.c` as reference but don't copy it.
2. **Medium** — Extend the Rust TCP state machine to track sequence numbers during the handshake.
   After SYN_SENT, the client's `snd_una` should advance. After ESTABLISHED, simulate sending
   data segments and advancing both sides' sequence counters.
3. **Hard** — Implement a passive close (the side that receives the first FIN). Drive the state
   machine through CLOSE_WAIT → LAST_ACK → CLOSED. Handle the edge case where both sides
   send FIN simultaneously (CLOSED → FIN_WAIT_1 → CLOSING → TIME_WAIT).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| UDP | "Unreliable protocol" | Connectionless datagram protocol — no ordering, no retransmit, no flow control |
| TCP | "Reliable protocol" | Connection-oriented byte-stream protocol with sequence numbers, ACKs, and state machine |
| Three-way handshake | "TCP connect" | SYN → SYN+ACK → ACK exchange that synchronizes initial sequence numbers |
| Four-way teardown | "TCP disconnect" | FIN → ACK → FIN → ACK exchange; each direction closes independently |
| TIME_WAIT | "Socket stuck after close" | State lasting 2×MSL after active close to drain old segments from the network |
| MSL | "Max segment lifetime" | Maximum time an IP packet can exist in the network; typically 30 seconds |
| Sequence number | "Seq num" | Byte offset of the first byte in this segment within the connection's data stream |
| SYN | "Synchronize" | TCP flag requesting connection establishment; carries the initial sequence number |
| FIN | "Finish" | TCP flag indicating the sender has no more data to send |
| RST | "Reset" | TCP flag aborting a connection immediately; used for errors or refused connections |

## Further Reading

- RFC 793 — Transmission Control Protocol (the original TCP spec)
- RFC 9293 — TCP specification update (2022, obsoletes RFC 793)
- Stevens, *TCP/IP Illustrated, Vol. 1*, Chapters 17–18 — TCP connection establishment and termination
- "High Performance Browser Networking" by Ilya Grigorik, Chapter 12 — TCP building blocks
