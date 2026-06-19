# Firewalls, NAT Traversal, STUN/TURN/ICE

> How two peers behind NATs find each other — and why video calls work at all.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 09 lessons 01–15
**Time:** ~60 minutes

## Learning Objectives

- Understand firewalls: packet filtering, stateful inspection, and application-layer firewalls.
- Explain how STUN discovers a peer's public address, and why TURN exists as a fallback.
- Implement the ICE candidate gathering and connectivity check algorithm.
- Build a NAT traversal toolkit that simulates two peers connecting through NATs.

## The Problem

You and a friend both sit behind home routers with NAT. Your phone has `192.168.1.5`, theirs has `192.168.1.3` — neither is reachable from the Internet. Yet Zoom, Teams, and WebRTC somehow establish direct peer-to-peer audio/video between you. How?

The answer is **NAT traversal**: a set of techniques (STUN, TURN, ICE) that punch through NAT devices to establish direct connections. Before you can traverse NATs, you must understand **firewalls** — the devices that decide what traffic is allowed in the first place.

## The Concept

### Firewalls

A firewall controls traffic flow based on rules. There are three generations:

**1. Packet filter (stateless)** — Examines each packet independently against a 5-tuple rule:

```
5-tuple = (src_ip, dst_ip, src_port, dst_port, protocol)
```

Rules are simple: ALLOW or DENY.

```
Rule 1: ALLOW tcp *:any → 10.0.0.1:80      (inbound web traffic)
Rule 2: ALLOW tcp 10.0.0.1:any → *:*        (return traffic)
Rule 3: DENY  *   *:*      → *:*            (default deny)
```

**2. Stateful inspection** — Tracks connection state. When an outbound SYN is seen, the firewall remembers the connection and automatically allows the reply. No explicit return rule needed.

```
ConnTrack table:
  (10.0.0.5:4000 → 93.184.216.34:80, tcp, ESTABLISHED)
  (10.0.0.5:5000 → 8.8.8.8:53, udp, LAST_SEEN 2s ago)
```

**3. Application-layer firewall (L7)** — Inspects protocol content. Can block specific HTTP paths, detect SQL injection in requests, or filter DNS queries by domain. Slower but more precise.

### NAT Types (Recap)

NAT behavior matters for traversal. The four types from most to least permissive:

| NAT Type | Who can send back? | Traversal difficulty |
|----------|-------------------|---------------------|
| Full cone | Anyone to mapped port | Easy |
| Restricted cone | Only IPs you contacted | Medium |
| Port-restricted cone | Only specific IP:port you contacted | Hard |
| Symmetric | Different mapping per destination | Hardest |

### STUN — Session Traversal Utilities for NAT

STUN (RFC 5389) answers one question: **"What is my public IP:port?"**

```
Client (192.168.1.5:4000)
  |
  | STUN Binding Request (UDP)
  v
STUN Server (public IP)
  |
  | Sees packet from 203.0.113.1:50001
  | Replies: "Your mapped address is 203.0.113.1:50001"
  v
Client learns its server-reflexive address
```

The STUN message format uses Type-Length-Value (TLV) attributes:

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|0 0|   STUN Message Type     |         Message Length          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        Magic Cookie (0x2112A442)              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        Transaction ID (96 bits)               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

Key attribute: `XOR-MAPPED-ADDRESS` (type 0x0020) — the client's public IP:port, XORed with the magic cookie to prevent NAT middleboxes from rewriting it.

### TURN — Traversal Using Relays around NAT

When STUN fails (symmetric NAT on both sides), TURN (RFC 5766) provides a **relay server**:

```
Peer A → TURN Server → Peer B
Peer B → TURN Server → Peer A
```

The TURN server allocates a relay address. All data flows through it. This works always but costs bandwidth — the relay server must relay every packet.

```
1. Allocate: Client asks TURN server for a relay address
2. CreatePermission: Allow specific peers to send to the relay
3. Send/Receive: Data flows client ↔ TURN ↔ peer
```

