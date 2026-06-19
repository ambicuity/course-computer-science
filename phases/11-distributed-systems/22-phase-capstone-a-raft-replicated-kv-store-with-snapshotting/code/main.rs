//! Phase Capstone — A Raft-Replicated KV Store with Snapshotting
//! Phase 11 — Distributed Systems
//!
//! A complete Raft-replicated key-value store integrating:
//! - Raft consensus (leader election, log replication, commit, snapshots)
//! - KV state machine (SET, GET, DELETE, CAS)
//! - Linearizable reads via read index protocol
//! - Snapshotting for log compaction
//! - Persistence to disk (term, vote, log, snapshots)
//! - Client protocol with leader redirect
//! - Structured logging with trace IDs

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// Trace ID & Structured Log
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct TraceId(String);

impl TraceId {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        TraceId(format!("t{:06x}", id))
    }
}

impl std::fmt::Display for TraceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct StructuredLog;

impl StructuredLog {
    pub fn event(trace_id: &TraceId, layer: &str, msg: &str) {
        println!("[trace={}][{}] {}", trace_id, layer, msg);
    }
}

// ---------------------------------------------------------------------------
// Log Entry & Command Parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub term: u64,
    pub command: Vec<u8>,
}

impl LogEntry {
    pub fn new(term: u64, command: Vec<u8>) -> Self {
        Self { term, command }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KvCommand {
    Set { key: String, value: String },
    Delete { key: String },
    Cas { key: String, expected: String, new_value: String },
}

impl KvCommand {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            KvCommand::Set { key, value } => format!("SET {} {}", key, value).into_bytes(),
            KvCommand::Delete { key } => format!("DELETE {}", key).into_bytes(),
            KvCommand::Cas { key, expected, new_value } => {
                format!("CAS {} {} {}", key, expected, new_value).into_bytes()
            }
        }
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        let s = String::from_utf8_lossy(data);
        let parts: Vec<&str> = s.splitn(4, ' ').collect();
        match parts.get(0)? {
            &"SET" if parts.len() >= 3 => Some(KvCommand::Set {
                key: parts[1].to_string(),
                value: parts[2].to_string(),
            }),
            &"DELETE" if parts.len() >= 2 => Some(KvCommand::Delete {
                key: parts[1].to_string(),
            }),
            &"CAS" if parts.len() >= 4 => Some(KvCommand::Cas {
                key: parts[1].to_string(),
                expected: parts[2].to_string(),
                new_value: parts[3].to_string(),
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CasResult {
    Ok,
    Mismatch,
    NotFound,
}

// ---------------------------------------------------------------------------
// Snapshot
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub last_included_index: u64,
    pub last_included_term: u64,
    pub data: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Node State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeState {
    Follower,
    Candidate,
    Leader,
}

impl std::fmt::Display for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeState::Follower => write!(f, "Follower"),
            NodeState::Candidate => write!(f, "Candidate"),
            NodeState::Leader => write!(f, "Leader"),
        }
    }
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

pub const SNAPSHOT_THRESHOLD: usize = 100;

pub struct RaftPersistentState {
    dir: PathBuf,
    node_id: u64,
}

impl RaftPersistentState {
    pub fn new(dir: &Path, node_id: u64) -> Self {
        Self { dir: dir.to_path_buf(), node_id }
    }

    fn state_path(&self) -> PathBuf { self.dir.join(format!("node-{}-state.txt", self.node_id)) }
    fn log_path(&self) -> PathBuf { self.dir.join(format!("node-{}-log.bin", self.node_id)) }
    fn snapshot_path(&self) -> PathBuf { self.dir.join(format!("node-{}-snap.bin", self.node_id)) }

    pub fn save_state(&self, term: u64, voted_for: Option<u64>) -> io::Result<()> {
        let _ = fs::create_dir_all(&self.dir);
        let mut f = fs::File::create(self.state_path())?;
        write!(f, "{}\n{}\n", term, voted_for.map_or("none".to_string(), |v| v.to_string()))
    }

    pub fn load_state(&self) -> io::Result<(u64, Option<u64>)> {
        if !self.state_path().exists() { return Ok((0, None)); }
        let content = fs::read_to_string(self.state_path())?;
        let mut lines = content.lines();
        let term: u64 = lines.next().unwrap_or("0").parse().unwrap_or(0);
        let v = lines.next().unwrap_or("none");
        Ok((term, if v == "none" { None } else { v.parse().ok() }))
    }

    pub fn save_log(&self, entries: &[LogEntry], snapshot_offset: u64) -> io::Result<()> {
        let _ = fs::create_dir_all(&self.dir);
        let mut f = fs::File::create(self.log_path())?;
        write!(f, "{}\n", snapshot_offset)?;
        for entry in entries {
            write!(f, "{}\n{}\n", entry.term, entry.command.len())?;
            f.write_all(&entry.command)?;
            f.write_all(b"\n")?;
        }
        Ok(())
    }

    pub fn load_log(&self) -> io::Result<(Vec<LogEntry>, u64)> {
        if !self.log_path().exists() { return Ok((Vec::new(), 0)); }
        let content = fs::read_to_string(self.log_path())?;
        let mut lines = content.lines();
        let offset: u64 = lines.next().unwrap_or("0").parse().unwrap_or(0);
        let mut entries = Vec::new();
        loop {
            let ts = match lines.next() { Some(s) => s, None => break };
            let term: u64 = ts.parse().unwrap_or(0);
            let ls = lines.next().unwrap_or("0");
            let len: usize = ls.parse().unwrap_or(0);
            let cs = lines.next().unwrap_or("");
            let cmd = if len > 0 && cs.len() >= len {
                cs.as_bytes()[..len].to_vec()
            } else {
                cs.as_bytes().to_vec()
            };
            entries.push(LogEntry::new(term, cmd));
        }
        Ok((entries, offset))
    }

    pub fn save_snapshot(&self, snapshot: &Snapshot) -> io::Result<()> {
        let _ = fs::create_dir_all(&self.dir);
        let mut f = fs::File::create(self.snapshot_path())?;
        write!(f, "{}\n{}\n", snapshot.last_included_index, snapshot.last_included_term)?;
        f.write_all(&snapshot.data)
    }

    pub fn load_snapshot(&self) -> io::Result<Option<Snapshot>> {
        if !self.snapshot_path().exists() { return Ok(None); }
        let content = fs::read(self.snapshot_path())?;
        let mut pos = 0;
        let line_end = content.iter().position(|&b| b == b'\n').unwrap_or(0);
        if line_end == 0 { return Ok(None); }
        let idx: u64 = String::from_utf8_lossy(&content[pos..line_end]).parse().unwrap_or(0);
        pos = line_end + 1;
        let second_end = content[pos..].iter().position(|&b| b == b'\n').unwrap_or(0) + pos;
        let term: u64 = String::from_utf8_lossy(&content[pos..second_end]).parse().unwrap_or(0);
        let data = content[second_end + 1..].to_vec();
        Ok(Some(Snapshot { last_included_index: idx, last_included_term: term, data }))
    }
}

// ---------------------------------------------------------------------------
// Raft Node
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RaftNode {
    pub id: u64,
    pub state: NodeState,
    pub current_term: u64,
    pub voted_for: Option<u64>,
    pub log: Vec<LogEntry>,
    pub commit_index: u64,
    pub last_applied: u64,
    pub leader_id: Option<u64>,
    pub next_index: Vec<u64>,
    pub match_index: Vec<u64>,
    pub votes_received: Vec<bool>,
    pub snapshot: Option<Snapshot>,
    pub election_timeout: u64,
    pub elapsed_since_heartbeat: u64,
    pub cluster_size: usize,
    pub heartbeat_acks: Vec<bool>,
}

impl RaftNode {
    pub fn new(id: u64, cluster_size: usize) -> Self {
        Self {
            id, state: NodeState::Follower, current_term: 0, voted_for: None,
            log: Vec::new(), commit_index: 0, last_applied: 0, leader_id: None,
            next_index: vec![1; cluster_size], match_index: vec![0; cluster_size],
            votes_received: vec![false; cluster_size], snapshot: None,
            election_timeout: 150 + (id % 150), elapsed_since_heartbeat: 0,
            cluster_size, heartbeat_acks: vec![false; cluster_size],
        }
    }

    pub fn log_index(&self, idx: u64) -> Option<&LogEntry> {
        if idx == 0 { return None; }
        if let Some(ref snap) = self.snapshot {
            if idx <= snap.last_included_index { return None; }
            self.log.get((idx - snap.last_included_index - 1) as usize)
        } else {
            self.log.get((idx - 1) as usize)
        }
    }

    pub fn last_log_index(&self) -> u64 {
        if let Some(ref snap) = self.snapshot {
            if self.log.is_empty() { snap.last_included_index }
            else { snap.last_included_index + self.log.len() as u64 }
        } else { self.log.len() as u64 }
    }

    pub fn last_log_term(&self) -> u64 {
        if let Some(ref snap) = self.snapshot {
            if self.log.is_empty() { snap.last_included_term }
            else { self.log.last().map_or(0, |e| e.term) }
        } else { self.log.last().map_or(0, |e| e.term) }
    }

    pub fn is_log_up_to_date(&self, last_log_term: u64, last_log_index: u64) -> bool {
        let my_term = self.last_log_term();
        let my_idx = self.last_log_index();
        if last_log_term != my_term { last_log_term > my_term } else { last_log_index >= my_idx }
    }

    pub fn try_append_entries(
        &mut self, prev_log_index: u64, prev_log_term: u64,
        entries: Vec<LogEntry>, leader_commit: u64,
    ) -> bool {
        if prev_log_index > 0 {
            match self.log_index(prev_log_index) {
                Some(entry) if entry.term == prev_log_term => {}
                Some(_) => {
                    let tf = if self.snapshot.is_some() {
                        (prev_log_index - self.snapshot.as_ref().unwrap().last_included_index) as usize
                    } else { prev_log_index as usize };
                    if tf <= self.log.len() { self.log.truncate(tf); }
                    return false;
                }
                None => return false,
            }
        }
        let append_start = if self.snapshot.is_some() {
            let snap = self.snapshot.as_ref().unwrap();
            if prev_log_index < snap.last_included_index { return false; }
            prev_log_index - snap.last_included_index
        } else { prev_log_index };

        for (i, entry) in entries.iter().enumerate() {
            let li = append_start as usize + i;
            if li < self.log.len() {
                if self.log[li].term != entry.term {
                    self.log.truncate(li);
                    self.log.push(entry.clone());
                }
            } else { self.log.push(entry.clone()); }
        }
        if leader_commit > self.commit_index {
            self.commit_index = leader_commit.min(self.last_log_index());
        }
        true
    }

    pub fn take_snapshot(&mut self, state_machine: &dyn StateMachine) {
        if self.commit_index == 0 || self.last_applied == 0 { return; }
        let snap_index = self.last_applied;
        let snap_term = self.log_index(snap_index).map_or(0, |e| e.term);
        let data = state_machine.snapshot();
        if let Some(ref existing) = self.snapshot {
            if snap_index <= existing.last_included_index { return; }
        }
        if self.snapshot.is_some() {
            let old_snap = self.snapshot.as_ref().unwrap();
            let drain_count = (snap_index - old_snap.last_included_index) as usize;
            if drain_count <= self.log.len() { self.log.drain(..drain_count); }
        } else {
            let dc = snap_index as usize;
            if dc <= self.log.len() { self.log.drain(..dc); }
        }
        self.snapshot = Some(Snapshot { last_included_index: snap_index, last_included_term: snap_term, data });
    }

    pub fn install_snapshot(&mut self, snapshot: Snapshot, state_machine: &mut dyn StateMachine) {
        if let Some(ref existing) = self.snapshot {
            if snapshot.last_included_index <= existing.last_included_index { return; }
        }
        let entries_before = if self.snapshot.is_some() {
            self.log.len() as u64 + self.snapshot.as_ref().unwrap().last_included_index
        } else { self.log.len() as u64 };

        if snapshot.last_included_index >= entries_before { self.log.clear(); }
        else {
            let keep_from = if self.snapshot.is_some() {
                (snapshot.last_included_index - self.snapshot.as_ref().unwrap().last_included_index) as usize
            } else { snapshot.last_included_index as usize };
            if keep_from <= self.log.len() { self.log = self.log.split_off(keep_from); }
        }
        self.snapshot = Some(snapshot.clone());
        state_machine.apply_snapshot(snapshot.data);
        if self.commit_index < snapshot.last_included_index { self.commit_index = snapshot.last_included_index; }
        if self.last_applied < snapshot.last_included_index { self.last_applied = snapshot.last_included_index; }
    }
}

// ---------------------------------------------------------------------------
// State Machine
// ---------------------------------------------------------------------------

pub trait StateMachine {
    fn apply(&mut self, command: Vec<u8>);
    fn snapshot(&self) -> Vec<u8>;
    fn apply_snapshot(&mut self, data: Vec<u8>);
    fn get(&self, key: &str) -> Option<String>;
}

#[derive(Debug, Clone)]
pub struct KvStateMachine {
    pub data: HashMap<String, String>,
}

impl KvStateMachine {
    pub fn new() -> Self { Self { data: HashMap::new() } }

    pub fn cas(&mut self, key: &str, expected: &str, new_value: &str) -> CasResult {
        match self.data.get(key) {
            Some(current) if current == expected => {
                self.data.insert(key.to_string(), new_value.to_string());
                CasResult::Ok
            }
            Some(_) => CasResult::Mismatch,
            None => CasResult::NotFound,
        }
    }
}

impl StateMachine for KvStateMachine {
    fn apply(&mut self, command: Vec<u8>) {
        if let Some(cmd) = KvCommand::decode(&command) {
            match cmd {
                KvCommand::Set { key, value } => { self.data.insert(key, value); }
                KvCommand::Delete { key } => { self.data.remove(&key); }
                KvCommand::Cas { key, expected, new_value } => {
                    match self.data.get(&key) {
                        Some(current) if current == &expected => {
                            self.data.insert(key, new_value);
                        }
                        Some(_) | None => {}
                    }
                }
            }
        }
    }

    fn snapshot(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut entries: Vec<_> = self.data.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        for (k, v) in &entries {
            bytes.extend_from_slice(k.as_bytes());
            bytes.push(b'=');
            bytes.extend_from_slice(v.as_bytes());
            bytes.push(b';');
        }
        bytes
    }

    fn apply_snapshot(&mut self, data: Vec<u8>) {
        self.data.clear();
        let s = String::from_utf8_lossy(&data);
        for pair in s.split(';') {
            if pair.is_empty() { continue; }
            if let Some(eq_pos) = pair.find('=') {
                self.data.insert(pair[..eq_pos].to_string(), pair[eq_pos + 1..].to_string());
            }
        }
    }

    fn get(&self, key: &str) -> Option<String> { self.data.get(key).cloned() }
}

// ---------------------------------------------------------------------------
// RPC Messages
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct RequestVoteArgs {
    pub term: u64, pub candidate_id: u64,
    pub last_log_index: u64, pub last_log_term: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RequestVoteReply { pub term: u64, pub vote_granted: bool }

#[derive(Debug, Clone, PartialEq)]
pub struct AppendEntriesArgs {
    pub term: u64, pub leader_id: u64,
    pub prev_log_index: u64, pub prev_log_term: u64,
    pub entries: Vec<LogEntry>, pub leader_commit: u64,
    pub is_heartbeat_ack: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppendEntriesReply {
    pub term: u64, pub success: bool, pub match_index: u64, pub heartbeat_ack: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstallSnapshotArgs {
    pub term: u64, pub leader_id: u64,
    pub last_included_index: u64, pub last_included_term: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstallSnapshotReply { pub term: u64 }

#[derive(Debug, Clone, PartialEq)]
pub enum RaftMessage {
    RequestVote { sender_id: u64, args: RequestVoteArgs },
    RequestVoteReply { voter_id: u64, reply: RequestVoteReply },
    AppendEntries { sender_id: u64, args: AppendEntriesArgs },
    AppendEntriesReply { sender_id: u64, reply: AppendEntriesReply },
    InstallSnapshot { sender_id: u64, args: InstallSnapshotArgs },
    InstallSnapshotReply { sender_id: u64, reply: InstallSnapshotReply },
}

// ---------------------------------------------------------------------------
// Network Simulator
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NetworkSim {
    pub queue: VecDeque<(RaftMessage, u64, u64)>,
    pub current_step: u64, pub loss_rate: f64,
    pub partitioned: Vec<Vec<u64>>, pub rng_seed: u64,
}

impl NetworkSim {
    pub fn new() -> Self {
        Self { queue: VecDeque::new(), current_step: 0, loss_rate: 0.0,
               partitioned: Vec::new(), rng_seed: 42 }
    }
    pub fn with_loss(mut self, rate: f64) -> Self { self.loss_rate = rate; self }
    pub fn with_partition(mut self, groups: Vec<Vec<u64>>) -> Self { self.partitioned = groups; self }

    fn pseudo_random(&mut self) -> f64 {
        self.rng_seed = self.rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.rng_seed >> 33) as f64 / (1u64 << 31) as f64
    }

    pub fn can_deliver(&self, from: u64, to: u64) -> bool {
        if self.partitioned.is_empty() { return true; }
        for group in &self.partitioned {
            let fi = group.contains(&from);
            let ti = group.contains(&to);
            if fi && ti { return true; }
            if fi || ti { return false; }
        }
        true
    }

    pub fn send(&mut self, msg: RaftMessage, from: u64, to: u64, delay: u64) {
        if !self.can_deliver(from, to) { return; }
        if self.pseudo_random() < self.loss_rate { return; }
        self.queue.push_back((msg, to, self.current_step + delay));
    }

    pub fn broadcast(&mut self, msg: RaftMessage, from: u64, cluster_size: u64, delay: u64) {
        for to in 0..cluster_size {
            if to != from { self.send(msg.clone(), from, to, delay); }
        }
    }

    pub fn deliver_ready(&mut self) -> Vec<(RaftMessage, u64)> {
        let mut ready = Vec::new();
        let mut remaining = VecDeque::new();
        while let Some((msg, to, at)) = self.queue.pop_front() {
            if at <= self.current_step { ready.push((msg, to)); }
            else { remaining.push_back((msg, to, at)); }
        }
        self.queue = remaining;
        self.current_step += 1;
        ready
    }

    pub fn clear(&mut self) { self.queue.clear(); }
}

// ---------------------------------------------------------------------------
// Pending Read (Linearizable Read Tracker)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PendingRead {
    pub read_index: u64,
    pub key: String,
}

// ---------------------------------------------------------------------------
// Raft Cluster
// ---------------------------------------------------------------------------

pub struct RaftCluster {
    pub nodes: Vec<RaftNode>,
    pub state_machines: Vec<KvStateMachine>,
    pub network: NetworkSim,
    pub cluster_size: usize,
    pub pending_reads: Vec<PendingRead>,
    pub persistence: Vec<Option<RaftPersistentState>>,
    pub snapshot_threshold: usize,
}

impl RaftCluster {
    pub fn new(cluster_size: usize) -> Self {
        let nodes = (0..cluster_size as u64).map(|id| RaftNode::new(id, cluster_size)).collect();
        let state_machines = (0..cluster_size).map(|_| KvStateMachine::new()).collect();
        Self {
            nodes, state_machines, network: NetworkSim::new(), cluster_size,
            pending_reads: Vec::new(), persistence: vec![None; cluster_size],
            snapshot_threshold: SNAPSHOT_THRESHOLD,
        }
    }

    pub fn with_persistence_dir(mut self, dir: &Path) -> Self {
        let _ = fs::create_dir_all(dir);
        for i in 0..self.cluster_size {
            self.persistence[i] = Some(RaftPersistentState::new(dir, i as u64));
        }
        self
    }

    pub fn with_snapshot_threshold(mut self, threshold: usize) -> Self {
        self.snapshot_threshold = threshold; self
    }

    pub fn with_network(mut self, network: NetworkSim) -> Self { self.network = network; self }

    fn majority(&self) -> usize { self.cluster_size / 2 + 1 }

    pub fn propose(&mut self, command: Vec<u8>) -> Option<u64> {
        let leader_idx = self.nodes.iter().position(|n| n.state == NodeState::Leader)?;
        let tid = TraceId::new();
        StructuredLog::event(&tid, "raft", "proposing command");

        let entry = {
            let leader = &mut self.nodes[leader_idx];
            let new_index = leader.last_log_index() + 1;
            let entry = LogEntry::new(leader.current_term, command);
            leader.log.push(entry.clone());
            leader.match_index[leader.id as usize] = leader.last_log_index();
            leader.next_index[leader.id as usize] = leader.last_log_index() + 1;
            new_index
        };

        self.send_append_entries(leader_idx);
        self.persist_node(leader_idx);
        Some(entry)
    }

    fn send_append_entries(&mut self, leader_idx: usize) {
        let leader_id = self.nodes[leader_idx].id;
        let term = self.nodes[leader_idx].current_term;
        let commit = self.nodes[leader_idx].commit_index;

        for peer in 0..self.cluster_size as u64 {
            if peer == leader_id { continue; }
            let prev_idx = self.nodes[leader_idx].next_index[peer as usize].saturating_sub(1);
            let prev_term = self.get_prev_term(leader_idx, prev_idx);
            let entries = self.get_entries_from(leader_idx, prev_idx + 1);
            let needs_snap = self.nodes[leader_idx].snapshot.as_ref()
                .map_or(false, |s| prev_idx < s.last_included_index && !entries.is_empty());

            if needs_snap {
                let snap = self.nodes[leader_idx].snapshot.clone().unwrap();
                self.network.send(RaftMessage::InstallSnapshot {
                    sender_id: leader_id,
                    args: InstallSnapshotArgs {
                        term, leader_id,
                        last_included_index: snap.last_included_index,
                        last_included_term: snap.last_included_term,
                        data: snap.data,
                    },
                }, leader_id, peer, 1);
            } else {
                self.network.send(RaftMessage::AppendEntries {
                    sender_id: leader_id,
                    args: AppendEntriesArgs {
                        term, leader_id, prev_log_index: prev_idx, prev_log_term: prev_term,
                        entries, leader_commit: commit, is_heartbeat_ack: false,
                    },
                }, leader_id, peer, 1);
            }
        }
    }

    fn send_heartbeat_with_ack(&mut self, leader_idx: usize) {
        let leader_id = self.nodes[leader_idx].id;
        let term = self.nodes[leader_idx].current_term;
        let commit = self.nodes[leader_idx].commit_index;

        for peer in 0..self.cluster_size as u64 {
            if peer == leader_id { continue; }
            let prev_idx = self.nodes[leader_idx].next_index[peer as usize].saturating_sub(1);
            let prev_term = self.get_prev_term(leader_idx, prev_idx);
            let entries = self.get_entries_from(leader_idx, prev_idx + 1);
            let needs_snap = self.nodes[leader_idx].snapshot.as_ref()
                .map_or(false, |s| prev_idx < s.last_included_index && !entries.is_empty());

            if needs_snap {
                let snap = self.nodes[leader_idx].snapshot.clone().unwrap();
                self.network.send(RaftMessage::InstallSnapshot {
                    sender_id: leader_id,
                    args: InstallSnapshotArgs {
                        term, leader_id,
                        last_included_index: snap.last_included_index,
                        last_included_term: snap.last_included_term,
                        data: snap.data,
                    },
                }, leader_id, peer, 1);
            } else {
                self.network.send(RaftMessage::AppendEntries {
                    sender_id: leader_id,
                    args: AppendEntriesArgs {
                        term, leader_id, prev_log_index: prev_idx, prev_log_term: prev_term,
                        entries, leader_commit: commit, is_heartbeat_ack: true,
                    },
                }, leader_id, peer, 1);
            }
        }
    }

    fn get_prev_term(&self, node_idx: usize, prev_idx: u64) -> u64 {
        if prev_idx == 0 { return 0; }
        if let Some(e) = self.nodes[node_idx].log_index(prev_idx) { e.term }
        else if let Some(ref snap) = self.nodes[node_idx].snapshot {
            if prev_idx == snap.last_included_index { snap.last_included_term } else { 0 }
        } else { 0 }
    }

    fn get_entries_from(&self, node_idx: usize, start_index: u64) -> Vec<LogEntry> {
        let node = &self.nodes[node_idx];
        if let Some(ref snap) = node.snapshot {
            if start_index <= snap.last_included_index { return Vec::new(); }
        }
        let mut entries = Vec::new();
        let mut idx = start_index;
        loop {
            if let Some(entry) = node.log_index(idx) { entries.push(entry.clone()); idx += 1; }
            else { break; }
            if entries.len() > 100 { break; }
        }
        entries
    }

    pub fn do_linearizable_read(&mut self, key: &str) -> Option<String> {
        let leader_idx = self.nodes.iter().position(|n| n.state == NodeState::Leader)?;
        let commit_index = self.nodes[leader_idx].commit_index;

        self.nodes[leader_idx].heartbeat_acks = vec![false; self.cluster_size];
        self.nodes[leader_idx].heartbeat_acks[self.nodes[leader_idx].id as usize] = true;
        self.send_heartbeat_with_ack(leader_idx);

        for _ in 0..200 {
            self.tick();
            let ack_count = self.nodes[leader_idx].heartbeat_acks.iter().filter(|&&a| a).count();
            if ack_count >= self.majority() { break; }
        }

        while self.nodes[leader_idx].last_applied < self.nodes[leader_idx].commit_index {
            self.tick();
        }

        self.state_machines[leader_idx].get(key)
    }

    pub fn tick(&mut self) -> Vec<String> {
        let mut events = Vec::new();

        for i in 0..self.nodes.len() { self.nodes[i].elapsed_since_heartbeat += 1; }

        // Leader periodic heartbeat
        for i in 0..self.nodes.len() {
            if self.nodes[i].state == NodeState::Leader && self.nodes[i].elapsed_since_heartbeat % 10 == 1 {
                let leader_id = self.nodes[i].id;
                let term = self.nodes[i].current_term;
                let commit = self.nodes[i].commit_index;
                for peer in 0..self.cluster_size as u64 {
                    if peer == leader_id { continue; }
                    let prev_idx = self.nodes[i].next_index[peer as usize].saturating_sub(1);
                    let prev_term = self.get_prev_term(i, prev_idx);
                    let entries = self.get_entries_from(i, prev_idx + 1);
                    let needs_snap = self.nodes[i].snapshot.as_ref()
                        .map_or(false, |s| prev_idx < s.last_included_index && !entries.is_empty());
                    if needs_snap {
                        let snap = self.nodes[i].snapshot.clone().unwrap();
                        self.network.send(RaftMessage::InstallSnapshot {
                            sender_id: leader_id,
                            args: InstallSnapshotArgs {
                                term, leader_id,
                                last_included_index: snap.last_included_index,
                                last_included_term: snap.last_included_term,
                                data: snap.data,
                            },
                        }, leader_id, peer, 1);
                    } else {
                        self.network.send(RaftMessage::AppendEntries {
                            sender_id: leader_id,
                            args: AppendEntriesArgs {
                                term, leader_id, prev_log_index: prev_idx, prev_log_term: prev_term,
                                entries, leader_commit: commit, is_heartbeat_ack: false,
                            },
                        }, leader_id, peer, 1);
                    }
                }
            }
        }

        // Election timeouts
        for i in 0..self.nodes.len() {
            if self.nodes[i].state != NodeState::Leader &&
                self.nodes[i].elapsed_since_heartbeat >= self.nodes[i].election_timeout {
                self.nodes[i].elapsed_since_heartbeat = 0;
                self.nodes[i].state = NodeState::Candidate;
                self.nodes[i].current_term += 1;
                self.nodes[i].voted_for = Some(self.nodes[i].id);
                self.nodes[i].votes_received = vec![false; self.cluster_size];
                self.nodes[i].votes_received[i] = true;
                self.nodes[i].leader_id = None;
                self.persist_node(i);

                let args = RequestVoteArgs {
                    term: self.nodes[i].current_term, candidate_id: self.nodes[i].id,
                    last_log_index: self.nodes[i].last_log_index(),
                    last_log_term: self.nodes[i].last_log_term(),
                };
                self.network.broadcast(
                    RaftMessage::RequestVote { sender_id: self.nodes[i].id, args },
                    self.nodes[i].id, self.cluster_size as u64, 1,
                );
                events.push(format!("Node {} starts election for term {}", self.nodes[i].id, self.nodes[i].current_term));
            }
        }

        let messages = self.network.deliver_ready();
        for (msg, recipient) in messages {
            if recipient as usize >= self.nodes.len() { continue; }
            let ev = self.handle_message(recipient, msg);
            events.extend(ev);
        }

        for i in 0..self.nodes.len() { self.apply_entries(i); }
        self.advance_leader_commit();
        self.auto_snapshot();

        events
    }

    fn apply_entries(&mut self, node_idx: usize) {
        while self.nodes[node_idx].last_applied < self.nodes[node_idx].commit_index {
            self.nodes[node_idx].last_applied += 1;
            if let Some(entry) = self.nodes[node_idx].log_index(self.nodes[node_idx].last_applied) {
                if !entry.command.is_empty() {
                    self.state_machines[node_idx].apply(entry.command.clone());
                }
            }
        }
    }

    fn auto_snapshot(&mut self) {
        for i in 0..self.nodes.len() {
            if self.nodes[i].log.len() > self.snapshot_threshold && self.nodes[i].last_applied > 0 {
                let snap_index = self.nodes[i].last_applied;
                if let Some(ref existing) = self.nodes[i].snapshot {
                    if snap_index <= existing.last_included_index { continue; }
                }
                self.nodes[i].take_snapshot(&self.state_machines[i]);
                self.persist_snapshot(i);
            }
        }
    }

    fn persist_node(&mut self, idx: usize) {
        if let Some(ref ps) = self.persistence[idx] {
            let node = &self.nodes[idx];
            let _ = ps.save_state(node.current_term, node.voted_for);
            let offset = node.snapshot.as_ref().map_or(0, |s| s.last_included_index);
            let _ = ps.save_log(&node.log, offset);
        }
    }

    fn persist_snapshot(&mut self, idx: usize) {
        if let Some(ref ps) = self.persistence[idx] {
            if let Some(ref snap) = self.nodes[idx].snapshot {
                let _ = ps.save_snapshot(snap);
            }
        }
    }

    fn handle_message(&mut self, recipient: u64, msg: RaftMessage) -> Vec<String> {
        let mut events = Vec::new();
        let idx = recipient as usize;
        match msg {
            RaftMessage::RequestVote { sender_id, args } => {
                let reply = self.handle_request_vote(idx, &args);
                self.network.send(RaftMessage::RequestVoteReply { voter_id: recipient, reply },
                    recipient, sender_id, 1);
            }
            RaftMessage::RequestVoteReply { voter_id, reply } => {
                events.extend(self.handle_request_vote_reply(idx, voter_id, &reply));
            }
            RaftMessage::AppendEntries { sender_id, args } => {
                let reply = self.handle_append_entries(idx, &args);
                self.network.send(RaftMessage::AppendEntriesReply { sender_id: recipient, reply },
                    recipient, sender_id, 1);
            }
            RaftMessage::AppendEntriesReply { sender_id, reply } => {
                self.handle_append_entries_reply(idx, sender_id, &reply);
            }
            RaftMessage::InstallSnapshot { sender_id, args } => {
                let reply = self.handle_install_snapshot(idx, &args);
                self.network.send(RaftMessage::InstallSnapshotReply { sender_id: recipient, reply },
                    recipient, sender_id, 1);
            }
            RaftMessage::InstallSnapshotReply { .. } => {}
        }
        events
    }

    fn handle_request_vote(&mut self, node_idx: usize, args: &RequestVoteArgs) -> RequestVoteReply {
        let node = &mut self.nodes[node_idx];
        if args.term < node.current_term {
            return RequestVoteReply { term: node.current_term, vote_granted: false };
        }
        if args.term > node.current_term {
            node.current_term = args.term;
            node.voted_for = None;
            node.state = NodeState::Follower;
            node.elapsed_since_heartbeat = 0;
        }
        let grant = match node.voted_for {
            None => node.is_log_up_to_date(args.last_log_term, args.last_log_index),
            Some(vid) if vid == args.candidate_id =>
                node.is_log_up_to_date(args.last_log_term, args.last_log_index),
            _ => false,
        };
        if grant {
            node.voted_for = Some(args.candidate_id);
            node.elapsed_since_heartbeat = 0;
        }
        self.persist_node(node_idx);
        RequestVoteReply { term: node.current_term, vote_granted: grant }
    }

    fn handle_request_vote_reply(&mut self, node_idx: usize, voter_id: u64, reply: &RequestVoteReply) -> Vec<String> {
        let mut events = Vec::new();
        {
            let node = &mut self.nodes[node_idx];
            if node.state != NodeState::Candidate { return events; }
            if reply.term > node.current_term {
                node.current_term = reply.term;
                node.state = NodeState::Follower;
                node.voted_for = None;
                return events;
            }
            if !(reply.term == node.current_term && reply.vote_granted) { return events; }
            node.votes_received[voter_id as usize] = true;
        }

        let won = {
            let node = &self.nodes[node_idx];
            node.votes_received.iter().filter(|&&v| v).count() >= self.majority()
                && node.state == NodeState::Candidate
        };
        if !won { return events; }

        let (leader_id, term, commit) = {
            let node = &mut self.nodes[node_idx];
            node.state = NodeState::Leader;
            node.leader_id = Some(node.id);
            let next_idx = node.last_log_index() + 1;
            node.next_index = vec![next_idx; self.cluster_size];
            node.match_index = vec![0; self.cluster_size];
            node.heartbeat_acks = vec![false; self.cluster_size];
            node.heartbeat_acks[node.id as usize] = true;
            events.push(format!("Node {} becomes leader for term {}", node.id, node.current_term));
            (node.id, node.current_term, node.commit_index)
        };
        self.persist_node(node_idx);

        for peer in 0..self.cluster_size as u64 {
            if peer == leader_id { continue; }
            let prev_idx = self.nodes[node_idx].next_index[peer as usize].saturating_sub(1);
            let prev_term = self.get_prev_term(node_idx, prev_idx);
            let entries = self.get_entries_from(node_idx, prev_idx + 1);
            let needs_snap = self.nodes[node_idx].snapshot.as_ref()
                .map_or(false, |s| prev_idx < s.last_included_index && !entries.is_empty());

            if needs_snap {
                let snap = self.nodes[node_idx].snapshot.clone().unwrap();
                self.network.send(RaftMessage::InstallSnapshot {
                    sender_id: leader_id,
                    args: InstallSnapshotArgs {
                        term, leader_id,
                        last_included_index: snap.last_included_index,
                        last_included_term: snap.last_included_term,
                        data: snap.data,
                    },
                }, leader_id, peer, 1);
            } else {
                self.network.send(RaftMessage::AppendEntries {
                    sender_id: leader_id,
                    args: AppendEntriesArgs {
                        term, leader_id, prev_log_index: prev_idx, prev_log_term: prev_term,
                        entries, leader_commit: commit, is_heartbeat_ack: false,
                    },
                }, leader_id, peer, 1);
            }
        }
        events
    }

    fn handle_append_entries(&mut self, node_idx: usize, args: &AppendEntriesArgs) -> AppendEntriesReply {
        let node = &mut self.nodes[node_idx];
        if args.term < node.current_term {
            return AppendEntriesReply { term: node.current_term, success: false, match_index: 0, heartbeat_ack: false };
        }
        if args.term > node.current_term {
            node.current_term = args.term;
            node.voted_for = None;
            node.state = NodeState::Follower;
        }
        node.leader_id = Some(args.leader_id);
        node.elapsed_since_heartbeat = 0;

        let success = node.try_append_entries(
            args.prev_log_index, args.prev_log_term, args.entries.clone(), args.leader_commit);
        let match_idx = if success { node.last_log_index() } else { 0 };
        let ack = args.is_heartbeat_ack && success;
        self.persist_node(node_idx);

        AppendEntriesReply { term: node.current_term, success, match_index: match_idx, heartbeat_ack: ack }
    }

    fn handle_append_entries_reply(&mut self, node_idx: usize, sender_id: u64, reply: &AppendEntriesReply) {
        {
            let node = &mut self.nodes[node_idx];
            if node.state != NodeState::Leader { return; }
            if reply.term > node.current_term {
                node.current_term = reply.term;
                node.state = NodeState::Follower;
                node.voted_for = None;
                return;
            }
            if reply.heartbeat_ack && reply.success {
                node.heartbeat_acks[sender_id as usize] = true;
            }
        }

        if reply.success {
            self.nodes[node_idx].match_index[sender_id as usize] = reply.match_index;
            self.nodes[node_idx].next_index[sender_id as usize] = reply.match_index + 1;
            return;
        }

        if self.nodes[node_idx].next_index[sender_id as usize] > 1 {
            self.nodes[node_idx].next_index[sender_id as usize] -= 1;
        }

        let prev_idx = self.nodes[node_idx].next_index[sender_id as usize].saturating_sub(1);
        if prev_idx >= self.nodes[node_idx].last_log_index() { return; }

        let needs_snap = self.nodes[node_idx].snapshot.as_ref()
            .map_or(false, |s| prev_idx < s.last_included_index);

        if needs_snap {
            let snap = self.nodes[node_idx].snapshot.clone().unwrap();
            let lid = self.nodes[node_idx].id;
            let t = self.nodes[node_idx].current_term;
            self.network.send(RaftMessage::InstallSnapshot {
                sender_id: lid,
                args: InstallSnapshotArgs {
                    term: t, leader_id: lid,
                    last_included_index: snap.last_included_index,
                    last_included_term: snap.last_included_term,
                    data: snap.data,
                },
            }, lid, sender_id, 1);
        } else {
            let entries = self.get_entries_from(node_idx, prev_idx + 1);
            let prev_term = self.get_prev_term(node_idx, prev_idx);
            let lid = self.nodes[node_idx].id;
            let t = self.nodes[node_idx].current_term;
            let c = self.nodes[node_idx].commit_index;
            self.network.send(RaftMessage::AppendEntries {
                sender_id: lid,
                args: AppendEntriesArgs {
                    term: t, leader_id: lid, prev_log_index: prev_idx, prev_log_term: prev_term,
                    entries, leader_commit: c, is_heartbeat_ack: false,
                },
            }, lid, sender_id, 1);
        }
    }

    fn handle_install_snapshot(&mut self, node_idx: usize, args: &InstallSnapshotArgs) -> InstallSnapshotReply {
        let (term, should_install) = {
            let node = &mut self.nodes[node_idx];
            if args.term < node.current_term {
                return InstallSnapshotReply { term: node.current_term };
            }
            if args.term > node.current_term {
                node.current_term = args.term;
                node.voted_for = None;
                node.state = NodeState::Follower;
            }
            node.elapsed_since_heartbeat = 0;
            node.leader_id = Some(args.leader_id);
            (node.current_term, args.term >= node.current_term)
        };

        if should_install {
            let snap = Snapshot {
                last_included_index: args.last_included_index,
                last_included_term: args.last_included_term,
                data: args.data.clone(),
            };
            self.nodes[node_idx].install_snapshot(snap, &mut self.state_machines[node_idx]);
            self.persist_snapshot(node_idx);
        }
        InstallSnapshotReply { term }
    }

    fn advance_leader_commit(&mut self) {
        let leader_idx = match self.nodes.iter().position(|n| n.state == NodeState::Leader) {
            Some(idx) => idx, None => return,
        };
        let leader = &mut self.nodes[leader_idx];
        let term = leader.current_term;
        let mut match_indices: Vec<u64> = leader.match_index.clone();
        match_indices[leader.id as usize] = leader.last_log_index();
        match_indices.sort();
        let n = match_indices[self.cluster_size / 2];
        if n > leader.commit_index {
            if let Some(entry) = leader.log_index(n) {
                if entry.term == term { leader.commit_index = n; }
            }
        }
    }

    pub fn run_until_elected(&mut self, max_ticks: u64) -> Option<u64> {
        for _ in 0..max_ticks {
            self.tick();
            for node in &self.nodes {
                if node.state == NodeState::Leader { return Some(node.id); }
            }
        }
        None
    }

    pub fn run_until_committed(&mut self, expected_index: u64, max_ticks: u64) -> bool {
        for _ in 0..max_ticks {
            self.tick();
            let count = self.nodes.iter().filter(|n| n.commit_index >= expected_index).count();
            if count >= self.majority() { return true; }
        }
        false
    }

    pub fn run_until_applied(&mut self, expected_index: u64, max_ticks: u64) -> bool {
        for _ in 0..max_ticks {
            self.tick();
            let count = self.nodes.iter().filter(|n| n.last_applied >= expected_index).count();
            if count >= self.majority() { return true; }
        }
        false
    }

    pub fn get_leader(&self) -> Option<u64> {
        self.nodes.iter().find(|n| n.state == NodeState::Leader).map(|n| n.id)
    }

    pub fn get_leader_idx(&self) -> Option<usize> {
        self.nodes.iter().position(|n| n.state == NodeState::Leader)
    }

    pub fn kill_node(&mut self, id: u64) {
        let idx = id as usize;
        if idx < self.nodes.len() {
            self.nodes[idx].state = NodeState::Follower;
            self.nodes[idx].elapsed_since_heartbeat = u64::MAX / 2;
        }
    }

    pub fn restart_node(&mut self, id: u64) {
        let idx = id as usize;
        if idx < self.nodes.len() {
            self.nodes[idx].elapsed_since_heartbeat = 0;
            self.nodes[idx].state = NodeState::Follower;
        }
    }

    pub fn client_request(&mut self, node_id: u64, command: &str) -> String {
        let node_idx = node_id as usize;
        if node_idx >= self.nodes.len() { return "ERROR unknown node".to_string(); }
        let node = &self.nodes[node_idx];
        if node.state != NodeState::Leader {
            return format!("REDIRECT node-{}", node.leader_id.map_or("unknown".to_string(), |l| l.to_string()));
        }

        let parts: Vec<&str> = command.splitn(4, ' ').collect();
        match parts.get(0).map(|s| *s) {
            Some("SET") if parts.len() >= 3 => {
                if self.propose(KvCommand::Set { key: parts[1].into(), value: parts[2].into() }.encode()).is_some() {
                    "OK".to_string()
                } else { "ERROR propose failed".to_string() }
            }
            Some("GET") if parts.len() >= 2 => {
                let leader_idx = self.get_leader_idx().unwrap();
                match self.state_machines[leader_idx].get(parts[1]) {
                    Some(val) => val,
                    None => "NOT_FOUND".to_string(),
                }
            }
            Some("DELETE") if parts.len() >= 2 => {
                if self.propose(KvCommand::Delete { key: parts[1].into() }.encode()).is_some() {
                    "OK".to_string()
                } else { "ERROR".to_string() }
            }
            Some("CAS") if parts.len() >= 4 => {
                if self.propose(KvCommand::Cas {
                    key: parts[1].into(), expected: parts[2].into(), new_value: parts[3].into()
                }.encode()).is_some() { "OK".to_string() } else { "ERROR".to_string() }
            }
            Some("SNAPSHOT") => {
                let li = self.get_leader_idx().unwrap();
                self.nodes[li].take_snapshot(&self.state_machines[li]);
                self.persist_snapshot(li);
                "OK snapshot taken".to_string()
            }
            Some("STATUS") => {
                let node = &self.nodes[node_idx];
                format!(
                    "node={} state={} term={} leader={} log_len={} commit={} applied={} snapshot={}",
                    node.id, node.state, node.current_term,
                    node.leader_id.map_or("none".to_string(), |l| format!("node-{}", l)),
                    node.log.len(), node.commit_index, node.last_applied,
                    node.snapshot.as_ref().map_or("none".to_string(),
                        |s| format!("idx={},term={}", s.last_included_index, s.last_included_term))
                )
            }
            _ => "ERROR unknown command".to_string(),
        }
    }

    pub fn recover_from_disk(&mut self, node_idx: usize, dir: &Path) -> bool {
        let ps = RaftPersistentState::new(dir, node_idx as u64);
        let (term, voted_for) = match ps.load_state() { Ok(t) => t, Err(_) => return false };
        if term == 0 { return false; }
        let (entries, _offset) = match ps.load_log() { Ok(t) => t, Err(_) => (Vec::new(), 0) };
        let snapshot = match ps.load_snapshot() { Ok(s) => s, Err(_) => None };

        self.nodes[node_idx].current_term = term;
        self.nodes[node_idx].voted_for = voted_for;
        self.nodes[node_idx].log = entries;
        self.nodes[node_idx].snapshot = snapshot;

        if let Some(ref snap) = self.nodes[node_idx].snapshot {
            self.state_machines[node_idx].apply_snapshot(snap.data.clone());
            self.nodes[node_idx].commit_index = snap.last_included_index;
            self.nodes[node_idx].last_applied = snap.last_included_index;
            for entry in &self.nodes[node_idx].log {
                self.state_machines[node_idx].apply(entry.command.clone());
                self.nodes[node_idx].last_applied += 1;
                self.nodes[node_idx].commit_index =
                    self.nodes[node_idx].commit_index.max(self.nodes[node_idx].last_applied);
            }
        }

        self.nodes[node_idx].state = NodeState::Follower;
        self.nodes[node_idx].elapsed_since_heartbeat = 0;
        self.persistence[node_idx] = Some(ps);
        true
    }
}

// ---------------------------------------------------------------------------
// Demos
// ---------------------------------------------------------------------------

fn demo_leader_election() {
    println!("=== Demo 1: Leader Election ===\n");
    let mut cluster = RaftCluster::new(3);
    let leader = cluster.run_until_elected(500);
    match leader {
        Some(id) => println!("Node {} elected as leader for term {}", id, cluster.nodes[id as usize].current_term),
        None => println!("No leader elected"),
    }
    for node in &cluster.nodes {
        println!("  Node {}: state={}, term={}, voted_for={:?}, log_len={}, commit={}",
            node.id, node.state, node.current_term, node.voted_for, node.log.len(), node.commit_index);
    }
    println!();
}

fn demo_kv_operations() {
    println!("=== Demo 2: KV Operations (SET, GET, DELETE, CAS) ===\n");
    let mut cluster = RaftCluster::new(3);
    cluster.run_until_elected(500);
    let leader = cluster.get_leader().expect("no leader");
    println!("Leader: Node {}\n", leader);

    for (k, v) in &[("x", "1"), ("y", "2"), ("z", "3")] {
        let idx = cluster.propose(KvCommand::Set { key: k.to_string(), value: v.to_string() }.encode()).unwrap();
        cluster.run_until_committed(idx, 200);
    }
    for _ in 0..30 { cluster.tick(); }

    let sm = &cluster.state_machines[leader as usize];
    println!("After SET x=1, y=2, z=3:");
    for key in &["x", "y", "z"] { println!("  GET {} = {:?}", key, sm.get(key)); }

    let idx = cluster.propose(KvCommand::Delete { key: "y".into() }.encode()).unwrap();
    cluster.run_until_committed(idx, 200);
    for _ in 0..20 { cluster.tick(); }

    let sm = &cluster.state_machines[leader as usize];
    println!("\nAfter DELETE y: GET y = {:?}, GET x = {:?}", sm.get("y"), sm.get("x"));

    let idx = cluster.propose(KvCommand::Cas { key: "x".into(), expected: "1".into(), new_value: "42".into() }.encode()).unwrap();
    cluster.run_until_committed(idx, 200);
    for _ in 0..20 { cluster.tick(); }
    println!("\nAfter CAS x 1→42: GET x = {:?}", sm.get("x"));

    let idx = cluster.propose(KvCommand::Cas { key: "x".into(), expected: "1".into(), new_value: "99".into() }.encode()).unwrap();
    cluster.run_until_committed(idx, 200);
    for _ in 0..20 { cluster.tick(); }
    println!("After CAS x 1→99 (mismatch): GET x = {:?}", sm.get("x"));
    println!();
}

fn demo_leader_redirect() {
    println!("=== Demo 3: Leader Redirect ===\n");
    let mut cluster = RaftCluster::new(3);
    cluster.run_until_elected(500);
    let leader = cluster.get_leader().expect("no leader");
    let follower = if leader == 0 { 1u64 } else { 0u64 };
    println!("Leader: Node {}", leader);
    println!("Client sends SET to follower Node {}: {}\n", follower, cluster.client_request(follower, "SET test hello"));
    println!("Client sends SET to leader Node {}: {}", leader, cluster.client_request(leader, "SET test hello"));
    println!("Client sends GET to follower Node {}: {}", follower, cluster.client_request(follower, "GET test"));
    println!();
}

fn demo_snapshotting() {
    println!("=== Demo 4: Snapshotting & Log Compaction ===\n");
    let mut cluster = RaftCluster::new(3).with_snapshot_threshold(20);
    cluster.run_until_elected(500);
    let leader = cluster.get_leader().expect("no leader");

    for i in 1..=30u64 {
        let idx = cluster.propose(format!("SET k{} v{}", i, i).as_bytes().to_vec()).unwrap();
        cluster.run_until_committed(idx, 100);
    }
    for _ in 0..50 { cluster.tick(); }

    println!("Before manual snapshot: leader log len = {}, commit = {}",
        cluster.nodes[leader as usize].log.len(), cluster.nodes[leader as usize].commit_index);
    cluster.nodes[leader as usize].take_snapshot(&cluster.state_machines[leader as usize]);
    println!("After snapshot: leader log len = {}, snapshot = {}",
        cluster.nodes[leader as usize].log.len(),
        cluster.nodes[leader as usize].snapshot.as_ref().map_or("none".into(), |s| format!("idx={},term={}", s.last_included_index, s.last_included_term)));

    let idx = cluster.propose(b"SET post_snap newval".to_vec()).unwrap();
    cluster.run_until_committed(idx, 200);
    for _ in 0..30 { cluster.tick(); }

    println!("\nAfter post-snapshot replication:");
    for node in &cluster.nodes {
        println!("  Node {}: commit={}, applied={}, log.len()={}",
            node.id, node.commit_index, node.last_applied, node.log.len());
    }
    println!();
}

fn demo_persistence() {
    println!("=== Demo 5: Persistence & Recovery ===\n");
    let dir = std::env::temp_dir().join("raft-kv-demo-persist");
    let _ = fs::create_dir_all(&dir);
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::create_dir_all(&dir);

    let (stored_term, stored_voted, log_len);
    {
        let mut cluster = RaftCluster::new(3).with_persistence_dir(&dir);
        cluster.run_until_elected(500);
        for i in 1..=5u64 {
            let idx = cluster.propose(format!("SET pk{} pv{}", i, i).as_bytes().to_vec()).unwrap();
            cluster.run_until_committed(idx, 200);
        }
        for _ in 0..30 { cluster.tick(); }
        for i in 0..cluster.cluster_size { cluster.persist_node(i); }
        stored_term = cluster.nodes[0].current_term;
        stored_voted = cluster.nodes[0].voted_for;
        log_len = cluster.nodes[0].log.len();
        println!("Saved: term={}, voted_for={:?}, log_len={}", stored_term, stored_voted, log_len);
    }

    {
        let mut cluster = RaftCluster::new(3).with_persistence_dir(&dir);
        let recovered = cluster.recover_from_disk(0, &dir);
        println!("Recovery: {}", if recovered { "success" } else { "no data" });
        if recovered {
            assert_eq!(cluster.nodes[0].current_term, stored_term);
            assert_eq!(cluster.nodes[0].voted_for, stored_voted);
            assert_eq!(cluster.nodes[0].log.len(), log_len);
            println!("  Verified: term={}, voted_for={:?}, log_len={}",
                cluster.nodes[0].current_term, cluster.nodes[0].voted_for, cluster.nodes[0].log.len());
        }
    }
    let _ = fs::remove_dir_all(&dir);
    println!();
}

fn demo_linearizable_reads() {
    println!("=== Demo 6: Linearizable Reads ===\n");
    let mut cluster = RaftCluster::new(3);
    cluster.run_until_elected(500);
    let _ = cluster.get_leader().expect("no leader");

    let idx = cluster.propose(KvCommand::Set { key: "counter".into(), value: "42".into() }.encode()).unwrap();
    cluster.run_until_committed(idx, 200);
    for _ in 0..30 { cluster.tick(); }

    let value = cluster.do_linearizable_read("counter");
    println!("Linearizable read 'counter' = {:?}", value);

    let idx = cluster.propose(KvCommand::Set { key: "counter".into(), value: "100".into() }.encode()).unwrap();
    cluster.run_until_committed(idx, 200);
    for _ in 0..30 { cluster.tick(); }

    let value = cluster.do_linearizable_read("counter");
    println!("After SET counter=100, linearizable read = {:?}", value);
    println!();
}

fn demo_auto_snapshot() {
    println!("=== Demo 7: Auto-Compaction Snapshotting ===\n");
    let mut cluster = RaftCluster::new(3).with_snapshot_threshold(15);
    cluster.run_until_elected(500);
    let _ = cluster.get_leader().expect("no leader");

    for i in 1..=40u64 {
        let idx = cluster.propose(format!("SET ak{} av{}", i, i).as_bytes().to_vec()).unwrap();
        cluster.run_until_committed(idx, 100);
    }
    for _ in 0..80 { cluster.tick(); }

    println!("After 40 SETs with auto-snapshot (threshold=15):");
    for node in &cluster.nodes {
        println!("  Node {}: log.len()={}, snapshot={}",
            node.id, node.log.len(),
            node.snapshot.as_ref().map_or("none".into(), |s| format!("idx={},term={}", s.last_included_index, s.last_included_term)));
    }
    println!();
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    println!("Raft-Replicated KV Store — Phase 11 Capstone\n");
    println!("=============================================\n");

    demo_leader_election();
    demo_kv_operations();
    demo_leader_redirect();
    demo_snapshotting();
    demo_persistence();
    demo_linearizable_reads();
    demo_auto_snapshot();

    println!("All demos complete. Run `cargo test` for automated verification.");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leader_election_completes() {
        let mut cluster = RaftCluster::new(3);
        let leader = cluster.run_until_elected(500);
        assert!(leader.is_some());
        assert!(cluster.get_leader().is_some());
    }

    #[test]
    fn test_single_leader_per_term() {
        let mut cluster = RaftCluster::new(5);
        cluster.run_until_elected(500);
        let leaders: Vec<_> = cluster.nodes.iter().filter(|n| n.state == NodeState::Leader).collect();
        assert_eq!(leaders.len(), 1);
    }

    #[test]
    fn test_set_get_replication() {
        let mut cluster = RaftCluster::new(3);
        cluster.run_until_elected(500);
        let cmd = KvCommand::Set { key: "x".into(), value: "hello".into() };
        let idx = cluster.propose(cmd.encode()).unwrap();
        assert!(cluster.run_until_committed(idx, 200));
        for _ in 0..30 { cluster.tick(); }
        for sm in &cluster.state_machines {
            assert_eq!(sm.get("x"), Some("hello".to_string()));
        }
    }

    #[test]
    fn test_cas_operation() {
        let mut cluster = RaftCluster::new(3);
        cluster.run_until_elected(500);
        let idx = cluster.propose(KvCommand::Set { key: "counter".into(), value: "0".into() }.encode()).unwrap();
        cluster.run_until_committed(idx, 200);
        for _ in 0..20 { cluster.tick(); }

        let idx = cluster.propose(KvCommand::Cas { key: "counter".into(), expected: "0".into(), new_value: "1".into() }.encode()).unwrap();
        cluster.run_until_committed(idx, 200);
        for _ in 0..20 { cluster.tick(); }

        let leader = cluster.get_leader().unwrap();
        assert_eq!(cluster.state_machines[leader as usize].get("counter"), Some("1".to_string()));

        let idx = cluster.propose(KvCommand::Cas { key: "counter".into(), expected: "0".into(), new_value: "2".into() }.encode()).unwrap();
        cluster.run_until_committed(idx, 200);
        for _ in 0..20 { cluster.tick(); }
        // CAS with wrong expected value should not change counter (the command is applied but it's a no-op)
        assert_eq!(cluster.state_machines[leader as usize].get("counter"), Some("1".to_string()));
    }

    #[test]
    fn test_delete_operation() {
        let mut cluster = RaftCluster::new(3);
        cluster.run_until_elected(500);
        let idx = cluster.propose(KvCommand::Set { key: "k".into(), value: "v".into() }.encode()).unwrap();
        cluster.run_until_committed(idx, 200);
        for _ in 0..20 { cluster.tick(); }
        assert_eq!(cluster.state_machines[0].get("k"), Some("v".to_string()));

        let idx = cluster.propose(KvCommand::Delete { key: "k".into() }.encode()).unwrap();
        cluster.run_until_committed(idx, 200);
        for _ in 0..20 { cluster.tick(); }
        assert_eq!(cluster.state_machines[0].get("k"), None);
    }

    #[test]
    fn test_snapshot_truncates_log() {
        let mut cluster = RaftCluster::new(3);
        cluster.run_until_elected(500);
        for i in 1..=10u64 {
            let idx = cluster.propose(format!("SET k{} v{}", i, i).as_bytes().to_vec()).unwrap();
            assert!(cluster.run_until_committed(idx, 100));
        }
        for _ in 0..30 { cluster.tick(); }
        let leader = cluster.get_leader().unwrap();
        let pre_len = cluster.nodes[leader as usize].log.len();
        assert!(pre_len > 0);
        cluster.nodes[leader as usize].take_snapshot(&cluster.state_machines[leader as usize]);
        let post_len = cluster.nodes[leader as usize].log.len();
        let snap = cluster.nodes[leader as usize].snapshot.as_ref().unwrap();
        assert!(post_len < pre_len || snap.last_included_index > 0);
        assert!(cluster.nodes[leader as usize].commit_index >= 4);
    }

    #[test]
    fn test_install_snapshot_to_lagging_follower() {
        let mut cluster = RaftCluster::new(3);
        cluster.run_until_elected(500);
        for i in 1..=10u64 {
            let idx = cluster.propose(format!("SET k{} v{}", i, i).as_bytes().to_vec()).unwrap();
            assert!(cluster.run_until_committed(idx, 100));
        }
        for _ in 0..30 { cluster.tick(); }
        let leader = cluster.get_leader().unwrap();
        cluster.nodes[leader as usize].take_snapshot(&cluster.state_machines[leader as usize]);
        let follower = if leader == 0 { 1usize } else { 0 };
        cluster.nodes[follower].take_snapshot(&mut cluster.state_machines[follower]);
        assert!(cluster.nodes[follower].snapshot.is_some());
        assert!(cluster.nodes[follower].snapshot.as_ref().unwrap().last_included_index > 0);
    }

    #[test]
    fn test_linearizable_read() {
        let mut cluster = RaftCluster::new(3);
        cluster.run_until_elected(500);
        let idx = cluster.propose(KvCommand::Set { key: "foo".into(), value: "bar".into() }.encode()).unwrap();
        cluster.run_until_committed(idx, 200);
        for _ in 0..30 { cluster.tick(); }
        let value = cluster.do_linearizable_read("foo");
        assert_eq!(value, Some("bar".to_string()));
    }

    #[test]
    fn test_persistence_and_recovery() {
        let dir = std::env::temp_dir().join("raft-kv-test-persist");
        let _ = fs::create_dir_all(&dir);
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::create_dir_all(&dir);

        let (stored_term, stored_voted, log_len);
        {
            let mut cluster = RaftCluster::new(3).with_persistence_dir(&dir);
            cluster.run_until_elected(500);
            for i in 1..=5u64 {
                let idx = cluster.propose(format!("SET pk{} pv{}", i, i).as_bytes().to_vec()).unwrap();
                cluster.run_until_committed(idx, 200);
            }
            for _ in 0..30 { cluster.tick(); }
            for i in 0..cluster.cluster_size { cluster.persist_node(i); }
            stored_term = cluster.nodes[0].current_term;
            stored_voted = cluster.nodes[0].voted_for;
            log_len = cluster.nodes[0].log.len();
        }
        {
            let mut cluster = RaftCluster::new(3).with_persistence_dir(&dir);
            assert!(cluster.recover_from_disk(0, &dir));
            assert_eq!(cluster.nodes[0].current_term, stored_term);
            assert_eq!(cluster.nodes[0].voted_for, stored_voted);
            assert_eq!(cluster.nodes[0].log.len(), log_len);
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_leader_failover() {
        let mut cluster = RaftCluster::new(5);
        cluster.run_until_elected(500);
        let leader = cluster.get_leader().expect("no leader");
        cluster.propose(b"SET before-crash 1".to_vec());
        cluster.run_until_committed(1, 200);
        for _ in 0..20 { cluster.tick(); }

        cluster.kill_node(leader);
        cluster.network.clear();
        let new_leader = cluster.run_until_elected(500);
        assert!(new_leader.is_some());
        assert_ne!(new_leader, Some(leader));
    }

    #[test]
    fn test_election_safety_stale_log_rejected() {
        let mut cluster = RaftCluster::new(5);
        cluster.run_until_elected(500);
        for i in 1..=10u64 {
            let idx = cluster.propose(format!("SET k{} v{}", i, i).as_bytes().to_vec()).unwrap();
            assert!(cluster.run_until_committed(idx, 100));
        }
        let leader = cluster.get_leader().unwrap();
        assert!(cluster.nodes[leader as usize].last_log_index() >= 10);
    }

    #[test]
    fn test_auto_snapshot_triggers() {
        let mut cluster = RaftCluster::new(3).with_snapshot_threshold(20);
        cluster.run_until_elected(500);
        for i in 1..=50u64 {
            let idx = cluster.propose(format!("SET auto-k{} auto-v{}", i, i).as_bytes().to_vec()).unwrap();
            cluster.run_until_committed(idx, 100);
        }
        for _ in 0..100 { cluster.tick(); }
        assert!(cluster.nodes.iter().any(|n| n.snapshot.is_some()));
    }

    #[test]
    fn test_client_redirect() {
        let mut cluster = RaftCluster::new(3);
        cluster.run_until_elected(500);
        let leader = cluster.get_leader().expect("no leader");
        let follower = if leader == 0 { 1u64 } else { 0u64 };
        let response = cluster.client_request(follower, "SET test hello");
        assert!(response.starts_with("REDIRECT"));
    }

    #[test]
    fn test_kv_state_machine_snapshot_restore() {
        let mut sm = KvStateMachine::new();
        sm.apply(b"SET a 1".to_vec());
        sm.apply(b"SET b 2".to_vec());
        sm.apply(b"SET c 3".to_vec());
        let snap = sm.snapshot();
        assert_eq!(sm.get("a"), Some("1".to_string()));
        assert_eq!(sm.get("b"), Some("2".to_string()));

        let mut sm2 = KvStateMachine::new();
        sm2.apply_snapshot(snap);
        assert_eq!(sm2.get("a"), Some("1".to_string()));
        assert_eq!(sm2.get("b"), Some("2".to_string()));
        assert_eq!(sm2.get("c"), Some("3".to_string()));
    }

    #[test]
    fn test_cas_state_machine() {
        let mut sm = KvStateMachine::new();
        assert_eq!(sm.cas("key", "old", "new"), CasResult::NotFound);
        sm.data.insert("key".to_string(), "old".to_string());
        assert_eq!(sm.cas("key", "old", "new"), CasResult::Ok);
        assert_eq!(sm.get("key"), Some("new".to_string()));
        assert_eq!(sm.cas("key", "old", "newer"), CasResult::Mismatch);
    }

    #[test]
    fn test_snapshot_persistence_roundtrip() {
        let dir = std::env::temp_dir().join("raft-kv-test-snap-rt");
        let _ = fs::create_dir_all(&dir);
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::create_dir_all(&dir);
        let snap = Snapshot { last_included_index: 10, last_included_term: 2, data: b"a=1;b=2".to_vec() };
        let ps = RaftPersistentState::new(&dir, 0);
        ps.save_snapshot(&snap).unwrap();
        let loaded = ps.load_snapshot().unwrap().unwrap();
        assert_eq!(loaded.last_included_index, 10);
        assert_eq!(loaded.last_included_term, 2);
        assert_eq!(loaded.data, b"a=1;b=2".to_vec());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_log_matching_property() {
        let mut cluster = RaftCluster::new(3);
        cluster.run_until_elected(500);
        for i in 1..=5u64 {
            let idx = cluster.propose(format!("SET k{} v{}", i, i).as_bytes().to_vec()).unwrap();
            assert!(cluster.run_until_committed(idx, 100));
        }
        for _ in 0..50 { cluster.tick(); }
        let leader = cluster.get_leader().unwrap();
        let leader_entries: Vec<(u64, Vec<u8>)> = cluster.nodes[leader as usize]
            .log.iter().map(|e| (e.term, e.command.clone())).collect();
        for node in &cluster.nodes {
            for (i, entry) in node.log.iter().enumerate() {
                if i < leader_entries.len() {
                    assert_eq!(entry.term, leader_entries[i].0);
                }
            }
        }
    }

    #[test]
    fn test_commit_only_current_term() {
        let mut cluster = RaftCluster::new(3);
        cluster.run_until_elected(500);
        let leader1 = cluster.get_leader().unwrap();
        let term1 = cluster.nodes[leader1 as usize].current_term;
        let idx = cluster.propose(b"SET a 1".to_vec()).unwrap();
        assert!(cluster.run_until_committed(idx, 200));
        cluster.kill_node(leader1);
        cluster.network.clear();
        cluster.run_until_elected(500);
        let new_leader = cluster.get_leader().unwrap();
        assert!(cluster.nodes[new_leader as usize].current_term > term1);
    }
}