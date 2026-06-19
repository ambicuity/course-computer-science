# P2P Networks — Kademlia, BitTorrent

> A distributed hash table that finds any node in O(log N) steps and a peer-to-peer file-sharing protocol that rewards contributors.

**Type:** Learn
**Languages:** Rust, Python
**Prerequisites:** Phase 09 lessons 01–18
**Time:** ~75 minutes

## Learning Objectives

- Understand how distributed hash tables (DHTs) replace centralized directories in P2P networks.
- Implement Kademlia's k-bucket routing table, XOR distance metric, and iterative node lookup.
- Implement BitTorrent rarest-first piece selection and tit-for-tat choking/unchoking.
- Compare P2P architectures to client-server in terms of scalability, fault tolerance, and complexity.

## The Problem

You want to share a 4 GB file with a thousand people. A single server costs too much in bandwidth, and it's a single point of failure. If the server goes down, nobody gets the file. Even if it stays up, users on the other side of the world suffer high latency.

Without a P2P network, every byte travels through a central bottleneck. The server's outbound link saturates, downloads slow to a crawl, and the whole system collapses under its own success.

Distributing the file across peers solves the bandwidth problem — each peer that downloads also uploads. But now you have a new problem: how do you find which peers have which pieces of the file without a central directory? A tracker is one approach, but it's still a centralized component. The real solution is a **distributed hash table** (DHT) that maps file hashes to peer addresses across thousands of nodes with no single point of failure.

## The Concept

### Centralized vs Decentralized vs Distributed

| | Centralized | Decentralized | Distributed |
|---|---|---|---|
| Control | One server | Multiple servers (siloed) | Many peers, no single authority |
| Failure | Single point of failure | Partial | No single point of failure |
| Coordination | Server decides | Per-silo decisions | Consensus / DHT protocols |
| Example | Traditional web | Federated services | BitTorrent DHT, IPFS |

P2P systems are **distributed** — every node is both client and server. There is no "the server." The challenge is coordinating without central authority.

### Distributed Hash Tables (DHTs)

A DHT is a key-value store spread across many nodes. Any node can insert or retrieve a value by key, and the protocol guarantees you'll find it in O(log N) network hops. The key space is flat (not hierarchical like DNS), and responsibility for keys is split deterministically.

### Kademlia — Design

Kademlia assigns each node a 160-bit ID (typically SHA-1 of a public key or random seed). **Distance is defined as XOR** of two IDs, interpreted as an integer:

```
distance(A, B) = A xor B
```

XOR distance has three key properties that make it a good metric:
1. **Symmetry**: `dist(A, B) = dist(B, A)` — distance is the same in both directions.
2. **Identity**: `dist(A, A) = 0` — a node is distance zero from itself.
3. **Triangle inequality**: `dist(A, C) <= dist(A, B) + dist(B, C)` — routing converges.

**k-buckets**: Each node maintains a routing table organized as 160 buckets. Bucket `i` contains nodes whose IDs share the first `i` bits with the local node but differ at bit `i`. Each bucket holds up to `k` nodes (typically `k = 20`). When a bucket is full, the least-recently-seen node is evicted (after a ping check).

**Iterative lookup**: To find a node by ID, the local node:
1. Picks `alpha` (typically 3) closest known nodes from its routing table.
2. Sends each a `FIND_NODE` request.
3. Each response returns the `k` closest nodes the remote peer knows.
4. Repeat until no closer nodes are found — usually converges in O(log N) steps.

### BitTorrent — Design

BitTorrent splits a file into pieces (typically 256 KB–4 MB each). A **.torrent** metadata file contains piece hashes, file name, and size. Peers discover each other via:
- **Tracker**: An HTTP server that maintains a list of peers for each torrent.
- **DHT** (Mainline DHT): A Kademlia-based DHT where info-hashes are the keys and peer addresses are the values.

**Piece selection strategies**:
- **Rarest-first**: Download the piece that fewest peers have first. This ensures rare pieces are replicated early, making them available to everyone.
- **Rarest-last** (endgame mode): When only a few pieces are missing, request them from everyone simultaneously.
- **Sequential**: For streaming media, download pieces in order.

**Choking/Unchoking** (tit-for-tat):
- Each peer uploads to the `N` peers (typically 4) that upload fastest to it.
- One additional **optimistic unchoke** slot rotates randomly to discover better partners.
- Peers are choked (not uploaded to) if they don't reciprocate.
- This punishes free-riders and rewards contributors.