### ICE — Interactive Connectivity Establishment

ICE (RFC 8445) is the **orchestration layer** that combines STUN and TURN. It tries the cheapest path first:

```
Priority order:
  1. Host candidate      — direct LAN connection (cheapest)
  2. Server reflexive     — via STUN (public IP discovered)
  3. Peer reflexive       — discovered during connectivity checks
  4. Relay candidate      — via TURN (most expensive, always works)
```

**Candidate gathering**: Each peer collects all possible addresses:
- Host: its local interface IP (e.g., `192.168.1.5:4000`)
- Server reflexive: STUN discovers public mapping (e.g., `203.0.113.1:50001`)
- Relay: TURN allocates relay address (e.g., `198.51.100.1:60000`)

**Connectivity checks**: Each peer pairs its candidates with the remote peer's candidates. For each pair, send a STUN binding request. If a response comes back, that pair works.

**Nomination**: The controlling agent selects the highest-priority working pair. That becomes the data path.

WebRTC uses ICE for all peer connections — every video call in your browser goes through this process.

## Build It

### Step 1: Firewall Simulator

```python
from dataclasses import dataclass, field
from typing import Dict, Set, Tuple, Optional, List
from enum import Enum


class Action(Enum):
    ALLOW = "allow"
    DENY = "deny"


class NATType(Enum):
    FULL_CONE = "full_cone"
    RESTRICTED_CONE = "restricted_cone"
    PORT_RESTRICTED = "port_restricted"
    SYMMETRIC = "symmetric"


@dataclass
class FirewallRule:
    action: Action
    protocol: str  # "tcp", "udp", or "*"
    src_ip: str
    src_port: int  # -1 means any
    dst_ip: str
    dst_port: int  # -1 means any


@dataclass
class Firewall:
    rules: List[FirewallRule] = field(default_factory=list)
    state_table: Dict[Tuple[str, int, str, int, str], str] = field(default_factory=dict)
    _stateful: bool = True

    def add_rule(self, rule: FirewallRule) -> None:
        self.rules.append(rule)

    def check(self, src_ip: str, src_port: int, dst_ip: str,
              dst_port: int, protocol: str) -> Action:
        key = (src_ip, src_port, dst_ip, dst_port, protocol)

        if self._stateful and key in self.state_table:
            return Action.ALLOW

        for rule in self.rules:
            if self._match(rule, src_ip, src_port, dst_ip, dst_port, protocol):
                if self._stateful and rule.action == Action.ALLOW:
                    # Track the reverse path for stateful return traffic
                    reverse = (dst_ip, dst_port, src_ip, src_port, protocol)
                    self.state_table[reverse] = "established"
                return rule.action

        return Action.DENY

    @staticmethod
    def _match(rule: FirewallRule, src_ip: str, src_port: int,
               dst_ip: str, dst_port: int, protocol: str) -> bool:
        if rule.protocol != "*" and rule.protocol != protocol:
            return False
        if rule.src_ip != "*" and rule.src_ip != src_ip:
            return False
        if rule.src_port != -1 and rule.src_port != src_port:
            return False
        if rule.dst_ip != "*" and rule.dst_ip != dst_ip:
            return False
        if rule.dst_port != -1 and rule.dst_port != dst_port:
            return False
        return True
```

### Step 2: STUN Client

