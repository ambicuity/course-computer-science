//! P2P Networks — Kademlia, BitTorrent
//! Phase 09 — Computer Networks
//!
//! A Kademlia DHT routing table with 160-bit node IDs, XOR distance
//! computation, k-bucket storage, and closest-node lookup.
//! Includes BitTorrent rarest-first piece selection and tit-for-tat
//! choking.
//!
//! Compile with: rustc main.rs && ./main

use std::collections::HashSet;

/// ------------------------------------------------------------------
///  Kademlia Core
/// ------------------------------------------------------------------

/// A Kademlia node identified by a 160-bit (20-byte) ID.
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

    /// XOR distance between this node and another: id_a ^ id_b.
    fn xor_distance(&self, other: &Node) -> [u8; 20] {
        let mut dist = [0u8; 20];
        for i in 0..20 {
            dist[i] = self.id[i] ^ other.id[i];
        }
        dist
    }
}

/// Count leading zero bits in a 160-bit array.
/// Returns a value in [0, 160] indicating how many leading bits are zero.
fn leading_zero_bits(a: &[u8; 20]) -> usize {
    for (i, &byte) in a.iter().enumerate() {
        if byte != 0 {
            return i * 8 + (byte.leading_zeros() as usize);
        }
    }
    160
}

/// XOR distance as a u128 (using the first 16 bytes).
fn xor_distance_u128(a: &[u8; 20], b: &[u8; 20]) -> u128 {
    let mut d = [0u8; 16];
    for i in 0..16 {
        d[i] = a[i] ^ b[i];
    }
    u128::from_be_bytes(d)
}

/// A k-bucket holds up to K nodes sorted by last-seen recency.
#[derive(Debug, Clone)]
struct KBucket<const K: usize = 20> {
    nodes: Vec<Node>,
}

impl<const K: usize> KBucket<K> {
    fn new() -> Self {
        KBucket {
            nodes: Vec::with_capacity(K),
        }
    }

    /// Add a node. Moves existing node to tail (most recent).
    /// If full, the new node is dropped (production DHTs would
    /// ping the head node first to check liveness).
    fn add(&mut self, node: Node) {
        if let Some(pos) = self.nodes.iter().position(|n| n.id == node.id) {
            self.nodes.remove(pos);
            self.nodes.push(node);
        } else if self.nodes.len() < K {
            self.nodes.push(node);
        }
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
///
/// Bucket `i` covers nodes whose IDs share the first `i` bits with
/// the local ID but differ at bit `i`. This forms a binary tree
/// structure over the 160-bit address space.
struct RoutingTable<const K: usize = 20> {
    local_id: [u8; 20],
    buckets: [KBucket<K>; 160],
}

impl<const K: usize> RoutingTable<K> {
    fn new(local_id: [u8; 20]) -> Self {
        let buckets = [(); 160].map(|_| KBucket::new());
        RoutingTable { local_id, buckets }
    }

    /// Determine which k-bucket a node ID belongs to.
    fn bucket_index(&self, node_id: &[u8; 20]) -> usize {
        let mut diff = [0u8; 20];
        for i in 0..20 {
            diff[i] = self.local_id[i] ^ node_id[i];
        }
        leading_zero_bits(&diff).min(159)
    }

    /// Insert a node into the appropriate bucket.
    fn add(&mut self, node: Node) {
        if node.id == self.local_id {
            return;
        }
        let idx = self.bucket_index(&node.id);
        self.buckets[idx].add(node);
    }

    /// Remove a node from its bucket.
    fn remove(&mut self, node: &Node) {
        let idx = self.bucket_index(&node.id);
        self.buckets[idx].remove(node);
    }

    /// Return the `count` closest known nodes to `target_id`.
    ///
    /// Searches outward from the bucket that would contain the target,
    /// collecting nodes and sorting by XOR distance.
    fn find_closest(&self, target_id: &[u8; 20], count: usize) -> Vec<Node> {
        let idx = self.bucket_index(target_id).min(159);
        let mut candidates: Vec<(u128, Node)> = Vec::new();
        let mut seen: HashSet<[u8; 20]> = HashSet::new();

        let max_offset = 160usize;
        for offset in 0..max_offset {
            for sign in [-1isize, 1isize] {
                let neighbor = idx as isize + offset as isize * sign;
                if neighbor < 0 || neighbor >= 160 {
                    continue;
                }
                let bucket = &self.buckets[neighbor as usize];
                for node in bucket.iter() {
                    if !seen.insert(node.id) {
                        continue;
                    }
                    let dist = xor_distance_u128(&node.id, target_id);
                    candidates.push((dist, node.clone()));
                }
            }
        }

        candidates.sort_by_key(|(dist, _)| *dist);
        candidates.truncate(count);
        candidates.into_iter().map(|(_, n)| n).collect()
    }

    /// Return per-bucket node counts for diagnostics.
    fn bucket_counts(&self) -> Vec<usize> {
        self.buckets.iter().map(|b| b.len()).collect()
    }

    /// Total nodes across all buckets.
    fn total_nodes(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).sum()
    }
}

/// ------------------------------------------------------------------
///  BitTorrent Peer
/// ------------------------------------------------------------------

/// A BitTorrent peer with rarest-first piece selection
/// and tit-for-tat choking.
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

