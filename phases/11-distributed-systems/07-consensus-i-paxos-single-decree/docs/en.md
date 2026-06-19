# Consensus I — Paxos (Single-Decree)

> A chosen value can never be undone — Paxos makes that guarantee through two phases and overlapping quorums.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 11 lessons 01–06
**Time:** ~90 minutes

## Learning Objectives

- State the consensus problem: multiple nodes must agree on a single value despite crashes and message loss.
- Identify the three Paxos roles (Proposer, Acceptor, Learner) and explain how a node plays all three in practice.
- Walk through Phase 1 (Prepare) and Phase 2 (Accept) with concrete message traces for a 3-node cluster.
- Prove informally why overlapping quorums guarantee safety: if any value is chosen, no future quorum can choose a different value.
- Diagnose the dueling proposers liveness problem and explain why a distinguished proposer (leader) solves it.
- Implement single-decree Paxos in Rust and verify safety and progress through automated tests.

## The Problem

You have three replicas of a configuration database. A client wants to set the primary cluster to `"us-east"`. Two clients fire requests at the same time — one proposes `"us-east"`, the other proposes `"eu-west"`. Without consensus, the replicas can diverge: two vote for `"us-east"`, two for `"eu-west"`, and you have split-brain.

This is the **consensus problem**: get a group of nodes to agree on exactly one value, even when messages can be lost, reordered, or delayed, and nodes can crash and restart. You need consensus for:

- **Replicated state machines** — every replica must execute the same commands in the same order.
- **Configuration management** — which node is the leader? Which partition is primary?
- **Leader election** — which node gets the right to coordinate?

In Phase 11 lesson 10 you saw quorum replication. Quorums guarantee that *some* subset agrees, but they don't guarantee that *all* subsets agree on the *same* value. Paxos does.

## The Concept

### The Three Roles

Paxos separates consensus into three logical roles:

| Role | Responsibility |
|------|---------------|
| **Proposer** | Picks a proposal number and a value. Drives the consensus protocol. |
| **Acceptor** | Votes on proposals. Stores promises and accepted values. The memory of the system. |
| **Learner** | Discovers which value was chosen by collecting accepted responses from a quorum. |

In real systems, a single node plays all three roles. The separation is logical, not physical.

### Proposal Numbers

Every proposal has a unique number. If two proposers exist, their numbers must never collide. The standard trick: encode the proposer's node ID in the lower bits.

```
proposal_number = (round << 16) | node_id
```

Higher proposal numbers take priority. This is how Paxos resolves conflicts: the later proposal wins the right to propose, but it must preserve any value already chosen.

### Phase 1: Prepare

The proposer wants to get the cluster to agree on a value. Before proposing, it checks whether any value has already been chosen.

```
Proposer → all Acceptors:  PREPARE(n)
Acceptor → Proposer:       PROMISE(n, accepted_n, accepted_value)  or  REJECT(n)
```

An acceptor promises: "I will never again accept a proposal numbered less than n." If the acceptor has already accepted a value (from a previous round), it tells the proposer what that value was.

**If a quorum of acceptors promise**, Phase 1 succeeds. The proposer now knows the highest-numbered previously accepted value (if any). It **must** use that value in Phase 2 — not its own — because that value may already be chosen.

### Phase 2: Accept

The proposer sends the value to all acceptors:

```
Proposer → all Acceptors:  ACCEPT(n, value)
Acceptor → Proposer:       ACCEPTED(n)  or  REJECT(n)
```

An acceptor accepts if `n >= its promised_n`. If it accepted an earlier proposal, it already told the proposer about it in Phase 1, and the proposer used that value — so the acceptor's state stays consistent.

**If a quorum of acceptors accept**, the value is **chosen**. The learner role discovers this and can act on it.

### Quorum Overlap Guarantees Safety

A **quorum** is any majority of acceptors. For 5 acceptors, a quorum is 3. For 3, it's 2.

```
Quorum A:  A1  A2  A3          Quorum B:       A3  A4  A5
                ↑ overlap ↑
```

Any two quorums overlap by at least one acceptor. That acceptor is the witness that connects the two rounds:

1. If a value was chosen by quorum Q1, at least one acceptor in Q1 has `accepted_value = V` and `accepted_n = N`.
2. If a new proposer runs Phase 1 with `n' > N` and reaches quorum Q2, at least one acceptor in Q2 ∩ Q1 will report `accepted_value = V` with `accepted_n = N`.
3. The proposer must use `V`. The higher proposal number cannot change an already-chosen value.