```python
import struct
import os


STUN_MAGIC_COOKIE = 0x2112A442
STUN_BINDING_REQUEST = 0x0001
STUN_BINDING_RESPONSE = 0x0101
ATTR_XOR_MAPPED_ADDRESS = 0x0020


@dataclass
class STUNMessage:
    msg_type: int
    transaction_id: bytes
    attributes: Dict[int, bytes] = field(default_factory=dict)

    def encode(self) -> bytes:
        attrs_data = b''
        for attr_type, attr_value in self.attributes.items():
            attrs_data += struct.pack('!HH', attr_type, len(attr_value))
            attrs_data += attr_value
            # Pad to 4-byte boundary
            pad = (4 - len(attr_value) % 4) % 4
            attrs_data += b'\x00' * pad

        header = struct.pack('!HH', self.msg_type, len(attrs_data))
        header += struct.pack('!I', STUN_MAGIC_COOKIE)
        header += self.transaction_id
        return header + attrs_data

    @classmethod
    def decode(cls, data: bytes) -> 'STUNMessage':
        msg_type, length = struct.unpack('!HH', data[:4])
        magic = struct.unpack('!I', data[4:8])[0]
        txn_id = data[8:20]

        attributes = {}
        offset = 20
        while offset < 20 + length:
            attr_type, attr_len = struct.unpack('!HH', data[offset:offset+4])
            offset += 4
            attr_value = data[offset:offset+attr_len]
            attributes[attr_type] = attr_value
            offset += attr_len + (4 - attr_len % 4) % 4

        return cls(msg_type=msg_type, transaction_id=txn_id, attributes=attributes)


@dataclass
class STUNClient:
    """Simulated STUN client that discovers its server-reflexive address."""
    local_ip: str
    local_port: int
    nat_ip: str  # The NAT's public IP
    _nat_port_map: Dict[int, int] = field(default_factory=dict)
    _next_port: int = 50000

    def send_binding_request(self) -> Tuple[str, int]:
        """Send a STUN binding request and return the mapped address."""
        # Simulate NAT: allocate a public port
        if self.local_port not in self._nat_port_map:
            self._nat_port_map[self.local_port] = self._next_port
            self._next_port += 1
        public_port = self._nat_port_map[self.local_port]

        txn_id = os.urandom(12)
        request = STUNMessage(msg_type=STUN_BINDING_REQUEST, transaction_id=txn_id)

        # Server sees the packet from the NAT's public address
        # Encodes it XOR'd with magic cookie
        mapped_ip_bytes = bytes(map(int, self.nat_ip.split('.')))
        xor_port = public_port ^ (STUN_MAGIC_COOKIE >> 16)
        xor_ip = bytes(a ^ b for a, b in zip(
            mapped_ip_bytes,
            struct.pack('!I', STUN_MAGIC_COOKIE)
        ))

        xor_attr = struct.pack('!H', 0x0001)  # IPv4
        xor_attr += struct.pack('!H', xor_port)
        xor_attr += xor_ip

        response = STUNMessage(
            msg_type=STUN_BINDING_RESPONSE,
            transaction_id=txn_id,
            attributes={ATTR_XOR_MAPPED_ADDRESS: xor_attr}
        )

        # Decode the XOR-MAPPED-ADDRESS from the response
        attr_data = response.attributes[ATTR_XOR_MAPPED_ADDRESS]
        family = struct.unpack('!H', attr_data[:2])[0]
        xport = struct.unpack('!H', attr_data[2:4])[0]
        xip = attr_data[4:8]

        mapped_port = xport ^ (STUN_MAGIC_COOKIE >> 16)
        mapped_ip = '.'.join(str(a ^ b) for a, b in zip(
            xip, struct.pack('!I', STUN_MAGIC_COOKIE)
        ))

        return (mapped_ip, mapped_port)
```

### Step 3: TURN Client

```python
@dataclass
class TURNClient:
    """Simulated TURN client that allocates a relay address."""
    server_ip: str
    relay_ip: str
    _allocations: Dict[str, Tuple[str, int]] = field(default_factory=dict)
    _next_relay_port: int = 60000

    def allocate(self, client_id: str) -> Tuple[str, int]:
        """Allocate a relay address for the client."""
        port = self._next_relay_port
        self._next_relay_port += 1
        self._allocations[client_id] = (self.relay_ip, port)
        return (self.relay_ip, port)

    def relay_data(self, sender_id: str, data: bytes,
                   dst: Tuple[str, int]) -> Optional[bytes]:
        """Relay data from sender to destination through the TURN server."""
        if sender_id not in self._allocations:
            return None
        return data  # In reality, TURN wraps this in a ChannelData message
```

