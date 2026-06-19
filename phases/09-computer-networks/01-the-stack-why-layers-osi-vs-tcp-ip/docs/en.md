# The Stack — Why Layers (OSI vs TCP/IP)

## The Problem: Networks Are Enormously Complex

Imagine you want to send an email. Your message must travel through dozens of routers, cross
different physical media (copper, fiber, wireless), survive noise and congestion, and arrive
intact at a machine you never designed. Trying to build one monolithic program that handles
every aspect of this journey is a recipe for disaster.

**Layers** are the answer: split the problem into small, focused pieces. Each piece solves one
well-defined sub-problem and exposes a clean interface to the layer above.

---

## Why Layers?

Three engineering principles make layered networking indispensable:

### 1. Separation of Concerns

Each layer handles exactly one job. The physical layer worries about voltages and radio
frequencies; the transport layer worries about reliable delivery. Neither needs to know
the other's implementation details.

### 2. Abstraction

Lower layers hide their complexity. An application programmer writes `send(data)` and the
transport layer guarantees delivery, the network layer finds a route, and the link layer
puts bits on a wire — all invisible to the programmer.

### 3. Modularity

You can swap any layer without breaking the rest. Upgrade from Ethernet to Wi-Fi at the
link layer, and everything above keeps working. Replace IPv4 with IPv6 at the network
layer, and applications still use the same socket API.

---

## The OSI Model

The **Open Systems Interconnection** model defines **7 layers**. It is a conceptual
reference — no real protocol stack implements all seven as separate software — but it
provides a shared vocabulary that every network engineer uses.

```
Layer 7  Application       Data          (HTTP, FTP, SMTP, DNS)
Layer 6  Presentation      Data          (TLS, JPEG, ASCII conversion)
Layer 5  Session           Data          (RPC, NetBIOS)
Layer 4  Transport         Segment/Datagram (TCP, UDP)
Layer 3  Network           Packet        (IP, ICMP, OSPF)
Layer 2  Data Link         Frame         (Ethernet, Wi-Fi, PPP)
Layer 1  Physical          Bits          (cables, radio, voltages)
```

### Layer-by-Layer Responsibilities

| Layer | Name | Responsibility | PDU | Example Protocols |
|-------|------|---------------|-----|-------------------|
| 7 | Application | User-facing services | Data | HTTP, SMTP, DNS, SSH |
| 6 | Presentation | Data format, encryption, compression | Data | TLS, JPEG, ASCII |
| 5 | Session | Dialog control, synchronization | Data | RPC, SIP |
| 4 | Transport | End-to-end reliability, multiplexing | Segment / Datagram | TCP, UDP |
| 3 | Network | Logical addressing, routing | Packet | IP, ICMP, IGMP |
| 2 | Data Link | Framing, MAC addressing, error detection | Frame | Ethernet, Wi-Fi |
| 1 | Physical | Bits on the medium | Bits | Cables, hubs, radio |

**PDU** = Protocol Data Unit. Each layer names its chunk of data differently.

### Mnemonic

Top-down: **A**ll **P**eople **S**eem **T**o **N**eed **D**ata **P**rocessing.

---

## The TCP/IP Model

The model that the real Internet is built on has **4 layers**:

```
Layer 4  Application    (HTTP, SMTP, DNS, SSH, TLS sits here too)
Layer 3  Transport      (TCP, UDP)
Layer 2  Internet       (IP, ICMP, ARP — yes, ARP is debated)
Layer 1  Network Access (Ethernet, Wi-Fi, PPP, DSL)
```

### OSI → TCP/IP Mapping

```
  OSI                          TCP/IP
  ─────────────────────────    ──────────────────
  Application                  Application
  Presentation       →         (folded into
  Session                      Application)
  ─────────────────────────    ──────────────────
  Transport                    Transport
  ─────────────────────────    ──────────────────
  Network                      Internet
  ─────────────────────────    ──────────────────
  Data Link                    Network Access
  Physical
```

TCP/IP merges OSI layers 5–7 into a single Application layer. In practice, no one implements
Session and Presentation as separate layers — TLS, serialization, and session management live
inside application libraries or middleware.

---

## Encapsulation: How a Packet Flows

When application data travels down the stack, each layer wraps the data in its own
**header** (and sometimes a **trailer**). This wrapping is called **encapsulation**.

```
Layer 7:  ┌──────────────────────┐
          │  "Hello, server!"    │
          └──────────────────────┘

Layer 4:  ┌─────────┬──────────────────────┐
          │ TCP hdr │  "Hello, server!"    │
          └─────────┴──────────────────────┘
                        = Segment

Layer 3:  ┌─────────┬─────────┬──────────────────────┐
          │ IP hdr  │ TCP hdr │  "Hello, server!"    │
          └─────────┴─────────┴──────────────────────┘
                  = Packet

Layer 2:  ┌──────┬──────────────────────────────┬─────┐
          │ Eth  │       Packet (IP+TCP+Data)   │ FCS │
          │ hdr  │                              │     │
          └──────┴──────────────────────────────┴─────┘
                      = Frame

Layer 1:  10110010 11010101 01100011 ...
          = Bits on the wire
```

