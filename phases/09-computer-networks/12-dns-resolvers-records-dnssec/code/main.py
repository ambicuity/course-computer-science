"""
main.py — DNS resolver, query builder, and response parser
Phase 09 — Computer Networks, Lesson 12

Builds DNS query packets, parses responses, and implements
a recursive DNS resolver from scratch.

Run:    python main.py
"""

import struct
import random
import socket
import sys

# ─── DNS Record Types ────────────────────────────────────────────────────────

TYPE_A = 1
TYPE_NS = 2
TYPE_CNAME = 5
TYPE_SOA = 6
TYPE_MX = 15
TYPE_TXT = 16
TYPE_AAAA = 28

TYPE_NAMES = {
    1: 'A', 2: 'NS', 5: 'CNAME', 6: 'SOA',
    15: 'MX', 16: 'TXT', 28: 'AAAA', 255: 'ANY',
}

# ─── DNS Name Encoding / Decoding ────────────────────────────────────────────


def encode_name(domain: str) -> bytes:
    """Encode a domain name in DNS wire format.

    Example: 'www.google.com' -> b'\\x03www\\x06google\\x03com\\x00'
    """
    parts = domain.rstrip(".").split(".")
    result = b""
    for part in parts:
        encoded = part.encode("ascii")
        result += bytes([len(encoded)]) + encoded
    result += b"\x00"
    return result


def decode_name(data: bytes, offset: int) -> tuple[str, int]:
    """Decode a DNS name from wire format, handling compression pointers.

    Returns (name_string, new_offset).
    """
    labels: list[str] = []
    jumped = False
    original_offset = offset
    max_jumps = 20

    while max_jumps > 0:
        if offset >= len(data):
            break

        length = data[offset]

        if length == 0:
            offset += 1
            break

        if (length & 0xC0) == 0xC0:
            # Compression pointer: 2-byte pointer to earlier name
            if not jumped:
                original_offset = offset + 2
            pointer = struct.unpack("!H", data[offset : offset + 2])[0] & 0x3FFF
            offset = pointer
            jumped = True
            max_jumps -= 1
            continue

        offset += 1
        labels.append(data[offset : offset + length].decode("ascii"))
        offset += length

    name = ".".join(labels)
    final_offset = original_offset if jumped else offset
    return name, final_offset


# ─── DNS Query Builder ───────────────────────────────────────────────────────


def build_query(domain: str, qtype: int = TYPE_A) -> bytes:
    """Build a DNS query packet.

    Args:
        domain: The domain name to query (e.g., 'google.com').
        qtype: The record type (1=A, 28=AAAA, 15=MX, etc.).

    Returns:
        Raw DNS query packet as bytes.
    """
    txn_id = random.randint(0, 0xFFFF)
    flags = 0x0100  # Standard query, recursion desired
    header = struct.pack("!HHHHHH", txn_id, flags, 1, 0, 0, 0)
    question = encode_name(domain) + struct.pack("!HH", qtype, 1)  # QCLASS=IN
    return header + question


# ─── DNS Response Parser ─────────────────────────────────────────────────────


def parse_rdata(rtype: int, rdata: bytes, full_packet: bytes) -> str:
    """Parse RDATA based on record type, returning a human-readable string."""
    if rtype == TYPE_A and len(rdata) == 4:
        return socket.inet_ntoa(rdata)

    if rtype == TYPE_AAAA and len(rdata) == 16:
        # Format IPv6 address
        parts = [rdata[i : i + 2].hex() for i in range(0, 16, 2)]
        # Remove leading zeros and compress
        addr = ":".join(parts)
        # Simple compression: replace longest run of 0000 with ::
        while "::" not in addr:
            addr = addr.replace(":0000", ":0:", 1)
        addr = addr.replace(":0:", "::", 1)
        addr = addr.strip(":")
        return addr

    if rtype in (TYPE_NS, TYPE_CNAME):
        name, _ = _decode_rdata_name(rdata, full_packet)
        return name

    if rtype == TYPE_MX:
        priority = struct.unpack("!H", rdata[:2])[0]
        name, _ = _decode_rdata_name(rdata[2:], full_packet)
        return f"{priority} {name}"

    if rtype == TYPE_TXT:
        if len(rdata) > 0:
            txt_len = rdata[0]
            return rdata[1 : 1 + txt_len].decode("utf-8", errors="replace")
        return ""

    if rtype == TYPE_SOA:
        primary_ns, offset = _decode_rdata_name(rdata, full_packet)
        admin_mb, offset = _decode_rdata_name(rdata, offset - (rdata[0:0].__len__() - len(rdata) + offset) + len(rdata))
        serial, refresh, retry, expire, minimum = struct.unpack(
            "!IIIII", rdata[offset : offset + 20]
        )
        return f"{primary_ns} {admin_mb} {serial} {refresh} {retry} {expire} {minimum}"

    return rdata.hex()