## Build It

### Step 1: Kademlia DHT in Python

We implement a Kademlia node with k-bucket routing table, XOR distance, and iterative node lookup.

```python
import hashlib
import random
import struct
from typing import Optional


class Node:
    """A Kademlia node identified by a 160-bit ID."""
    def __init__(self, node_id: bytes = None, ip="127.0.0.1", port=0):
        self.id = node_id or hashlib.sha1(random.randbytes(32)).digest()
        self.ip = ip
        self.port = port

    def distance(self, other: "Node") -> bytes:
        """XOR distance to another node."""
        return bytes(a ^ b for a, b in zip(self.id, other.id))

    def __eq__(self, other):
        return isinstance(other, Node) and self.id == other.id

    def __hash__(self):
        return hash(self.id)

    def __repr__(self):
        return f"Node({self.id[:8].hex()}...@{self.ip}:{self.port})"


def xor_distance_bytes(a: bytes, b: bytes) -> bytes:
    """Compute XOR distance between two byte strings."""
    return bytes(x ^ y for x, y in zip(a, b))


def bit_prefix_length(a: bytes, b: bytes) -> int:
    """Count leading bits where a and b agree."""
    diff = xor_distance_bytes(a, b)
    for i, byte in enumerate(diff):
        if byte != 0:
            return i * 8 + byte.bit_length() - 1
    return len(a) * 8


class KBucket:
    """A k-bucket stores up to k nodes within a specific XOR distance range.

    Nodes are ordered by last-seen time (most recently seen at the tail).
    When full, the least-recently-seen node is evicted after a ping check.
    """
    def __init__(self, k: int = 20):
        self.k = k
        self.nodes: list[Node] = []

    def add(self, node: Node):
        """Add or refresh a node. Move to tail if already present."""
        if node in self.nodes:
            self.nodes.remove(node)
            self.nodes.append(node)
        elif len(self.nodes) < self.k:
            self.nodes.append(node)
        else:
            # In production: ping the head (least-recently-seen).
            # If it responds, move it to tail and discard new node.
            pass

    def remove(self, node: Node):
        if node in self.nodes:
            self.nodes.remove(node)

    def get_nodes(self) -> list[Node]:
        return list(self.nodes)

    def __len__(self):
        return len(self.nodes)

    def __repr__(self):
        return f"KBucket(k={self.k}, nodes={len(self.nodes)})"


class RoutingTable:
    """Kademlia routing table: binary tree with k-buckets per bit prefix.

    For a 160-bit address space, bucket i contains nodes whose IDs
    share the first i bits with the local node but differ at bit i.
    """
    def __init__(self, local_id: bytes, k: int = 20, num_buckets: int = 160):
        self.local_id = local_id
        self.k = k
        self.buckets = [KBucket(k) for _ in range(num_buckets)]

    def _bucket_index(self, node_id: bytes) -> int:
        """Return the k-bucket index appropriate for node_id."""
        return bit_prefix_length(self.local_id, node_id)

    def add(self, node: Node):
        if node.id == self.local_id:
            return
        idx = self._bucket_index(node.id)
        idx = min(idx, len(self.buckets) - 1)
        self.buckets[idx].add(node)

    def remove(self, node: Node):
        if node.id == self.local_id:
            return
        idx = self._bucket_index(node.id)
        idx = min(idx, len(self.buckets) - 1)
        self.buckets[idx].remove(node)

    def find_closest(self, target_id: bytes, count: int = 20) -> list[Node]:
        """Return up to 'count' nodes closest to target_id.

        Searches outward from the bucket that would contain the target.
        """
        seen: set[bytes] = set()
        candidates: list[tuple[int, Node]] = []

        idx = self._bucket_index(target_id)
        idx = min(idx, len(self.buckets) - 1)

        # Spread outward from the target bucket
        max_offset = max(len(self.buckets), 1)
        for offset in range(max_offset):
            for sign in (-1, 1):
                neighbor = idx + offset * sign
                if 0 <= neighbor < len(self.buckets):
                    bucket = self.buckets[neighbor]
                    for n in bucket.get_nodes():
                        if n.id not in seen:
                            seen.add(n.id)
                            d = int.from_bytes(
                                xor_distance_bytes(n.id, target_id), 'big'
                            )
                            candidates.append((d, n))

        candidates.sort(key=lambda x: x[0])
        return [n for _, n in candidates[:count]]


class NodeRegistry:
    """Global registry for simulation — maps node IDs to their known peers.

    In a real DHT, FIND_NODE requests are sent as UDP messages. Here we
    use a shared registry so each simulated node can respond to lookups
    with the peers it would know in a real network.
    """
    def __init__(self):
        self._peers: dict[bytes, list[Node]] = {}

    def register(self, node_id: bytes, peers: list[Node]):
        self._peers[node_id] = peers

    def lookup(self, node_id: bytes) -> list[Node]:
        return self._peers.get(node_id, [])


class KademliaNode:
    """A Kademlia DHT node supporting FIND_NODE and FIND_VALUE operations."""
    def __init__(self, node: Node, registry: NodeRegistry):
        self.node = node
        self.routing_table = RoutingTable(node.id)
        self.data: dict[str, str] = {}
        self.registry = registry

    def add_peer(self, peer: Node):
        self.routing_table.add(peer)

    def find_node(self, target_id: bytes, alpha: int = 3) -> list[Node]:
        """Iterative Kademlia lookup: returns the alpha closest nodes.

        1. Seed candidates from local routing table.
        2. Query unqueried candidates for closer nodes.
        3. Repeat until no closer nodes found.
        """
        candidates = self.routing_table.find_closest(target_id, alpha * 2)
        queried: set[bytes] = set()
        changed = True

        while changed:
            changed = False
            to_query = [n for n in candidates if n.id not in queried][:alpha]
            for peer in to_query:
                queried.add(peer.id)
                if peer.id == self.node.id:
                    continue
                results = self.registry.lookup(peer.id)
                for rn in results:
                    if rn.id not in queried and rn.id != self.node.id:
                        self.routing_table.add(rn)
                        changed = True

            new_candidates = self.routing_table.find_closest(target_id, alpha * 2)
            # Check if we found closer nodes
            current_best = candidates[0].id if candidates else None
            new_best = new_candidates[0].id if new_candidates else None
            if new_best != current_best:
                candidates = new_candidates
            else:
                break

        return [n for n in candidates if n.id != self.node.id][:alpha]

    def store(self, key: str, value: str):
        """Store a key-value pair locally."""
        self.data[key] = value

    def find_value(self, key: str) -> Optional[str]:
        """Iterative FIND_VALUE: looks for a key's value in the DHT."""
        key_hash = hashlib.sha1(key.encode()).digest()
        closest = self.find_node(key_hash)
        for peer in closest:
            # In a real DHT we'd send FIND_VALUE to each peer.
            # Here we simulate by checking a local response table.
            response = self.registry.lookup(peer.id)
            for rn in response:
                dist = int.from_bytes(
                    xor_distance_bytes(rn.id, key_hash), 'big'
                )
                if dist < 2 ** 30 and key in self.data:
                    return self.data[key]
        return None


class BitTorrentPeer:
    """A BitTorrent peer managing piece availability, selection, and choking."""
    def __init__(self, peer_id: str, total_pieces: int = 100):
        self.peer_id = peer_id
        self.total_pieces = total_pieces
        self.have_pieces: set[int] = set()
        self.upload_rates: dict[str, float] = {}
        self.optimistic_unchoke_peer: Optional[str] = None
        self._opt_unchoke_counter = 0

    def have(self, piece_index: int) -> bool:
        return piece_index in self.have_pieces

    def add_piece(self, piece_index: int):
        if 0 <= piece_index < self.total_pieces:
            self.have_pieces.add(piece_index)

    def rarest_first_selection(self, swarm_rarity: dict[int, int], count: int = 5) -> list[int]:
        """Select 'count' pieces not yet downloaded, ordered by rarity.

        Rarity = how many peers have the piece. Lower number = rarer.
        """
        missing = [
            p for p in range(self.total_pieces) if p not in self.have_pieces
        ]
        missing.sort(key=lambda p: swarm_rarity.get(p, 0))
        return missing[:count]

    def compute_unchoke_set(self, upload_slots: int = 4) -> list[str]:
        """Tit-for-tat unchoking: unchoke peers with highest upload rate.

        One optimistic unchoke slot is rotated randomly to discover
        peers that might become good upload partners.
        """
        sorted_peers = sorted(
            self.upload_rates.items(),
            key=lambda x: x[1],
            reverse=True
        )
        unchoked = [pid for pid, _ in sorted_peers[:upload_slots]]

        # Optimistic unchoke: rotate through remaining peers
        remaining = [pid for pid in self.upload_rates if pid not in unchoked]
        if remaining:
            self._opt_unchoke_counter += 1
            if self._opt_unchoke_counter % 3 == 0 or self.optimistic_unchoke_peer is None:
                self.optimistic_unchoke_peer = random.choice(remaining)
            if self.optimistic_unchoke_peer not in unchoked:
                unchoked.append(self.optimistic_unchoke_peer)

        return unchoked

    def update_upload_rate(self, peer_id: str, rate: float):
        self.upload_rates[peer_id] = rate


def main():
    print("=" * 60)
    print("P2P Networks — Kademlia DHT + BitTorrent")
    print("=" * 60)

    # ------------------------------------------------------------------ #
    #  Part 1: Kademlia DHT                                              #
    # ------------------------------------------------------------------ #
    print("\n[1] Kademlia DHT Simulation\n")

    registry = NodeRegistry()

    # Bootstrap node
    bootstrap = Node(ip="127.0.0.1", port=8000)
    knode = KademliaNode(bootstrap, registry)

    # Create 50 nodes and add them to the bootstrap's routing table
    peers_added = []
    for i in range(50):
        peer = Node(ip="127.0.0.1", port=8001 + i)
        knode.add_peer(peer)
        peers_added.append(peer)
    registry.register(bootstrap.id, peers_added)

    # Also register some peers as knowing about others (simulating
    # a partially populated DHT)
    for i, peer in enumerate(peers_added):
        known = [
            p for j, p in enumerate(peers_added)
            if j != i and j % 3 == 0
        ]
        registry.register(peer.id, known)

    # Perform a FIND_NODE for a random target
    target = Node()
    print(f"Local node ID: {bootstrap.id.hex()[:16]}...")
    print(f"Target node ID: {target.id.hex()[:16]}...")
    closest = knode.find_node(target.id)
    print(f"Found {len(closest)} closest nodes:")
    for c in closest:
        dist = int.from_bytes(xor_distance_bytes(c.id, target.id), 'big')
        print(f"  {c.id.hex()[:16]}...  distance=0x{dist:x}")

    # Test XOR distance property: symmetry
    d1 = int.from_bytes(xor_distance_bytes(bootstrap.id, target.id), 'big')
    d2 = int.from_bytes(xor_distance_bytes(target.id, bootstrap.id), 'big')
    assert d1 == d2, "XOR distance is not symmetric!"
    print(f"\nXOR symmetry verified: dist(A,B) = {d1} = dist(B,A)")

    # Store and retrieve a value
    knode.store("greeting", "hello_from_kademlia")
    result = knode.find_value("greeting")
    print(f"Stored value for 'greeting': {result}")

    # ------------------------------------------------------------------ #
    #  Part 2: BitTorrent Piece Selection                                #
    # ------------------------------------------------------------------ #
    print("\n[2] BitTorrent Piece Selection\n")

    # Simulate a swarm: 100 pieces, each with a random rarity count
    swarm_rarity = {i: random.randint(1, 50) for i in range(100)}

    peer_a = BitTorrentPeer("peer-a", total_pieces=100)
    # Seed 30 random pieces
    peer_a.have_pieces = set(random.sample(range(100), 30))

    selected = peer_a.rarest_first_selection(swarm_rarity, count=10)
    print(f"Peer has {len(peer_a.have_pieces)}/100 pieces")
    print(f"Rarest pieces selected (rarity score in parens):")
    for p in selected:
        print(f"  Piece {p:3d} (rarity={swarm_rarity.get(p, 0):2d})")

    # Verify that selected pieces are indeed the rarest among missing ones
    missing_rarities = [
        swarm_rarity[p] for p in range(100) if p not in peer_a.have_pieces
    ]
    missing_rarities.sort()
    selected_rarities = [swarm_rarity[p] for p in selected]
    assert selected_rarities == missing_rarities[:len(selected)], \
        "Rarest-first selection failed!"
    print("Rarest-first correctness verified.")

    # ------------------------------------------------------------------ #
    #  Part 3: BitTorrent Choking (Tit-for-tat)                          #
    # ------------------------------------------------------------------ #
    print("\n[3] BitTorrent Tit-for-Tat Choking\n")

    choke_peer = BitTorrentPeer("choker", total_pieces=100)
    # Simulate 8 leechers with varying upload rates
    leechers = [f"leecher-{i}" for i in range(8)]
    for l_id in leechers:
        rate = random.uniform(50, 1000)
        choke_peer.update_upload_rate(l_id, rate)

    unchanged = choke_peer.compute_unchoke_set(upload_slots=4)
    print(f"Upload rates:")
    for pid, rate in sorted(choke_peer.upload_rates.items(),
                            key=lambda x: x[1], reverse=True):
        tag = "UNCHOKED" if pid in unchanged else "CHOKED  "
        print(f"  {pid:12s}: {rate:7.1f} KB/s  [{tag}]")

    print(f"\nOptimistic unchoke peer: {choke_peer.optimistic_unchoke_peer}")

    print("\nDone.")


if __name__ == "__main__":
    main()
```

