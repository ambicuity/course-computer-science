# Consensus II — Multi-Paxos and Variants

> One value is a curiosity. A log of values is a database.

**Type:** Learn
**Languages:** Rust
**Prerequisites:** Phase 11 lessons 01–07 (especially Lesson 07 — Single-Decree Paxos)
**Time:** ~75 minutes

## Learning Objectives

- Explain why single-decree Paxos is insufficient for replicated state machines and how Multi-Paxos extends it to decide a *sequence* of values.
- Describe the Phase 1 skip optimization: a stable leader runs Phase 1 once and then proposes directly into subsequent slots.
- Implement a replicated log where each slot is an independent Paxos instance, including gap detection and fill by a new leader.
- Compare Paxos variants (Cheap, Fast, Egalitarian, Vertical) and their trade-offs.
- Map Multi-Paxos concepts to real systems: Chubby, Spanner, and the engineering lessons from *Paxos Made Live*.

## The Problem

In Lesson 07 you built single-decree Paxos. It reaches agreement on *one* value. A distributed database doesn't need one value — it needs a *log*, an ordered sequence of thousands or millions of decisions. Each client command (SET x=5, APPEND y="hello") must be assigned a position in a total order, and every replica must agree on every position.

If you try to naïvely run separate Paxos instances per command, two problems appear immediately:

1. **Latency.** Each instance requires Phase 1 (prepare/promise) and Phase 2 (accept/learn) — two round-trips per command. On a WAN with 100 ms RTT, that's 200 ms per operation. Unusable.
2. **Gaps.** If a leader crashes after Phase 2 of slot 5 but before slot 4 finishes, slot 4 is empty. A new leader can't execute slot 6 before slot 4 — the state machine requires sequential application.

Real systems solve both: stable leaders skip Phase 1 after the first proposal, and new leaders scan for and fill gaps before advancing the log tail. This lesson builds that system.

## The Concept

### From One Decision to Many

Single-decree Paxos decides one `(proposal_number, value)` pair. Multi-Paxos runs one such instance per **slot** (also called **instance**):

```
Slot 0: SET x=1
Slot 1: SET y=2
Slot 2: APPEND z="a"
Slot 3: (empty — leader crashed)
Slot 4: DELETE w
```

Each slot is an independent Paxos machine with its own set of promises and accepted values. The replicated log is the sequence of decided values across all slots, applied in order.

### The Phase 1 Skip (Leader Optimization)

Phase 1 serves one purpose: discover any previously accepted value and establish that no higher-numbered proposal exists. Once a proposer wins Phase 1 and becomes the *distinguished leader*, it knows it has the highest proposal number. For every *subsequent* slot, it can skip Phase 1 and go straight to Phase 2:

```
First proposal (full Paxos):
  Leader → Acceptors:  Prepare(n)
  Acceptors → Leader:   Promise(n, accepted_value)
  Leader → Acceptors:  Accept(n, slot=0, value)
  Acceptors → Leader:   Accepted

Subsequent proposals (Multi-Paxos, Phase 1 skipped):
  Leader → Acceptors:  Accept(n, slot=1, value)
  Acceptors → Leader:   Accepted
  Leader → Acceptors:  Accept(n, slot=2, value)
  Acceptors → Leader:   Accepted
```

Message count drops from 2 round-trips per command to 1. Over a WAN, this is the difference between 200 ms and 100 ms per operation.

### Log Gaps and Recovery

When a leader crashes mid-stream, slots may be empty:

```
Slot 0: SET x=1   (committed)
Slot 1: SET y=2   (committed)
Slot 2: ???        (leader crashed during Phase 2 — maybe accepted on 1 of 3 acceptors)
Slot 3: SET z=3   (maybe accepted on 2 of 3, but not committed)
Slot 4: ???        (never proposed)
```

A new leader must:
1. Run Phase 1 for *all* undecided slots (starting from the first gap).
2. Phase 1 reveals any values already accepted in those slots.
3. Re-propose those values (Paxos guarantees they survive).
4. Fill truly empty slots with no-op commands.
5. Once all slots up to the log tail are committed, resume normal operation.

