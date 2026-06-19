# Consensus III — Raft (with a working implementation)

> If you can't explain it to a freshman, the algorithm has a design flaw — Raft is Paxos for people who want to ship.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 11 lessons 01–08 (especially Lesson 07 — Single-Decree Paxos and Lesson 08 — Multi-Paxos)
**Time:** ~120 minutes

## Learning Objectives

- Explain why Raft was designed: Paxos provides equivalent safety but is notoriously hard to understand; Raft decomposes consensus into independent subproblems.
- Describe Raft's five subproblems: leader election, log replication, safety, membership changes, and log compaction (snapshots).
- Implement leader election with terms, randomized timeouts, the RequestVote RPC, and split-vote recovery.
- Implement log replication with the AppendEntries RPC, consistency checks (prev_log_index/prev_log_term), and commit advancement based on majority replication.
- State and apply Raft's safety invariants: the election restriction (candidates must have up-to-date logs) and the log matching property.
- Explain why a leader can only commit entries from its current term directly — committing entries from previous terms requires indirect commitment through a current-term entry.
- Implement log compaction via snapshots (InstallSnapshot RPC) for followers that lag behind the leader's log truncation point.
- Build a complete Raft implementation in Rust and demonstrate leader election, log replication, failover, and snapshotting in a simulated 3-node cluster.

## The Problem

In lessons 07 and 08 you built Paxos and Multi-Paxos. They work — Google runs Paxos at planetary scale. But if you ask ten distributed-systems engineers to implement Multi-Paxos from memory, nine will produce a subtly buggy version. Lamport's own paper acknowledges the gap between the algorithm and an understandable description.

This matters because **implementation bugs in consensus are catastrophic**. A single safety violation — two nodes agreeing on different values — means data loss, downtime, or worse. When engineers can't reason clearly about the algorithm, they can't verify their implementation is correct.

Diego Ongaro and John Ousterhout set out to design an algorithm with one property Paxos lacks: **understandability**. Every design decision in Raft was evaluated by whether it made the algorithm easier to explain and implement correctly. The result is an algorithm that provides the same safety guarantee as Multi-Paxos but decomposes the problem into independent, separately verifiable pieces.

Without Raft, you'd be stuck in the same place as teams before 2014: implementing consensus correctly is possible but unnecessarily error-prone. Raft makes it *systematic*.

## The Concept

### Why Paxos Is Hard

Paxos defines consensus in terms of a single symmetric protocol where any node can propose at any time. This symmetry is elegant but forces you to reason about multiple concurrent proposers, overlapping quorums, and proposal number interactions simultaneously. The "Paxos Made Simple" paper is 14 pages long, and most readers still need multiple passes.

### Raft's Decomposition

Raft splits consensus into five subproblems, each solvable and testable in isolation:

| Subproblem | What it solves |
|---|---|
| **Leader election** | Who coordinates the cluster? |
| **Log replication** | How does the leader propagate commands? |
| **Safety** | Why can different terms never disagree? |
| **Membership changes** | How do you add or remove nodes? |
| **Log compaction** | What happens when the log grows without bound? |

Each subproblem has a clear specification. Solve them in order, test each independently, compose them at the end.

### Terms

Raft divides time into **terms**. A term is a logical clock — a monotonically increasing integer. Each term has at most one leader. Terms serve three purposes:

1. **Logical clock:** if a node sees a higher term, it immediately steps down to follower.
2. **Leader uniqueness:** at most one leader per term (enforced by election rules).
3. **Stale detection:** stale RPCs (from old terms) are rejected.

```
Term 1          Term 2          Term 3
|---A elected---|---B elected---|---C elected---|
                |               |
                A crashes       B crashes
```

### Leader Election

A node starts as a follower. If it receives no heartbeat within the **election timeout** (a randomized duration, e.g. 150–300ms), it transitions to candidate, increments its term, votes for itself, and sends `RequestVote` RPCs to all other nodes.

```
RequestVote RPC:
  Arguments:  term, candidate_id, last_log_index, last_log_term
  Results:    term, vote_granted
```

A candidate wins if it receives votes from a majority of nodes *for the same term*. Because each node votes at most once per term, at most one candidate can win per term. If the vote splits (two candidates each get 2 votes in a 5-node cluster), both time out, increment their terms, and try again. **Randomized election timeouts** make split votes unlikely: the candidate with the shorter timeout starts first and collects votes before the other wakes up.

The election restriction: a node only grants its vote if the candidate's log is *at least as up-to-date* as its own. "Up-to-date" means:

- If the last log entries have different terms, the one with the higher term is more up-to-date.
- If the last log entries have the same term, the longer log is more up-to-date.

This guarantees that the leader always has all committed entries — without it, a disconnected minority could elect a leader that doesn't know about committed data.

### Log Replication

Once elected, the leader sends `AppendEntries` RPCs to followers:

```
AppendEntries RPC:
  Arguments:  term, leader_id, prev_log_index, prev_log_term,
              entries[], leader_commit
  Results:    term, success
```

The consistency check: when a follower receives `AppendEntries`, it checks that its log contains an entry at `prev_log_index` with term `prev_log_term`. If not, it rejects the RPC. This enforces the **log matching property**: if two logs share an entry at the same index with the same term, all preceding entries are identical.

When the leader receives acknowledgments from a majority for an entry at index `i`, it advances `commit_index` to `i`. The leader then notifies followers via the `leader_commit` field in subsequent `AppendEntries`, and followers apply entries up to `commit_index` to their state machines.

### Committing Entries from Previous Terms

A critical subtle point: a leader cannot directly commit an entry from a previous term by counting replicas. Consider:

```
Time →   T1          T2          T3
Leader:  S1 entries   S1 crashes  S5 elected (doesn't have T1 entries)
```

If S5 (in a new term) overwrites the uncommitted T1 entries on some followers, the T1 entries could be lost — even though a majority had replicated them. Raft's solution: **a leader only commits entries from its current term**. Committing a current-term entry transitively commits all prior entries by the log matching property. This is Figure 8 from the Raft paper — the subtlest part of the algorithm.

### Safety Invariants

Two invariants underlie all of Raft:

1. **Election Safety:** At most one leader per term. (Enforced by the one-vote-per-term rule.)
2. **Log Matching:** If two entries in different logs have the same index and term, then they store the same command, and all preceding entries are identical. (Enforced by the AppendEntries consistency check.)

Together, these guarantee that once an entry is committed, it can never be overwritten by a future leader.

### Snapshots

A log that grows without bound eventually exhausts storage. Raft solves this with **snapshots**: the state machine serializes its current state, the log is truncated up to `last_included_index`, and the snapshot replaces the discarded entries.

```
Snapshot:
  last_included_index: 7
  last_included_term:  3
  data: <serialized state machine state>
```

When a follower has fallen behind and the leader has already truncated the entries the follower needs, the leader sends an `InstallSnapshot` RPC instead of `AppendEntries`:

```
InstallSnapshot RPC:
  Arguments:  term, leader_id, last_included_index, last_included_term,
              data, done
  Results:    term
```

The follower replaces its log with the snapshot and applies the snapshot data to its state machine.

### Membership Changes

If you change the cluster membership atomically (switch directly from {A, B, C} to {A, B, C, D, E}), there's a moment when two disjoint majorities could form — one in the old configuration, one in the new — creating a split-brain.

Raft's solution is **joint consensus** (also called C_old,new):

1. The leader proposes a special entry `C_old,new` containing both configurations.
2. While `C_old,new` is uncommitted, any decision requires a majority from *both* the old and new configurations.
3. Once committed, the leader proposes `C_new` containing only the new configuration.
4. Once `C_new` is committed, old nodes can be removed.

For **single-server changes** (add one node, or remove one node), joint consensus is unnecessary — a single-server change always preserves majority overlap between old and new configurations.

## Build It

We'll implement Raft in Rust. The code lives in `code/main.rs`. Run it with `cargo run`.

### Step 1: Terms, State, and LogEntry

Define the `RaftNode` struct with all the state Raft requires: `current_term`, `voted_for`, `log`, `commit_index`, `last_applied`, and `state` (Follower/Candidate/Leader). Define `LogEntry` with `term` and `command` (raw bytes).

### Step 2: RequestVote and Leader Election

Implement the `RequestVote` RPC handler: a node grants its vote only if the candidate's term is ≥ its own and the candidate's log is at least as up-to-date. Implement election timeout: on timeout, transition to Candidate, increment term, vote for self, broadcast RequestVote. When a majority grants votes, transition to Leader and begin sending heartbeats (empty AppendEntries).

### Step 3: AppendEntries and Log Replication

Implement the `AppendEntries` RPC: check the consistency condition (prev_log_index/prev_log_term match), truncate conflicting entries, append new entries, and advance `commit_index` when a majority has replicated entries from the leader's current term.

### Step 4: Commit Advancement and State Machine Application

The leader tracks `match_index` for each follower. Periodically, it finds the highest index `N` such that a majority of `match_index ≥ N` and `log[N].term == current_term`, and sets `commit_index = N`. Followers update their `commit_index` from `leader_commit` and apply entries to the state machine up to `commit_index`.

### Step 5: Snapshots and InstallSnapshot

When the log exceeds a threshold, take a snapshot: serialize state machine state into `data`, record `last_included_index` and `last_included_term`, truncate the log. Implement `InstallSnapshot` for followers that need entries the leader has already discarded.

### Step 6: Network Simulation and Demo

A simulated network with configurable message delivery. Run a 3-node cluster: elect a leader, replicate several entries, kill the leader, watch a new leader get elected, verify log consistency, and trigger snapshotting.