    /// Mark a piece as acquired.
    fn have(&mut self, piece: usize) {
        if piece < self.total_pieces {
            self.have_pieces[piece] = true;
        }
    }

    /// Return missing pieces sorted by rarity (rarest first).
    fn rarest_first(&self, swarm_rarity: &[usize]) -> Vec<usize> {
        let mut missing: Vec<(usize, usize)> = self
            .have_pieces
            .iter()
            .enumerate()
            .filter(|(_, &have)| !have)
            .map(|(i, _)| {
                let rarity = if i < swarm_rarity.len() {
                    swarm_rarity[i]
                } else {
                    0
                };
                (rarity, i)
            })
            .collect();
        missing.sort_by_key(|&(rarity, _)| rarity);
        missing.into_iter().map(|(_, i)| i).collect()
    }

    /// Return fraction of pieces owned.
    fn have_fraction(&self) -> f64 {
        let owned = self.have_pieces.iter().filter(|&&h| h).count();
        owned as f64 / self.total_pieces as f64
    }

    /// Set a peer's upload rate to us (in KB/s).
    fn set_upload_rate(&mut self, peer_id: &str, rate: f64) {
        if let Some(pos) = self.upload_rates.iter().position(|(id, _)| id == peer_id) {
            self.upload_rates[pos].1 = rate;
        } else {
            self.upload_rates.push((peer_id.to_string(), rate));
        }
    }

    /// Compute unchoke set: peers with highest upload rate fill the slots.
    fn compute_unchoke_set(&self, slots: usize) -> Vec<String> {
        let mut sorted = self.upload_rates.clone();
        sorted.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted
            .into_iter()
            .take(slots)
            .map(|(id, _)| id)
            .collect()
    }
}

/// ------------------------------------------------------------------
///  Main
/// ------------------------------------------------------------------