def _decode_rdata_name(rdata: bytes, full_packet: bytes) -> tuple[str, int]:
    """Decode a domain name from RDATA.

    We build a temporary buffer that combines the rdata with enough
    of the full packet so that compression pointers resolve correctly.
    """
    # Create a temporary buffer: rdata at offset 0, rest of packet after
    # This allows compression pointers in rdata to point into the original packet
    temp = bytearray(len(full_packet))
    temp[: len(rdata)] = rdata
    temp[len(rdata) :] = full_packet[len(rdata) :]
    name, new_offset = decode_name(bytes(temp), 0)
    return name, new_offset - len(rdata) + len(rdata)


def parse_response(data: bytes) -> dict:
    """Parse a DNS response packet into a structured dict.

    Returns dict with keys: id, flags, is_response, rcode,
    questions, answers, authority, additional.
    """
    if len(data) < 12:
        raise ValueError("DNS response too short")

    txn_id, flags, qdcount, ancount, nscount, arcount = struct.unpack(
        "!HHHHHH", data[:12]
    )

    result: dict = {
        "id": txn_id,
        "flags": flags,
        "is_response": (flags >> 15) & 1,
        "opcode": (flags >> 11) & 0x0F,
        "authoritative": (flags >> 10) & 1,
        "truncated": (flags >> 9) & 1,
        "recursion_desired": (flags >> 8) & 1,
        "recursion_available": (flags >> 7) & 1,
        "rcode": flags & 0x0F,
        "questions": [],
        "answers": [],
        "authority": [],
        "additional": [],
    }

    offset = 12

    # Parse question section
    for _ in range(qdcount):
        name, offset = decode_name(data, offset)
        if offset + 4 > len(data):
            break
        qtype, qclass = struct.unpack("!HH", data[offset : offset + 4])
        offset += 4
        result["questions"].append(
            {"name": name, "type": qtype, "class": qclass}
        )

    # Parse answer, authority, additional sections
    for section_name, count in [
        ("answers", ancount),
        ("authority", nscount),
        ("additional", arcount),
    ]:
        for _ in range(count):
            name, offset = decode_name(data, offset)
            if offset + 10 > len(data):
                break
            rtype, rclass, ttl, rdlength = struct.unpack(
                "!HHIH", data[offset : offset + 10]
            )
            offset += 10
            rdata_raw = data[offset : offset + rdlength]
            rdata = parse_rdata(rtype, rdata_raw, data)
            offset += rdlength

            result[section_name].append(
                {
                    "name": name,
                    "type": rtype,
                    "type_name": TYPE_NAMES.get(rtype, str(rtype)),
                    "class": rclass,
                    "ttl": ttl,
                    "data": rdata,
                }
            )

    return result


# ─── DNS Query Utility ───────────────────────────────────────────────────────


def dns_query(
    domain: str, qtype: int = TYPE_A, server: str = "8.8.8.8", timeout: float = 3.0
) -> dict:
    """Send a DNS query to a server and return the parsed response."""
    query = build_query(domain, qtype)
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(timeout)
    try:
        sock.sendto(query, (server, 53))
        data, _ = sock.recvfrom(4096)
        return parse_response(data)
    finally:
        sock.close()


# ─── Recursive Resolver ──────────────────────────────────────────────────────


