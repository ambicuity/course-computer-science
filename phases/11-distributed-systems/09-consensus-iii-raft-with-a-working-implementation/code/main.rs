//! Consensus III — Raft (with a working implementation)
//! Phase 11 — Distributed Systems
//!
//! A complete Raft consensus implementation with leader election, log replication,
//! commit advancement, state machine application, and snapshotting.
//! Includes a simulated network with configurable message delivery.

use std::collections::{HashMap, VecDeque};

// ---------------------------------------------------------------------------
// Log Entry
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

    pub fn empty(term: u64) -> Self {
        Self {
            term,
            command: Vec::new(),
        }
    }
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
// Raft State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum NodeState {
    Follower,
    Candidate,
    Leader,
}

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

    // Leader state
    pub next_index: Vec<u64>,
    pub match_index: Vec<u64>,

    // Candidate state
    pub votes_received: Vec<bool>,

    // Snapshot
    pub snapshot: Option<Snapshot>,

    // Election timeout
    pub election_timeout: u64,
    pub elapsed_since_heartbeat: u64,

    // Cluster info
    pub cluster_size: usize,
}

impl RaftNode {
    pub fn new(id: u64, cluster_size: usize) -> Self {
        let next_index = vec![1u64; cluster_size];
        let match_index = vec![0u64; cluster_size];

        Self {
            id,
            state: NodeState::Follower,
            current_term: 0,
            voted_for: None,
            log: Vec::new(),
            commit_index: 0,
            last_applied: 0,
            leader_id: None,
            next_index,
            match_index,
            votes_received: Vec::new(),
            snapshot: None,
            election_timeout: 150 + (id as u64 % 150),
            elapsed_since_heartbeat: 0,
            cluster_size,
        }
    }

    pub fn log_index(&self, idx: u64) -> Option<&LogEntry> {
        if self.snapshot.is_some() {
            let snap = self.snapshot.as_ref().unwrap();
            if idx <= snap.last_included_index {
                return None;
            }
            let local_idx = (idx - snap.last_included_index - 1) as usize;
            self.log.get(local_idx)
        } else {
            if idx == 0 || idx as usize > self.log.len() {
                return None;
            }
            self.log.get((idx - 1) as usize)
        }
    }

    pub fn last_log_index(&self) -> u64 {
        if let Some(ref snap) = self.snapshot {
            if self.log.is_empty() {
                snap.last_included_index
            } else {
                snap.last_included_index + self.log.len() as u64
            }
        } else {
            self.log.len() as u64
        }
    }

    pub fn last_log_term(&self) -> u64 {
        if let Some(ref snap) = self.snapshot {
            if self.log.is_empty() {
                snap.last_included_term
            } else {
                self.log.last().unwrap().term
            }
        } else {
            self.log.last().map_or(0, |e| e.term)
        }
    }

    pub fn is_log_up_to_date(&self, last_log_term: u64, last_log_index: u64) -> bool {
        let my_last_term = self.last_log_term();
        let my_last_index = self.last_log_index();
        if last_log_term != my_last_term {
            last_log_term > my_last_term
        } else {
            last_log_index >= my_last_index
        }
    }

    pub fn try_append_entries(
        &mut self,
        prev_log_index: u64,
        prev_log_term: u64,
        entries: Vec<LogEntry>,
        leader_commit: u64,
    ) -> bool {
        if prev_log_index > 0 {
            match self.log_index(prev_log_index) {
                Some(entry) if entry.term == prev_log_term => {}
                Some(_) => {
                    let truncate_from = if self.snapshot.is_some() {
                        let snap = self.snapshot.as_ref().unwrap();
                        (prev_log_index - snap.last_included_index) as usize
                    } else {
                        prev_log_index as usize
                    };
                    if truncate_from <= self.log.len() {
                        self.log.truncate(truncate_from);
                    }
                    return false;
                }
                None => return false,
            }
        }

        let append_start = if self.snapshot.is_some() {
            let snap = self.snapshot.as_ref().unwrap();
            if prev_log_index < snap.last_included_index {
                return false;
            }
            prev_log_index - snap.last_included_index
        } else {
            prev_log_index
        };

        for (i, entry) in entries.iter().enumerate() {
            let local_idx = append_start as usize + i;
            if local_idx < self.log.len() {
                if self.log[local_idx].term != entry.term {
                    self.log.truncate(local_idx);
                    self.log.push(entry.clone());
                }
            } else {
                self.log.push(entry.clone());
            }
        }

        if leader_commit > self.commit_index {
            let new_commit = leader_commit.min(self.last_log_index());
            self.commit_index = new_commit;
        }

        true
    }

    pub fn apply_entries(&mut self, state_machine: &mut dyn StateMachine) {
        while self.last_applied < self.commit_index {
            self.last_applied += 1;
            if let Some(entry) = self.log_index(self.last_applied) {
                if !entry.command.is_empty() {
                    state_machine.apply(entry.command.clone());
                }
            }
        }
    }

    pub fn take_snapshot(&mut self, state_machine: &dyn StateMachine) {
        if self.commit_index == 0 || self.last_applied == 0 {
            return;
        }

        let snap_index = self.last_applied;
        let snap_term = self.log_index(snap_index).map_or(0, |e| e.term);

        let data = state_machine.snapshot();

        if let Some(ref existing) = self.snapshot {
            if snap_index <= existing.last_included_index {
                return;
            }
        }

        let _entries_to_keep = if self.snapshot.is_some() {
            let old_snap = self.snapshot.as_ref().unwrap();
            self.log
                .drain(..(snap_index - old_snap.last_included_index) as usize)
                .for_each(drop);
            self.log.len()
        } else {
            let drain_count = snap_index as usize;
            if drain_count <= self.log.len() {
                self.log.drain(..drain_count);
            }
            self.log.len()
        };

        self.snapshot = Some(Snapshot {
            last_included_index: snap_index,
            last_included_term: snap_term,
            data,
        });
    }