## Use It

**etcd's Raft** is the most widely deployed Raft implementation. It powers etcd (the Kubernetes backing store), CockroachDB, TiKV, and Consul. The core is in [etcd-io/raft](https://github.com/etcd-io/etcd/tree/main/server/etcdserver/raft).

Compare our implementation against etcd's:

| Our Raft | etcd's Raft |
|---|---|
| Single-threaded simulation | Fully concurrent with goroutines |
| In-memory log | Persistent WAL (write-ahead log) on disk |
| Synchronous RPC | Asynchronous pipelined RPC |
| Simple snapshot (full state) | Incremental snapshots with reader/writer interfaces |
| No readIndex protocol | Linearizable reads via readIndex/lease |
| No batching | Batching and pipeline AppendEntries for throughput |

The key production features etcd adds: **linearizable reads** (a read must see the latest committed state — implemented by having the leader confirm it's still the leader before responding), **dynamic membership changes** (the joint-consensus protocol in practice), and **WAL persistence** (every log entry is flushed to disk before it's acknowledged).

## Read the Source

- [etcd-io/raft/raft.go](https://github.com/etcd-io/etcd/blob/main/server/etcdserver/raft/raft.go) — the core Raft state machine. Look at `stepLeader`, `stepCandidate`, and `stepFollower` to see how the three states handle RPCs differently.
- [etcd-io/raft/storage.go](https://github.com/etcd-io/etcd/blob/main/server/etcdserver/raft/storage.go) — log storage and snapshot interface. Compare against our `Vec<LogEntry>`.

## Ship It

The reusable artifact lives in `outputs/`: a Rust Raft library with leader election, log replication, and snapshotting. It ships as a crate you can:

- Import in the Phase 11 capstone (lesson 22) to build a replicated KV store.
- Extend with dynamic membership changes for production use.
- Use as a reference implementation for interview or teaching purposes.

Run `cargo run` to see the 5 demos. Run `cargo test` for automated safety and liveness verification.

## Exercises

1. **Easy** — Add a 4th demo: start with a 5-node cluster, kill 2 nodes (leaving a 3-node quorum), and verify the cluster still makes progress. Then kill one more node (leaving 2 — no quorum) and verify the cluster stops making progress but resumes when one machine comes back.
2. **Medium** — Implement dynamic membership changes using joint consensus. Add a `add_node` and `remove_node` method that transitions through `C_old,new` before reaching `C_new`. Test that a 3-node cluster can safely add a 4th node.
3. **Hard** — Implement linearizable reads. A client sends a read request to the leader. The leader must confirm it's still the leader (by getting heartbeat acknowledgments from a majority) before responding. This prevents stale reads from a partitioned old leader. (This is etcd's `readIndex` protocol.)

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Term | "epoch" or "generation" | A logical time period during which at most one leader exists; incremented on each election |
| Election timeout | "heartbeat timeout" | A randomized duration (e.g. 150–300ms) after which a follower becomes a candidate; randomization prevents split votes |
| Log matching property | "logs are the same" | If two logs have an entry at the same index with the same term, all prior entries are identical — enforced by AppendEntries' prev_log check |
| Commit | "it's decided" | An entry is committed when a majority of nodes have stored it and the leader has advanced commit_index; committed entries are never lost |
| Election restriction | "only good nodes win" | A candidate must have a log at least as up-to-date as any voter's log to receive that vote — prevents a stale node from becoming leader |
| Snapshot | "log truncation" | A serialized state machine state that replaces the log up to last_included_index; needed because logs can't grow forever |
| Joint consensus | "two configs at once" | A configuration containing both old and new membership; any decision requires a majority from both configs, preventing split-brain during membership changes |

## Further Reading

- [In Search of an Understandable Consensus Algorithm](https://raft.github.io/raft.pdf) — Ongaro and Ousterhout, 2014. The Raft paper. Read sections 5–7 carefully; section 8 (cluster membership changes) and section 7 (log compaction) are essential for production use.
- [Raft Refloated](https://web.stanford.edu/~ouster/cgi-bin/papers/raft-refloated.pdf) — Howard, Malkhi, Spiegel, 2020. Clarifications and corrections to the original paper, including the "commit only from current term" rule.
- [etcd Raft documentation](https://etcd.io/docs/v3.5/learning/raft/) — The most widely deployed Raft implementation. Walk through the state diagram and message flow diagrams.
- [Kafka's KRaft mode](https://developer.confluent.io/learn/kraft/) — Apache Kafka replaced ZooKeeper with its own Raft variant (KRaft). Compare: Kafka uses the Raft log for metadata, not data, which changes the design trade-offs.
- [Raft Visualizations](https://raft.github.io/) — Animated raft scope and interactive raft simulations. Use these to develop intuition before reading the paper.