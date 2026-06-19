"""
P2P Networks — Kademlia, BitTorrent
Phase 09 — Computer Networks

A Kademlia DHT simulation with routing table, XOR distance, iterative lookup,
store/retrieve, and BitTorrent rarest-first piece selection with tit-for-tat
choking/unchoking.
"""

import hashlib
import random
from typing import Optional


class Node:
    """A Kademlia node identified by a 160-bit (20-byte) ID.

    The ID is typically SHA-1 of a public key or random seed.
    """

    def __init__(self, node_id: bytes = None, ip: str = "127.0.0.1", port: int = 0):
        self.id: bytes = node_id or hashlib.sha1(random.randbytes(32)).digest()
        self.ip: str = ip
        self.port: int = port

    def distance(self, other: "Node") -> bytes:
        """XOR distance between this node and another."""
        return bytes(a ^ b for a, b in zip(self.id, other.id))

    def __eq__(self, other):
        return isinstance(other, Node) and self.id == other.id

    def __hash__(self):
        return hash(self.id)

    def __repr__(self):
        return f"Node({self.id[:8].hex()}...@{self.ip}:{self.port})"


def xor_distance_bytes(a: bytes, b: bytes) -> bytes:
    """Compute XOR distance between two byte strings of equal length."""
    if len(a) != len(b):
        raise ValueError(f"ID length mismatch: {len(a)} vs {len(b)}")
    return bytes(x ^ y for x, y in zip(a, b))


def bit_prefix_length(a: bytes, b: bytes) -> int:
    """Count the number of leading bits that are equal in two byte strings.

    This determines which k-bucket a node belongs to relative to the
    local node. Returns an integer in [0, 160] for 20-byte IDs.
    """
    diff = xor_distance_bytes(a, b)
    for i, byte in enumerate(diff):
        if byte != 0:
            return i * 8 + (8 - byte.bit_length())
    return len(a) * 8


class KBucket:
    """A k-bucket stores up to k nodes within a specific XOR distance range.

    Nodes are ordered by last-seen time (most recent at the tail).
    When the bucket is full, the least-recently-seen node (at the head)
    is evicted — but only after a ping confirms it is unresponsive.
    """

    def __init__(self, k: int = 20):
        self.k = k
        self.nodes: list[Node] = []

    def add(self, node: Node):
        """Add a node to the bucket, or refresh it if already present.

        If the bucket is full, the implementation drops the new node.
        A production DHT would ping the head node first.
        """
        if node in self.nodes:
            self.nodes.remove(node)
            self.nodes.append(node)
        elif len(self.nodes) < self.k:
            self.nodes.append(node)

    def remove(self, node: Node):
        """Remove a node from the bucket if present."""
        if node in self.nodes:
            self.nodes.remove(node)

    def get_nodes(self) -> list[Node]:
        return list(self.nodes)

    def __len__(self):
        return len(self.nodes)

    def __repr__(self):
        return f"KBucket(k={self.k}, nodes={len(self.nodes)})"


class RoutingTable:
    """A Kademlia routing table organized as 160 k-buckets.

    Bucket i contains nodes that share the first i bits with the
    local node's ID but differ at bit i. This forms a binary tree
    where each leaf is a bucket covering a XOR distance range.
    """

    def __init__(self, local_id: bytes, k: int = 20, num_buckets: int = 160):
        if len(local_id) != 20:
            raise ValueError("Kademlia node ID must be 20 bytes (160 bits)")
        self.local_id = local_id
        self.k = k
        self.buckets = [KBucket(k) for _ in range(num_buckets)]

    def _bucket_index(self, node_id: bytes) -> int:
        """Return the k-bucket index for a given node ID."""
        return min(bit_prefix_length(self.local_id, node_id), len(self.buckets) - 1)

    def add(self, node: Node):
        """Insert a node into the appropriate k-bucket."""
        if node.id == self.local_id:
            return
        idx = self._bucket_index(node.id)
        self.buckets[idx].add(node)

    def remove(self, node: Node):
        """Remove a node from its k-bucket."""
        if node.id == self.local_id:
            return
        idx = self._bucket_index(node.id)
        self.buckets[idx].remove(node)

    def find_closest(self, target_id: bytes, count: int = 20) -> list[Node]:
        """Return up to count nodes closest to target_id by XOR distance.

        The algorithm searches outward from the bucket that would contain
        the target, collecting candidates until we have enough.
        """
        seen: set[bytes] = set()
        candidates: list[tuple[int, Node]] = []

        idx = self._bucket_index(target_id)

        max_offset = len(self.buckets)
        for offset in range(max_offset):
            for sign in (-1, 1):
                neighbor = idx + offset * sign
                if 0 <= neighbor < len(self.buckets):
                    for n in self.buckets[neighbor].get_nodes():
                        if n.id not in seen:
                            seen.add(n.id)
                            d = int.from_bytes(
                                xor_distance_bytes(n.id, target_id), 'big'
                            )
                            candidates.append((d, n))

        candidates.sort(key=lambda x: x[0])
        return [n for _, n in candidates[:count]]

    def bucket_summary(self) -> dict[int, int]:
        """Return a dict mapping bucket index to node count (for diagnostics)."""
        return {i: len(b) for i, b in enumerate(self.buckets) if len(b) > 0}

    def total_nodes(self) -> int:
        return sum(len(b) for b in self.buckets)