### Comparison: Paxos Family and Relatives

| Property | Single-Decree Paxos | Multi-Paxos | Raft | Zab | EPaxos |
|---|---|---|---|---|---|
| Decides | One value | Log sequence | Log sequence | Log sequence | Log sequence |
| Leader | Any proposer | Distinguished (stable) | Elected term leader | Dedicated leader | Any replica |
| Phase 1 skip | N/A | Yes (stable leader) | Implicit (term system) | Yes | N/A (no phases) |
| Gap handling | N/A | Fill with no-ops | Commit prev entry first | Commit prev first | Dependency graph |
| Quorum | Majority | Majority | Majority | Majority | Fast path: 3/4 or slow: majority |
| Reconfiguration | External | Vertical Paxos variant | Joint consensus | Not specified | Not specified |
| Key strength | Theoretical foundation | General, flexible | Understandable | Simple for ZK order | Low-latency reads anywhere |

### Paxos Variants

**Cheap Paxos:** Reduce acceptor count below majority by using stable storage and a *auxiliary* acceptor that is only activated when a primary acceptor fails. Saves machines but adds recovery complexity.

**Fast Paxos:** The leader can propose a *fast* proposal number such that any quorum of 2F+1 acceptors (out of 2F+1 total — essentially all) can decide without Phase 1. Larger quorum requirement means more messages, but zero-round-trip latency for the fast path. Falls back to Classic Paxos on collision.

**Egalitarian Paxos (EPaxos):** No distinguished leader. Any replica can propose a command. Commands that commute (no dependency) commit in one round trip. Commands that conflict require a second round trip to resolve ordering. Optimizes for the common case where most commands are independent, spreading load evenly across replicas.

**Vertical Paxos:** Allows reconfiguration (changing the set of acceptors) *without stopping* the system. Each slot can be decided by a different configuration. A special configuration-change command installs a new configuration for all future slots.

### Paxos Made Live

Google's 2007 paper *Paxos Made Live* documents the gap between the Paxos algorithm and a production system. Key engineering challenges:

- **Disk persistence:** Acceptors must write promises and accepted values to stable storage before responding. Performance requires group commit.
- **Snapshotting:** The log grows without bound. Periodically, the system snapshots the state machine and truncates the log.
- **Group membership:** Nodes join and leave. The system must reconfigure without losing quorum or blocking progress.
- **Master failover:** detecting a dead leader, electing a new one, and recovering in-flight state requires careful timeouts and heuristics.
- **Correctness traps:** subtle bugs in implementation (e.g., logging a value before it's committed, or mishandling a promise that arrives late) cause data loss.

The paper is worth reading in full — it's the best documentation of the distance between an algorithm and a deployed system.

## Build It

We build a Multi-Paxos simulation in Rust. The simulation runs in a single thread for determinism, but models the same message-passing protocol a real cluster uses.

### Step 1: Single-Decree Paxos Instance per Slot

Each slot holds its own Paxos state: promised proposal number, accepted proposal and value, and whether the value is decided.

```rust
use std::collections::HashMap;

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
```

### Step 2: Multi-Paxos Node with Leader Optimization

Each node is a proposer, acceptor, and learner for all slots. A stable leader tracks its proposal number and skips Phase 1 for subsequent proposals.

```rust
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
}
```

### Step 3: Message-Passing Simulation

We model the full protocol with explicit messages: `Prepare`, `Promise`, `Accept`, and `Learn`. The cluster processes messages in deterministic order.

```rust
#[derive(Clone, Debug)]
enum Message {
    Prepare { slot: usize, proposal_number: u64, from: usize },
    Promise { slot: usize, proposal_number: u64, accepted: Option<Proposal>, from: usize },
    Accept { slot: usize, proposal_number: u64, value: String, from: usize },
    Learn { slot: usize, value: String, from: usize },
}
```

### Step 4: Full Cluster with Gap Filling and Leader Failover

The `MultiPaxosCluster` manages 5 nodes, processes messages, handles leader failover, and fills gaps. See the complete implementation in `code/main.rs`.

## Use It

**etcd (Raft):** etcd's Raft implementation mirrors Multi-Paxos's leader optimization. The leader sends `AppendEntries` (equivalent to Accept) without a preceding election round for every log entry. etcd's `raft.go` at [github.com/etcd-io/etcd/tree/main/server/etcdserver/raft](https://github.com/etcd-io/etcd) shows this in action.

**Google Chubby:** Google's Chubby lock service runs Multi-Paxos internally. The Chubby paper notes that a stable master can propose thousands of values per second because Phase 1 is amortized across all proposals.

**CockroachDB:** Uses a Multi-Paxos variant where each range (shard of data) runs its own Paxos group. Each range has a lease holder that acts as the distinguished proposer, skipping Phase 1 for the lease duration.

Production systems add what our simulation omits: persistent storage (write-ahead log on disk), batching (grouping multiple commands into one Accept round), and pipelining (sending Accept messages for slots N, N+1, N+2 before N's response returns).

## Read the Source

- `etcd/server/etcdserver/raft/raft.go` — Raft's core log replication loop. Compare the `appendEntry` path to Multi-Paxos's Phase 2: same single-round-trip optimization under a stable leader.
- `cockroachdb/pkg/kv/kvserver/replica_proposal.go` — CockroachDB's proposal path. The `propose` function submits commands to the replicated log via Paxos, with the lease holder acting as the distinguished proposer.

## Ship It

The reusable artifact is in `outputs/`: a Rust library implementing Multi-Paxos with leader optimization and gap filling, ready to reuse in Lesson 09 (Raft) and the phase capstone.

## Exercises

1. **Easy** — Run the simulation with 3 nodes instead of 5. What's the minimum quorum size? Verify that the simulation still commits values correctly.
2. **Medium** — Add a `noop` fill strategy for gaps: when the new leader finds an empty slot, propose a special `"<noop>"` value. Show that no-ops preserve the state machine guarantee (all replicas see the same sequence).
3. **Hard** — Implement Fast Paxos: allow a leader to use a quorum of all acceptors (2F+1 out of 2F+1) to decide a value without Phase 1. On collision, fall back to Classic Paxos. Compare message counts.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Slot (instance) | "position in the log" | An independent Paxos state machine that decides exactly one value, identified by a sequence number |
| Phase 1 skip | "leader optimization" | A stable leader already has the highest proposal number, so it can skip prepare/promise and go straight to accept for new slots |
| Distinguished leader | "the leader" | The sole proposer allowed to propose in Multi-Paxos; chosen by winning Phase 1 on some slot |
| Log gap | "hole in the log" | A slot with no decided value, typically caused by a leader crash during Phase 2; must be filled before later slots can be applied |
| No-op fill | "filling holes" | Proposing a no-operation command in an empty slot so the log sequence remains contiguous |
| Fast Paxos | "zero-round-trip Paxos" | A variant where any quorum of all acceptors can decide a value without a leader round, at the cost of requiring larger quorums |
| EPaxos | "leaderless Paxos" | Egalitarian Paxos where any replica proposes; commutative commands commit in one round trip, conflicting commands require a second |

## Further Reading

- [Paxos Made Live — An Engineering Perspective](https://research.google.com/archive/paxos_made_live.pdf) (Chandra, Griesemer, Redstone, 2007) — The gap between the algorithm and the deployed system
- [Paxos Made Moderate](https://lamport.azurewebsites.net/tla/paxos-Moderate.pdf) (Lamport) — A more practical description of Multi-Paxos
- [EPaxos: Making Every Replica Count](https://www.cs.cmu.edu/~dga/papers/epaxos-sosp2013.pdf) (Moraru et al., 2013) — Leaderless Paxos for low-latency wide-area replication
- [Fast Paxos](https://lamport.azurewebsites.net/tla/fastpaxos.pdf) (Lamport, 2006) — The theoretical foundation for avoiding Phase 1 entirely
- [Vertical Paxos](https://lamport.azurewebsites.net/tla/verticalaxos.pdf) (Lamport, 2011) — Reconfiguration without stopping