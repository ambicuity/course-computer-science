"""
Subnet Calculator

Parses CIDR notation, computes network/broadcast/host ranges,
splits subnets, and aggregates subnets into supernets.

Usage:
    python main.py
"""

from dataclasses import dataclass
from typing import Optional


def ip_to_binary(ip: str) -> int:
    """Convert dotted-decimal IP to a 32-bit integer."""
    parts = ip.split(".")
    if len(parts) != 4:
        raise ValueError(f"Invalid IP: {ip}")
    result = 0
    for part in parts:
        octet = int(part)
        if not 0 <= octet <= 255:
            raise ValueError(f"Invalid octet: {octet}")
        result = (result << 8) | octet
    return result


def binary_to_ip(bits: int) -> str:
    """Convert a 32-bit integer to dotted-decimal IP string."""
    return f"{(bits >> 24) & 0xFF}.{(bits >> 16) & 0xFF}.{(bits >> 8) & 0xFF}.{bits & 0xFF}"


@dataclass
class Subnet:
    network: int       # 32-bit network address
    prefix: int        # prefix length (0-32)

    @property
    def mask(self) -> int:
        """Subnet mask as 32-bit integer."""
        if self.prefix == 0:
            return 0
        return (0xFFFFFFFF << (32 - self.prefix)) & 0xFFFFFFFF

    @property
    def cidr(self) -> str:
        return f"{binary_to_ip(self.network)}/{self.prefix}"

    @property
    def broadcast(self) -> int:
        return self.network | (~self.mask & 0xFFFFFFFF)

    @property
    def num_hosts(self) -> int:
        host_bits = 32 - self.prefix
        if host_bits <= 1:
            return 0
        return (1 << host_bits) - 2

    @property
    def host_range(self) -> tuple[str, str]:
        if self.num_hosts == 0:
            return (binary_to_ip(self.network), binary_to_ip(self.network))
        first = self.network + 1
        last = self.broadcast - 1
        return (binary_to_ip(first), binary_to_ip(last))

    @property
    def mask_str(self) -> str:
        return binary_to_ip(self.mask)

    def __str__(self) -> str:
        first, last = self.host_range
        return (
            f"Network:     {binary_to_ip(self.network)}/{self.prefix}\n"
            f"Mask:        {self.mask_str}\n"
            f"Broadcast:   {binary_to_ip(self.broadcast)}\n"
            f"Host range:  {first} - {last}\n"
            f"Usable hosts: {self.num_hosts}"
        )


def parse_cidr(cidr_string: str) -> Subnet:
    """Parse a CIDR string like '192.168.1.0/24' into a Subnet."""
    if "/" not in cidr_string:
        raise ValueError(f"Not a CIDR notation: {cidr_string}")
    ip_part, prefix_part = cidr_string.split("/")
    prefix = int(prefix_part)
    if not 0 <= prefix <= 32:
        raise ValueError(f"Invalid prefix: {prefix}")
    ip_bits = ip_to_binary(ip_part)
    # Mask off host bits to get the network address
    mask = (0xFFFFFFFF << (32 - prefix)) & 0xFFFFFFFF if prefix > 0 else 0
    network = ip_bits & mask
    return Subnet(network=network, prefix=prefix)


def split_subnet(cidr_string: str, new_prefix: int) -> list[Subnet]:
    """Split a subnet into smaller subnets with the given prefix length."""
    parent = parse_cidr(cidr_string)
    if new_prefix <= parent.prefix:
        raise ValueError(
            f"New prefix /{new_prefix} must be larger than /{parent.prefix}"
        )
    subnets = []
    step = 1 << (32 - new_prefix)
    current = parent.network
    end = parent.broadcast
    while current <= end:
        subnets.append(Subnet(network=current, prefix=new_prefix))
        current += step
    return subnets


def aggregate_subnets(cidr_strings: list[str]) -> Optional[Subnet]:
    """
    Attempt to aggregate a list of CIDR strings into a single supernet.
    Returns None if the subnets cannot be cleanly aggregated.
    """
    subnets = [parse_cidr(c) for c in cidr_strings]
    subnets.sort(key=lambda s: s.network)

    if not subnets:
        return None

    # Check if all subnets have the same prefix
    base_prefix = subnets[0].prefix
    if not all(s.prefix == base_prefix for s in subnets):
        return None

    # Check consecutive alignment
    step = 1 << (32 - base_prefix)
    for i in range(1, len(subnets)):
        if subnets[i].network != subnets[i - 1].network + step:
            return None

    # Find the supernet prefix
    count = len(subnets)
    # count must be a power of 2 for clean aggregation
    if count & (count - 1) != 0:
        return None

    import math
    bits_to_reduce = int(math.log2(count))
    new_prefix = base_prefix - bits_to_reduce

    # Verify all subnets align to the new supernet boundary
    supernet_network = subnets[0].network
    supernet_mask = (0xFFFFFFFF << (32 - new_prefix)) & 0xFFFFFFFF if new_prefix > 0 else 0
    if supernet_network != subnets[0].network & supernet_mask:
        return None

    return Subnet(network=supernet_network, prefix=new_prefix)


def demo():
    """Demonstrate the subnet calculator."""
    print("=" * 60)
    print("Subnet Calculator Demo")
    print("=" * 60)

    # Basic CIDR parsing
    print("\n--- Parse 192.168.1.0/24 ---")
    s = parse_cidr("192.168.1.0/24")
    print(s)

    # Non-standard network address (host bits set)
    print("\n--- Parse 10.0.0.130/26 (auto-aligns to network) ---")
    s = parse_cidr("10.0.0.130/26")
    print(s)

    # Large network
    print("\n--- Parse 10.0.0.0/8 ---")
    s = parse_cidr("10.0.0.0/8")
    print(s)

    # Point-to-point link
    print("\n--- Parse 172.16.0.0/30 ---")
    s = parse_cidr("172.16.0.0/30")
    print(s)

    # Split a /24 into /26 subnets
    print("\n--- Split 10.1.0.0/24 into /26 subnets ---")
    subs = split_subnet("10.1.0.0/24", 26)
    for i, sub in enumerate(subs):
        first, last = sub.host_range
        print(f"  Subnet {i+1}: {sub.cidr:<18} hosts: {first} - {last} ({sub.num_hosts})")

    # Aggregate four /24s into a /22
    print("\n--- Aggregate 192.168.{0,1,2,3}.0/24 ---")
    supernet = aggregate_subnets([
        "192.168.0.0/24",
        "192.168.1.0/24",
        "192.168.2.0/24",
        "192.168.3.0/24",
    ])
    if supernet:
        print(supernet)
    else:
        print("  Aggregation failed.")

    # Binary conversion demo
    print("\n--- Binary Conversion ---")
    ip = "192.168.10.50"
    bits = ip_to_binary(ip)
    print(f"  {ip} = {bin(bits)} = {bits:#010x}")
    print(f"  Back: {binary_to_ip(bits)}")


if __name__ == "__main__":
    demo()
