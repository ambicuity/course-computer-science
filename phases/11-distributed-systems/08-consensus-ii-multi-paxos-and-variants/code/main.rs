
#[derive(Clone, Debug, PartialEq)]
struct Proposal {
    number: u64,
    value: String,
}

#[derive(Clone, Debug)]
struct SlotState {
    promised: u64,
    accepted: Option<Proposal>,
    decided: Option<String>,
}

impl SlotState {
    fn new() -> Self {
        SlotState {
            promised: 0,
            accepted: None,
            decided: None,
        }
    }
}

#[derive(Clone, Debug)]
struct MultiPaxosNode {
    id: usize,
    is_leader: bool,
    proposal_number: u64,
    log: Vec<SlotState>,
    next_slot: usize,
}

impl MultiPaxosNode {
    fn new(id: usize) -> Self {
        MultiPaxosNode {
            id,
            is_leader: false,
            proposal_number: 0,
            log: vec![],
            next_slot: 0,
        }
    }

    fn ensure_slot(&mut self, slot: usize) {
        while self.log.len() <= slot {
            self.log.push(SlotState::new());
        }
    }

    fn decided_value(&self, slot: usize) -> Option<&String> {
        self.log.get(slot).and_then(|s| s.decided.as_ref())
    }

    fn is_slot_decided(&self, slot: usize) -> bool {
        self.log.get(slot).map_or(false, |s| s.decided.is_some())
    }
}

#[derive(Clone, Debug)]
enum Message {
    Prepare {
        slot: usize,
        proposal_number: u64,
        from: usize,
    },
    Promise {
        slot: usize,
        proposal_number: u64,
        accepted: Option<Proposal>,
        from: usize,
    },
    Accept {
        slot: usize,
        proposal_number: u64,
        value: String,
        from: usize,
    },
    Learn {
        slot: usize,
        value: String,
        from: usize,
    },
}

struct MultiPaxosCluster {
    nodes: Vec<MultiPaxosNode>,
    messages: Vec<Message>,
    quorum: usize,
}

impl MultiPaxosCluster {
    fn new(count: usize) -> Self {
        let quorum = count / 2 + 1;
        let nodes = (0..count).map(MultiPaxosNode::new).collect();
        MultiPaxosCluster {
            nodes,
            messages: vec![],
            quorum,
        }
    }

    fn elect_leader(&mut self, leader_id: usize) {
        let proposal_number = self.nodes.iter().map(|n| n.proposal_number).max().unwrap_or(0) + 1;
        self.nodes[leader_id].is_leader = true;
        self.nodes[leader_id].proposal_number = proposal_number;
        self.run_phase1_for_leadership(leader_id, proposal_number);
    }

    fn run_phase1_for_leadership(&mut self, leader_id: usize, proposal_number: u64) {
        let slot = self.find_first_undecided_slot();
        let node_count = self.nodes.len();

        self.nodes[leader_id].ensure_slot(slot);
        self.messages.push(Message::Prepare {
            slot,
            proposal_number,
            from: leader_id,
        });

        for i in 0..node_count {
            if i == leader_id {
                continue;
            }
            self.nodes[i].ensure_slot(slot);
        }

        self.process_messages();
    }

    fn find_first_undecided_slot(&self) -> usize {
        let max_log_len = self.nodes.iter().map(|n| n.log.len()).max().unwrap_or(0);
        for slot in 0..max_log_len {
            if !self.all_decided_at(slot) {
                return slot;
            }
        }
        max_log_len
    }

    fn all_decided_at(&self, slot: usize) -> bool {
        self.nodes.iter().all(|n| n.is_slot_decided(slot))
    }

    fn propose(&mut self, leader_id: usize, value: String) -> usize {
        let slot = self.find_next_available_slot(leader_id);
        self.propose_at_slot(leader_id, slot, value);
        slot
    }

    fn find_next_available_slot(&self, leader_id: usize) -> usize {
        let next = self.nodes[leader_id].next_slot;
        let max_log = self.nodes.iter().map(|n| n.log.len()).max().unwrap_or(0);
        for s in next..max_log + 10 {
            if !self.all_decided_at(s) {
                return s;
            }
        }
        next
    }