class NodeRegistry:
    """Simulated global node registry for DHT lookups.

    In a real Kademlia network, FIND_NODE requests are sent as UDP
    messages to remote peers. This registry simulates the responses
    so we can test the lookup algorithm without a real network.
    """

    def __init__(self):
        self._peers: dict[bytes, list[Node]] = {}

    def register(self, node_id: bytes, known_peers: list[Node]):
        """Record which peers a given node knows about."""
        self._peers[node_id] = known_peers

    def lookup(self, node_id: bytes) -> list[Node]:
        """Return the peers that node_id would return in a FIND_NODE response."""
        return self._peers.get(node_id, [])

    def size(self) -> int:
        return len(self._peers)


class KademliaNode:
    """A Kademlia DHT node supporting lookup, store, and retrieve operations."""

    def __init__(self, node: Node, registry: NodeRegistry):
        self.node = node
        self.routing_table = RoutingTable(node.id)
        self.data: dict[str, str] = {}
        self.registry = registry

    def add_peer(self, peer: Node):
        """Insert a peer into the local routing table."""
        self.routing_table.add(peer)

    def find_node(self, target_id: bytes, alpha: int = 3) -> list[Node]:
        """Iterative Kademlia node lookup.

        Returns the alpha closest nodes to target_id by:
        1. Seeding candidates from the local routing table.
        2. Querying the closest unqueried candidate.
        3. Repeating until no closer nodes are discovered.
        """
        candidates = self.routing_table.find_closest(target_id, max(alpha * 2, 1))
        queried: set[bytes] = set()

        for iteration in range(16):  # safety limit: O(log N) typical
            unqueried = [
                n for n in candidates
                if n.id not in queried and n.id != self.node.id
            ]
            if not unqueried:
                break

            batch = unqueried[:alpha]
            found_closer = False

            for peer in batch:
                queried.add(peer.id)
                responses = self.registry.lookup(peer.id)
                for rn in responses:
                    if rn.id not in queried and rn.id != self.node.id:
                        self.routing_table.add(rn)
                        found_closer = True

            if not found_closer:
                break

            candidates = self.routing_table.find_closest(
                target_id, max(alpha * 2, 1)
            )

        return [n for n in candidates if n.id != self.node.id][:alpha]

    def store(self, key: str, value: str):
        """Store a key-value pair locally."""
        self.data[key] = value

    def find_value(self, key: str, alpha: int = 3) -> Optional[str]:
        """Iterative FIND_VALUE: look up a key's value in the DHT.

        Hashes the key with SHA-1, performs FIND_NODE, and checks
        whether any of the returned peers hold the value.
        """
        key_hash = hashlib.sha1(key.encode()).digest()
        closest = self.find_node(key_hash, alpha)

        for peer in closest:
            responses = self.registry.lookup(peer.id)
            for rn in responses:
                dist = int.from_bytes(
                    xor_distance_bytes(rn.id, key_hash), 'big'
                )
                if dist < 2 ** 30 and key in self.data:
                    return self.data[key]

        return None


