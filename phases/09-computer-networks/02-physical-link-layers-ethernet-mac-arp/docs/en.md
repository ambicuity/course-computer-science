# Physical & Link Layers — Ethernet, MAC, ARP

## Overview

The bottom two OSI layers handle the most fundamental networking task:
getting bits from one device to the next on the same LAN segment.
This lesson covers **Ethernet framing**, **MAC addresses**, and **ARP**.

---

## Ethernet Frame Format

An Ethernet frame (IEEE 802.3) is the basic unit of data on a wired LAN:

```
┌───────────┬────────┬────────┬───────┬──────────────────┬─────┐
│ Preamble  │  Dst   │  Src   │Ether- │                  │ FCS │
│ (8 bytes) │ MAC(6) │ MAC(6) │Type(2)│ Payload (46-1500)│ (4) │
└───────────┴────────┴────────┴───────┴──────────────────┴─────┘
```

| Field | Size | Description |
|-------|------|-------------|
| Preamble | 7+1 bytes | Bit synchronization (SFD = 10101011) |
| Destination MAC | 6 bytes | Target device's MAC address |
| Source MAC | 6 bytes | Sender's MAC address |
| EtherType | 2 bytes | Upper-layer protocol (0x0800=IPv4, 0x0806=ARP, 0x86DD=IPv6) |
| Payload | 46–1500 bytes | Data (padding added if < 46 bytes) |
| FCS | 4 bytes | CRC-32 checksum for error detection |

The **MTU** for standard Ethernet is **1500 bytes** of payload.

---

## MAC Addresses

A **MAC address** is a 48-bit (6-byte) identifier burned into every NIC.

```
  AA:BB:CC:DD:EE:FF
  ├─OUI─┤ ├─NIC─┤
  (24 bit) (24 bit)
```

- **OUI**: manufacturer identifier (first 3 bytes).
- **NIC**: unique per device (last 3 bytes).
- Written as six hex octets separated by colons.

### Address Types

| Type | Rule | Example | Meaning |
|------|------|---------|---------|
| Unicast | LSBit of first byte = 0 | `00:1A:2B:3C:4D:5E` | One device |
| Multicast | LSBit of first byte = 1 | `01:00:5E:00:00:01` | A group |
| Broadcast | All 48 bits = 1 | `FF:FF:FF:FF:FF:FF` | Every device |

---

## ARP — Address Resolution Protocol

A host knows the destination **IP** but needs the **MAC** to send a frame.
**ARP** bridges this gap.

```
Host A (10.0.0.1, MAC_A) → Host B (10.0.0.2, unknown MAC)

1. ARP REQUEST (broadcast):
   "Who has 10.0.0.2? Tell 10.0.0.1"
   Dst MAC: FF:FF:FF:FF:FF:FF

2. ARP REPLY (unicast):
   "10.0.0.2 is at MAC_B"
   Dst MAC: MAC_A

3. Host A caches 10.0.0.2 → MAC_B.
```

### ARP Cache

Entries have a **TTL** (60–300 seconds). Stale entries are re-resolved:

```bash
$ arp -a
? (10.0.0.1) at aa:bb:cc:dd:ee:01 on en0 [ethernet]
```

### ARP Spoofing

Because ARP has **no authentication**, an attacker can send gratuitous replies
claiming to own a victim's IP. This poisons the cache and enables
man-in-the-middle attacks. Defenses: Dynamic ARP Inspection, static ARP entries,
802.1X port security.

---

## Ethernet Switches

A **switch** operates at Layer 2, learning which MAC addresses are on which ports.

1. Frame arrives on port 3 with source MAC `AA:BB:CC:DD:EE:01`.
2. Switch records: `AA:BB:CC:DD:EE:01 → port 3`.
3. Switch looks up destination MAC:
   - **Found**: forward to that port only.
   - **Not found**: flood all ports except source.
   - **Broadcast** (`FF:FF:FF:FF:FF:FF`): flood all ports.

---

## CSMA/CD (Legacy)

In hub-based Ethernet, **Carrier Sense Multiple Access with Collision Detection**:
1. **Listen** before transmitting.
2. Collision detected → **stop**, send jam signal.
3. Wait **random backoff**, retry.

Full-duplex switched Ethernet eliminates collisions entirely.

---

## Build It: Ethernet Frame Parser

See `code/main.c` — a C program that parses raw Ethernet frame bytes,
prints MAC addresses, EtherType, payload length, and verifies CRC-32.

---

## Build It: ARP Implementation

See `code/main.py` — a Python ARP table simulation with cache, TTL expiration,
request/reply handling, and a demo of ARP resolution.

---

## Use It: Inspecting Layer 2

```bash
arp -a                                    # view ARP cache
show mac address-table                    # Cisco switch MAC table
eth.dst == ff:ff:ff:ff:ff:ff             # Wireshark: broadcast frames
arp.opcode == 1                           # Wireshark: ARP requests
```

---

## Ship It: ARP Library

`code/main.py` provides a reusable `ARPTable` class: add/expire entries with
configurable TTL, simulate ARP request broadcast, resolve IP → MAC.

---

## Exercises

### Level 1 — Frame Anatomy

Minimum Ethernet frame = 64 bytes. Header + FCS = 18 bytes, minimum payload =
46 bytes. Explain why a frame with 10 bytes of data must be padded.

### Level 2 — ARP Trace

Host A (IP `192.168.1.10`, MAC `00:11:22:33:44:55`) sends to Host C
(`192.168.1.20`) for the first time. Trace every ARP and Ethernet frame exchanged
with source/destination MACs and ARP opcodes.

### Level 3 — ARP Spoof Simulation

Extend `code/main.py`: implement `arp_spoof(table, ip, fake_mac)` to poison
the cache, then `detect_spoof(table)` to flag rapid MAC changes.

---

## Summary

- **Ethernet frames** carry data on LANs using 48-bit MAC addresses.
- **ARP** maps IP → MAC via broadcast request / unicast reply.
- **Switches** learn MAC→port mappings and forward frames intelligently.
- ARP has no authentication → vulnerable to **spoofing**.
