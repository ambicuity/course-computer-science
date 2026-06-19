# P2P Network Simulator

Reusable artifact from Lesson 19 — P2P Networks: Kademlia, BitTorrent.

## What it is

A **P2P network simulator** demonstrating:

- **Kademlia DHT** — Routing table with k-buckets, XOR distance metric, iterative node lookup, and key-value storage/retrieval.
- **BitTorrent piece management** — Rarest-first piece selection and tit-for-tat choking/unchoking.

## Files

| File | Language | What it demonstrates |
|------|----------|---------------------|
| `../code/main.py` | Python | Full Kademlia DHT node (Node, KBucket, RoutingTable, KademliaNode) + BitTorrentPeer with rarest-first and choking |
| `../code/main.rs` | Rust | Kademlia routing table with find_closest, XOR distance verification, BitTorrent rarest-first and tit-for-tat |

## Running

```bash
# Python
cd ../code && python main.py

# Rust
cd ../code && rustc main.rs && ./main
```

## Reuse

This artifact is referenced in:
- **Phase 14 (Distributed Systems)** — DHT concepts reappear in consistent hashing and gossip protocols.
- **Phase 19 Capstone Projects** — Build a P2P file-sharing application using the Kademlia DHT and BitTorrent-style piece exchange.