    fn propose_at_slot(&mut self, leader_id: usize, slot: usize, value: String) {
        let proposal_number = self.nodes[leader_id].proposal_number;
        self.nodes[leader_id].ensure_slot(slot);
        self.nodes[leader_id].next_slot = slot + 1;

        self.messages.push(Message::Accept {
            slot,
            proposal_number,
            value,
            from: leader_id,
        });

        self.process_messages();
    }

    fn fill_gaps(&mut self, leader_id: usize) {
        let max_log = self.nodes.iter().map(|n| n.log.len()).max().unwrap_or(0);
        for slot in 0..max_log {
            if !self.all_decided_at(slot) {
                let proposal_number = self.nodes[leader_id].proposal_number;
                self.nodes[leader_id].ensure_slot(slot);

                let existing_value = self.find_accepted_value(slot);

                let value = existing_value.unwrap_or_else(|| "<noop>".to_string());

                self.messages.push(Message::Accept {
                    slot,
                    proposal_number,
                    value,
                    from: leader_id,
                });
            }
        }
        self.process_messages();
    }

    fn find_accepted_value(&self, slot: usize) -> Option<String> {
        let mut best: Option<Proposal> = None;
        for node in &self.nodes {
            if let Some(ref state) = node.log.get(slot) {
                if let Some(ref accepted) = state.accepted {
                    match &best {
                        None => best = Some(accepted.clone()),
                        Some(current) if accepted.number > current.number => {
                            best = Some(accepted.clone());
                        }
                        _ => {}
                    }
                }
            }
        }
        best.map(|p| p.value)
    }

    fn process_messages(&mut self) {
        let mut iterations = 0;
        while !self.messages.is_empty() && iterations < 1000 {
            let messages: Vec<Message> = self.messages.drain(..).collect();
            for msg in messages {
                self.handle_message(msg);
            }
            iterations += 1;
        }
    }

    fn handle_message(&mut self, msg: Message) {
        match msg {
            Message::Prepare {
                slot,
                proposal_number,
                from,
            } => {
                for i in 0..self.nodes.len() {
                    if i != from {
                        self.nodes[i].ensure_slot(slot);
                        let state = &mut self.nodes[i].log[slot];
                        if proposal_number >= state.promised {
                            let accepted = state.accepted.clone();
                            state.promised = proposal_number;
                            self.messages.push(Message::Promise {
                                slot,
                                proposal_number,
                                accepted,
                                from: i,
                            });
                        }
                    }
                }
            }
            Message::Promise {
                slot: _slot,
                proposal_number: _proposal_number,
                accepted,
                from: _from,
            } => {
                self.nodes.iter_mut().for_each(|n| {
                    n.ensure_slot(_slot);
                });
                if let Some(accepted_prop) = accepted {
                    for node in &mut self.nodes {
                        node.ensure_slot(_slot);
                        let state = &mut node.log[_slot];
                        if state.accepted.is_none()
                            || state.accepted.as_ref().unwrap().number < accepted_prop.number
                        {
                            state.accepted = Some(accepted_prop.clone());
                        }
                    }
                }
            }
            Message::Accept {
                slot,
                proposal_number,
                value,
                from: _from,
            } => {
                for i in 0..self.nodes.len() {
                    self.nodes[i].ensure_slot(slot);
                    let state = &mut self.nodes[i].log[slot];
                    if proposal_number >= state.promised {
                        state.promised = proposal_number;
                        state.accepted = Some(Proposal {
                            number: proposal_number,
                            value: value.clone(),
                        });
                    }
                }

                let mut accept_count = 0;
                for node in &self.nodes {
                    if let Some(ref state) = node.log.get(slot) {
                        if let Some(ref accepted) = state.accepted {
                            if accepted.number == proposal_number && accepted.value == value {
                                accept_count += 1;
                            }
                        }
                    }
                }

                if accept_count >= self.quorum {
                    for i in 0..self.nodes.len() {
                        self.nodes[i].ensure_slot(slot);
                        if self.nodes[i].log[slot].decided.is_none() {
                            self.nodes[i].log[slot].decided = Some(value.clone());
                        }
                    }
                }
            }
            Message::Learn { .. } => {}
        }
    }