This produces:
- DHT node lookup that converges in ~3 queries
- XOR distance verification (symmetry property)
- Rarest-first piece selection with correctness check
- Tit-for-tat unchoking with 4 regular slots + 1 optimistic slot

### Step 2: Kademlia Routing in Rust

The Rust implementation focuses on the routing table: k-buckets, XOR distance, and closest-node queries.

```rust
/// A Kademlia node identified by a 160-bit ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Node {
    id: [u8; 20],
    ip: [u8; 4],
    port: u16,
}

impl Node {
    fn new(id: [u8; 20], ip: [u8; 4], port: u16) -> Self {
        Node { id, ip, port }
    }

    /// XOR distance between two node IDs.
    fn xor_distance(&self, other: &Node) -> [u8; 20] {
        let mut dist = [0u8; 20];
        for i in 0..20 {
            dist[i] = self.id[i] ^ other.id[i];
        }
        dist
    }
}

/// A k-bucket holds up to k nodes sorted by last-seen time.
#[derive(Debug, Clone)]
struct KBucket<const K: usize = 20> {
    nodes: Vec<Node>,
}

impl<const K: usize> KBucket<K> {
    fn new() -> Self {
        KBucket { nodes: Vec::with_capacity(K) }
    }

    fn add(&mut self, node: Node) {
        if let Some(pos) = self.nodes.iter().position(|n| n.id == node.id) {
            self.nodes.remove(pos);
            self.nodes.push(node);
        } else if self.nodes.len() < K {
            self.nodes.push(node);
        }
        // Full bucket: in production we'd ping the head node.
        // If it responds, move it to tail and discard new node.
    }

    fn remove(&mut self, node: &Node) {
        self.nodes.retain(|n| n.id != node.id);
    }

    fn iter(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter()
    }

    fn len(&self) -> usize {
        self.nodes.len()
    }
}

/// A Kademlia routing table using 160 k-buckets.
struct RoutingTable<const K: usize = 20> {
    local_id: [u8; 20],
    buckets: [KBucket<K>; 160],
}

/// Count leading zero bytes, then leading zero bits in the first non-zero byte.
fn leading_zero_bits(a: &[u8; 20]) -> usize {
    for (i, &byte) in a.iter().enumerate() {
        if byte != 0 {
            return i * 8 + (7 - (byte.leading_zeros() as usize));
        }
    }
    160
}

impl<const K: usize> RoutingTable<K> {
    fn new(local_id: [u8; 20]) -> Self {
        let buckets = [(); 160].map(|_| KBucket::new());
        RoutingTable { local_id, buckets }
    }

    fn bucket_index(&self, node_id: &[u8; 20]) -> usize {
        let mut diff = [0u8; 20];
        for i in 0..20 {
            diff[i] = self.local_id[i] ^ node_id[i];
        }
        let bits = leading_zero_bits(&diff);
        bits.min(159)
    }

    fn add(&mut self, node: Node) {
        if node.id == self.local_id {
            return;
        }
        let idx = self.bucket_index(&node.id);
        self.buckets[idx].add(node);
    }

    fn remove(&mut self, node: &Node) {
        let idx = self.bucket_index(&node.id);
        self.buckets[idx].remove(node);
    }

    /// Find the `count` closest known nodes to `target_id`.
    fn find_closest(&self, target_id: &[u8; 20], count: usize) -> Vec<Node> {
        use std::collections::HashSet;

        let idx = self.bucket_index(target_id).min(159);
        let mut candidates: Vec<(u128, Node)> = Vec::new();
        let mut seen: HashSet<[u8; 20]> = HashSet::new();

        // Spread outward from the target bucket
        let max_offset = 160;
        for offset in 0..max_offset {
            for &sign in &[-1isize, 1isize] {
                let neighbor = idx as isize + offset as isize * sign;
                if neighbor < 0 || neighbor >= 160 {
                    continue;
                }
                let bucket = &self.buckets[neighbor as usize];
                'nodes: for node in bucket.iter() {
                    if !seen.insert(node.id) {
                        continue 'nodes;
                    }
                    let mut d = [0u8; 20];
                    for i in 0..20 {
                        d[i] = node.id[i] ^ target_id[i];
                    }
                    let dist = u128::from_be_bytes([
                        d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7],
                        d[8], d[9], d[10], d[11], d[12], d[13], d[14], d[15],
                    ]);
                    candidates.push((dist, node.clone()));
                }
            }
        }

        candidates.sort_by_key(|(dist, _)| *dist);
        candidates.truncate(count);
        candidates.into_iter().map(|(_, n)| n).collect()
    }

    fn bucket_count(&self) -> Vec<usize> {
        self.buckets.iter().map(|b| b.len()).collect()
    }
}

/// BitTorrent peer with rarest-first piece selection.
struct BitTorrentPeer {
    peer_id: String,
    total_pieces: usize,
    have_pieces: Vec<bool>,
    upload_rates: Vec<(String, f64)>,
}

impl BitTorrentPeer {
    fn new(peer_id: &str, total_pieces: usize) -> Self {
        BitTorrentPeer {
            peer_id: peer_id.to_string(),
            total_pieces,
            have_pieces: vec![false; total_pieces],
            upload_rates: Vec::new(),
        }
    }

    fn have(&mut self, piece: usize) {
        if piece < self.total_pieces {
            self.have_pieces[piece] = true;
        }
    }

    /// Return pieces sorted by rarity (fewest copies first) that we don't have.
    fn rarest_first(&self, swarm_rarity: &[usize]) -> Vec<usize> {
        let mut missing: Vec<(usize, usize)> = self.have_pieces
            .iter()
            .enumerate()
            .filter(|(_, &have)| !have)
            .map(|(i, _)| (swarm_rarity[i], i))
            .collect();
        missing.sort_by_key(|&(rarity, _)| rarity);
        missing.into_iter().map(|(_, i)| i).collect()
    }

    /// Tit-for-tat unchoking: select `slots` peers with highest upload rates.
    fn compute_unchoke_set(&self, slots: usize) -> Vec<String> {
        let mut sorted: Vec<(String, f64)> = self.upload_rates.clone();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        sorted.into_iter().take(slots).map(|(id, _)| id).collect()
    }

    fn set_upload_rate(&mut self, peer_id: &str, rate: f64) {
        if let Some(pos) = self.upload_rates.iter().position(|(id, _)| id == peer_id) {
            self.upload_rates[pos].1 = rate;
        } else {
            self.upload_rates.push((peer_id.to_string(), rate));
        }
    }
}

fn main() {
    println!("========================================");
    println!("P2P Networks — Kademlia + BitTorrent");
    println!("========================================");

    // Part 1: Kademlia Routing Table
    println!("\n--- Kademlia Routing Table ---\n");

    // Create a local node with a hardcoded 160-bit ID
    let local_id: [u8; 20] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        0x00, 0x11, 0x22, 0x33,
    ];
    let local = Node::new(local_id, [10, 0, 0, 1], 8000);
    let mut table = RoutingTable::<20>::new(local.id);

    // Add 60 random nodes to the routing table
    let ip_base = [10, 0, 0, 0];
    for i in 0..60 {
        let mut nid = [0u8; 20];
        nid[0] = (i * 37) as u8;
        nid[1] = (i * 73) as u8;
        nid[2] = (i * 151) as u8;
        nid[19] = i as u8;
        let node = Node::new(nid, ip_base, 9000 + i as u16);
        table.add(node);
    }
    println!("Added 60 nodes to routing table.");

    // Show bucket occupancy
    let counts = table.bucket_count();
    let occupied: Vec<usize> = counts.into_iter()
        .enumerate()
        .filter(|(_, c)| *c > 0)
        .map(|(i, c)| { println!("  Bucket {:3}: {} nodes", i, c); i })
        .collect();
    println!("Occupied buckets: {}", occupied.len());

    // FIND_CLOSEST with a target ID
    let target_id: [u8; 20] = [
        0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x88,
        0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0x00,
        0xff, 0xee, 0xdd, 0xcc,
    ];
    let target = Node::new(target_id, [10, 0, 0, 2], 8001);

    let closest = table.find_closest(&target.id, 5);
    println!("\nClosest 5 nodes to target:");
    for (i, n) in closest.iter().enumerate() {
        let dist = local.xor_distance(n);
        println!("  {}. id={:02x}{:02x}{:02x}{:02x}...  distance=0x{:02x}{:02x}{:02x}{:02x}...",
            i + 1, n.id[0], n.id[1], n.id[2], n.id[3],
            dist[0], dist[1], dist[2], dist[3]);
    }

    // Verify XOR distance symmetry
    let d1 = local.xor_distance(&target);
    let d2 = target.xor_distance(&local);
    assert_eq!(d1, d2, "XOR distance must be symmetric");
    println!("\nXOR symmetry: verified.");

    // Part 2: BitTorrent Piece Selection
    println!("\n--- BitTorrent Piece Selection ---\n");

    let mut swarm = vec![0usize; 100];
    for i in 0..100 {
        swarm[i] = ((i * 7 + 13) % 50) + 1;
    }

    let mut peer = BitTorrentPeer::new("peer-a", 100);
    // Seed 30 random pieces
    let seeded: [usize; 30] = [
        3, 7, 12, 15, 18, 22, 27, 31, 34, 38,
        42, 45, 49, 51, 55, 58, 62, 66, 70, 73,
        77, 80, 84, 87, 89, 91, 94, 96, 98, 99,
    ];
    for &p in &seeded {
        peer.have(p);
    }

    let rarest_order = peer.rarest_first(&swarm);
    println!("Rarest pieces (top 10):");
    for &p in rarest_order.iter().take(10) {
        println!("  Piece {:>3} (rarity={})", p, swarm[p]);
    }

    // Tit-for-tat choking simulation
    println!("\n--- Tit-for-Tat Choking ---\n");
    // 3 fast peers, 2 slow peers
    peer.set_upload_rate("fast-a", 950.0);
    peer.set_upload_rate("fast-b", 880.0);
    peer.set_upload_rate("fast-c", 720.0);
    peer.set_upload_rate("slow-a", 120.0);
    peer.set_upload_rate("slow-b", 85.0);

    let unchoked = peer.compute_unchoke_set(3);
    println!("Upload slots = 3");
    for (id, rate) in &peer.upload_rates {
        let status = if unchoked.contains(id) { "UNCHOKED" } else { "CHOKED" };
        println!("  {:10s}: {:6.1} KB/s  [{}]", id, rate, status);
    }

    println!("\nDone.");
}
```

