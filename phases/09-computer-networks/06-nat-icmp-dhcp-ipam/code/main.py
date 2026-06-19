"""
NAT, ICMP, DHCP, IPAM
Phase 09 — Computer Networks

Simulators for Network Address Translation, DHCP lease management,
ICMP ping/traceroute, and basic IP address management.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple


# ---------------------------------------------------------------------------
# NAT
# ---------------------------------------------------------------------------

@dataclass
class NAT:
    """Simulates a PAT (Port Address Translation) device."""
    external_ip: str
    table: Dict[Tuple[str, int], Tuple[str, int]] = field(default_factory=dict)
    reverse: Dict[Tuple[str, int], Tuple[str, int]] = field(default_factory=dict)
    _next_port: int = 50000

    def translate(self, src_ip: str, src_port: int) -> Tuple[str, int]:
        """Translate internal (ip, port) to external (ip, port)."""
        key = (src_ip, src_port)
        if key not in self.table:
            ext_port = self._next_port
            self._next_port += 1
            self.table[key] = (self.external_ip, ext_port)
            self.reverse[(self.external_ip, ext_port)] = key
        return self.table[key]

    def reverse_translate(self, ext_ip: str, ext_port: int) -> Optional[Tuple[str, int]]:
        """Look up the internal (ip, port) for an incoming packet."""
        return self.reverse.get((ext_ip, ext_port))

    def print_table(self):
        print(f"  NAT Table (external IP: {self.external_ip}):")
        for (int_ip, int_port), (ext_ip, ext_port) in sorted(self.table.items()):
            print(f"    {int_ip}:{int_port} <-> {ext_ip}:{ext_port}")


# ---------------------------------------------------------------------------
# DHCP
# ---------------------------------------------------------------------------

@dataclass
class DHCP:
    """Simulates a DHCP server with DORA lease flow."""
    pool_start: str
    pool_end: str
    subnet_mask: str = "255.255.255.0"
    gateway: str = "192.168.1.1"
    dns: str = "8.8.8.8"
    lease_time: int = 3600
    leases: Dict[str, Tuple[str, int]] = field(default_factory=dict)  # mac -> (ip, expiry)
    _available: List[str] = field(default_factory=list)
    _allocated: set = field(default_factory=set)

    def __post_init__(self):
        start = self._ip_to_int(self.pool_start)
        end = self._ip_to_int(self.pool_end)
        self._available = [self._int_to_ip(i) for i in range(start, end + 1)]

    def discover_offer(self, mac: str) -> Optional[str]:
        """DHCP Discover -> Offer. Returns offered IP or None."""
        # If this MAC already has a lease, offer the same IP
        if mac in self.leases:
            ip = self.leases[mac][0]
            return ip
        # Otherwise offer a new IP from the pool
        if not self._available:
            return None
        ip = self._available.pop(0)
        return ip

    def request_ack(self, mac: str, ip: str) -> bool:
        """DHCP Request -> Ack. Commits the lease."""
        self.leases[mac] = (ip, self.lease_time)
        self._allocated.add(ip)
        return True

    def release(self, mac: str):
        """Release a lease back to the pool."""
        if mac in self.leases:
            ip, _ = self.leases.pop(mac)
            self._allocated.discard(ip)
            self._available.append(ip)

    def print_leases(self):
        print("  DHCP Leases:")
        if not self.leases:
            print("    (none)")
        for mac, (ip, expiry) in sorted(self.leases.items()):
            print(f"    {mac} -> {ip} (lease {expiry}s)")

    @staticmethod
    def _ip_to_int(ip: str) -> int:
        parts = ip.split(".")
        return (int(parts[0]) << 24) | (int(parts[1]) << 16) | (int(parts[2]) << 8) | int(parts[3])

    @staticmethod
    def _int_to_ip(n: int) -> str:
        return f"{(n >> 24) & 0xFF}.{(n >> 16) & 0xFF}.{(n >> 8) & 0xFF}.{n & 0xFF}"


# ---------------------------------------------------------------------------
# ICMP
# ---------------------------------------------------------------------------

@dataclass
class ICMPPacket:
    """Represents an ICMP message."""
    type: int        # 0=echo_reply, 8=echo_request, 11=time_exceeded, 3=dest_unreachable
    code: int
    src: str
    dst: str
    ttl: int = 64
    payload: str = ""

    def __repr__(self):
        type_names = {0: "echo_reply", 8: "echo_request", 11: "time_exceeded", 3: "dest_unreachable"}
        tname = type_names.get(self.type, f"type_{self.type}")
        return f"ICMP({tname} code={self.code} {self.src} -> {self.dst} ttl={self.ttl})"


def ping(src: str, dst: str, ttl: int = 64) -> ICMPPacket:
    """Simulate a ping (echo request -> echo reply)."""
    request = ICMPPacket(type=8, code=0, src=src, dst=dst, ttl=ttl, payload="ping")
    reply = ICMPPacket(type=0, code=0, src=dst, dst=src, ttl=64, payload="pong")
    return reply


def traceroute(src: str, dst: str, hops: List[str], max_hops: int = 30) -> List[Tuple[int, str]]:
    """Simulate traceroute. Returns list of (ttl, hop_ip)."""
    result = []
    for ttl in range(1, max_hops + 1):
        if ttl > len(hops):
            result.append((ttl, "*"))
            continue
        hop = hops[ttl - 1]
        if hop == dst:
            result.append((ttl, hop))
            break
        # Router sends ICMP Time Exceeded
        time_exceeded = ICMPPacket(type=11, code=0, src=hop, dst=src, ttl=64)
        result.append((ttl, hop))
    return result


# ---------------------------------------------------------------------------
# IPAM
# ---------------------------------------------------------------------------

@dataclass
class IPAMEntry:
    ip: str
    mac: Optional[str]
    hostname: Optional[str]
    reserved: bool = False


class IPAM:
    """Simple IP Address Management tracker."""

    def __init__(self):
        self.entries: Dict[str, IPAMEntry] = {}

    def add_entry(self, ip: str, mac: Optional[str] = None,
                  hostname: Optional[str] = None, reserved: bool = False):
        self.entries[ip] = IPAMEntry(ip=ip, mac=mac, hostname=hostname, reserved=reserved)

    def is_allocated(self, ip: str) -> bool:
        return ip in self.entries

    def print_table(self):
        print("  IPAM Table:")
        for ip in sorted(self.entries):
            e = self.entries[ip]
            status = "reserved" if e.reserved else "allocated"
            mac = e.mac or "-"
            host = e.hostname or "-"
            print(f"    {ip}  mac={mac}  host={host}  [{status}]")


# ---------------------------------------------------------------------------
# Demos
# ---------------------------------------------------------------------------

def demo_nat():
    """Demonstrate NAT translation."""
    print("=" * 60)
    print("DEMO 1: NAT / PAT Translation")
    print("=" * 60)
    nat = NAT(external_ip="203.0.113.1")

    # Three internal hosts connect to external server
    hosts = [
        ("192.168.1.5", 4000),
        ("192.168.1.6", 4000),  # same port as host 1 — NAT must assign different ext port
        ("192.168.1.5", 5000),
    ]

    print("  Outbound translations:")
    for ip, port in hosts:
        ext_ip, ext_port = nat.translate(ip, port)
        print(f"    {ip}:{port} -> {ext_ip}:{ext_port}")

    print("\n  Inbound reverse lookup:")
    for ext_ip, ext_port in [(nat.external_ip, 50000), (nat.external_ip, 50001)]:
        result = nat.reverse_translate(ext_ip, ext_port)
        if result:
            print(f"    {ext_ip}:{ext_port} -> {result[0]}:{result[1]}")
        else:
            print(f"    {ext_ip}:{ext_port} -> no mapping")

    print()
    nat.print_table()
    print()


def demo_dhcp():
    """Demonstrate DHCP DORA flow."""
    print("=" * 60)
    print("DEMO 2: DHCP DORA Lease Process")
    print("=" * 60)
    server = DHCP(pool_start="192.168.1.100", pool_end="192.168.1.110",
                   gateway="192.168.1.1", dns="8.8.8.8")

    clients = ["aa:bb:cc:dd:ee:01", "aa:bb:cc:dd:ee:02", "aa:bb:cc:dd:ee:03"]

    for mac in clients:
        print(f"\n  Client {mac}:")
        # Discover -> Offer
        offered_ip = server.discover_offer(mac)
        print(f"    DISCOVER -> OFFER: {offered_ip}")
        # Request -> Ack
        if offered_ip:
            ok = server.request_ack(mac, offered_ip)
            print(f"    REQUEST  -> ACK:   {'granted' if ok else 'denied'}")
            print(f"    Config: ip={offered_ip}, mask={server.subnet_mask}, gw={server.gateway}, dns={server.dns}")

    print()
    server.print_leases()

    # Release one lease
    print("\n  >>> Releasing lease for", clients[0])
    server.release(clients[0])
    server.print_leases()

    # IPAM tie-in
    print()
    ipam = IPAM()
    for mac, (ip, _) in server.leases.items():
        ipam.add_entry(ip=ip, mac=mac, hostname=f"host-{mac[-2:]}")
    ipam.add_entry(ip="192.168.1.1", mac=None, hostname="gateway", reserved=True)
    ipam.print_table()
    print()


def demo_icmp():
    """Demonstrate ICMP ping and traceroute."""
    print("=" * 60)
    print("DEMO 3: ICMP Ping and Traceroute")
    print("=" * 60)

    reply = ping("192.168.1.5", "8.8.8.8")
    print(f"  Ping: {reply}\n")

    path = ["192.168.1.1", "10.0.0.1", "72.14.215.1", "8.8.8.8"]
    print("  Traceroute from 192.168.1.5 to 8.8.8.8:")
    hops = traceroute("192.168.1.5", "8.8.8.8", path)
    for ttl, hop in hops:
        print(f"    {ttl}: {hop}")
    print()


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    demo_nat()
    demo_dhcp()
    demo_icmp()


if __name__ == "__main__":
    main()