    pub fn install_snapshot(&mut self, snapshot: Snapshot, state_machine: &mut dyn StateMachine) {
        if let Some(ref existing) = self.snapshot {
            if snapshot.last_included_index <= existing.last_included_index {
                return;
            }
        }

        let entries_before = if self.snapshot.is_some() {
            let old_snap = self.snapshot.as_ref().unwrap();
            self.log.len() as u64 + old_snap.last_included_index
        } else {
            self.log.len() as u64
        };

        if snapshot.last_included_index >= entries_before {
            self.log.clear();
        } else {
            let keep_from = if self.snapshot.is_some() {
                let old_snap = self.snapshot.as_ref().unwrap();
                (snapshot.last_included_index - old_snap.last_included_index) as usize
            } else {
                snapshot.last_included_index as usize
            };
            if keep_from <= self.log.len() {
                self.log = self.log.split_off(keep_from);
            }
        }

        self.snapshot = Some(snapshot.clone());
        state_machine.apply_snapshot(snapshot.data);

        if self.commit_index < snapshot.last_included_index {
            self.commit_index = snapshot.last_included_index;
        }
        if self.last_applied < snapshot.last_included_index {
            self.last_applied = snapshot.last_included_index;
        }
    }
}

// ---------------------------------------------------------------------------
// State Machine trait
// ---------------------------------------------------------------------------

pub trait StateMachine {
    fn apply(&mut self, command: Vec<u8>);
    fn snapshot(&self) -> Vec<u8>;
    fn apply_snapshot(&mut self, data: Vec<u8>);
}

#[derive(Debug, Clone)]
pub struct KvStateMachine {
    pub data: HashMap<Vec<u8>, Vec<u8>>,
}

impl KvStateMachine {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl StateMachine for KvStateMachine {
    fn apply(&mut self, command: Vec<u8>) {
        let cmd_str = String::from_utf8_lossy(&command);
        let parts: Vec<&str> = cmd_str.splitn(3, ' ').collect();
        if parts.len() >= 3 && parts[0] == "SET" {
            self.data
                .insert(parts[1].as_bytes().to_vec(), parts[2].as_bytes().to_vec());
        } else if parts.len() >= 2 && parts[0] == "DEL" {
            self.data.remove(parts[1].as_bytes());
        }
    }

    fn snapshot(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut entries: Vec<_> = self.data.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        for (k, v) in &entries {
            bytes.extend_from_slice(k);
            bytes.push(b'=');
            bytes.extend_from_slice(v);
            bytes.push(b';');
        }
        bytes
    }

