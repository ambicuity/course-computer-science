# DNS — Resolvers, Records, DNSSEC

> DNS — Resolvers, Records, DNSSEC — the part of CS you can't skip.

**Type:** Learn
**Languages:** Python, C
**Prerequisites:** Phase 09 lessons 01–11
**Time:** ~75 minutes

## Learning Objectives

- Understand DNS: how human-readable names map to IP addresses and beyond.
- Implement a DNS resolver that queries the DNS hierarchy from scratch.
- Compare your resolver against production tools (`dig`, `unbound`).
- Ship a DNS toolkit you can reuse for DNS-based service discovery.

## The Problem

You typed `google.com` in your browser. Your machine has no idea what IP that maps to. The TCP stack needs an IP address to connect to, but humans use names. **DNS** (Domain Name System, RFC 1035) bridges this gap — a distributed, hierarchical database mapping names to values.

DNS is arguably the most critical infrastructure on the internet. If DNS goes down, *nothing* works: websites, email, APIs, CDNs, even SSH (if you use hostnames). Understanding DNS means you can debug "the internet is down" in seconds.

## The Concept

### The DNS hierarchy

```
                    . (root)
                   / | \
                com  org  net  ...
               / | \
          google  amazon  github
         /    \
        www   mail
```

Resolution walks this tree top-down:

1. Your **recursive resolver** (e.g., 8.8.8.8) receives the query.
2. It asks a **root server**: "Who handles .com?" → gets `.com` TLD servers.
3. It asks a **TLD server**: "Who handles google.com?" → gets Google's authoritative servers.
4. It asks the **authoritative server**: "What is www.google.com?" → gets the IP.
5. It caches the answer for the **TTL** (time-to-live) duration.

### Record types

| Record | Purpose | Example |
|--------|---------|---------|
| **A** | IPv4 address | `google.com → 142.250.80.46` |
| **AAAA** | IPv6 address | `google.com → 2607:f8b0:4004:800::200e` |
| **CNAME** | Alias to another name | `www.example.com → example.com` |
| **MX** | Mail server (with priority) | `google.com → 10 smtp.google.com` |
| **NS** | Authoritative nameserver | `google.com → ns1.google.com` |
| **TXT** | Arbitrary text (SPF, DKIM, verification) | `google.com → "v=spf1 ..."` |
| **SOA** | Start of authority (zone metadata) | Primary NS, serial, refresh, retry |
| **PTR** | Reverse DNS (IP → name) | `46.80.250.142.in-addr.arpa → ...` |

### DNS packet format

Every DNS packet has this structure:

```
+--------------------------+
| Header (12 bytes)        |
|  ID, flags, counts       |
+--------------------------+
| Question section         |
|  QNAME, QTYPE, QCLASS    |
+--------------------------+
| Answer section           |
|  NAME, TYPE, CLASS, TTL, |
|  RDLENGTH, RDATA         |
+--------------------------+
| Authority section        |
+--------------------------+
| Additional section       |
+--------------------------+
```

### DNS header (12 bytes)

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           Transaction ID (random)                             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|QR|Opcode |AA|TC|RD|RA| Z  |RCODE |                          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|          QDCOUNT (questions)  |          ANCOUNT (answers)    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|          NSCOUNT (authority)  |          ARCOUNT (additional) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

- **QR**: 0=query, 1=response
- **RD**: Recursion Desired
- **RA**: Recursion Available

### DNS name encoding

Names are encoded as a sequence of length-prefixed labels, terminated by a zero byte:

```
www.google.com → \x03www\x06google\x03com\x00
```

This is called **DNS wire format** for names.

### DNSSEC

DNS is inherently insecure — responses can be spoofed. **DNSSEC** (RFC 4033–4035) adds cryptographic signatures:

1. Each zone has a **DNSKEY** — the public key for that zone.
2. Every record set is signed, producing an **RRSIG** record.
3. The **DS** (Delegation Signer) record in the parent zone binds the child's key.
4. Validation: verify RRSIG with DNSKEY, verify DNSKEY with DS from parent, chain up to the root (whose key is pre-configured as a trust anchor).

