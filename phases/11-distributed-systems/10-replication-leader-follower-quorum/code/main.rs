//! Replication — Leader/Follower, Quorum
//! Phase 11 — Distributed Systems
//!
//! Primary-backup replication with configurable quorum:
//! - ReplicatedLog: leader maintains WAL, replicates to followers
//! - QuorumConfig: N replicas, R read quorum, W write quorum (R + W > N)
//! - ReplicaNode: primary or follower, log, key-value store
//! - PrimaryBack: write with W-quorum, read with R-quorum, read-repair

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub index: u64,
    pub key: String,
    pub value: String,
    pub term: u64,
}

#[derive(Debug, Clone)]
pub struct QuorumConfig {
    pub n: usize,
    pub r: usize,
    pub w: usize,
}

impl QuorumConfig {
    pub fn new(n: usize, r: usize, w: usize) -> Result<Self, String> {
        if r + w <= n {
            return Err(format!(
                "Quorum condition violated: R({}) + W({}) must be > N({}), got {} + {} = {}",
                r, w, n, r, w, r + w
            ));
        }
        if r == 0 || w == 0 {
            return Err("R and W must be at least 1".into());
        }
        if r > n || w > n {
            return Err("R and W must be <= N".into());
        }
        Ok(Self { n, r, w })
    }

    pub fn majority(n: usize) -> Result<Self, String> {
        let q = (n / 2) + 1;
        Self::new(n, q, q)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeRole {
    Primary,
    Follower,
}

#[derive(Debug, Clone)]
pub struct ReplicaNode {
    pub id: usize,
    pub role: NodeRole,
    pub log: Vec<LogEntry>,
    pub kv: HashMap<String, (String, u64)>,
    pub alive: bool,
}

impl ReplicaNode {
    pub fn new(id: usize, role: NodeRole) -> Self {
        Self {
            id,
            role,
            log: Vec::new(),
            kv: HashMap::new(),
            alive: true,
        }
    }

    pub fn last_log_index(&self) -> u64 {
        self.log.last().map(|e| e.index).unwrap_or(0)
    }

    pub fn apply_entry(&mut self, entry: &LogEntry) {
        self.log.push(entry.clone());
        self.kv.insert(entry.key.clone(), (entry.value.clone(), entry.index));
    }

    pub fn apply_log(&mut self) {
        let last_applied = self.kv.values().map(|(_, idx)| *idx).max().unwrap_or(0);
        for entry in &self.log {
            if entry.index > last_applied {
                self.kv.insert(entry.key.clone(), (entry.value.clone(), entry.index));
            }
        }
    }

    pub fn read(&self, key: &str) -> Option<(String, u64)> {
        self.kv.get(key).cloned()
    }

    pub fn kill(&mut self) {
        self.alive = false;
    }

    pub fn revive(&mut self) {
        self.alive = true;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WriteResult {
    Ok { committed: usize, index: u64 },
    QuorumNotReached { acks: usize, needed: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReadResult {
    pub value: Option<String>,
    pub index: u64,
    pub replicas_responded: usize,
    pub stale_replicas: Vec<usize>,
    pub read_repairs: usize,
}

pub struct PrimaryBack {
    pub nodes: Vec<ReplicaNode>,
    pub config: QuorumConfig,
    pub current_term: u64,
    pub next_index: u64,
}

impl PrimaryBack {
    pub fn new(config: QuorumConfig) -> Result<Self, String> {
        let validated = QuorumConfig::new(config.n, config.r, config.w)?;
        let nodes = (0..validated.n)
            .map(|i| {
                if i == 0 {
                    ReplicaNode::new(i, NodeRole::Primary)
                } else {
                    ReplicaNode::new(i, NodeRole::Follower)
                }
            })
            .collect();
        Ok(Self {
            nodes,
            config: validated,
            current_term: 1,
            next_index: 1,
        })
    }

    pub fn write(&mut self, key: &str, value: &str) -> WriteResult {
        let entry = LogEntry {
            index: self.next_index,
            key: key.to_string(),
            value: value.to_string(),
            term: self.current_term,
        };
        self.next_index += 1;

        let mut acks = 0;
        let mut committed_nodes: Vec<usize> = Vec::new();

        for node in &mut self.nodes {
            if node.alive {
                node.apply_entry(&entry);
                acks += 1;
                committed_nodes.push(node.id);
            }
        }

        if acks >= self.config.w {
            WriteResult::Ok {
                committed: acks,
                index: entry.index,
            }
        } else {
            for node_id in committed_nodes {
                if let Some(node) = self.nodes.get_mut(node_id) {
                    node.log.retain(|e| e.index != entry.index);
                    let key_to_remove = node.kv.iter()
                        .find(|(_, (_, idx))| *idx == entry.index)
                        .map(|(k, _)| k.clone());
                    if let Some(k) = key_to_remove {
                        node.kv.remove(&k);
                    }
                }
            }
            WriteResult::QuorumNotReached {
                acks,
                needed: self.config.w,
            }
        }
    }

    pub fn read(&mut self, key: &str) -> ReadResult {
        let mut responses: Vec<(usize, Option<(String, u64)>)> = Vec::new();
        let mut responded = 0;

        for node in &self.nodes {
            if node.alive {
                responses.push((node.id, node.read(key)));
                responded += 1;
                if responded >= self.config.r {
                    break;
                }
            }
        }

        let latest: Option<(String, u64)> = responses
            .iter()
            .filter_map(|(_, v)| v.as_ref())
            .max_by_key(|(_, idx)| *idx)
            .cloned();

        let stale_count = if let Some((_, latest_idx)) = &latest {
            responses
                .iter()
                .filter(|(node_id, v)| {
                    match v {
                        Some((_, idx)) => idx < latest_idx,
                        None => {
                            let node = &self.nodes[*node_id];
                            node.last_log_index() < *latest_idx
                        }
                    }
                })
                .map(|(id, _)| *id)
                .collect()
        } else {
            Vec::new()
        };

        let read_repairs = stale_count.len();
        for stale_id in &stale_count {
            let latest_clone = latest.clone();
            if let Some(node) = self.nodes.get_mut(*stale_id) {
                if let Some((val, idx)) = latest_clone {
                    node.kv.insert(key.to_string(), (val, idx));
                }
            }
        }

        let (value, index) = match latest {
            Some((v, idx)) => (Some(v), idx),
            None => (None, 0),
        };
        ReadResult {
            value,
            index,
            replicas_responded: responded,
            stale_replicas: stale_count,
            read_repairs,
        }
    }

    pub fn kill_node(&mut self, node_id: usize) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.kill();
        }
    }

    pub fn revive_node(&mut self, node_id: usize) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.revive();
        }
    }

    pub fn alive_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.alive).count()
    }