This is the **safety guarantee**: a chosen value can never be overwritten.

### The Safety Proof (Sketch)

Formally: if value V is chosen at proposal number N, then for any proposal number N' > N, the proposer will propose V (not some other V').

- V was chosen by a quorum Q1, so every acceptor in Q1 accepted (N, V).
- The new proposer reaches quorum Q2 in Phase 1. Since Q1 ∩ Q2 ≠ ∅, at least one acceptor in the intersection reports (accepted_n, accepted_value) = some pair (n_i, v_i) where n_i ≥ N.
- The proposer takes the highest accepted_n from all Phase 1 responses. Since at least one response has accepted_n ≥ N, the highest accepted_n is ≥ N, and its value is V (because V was chosen at N, and no value can be chosen between N and the highest accepted_n without going through the same process).
- Therefore the proposer uses V. Safety holds.

### Liveness: The Dueling Proposers Problem

Paxos guarantees safety but not liveness. Consider two proposers P1 and P2:

```
P1: PREPARE(1) → quorum promises 1
P2: PREPARE(2) → quorum promises 2 (overriding P1's promise)
P1: ACCEPT(1, V1) → REJECTED (acceptors promised ≥ 2)
P1: PREPARE(3) → quorum promises 3 (overriding P2's promise)
P2: ACCEPT(2, V2) → REJECTED (acceptors promised ≥ 3)
P2: PREPARE(4) → quorum promises 4
...forever
```

Neither proposer can get Phase 2 accepted because the other keeps preempting with a higher number. This is the **dueling proposers** problem.

**Solution:** elect a **distinguished proposer** (a leader). Only one proposer runs at a time. Leader election itself needs consensus (or at least a stable leader), but in practice we use a simple timeout-based approach: if you haven't heard from the leader in a while, try to become leader yourself. This breaks the cycle because eventually one proposer holds the floor long enough to complete both phases.

### Paxos in Practice

| System | Consensus Algorithm | Notes |
|--------|-------------------|-------|
| Google Chubby | Paxos | Original production use; Chubby is a lock service |
| etcd | Raft | Simpler to understand than Paxos; same safety guarantee |
| ZooKeeper | Zab (Zookeeper Atomic Broadcast) | Similar to Multi-Paxos; optimized for primary-backup |
| Spanner | Paxos | Used for replicated state machine per shard |

Raft (lesson 09) was designed to be "Paxos for mortals" — equivalent safety, easier to understand. But Paxos remains important because: (1) it's the theoretical foundation, (2) some production systems still use it, and (3) understanding Paxos makes Raft's design choices clearer.

## Build It

We'll implement single-decree Paxos in Rust. The code is in `code/main.rs` — run it with `cargo run`.

### Step 1: Messages and Roles

Define the message types (`Prepare`, `Promise`, `Accept`, `Accepted`) and the three roles (Acceptor, Proposer, Learner). Each message carries a proposal number and optionally a value.

### Step 2: Acceptor

The Acceptor maintains two pieces of persistent state: `promised_n` (the highest proposal number it has promised to honor) and `accepted_n` / `accepted_value` (the proposal it has accepted, if any). On a `Prepare(n)`, it promises not to accept anything < n and returns any previously accepted value. On an `Accept(n, v)`, it accepts if n ≥ promised_n.

### Step 3: Proposer

The Proposer generates unique proposal numbers using `(round, node_id)`, runs Phase 1 to discover any previously chosen value, then runs Phase 2 to get a quorum to accept. If Phase 1 reveals a previously accepted value, the proposer must use it — not its own proposed value.

### Step 4: Learner

The Learner collects `Accepted` responses from acceptors. Once it sees the same (n, value) from a quorum, it declares that value chosen.

### Step 5: Network Simulation

A simulated network that can reorder, delay, duplicate, and drop messages. This lets us test Paxos under adverse conditions — the exact conditions it was designed to handle.

### Step 6: PaxosCluster

Combine Proposer + Acceptor + Learner on each node. Run consensus across 3 or 5 nodes. Demos:

1. **Normal case:** three proposals compete, only one value is chosen.
2. **Dueling proposers:** show livelock when two proposers alternate.
3. **Leader-based resolution:** elect a leader, observe that consensus completes.

### Step 7: Safety and Progress Tests