### DNS over HTTPS (DoH) / DNS over TLS (DoT)

Plain DNS (port 53, UDP/TCP) is unencrypted — ISPs can see every name you query. DoH (RFC 8484) wraps queries in HTTPS. DoT (RFC 7858) wraps them in TLS. Both provide confidentiality.

## Build It

### Step 1: DNS query builder (Python)

```python
import struct
import random
import socket

def encode_name(domain: str) -> bytes:
    """Encode a domain name in DNS wire format."""
    parts = domain.rstrip('.').split('.')
    result = b''
    for part in parts:
        result += bytes([len(part)]) + part.encode('ascii')
    result += b'\x00'
    return result

def build_query(domain: str, qtype: int = 1) -> bytes:
    """Build a DNS query packet. qtype: 1=A, 28=AAAA, 15=MX, etc."""
    txn_id = random.randint(0, 65535)
    flags = 0x0100  # Standard query, recursion desired
    header = struct.pack('!HHHHHH', txn_id, flags, 1, 0, 0, 0)
    question = encode_name(domain) + struct.pack('!HH', qtype, 1)  # QCLASS=IN
    return header + question
```

### Step 2: DNS response parser

```python
def decode_name(data: bytes, offset: int) -> tuple[str, int]:
    """Decode a DNS name, handling compression pointers."""
    labels = []
    jumped = False
    original_offset = offset
    max_jumps = 20

    while max_jumps > 0:
        length = data[offset]
        if length == 0:
            offset += 1
            break
        if (length & 0xC0) == 0xC0:
            # Compression pointer
            if not jumped:
                original_offset = offset + 2
            pointer = struct.unpack('!H', data[offset:offset+2])[0] & 0x3FFF
            offset = pointer
            jumped = True
            max_jumps -= 1
            continue
        offset += 1
        labels.append(data[offset:offset+length].decode('ascii'))
        offset += length

    name = '.'.join(labels)
    final_offset = original_offset if jumped else offset
    return name, final_offset

def parse_response(data: bytes) -> dict:
    """Parse a DNS response packet."""
    txn_id, flags, qdcount, ancount, nscount, arcount = \
        struct.unpack('!HHHHHH', data[:12])

    result = {
        'id': txn_id,
        'flags': flags,
        'is_response': (flags >> 15) & 1,
        'rcode': flags & 0x0F,
        'questions': [],
        'answers': [],
        'authority': [],
        'additional': [],
    }

    offset = 12

    # Parse questions
    for _ in range(qdcount):
        name, offset = decode_name(data, offset)
        qtype, qclass = struct.unpack('!HH', data[offset:offset+4])
        offset += 4
        result['questions'].append({
            'name': name,
            'type': qtype,
            'class': qclass,
        })

    # Parse resource records (answers, authority, additional)
    for section, count in [('answers', ancount),
                            ('authority', nscount),
                            ('additional', arcount)]:
        for _ in range(count):
            name, offset = decode_name(data, offset)
            rtype, rclass, ttl, rdlength = \
                struct.unpack('!HHIH', data[offset:offset+10])
            offset += 10
            rdata_raw = data[offset:offset+rdlength]
            rdata = parse_rdata(rtype, rdata_raw, data)
            offset += rdlength
            result[section].append({
                'name': name,
                'type': rtype,
                'class': rclass,
                'ttl': ttl,
                'data': rdata,
                'raw': rdata_raw,
            })

    return result

TYPE_NAMES = {
    1: 'A', 2: 'NS', 5: 'CNAME', 6: 'SOA', 15: 'MX',
    16: 'TXT', 28: 'AAAA', 33: 'SRV', 255: 'ANY',
}

def parse_rdata(rtype: int, rdata: bytes, full_packet: bytes) -> str:
    """Parse RDATA based on record type."""
    if rtype == 1 and len(rdata) == 4:
        return '.'.join(str(b) for b in rdata)
    elif rtype == 28 and len(rdata) == 16:
        return ':'.join(f'{rdata[i:i+2].hex()}' for i in range(0, 16, 2))
    elif rtype in (2, 5):  # NS, CNAME
        name, _ = decode_name(full_packet, 0)  # offset from rdata start
        # Actually need to decode from rdata bytes
        name = decode_rdata_name(rdata, full_packet)
        return name
    elif rtype == 15:  # MX
        priority = struct.unpack('!H', rdata[:2])[0]
        name = decode_rdata_name(rdata[2:], full_packet)
        return f'{priority} {name}'
    elif rtype == 16:  # TXT
        txt_len = rdata[0]
        return rdata[1:1+txt_len].decode('utf-8', errors='replace')
    return rdata.hex()

def decode_rdata_name(rdata: bytes, full_packet: bytes) -> str:
    """Decode a domain name from RDATA (may contain compression pointers)."""
    # Build a fake packet starting with the rdata to reuse decode_name
    temp = rdata
    name, _ = decode_name(temp, 0)
    return name
```

