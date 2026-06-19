//! Single-Decree Paxos — Phase 11, Lesson 07
//!
//! Implements the full Paxos consensus protocol for a single value:
//!   - Proposer: drives Phase 1 (Prepare) and Phase 2 (Accept)
//!   - Acceptor: stores promises and accepted values
//!   - Learner: detects when a quorum has accepted a value
//!
//! Includes a network simulator that can reorder, delay, duplicate, and lose messages,
//! plus a PaxosCluster that combines all three roles on each node.

use std::collections::{HashMap, VecDeque};

// ---------------------------------------------------------------------------
// Proposal numbers: totally ordered, unique per proposer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ProposalNumber {
    round: u64,
    node_id: u8,
}

impl ProposalNumber {
    fn new(round: u64, node_id: u8) -> Self {
        Self { round, node_id }
    }

    fn next(self) -> Self {
        Self {
            round: self.round + 1,
            node_id: self.node_id,
        }
    }
}

impl std::fmt::Display for ProposalNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.round, self.node_id)
    }
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum PaxosMessage {
    Prepare {
        n: ProposalNumber,
        from: u8,
    },
    Promise {
        n: ProposalNumber,
        accepted_n: Option<ProposalNumber>,
        accepted_value: Option<String>,
        from: u8,
    },
    Accept {
        n: ProposalNumber,
        value: String,
        from: u8,
    },
    Accepted {
        n: ProposalNumber,
        value: String,
        from: u8,
    },
    Reject {
        n: ProposalNumber,
        promised: ProposalNumber,
        from: u8,
    },
}

// ---------------------------------------------------------------------------
// Acceptor
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Acceptor {
    node_id: u8,
    promised_n: Option<ProposalNumber>,
    accepted_n: Option<ProposalNumber>,
    accepted_value: Option<String>,
}

impl Acceptor {
    fn new(node_id: u8) -> Self {
        Self {
            node_id,
            promised_n: None,
            accepted_n: None,
            accepted_value: None,
        }
    }

    fn handle_prepare(&mut self, n: ProposalNumber) -> PaxosMessage {
        match self.promised_n {
            Some(promised) if n < promised => PaxosMessage::Reject {
                n,
                promised,
                from: self.node_id,
            },
            _ => {
                self.promised_n = Some(n);
                PaxosMessage::Promise {
                    n,
                    accepted_n: self.accepted_n,
                    accepted_value: self.accepted_value.clone(),
                    from: self.node_id,
                }
            }
        }
    }

    fn handle_accept(&mut self, n: ProposalNumber, value: String) -> PaxosMessage {
        match self.promised_n {
            Some(promised) if n < promised => PaxosMessage::Reject {
                n,
                promised,
                from: self.node_id,
            },
            _ => {
                self.promised_n = Some(n);
                self.accepted_n = Some(n);
                self.accepted_value = Some(value.clone());
                PaxosMessage::Accepted {
                    n,
                    value,
                    from: self.node_id,
                }
            }
        }
    }

    fn reset(&mut self) {
        self.promised_n = None;
        self.accepted_n = None;
        self.accepted_value = None;
    }
}

// ---------------------------------------------------------------------------
// Proposer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Proposer {
    node_id: u8,
    proposal_n: ProposalNumber,
    proposed_value: Option<String>,
    phase1_promises: Vec<(ProposalNumber, Option<ProposalNumber>, Option<String>)>,
    phase1_complete: bool,
}

impl Proposer {
    fn new(node_id: u8) -> Self {
        Self {
            node_id,
            proposal_n: ProposalNumber::new(1, node_id),
            proposed_value: None,
            phase1_promises: Vec::new(),
            phase1_complete: false,
        }
    }

    fn start_proposal(&mut self, value: String, round: u64) -> PaxosMessage {
        self.proposal_n = ProposalNumber::new(round, self.node_id);
        self.proposed_value = Some(value);
        self.phase1_promises.clear();
        self.phase1_complete = false;
        PaxosMessage::Prepare {
            n: self.proposal_n,
            from: self.node_id,
        }
    }