class DNSResolver:
    """A recursive DNS resolver that walks the DNS hierarchy."""

    ROOT_SERVERS = [
        "198.41.0.4",  # a.root-servers.net
        "199.9.14.201",  # b.root-servers.net
        "192.33.4.12",  # c.root-servers.net
        "199.7.91.13",  # d.root-servers.net
        "192.203.230.10",  # e.root-servers.net
    ]

    def __init__(self) -> None:
        self.cache: dict[str, list[str]] = {}
        self.queries_sent = 0

    def resolve(self, domain: str, qtype: int = TYPE_A) -> list[str]:
        """Resolve a domain name recursively from root servers."""
        cache_key = f"{domain}:{qtype}"
        if cache_key in self.cache:
            return self.cache[cache_key]

        result = self._resolve_recursive(domain, qtype, self.ROOT_SERVERS)
        self.cache[cache_key] = result
        return result

    def _resolve_recursive(
        self, domain: str, qtype: int, servers: list[str]
    ) -> list[str]:
        """Query servers iteratively until we get an answer or fail."""
        query = build_query(domain, qtype)

        for server in servers:
            try:
                sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
                sock.settimeout(3)
                sock.sendto(query, (server, 53))
                self.queries_sent += 1
                response_data, _ = sock.recvfrom(4096)
                sock.close()
                parsed = parse_response(response_data)

                if parsed["rcode"] != 0:
                    continue

                # Direct answer
                answers = [a for a in parsed["answers"] if a["type"] == qtype]
                if answers:
                    return [a["data"] for a in answers]

                # CNAME redirect
                cnames = [a for a in parsed["answers"] if a["type"] == TYPE_CNAME]
                if cnames:
                    cname_target = cnames[0]["data"]
                    return self._resolve_recursive(cname_target, qtype, self.ROOT_SERVERS)

                # Referral: follow NS records
                ns_records = [a["data"] for a in parsed["authority"] if a["type"] == TYPE_NS]
                if ns_records:
                    # Try IPs from additional section first
                    ns_ips = []
                    for ns_name in ns_records:
                        for addl in parsed["additional"]:
                            if addl["name"] == ns_name and addl["type"] == TYPE_A:
                                ns_ips.append(addl["data"])

                    if ns_ips:
                        return self._resolve_recursive(domain, qtype, ns_ips)

                    # Resolve NS names ourselves
                    for ns_name in ns_records:
                        try:
                            ns_ips = self._resolve_recursive(
                                ns_name, TYPE_A, self.ROOT_SERVERS
                            )
                            if ns_ips:
                                return self._resolve_recursive(domain, qtype, ns_ips)
                        except RuntimeError:
                            continue

            except (socket.timeout, socket.error):
                continue

        raise RuntimeError(f"Could not resolve {domain} (tried {len(servers)} servers)")


# ─── Main ────────────────────────────────────────────────────────────────────


def main() -> None:
    print("=" * 60)
    print("  DNS Resolver & Packet Parser — Lesson 12")
    print("=" * 60)

    # ── Part 1: Raw DNS query with dig-style output ──────────────────────

    print("\n── Part 1: Raw DNS Query (dig-style) ──\n")
    query = build_query("www.example.com", TYPE_A)
    print(f"Query packet: {len(query)} bytes")
    print(f"  Transaction ID: {struct.unpack('!H', query[:2])[0]:#06x}")
    print(f"  Flags: {struct.unpack('!H', query[2:4])[0]:#06x}")
    print(f"  Questions: {struct.unpack('!H', query[4:6])[0]}")

    try:
        resp = dns_query("www.example.com", TYPE_A, "8.8.8.8")
        print(f"\nResponse from 8.8.8.8:")
        print(f"  RCODE: {resp['rcode']}")
        print(f"  Answers: {len(resp['answers'])}")
        for ans in resp["answers"]:
            print(f"    {ans['type_name']:6s}  {ans['name']:30s}  →  {ans['data']:20s}  (TTL {ans['ttl']}s)")
        if resp["authority"]:
            print(f"  Authority: {len(resp['authority'])}")
            for auth in resp["authority"][:3]:
                print(f"    {auth['type_name']:6s}  {auth['name']:30s}  →  {auth['data']}")
    except Exception as e:
        print(f"  (skipped — network unavailable: {e})")

    # ── Part 2: Multiple record types ────────────────────────────────────

    print("\n── Part 2: Multiple Record Types ──\n")
    test_queries = [
        ("google.com", TYPE_A, "A (IPv4)"),
        ("google.com", TYPE_AAAA, "AAAA (IPv6)"),
        ("google.com", TYPE_MX, "MX (Mail)"),
        ("google.com", TYPE_NS, "NS (Nameserver)"),
    ]

    for domain, qtype, label in test_queries:
        try:
            resp = dns_query(domain, qtype, "8.8.8.8")
            answers = [a for a in resp["answers"] if a["type"] == qtype]
            print(f"{domain:20s}  {label:20s}  →  ", end="")
            if answers:
                print(", ".join(a["data"] for a in answers[:3]))
            else:
                print("(no answer)")
        except Exception as e:
            print(f"(skipped: {e})")

    # ── Part 3: Recursive resolution from root ───────────────────────────

    print("\n── Part 3: Recursive Resolver (from root servers) ──\n")
    resolver = DNSResolver()

    domains_to_resolve = ["example.com", "google.com"]
    for domain in domains_to_resolve:
        print(f"Resolving {domain}...")
        try:
            ips = resolver.resolve(domain, TYPE_A)
            for ip in ips:
                print(f"  {domain} → {ip}")
        except RuntimeError as e:
            print(f"  Error: {e}")

    print(f"\nTotal DNS queries sent: {resolver.queries_sent}")
    print(f"Cache entries: {len(resolver.cache)}")

    print("\nDone.")


if __name__ == "__main__":
    main()
