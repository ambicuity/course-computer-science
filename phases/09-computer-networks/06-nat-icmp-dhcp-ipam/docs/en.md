# NAT, ICMP, DHCP, IPAM

> The plumbing that makes private networks talk to the Internet, devices get addresses automatically, and engineers can actually debug things.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 09 lessons 01–05
**Time:** ~60 minutes

## Learning Objectives

- Explain how NAT maps private addresses to public ones and why PAT enables many-to-one translation.
- Describe the ICMP protocol and how ping and traceroute work.
- Trace the DHCP DORA (Discover, Offer, Request, Ack) lease process step by step.
- Implement simulators for NAT, DHCP, and ICMP.

## The Problem

There are roughly 4.3 billion IPv4 addresses. There are roughly 15 billion devices connected to the Internet. The gap is bridged by **NAT** — your home router presents one public IP to the Internet while your phone, laptop, and smart TV all use private IPs (`192.168.x.x`, `10.x.x.x`).

But how do devices get those private addresses in the first place? **DHCP** assigns them automatically. And when something goes wrong, you need **ICMP** (ping, traceroute) to diagnose it. **IPAM** ties it all together by tracking who has what address.

## The Concept

### NAT — Network Address Translation

NAT sits at the boundary between a private network and the public Internet. It rewrites packet headers:

```
Internal host 192.168.1.5:4000 → NAT → 203.0.113.1:50001 → Internet
Internet reply → NAT → 192.168.1.5:4000
```

**PAT (Port Address Translation)** — also called NAT overload — maps many internal (ip, port) pairs to a single public IP using different port numbers:

| Internal IP:Port | External IP:Port |
|------------------|-----------------|
| 192.168.1.5:4000 | 203.0.113.1:50001 |
| 192.168.1.6:4000 | 203.0.113.1:50002 |
| 192.168.1.5:5000 | 203.0.113.1:50003 |

**NAT types** (relevant for peer-to-peer and gaming):
- **Full cone**: any external host can send to the mapped port once a mapping exists.
- **Restricted cone**: only the external IP that was contacted can send back.
- **Port-restricted cone**: only the specific external IP:port pair can send back.
- **Symmetric**: each destination gets a different mapping — hardest to traverse.

### ICMP — Internet Control Message Protocol

ICMP rides on top of IP (protocol number 1). Key message types:

- **Echo Request / Reply (type 8/0)**: ping. "Are you there?" → "Yes, here."
- **Time Exceeded (type 11)**: traceroute. Each router decrements TTL. When TTL hits 0, the router sends ICMP Time Exceeded back. By sending packets with TTL=1, 2, 3... you discover each hop.
- **Destination Unreachable (type 3)**: tells the sender that delivery failed (code 0 = network, code 1 = host, code 3 = port).

### DHCP — Dynamic Host Configuration Protocol

DHCP uses UDP ports 67 (server) / 68 (client). The **DORA** process:

1. **Discover**: Client broadcasts "I need an IP address" (src=0.0.0.0, dst=255.255.255.255).
2. **Offer**: Server responds with an offered IP, subnet mask, gateway, DNS.
3. **Request**: Client accepts the offer by broadcasting "I'll take this IP."
4. **Ack**: Server confirms. Client configures its interface.

**Lease management**: Each assignment has a lease duration. The client must renew (at 50% of lease time) or the address returns to the pool.

### IPAM — IP Address Management

IPAM is the discipline of tracking IP address allocations — which ranges are assigned to which subnets, which DHCP pools exist, which static assignments are in use. In practice, this means databases, spreadsheets, or tools like NetBox and phpIPAM.

## Build It

### Step 1: NAT Simulator

```python
from dataclasses import dataclass, field
from typing import Dict, Tuple, Optional


@dataclass
class NAT:
    external_ip: str
    table: Dict[Tuple[str, int], Tuple[str, int]] = field(default_factory=dict)
    reverse: Dict[Tuple[str, int], Tuple[str, int]] = field(default_factory=dict)
    _next_port: int = 50000

    def translate(self, src_ip: str, src_port: int) -> Tuple[str, int]:
        key = (src_ip, src_port)
        if key not in self.table:
            ext_port = self._next_port
            self._next_port += 1
            self.table[key] = (self.external_ip, ext_port)
            self.reverse[(self.external_ip, ext_port)] = key
        return self.table[key]

    def reverse_translate(self, ext_ip: str, ext_port: int) -> Optional[Tuple[str, int]]:
        return self.reverse.get((ext_ip, ext_port))
```

### Step 2: DHCP Simulator