fn main() {
    println!("========================================");
    println!("P2P Networks \u2014 Kademlia + BitTorrent");
    println!("========================================");

    // ==============================================================
    // Part 1: Kademlia Routing Table
    // ==============================================================
    println!("\n--- Kademlia Routing Table ---\n");

    let local_id: [u8; 20] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99,
        0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11, 0x22, 0x33,
    ];
    let local = Node::new(local_id, [10, 0, 0, 1], 8000);
    let mut table = RoutingTable::<20>::new(local.id);

    // Create 60 nodes with predictable IDs and add them to the table
    let ip: [u8; 4] = [10, 0, 0, 0];
    for i in 0..60 {
        let mut nid = [0u8; 20];
        nid[0] = (i.wrapping_mul(37) & 0xff) as u8;
        nid[1] = (i.wrapping_mul(73) & 0xff) as u8;
        nid[2] = (i.wrapping_mul(151) & 0xff) as u8;
        nid[19] = i as u8;
        let node = Node::new(nid, ip, 9000 + i as u16);
        table.add(node);
    }
    println!("Added 60 nodes to routing table.");
    let counts = table.bucket_counts();
    let occupied: usize = counts.iter().filter(|&&c| c > 0).count();
    println!(
        "Total nodes in routing table: {} ({} occupied buckets)",
        table.total_nodes(),
        occupied
    );

    // Display buckets with most nodes
    let mut bucket_stats: Vec<(usize, usize)> =
        counts.iter().enumerate().filter(|(_, &c)| c > 0).map(|(i, &c)| (i, c)).collect();
    bucket_stats.sort_by_key(|&(_, c)| std::cmp::Reverse(c));
    println!("Top 5 most populated buckets:");
    for (idx, count) in bucket_stats.iter().take(5) {
        println!("  Bucket {:>3}: {} nodes", idx, count);
    }

    // FIND_CLOSEST lookup
    let target_id: [u8; 20] = [
        0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x88, 0x77, 0x66,
        0x55, 0x44, 0x33, 0x22, 0x11, 0x00, 0xff, 0xee, 0xdd, 0xcc,
    ];
    let target = Node::new(target_id, [10, 0, 0, 2], 8001);

    let closest = table.find_closest(&target.id, 5);
    println!("\nClosest 5 nodes to target");
    println!("  Target ID: {:02x}{:02x}{:02x}{:02x}...", target_id[0], target_id[1], target_id[2], target_id[3]);
    for (i, n) in closest.iter().enumerate() {
        let dist = local.xor_distance(n);
        println!(
            "  {}. {:02x}{:02x}{:02x}{:02x}...  distance=0x{:02x}{:02x}{:02x}{:02x}...",
            i + 1,
            n.id[0], n.id[1], n.id[2], n.id[3],
            dist[0], dist[1], dist[2], dist[3]
        );
    }

    // Verify XOR distance symmetry
    let d1 = local.xor_distance(&target);
    let d2 = target.xor_distance(&local);
    assert_eq!(d1, d2, "XOR distance must be symmetric");
    println!("\nXOR symmetry: dist(A,B) == dist(B,A) (OK)");

    // Verify XOR self-distance is zero
    let self_dist = local.xor_distance(&local);
    assert!(self_dist.iter().all(|&b| b == 0), "Self-distance must be 0");
    println!("XOR identity: dist(A,A) == 0 (OK)");

    // ==============================================================
    // Part 2: BitTorrent Piece Selection
    // ==============================================================
    println!("\n--- BitTorrent Piece Selection ---\n");

    // Build a swarm rarity map: 100 pieces with pseudo-random availability
    // rarity = number of peers that have the piece (higher = more common)
    let mut swarm_rarity = vec![0usize; 100];
    for i in 0..100 {
        swarm_rarity[i] = ((i.wrapping_mul(7) + 13) % 50) + 1;
    }

    let mut bt_peer = BitTorrentPeer::new("peer-a", 100);

    // Seed with 30 owned pieces
    let seeded: [usize; 30] = [
        3, 7, 12, 15, 18, 22, 27, 31, 34, 38, 42, 45, 49, 51, 55,
        58, 62, 66, 70, 73, 77, 80, 84, 87, 89, 91, 94, 96, 98, 99,
    ];
    for &p in &seeded {
        bt_peer.have(p);
    }

    println!(
        "Peer has {}/100 pieces ({:.0}%)",
        bt_peer.have_pieces.iter().filter(|&&h| h).count(),
        bt_peer.have_fraction() * 100.0
    );

    let rarest_order = bt_peer.rarest_first(&swarm_rarity);
    println!("Rarest-first selection (top 10):");
    for &p in rarest_order.iter().take(10) {
        println!("  Piece {:>3}  rarity={}", p, swarm_rarity[p]);
    }

    // Verify rarest-first correctness
    let missing_rarities: Vec<usize> = bt_peer
        .have_pieces
        .iter()
        .enumerate()
        .filter(|(_, &have)| !have)
        .map(|(i, _)| swarm_rarity[i])
        .collect();
    let mut sorted_missing = missing_rarities.clone();
    sorted_missing.sort();
    let selected_rarities: Vec<usize> =
        rarest_order.iter().take(10).map(|&p| swarm_rarity[p]).collect();
    assert_eq!(
        selected_rarities,
        sorted_missing[..10.min(sorted_missing.len())],
        "Rarest-first order incorrect"
    );
    println!("  Rarest-first correctness: verified (OK)");

    // ==============================================================
    // Part 3: Tit-for-Tat Choking
    // ==============================================================
    println!("\n--- Tit-for-Tat Choking ---\n");

    let mut choker = BitTorrentPeer::new("choker", 100);

    // 3 fast peers, 3 medium, 2 slow
    choker.set_upload_rate("fast-a", 950.0);
    choker.set_upload_rate("fast-b", 880.0);
    choker.set_upload_rate("fast-c", 720.0);
    choker.set_upload_rate("medium-a", 510.0);
    choker.set_upload_rate("medium-b", 430.0);
    choker.set_upload_rate("medium-c", 390.0);
    choker.set_upload_rate("slow-a", 120.0);
    choker.set_upload_rate("slow-b", 85.0);

    let unchoked = choker.compute_unchoke_set(4);
    println!("Upload slots = 4 (tit-for-tat)");
    println!("{:<12s} {:>10s}  {:<10s}", "Peer", "Rate KB/s", "Status");
    println!("{}", "-".repeat(36));
    for (id, rate) in &choker.upload_rates {
        let status = if unchoked.contains(id) {
            "UNCHOKED"
        } else {
            "CHOKED"
        };
        println!("{:<12s} {:>10.1}  [{:<8s}]", id, rate, status);
    }

    let fast_unchoked: Vec<&String> = unchoked.iter().filter(|id| id.starts_with("fast")).collect();
    assert_eq!(fast_unchoked.len(), 3, "All fast peers should be unchoked");
    println!("\n  All 3 fast peers unchoked: verified (OK)");

    println!("\nDone.");
}