### Step 3: Recursive resolver

```python
class DNSResolver:
    """A recursive DNS resolver."""

    def __init__(self):
        self.cache: dict[str, tuple[float, str, int]] = {}
        # Root server hints (IPv4)
        self.roots = [
            '198.41.0.4',     # a.root-servers.net
            '199.9.14.201',   # b.root-servers.net
            '192.33.4.12',    # c.root-servers.net
            '199.7.91.13',    # d.root-servers.net
            '192.203.230.10', # e.root-servers.net
        ]

    def resolve(self, domain: str, qtype: int = 1) -> list[str]:
        """Resolve a domain name recursively."""
        cache_key = f'{domain}:{qtype}'
        if cache_key in self.cache:
            return [self.cache[cache_key][1]]

        return self._resolve_recursive(domain, qtype, self.roots)

    def _resolve_recursive(self, domain: str, qtype: int,
                            servers: list[str]) -> list[str]:
        """Recursively resolve by querying the given servers."""
        query = build_query(domain, qtype)

        for server in servers:
            try:
                sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
                sock.settimeout(3)
                sock.sendto(query, (server, 53))
                response_data, _ = sock.recvfrom(4096)
                sock.close()
                parsed = parse_response(response_data)

                if parsed['rcode'] != 0:
                    continue

                # Check for answers
                answers = [a for a in parsed['answers']
                           if a['type'] == qtype]
                if answers:
                    result = [a['data'] for a in answers]
                    for a in answers:
                        self.cache[f'{domain}:{qtype}'] = (
                            0, a['data'], a['ttl']
                        )
                    return result

                # Check for CNAME redirect
                cnames = [a for a in parsed['answers']
                          if a['type'] == 5]
                if cnames:
                    cname_target = cnames[0]['data']
                    return self._resolve_recursive(cname_target, qtype, self.roots)

                # Follow referrals — look for NS records in authority
                ns_records = [a['data'] for a in parsed['authority']
                              if a['type'] == 2]
                if ns_records:
                    # Try to find IPs for NS records in additional section
                    ns_ips = []
                    for ns_name in ns_records:
                        for addl in parsed['additional']:
                            if addl['name'] == ns_name and addl['type'] == 1:
                                ns_ips.append(addl['data'])
                    if ns_ips:
                        return self._resolve_recursive(domain, qtype, ns_ips)
                    # Resolve NS names ourselves
                    for ns_name in ns_records:
                        try:
                            ns_ips = self._resolve_recursive(ns_name, 1, self.roots)
                            if ns_ips:
                                return self._resolve_recursive(domain, qtype, ns_ips)
                        except Exception:
                            continue

            except (socket.timeout, socket.error):
                continue

        raise RuntimeError(f"Could not resolve {domain} from any server")
```

### Step 4: Demo