    pub fn node_state(&self, node_id: usize) -> Option<&ReplicaNode> {
        self.nodes.get(node_id)
    }
}

fn section(title: &str) {
    println!("\n{}", "=".repeat(60));
    println!("  {}", title);
    println!("{}", "=".repeat(60));
}

fn main() {
    section("1. Quorum Configuration Validation");
    println!("\nValid quorum configs:");
    for (n, r, w) in [(3, 2, 2), (5, 3, 3), (5, 2, 3), (5, 4, 2)] {
        let qc = QuorumConfig::new(n, r, w);
        match qc {
            Ok(c) => println!("  N={}, R={}, W={}  ✓ (R+W={} > N={})", c.n, c.r, c.w, c.r + c.w, c.n),
            Err(e) => println!("  N={}, R={}, W={}  ✗ {}", n, r, w, e),
        }
    }
    println!("\nInvalid quorum configs:");
    for (n, r, w) in [(3, 1, 1), (5, 2, 2), (3, 1, 2)] {
        match QuorumConfig::new(n, r, w) {
            Ok(_) => println!("  N={}, R={}, W={}  ✓ (unexpected!)", n, r, w),
            Err(e) => println!("  N={}, R={}, W={}  ✗ {}", n, r, w, e),
        }
    }
    println!("\nMajority quorum configs:");
    for n in [3, 5, 7] {
        let qc = QuorumConfig::majority(n).unwrap();
        println!("  N={} → R={}, W={} (majority quorum, tolerates {} failures)",
            qc.n, qc.r, qc.w, (n - 1) / 2);
    }

    section("2. Primary-Backup Replication — Normal Operation (N=5, W=3, R=3)");
    let config = QuorumConfig::new(5, 3, 3).unwrap();
    let mut cluster = PrimaryBack::new(config).unwrap();
    println!("\nCluster: {} nodes (1 primary + 4 followers)", cluster.nodes.len());
    println!("Quorum: W={}, R={}, N={}\n", cluster.config.w, cluster.config.r, cluster.config.n);

    println!("Writing key='name' value='alice'...");
    let r = cluster.write("name", "alice");
    println!("  Result: {:?}", r);

    println!("Writing key='city' value='sf'...");
    let r = cluster.write("city", "sf");
    println!("  Result: {:?}", r);

    println!("Writing key='name' value='bob' (update)...");
    let r = cluster.write("name", "bob");
    println!("  Result: {:?}", r);

    println!("\nReading 'name' from {} replicas...", cluster.config.r);
    let r = cluster.read("name");
    println!("  Value: {:?}  (from {} replicas)", r.value, r.replicas_responded);
    println!("  Log index: {}", r.index);

    println!("\nReading 'city' from {} replicas...", cluster.config.r);
    let r = cluster.read("city");
    println!("  Value: {:?}  (from {} replicas)", r.value, r.replicas_responded);

    println!("\nNode states after writes:");
    for node in &cluster.nodes {
        let name_val = node.kv.get("name").map(|(v, i)| format!("name={}@{}", v, i)).unwrap_or("—".into());
        let city_val = node.kv.get("city").map(|(v, i)| format!("city={}@{}", v, i)).unwrap_or("—".into());
        println!("  Node {} [{}] log_len={}  {}  {}",
            node.id, if node.alive { "UP" } else { "DOWN" }, node.log.len(), name_val, city_val);
    }

    section("3. Fault Tolerance — Kill Node 2, Writes and Reads Still Work");
    println!("\nKilling node 2...");
    cluster.kill_node(2);
    println!("Alive nodes: {}/{}", cluster.alive_count(), cluster.config.n);

    println!("\nWriting key='score' value='100' with W=3 quorum...");
    let r = cluster.write("score", "100");
    println!("  Result: {:?}", r);

    println!("\nReading 'score' with R=3 quorum (from 4 alive nodes)...");
    let r = cluster.read("score");
    println!("  Value: {:?}  (from {} replicas)", r.value, r.replicas_responded);

    println!("\nReading 'name' with R=3 quorum...");
    let r = cluster.read("name");
    println!("  Value: {:?}  (from {} replicas)", r.value, r.replicas_responded);

    println!("\nNode states after node 2 killed:");
    for node in &cluster.nodes {
        let score_val = node.kv.get("score").map(|(v, i)| format!("score={}@{}", v, i)).unwrap_or("—".into());
        println!("  Node {} [{}] log_len={}  {}",
            node.id, if node.alive { "UP" } else { "DOWN" }, node.log.len(), score_val);
    }

    section("4. Read-Repair — Stale Replica Detection and Repair");
    println!("\nReviving node 2 (it was down during the 'score' write)...");
    cluster.revive_node(2);
    println!("Alive nodes: {}/{}", cluster.alive_count(), cluster.config.n);

    // Node 2 was down when "score" was written, so it has stale data
    let node2 = cluster.node_state(2).unwrap();
    let n2_score = node2.kv.get("score");
    println!("\nNode 2 state after revival:");
    println!("  Has 'score'? {:?}", n2_score);
    println!("  Log length: {} (missing entries written while down)", node2.log.len());

    // Simulate node 2 partially catching up (has "name" but not "score" or latest "name")
    // In our model, node 2 was down during "score" write, so kv won't have it

    println!("\nReading 'score' with R=3 quorum...");
    println!("  (This read will detect that node 2 is stale and repair it)");
    let r = cluster.read("score");
    println!("  Value: {:?}", r.value);
    println!("  Replicas responded: {}", r.replicas_responded);
    println!("  Stale replicas found: {:?}", r.stale_replicas);
    println!("  Read repairs performed: {}", r.read_repairs);

    // Verify node 2 is now repaired
    let node2 = cluster.node_state(2).unwrap();
    let n2_score_after = node2.kv.get("score");
    println!("\nNode 2 after read-repair:");
    println!("  Has 'score'? {:?}", n2_score_after);

    section("5. Quorum Failure — Too Many Nodes Down");
    println!("\nWith N=5, W=3: killing 3 nodes makes writes impossible.");
    let mut fail_cluster = PrimaryBack::new(QuorumConfig::new(5, 3, 3).unwrap()).unwrap();
    fail_cluster.write("x", "1");
    fail_cluster.kill_node(1);
    fail_cluster.kill_node(2);
    fail_cluster.kill_node(3);
    println!("Alive nodes: {}/{}", fail_cluster.alive_count(), fail_cluster.config.n);
    println!("Attempting write with W=3...");
    let result = fail_cluster.write("y", "2");
    println!("  Result: {:?}", result);
    println!("\nWith N=5, R=3: killing 3 nodes also makes reads impossible.");
    println!("Attempting read with R=3...");
    let result = fail_cluster.read("x");
    println!("  Value: {:?}, responded: {}/{}", result.value, result.replicas_responded, 5);

    section("6. Quorum Configurations Compared");
    let configs = [
        ("Write-all (R=1, W=N)", 5, 1, 5),
        ("Read-all (R=N, W=1)", 5, 5, 1),
        ("Majority (R=3, W=3)", 5, 3, 3),
        ("Dynamo-style (R=2, W=2, N=3)", 3, 2, 2),
    ];
    println!("\n{:35} {:>4} {:>4} {:>4} {:>12} {:>12}", "Config", "N", "R", "W", "R+W>N?", "Tolerates");
    println!("{}", "-".repeat(75));
    for (name, n, r, w) in &configs {
        let valid = r + w > *n;
        let tol = if valid {
            let min_rw = (*r).min(*w);
            let max_fail = n - min_rw;
            format!("{} failures", max_fail)
        } else {
            "UNSAFE".into()
        };
        println!("{:35} {:>4} {:>4} {:>4} {:>12} {:>12}", name, n, r, w, if valid { "✓" } else { "✗" }, tol);
    }

    section("7. Synchronous vs Asynchronous vs Semi-Synchronous Simulation");
    println!("\nSimulating replication modes with N=3, key='balance':\n");

    // Synchronous: wait for all followers
    let mut sync_cluster = PrimaryBack::new(QuorumConfig::new(3, 3, 3).unwrap()).unwrap();
    println!("Synchronous (W=N=3): every write waits for ALL replicas");
    println!("  Write 'balance=500'...");
    let r = sync_cluster.write("balance", "500");
    println!("  Result: {:?}", r);
    println!("  All 3 replicas have the value. If primary crashes, no data is lost.");
    println!("  Trade-off: slow writes (must wait for slowest replica).\n");

    // Asynchronous: write only to primary, no quorum enforcement
    // This uses W=2 R=2 for quorum validity but simulates async by not
    // requiring follower acks in practice — the key insight is that
    // with W=1 (which violates R+W>N), reads may return stale data.
    // We demonstrate this with a W=2 config but only write to primary first.
    let mut async_cluster = PrimaryBack::new(QuorumConfig::new(3, 2, 2).unwrap()).unwrap();
    println!("Asynchronous-style (W=2 but primary acks before followers sync):");
    println!("  Write 'balance=600'...");
    let r = async_cluster.write("balance", "600");
    println!("  Result: {:?}", r);
    async_cluster.kill_node(0);
    println!("  Primary dies! If followers had not yet replicated, data could be lost.");
    println!("  Trade-off: fast writes, but data loss risk if primary crashes.\n");

    // Semi-synchronous: wait for 1 follower (W=2)
    let mut semi_cluster = PrimaryBack::new(QuorumConfig::new(3, 2, 2).unwrap()).unwrap();
    println!("Semi-synchronous (W=2): wait for primary + 1 follower");
    println!("  Write 'balance=700'...");
    let r = semi_cluster.write("balance", "700");
    println!("  Result: {:?}", r);
    println!("  At least 2 replicas have the value. If primary dies, 1 copy survives.");
    println!("  Trade-off: moderate latency, moderate safety.");

    section("Summary");
    println!("\n  • Replication provides availability, durability, and latency gains");
    println!("  • Single-leader: strong consistency, but primary is a SPOF for writes");
    println!("  • Synchronous: strong consistency, slow writes, no data loss");
    println!("  • Asynchronous: fast writes, risk of data loss, eventual consistency");
    println!("  • Semi-synchronous: compromise — at least one follower confirms");
    println!("  • Leaderless: any node accepts writes, quorum R+W>N ensures consistency");
    println!("  • Read-repair: fix stale replicas during reads");
    println!("  • Anti-entropy: background sync for data that isn't being read");
    println!("  • Quorum majority: with N=5, R=3, W=3 tolerates 2 failures");
}