### Step 4: ICE Agent

```python
@dataclass
class Candidate:
    foundation: str
    component: int
    transport: str
    priority: int
    address: str
    port: int
    type: str  # "host", "srflx", "prflx", "relay"

    def __repr__(self):
        return f"{self.type}:{self.address}:{self.port}"


@dataclass
class ICEAgent:
    """Gathers candidates, performs connectivity checks, selects best path."""
    name: str
    stun_client: STUNClient
    turn_client: TURNClient
    candidates: List[Candidate] = field(default_factory=list)
    selected_pair: Optional[Tuple[Candidate, Candidate]] = None
    controlling: bool = False

    def gather_candidates(self) -> List[Candidate]:
        """Gather all candidate types: host, server-reflexive, relay."""
        self.candidates = []

        # Host candidate
        self.candidates.append(Candidate(
            foundation="host1",
            component=1,
            transport="udp",
            priority=100,
            address=self.stun_client.local_ip,
            port=self.stun_client.local_port,
            type="host"
        ))

        # Server-reflexive candidate (via STUN)
        srflx_ip, srflx_port = self.stun_client.send_binding_request()
        self.candidates.append(Candidate(
            foundation="srflx1",
            component=1,
            transport="udp",
            priority=200,
            address=srflx_ip,
            port=srflx_port,
            type="srflx"
        ))

        # Relay candidate (via TURN)
        relay_ip, relay_port = self.turn_client.allocate(self.name)
        self.candidates.append(Candidate(
            foundation="relay1",
            component=1,
            transport="udp",
            priority=50,
            address=relay_ip,
            port=relay_port,
            type="relay"
        ))

        return self.candidates

    def connectivity_check(self, local: Candidate,
                           remote: Candidate) -> bool:
        """Simulate a connectivity check between a local and remote candidate."""
        # Host-to-host on same subnet: works
        if local.type == "host" and remote.type == "host":
            return local.address.split('.')[:3] == remote.address.split('.')[:3]

        # Server-reflexive: works unless both sides are behind symmetric NAT
        if local.type == "srflx" or remote.type == "srflx":
            return True

        # Relay: always works
        if local.type == "relay" or remote.type == "relay":
            return True

        return False

    def run_ice(self, remote_agent: 'ICEAgent') -> Tuple[Candidate, Candidate]:
        """Full ICE: gather, check all pairs, select best."""
        self.gather_candidates()
        remote_agent.gather_candidates()

        # Sort candidates by priority (descending)
        local_sorted = sorted(self.candidates, key=lambda c: c.priority, reverse=True)
        remote_sorted = sorted(remote_agent.candidates, key=lambda c: c.priority, reverse=True)

        best_pair = None
        best_priority = -1

        for local in local_sorted:
            for remote in remote_sorted:
                if self.connectivity_check(local, remote):
                    pair_priority = min(local.priority, remote.priority)
                    if pair_priority > best_priority:
                        best_priority = pair_priority
                        best_pair = (local, remote)

        if best_pair:
            self.selected_pair = best_pair
            remote_agent.selected_pair = (best_pair[1], best_pair[0])

        return best_pair
```

### Step 5: Full Demo