The key implementation difference: Rust's version uses arrays (`[u8; 20]`) instead of Python's `bytes`, const generics for `k-bucket` sizing, and manual XOR loops. The `find_closest` method in both languages follows the same algorithm — spread outward from the target's bucket — giving identical lookup behavior.

## Use It

Production P2P networks that use Kademlia or related DHTs:

- **Mainline DHT** — The BitTorrent DHT, the largest deployed Kademlia network with millions of concurrent nodes. Every modern BitTorrent client includes a Kademlia node (uTorrent, qBittorrent, Transmission, libtorrent). Your implementation covers the core routing table; Mainline adds UDP wire protocol, bootstrapping via hardcoded nodes, and token-based storage.

- **IPFS** (InterPlanetary File System) — Uses a Kademlia-inspired DHT (`go-libp2p-kad-dht`) but extends it with provider records (who has a given CID), WAN/LAN mode separation, and adaptive timeouts. Their DHT is layered with a BitSwap protocol (similar to BitTorrent piece exchange) for actual data transfer.

- **libtorrent** — The reference BitTorrent implementation. Its Kademlia node (`src/kademlia/node.cpp`) handles concurrent UDP RPC, recursive lookups with parallel alpha, and bucket splitting/merging as the routing table grows. The rarest-first implementation in `src/piece_picker.cpp` is the production version of our simple selection.