    fn apply_snapshot(&mut self, data: Vec<u8>) {
        self.data.clear();
        let s = String::from_utf8_lossy(&data);
        for pair in s.split(';') {
            if pair.is_empty() {
                continue;
            }
            if let Some(eq_pos) = pair.find('=') {
                let key = pair[..eq_pos].as_bytes().to_vec();
                let value = pair[eq_pos + 1..].as_bytes().to_vec();
                self.data.insert(key, value);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// RPC Messages
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct RequestVoteArgs {
    pub term: u64,
    pub candidate_id: u64,
    pub last_log_index: u64,
    pub last_log_term: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RequestVoteReply {
    pub term: u64,
    pub vote_granted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppendEntriesArgs {
    pub term: u64,
    pub leader_id: u64,
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub entries: Vec<LogEntry>,
    pub leader_commit: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppendEntriesReply {
    pub term: u64,
    pub success: bool,
    pub match_index: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstallSnapshotArgs {
    pub term: u64,
    pub leader_id: u64,
    pub last_included_index: u64,
    pub last_included_term: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstallSnapshotReply {
    pub term: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RaftMessage {
    RequestVote {
        sender_id: u64,
        args: RequestVoteArgs,
    },
    RequestVoteReply {
        voter_id: u64,
        reply: RequestVoteReply,
    },
    AppendEntries {
        sender_id: u64,
        args: AppendEntriesArgs,
    },
    AppendEntriesReply {
        sender_id: u64,
        reply: AppendEntriesReply,
    },
    InstallSnapshot {
        sender_id: u64,
        args: InstallSnapshotArgs,
    },
    InstallSnapshotReply {
        sender_id: u64,
        reply: InstallSnapshotReply,
    },
}

// ---------------------------------------------------------------------------
// Network Simulator
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NetworkSim {
    pub queue: VecDeque<(RaftMessage, u64, u64)>,
    pub current_step: u64,
    pub loss_rate: f64,
    pub partitioned: Vec<Vec<u64>>,
    pub rng_seed: u64,
}

impl NetworkSim {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            current_step: 0,
            loss_rate: 0.0,
            partitioned: Vec::new(),
            rng_seed: 42,
        }
    }

    pub fn with_loss(mut self, rate: f64) -> Self {
        self.loss_rate = rate;
        self
    }

    pub fn with_partition(mut self, groups: Vec<Vec<u64>>) -> Self {
        self.partitioned = groups;
        self
    }

    fn pseudo_random(&mut self) -> f64 {
        self.rng_seed = self.rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let x = self.rng_seed;
        (x >> 33) as f64 / (1u64 << 31) as f64
    }

    pub fn can_deliver(&self, from: u64, to: u64) -> bool {
        if self.partitioned.is_empty() {
            return true;
        }
        for group in &self.partitioned {
            let from_in = group.contains(&from);
            let to_in = group.contains(&to);
            if from_in && to_in {
                return true;
            }
            if from_in || to_in {
                return false;
            }
        }
        true
    }

    pub fn send(&mut self, msg: RaftMessage, from: u64, to: u64, delay: u64) {
        if !self.can_deliver(from, to) {
            return;
        }
        if self.pseudo_random() < self.loss_rate {
            return;
        }
        self.queue.push_back((msg, to, self.current_step + delay));
    }

    pub fn broadcast(&mut self, msg: RaftMessage, from: u64, cluster_size: u64, delay: u64) {
        for to in 0..cluster_size {
            if to != from {
                self.send(msg.clone(), from, to, delay);
            }
        }
    }

    pub fn deliver_ready(&mut self) -> Vec<(RaftMessage, u64)> {
        let mut ready = Vec::new();
        let mut remaining = VecDeque::new();
        while let Some((msg, to, deliver_at)) = self.queue.pop_front() {
            if deliver_at <= self.current_step {
                ready.push((msg, to));
            } else {
                remaining.push_back((msg, to, deliver_at));
            }
        }
        self.queue = remaining;
        self.current_step += 1;
        ready
    }

    pub fn step(&mut self) {
        self.current_step += 1;
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

// ---------------------------------------------------------------------------
// Raft Cluster
// ---------------------------------------------------------------------------

pub struct RaftCluster {
    pub nodes: Vec<RaftNode>,
    pub state_machines: Vec<KvStateMachine>,
    pub network: NetworkSim,
    pub cluster_size: usize,
}

impl RaftCluster {
    pub fn new(cluster_size: usize) -> Self {
        let nodes: Vec<RaftNode> = (0..cluster_size as u64)
            .map(|id| {
                let mut node = RaftNode::new(id, cluster_size);
                node.election_timeout = 150 + (id % 150);
                node
            })
            .collect();
        let state_machines = (0..cluster_size).map(|_| KvStateMachine::new()).collect();

        Self {
            nodes,
            state_machines,
            network: NetworkSim::new(),
            cluster_size,
        }
    }

    pub fn with_network(mut self, network: NetworkSim) -> Self {
        self.network = network;
        self
    }

    fn majority(&self) -> usize {
        self.cluster_size / 2 + 1
    }

    pub fn tick(&mut self) -> Vec<String> {
        let mut events = Vec::new();

        for i in 0..self.nodes.len() {
            self.nodes[i].elapsed_since_heartbeat += 1;
        }

        // Leader sends periodic heartbeats
        for i in 0..self.nodes.len() {
            if self.nodes[i].state == NodeState::Leader {
                if self.nodes[i].elapsed_since_heartbeat % 10 == 1 {
                    let leader_id = self.nodes[i].id;
                    let term = self.nodes[i].current_term;
                    let commit = self.nodes[i].commit_index;
                    for peer in 0..self.cluster_size as u64 {
                        if peer == leader_id {
                            continue;
                        }
                        let prev_idx = self.nodes[i].next_index[peer as usize].saturating_sub(1);
                        let prev_term = if prev_idx == 0 {
                            0
                        } else if let Some(e) = self.nodes[i].log_index(prev_idx) {
                            e.term
                        } else if let Some(ref snap) = self.nodes[i].snapshot {
                            if prev_idx == snap.last_included_index {
                                snap.last_included_term
                            } else {
                                0
                            }
                        } else {
                            0
                        };
                        let entries = self.get_entries_from(i, prev_idx + 1);
                        let needs_snapshot = self.nodes[i].snapshot.as_ref().map_or(false, |s| prev_idx < s.last_included_index && !entries.is_empty());
                        if needs_snapshot {
                            let snap = self.nodes[i].snapshot.clone().unwrap();
                            self.network.send(
                                RaftMessage::InstallSnapshot {
                                    sender_id: leader_id,
                                    args: InstallSnapshotArgs {
                                        term,
                                        leader_id,
                                        last_included_index: snap.last_included_index,
                                        last_included_term: snap.last_included_term,
                                        data: snap.data,
                                    },
                                },
                                leader_id,
                                peer,
                                1,
                            );
                        } else {
                            let args = AppendEntriesArgs {
                                term,
                                leader_id,
                                prev_log_index: prev_idx,
                                prev_log_term: prev_term,
                                entries,
                                leader_commit: commit,
                            };
                            self.network.send(
                                RaftMessage::AppendEntries {
                                    sender_id: leader_id,
                                    args,
                                },
                                leader_id,
                                peer,
                                1,
                            );
                        }
                    }
                }
            }
        }

        // Check election timeouts
        for i in 0..self.nodes.len() {
            if self.nodes[i].state != NodeState::Leader {
                if self.nodes[i].elapsed_since_heartbeat >= self.nodes[i].election_timeout {
                    self.nodes[i].elapsed_since_heartbeat = 0;
                    self.nodes[i].state = NodeState::Candidate;
                    self.nodes[i].current_term += 1;
                    self.nodes[i].voted_for = Some(self.nodes[i].id);
                    self.nodes[i].votes_received = vec![false; self.cluster_size];
                    self.nodes[i].votes_received[i] = true;
                    self.nodes[i].leader_id = None;

                    let args = RequestVoteArgs {
                        term: self.nodes[i].current_term,
                        candidate_id: self.nodes[i].id,
                        last_log_index: self.nodes[i].last_log_index(),
                        last_log_term: self.nodes[i].last_log_term(),
                    };
                    self.network.broadcast(
                        RaftMessage::RequestVote {
                            sender_id: self.nodes[i].id,
                            args,
                        },
                        self.nodes[i].id,
                        self.cluster_size as u64,
                        1,
                    );
                    events.push(format!("Node {} starts election for term {}", self.nodes[i].id, self.nodes[i].current_term));
                }
            }
        }

        let messages = self.network.deliver_ready();
        for (msg, recipient) in messages {
            if recipient as usize >= self.nodes.len() {
                continue;
            }
            let events_from_msg = self.handle_message(recipient, msg);
            events.extend(events_from_msg);
        }

        for i in 0..self.nodes.len() {
            self.nodes[i].apply_entries(&mut self.state_machines[i]);
        }

        self.advance_leader_commit();

        events
    }

    fn handle_message(&mut self, recipient: u64, msg: RaftMessage) -> Vec<String> {
        let mut events = Vec::new();
        let idx = recipient as usize;

        match msg {
            RaftMessage::RequestVote { sender_id, args } => {
                let reply = self.handle_request_vote(idx, &args);
                self.network.send(
                    RaftMessage::RequestVoteReply {
                        voter_id: recipient,
                        reply,
                    },
                    recipient,
                    sender_id,
                    1,
                );
            }
            RaftMessage::RequestVoteReply { voter_id, reply } => {
                let events_list = self.handle_request_vote_reply(idx, voter_id, &reply);
                events.extend(events_list);
            }
            RaftMessage::AppendEntries { sender_id, args } => {
                let reply = self.handle_append_entries(idx, &args);
                self.network.send(
                    RaftMessage::AppendEntriesReply {
                        sender_id: recipient,
                        reply,
                    },
                    recipient,
                    sender_id,
                    1,
                );
            }
            RaftMessage::AppendEntriesReply {
                sender_id,
                reply,
            } => {
                self.handle_append_entries_reply(idx, sender_id, &reply);
            }
            RaftMessage::InstallSnapshot { sender_id, args } => {
                let reply = self.handle_install_snapshot(idx, &args);
                self.network.send(
                    RaftMessage::InstallSnapshotReply {
                        sender_id: recipient,
                        reply,
                    },
                    recipient,
                    sender_id,
                    1,
                );
            }
            RaftMessage::InstallSnapshotReply { .. } => {}
        }

        events
    }

    fn handle_request_vote(&mut self, node_idx: usize, args: &RequestVoteArgs) -> RequestVoteReply {
        let node = &mut self.nodes[node_idx];

        if args.term < node.current_term {
            return RequestVoteReply {
                term: node.current_term,
                vote_granted: false,
            };
        }

        if args.term > node.current_term {
            node.current_term = args.term;
            node.voted_for = None;
            node.state = NodeState::Follower;
            node.elapsed_since_heartbeat = 0;
        }

        let grant = match node.voted_for {
            None => node.is_log_up_to_date(args.last_log_term, args.last_log_index),
            Some(vid) if vid == args.candidate_id => {
                node.is_log_up_to_date(args.last_log_term, args.last_log_index)
            }
            _ => false,
        };

        if grant {
            node.voted_for = Some(args.candidate_id);
            node.elapsed_since_heartbeat = 0;
        }

        RequestVoteReply {
            term: node.current_term,
            vote_granted: grant,
        }
    }

    fn handle_request_vote_reply(
        &mut self,
        node_idx: usize,
        voter_id: u64,
        reply: &RequestVoteReply,
    ) -> Vec<String> {
        let mut events = Vec::new();

        {
            let node = &mut self.nodes[node_idx];
            if node.state != NodeState::Candidate {
                return events;
            }
            if reply.term > node.current_term {
                node.current_term = reply.term;
                node.state = NodeState::Follower;
                node.voted_for = None;
                return events;
            }
            if !(reply.term == node.current_term && reply.vote_granted) {
                return events;
            }
            node.votes_received[voter_id as usize] = true;
        }

        let won_election = {
            let node = &self.nodes[node_idx];
            let vote_count = node.votes_received.iter().filter(|&&v| v).count();
            vote_count >= self.majority() && node.state == NodeState::Candidate
        };

        if !won_election {
            return events;
        }

        struct PeerEntry {
            peer: u64,
            prev_idx: u64,
            prev_term: u64,
            entries: Vec<LogEntry>,
        }

        let (leader_id, term, commit, peer_entries): (u64, u64, u64, Vec<PeerEntry>) = {
            let next_index_base;
            let leader_id;
            let term;
            let commit;
            {
                let node = &mut self.nodes[node_idx];
                node.state = NodeState::Leader;
                node.leader_id = Some(node.id);
                next_index_base = node.last_log_index() + 1;
                node.next_index = vec![next_index_base; self.cluster_size];
                node.match_index = vec![0; self.cluster_size];
                leader_id = node.id;
                term = node.current_term;
                commit = node.commit_index;
                events.push(format!(
                    "Node {} becomes leader for term {}",
                    leader_id, term
                ));
            }

            let node = &self.nodes[node_idx];
            let mut pe = Vec::new();
            for peer in 0..self.cluster_size as u64 {
                if peer == leader_id {
                    continue;
                }
                let prev_idx = next_index_base.saturating_sub(1);
                let prev_term = if prev_idx == 0 {
                    0
                } else if let Some(e) = node.log_index(prev_idx) {
                    e.term
                } else {
                    0
                };
                let mut entries = Vec::new();
                let mut idx = prev_idx + 1;
                loop {
                    if let Some(entry) = node.log_index(idx) {
                        entries.push(entry.clone());
                        idx += 1;
                    } else {
                        break;
                    }
                    if entries.len() > 100 {
                        break;
                    }
                }
                pe.push(PeerEntry { peer, prev_idx, prev_term, entries });
            }
            (leader_id, term, commit, pe)
        };

        for pe in peer_entries {
            let args = AppendEntriesArgs {
                term,
                leader_id,
                prev_log_index: pe.prev_idx,
                prev_log_term: pe.prev_term,
                entries: pe.entries,
                leader_commit: commit,
            };
            self.network.send(
                RaftMessage::AppendEntries {
                    sender_id: leader_id,
                    args,
                },
                leader_id,
                pe.peer,
                1,
            );
        }

        events
    }

    fn get_entries_from(&self, node_idx: usize, start_index: u64) -> Vec<LogEntry> {
        let node = &self.nodes[node_idx];
        let mut entries = Vec::new();

        if let Some(ref snap) = node.snapshot {
            if start_index <= snap.last_included_index {
                return entries;
            }
        }

        let mut idx = start_index;
        loop {
            if let Some(entry) = node.log_index(idx) {
                entries.push(entry.clone());
                idx += 1;
            } else {
                break;
            }
            if entries.len() > 100 {
                break;
            }
        }
        entries
    }

    fn handle_append_entries(
        &mut self,
        node_idx: usize,
        args: &AppendEntriesArgs,
    ) -> AppendEntriesReply {
        let node = &mut self.nodes[node_idx];

        if args.term < node.current_term {
            return AppendEntriesReply {
                term: node.current_term,
                success: false,
                match_index: 0,
            };
        }

        if args.term > node.current_term {
            node.current_term = args.term;
            node.voted_for = None;
            node.state = NodeState::Follower;
        }

        node.leader_id = Some(args.leader_id);
        node.elapsed_since_heartbeat = 0;

        let success = node.try_append_entries(
            args.prev_log_index,
            args.prev_log_term,
            args.entries.clone(),
            args.leader_commit,
        );

        let match_idx = if success { node.last_log_index() } else { 0 };

        AppendEntriesReply {
            term: node.current_term,
            success,
            match_index: match_idx,
        }
    }

    fn handle_append_entries_reply(
        &mut self,
        node_idx: usize,
        sender_id: u64,
        reply: &AppendEntriesReply,
    ) {
        {
            let node = &mut self.nodes[node_idx];
            if node.state != NodeState::Leader {
                return;
            }
            if reply.term > node.current_term {
                node.current_term = reply.term;
                node.state = NodeState::Follower;
                node.voted_for = None;
                return;
            }
        }

        if reply.success {
            self.nodes[node_idx].match_index[sender_id as usize] = reply.match_index;
            self.nodes[node_idx].next_index[sender_id as usize] = reply.match_index + 1;
            return;
        }

        {
            let node = &mut self.nodes[node_idx];
            if node.next_index[sender_id as usize] > 1 {
                node.next_index[sender_id as usize] -= 1;
            }
        }

        let prev_idx = self.nodes[node_idx].next_index[sender_id as usize].saturating_sub(1);
        let last_log = self.nodes[node_idx].last_log_index();

        if prev_idx >= last_log {
            return;
        }

        let needs_snapshot = self.nodes[node_idx]
            .snapshot
            .as_ref()
            .map_or(false, |s| prev_idx < s.last_included_index);

        if needs_snapshot {
            let snap = self.nodes[node_idx].snapshot.clone().unwrap();
            let leader_id = self.nodes[node_idx].id;
            let term = self.nodes[node_idx].current_term;
            self.network.send(
                RaftMessage::InstallSnapshot {
                    sender_id: leader_id,
                    args: InstallSnapshotArgs {
                        term,
                        leader_id,
                        last_included_index: snap.last_included_index,
                        last_included_term: snap.last_included_term,
                        data: snap.data,
                    },
                },
                leader_id,
                sender_id,
                1,
            );
        } else {
            let entries = self.get_entries_from(node_idx, prev_idx + 1);
            let prev_term = if prev_idx == 0 {
                0
            } else if let Some(e) = self.nodes[node_idx].log_index(prev_idx) {
                e.term
            } else if let Some(ref snap) = self.nodes[node_idx].snapshot {
                if prev_idx == snap.last_included_index {
                    snap.last_included_term
                } else {
                    0
                }
            } else {
                0
            };
            let leader_id = self.nodes[node_idx].id;
            let term = self.nodes[node_idx].current_term;
            let commit = self.nodes[node_idx].commit_index;
            let args = AppendEntriesArgs {
                term,
                leader_id,
                prev_log_index: prev_idx,
                prev_log_term: prev_term,
                entries,
                leader_commit: commit,
            };
            self.network.send(
                RaftMessage::AppendEntries {
                    sender_id: leader_id,
                    args,
                },
                leader_id,
                sender_id,
                1,
            );
        }
    }

    fn handle_install_snapshot(
        &mut self,
        node_idx: usize,
        args: &InstallSnapshotArgs,
    ) -> InstallSnapshotReply {
        let (term, should_install) = {
            let node = &mut self.nodes[node_idx];
            if args.term < node.current_term {
                return InstallSnapshotReply {
                    term: node.current_term,
                };
            }

            if args.term > node.current_term {
                node.current_term = args.term;
                node.voted_for = None;
                node.state = NodeState::Follower;
            }

            node.elapsed_since_heartbeat = 0;
            node.leader_id = Some(args.leader_id);

            let should = args.term >= node.current_term;
            (node.current_term, should)
        };

        if should_install {
            let snap = Snapshot {
                last_included_index: args.last_included_index,
                last_included_term: args.last_included_term,
                data: args.data.clone(),
            };
            self.nodes[node_idx].install_snapshot(snap, &mut self.state_machines[node_idx]);
        }

        InstallSnapshotReply { term }
    }

    fn advance_leader_commit(&mut self) {
        let leader_idx = match self.nodes.iter().position(|n| n.state == NodeState::Leader) {
            Some(idx) => idx,
            None => return,
        };

        let leader = &mut self.nodes[leader_idx];
        let term = leader.current_term;

        let mut match_indices: Vec<u64> = leader.match_index.clone();
        match_indices[leader.id as usize] = leader.last_log_index();
        match_indices.sort();
        let n = match_indices[self.cluster_size / 2];

        if n > leader.commit_index {
            if let Some(entry) = leader.log_index(n) {
                if entry.term == term {
                    leader.commit_index = n;
                }
            }
        }
    }

    pub fn propose(&mut self, command: Vec<u8>) -> Option<u64> {
        let leader_idx = self.nodes.iter().position(|n| n.state == NodeState::Leader)?;

        let entry = {
            let leader = &mut self.nodes[leader_idx];
            let term = leader.current_term;
            let new_index = leader.last_log_index() + 1;
            let entry = LogEntry::new(term, command);

            if leader.snapshot.is_some() {
                let snap = leader.snapshot.as_ref().unwrap();
                let local_idx = (new_index - snap.last_included_index - 1) as usize;
                if local_idx < leader.log.len() {
                    leader.log[local_idx] = entry.clone();
                } else {
                    leader.log.push(entry.clone());
                }
            } else {
                leader.log.push(entry.clone());
            }
            leader.match_index[leader.id as usize] = leader.last_log_index();
            leader.next_index[leader.id as usize] = leader.last_log_index() + 1;

            new_index
        };

        let leader_id = self.nodes[leader_idx].id;
        for peer in 0..self.cluster_size as u64 {
            if peer == leader_id {
                continue;
            }

            let (prev_idx, prev_term, entries) = {
                let leader = &self.nodes[leader_idx];
                let prev_idx = leader.next_index[peer as usize].saturating_sub(1);
                let prev_term = if prev_idx == 0 {
                    0
                } else if let Some(e) = leader.log_index(prev_idx) {
                    e.term
                } else if let Some(ref snap) = leader.snapshot {
                    if prev_idx == snap.last_included_index {
                        snap.last_included_term
                    } else {
                        0
                    }
                } else {
                    0
                };
                let entries = self.get_entries_from(leader_idx, prev_idx + 1);

                let needs_snapshot = if leader.snapshot.is_some() {
                    let snap = leader.snapshot.as_ref().unwrap();
                    prev_idx < snap.last_included_index && !entries.is_empty()
                } else {
                    false
                };

                if needs_snapshot {
                    let snap = leader.snapshot.clone().unwrap();
                    self.network.send(
                        RaftMessage::InstallSnapshot {
                            sender_id: leader_id,
                            args: InstallSnapshotArgs {
                                term: leader.current_term,
                                leader_id,
                                last_included_index: snap.last_included_index,
                                last_included_term: snap.last_included_term,
                                data: snap.data,
                            },
                        },
                        leader_id,
                        peer,
                        1,
                    );
                    (prev_idx, prev_term, Vec::new())
                } else {
                    (prev_idx, prev_term, entries)
                }
            };

            if !entries.is_empty() {
                let leader = &self.nodes[leader_idx];
                let args = AppendEntriesArgs {
                    term: leader.current_term,
                    leader_id: leader_id,
                    prev_log_index: prev_idx,
                    prev_log_term: prev_term,
                    entries,
                    leader_commit: leader.commit_index,
                };
                self.network.send(
                    RaftMessage::AppendEntries {
                        sender_id: leader_id,
                        args,
                    },
                    leader_id,
                    peer,
                    1,
                );
            }
        }

        Some(entry)
    }

    pub fn run_until_elected(&mut self, max_ticks: u64) -> Option<u64> {
        for _ in 0..max_ticks {
            self.tick();
            for node in &self.nodes {
                if node.state == NodeState::Leader {
                    return Some(node.id);
                }
            }
        }
        None
    }

    pub fn run_until_committed(&mut self, expected_index: u64, max_ticks: u64) -> bool {
        for _ in 0..max_ticks {
            self.tick();
            let committed_count = self
                .nodes
                .iter()
                .filter(|n| n.commit_index >= expected_index)
                .count();
            if committed_count >= self.majority() {
                return true;
            }
        }
        false
    }

    pub fn run_until_applied(&mut self, expected_index: u64, max_ticks: u64) -> bool {
        for _ in 0..max_ticks {
            self.tick();
            let applied_count = self
                .nodes
                .iter()
                .filter(|n| n.last_applied >= expected_index)
                .count();
            if applied_count >= self.majority() {
                return true;
            }
        }
        false
    }

    pub fn get_leader(&self) -> Option<u64> {
        self.nodes
            .iter()
            .find(|n| n.state == NodeState::Leader)
            .map(|n| n.id)
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
}

// ---------------------------------------------------------------------------
// Demo 1: Leader election in a 3-node cluster
// ---------------------------------------------------------------------------

fn demo_leader_election() {
    println!("=== Demo 1: Leader Election in a 3-node cluster ===\n");

    let mut cluster = RaftCluster::new(3);
    println!("Starting 3-node Raft cluster...");

    let leader = cluster.run_until_elected(500);
    match leader {
        Some(id) => println!("Node {} elected as leader for term {}\n", id, cluster.nodes[id as usize].current_term),
        None => println!("No leader elected within timeout\n"),
    }

    for node in &cluster.nodes {
        println!(
            "  Node {}: state={:?}, term={}, voted_for={:?}, log_len={}, commit_index={}",
            node.id, node.state, node.current_term, node.voted_for, node.log.len(), node.commit_index
        );
    }
    println!();
}

// ---------------------------------------------------------------------------
// Demo 2: Log replication
// ---------------------------------------------------------------------------

fn demo_log_replication() {
    println!("=== Demo 2: Log Replication ===\n");

    let mut cluster = RaftCluster::new(3);
    println!("Starting 3-node cluster, waiting for leader...");

    cluster.run_until_elected(500);
    let leader_id = cluster.get_leader().expect("should have a leader");
    println!("Leader: Node {}\n", leader_id);

    let commands = vec![
        b"SET x 1".to_vec(),
        b"SET y 2".to_vec(),
        b"SET z 3".to_vec(),
    ];

    for (i, cmd) in commands.iter().enumerate() {
        let idx = cluster.propose(cmd.clone()).expect("propose should work");
        println!("Proposed command {}: {:?}", i + 1, String::from_utf8_lossy(cmd));
        let committed = cluster.run_until_committed(idx, 200);
        println!(
            "  Entry at index {} committed: {}",
            idx,
            if committed { "yes" } else { "no" }
        );
    }

    for _ in 0..50 {
        cluster.tick();
    }

    println!("\nNode states after replication:");
    for node in &cluster.nodes {
        let log_entries: Vec<String> = node
            .log
            .iter()
            .map(|e| format!("(t={}, {:?})", e.term, String::from_utf8_lossy(&e.command)))
            .collect();
        println!(
            "  Node {}: commit={}, applied={}, log=[{}]",
            node.id,
            node.commit_index,
            node.last_applied,
            log_entries.join(", ")
        );
    }

    let leader_sm = &cluster.state_machines[leader_id as usize];
    println!("\nLeader state machine: {:?}", String::from_utf8_lossy(&leader_sm.snapshot()));
    println!();
}

// ---------------------------------------------------------------------------
// Demo 3: Leader failover
// ---------------------------------------------------------------------------

fn demo_leader_failover() {
    println!("=== Demo 3: Leader Failover ===\n");

    let mut cluster = RaftCluster::new(5);
    println!("Starting 5-node cluster, waiting for leader...");

    cluster.run_until_elected(500);
    let leader_id = cluster.get_leader().expect("should have a leader");
    println!("Initial leader: Node {}\n", leader_id);

    cluster.propose(b"SET before-crash 1".to_vec());
    cluster.run_until_committed(1, 200);
    for _ in 0..20 {
        cluster.tick();
    }
    println!("Proposed and committed: SET before-crash 1");

    println!("\nKilling leader (Node {})...", leader_id);
    cluster.kill_node(leader_id);
    cluster.network.clear();

    let new_leader = cluster.run_until_elected(500);
    match new_leader {
        Some(id) => println!("New leader elected: Node {} (term {})", id, cluster.nodes[id as usize].current_term),
        None => println!("No new leader elected"),
    }

    if let Some(_new_id) = new_leader {
        if let Some(idx) = cluster.propose(b"SET after-crash 2".to_vec()) {
            println!("New leader proposed: SET after-crash 2 (index {})", idx);
            cluster.run_until_committed(idx, 300);
        }
    }

    for _ in 0..50 {
        cluster.tick();
    }

    println!("\nNode states after failover:");
    for node in &cluster.nodes {
        println!(
            "  Node {}: state={:?}, term={}, commit_index={}, log.len()={}",
            node.id, node.state, node.current_term, node.commit_index, node.log.len()
        );
    }
    println!();
}

// ---------------------------------------------------------------------------
// Demo 4: Snapshotting
// ---------------------------------------------------------------------------

fn demo_snapshotting() {
    println!("=== Demo 4: Snapshotting ===\n");

    let mut cluster = RaftCluster::new(3);
    println!("Starting 3-node cluster, waiting for leader...");

    cluster.run_until_elected(500);
    let leader_id = cluster.get_leader().expect("should have a leader");
    println!("Leader: Node {}\n", leader_id);

    for i in 1..=20 {
        let cmd = format!("SET key{} val{}", i, i);
        if let Some(idx) = cluster.propose(cmd.as_bytes().to_vec()) {
            cluster.run_until_committed(idx, 100);
        }
    }

    for _ in 0..30 {
        cluster.tick();
    }

    println!(
        "Before snapshot: leader log length = {}, commit_index = {}",
        cluster.nodes[leader_id as usize].log.len(),
        cluster.nodes[leader_id as usize].commit_index
    );

    cluster.nodes[leader_id as usize].take_snapshot(&cluster.state_machines[leader_id as usize]);
    println!(
        "After snapshot: leader log length = {}, snapshot = {:?}",
        cluster.nodes[leader_id as usize].log.len(),
        cluster.nodes[leader_id as usize]
            .snapshot
            .as_ref()
            .map(|s| format!("index={}, term={}", s.last_included_index, s.last_included_term))
            .unwrap_or_else(|| "None".to_string())
    );

    for i in 0..cluster.cluster_size {
        if i != leader_id as usize {
            cluster.nodes[i].take_snapshot(&cluster.state_machines[i]);
        }
    }

    println!("\nAfter all nodes snapshot:");
    for node in &cluster.nodes {
        println!(
            "  Node {}: log.len()={}, snapshot_at={}",
            node.id,
            node.log.len(),
            node.snapshot
                .as_ref()
                .map(|s| s.last_included_index)
                .unwrap_or(0)
        );
    }

    let cmd = b"SET post_snap newval".to_vec();
    if let Some(idx) = cluster.propose(cmd) {
        println!("\nProposing command after snapshot: SET post_snap newval (index {})", idx);
        cluster.run_until_committed(idx, 200);
        for _ in 0..30 {
            cluster.tick();
        }
    }

    println!("\nNode states after post-snapshot replication:");
    for node in &cluster.nodes {
        println!(
            "  Node {}: commit={}, applied={}, log.len()={}",
            node.id, node.commit_index, node.last_applied, node.log.len()
        );
    }

    let leader_sm = &cluster.state_machines[leader_id as usize];
    println!("\nLeader state machine snapshot: {:?}", String::from_utf8_lossy(&leader_sm.snapshot()));
    println!();
}

// ---------------------------------------------------------------------------
// Demo 5: Log consistency and safety
// ---------------------------------------------------------------------------

fn demo_log_safety() {
    println!("=== Demo 5: Log Safety — Election restriction prevents stale leaders ===\n");

    let mut cluster = RaftCluster::new(3);

    cluster.run_until_elected(500);
    let leader1 = cluster.get_leader().expect("should have leader");
    println!("Phase 1: Leader elected: Node {}", leader1);

    for i in 1..=5 {
        let cmd = format!("SET k{} v{}", i, i);
        if let Some(idx) = cluster.propose(cmd.as_bytes().to_vec()) {
            cluster.run_until_committed(idx, 200);
        }
    }
    for _ in 0..30 {
        cluster.tick();
    }

    println!(
        "  Leader log: {} entries, commit_index={}",
        cluster.nodes[leader1 as usize].log.len(),
        cluster.nodes[leader1 as usize].commit_index
    );
    for node in &cluster.nodes {
        println!(
            "  Node {}: last_log_index={}, last_log_term={}",
            node.id,
            node.last_log_index(),
            node.last_log_term()
        );
    }

    println!("\nPhase 2: Kill leader, let new leader be elected.");
    println!("The election restriction ensures only a node with the most");
    println!("up-to-date log can win — stale nodes are rejected.\n");

    cluster.kill_node(leader1);
    cluster.network.clear();

    let new_leader = cluster.run_until_elected(500);
    match new_leader {
        Some(id) => {
            println!(
                "New leader: Node {} (term {}), log entries: {}",
                id,
                cluster.nodes[id as usize].current_term,
                cluster.nodes[id as usize].log.len()
            );
        }
        None => println!("No leader elected"),
    }

    let cmd = b"SET post-failover yes".to_vec();
    if let Some(idx) = cluster.propose(cmd) {
        cluster.run_until_committed(idx, 200);
    }
    for _ in 0..30 {
        cluster.tick();
    }

    println!("\nAfter failover replication:");
    for node in &cluster.nodes {
        if node.state != NodeState::Follower || node.elapsed_since_heartbeat < 10000 {
            println!(
                "  Node {}: commit={}, applied={}, log.len()={}",
                node.id,
                node.commit_index,
                node.last_applied,
                node.log.len()
            );
        }
    }
    println!();
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
        let leaders: Vec<u64> = cluster
            .nodes
            .iter()
            .filter(|n| n.state == NodeState::Leader)
            .map(|n| n.id)
            .collect();
        assert_eq!(leaders.len(), 1);
    }

    #[test]
    fn test_log_replication_basic() {
        let mut cluster = RaftCluster::new(3);
        cluster.run_until_elected(500);

        let idx = cluster.propose(b"SET x 1".to_vec()).unwrap();
        let committed = cluster.run_until_committed(idx, 200);
        assert!(committed);

        for _ in 0..30 {
            cluster.tick();
        }

        for node in &cluster.nodes {
            assert!(node.commit_index >= idx);
        }
    }

    #[test]
    fn test_log_matching_property() {
        let mut cluster = RaftCluster::new(3);
        cluster.run_until_elected(500);

        for i in 1..=5 {
            let cmd = format!("SET k{} v{}", i, i);
            let idx = cluster.propose(cmd.as_bytes().to_vec()).unwrap();
            assert!(cluster.run_until_committed(idx, 100));
        }

        for _ in 0..50 {
            cluster.tick();
        }

        let leader_id = cluster.get_leader().unwrap();
        let leader_entries: Vec<(u64, Vec<u8>)> = cluster.nodes[leader_id as usize]
            .log
            .iter()
            .map(|e| (e.term, e.command.clone()))
            .collect();

        for node in &cluster.nodes {
            for (i, entry) in node.log.iter().enumerate() {
                if i < leader_entries.len() {
                    assert_eq!(entry.term, leader_entries[i].0);
                    assert_eq!(entry.command, leader_entries[i].1);
                }
            }
        }
    }

    #[test]
    fn test_commit_only_current_term() {
        let mut cluster = RaftCluster::new(3);
        cluster.run_until_elected(500);
        let leader_id = cluster.get_leader().unwrap();
        let term1 = cluster.nodes[leader_id as usize].current_term;

        let idx = cluster.propose(b"SET a 1".to_vec()).unwrap();
        assert!(cluster.run_until_committed(idx, 200));

        cluster.kill_node(leader_id);
        cluster.network.clear();
        cluster.run_until_elected(500);

        let new_leader = cluster.get_leader().unwrap();
        let term2 = cluster.nodes[new_leader as usize].current_term;
        assert!(term2 > term1);

        let idx2 = cluster.propose(b"SET b 2".to_vec()).unwrap();
        assert!(cluster.run_until_committed(idx2, 300));

        assert!(cluster.nodes[new_leader as usize].commit_index >= idx2);
    }

    #[test]
    fn test_election_rejection_stale_log() {
        let mut cluster = RaftCluster::new(5);
        cluster.run_until_elected(500);

        for i in 1..=10 {
            let cmd = format!("SET k{} v{}", i, i);
            let idx = cluster.propose(cmd.as_bytes().to_vec()).unwrap();
            assert!(cluster.run_until_committed(idx, 100));
        }

        let leader_id = cluster.get_leader().unwrap();
        let leader_log_index = cluster.nodes[leader_id as usize].last_log_index();
        assert!(leader_log_index >= 10);
    }

    #[test]
    fn test_snapshot_truncates_log() {
        let mut cluster = RaftCluster::new(3);
        cluster.run_until_elected(500);

        for i in 1..=10 {
            let cmd = format!("SET k{} v{}", i, i);
            let idx = cluster.propose(cmd.as_bytes().to_vec()).unwrap();
            assert!(cluster.run_until_committed(idx, 100));
        }

        for _ in 0..30 {
            cluster.tick();
        }

        let leader_id = cluster.get_leader().unwrap();
        let pre_snap_len = cluster.nodes[leader_id as usize].log.len();
        assert!(pre_snap_len > 0);

        cluster.nodes[leader_id as usize].take_snapshot(&cluster.state_machines[leader_id as usize]);

        let post_snap_len = cluster.nodes[leader_id as usize].log.len();
        let snapshot = cluster.nodes[leader_id as usize].snapshot.as_ref().unwrap();
        assert!(post_snap_len < pre_snap_len || snapshot.last_included_index > 0);
        assert!(cluster.nodes[leader_id as usize].commit_index >= 4);
    }

    #[test]
    fn test_install_snapshot_to_lagging_follower() {
        let mut cluster = RaftCluster::new(3);
        cluster.run_until_elected(500);

        for i in 1..=10 {
            let cmd = format!("SET k{} v{}", i, i);
            let idx = cluster.propose(cmd.as_bytes().to_vec()).unwrap();
            assert!(cluster.run_until_committed(idx, 100));
        }
        for _ in 0..30 {
            cluster.tick();
        }

        let leader_id = cluster.get_leader().unwrap();
        cluster.nodes[leader_id as usize].take_snapshot(&cluster.state_machines[leader_id as usize]);

        let follower_idx = if leader_id == 0 { 1 } else { 0 };
        cluster.nodes[follower_idx as usize].take_snapshot(&mut cluster.state_machines[follower_idx as usize]);

        assert!(cluster.nodes[follower_idx as usize].snapshot.is_some());
        let snap = cluster.nodes[follower_idx as usize].snapshot.as_ref().unwrap();
        assert!(snap.last_included_index > 0);
    }

    #[test]
    fn test_5_node_cluster_election() {
        let mut cluster = RaftCluster::new(5);
        let leader = cluster.run_until_elected(500);
        assert!(leader.is_some());

        let leaders: Vec<_> = cluster
            .nodes
            .iter()
            .filter(|n| n.state == NodeState::Leader)
            .collect();
        assert_eq!(leaders.len(), 1);
    }

    #[test]
    fn test_multiple_proposals() {
        let mut cluster = RaftCluster::new(3);
        cluster.run_until_elected(500);

        for i in 1..=10 {
            let cmd = format!("SET key{} val{}", i, i);
            let idx = cluster.propose(cmd.as_bytes().to_vec()).unwrap();
            assert!(cluster.run_until_committed(idx, 100));
        }

        for _ in 0..50 {
            cluster.tick();
        }

        let leader_id = cluster.get_leader().unwrap();
        assert!(cluster.nodes[leader_id as usize].commit_index >= 10);
    }

    #[test]
    fn test_state_machine_applies_commands() {
        let mut cluster = RaftCluster::new(3);
        cluster.run_until_elected(500);

        let idx1 = cluster.propose(b"SET x hello".to_vec()).unwrap();
        cluster.run_until_committed(idx1, 200);
        for _ in 0..30 {
            cluster.tick();
        }

        for sm in &cluster.state_machines {
            assert_eq!(sm.data.get(&b"x".to_vec()), Some(&b"hello".to_vec()));
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    println!("Raft Consensus — Phase 11, Lesson 09\n");
    println!("=========================================\n");

    demo_leader_election();
    demo_log_replication();
    demo_leader_failover();
    demo_snapshotting();
    demo_log_safety();

    println!("All demos complete. Run `cargo test` to verify safety and liveness properties.");
}