```python
def main() -> None:
    print("=" * 60)
    print("Firewalls, NAT Traversal, STUN/TURN/ICE")
    print("=" * 60)

    # --- Firewall Demo ---
    print("\n--- Firewall Simulation ---\n")
    fw = Firewall()
    fw.add_rule(FirewallRule(
        action=Action.ALLOW, protocol="tcp",
        src_ip="*", src_port=-1,
        dst_ip="10.0.0.1", dst_port=80
    ))
    fw.add_rule(FirewallRule(
        action=Action.ALLOW, protocol="udp",
        src_ip="*", src_port=-1,
        dst_ip="10.0.0.1", dst_port=53
    ))
    fw.add_rule(FirewallRule(
        action=Action.DENY, protocol="*",
        src_ip="*", src_port=-1,
        dst_ip="*", dst_port=-1
    ))

    tests = [
        ("192.168.1.5", 4000, "10.0.0.1", 80, "tcp"),
        ("192.168.1.5", 5000, "10.0.0.1", 53, "udp"),
        ("192.168.1.5", 6000, "10.0.0.1", 443, "tcp"),
    ]
    for src_ip, src_port, dst_ip, dst_port, proto in tests:
        result = fw.check(src_ip, src_port, dst_ip, dst_port, proto)
        print(f"  {proto} {src_ip}:{src_port} → {dst_ip}:{dst_port} = {result.value}")

    # Stateful: return traffic for allowed connections should pass
    print("\n  Stateful check (return traffic for port 80 connection):")
    ret = fw.check("10.0.0.1", 80, "192.168.1.5", 4000, "tcp")
    print(f"  tcp 10.0.0.1:80 → 192.168.1.5:4000 = {ret.value}")

    # --- STUN Demo ---
    print("\n--- STUN Client Simulation ---\n")
    stun = STUNClient(
        local_ip="192.168.1.5", local_port=4000,
        nat_ip="203.0.113.1"
    )
    public_addr = stun.send_binding_request()
    print(f"  Local address:  {stun.local_ip}:{stun.local_port}")
    print(f"  Public address: {public_addr[0]}:{public_addr[1]}")

    # --- ICE Demo ---
    print("\n--- ICE Agent: Two Peers Behind NAT ---\n")

    # Peer A: behind a moderate NAT
    stun_a = STUNClient(local_ip="192.168.1.5", local_port=4000, nat_ip="203.0.113.1")
    turn_a = TURNClient(server_ip="198.51.100.1", relay_ip="198.51.100.1")
    agent_a = ICEAgent(name="peer_a", stun_client=stun_a, turn_client=turn_a, controlling=True)

    # Peer B: behind a different NAT
    stun_b = STUNClient(local_ip="192.168.2.10", local_port=5000, nat_ip="203.0.113.2")
    turn_b = TURNClient(server_ip="198.51.100.1", relay_ip="198.51.100.1")
    agent_b = ICEAgent(name="peer_b", stun_client=stun_b, turn_client=turn_b)

    best = agent_a.run_ice(agent_b)

    print(f"  Peer A candidates:")
    for c in agent_a.candidates:
        print(f"    {c.type:6s}  {c.address}:{c.port}  (priority {c.priority})")

    print(f"\n  Peer B candidates:")
    for c in agent_b.candidates:
        print(f"    {c.type:6s}  {c.address}:{c.port}  (priority {c.priority})")

    if best:
        print(f"\n  Selected pair: {best[0]} ↔ {best[1]}")
        print(f"  Path type: {best[0].type} + {best[1].type}")
    else:
        print("\n  No working pair found — would need TURN relay")

    # --- Simulate symmetric NAT scenario ---
    print("\n--- ICE with Symmetric NAT (TURN fallback) ---\n")

    class SymmetricSTUNClient(STUNClient):
        """Symmetric NAT: each destination gets a different mapped port."""
        _dest_port_map: Dict[Tuple[int, str, int], int] = field(default_factory=dict)

        def send_binding_request(self) -> Tuple[str, int]:
            # Different port per "destination" — simulate by using a random port
            import random
            port = random.randint(50000, 60000)
            return (self.nat_ip, port)

    stun_sym_a = SymmetricSTUNClient(
        local_ip="10.0.0.5", local_port=4000, nat_ip="192.0.2.1"
    )
    stun_sym_b = SymmetricSTUNClient(
        local_ip="10.0.0.8", local_port=5000, nat_ip="192.0.2.2"
    )
    turn_sym = TURNClient(server_ip="198.51.100.1", relay_ip="198.51.100.1")
    agent_sym_a = ICEAgent(
        name="sym_a", stun_client=stun_sym_a,
        turn_client=turn_sym, controlling=True
    )
    agent_sym_b = ICEAgent(
        name="sym_b", stun_client=stun_sym_b, turn_client=turn_sym
    )

    best_sym = agent_sym_a.run_ice(agent_sym_b)
    if best_sym:
        print(f"  Selected pair: {best_sym[0]} ↔ {best_sym[1]}")
        print(f"  Relayed: {best_sym[0].type == 'relay' or best_sym[1].type == 'relay'}")


if __name__ == "__main__":
    main()
```