- **Epidemic/gossip protocols** — Cassandra, DynamoDB, and Redis Cluster use gossip for membership and failure detection rather than DHT routing. Each node periodically exchanges state with a random peer, and the information spreads exponentially. Unlike Kademlia's deterministic XOR routing, gossip is probabilistic but simpler.

## Read the Source

- `libtorrent/src/kademlia/node.cpp` — Production Kademlia DHT node with concurrent lookup, routing table maintenance, and UDP RPC handling.
- `libtorrent/src/piece_picker.cpp` — BitTorrent rarest-first, endgame mode, and sequential piece selection in a real client.
- `go-libp2p-kad-dht/dht.go` — IPFS's Kademlia DHT implementation, including provider records and WAN/LAN splitting.
- `BEP 5` — DHT Protocol: the BitTorrent Enhancement Proposal specifying how Kademlia is used for trackerless torrents.
- `BEP 3` — The BitTorrent Protocol Specification: peer wire protocol, choking, piece messages.

## Ship It

The reusable artifact is a **P2P network simulator** that demonstrates Kademlia DHT routing and BitTorrent piece selection. Files in `outputs/`:

- `dht_sim.py` — Standalone Kademlia DHT node with routing table, iterative lookup, and store/retrieve operations.
- `bittorrent_sim.py` — BitTorrent piece management with rarest-first selection and tit-for-tat unchoking.