    fn fail_node(&mut self, node_id: usize) {
        self.nodes[node_id].is_leader = false;
    }

    fn print_log(&self) {
        println!("\n=== Replicated Log State ===");
        let max_len = self.nodes.iter().map(|n| n.log.len()).max().unwrap_or(0);
        if max_len == 0 {
            println!("  (empty)");
            return;
        }
        print!("  Slot:  ");
        for s in 0..max_len {
            print!("{:>12}", format!("slot {}", s));
        }
        println!();

        for node in &self.nodes {
            print!("  Node {}: ", node.id);
            for s in 0..max_len {
                let val = node
                    .log
                    .get(s)
                    .and_then(|st| st.decided.as_ref())
                    .map(|v| v.as_str())
                    .unwrap_or("???");
                print!("{:>12}", val);
            }
            println!();
        }
    }

    fn print_summary(&self) {
        let max_len = self.nodes.iter().map(|n| n.log.len()).max().unwrap_or(0);
        println!("\n=== Committed Log ===");
        for s in 0..max_len {
            let val = self.nodes[0]
                .log
                .get(s)
                .and_then(|st| st.decided.as_ref());
            match val {
                Some(v) => println!("  Slot {}: {}", s, v),
                None => println!("  Slot {}: (undecided)", s),
            }
        }
    }
}

fn demo_normal_operation() {
    println!("╔══════════════════════════════════════════════════╗");
    println!("║   Demo 1: Normal Multi-Paxos Operation          ║");
    println!("║   5 nodes, leader proposes 5 commands            ║");
    println!("╚══════════════════════════════════════════════════╝\n");

    let mut cluster = MultiPaxosCluster::new(5);
    println!("Cluster: 5 nodes, quorum = {}", cluster.quorum);

    println!("\nElecting node 0 as leader...");
    cluster.elect_leader(0);

    let commands = vec![
        "SET x=1".to_string(),
        "SET y=2".to_string(),
        "APPEND z=a".to_string(),
        "DELETE w".to_string(),
        "SET x=3".to_string(),
    ];

    println!("Leader proposes 5 commands (Phase 1 skipped after first):\n");
    for (_i, cmd) in commands.iter().enumerate() {
        let slot = cluster.propose(0, cmd.clone());
        println!(
            "  Propose \"{}\" → slot {} (Phase 1 skipped, single round-trip)",
            cmd, slot
        );
    }

    cluster.print_log();
    cluster.print_summary();
}

fn demo_leader_failover() {
    println!("\n╔══════════════════════════════════════════════════╗");
    println!("║   Demo 2: Leader Failover and Gap Filling      ║");
    println!("║   Leader crashes, new leader fills gaps         ║");
    println!("╚══════════════════════════════════════════════════╝\n");

    let mut cluster = MultiPaxosCluster::new(5);
    println!("Cluster: 5 nodes, quorum = {}", cluster.quorum);

    println!("\nElecting node 0 as leader...");
    cluster.elect_leader(0);

    println!("\nLeader proposes 3 commands...");
    cluster.propose(0, "SET a=1".to_string());
    cluster.propose(0, "SET b=2".to_string());
    cluster.propose(0, "SET c=3".to_string());

    println!("\nLeader (node 0) CRASHES after proposing slot 3 but before commit!");
    cluster.fail_node(0);

    println!("\nElecting node 2 as new leader (higher proposal number)...");
    cluster.elect_leader(2);

    println!("\nNew leader proposes 2 more commands...");
    cluster.propose(2, "SET d=4".to_string());
    cluster.propose(2, "SET e=5".to_string());

    println!("\nNew leader fills gaps in the log...");
    cluster.fill_gaps(2);

    cluster.print_log();
    cluster.print_summary();
}