## Use It

In production:

- **Firewalls**: Linux `iptables`/`nftables` with conntrack (`nf_conntrack`) for stateful inspection. See `net/netfilter/nf_conntrack_core.c`.
- **STUN**: `coturn` or `stunserver`. WebRTC browsers call `RTCPeerConnection.addIceCandidate()` which runs STUN internally. RFC 5389 defines the protocol.
- **TURN**: `coturn` is the standard open-source TURN server. Bandwidth costs make TURN expensive — services like Twilio charge per GB relayed.
- **ICE**: Implemented in `libnice` (C), `pion/ice` (Go), and every WebRTC stack. The algorithm is in RFC 8445, Section 6.

Zoom, Teams, and Discord all use ICE. When a direct path fails (symmetric NAT, corporate firewall), they fall back to TURN. When even TURN is blocked (deep packet inspection), they relay over TCP/TLS on port 443.

## Read the Source

- `libnice/agent/conncheck.c` — ICE connectivity check state machine (the `conn_check_tick` function).
- `coturn/src/server/ns_turn_server.c` — TURN server: look at `handle_turn_allocate()`.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A NAT traversal toolkit** — firewall simulator + STUN client + TURN allocator + ICE agent for modeling peer-to-peer connection establishment.

## Exercises

1. **Easy** — Add a firewall rule that blocks all inbound traffic except on port 22 (SSH). Verify that a packet to port 22 is allowed and port 80 is denied.

2. **Medium** — Extend the ICE agent to simulate the case where both peers are behind restricted-cone NATs. Show that the server-reflexive candidates can connect, but host candidates cannot. Print the connectivity check results for every candidate pair.

3. **Hard** — Implement ICE restart: when the selected pair stops working (network change simulation), re-gather candidates and re-run connectivity checks. Track the generation number (RFC 8445, Section 9) so old candidates are discarded.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Firewall | "Blocks traffic" | Rule-based packet filter (stateless or stateful) controlling which 5-tuple flows are allowed |
| Stateful inspection | "Tracks connections" | Firewall remembers outbound connections and auto-allows return traffic |
| 5-tuple | "Connection tuple" | (src_ip, src_port, dst_ip, dst_port, protocol) — uniquely identifies a flow |
| STUN | "What's my IP?" | Protocol to discover your public IP:port as seen by a STUN server |
| TURN | "Relay fallback" | Server that relays traffic between peers when direct connection fails |
| ICE | "Connection negotiation" | Orchestration: gather candidates → connectivity checks → select best path |
| Candidate | "Possible address" | A (IP, port, type) that a peer can be reached at: host, srflx, prflx, or relay |
| Server reflexive | "Public IP via STUN" | Candidate discovered by STUN — the NAT's public mapping of the host address |

## Further Reading

- RFC 8445 — Interactive Connectivity Establishment (ICE)
- RFC 5389 — Session Traversal Utilities for NAT (STUN)
- RFC 5766 — Traversal Using Relays around NAT (TURN)
- [WebRTC samples](https://webrtc.github.io/samples/) — Browser-based ICE demos showing candidate gathering in real time
- [Cloudflare STUN/TURN blog](https://blog.cloudflare.com/cloudflare-call/) — How Cloudflare runs TURN at scale