These can be reused as reference implementations when studying distributed systems (Phase 14) or building P2P capstone projects.

## Exercises

1. **Easy** — Implement the XOR distance function for two 160-bit node IDs and verify: (a) distance is symmetric, (b) `dist(A, A) = 0`, (c) `dist(A, C) <= dist(A, B) + dist(B, C)` for three sample IDs.

2. **Medium** — Extend the Kademlia implementation with a `STORE` operation that sends key-value pairs to the `k` closest nodes. Implement `FIND_VALUE`: lookup the key's hash, query each returned node for the value, and return on first hit.

3. **Hard** — Implement the full BitTorrent tit-for-tat unchoking algorithm as described in BEP 3: each peer uploads to the 4 peers that upload fastest to it, re-evaluates every 10 seconds, and optimistically unchokes a random peer (rotated every 30 seconds). Simulate 20 peers with varying upload/download ratios and measure download completion times for free-riders vs. contributors.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| DHT | "A distributed key-value store" | A hash table partitioned across many nodes; any node can route a key lookup to the responsible node in O(log N) hops |
| Kademlia | "A P2P routing protocol" | A DHT that uses XOR as a distance metric, k-buckets for routing tables, and iterative parallel lookups for fault tolerance |
| k-bucket | "A bucket of k nodes" | A routing table entry containing up to k nodes whose IDs share a common prefix of bits with the local node; full buckets evict unresponsive peers |
| XOR metric | "XOR distance" | `dist(A, B) = A xor B` interpreted as an integer; it is symmetric, satisfies the triangle inequality, and makes routing converge in O(log N) |
| BitTorrent | "A P2P file-sharing protocol" | A protocol that splits files into pieces, uses a tracker or DHT for peer discovery, and incentivizes uploads via tit-for-tat choking |
| Rarest-first | "Download rarest pieces first" | A piece selection policy that prioritizes pieces held by the fewest peers, ensuring rare pieces are replicated before they disappear |
| Choking/Unchoking | "Tit-for-tat" | A rate-limitting mechanism where each peer uploads to the N peers that upload fastest to it, plus one randomly rotated optimistic unchoke |
| Epidemic broadcast | "Gossip protocol" | A broadcast mechanism where each node forwards a message to a random subset of neighbors; information spreads exponentially without centralized coordination |

## Further Reading

- [Kademlia: A Peer-to-Peer Information System Based on the XOR Metric (Maymounkov & Mazières, 2002)](https://dl.acm.org/doi/10.1007/3-540-45748-8_5) — The original paper describing Kademlia's design, analysis, and proofs.
- [BEP 3: The BitTorrent Protocol Specification](https://www.bittorrent.org/beps/bep_0003.html) — The wire protocol, handshake, piece messages, and choking algorithm.
- [BEP 5: DHT Protocol](https://www.bittorrent.org/beps/bep_0005.html) — How BitTorrent uses Kademlia for trackerless torrents (Mainline DHT).
- [Dynamo: Amazon's Highly Available Key-value Store](https://www.allthingsdistributed.com/files/amazon-dynamo-sosp2007.pdf) — Uses DHT-like partitioning with consistent hashing, gossip-based membership, and vector clocks.
- [Chord: A Scalable Peer-to-peer Lookup Service for Internet Applications](https://pdos.csail.mit.edu/papers/chord:sigcomm01/chord_sigcomm.pdf) — An alternative DHT using a circular ID space; compare with Kademlia's XOR metric.