fn demo_gap_filling() {
    println!("\n╔══════════════════════════════════════════════════╗");
    println!("║   Demo 3: Explicit Gap Detection and Repair     ║");
    println!("║   Slots with no accepted value get no-op fills  ║");
    println!("╚══════════════════════════════════════════════════╝\n");

    let mut cluster = MultiPaxosCluster::new(3);
    println!("Cluster: 3 nodes, quorum = {}", cluster.quorum);

    println!("\nElecting node 0 as leader...");
    cluster.elect_leader(0);

    println!("\nProposing to slots 0 and 2 only (skipping slot 1)...");
    cluster.propose_at_slot(0, 0, "CMD_A".to_string());
    cluster.propose_at_slot(0, 2, "CMD_C".to_string());

    println!("\nSlot 1 is a GAP — no value decided.");
    println!("Node 0 log:");
    for s in 0..3 {
        let val = cluster.nodes[0]
            .log
            .get(s)
            .and_then(|st| st.decided.as_ref())
            .map(|v| v.as_str())
            .unwrap_or("(gap)");
        println!("  Slot {}: {}", s, val);
    }

    println!("\nNew leader (node 1) takes over and fills gaps...");
    cluster.elect_leader(1);
    cluster.fill_gaps(1);

    println!("\nAfter gap filling:");
    for s in 0..3 {
        let val = cluster.nodes[1]
            .log
            .get(s)
            .and_then(|st| st.decided.as_ref())
            .map(|v| v.as_str())
            .unwrap_or("(undecided)");
        println!("  Slot {}: {}", s, val);
    }
}

fn demo_phase1_skip_comparison() {
    println!("\n╔══════════════════════════════════════════════════╗");
    println!("║   Demo 4: Phase 1 Skip Optimization             ║");
    println!("║   First proposal: 2 round-trips (Phase 1 + 2)   ║");
    println!("║   Subsequent: 1 round-trip (Phase 2 only)       ║");
    println!("╚══════════════════════════════════════════════════╝\n");

    println!("Single-decree Paxos (no optimization):");
    println!("  5 commands × 2 round-trip = 10 round-trips total");

    println!("\nMulti-Paxos with Phase 1 skip:");
    println!("  1st proposal: Phase 1 + Phase 2 = 2 round-trips");
    println!("  5th proposal: Phase 2 only     = 1 round-trip");
    println!("  Total: 2 + (5-1)×1 = 6 round-trips");
    println!("  Savings: 40% fewer round-trips for 5 proposals");
    println!("  As N grows: (N+1)/(2N) → approaches 50% savings");
}

fn demo_variant_comparison() {
    println!("\n╔══════════════════════════════════════════════════╗");
    print!("║   Multi-Paxos and Variants Comparison           ║");
    println!("╚══════════════════════════════════════════════════╝\n");

    println!("┌──────────────────┬────────────┬──────────────┬──────────┬─────────┬──────────┐");
    println!("│ Property         │ Single-Dec │ Multi-Paxos  │ Raft     │ Zab     │ EPaxos   │");
    println!("├──────────────────┼────────────┼──────────────┼──────────┼─────────┼──────────┤");
    println!("│ Decides          │ One value  │ Log sequence │ Log seq  │ Log seq │ Log seq  │");
    println!("│ Leader           │ Any proposer│ Stable leader│ Elected  │ Dedicated│ Any node │");
    println!("│ Phase 1 skip     │ N/A        │ Yes          │ Implicit │ Yes     │ N/A      │");
    println!("│ Gap fill         │ N/A        │ No-op fill   │ Auto     │ Auto    │ Dep graph│");
    println!("│ Quorum           │ Majority   │ Majority     │ Majority │ Majority│ 3/4 or 1/2│");
    println!("│ Reconfig         │ External   │ Vertical Px  │ Joint    │ N/A     │ N/A      │");
    println!("│ Latency (stable) │ 2 RTT      │ 1 RTT        │ 1 RTT    │ 1 RTT   │ 1 RTT    │");
    println!("│ Used by          │ Theory     │ Chubby/DB    │ etcd     │ ZooKpr  │ Cockroach│");
    println!("└──────────────────┴────────────┴──────────────┴──────────┴─────────┴──────────┘");
}

fn main() {
    demo_normal_operation();
    demo_leader_failover();
    demo_gap_filling();
    demo_phase1_skip_comparison();
    demo_variant_comparison();

    println!("\n✓ All demos complete.");
}