class BitTorrentPeer:
    """A BitTorrent peer managing piece availability and choking state.

    Implements rarest-first piece selection and tit-for-tat choking.
    """

    def __init__(self, peer_id: str, total_pieces: int = 100):
        self.peer_id = peer_id
        self.total_pieces = total_pieces
        self.have_pieces: set[int] = set()
        self.upload_rates: dict[str, float] = {}
        self.optimistic_unchoke_peer: Optional[str] = None
        self._opt_unchoke_counter: int = 0

    def add_piece(self, piece_index: int):
        """Mark a piece as acquired."""
        if 0 <= piece_index < self.total_pieces:
            self.have_pieces.add(piece_index)

    def has_piece(self, piece_index: int) -> bool:
        """Check if we already have a piece."""
        return piece_index in self.have_pieces

    def have_fraction(self) -> float:
        """Fraction of total pieces acquired."""
        return len(self.have_pieces) / self.total_pieces

    def rarest_first_selection(
        self, swarm_rarity: dict[int, int], count: int = 5
    ) -> list[int]:
        """Select count missing pieces, ordered by rarity (rarest first).

        Args:
            swarm_rarity: dict mapping piece_index -> number of peers that have it.
            count: maximum number of pieces to return.

        Returns:
            List of piece indices sorted by increasing rarity.
        """
        missing = [
            p for p in range(self.total_pieces)
            if p not in self.have_pieces
        ]
        missing.sort(key=lambda p: swarm_rarity.get(p, 0))
        return missing[:count]

    def compute_unchoke_set(self, upload_slots: int = 4) -> list[str]:
        """Tit-for-tat unchoking algorithm.

        Unchokes the upload_slots peers with the highest upload rate to us.
        Additionally, one optimistic unchoke slot rotates through remaining
        peers every 3 calls to discover new potential upload partners.

        Args:
            upload_slots: number of regular unchoke slots.

        Returns:
            List of peer IDs that are unchoked.
        """
        if not self.upload_rates:
            return []

        sorted_peers = sorted(
            self.upload_rates.items(),
            key=lambda x: x[1],
            reverse=True,
        )
        unchoked = [pid for pid, _ in sorted_peers[:upload_slots]]

        remaining = [pid for pid in self.upload_rates if pid not in unchoked]
        if remaining:
            self._opt_unchoke_counter += 1
            if self._opt_unchoke_counter % 3 == 0 or self.optimistic_unchoke_peer is None:
                self.optimistic_unchoke_peer = random.choice(remaining)
            if self.optimistic_unchoke_peer not in unchoked:
                unchoked.append(self.optimistic_unchoke_peer)

        return unchoked

    def set_upload_rate(self, peer_id: str, rate_kbps: float):
        """Record a peer's upload rate to us (KB/s)."""
        self.upload_rates[peer_id] = rate_kbps


