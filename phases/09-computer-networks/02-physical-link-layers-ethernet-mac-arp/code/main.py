"""
ARP (Address Resolution Protocol) Implementation

Simulates an ARP table with caching, TTL-based expiration,
ARP request broadcast, and unicast ARP reply handling.

Usage:
    python main.py
"""

import time
from dataclasses import dataclass, field
from typing import Optional


@dataclass
class ARPEntry:
    ip: str
    mac: str
    created_at: float
    ttl: float  # seconds

    @property
    def expired(self) -> bool:
        return (time.time() - self.created_at) > self.ttl


class ARPTable:
    """ARP cache with TTL-based entry expiration."""

    def __init__(self, default_ttl: float = 120.0):
        self.entries: dict[str, ARPEntry] = {}
        self.default_ttl = default_ttl
        self.stats = {"requests_sent": 0, "replies_received": 0, "cache_hits": 0}

    def add(self, ip: str, mac: str, ttl: Optional[float] = None) -> None:
        self.entries[ip] = ARPEntry(
            ip=ip,
            mac=mac,
            created_at=time.time(),
            ttl=ttl if ttl is not None else self.default_ttl,
        )

    def remove(self, ip: str) -> None:
        self.entries.pop(ip, None)

    def resolve(self, ip: str) -> Optional[str]:
        """Resolve an IP to a MAC address. Returns None if not cached."""
        entry = self.entries.get(ip)
        if entry is None:
            return None
        if entry.expired:
            self.remove(ip)
            return None
        self.stats["cache_hits"] += 1
        return entry.mac

    def cleanup_expired(self) -> int:
        """Remove all expired entries. Returns count of removed entries."""
        expired = [ip for ip, e in self.entries.items() if e.expired]
        for ip in expired:
            self.remove(ip)
        return len(expired)

    def arp_request(self, target_ip: str, sender_ip: str, sender_mac: str,
                    network_mac_table: dict[str, str]) -> Optional[str]:
        """
        Simulate sending an ARP request (broadcast) and receiving a reply.

        In a real network, this would broadcast on the wire. Here, we look up
        the target in a simulated network MAC table.

        Args:
            target_ip: The IP we want to resolve.
            sender_ip: Our IP.
            sender_mac: Our MAC.
            network_mac_table: Simulated mapping of IP -> MAC for all hosts.

        Returns:
            The MAC address of the target, or None if unreachable.
        """
        print(f"[ARP] Sending REQUEST: Who has {target_ip}? Tell {sender_ip}")
        self.stats["requests_sent"] += 1

        target_mac = network_mac_table.get(target_ip)
        if target_mac is None:
            print(f"[ARP] No reply for {target_ip} (host unreachable)")
            return None

        print(f"[ARP] Received REPLY: {target_ip} is at {target_mac}")
        self.stats["replies_received"] += 1
        self.add(target_ip, target_mac)
        return target_mac

    def arp_reply(self, request_ip: str, request_mac: str,
                  reply_ip: str, reply_mac: str) -> None:
        """Handle an incoming ARP reply — add to cache."""
        print(f"[ARP] Processing REPLY: {reply_ip} is at {reply_mac}")
        self.add(reply_ip, reply_mac)
        self.stats["replies_received"] += 1

    def display(self) -> None:
        """Print the ARP table."""
        self.cleanup_expired()
        print("\n=== ARP Table ===")
        print(f"{'IP Address':<18} {'MAC Address':<20} {'TTL (s)':<10} {'Status'}")
        print("-" * 60)
        if not self.entries:
            print("(empty)")
        for ip, entry in sorted(self.entries.items()):
            remaining = max(0, entry.ttl - (time.time() - entry.created_at))
            status = "VALID" if not entry.expired else "EXPIRED"
            print(f"{entry.ip:<18} {entry.mac:<20} {remaining:<10.0f} {status}")
        print(f"\nStats: {self.stats}")
        print()


def demo():
    """Demonstrate ARP resolution and cache management."""
    # Simulated network: IP -> MAC mappings
    network = {
        "10.0.0.1": "aa:bb:cc:dd:ee:01",
        "10.0.0.2": "de:ad:be:ef:00:02",
        "10.0.0.3": "01:23:45:67:89:ab",
    }

    my_ip = "10.0.0.100"
    my_mac = "ff:ee:dd:cc:bb:aa"

    table = ARPTable(default_ttl=5.0)

    print("--- ARP Resolution Demo ---\n")

    # Resolve each host on the simulated network
    for ip in network:
        result = table.arp_request(ip, my_ip, my_mac, network)
        if result:
            print(f"  Resolved {ip} -> {result}\n")

    table.display()

    # Test cache hit (no new request needed)
    print("--- Cache Hit Test ---")
    cached = table.resolve("10.0.0.1")
    print(f"Resolving 10.0.0.1 from cache: {cached}")
    print(f"Cache hits so far: {table.stats['cache_hits']}\n")

    # Simulate TTL expiration
    print("--- Waiting for TTL expiration (6 seconds) ---")
    time.sleep(6)
    expired_count = table.cleanup_expired()
    print(f"Expired entries removed: {expired_count}")
    table.display()

    # Unreachable host
    print("--- Unreachable Host Test ---")
    table.arp_request("10.0.0.99", my_ip, my_mac, network)
    table.display()


if __name__ == "__main__":
    demo()