At the receiver, each layer strips its header, processes it, and passes the payload upward.
This is **decapsulation**.

### Real-World Walk

1. Browser (Application) creates an HTTP GET request.
2. TCP (Transport) adds a source port and destination port (443) with sequence numbers.
3. IP (Network) adds source IP `192.168.1.50` and destination IP `93.184.216.34`.
4. Ethernet (Link) adds source MAC `aa:bb:cc:dd:ee:01` and destination MAC of the
   default gateway.
5. Physical layer converts to electrical signals.
6. Each router on the path decapsulates to Layer 3, makes a routing decision,
   re-encapsulates, and forwards.

---

## Layer Responsibilities — A Deeper Look

### Physical (Layer 1)
Raw bit transmission. Concerns: voltage levels, cable types (Cat6, fiber), connectors
(RJ-45, LC), signaling (Manchester encoding), data rates. Devices: hubs, repeaters.

### Data Link (Layer 2)
Framing, MAC addressing, error detection (CRC/FCS). Switches operate here, learning
which MAC addresses are on which ports. Protocols: Ethernet (IEEE 802.3), Wi-Fi
(IEEE 802.11), ARP (bridges L2–L3).

### Network (Layer 3)
Logical addressing and routing. IP addresses are hierarchical (network + host). Routers
forward packets based on routing tables. Protocols: IPv4, IPv6, ICMP, OSPF, BGP.

### Transport (Layer 4)
End-to-end communication between processes. Multiplexing via port numbers. TCP provides
reliable, ordered delivery; UDP provides best-effort datagrams.

### Application (Layers 5–7 / TCP/IP Layer 4)
Everything else: protocols (HTTP, SMTP, DNS), data formatting (JSON, TLS), session
management, and the actual user-facing software.

---

## Use It: Debugging with Layers

When something breaks, ask: **which layer is the problem?**

```
Cannot ping?            → Layer 3 (Network — IP routing, firewall)
Ping works, HTTP fails? → Layer 4+ (Transport/Application — port, TLS, server)
No link light?          → Layer 1 (Physical — cable unplugged, NIC dead)
ARP table empty?        → Layer 2 (Data Link — switch issue, VLAN misconfig)
```

The `ping` command tests Layers 3–4. `curl` tests Layers 4–7. A link light on your
NIC is Layer 1. Start at the bottom and work up.

### Common Diagnostic Commands

| Command | Layer Tested | What It Tells You |
|---------|-------------|-------------------|
| `ip link` / `ifconfig` | 1–2 | Interface status, MAC address |
| `arp -a` | 2–3 | IP-to-MAC mappings |
| `ping <ip>` | 3 | Reachability |
| `traceroute <ip>` | 3 | Path through routers |
| `ss -tlnp` / `netstat` | 4 | Listening TCP ports |
| `curl <url>` | 4–7 | Full application stack |

---

## Ship It: Networking Concept Map

Draw or mentally walk through this concept map:

```
Application Data
  └─ Transport: TCP/UDP + port numbers
       └─ Network: IP + routing
            └─ Link: Ethernet + MAC + FCS
                 └─ Physical: bits
```

Every network conversation you ever debug will start at one of these layers.

---

## Exercises

### Level 1 — Identify the Layer

Match each scenario to the OSI layer most likely responsible:

1. A cable is unplugged from a server.
2. An HTTP 500 error is returned by a web server.
3. A router drops a packet because its TTL reached 0.
4. A switch learns a new MAC address on port 7.

### Level 2 — Encapsulation Trace

A DNS query is sent over UDP from host `10.0.0.5` to DNS server `10.0.0.1`.
- Source port: random (e.g., 51234), destination port: 53.
- Source IP: 10.0.0.5, destination IP: 10.0.0.1.
- Source MAC: `de:ad:be:ef:00:05`, destination MAC: `de:ad:be:ef:00:01`.

Write out (in ASCII) the encapsulation at each layer, from Application down to Link.
Label each PDU.

### Level 3 — Design a Layer

You are building a file-transfer application. Decide which OSI layers your application
library must directly handle, and which are provided by the OS. Justify your choices
for reliability, ordering, and error detection.

---

## Summary

- Networks use **layers** for separation of concerns, abstraction, and modularity.
- The **OSI model** has 7 layers; the **TCP/IP model** has 4.
- **Encapsulation**: each layer wraps the previous layer's data in a header.
- Debugging network issues means identifying which **layer** is broken.
- Real-world protocols map cleanly to layers: Ethernet → IP → TCP → HTTP.