```python
def main() -> None:
    resolver = DNSResolver()

    print("=== DNS Resolver Demo ===\n")

    domains = ['google.com', 'github.com', 'example.com']

    for domain in domains:
        print(f"Resolving {domain}...")
        try:
            ips = resolver.resolve(domain, 1)  # A record
            for ip in ips:
                print(f"  A  → {ip}")
        except RuntimeError as e:
            print(f"  Error: {e}")
        print()

    # Resolve with dig-style query (raw UDP)
    print("=== Raw DNS Query (dig-style) ===\n")
    query = build_query('www.example.com', 1)
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(3)
    try:
        sock.sendto(query, ('8.8.8.8', 53))
        data, _ = sock.recvfrom(4096)
        parsed = parse_response(data)
        print(f"Query ID: {parsed['id']}")
        print(f"Response code: {parsed['rcode']}")
        for ans in parsed['answers']:
            tname = TYPE_NAMES.get(ans['type'], str(ans['type']))
            print(f"  {tname}  {ans['name']} → {ans['data']}  (TTL {ans['ttl']}s)")
    except socket.timeout:
        print("  Timeout querying 8.8.8.8")
    finally:
        sock.close()


if __name__ == '__main__':
    main()
```

## Use It

**Production DNS tools and servers:**

- **`dig`** (BIND): The standard DNS debugging tool. `dig google.com A @8.8.8.8` does exactly what our resolver does — builds a query, sends UDP to a nameserver, parses the response.
- **`unbound`**: Production recursive resolver. Implements DNSSEC validation, aggressive caching, prefetching, and QNAME minimization (privacy).
- **CoreDNS**: Cloud-native DNS server (Kubernetes default). Written in Go. Uses plugin architecture.
- **`getaddrinfo()`**: The libc function your programs actually call. It reads `/etc/resolv.conf` to find the recursive resolver, then does the resolution for you.

Our resolver is ~100 lines. Production resolvers add: TCP fallback (truncated responses), EDNS0 (extended DNS), DNSSEC validation, caching with TTL expiry, rate limiting, and query logging.

## Read the Source

- BIND 9 `lib/dns/resolver.c` — the `dns_resolver_create()` function shows how production resolvers manage iterative queries, timeouts, and caching.
- glibc `sysdeps/posix/getaddrinfo.c` — how `getaddrinfo()` actually works: calls into the system resolver, handles `/etc/hosts`, `/etc/resolv.conf`, and nsswitch.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A DNS toolkit: query builder, response parser, and recursive resolver** — reusable for service discovery, health checks, and monitoring in later lessons.

## Exercises

1. **Easy** — Add AAAA record resolution (qtype=28) to the resolver. Query `google.com` for both A and AAAA records and print the results.

2. **Medium** — Implement a simple DNS cache with TTL expiry. When a cached record's TTL expires, re-query. Add a `--flush` flag to clear the cache. Measure resolution time with and without cache.

3. **Hard** — Implement DNSSEC validation: given a DNS response with RRSIG records, verify the signature using the zone's DNSKEY. Start by validating a single zone (no chain). Then implement DS record verification to validate across zones. Use the `cryptography` library for RSA/ECDSA signature verification.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Recursive resolver | "Your DNS server" | A DNS server that queries the hierarchy on your behalf and returns a final answer |
| Authoritative server | "Auth server" | The DNS server that holds the definitive records for a zone |
| TLD | "Top-level domain" | The rightmost label: `.com`, `.org`, `.net` — managed by registry operators |
| TTL | "Time to live" | Seconds a cached DNS record is valid before it must be re-fetched |
| DNSSEC | "DNS Security" | Cryptographic extensions that sign DNS records to prevent spoofing |
| DoH / DoT | "Encrypted DNS" | DNS over HTTPS (port 443) / DNS over TLS (port 853) for privacy |
| NSEC / NSEC3 | "Negative answers" | DNSSEC records proving a name does NOT exist (authenticated denial of existence) |

## Further Reading

- RFC 1035 — the original DNS specification. Section 4: "Messages" defines the wire format.
- RFC 4033–4035 — DNSSEC specifications.
- [Julia Evans' DNS zine](https://wizardzines.com/comics/dns/) — visual explanation of DNS resolution.
- [How DNS Works](https://howdns.works/) — illustrated guide to the DNS hierarchy.