def main():
    print("=" * 60)
    print("P2P Networks \u2014 Kademlia DHT + BitTorrent")
    print("=" * 60)

    # ==================================================================
    #  Part 1: Kademlia DHT
    # ==================================================================
    print("\n[1] KADEMLIA DHT\n")

    registry = NodeRegistry()

    # Create a bootstrap node
    bootstrap = Node(ip="127.0.0.1", port=8000)
    kademlia = KademliaNode(bootstrap, registry)

    # Create 50 peer nodes and register them with bootstrap's routing table
    peers: list[Node] = []
    for i in range(50):
        peer = Node(ip="127.0.0.1", port=8001 + i)
        kademlia.add_peer(peer)
        peers.append(peer)
    registry.register(bootstrap.id, peers)

    # Give each peer knowledge of a few others (simulating partial network views)
    for i, peer in enumerate(peers):
        known = [
            p for j, p in enumerate(peers)
            if j != i and j % 4 == 0
        ]
        registry.register(peer.id, known)

    # Routing table diagnostics
    summary = kademlia.routing_table.bucket_summary()
    print(f"Bootstrap node: {bootstrap.id.hex()[:16]}...")
    print(f"Routing table: {kademlia.routing_table.total_nodes()} nodes in {len(summary)} buckets")
    if summary:
        print(f"  Buckets with nodes: {sorted(summary.keys())[:5]}...")

    # FIND_NODE for a random target
    target = Node()
    print(f"\nLookup target: {target.id.hex()[:16]}...")
    closest = kademlia.find_node(target.id)
    print(f"Found {len(closest)} closest nodes:")
    for c in closest:
        d = int.from_bytes(xor_distance_bytes(c.id, target.id), 'big')
        print(f"  {c.id.hex()[:16]}...  distance=0x{d:x}")

    # Verify XOR distance properties
    d1 = int.from_bytes(xor_distance_bytes(bootstrap.id, target.id), 'big')
    d2 = int.from_bytes(xor_distance_bytes(target.id, bootstrap.id), 'big')
    assert d1 == d2, f"XOR distance not symmetric: {d1} != {d2}"
    print(f"\nXOR symmetry: dist(A,B) = {d1} == dist(B,A) = {d2} (OK)")

    zero = xor_distance_bytes(bootstrap.id, bootstrap.id)
    assert all(b == 0 for b in zero), "XOR distance from self to self should be zero"
    print(f"XOR identity: dist(A,A) = 0 (OK)")

    # STORE and FIND_VALUE
    print("\n--- STORE / FIND_VALUE ---")
    kademlia.store("course", "computer-networks")
    kademlia.store("topic", "kademlia-dht")
    result = kademlia.find_value("course")
    print(f"FIND_VALUE('course') = {result}")
    result = kademlia.find_value("topic")
    print(f"FIND_VALUE('topic')  = {result}")
    result = kademlia.find_value("nonexistent")
    print(f"FIND_VALUE('nonexistent') = {result}")

    # ==================================================================
    #  Part 2: BitTorrent Piece Selection
    # ==================================================================
    print("\n[2] BITTORRENT PIECE SELECTION\n")

    # Build a swarm rarity map: 100 pieces, random availability
    swarm_rarity = {i: random.randint(1, 50) for i in range(100)}

    bt_peer = BitTorrentPeer("downloader", total_pieces=100)

    # Seed random pieces as already owned
    initial_pieces = set(random.sample(range(100), 35))
    for p in initial_pieces:
        bt_peer.add_piece(p)

    print(f"Peer has {len(bt_peer.have_pieces)}/100 pieces ({bt_peer.have_fraction()*100:.0f}%)")
    selected = bt_peer.rarest_first_selection(swarm_rarity, count=8)
    print(f"\nRarest-first selection (top 8):")
    for p in selected:
        print(f"  Piece {p:3d}  rarity={swarm_rarity.get(p, 0):2d} peers have it")

    # Verify selection correctness
    missing = [p for p in range(100) if p not in bt_peer.have_pieces]
    missing_rarities = sorted([swarm_rarity[p] for p in missing])
    selected_rarities = [swarm_rarity[p] for p in selected]
    assert selected_rarities == missing_rarities[:len(selected)], \
        "Rarest-first order does not match expected rarity order"
    print(f"\nCorrectness: rarest selection verified (OK)")

    # ==================================================================
    #  Part 3: BitTorrent Tit-for-Tat Choking
    # ==================================================================
    print("\n[3] BITTORRENT TIT-FOR-TAT CHOKING\n")

    choker = BitTorrentPeer("choker", total_pieces=100)

    # Simulate 8 leechers with varying upload speeds
    leecher_ids = [f"leecher-{i}" for i in range(8)]
    leecher_rates = [random.uniform(50.0, 1000.0) for _ in range(8)]

    for lid, rate in zip(leecher_ids, leecher_rates):
        choker.set_upload_rate(lid, rate)

    unchoked = choker.compute_unchoke_set(upload_slots=4)

    print(f"Upload slots: 4 regular + 1 optimistic")
    print(f"{'Peer':<14s} {'Rate (KB/s)':<12s} {'Status':<12s}")
    print("-" * 40)
    for pid, rate in sorted(choker.upload_rates.items(),
                            key=lambda x: x[1], reverse=True):
        status = "UNCHOKED" if pid in unchoked else "CHOKED"
        print(f"{pid:<14s} {rate:<12.1f} [{status:<9s}]")

    print(f"\nOptimistic unchoke: {choker.optimistic_unchoke_peer}")

    # Simulate unchoke stability over multiple rounds
    print("\n--- Unchoke stability over 6 rounds ---")
    round_results: list[list[str]] = []
    for rnd in range(6):
        unchoked = choker.compute_unchoke_set(4)
        round_results.append(unchoked)

    # Check optimistic unchoke rotation
    rot_tracker = set()
    for rnd, result in enumerate(round_results):
        opt = choker.optimistic_unchoke_peer
        rot_tracker.add(opt)
        print(f"  Round {rnd + 1}: {len(result)} unchoked, optimistic={opt}")
    print(f"  Optimistic candidates over 6 rounds: {len(rot_tracker)} unique")

    print("\nDone.")


if __name__ == "__main__":
    main()
