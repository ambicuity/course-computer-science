# Network Layer — IPv4, IPv6, Subnetting, CIDR

## Overview

The **Network layer** (Layer 3) provides logical addressing and routing,
enabling packets to traverse multiple networks. This lesson covers **IPv4**,
**IPv6**, **subnetting**, and **CIDR**.

---

## IPv4 Header

Every IPv4 packet starts with a **20–60 byte** header:

```
  0                   1                   2                   3
  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |Version|  IHL  |    DSCP/ECN   |         Total Length          |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |         Identification        |Flags|    Fragment Offset      |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |  Time to Live |    Protocol   |        Header Checksum        |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |                       Source IP Address                        |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |                    Destination IP Address                      |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

| Field | Bits | Description |
|-------|------|-------------|
| Version | 4 | Always `4` |
| IHL | 4 | Header length in 32-bit words (min 5 = 20 bytes) |
| Total Length | 16 | Entire packet size (max 65,535) |
| TTL | 8 | Hop limit — discard at 0 |
| Protocol | 8 | TCP=6, UDP=17, ICMP=1 |
| Header Checksum | 16 | Error detection on header (recomputed each hop) |
| Source/Dest IP | 32 each | Sender and receiver addresses |

### Private Address Ranges (RFC 1918)

| Range | CIDR | Use |
|-------|------|-----|
| 10.0.0.0 – 10.255.255.255 | 10.0.0.0/8 | Large networks |
| 172.16.0.0 – 172.31.255.255 | 172.16.0.0/12 | Medium networks |
| 192.168.0.0 – 192.168.255.255 | 192.168.0.0/16 | Home/small office |

---

## IPv6

IPv4's 32-bit addresses yield ~4.3 billion addresses — not enough.
**IPv6** uses **128-bit addresses** (3.4 × 10³⁸).

### Address Format

```
2001:0db8:85a3:0000:0000:8a2e:0370:7334
```

- 8 groups of 4 hex digits, colons between groups.
- Leading zeros omitted: `0db8` → `db8`.
- One all-zero sequence replaced by `::` (once):
  `2001:db8:85a3::8a2e:370:7334`.

### IPv6 Header

Fixed 40-byte header: Version, Traffic Class, Flow Label, Payload Length,
Next Header, Hop Limit, Source/Destination (128 bits each).
Key differences: no checksum (delegated to transport), no fragmentation at
routers (source uses Path MTU Discovery), extension headers replace options.

---

## Subnetting

**Subnetting** divides a large network into smaller sub-networks.

### Subnet Mask

A 32-bit number separating **network** from **host** bits
(e.g., `255.255.255.0` = first 24 bits are network).

### Key Addresses (192.168.1.0/24)

| Type | Address | Description |
|------|---------|-------------|
| Network | 192.168.1.0 | All host bits = 0 |
| First usable | 192.168.1.1 | Network + 1 |
| Last usable | 192.168.1.254 | Broadcast - 1 |
| Broadcast | 192.168.1.255 | All host bits = 1 |

Usable hosts = 2^(host_bits) - 2.

### Subnetting Example

Split `10.0.0.0/24` into 4 subnets → borrow 2 bits → `/26`:

| Subnet | Network | Range | Broadcast |
|--------|---------|-------|-----------|
| 1 | 10.0.0.0/26 | .1 – .62 | 10.0.0.63 |
| 2 | 10.0.0.64/26 | .65 – .126 | 10.0.0.127 |
| 3 | 10.0.0.128/26 | .129 – .190 | 10.0.0.191 |
| 4 | 10.0.0.192/26 | .193 – .254 | 10.0.0.255 |

---

## CIDR — Classless Inter-Domain Routing

**CIDR** (RFC 1519) replaced classful addressing with variable-length prefixes:

```
a.b.c.d/p    (p = number of network bits)
```

### Common Prefixes

| CIDR | Mask | Usable Hosts | Notes |
|------|------|-------------|-------|
| /8 | 255.0.0.0 | 16,777,214 | Large network |
| /24 | 255.255.255.0 | 254 | Most common LAN |
| /30 | 255.255.255.252 | 2 | Point-to-point |
| /32 | 255.255.255.255 | 1 | Single host |

### Supernetting

Combine small networks into one larger route:

```
192.168.0.0/24 + .1.0/24 + .2.0/24 + .3.0/24 = 192.168.0.0/22
```

---

## Build It

See `code/main.py` — subnet calculator (CIDR parsing, split/aggregate, host range).
See `code/main.c` — IPv4 header parser (checksum verification, protocol ID).

---

## Use It

```bash
ip addr show              # view IPs and subnets
ip route show             # routing table
```

---

## Ship It

`code/main.py` provides a `Subnet` class ready to import into network planning tools.

---

## Exercises

### Level 1 — Subnet Identification

Given `172.16.50.100/20`: find the subnet mask, network address, broadcast
address, and number of usable hosts.

### Level 2 — Subnet Splitting

Split `10.1.1.0/24` into 8 equal subnets. List CIDR, network, broadcast,
and host range for each.

### Level 3 — IPv6 Planning

Organization gets `2001:db8:acad::/48`. They need 4 office subnets, each
supporting 1,000 internal subnets. Determine prefix lengths and give the
first three subnet addresses for the European office.

---

## Summary

- **IPv4**: 32-bit addresses, 20-byte header, checksum, fragmentation at routers.
- **IPv6**: 128-bit addresses, fixed 40-byte header, no checksum, extension headers.
- **Subnetting**: divide using subnet mask; compute network, broadcast, range.
- **CIDR**: prefix notation (`/p`) replaces classful addressing; enables supernetting.