Automated tests that:
- Verify safety: no two learners ever choose different values, regardless of message ordering.
- Verify progress: with a stable leader, consensus completes within a bounded number of rounds.
- Verify crash recovery: an acceptor that crashes and restarts (but retains its persistent state) still participates correctly.

## Use It

**Google Chubby** was the first major production deployment of Paxos. Chubby provides a distributed lock service used by Bigtable, GFS, and other Google infrastructure. Every Chubby cell runs Paxos to agree on the current master.

The key insight: Chubby doesn't run a new round of Paxos for every lock acquisition. Instead, it uses Paxos once to elect a master, and then the master handles all requests until it fails. This is essentially Multi-Paxos (lesson 08) — amortize the cost of Phase 1 across many decisions.

Compare our implementation:

| Our Paxos | Chubby's Paxos |
|-----------|---------------|
| Single decree (one value) | Multi-decree (log of values) |
| Simulated network | Real TCP with timeouts |
| No leader election | Built-in leader via lease |
| No log compaction | Snapshots + log truncation |

Chubby's production implementation adds these layers atop the same Phase 1 / Phase 2 core you just built.

## Read the Source

- [etcd/raft — raft.go](https://github.com/etcd-io/etcd/blob/main/server/etcdserver/raft.go) — the Raft state machine. Compare the simplicity of Raft's leader-based approach against Paxos's symmetric approach. Both guarantee the same safety property.

## Ship It

The reusable artifact is a **single-decree Paxos library** in `outputs/`. It ships as a Rust crate that you can:

- Import in lesson 08 (Multi-Paxos) to extend to multiple decrees.
- Import in lesson 09 (Raft) to compare safety properties.
- Use as a reference implementation for interview or teaching purposes.

Run with `cargo run` to see the three demos (normal, dueling, leader-based). Run `cargo test` for automated safety and progress verification.

## Exercises

1. **Easy** — Add a 4th demo: show that a single crashed acceptor (minority in a 5-node cluster) does not prevent consensus from completing.
2. **Medium** — Modify `NetworkSim` to introduce a network partition that splits the cluster into two groups. Show that the minority partition cannot choose a value, but the majority partition can. Show that when the partition heals, the minority acceptors adopt the majority's chosen value.
3. **Hard** — Implement a simple leader election on top of Paxos: use Paxos to agree on which node is the leader. Then use that leader to drive consensus on a second value. This is the bridge from single-decree to Multi-Paxos.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Consensus | "They all agree" | A quorum of nodes agree on exactly one value, and that value can never change once chosen |
| Quorum | "Majority vote" | More than half of acceptors — any two quorums overlap by at least one member, which is the safety guarantee |
| Prepare | "Phase 1" | The proposer asks acceptors to promise not to accept older proposals and to report any value they've already accepted |
| Accept | "Phase 2" | The proposer asks acceptors to accept a specific (n, value) pair — they accept only if n is still current |
| Chosen | "It's decided" | A value is chosen when a quorum of acceptors have accepted it — this is irrevocable |
| Dueling proposers | "Paxos is slow" | Two proposers keep preempting each other's Phase 1, preventing Phase 2 from ever completing — a liveness problem, not a safety problem |
| Distinguished proposer | "The leader" | A single node that has exclusive right to propose — breaks livelock by ensuring Phase 2 completes |
| Proposal number | "Sequence number" | A totally ordered identifier that ensures every proposal can be compared — higher numbers take priority |

## Further Reading

- [The Part-Time Parliament](https://lamport.azurewebsites.net/pubs/lamport-paxos.pdf) — Lamport's original 1990 paper (published 1998). Written as a fictional parliamentary procedure on the island of Paxos. Brilliant and opaque.
- [Paxos Made Simple](https://lamport.azurewebsites.net/pubs/paxos-simple.pdf) — Lamport's 2001 re-explanation in plain English. Start here, not with the original.
- [Paxos Made Live — Google's experience](https://www.cs.utexas.edu/~lorenzo/corsi/cs380d/papers/p226-chandra.pdf) — Chandra, Griesemer, Redstone, 2007. What you learn when you actually deploy Paxos.
- [Raft: In Search of an Understandable Consensus Algorithm](https://raft.github.io/raft.pdf) — Ongaro and Ousterhout, 2014. The motivation for Raft was "Paxos is hard to understand." Read this after you understand Paxos to appreciate the design differences.