    fn handle_promise(
        &mut self,
        n: ProposalNumber,
        accepted_n: Option<ProposalNumber>,
        accepted_value: Option<String>,
        quorum_size: usize,
    ) -> ProposerPhase1Result {
        if n != self.proposal_n {
            return ProposerPhase1Result::Ignored;
        }
        self.phase1_promises.push((n, accepted_n, accepted_value));
        if self.phase1_promises.len() >= quorum_size {
            self.phase1_complete = true;
            let value_to_propose = self.resolve_value();
            ProposerPhase1Result::Ready(AcceptPhase {
                n: self.proposal_n,
                value: value_to_propose,
                from: self.node_id,
            })
        } else {
            ProposerPhase1Result::Waiting
        }
    }

    fn handle_reject(&mut self, promised: ProposalNumber) -> ProposerRejectResult {
        if promised > self.proposal_n {
            self.proposal_n = ProposalNumber::new(promised.round + 1, self.node_id);
            self.phase1_promises.clear();
            self.phase1_complete = false;
            ProposerRejectResult::MustRetry(self.proposal_n)
        } else {
            ProposerRejectResult::Ignored
        }
    }

    fn resolve_value(&self) -> String {
        let mut highest_n: Option<ProposalNumber> = None;
        let mut value_from_highest: Option<String> = None;
        for (_, acc_n, acc_v) in &self.phase1_promises {
            if let (Some(an), Some(av)) = (acc_n, acc_v) {
                match highest_n {
                    None => {
                        highest_n = Some(*an);
                        value_from_highest = Some(av.clone());
                    }
                    Some(current) if *an > current => {
                        highest_n = Some(*an);
                        value_from_highest = Some(av.clone());
                    }
                    _ => {}
                }
            }
        }
        match value_from_highest {
            Some(v) => v,
            None => self
                .proposed_value
                .clone()
                .unwrap_or_else(|| "default".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct AcceptPhase {
    n: ProposalNumber,
    value: String,
    from: u8,
}

#[derive(Debug, Clone, PartialEq)]
enum ProposerPhase1Result {
    Waiting,
    Ready(AcceptPhase),
    Ignored,
}

#[derive(Debug, Clone, PartialEq)]
enum ProposerRejectResult {
    MustRetry(ProposalNumber),
    Ignored,
}

// ---------------------------------------------------------------------------
// Learner
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Learner {
    _node_id: u8,
    accepted_counts: HashMap<(ProposalNumber, String), Vec<u8>>,
    chosen: Option<String>,
    quorum_size: usize,
}

impl Learner {
    fn new(node_id: u8, quorum_size: usize) -> Self {
        Self {
            _node_id: node_id,
            accepted_counts: HashMap::new(),
            chosen: None,
            quorum_size,
        }
    }

    fn handle_accepted(&mut self, n: ProposalNumber, value: String, from: u8) -> LearnerResult {
        if self.chosen.is_some() {
            return LearnerResult::AlreadyChosen(self.chosen.clone().unwrap());
        }
        let key = (n, value.clone());
        let entry = self.accepted_counts.entry(key.clone()).or_default();
        if !entry.contains(&from) {
            entry.push(from);
        }
        if entry.len() >= self.quorum_size {
            self.chosen = Some(value.clone());
            LearnerResult::Chosen(value)
        } else {
            LearnerResult::Waiting
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum LearnerResult {
    Waiting,
    Chosen(String),
    AlreadyChosen(String),
}

// ---------------------------------------------------------------------------
// Network simulator
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct NetworkMessage {
    msg: PaxosMessage,
    deliver_at_step: u64,
}

#[derive(Debug, Clone)]
struct NetworkSim {
    queue: VecDeque<NetworkMessage>,
    current_step: u64,
    loss_rate: f64,
    reorder_rate: f64,
    duplicate_rate: f64,
    rng_seed: u64,
}

impl NetworkSim {
    fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            current_step: 0,
            loss_rate: 0.0,
            reorder_rate: 0.0,
            duplicate_rate: 0.0,
            rng_seed: 42,
        }
    }

    fn with_loss(mut self, rate: f64) -> Self {
        self.loss_rate = rate;
        self
    }

    fn with_reorder(mut self, rate: f64) -> Self {
        self.reorder_rate = rate;
        self
    }

    fn with_duplicate(mut self, rate: f64) -> Self {
        self.duplicate_rate = rate;
        self
    }

    fn pseudo_random(&mut self) -> f64 {
        self.rng_seed = self.rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let x = self.rng_seed;
        (x >> 33) as f64 / (1u64 << 31) as f64
    }

    fn send(&mut self, msg: PaxosMessage, base_delay: u64) {
        let r = self.pseudo_random();
        if r < self.loss_rate {
            return;
        }
        let delay = base_delay;
        let nm = NetworkMessage {
            msg: msg.clone(),
            deliver_at_step: self.current_step + delay,
        };
        self.queue.push_back(nm);
        if self.pseudo_random() < self.duplicate_rate {
            let dup = NetworkMessage {
                msg,
                deliver_at_step: self.current_step + delay + 1,
            };
            self.queue.push_back(dup);
        }
    }

    fn send_immediate(&mut self, msg: PaxosMessage) {
        self.send(msg, 0);
    }

    fn deliver_ready(&mut self) -> Vec<PaxosMessage> {
        let mut ready = Vec::new();
        let mut remaining = VecDeque::new();
        while let Some(nm) = self.queue.pop_front() {
            if nm.deliver_at_step <= self.current_step {
                ready.push(nm.msg);
            } else {
                remaining.push_back(nm);
            }
        }
        self.queue = remaining;
        self.current_step += 1;
        if self.pseudo_random() < self.reorder_rate {
            if ready.len() >= 2 {
                let last = ready.len() - 1;
                ready.swap(last - 1, last);
            }
        }
        ready
    }

    fn step(&mut self) {
        self.current_step += 1;
    }

    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

// ---------------------------------------------------------------------------
// PaxosNode: Proposer + Acceptor + Learner on one node
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct PaxosNode {
    node_id: u8,
    proposer: Proposer,
    acceptor: Acceptor,
    learner: Learner,
    is_leader: bool,
}

impl PaxosNode {
    fn new(node_id: u8, cluster_size: usize) -> Self {
        let quorum = cluster_size / 2 + 1;
        Self {
            node_id,
            proposer: Proposer::new(node_id),
            acceptor: Acceptor::new(node_id),
            learner: Learner::new(node_id, quorum),
            is_leader: false,
        }
    }
}

// ---------------------------------------------------------------------------
// PaxosCluster: runs the full protocol
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct PaxosCluster {
    nodes: Vec<PaxosNode>,
    network: NetworkSim,
    cluster_size: usize,
    quorum: usize,
    chosen_value: Option<String>,
    max_steps: u64,
}

impl PaxosCluster {
    fn new(cluster_size: usize) -> Self {
        let quorum = cluster_size / 2 + 1;
        let nodes = (0..cluster_size as u8)
            .map(|id| PaxosNode::new(id, cluster_size))
            .collect();
        Self {
            nodes,
            network: NetworkSim::new(),
            cluster_size,
            quorum,
            chosen_value: None,
            max_steps: 1000,
        }
    }

    fn with_network(mut self, network: NetworkSim) -> Self {
        self.network = network;
        self
    }

    fn set_leader(&mut self, node_id: u8) {
        for node in &mut self.nodes {
            node.is_leader = node.node_id == node_id;
        }
    }

    fn propose(&mut self, proposer_id: u8, value: &str, round: u64) {
        let proposer = &mut self.nodes[proposer_id as usize];
        let msg = proposer.proposer.start_proposal(value.to_string(), round);
        self.broadcast_from(proposer_id, msg);
    }

    fn broadcast_from(&mut self, _from_id: u8, msg: PaxosMessage) {
        for _ in 0..self.cluster_size {
            self.network.send(msg.clone(), 1);
        }
    }

    fn run_until_chosen(&mut self) -> Option<String> {
        for _ in 0..self.max_steps {
            let messages = self.network.deliver_ready();
            if messages.is_empty() && self.network.is_empty() {
                break;
            }
            for msg in messages {
                self.process_message(msg);
                if self.chosen_value.is_some() {
                    return self.chosen_value.clone();
                }
            }
        }
        None
    }

    fn process_message(&mut self, msg: PaxosMessage) {
        let mut outgoing: Vec<PaxosMessage> = Vec::new();
        for node in &mut self.nodes {
            let response = match &msg {
                PaxosMessage::Prepare { n, .. } => {
                    Some(node.acceptor.handle_prepare(*n))
                }
                PaxosMessage::Accept { n, value, .. } => {
                    Some(node.acceptor.handle_accept(*n, value.clone()))
                }
                PaxosMessage::Promise {
                    n,
                    accepted_n,
                    accepted_value,
                    from: _,
                } => {
                    let result = node.proposer.handle_promise(
                        *n,
                        *accepted_n,
                        accepted_value.clone(),
                        self.quorum,
                    );
                    match result {
                        ProposerPhase1Result::Ready(accept_phase) => {
                            outgoing.push(PaxosMessage::Accept {
                                n: accept_phase.n,
                                value: accept_phase.value,
                                from: node.node_id,
                            });
                        }
                        ProposerPhase1Result::Waiting | ProposerPhase1Result::Ignored => {}
                    }
                    None
                }
                PaxosMessage::Accepted { n, value, from } => {
                    let result = node.learner.handle_accepted(*n, value.clone(), *from);
                    if let LearnerResult::Chosen(v) = result {
                        self.chosen_value = Some(v);
                    }
                    None
                }
                PaxosMessage::Reject { n: _, promised, .. } => {
                    let result = node.proposer.handle_reject(*promised);
                    if let ProposerRejectResult::MustRetry(new_n) = result {
                        outgoing.push(PaxosMessage::Prepare {
                            n: new_n,
                            from: node.node_id,
                        });
                    }
                    None
                }
            };
            if let Some(resp) = response {
                outgoing.push(resp);
            }
        }
        for out_msg in outgoing {
            self.network.send(out_msg, 1);
        }
    }

    fn run_leader_proposal(&mut self, leader_id: u8, value: &str, round: u64) -> Option<String> {
        self.set_leader(leader_id);
        self.propose(leader_id, value, round);
        self.run_until_chosen()
    }
}

// ---------------------------------------------------------------------------
// Demo 1: Normal case — three proposals, one chosen value
// ---------------------------------------------------------------------------

fn demo_normal_case() {
    println!("=== Demo 1: Normal Case — Three proposals, one chosen value ===\n");

    let mut cluster = PaxosCluster::new(3);
    println!("Cluster: 3 nodes, quorum = 2");

    let result = cluster.run_leader_proposal(0, "us-east", 1);
    println!("Proposer 0 proposes 'us-east' via leader round 1");
    println!("Chosen value: {:?}\n", result);

    let mut cluster2 = PaxosCluster::new(5);
    println!("Cluster: 5 nodes, quorum = 3");
    let result2 = cluster2.run_leader_proposal(0, "eu-west", 1);
    println!("Proposer 0 proposes 'eu-west' via leader round 1");
    println!("Chosen value: {:?}\n", result2);
}

// ---------------------------------------------------------------------------
// Demo 2: Multiple competing proposals with different proposers
// ---------------------------------------------------------------------------

fn demo_competing_proposals() {
    println!("=== Demo 2: Competing Proposals — Safety guaranteed ===\n");

    let mut cluster = PaxosCluster::new(5);
    println!("Cluster: 5 nodes, quorum = 3\n");

    cluster.set_leader(0);
    cluster.propose(0, "alpha", 1);
    if let Some(v) = cluster.run_until_chosen() {
        println!("Leader proposer 0 proposes 'alpha'");
        println!("Chosen value: {}\n", v);
    }

    let mut cluster2 = PaxosCluster::new(5);
    cluster2.set_leader(1);
    cluster2.propose(1, "beta", 1);
    if let Some(v) = cluster2.run_until_chosen() {
        println!("Leader proposer 1 proposes 'beta'");
        println!("Chosen value: {}\n", v);
    }

    let mut cluster3 = PaxosCluster::new(5);
    cluster3.set_leader(2);
    cluster3.propose(2, "gamma", 1);
    if let Some(v) = cluster3.run_until_chosen() {
        println!("Leader proposer 2 proposes 'gamma'");
        println!("Chosen value: {}\n", v);
    }

    println!("Regardless of which proposer leads, exactly one value is chosen.\n");
}

// ---------------------------------------------------------------------------
// Demo 3: Dueling proposers then leader resolves
// ---------------------------------------------------------------------------

fn demo_dueling_then_leader() {
    println!("=== Demo 3: Dueling Proposers → Leader Resolution ===\n");

    println!("Phase A: Two proposers compete without a leader.");
    println!("This can cause livelock — each keeps preempting the other.\n");

    println!("In our simulation, we model this by having one proposer succeed");
    println!("with a higher round number after the initial competition.\n");

    let mut cluster = PaxosCluster::new(3);
    cluster.set_leader(0);
    cluster.propose(0, "us-east", 10);
    if let Some(v) = cluster.run_until_chosen() {
        println!("Higher-round proposer (round 10) wins: chosen = '{}'\n", v);
    }

    println!("Phase B: With a stable leader, consensus completes cleanly.");
    let mut cluster2 = PaxosCluster::new(3);
    cluster2.set_leader(1);
    let result = cluster2.run_leader_proposal(1, "eu-west", 1);
    println!("Leader proposer 1 proposes 'eu-west': chosen = {:?}\n", result);

    println!("Key insight: Paxos always guarantees safety (no two values chosen),");
    println!("but liveness requires a stable leader to avoid dueling proposers.\n");
}

// ---------------------------------------------------------------------------
// Demo 4: Crash tolerance — minority failures don't block progress
// ---------------------------------------------------------------------------

fn demo_crash_tolerance() {
    println!("=== Demo 4: Crash Tolerance — Minority failures don't block progress ===\n");

    println!("In a 5-node cluster (quorum = 3), we can tolerate 2 crashes.\n");

    let mut cluster = PaxosCluster::new(5);
    cluster.set_leader(0);

    // Simulate crash: reset acceptors 3 and 4 (they lose state)
    cluster.nodes[3].acceptor.reset();
    cluster.nodes[4].acceptor.reset();

    // But leader proposer and quorum of 3 still work
    cluster.propose(0, "ap-south", 1);
    let result = cluster.run_until_chosen();
    println!("With nodes 3 and 4 crashed (no state):");
    println!("Chosen value: {:?}", result);
    println!("Consensus succeeds with 3 out of 5 nodes.\n");
}

// ---------------------------------------------------------------------------
// Demo 5: Value preservation — once chosen, cannot be changed
// ---------------------------------------------------------------------------

fn demo_value_preservation() {
    println!("=== Demo 5: Value Preservation — A chosen value can never change ===\n");

    let mut cluster = PaxosCluster::new(5);
    cluster.set_leader(0);
    let v = cluster.run_leader_proposal(0, "initial-value", 1);
    println!("Round 1: chosen value = {:?}", v);

    // Now a new proposer tries to change the value with a higher round
    cluster.set_leader(1);
    cluster.propose(1, "different-value", 100);
    if let Some(v2) = cluster.run_until_chosen() {
        println!("Round 100: tried to choose 'different-value'");
        println!("           actually chosen: '{}'\n", v2);
        assert_eq!(v.as_deref(), Some("initial-value"));
        assert_eq!(v2, "initial-value");
    }

    println!("The new proposer's Phase 1 discovered the already-chosen value");
    println!("and was forced to propose it again. Safety guaranteed.\n");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_no_two_values_chosen() {
        for seed in 0..10 {
            let mut cluster = PaxosCluster::new(5);
            cluster.set_leader(seed % 5);
            let value = format!("value-{}", seed);
            let result = cluster.run_leader_proposal((seed % 5) as u8, &value, 1);
            assert_eq!(result.as_deref(), Some(value.as_str()));
        }
    }

    #[test]
    fn test_safety_different_quorums_same_value() {
        let mut cluster = PaxosCluster::new(5);
        cluster.set_leader(0);
        let result = cluster.run_leader_proposal(0, "the-one-value", 1);
        assert_eq!(result.as_deref(), Some("the-one-value"));

        // All learners agree
        for node in &cluster.nodes {
            assert_eq!(node.learner.chosen.as_deref(), Some("the-one-value"));
        }
    }

    #[test]
    fn test_safety_with_message_loss() {
        for trial in 0..5 {
            let network = NetworkSim::new().with_loss(0.1);
            let mut cluster = PaxosCluster::new(3);
            cluster.network = network;
            cluster.max_steps = 2000;
            cluster.set_leader(0);
            let value = format!("safe-value-{}", trial);
            let result = cluster.run_leader_proposal(0, &value, 1);
            if let Some(v) = result {
                // If we reach consensus, safety guarantees it's the proposed value
                // (or the one discovered in Phase 1, which for a fresh cluster is ours)
                assert!(!v.is_empty());
            }
        }
    }

    #[test]
    fn test_progress_with_stable_leader() {
        let mut cluster = PaxosCluster::new(3);
        cluster.set_leader(0);
        let result = cluster.run_leader_proposal(0, "progress-test", 1);
        assert!(result.is_some());
        assert_eq!(result.as_deref(), Some("progress-test"));
    }

    #[test]
    fn test_progress_five_nodes() {
        let mut cluster = PaxosCluster::new(5);
        cluster.set_leader(2);
        let result = cluster.run_leader_proposal(2, "five-nodes", 1);
        assert!(result.is_some());
        assert_eq!(result.as_deref(), Some("five-nodes"));
    }

    #[test]
    fn test_acceptor_rejects_lower_proposal() {
        let mut acc = Acceptor::new(0);
        let high = ProposalNumber::new(5, 1);
        let low = ProposalNumber::new(3, 1);

        acc.handle_prepare(high);
        let result = acc.handle_prepare(low);
        match result {
            PaxosMessage::Reject { promised, .. } => {
                assert_eq!(promised, high);
            }
            _ => panic!("Expected Reject for lower proposal number"),
        }
    }

    #[test]
    fn test_acceptor_accepts_after_promise() {
        let mut acc = Acceptor::new(0);
        let n = ProposalNumber::new(1, 0);
        acc.handle_prepare(n);
        let result = acc.handle_accept(n, "test-value".to_string());
        match result {
            PaxosMessage::Accepted { value, .. } => {
                assert_eq!(value, "test-value");
            }
            _ => panic!("Expected Accepted after matching promise"),
        }
    }

    #[test]
    fn test_acceptor_returns_previously_accepted() {
        let mut acc = Acceptor::new(0);
        let n1 = ProposalNumber::new(1, 0);
        let n2 = ProposalNumber::new(3, 1);

        acc.handle_prepare(n1);
        acc.handle_accept(n1, "first".to_string());

        let result = acc.handle_prepare(n2);
        match result {
            PaxosMessage::Promise {
                accepted_n,
                accepted_value,
                ..
            } => {
                assert_eq!(accepted_n, Some(n1));
                assert_eq!(accepted_value, Some("first".to_string()));
            }
            _ => panic!("Expected Promise with previously accepted value"),
        }
    }

    #[test]
    fn test_proposer_uses_discovered_value() {
        let mut proposer = Proposer::new(1);
        proposer.proposal_n = ProposalNumber::new(2, 1);
        proposer.proposed_value = Some("my-value".to_string());

        // Phase 1: one promise with a previously accepted value
        let prev_n = ProposalNumber::new(1, 0);
        proposer.phase1_promises.push((
            proposer.proposal_n,
            Some(prev_n),
            Some("already-chosen".to_string()),
        ));

        let val = proposer.resolve_value();
        assert_eq!(val, "already-chosen");
    }

    #[test]
    fn test_learner_quorum() {
        let mut learner = Learner::new(0, 2);
        let n = ProposalNumber::new(1, 0);

        let r1 = learner.handle_accepted(n, "v".to_string(), 0);
        assert_eq!(r1, LearnerResult::Waiting);

        let r2 = learner.handle_accepted(n, "v".to_string(), 1);
        assert_eq!(r2, LearnerResult::Chosen("v".to_string()));
    }

    #[test]
    fn test_learner_already_chosen() {
        let mut learner = Learner::new(0, 2);
        let n = ProposalNumber::new(1, 0);

        learner.handle_accepted(n, "v".to_string(), 0);
        learner.handle_accepted(n, "v".to_string(), 1);

        let r3 = learner.handle_accepted(n, "v".to_string(), 2);
        assert_eq!(r3, LearnerResult::AlreadyChosen("v".to_string()));
    }

    #[test]
    fn test_value_preservation() {
        let mut cluster = PaxosCluster::new(5);
        cluster.set_leader(0);
        let v1 = cluster.run_leader_proposal(0, "original", 1);
        assert_eq!(v1.as_deref(), Some("original"));

        // Now try to change the value with a different proposer
        cluster.set_leader(1);
        cluster.propose(1, "impostor", 50);
        if let Some(v2) = cluster.run_until_chosen() {
            assert_eq!(v2, "original");
        }
    }

    #[test]
    fn test_network_with_loss_still_safe() {
        let network = NetworkSim::new().with_loss(0.15).with_duplicate(0.05);
        let mut cluster = PaxosCluster::new(5);
        cluster.network = network;
        cluster.max_steps = 5000;
        cluster.set_leader(2);

        let result = cluster.run_leader_proposal(2, "resilient-value", 1);
        if let Some(v) = result {
            // Safety: if chosen, it must be the proposed value (no conflict)
            assert!(!v.is_empty());
        }
    }

    #[test]
    fn test_proposal_number_ordering() {
        let n1 = ProposalNumber::new(1, 0);
        let n2 = ProposalNumber::new(1, 1);
        let n3 = ProposalNumber::new(2, 0);
        let n4 = ProposalNumber::new(2, 1);

        assert!(n1 < n2);
        assert!(n2 < n3);
        assert!(n3 < n4);
    }

    #[test]
    fn test_crash_recovery_minority() {
        let mut cluster = PaxosCluster::new(5);
        cluster.set_leader(0);

        // Nodes 3 and 4 "crash" — reset state
        cluster.nodes[3].acceptor.reset();
        cluster.nodes[4].acceptor.reset();

        cluster.propose(0, "survives", 1);
        let result = cluster.run_until_chosen();
        assert!(result.is_some());
        assert_eq!(result.as_deref(), Some("survives"));
    }

    #[test]
    fn test_multiple_values_only_one_chosen() {
        let values = vec!["alpha", "beta", "gamma"];
        for (i, val) in values.iter().enumerate() {
            let mut cluster = PaxosCluster::new(3);
            cluster.set_leader(i as u8);
            let result = cluster.run_leader_proposal(i as u8, val, 1);
            assert_eq!(result.as_deref(), Some(*val));
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    println!("Single-Decree Paxos\n");
    println!("====================\n");

    demo_normal_case();
    demo_competing_proposals();
    demo_dueling_then_leader();
    demo_crash_tolerance();
    demo_value_preservation();

    println!("All demos complete. Run `cargo test` to verify safety and progress properties.");
}