```python
import random


@dataclass
class DHCP:
    pool_start: str
    pool_end: str
    subnet_mask: str = "255.255.255.0"
    gateway: str = "192.168.1.1"
    dns: str = "8.8.8.8"
    lease_time: int = 3600
    leases: Dict[str, Tuple[str, int]] = field(default_factory=dict)  # mac -> (ip, expiry)
    _available: list = field(default_factory=list)

    def __post_init__(self):
        start = self._ip_to_int(self.pool_start)
        end = self._ip_to_int(self.pool_end)
        self._available = [self._int_to_ip(i) for i in range(start, end + 1)]

    def discover_offer(self, mac: str) -> Optional[str]:
        if mac in self.leases:
            ip = self.leases[mac][0]
        elif self._available:
            ip = self._available.pop(0)
        else:
            return None
        return ip

    def request_ack(self, mac: str, ip: str) -> bool:
        self.leases[mac] = (ip, self.lease_time)
        return True

    @staticmethod
    def _ip_to_int(ip: str) -> int:
        parts = ip.split(".")
        return (int(parts[0]) << 24) | (int(parts[1]) << 16) | (int(parts[2]) << 8) | int(parts[3])

    @staticmethod
    def _int_to_int(n: int) -> str:
        return f"{(n >> 24) & 0xFF}.{(n >> 16) & 0xFF}.{(n >> 8) & 0xFF}.{n & 0xFF}"
```

### Step 3: ICMP Simulator

```python
@dataclass
class ICMPPingResult:
    src: str
    dst: str
    ttl: int
    response: str
    hops: int = 0


def ping(src: str, dst: str, ttl: int = 64) -> ICMPPingResult:
    return ICMPPingResult(src=src, dst=dst, ttl=ttl, response="echo_reply")


def traceroute(src: str, dst: str, max_hops: int = 30) -> list:
    hops = []
    for ttl in range(1, max_hops + 1):
        hop_router = f"hop-{ttl}"
        hops.append(hop_router)
        if hop_router == dst:
            break
    return hops
```

## Use It

In production:
- **NAT**: Linux `iptables` / `nftables` with `MASQUERADE` or `SNAT` rules. The conntrack module (`nf_conntrack`) maintains the NAT table in kernel memory. See `net/netfilter/nf_nat_core.c`.
- **DHCP**: `dnsmasq` or ISC DHCP server (`server/dhcp.c`). Handles lease files, static reservations, and option negotiation.
- **ICMP**: Linux kernel `net/ipv4/icmp.c` — handles echo request/reply and generates Time Exceeded messages when TTL expires.

Home routers combine all three: the WAN interface has a public IP (via DHCP from the ISP), NAT translates private LAN traffic, and the built-in DHCP server assigns addresses to LAN devices.

## Read the Source

- `linux/net/netfilter/nf_nat_core.c` — Kernel NAT implementation (conntrack-based).
- `linux/net/ipv4/icmp.c` — ICMP message handling.
- `isc-dhcp/server/dhcp.c` — ISC DHCP server: look at `discover()` and `ack()` functions.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A network services toolkit** (NAT + DHCP + ICMP simulators) for modeling network behavior.

## Exercises

1. **Easy** — Simulate a DHCP lease: client sends Discover, server Offers an IP, client Requests it, server Acks. Print the table at each step.
2. **Medium** — Simulate NAT with 3 internal hosts all connecting to the same external server on port 80. Verify each host gets a unique external port. Then simulate a reply and verify it routes back correctly.
3. **Hard** — Implement a stateful NAT that tracks connection state (TCP SYN seen? Established?). Only allow inbound traffic that matches an existing connection. This approximates how real NAT firewalls work.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| NAT | "Address translation" | Rewrite packet source/destination addresses at network boundaries |
| PAT | "NAT overload" | Map many (internal_ip:port) pairs to one public IP using unique port numbers |
| DORA | "The DHCP handshake" | Discover → Offer → Request → Ack — the 4-step DHCP lease process |
| ICMP | "Ping protocol" | Control/error message protocol (echo, time-exceeded, unreachable) riding on IP |
| Lease | "Temporary IP assignment" | A time-limited DHCP allocation that must be renewed or the IP returns to the pool |
| IPAM | "IP tracking" | The discipline and tooling for managing IP address space allocations |
| TTL | "Time to live" | Hop-count field decremented by each router; triggers ICMP Time Exceeded at zero |

## Further Reading

- RFC 3022 — Traditional IP Network Address Translator (Traditional NAT)
- RFC 2131 — Dynamic Host Configuration Protocol
- RFC 792 — Internet Control Message Protocol
- RFC 4787 — NAT Behavioral Requirements for Unicast UDP
