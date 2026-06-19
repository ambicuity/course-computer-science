# Phase Capstone — A Raft-Replicated KV Store with Snapshotting

> You've studied the pieces — now build the machine. A replicated KV store is where Raft, state machines, snapshots, persistence, and client protocol become one system.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 11 lessons 01–21 (especially lessons 07–09 on consensus, 10–11 on replication, and 17 on service discovery)
**Time:** ~180 minutes

## Learning Objectives

- Integrate Raft consensus (leader election, log replication, commit) with a KV state machine into a single coherent system.
- Implement linearizable reads via the read index protocol — the leader confirms it's still leader before responding.
- Build a client-facing protocol with leader redirect: followers forward clients to the current leader.
- Implement snapshotting for log compaction: serialize state machine state, truncate the log, and install snapshots on lagging followers.
- Add persistence: save Raft state (term, vote, log, snapshot metadata) to disk and recover on restart.
- Implement cluster membership changes (add/remove nodes) via joint consensus.
- Add structured logging and request tracing for observability across the distributed system.

## The Problem

You built Raft in lesson 09. You built a state machine. You read about linearizability, leader election, and service discovery. But each of these was a separate lesson with a separate artifact. The real challenge — and the reason this is a capstone — is getting them all to work together in one system.

Consider what happens without integration:

- A **Raft cluster** that replicates log entries but has no client protocol — nobody can use it.
- A **KV store** that serves reads but isn't replicated — one crash and your data is gone.
- A **snapshotting system** that compacts state but can't install snapshots on followers that need them — lagging followers replay every command from the beginning of time.
- A **system with no persistence** — every restart starts from scratch.

The capstone builds a complete system where these pieces compose correctly: clients talk to any node (followers redirect to the leader), the leader proposes commands through Raft, committed entries are applied to a KV state machine, snapshots compact the log, state is persisted to disk, and every operation produces structured log entries with trace IDs for debugging.

## The Concept

### System Architecture

The system has four layers, each building on the one below:

```
Client Request
  ↓
API Layer (text protocol: SET/GET/CAS/DELETE/SNAPSHOT/STATUS)
  ├── Leader: process request, propose to Raft, respond
  └── Follower: redirect client to leader
        ↓
Raft Layer
  ├── Leader election (terms, randomized timeouts)
  ├── Log replication (AppendEntries)
  ├── Commit advancement (majority ack)
  └── Membership changes (AddNode, RemoveNode via joint consensus)
        ↓
State Machine (KV Store)
  ├── Linearizable reads (read index protocol)
  ├── Single-key transactions (CAS: compare-and-swap)
  └── Snapshot (serialize state, truncate log)
        ↓
Storage Layer
  ├── Persistent state (term, vote, log entries → disk)
  └── Snapshot files (serialized state machine state → disk)
```

### How the Pieces Fit Together

**Raft consensus (lessons 07–09)** provides the replication substrate. Every KV mutation (SET, DELETE, CAS) is a command serialized into a Raft log entry. The leader proposes it, replicates it to a majority, commits it, and only then does the state machine apply it. This guarantees that all replicas apply commands in the same order — the fundamental property that makes the KV store consistent.

**MVCC-style reads (lessons 10–12)** — in a simpler form. Our KV store is not multi-version (that would be a significant extension), but it respects the principle that reads should be consistent. Linearizable reads via the read index protocol guarantee that a read sees all committed writes.

**Client protocol (lesson 17)** — clients need to find the leader and talk to it. In our system, any node can receive a client request. If the node is a follower, it responds with a redirect telling the client which node is the leader. This is exactly how etcd and Consul handle client routing.

**Snapshotting (lesson 09, extended)** — lesson 09 introduced snapshots for log compaction. Here we extend it: the leader takes a snapshot when the log exceeds a threshold, saves it to disk, and truncates the log. When a follower needs entries the leader has already truncated, the leader sends an `InstallSnapshot` RPC.

**Persistence** — a production Raft implementation must persist term, voted_for, and log entries to stable storage before acknowledging any RPC. We simulate this with a simple file-based store. On restart, the node recovers its state and rejoins the cluster.

**Observability (lesson 21)** — every request is tagged with a trace ID. Structured log entries record what happened at each layer: `[trace_id=abc] Client SET x 1 → proposing to Raft → committed at index 5 → applied → responded OK`.

### Linearizable Reads: The Read Index Protocol

A naive implementation serves reads directly from the leader's state machine. This is wrong. Consider:

1. Node A is leader in term 3. It commits entry at index 10 (SET x 42).
2. Network partition: A is isolated from the majority.
3. Node B gets elected leader in term 4. It commits entry at index 11 (SET x 99).
4. A (still thinks it's leader) serves a read for key x → returns 42 (stale!).

The **read index protocol** prevents this:

1. The leader records its current `commit_index` as the **read index**.
2. The leader sends a heartbeat to a majority of nodes (or checks if a heartbeat was received within the election timeout).
3. The leader waits until `commit_index ≥ read_index`.
4. The leader reads from the state machine and responds.

Step 2 confirms the leader is still the legitimate leader (a partitioned leader can't get heartbeats from a majority). Step 3 ensures the read sees all entries committed before the read was initiated.

### CAS: Compare-And-Swap

A CAS operation is the simplest form of transaction: `CAS key expected new` sets `key = new` only if `key == expected` at the time the command is applied. Because the command goes through Raft, all nodes see the same sequence of operations, so CAS is inherently safe — no concurrent modification can succeed between the compare and the swap, because both happen atomically when the state machine applies the log entry.

### Snapshotting in Practice

When the leader's log exceeds a configurable threshold (e.g., 100 entries), it:

1. Serializes the state machine state to bytes.
2. Records `last_included_index` and `last_included_term`.
3. Writes the snapshot to disk.
4. Truncates all log entries before `last_included_index`.
5. On future `AppendEntries`, if a follower needs entries that were truncated, the leader sends `InstallSnapshot` instead.

On restart, the node loads the latest snapshot from disk, then replays any log entries after the snapshot point.

### Joint Consensus for Membership Changes

Adding or removing a node is itself a Raft operation — the configuration change is proposed as a special log entry. For safety, Raft uses **joint consensus** during the transition:

1. The leader proposes `C_old,new` — both old and new configurations.
2. While `C_old,new` is uncommitted, any decision requires a majority from **both** configurations.
3. Once committed, the leader proposes `C_new` — only the new configuration.
4. Once `C_new` is committed, old nodes can be shut down.

For **single-server changes**, joint consensus isn't strictly necessary because a single addition/removal always preserves majority overlap. But we implement the full joint consensus protocol because it handles the general case.

## Build It

We'll build the complete system in seven steps. The code lives in `code/main.rs`. Run it with `cargo run`.

### Step 1: Raft Consensus (Reuse and Extend from Lesson 09)

Start with the Raft implementation from lesson 09: leader election, log replication, commit advancement. We extend it with:

- A `RaftPersistentState` struct for disk persistence.
- `ConfigChange` log entries for membership changes.
- Integration hooks for the state machine and client protocol.

The core Raft types remain the same: `LogEntry`, `RequestVote`, `AppendEntries`, `InstallSnapshot`. What changes is that they're now part of a larger system.

### Step 2: KV State Machine

The state machine applies committed log entries as operations:

```
PUT key value  → insert or update key
DELETE key     → remove key (no-op if key doesn't exist)
CAS key old new → if key == old, set key = new; otherwise no-op
```

Each operation is serialized as bytes in the log entry's `command` field. The state machine deserializes and applies it.

### Step 3: Linearizable Reads

The read index protocol:

```
fn linearizable_read(key) → value:
    read_index ← leader.commit_index      // record current commit point
    send heartbeat to majority             // confirm still leader
    wait until commit_index ≥ read_index    // ensure all prior writes are applied
    return state_machine.get(key)           // now safe to read
```

In our simulation, we implement this by checking that the leader has received AckHeartbeats from a majority since the read was initiated.

### Step 4: Client Protocol

A simple text-based TCP protocol:

```
SET key value   → OK or ERROR
GET key         → value or NOT_FOUND
CAS key old new → OK (swapped) or NOT_FOUND (key missing) or MISMATCH (value differs)
DELETE key      → OK or NOT_FOUND
SNAPSHOT        → OK (triggers snapshot on leader)
STATUS          → node_id, state, term, leader_id, log_length, commit_index
```

If a client sends a request to a follower, the response is `REDIRECT leader_id`. The client reconnects to the leader.

### Step 5: Snapshotting

When `log.len() > SNAPSHOT_THRESHOLD`, the leader:

1. Takes a snapshot of the state machine.
2. Saves the snapshot to disk.
3. Truncates the log.
4. On the next heartbeat to lagging followers, sends `InstallSnapshot` if needed.

Followers receiving `InstallSnapshot` replace their state and truncate their log.

### Step 6: Cluster Management

`ADD_NODE node_id address` and `REMOVE_NODE node_id` are proposed through Raft as configuration change entries. The joint consensus protocol ensures safety during transitions.

### Step 7: Persistence and Recovery

On startup, each node:

1. Loads the latest snapshot from disk.
2. Loads persisted Raft state (term, voted_for).
3. Loads persisted log entries.
4. Joins the cluster (starts as follower, receives heartbeats or starts election).

On shutdown or periodically, each node:

1. Persists current term and voted_for.
2. Persists new log entries.
3. Persists snapshot data.

## Use It

**etcd** is the production Raft-based KV store. Compare our system:

| Our KV Store | etcd |
|---|---|
| Text protocol over TCP | gRPC protocol with protobuf |
| In-memory state machine (snapshotted to disk) | MVCC store with BoltDB backend |
| Read index via heartbeat quorum | Read index with lease-based optimization |
| Single-key CAS | Multi-key transactions (Txn) |
| File-based persistence | WAL (write-ahead log) with fsync |
| No watch/notify | Watch API (key change notifications) |
| No lease system | Leases for ephemeral keys and session management |

Other systems to compare:

- **Consul** uses Raft for its consensus protocol and has a very similar KV API. Its watch system is simpler than etcd's.
- **ZooKeeper** uses ZAB (not Raft) but provides equivalent guarantees: linearizable writes, sequential consistency for reads. Its ephemeral znodes are conceptually similar to etcd's leases.
- **CockroachDB** uses a Raft group per range of keys, demonstrating how Raft scales to many consensus groups within one system.

The key production features these systems add beyond our capstone:

1. **Multi-key transactions** — etcd's `Txn` allows conditional updates across multiple keys atomically.
2. **Watch/notify** — clients subscribe to key changes and receive events without polling.
3. **Leases** — etcd's lease mechanism allows keys to expire (used for service registration, distributed locks).
4. **MVCC storage** — every write creates a new revision; reads can be served at a specific revision for snapshot isolation.
5. **Compaction** — etcd compacts old revisions (similar to our snapshotting) and provides a revision history window.

## Read the Source

- [etcd server/etcdserver](https://github.com/etcd-io/etcd/tree/main/server/etcdserver) — the main server loop. Look at `server.go` to see how etcd wires the Raft layer to the KV store and client API.
- [etcd raft module](https://github.com/etcd-io/etcd/tree/main/server/etcdserver/raft) — the production Raft implementation. Compare the state machine design against ours.
- [etcd mvcc](https://github.com/etcd-io/etcd/tree/main/server/mvcc) — the MVCC key-value store. Compare the revision-based storage against our simple `HashMap`.

## Ship It

The reusable artifact lives in `code/`: a Rust Raft-replicated KV store with snapshotting, persistence, linearizable reads, CAS operations, and a client protocol. It ships as a binary you can:

- Run as a cluster simulation to explore leader election, failover, and snapshotting.
- Use as a reference implementation for interview or teaching purposes.
- Extend with watches, leases, or multi-key transactions for production use.

Run `cargo run` to see the capstone demos. Run `cargo test` for automated verification of safety and liveness properties.

## Exercises

1. **Easy** — Add a `WATCH key` command that notifies clients of key changes. When a key is modified, any client watching that key receives a notification (`WATCH_EVENT key new_value`). Hint: maintain a watcher list per key in the state machine, and on each apply, check if the modified key has watchers.

2. **Medium** — Implement linearizable reads via the read index protocol. Currently reads go through the state machine without confirming the leader is still valid. Add a `read_index` method to the leader that: (a) records the current commit_index, (b) confirms leadership by checking that a majority have acknowledged heartbeats since the read was initiated, (c) waits for the state machine to apply up to the read index, then returns the value. Test that a partitioned old leader does NOT serve stale reads.

3. **Hard** — Add dynamic cluster membership. Implement `ADD_NODE` and `REMOVE_NODE` commands that use joint consensus (C_old,new). Test that: (a) adding a node to a 3-node cluster works without downtime, (b) removing the leader triggers a re-election, (c) after removing a node, the cluster still achieves consensus with the remaining members. This requires modifying the quorum calculation during the transition period.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Linearizable read | "strong read" or "consistent read" | A read that returns the most recent value committed before the read began; requires the leader to confirm it's still the leader before responding |
| Read index | "read commit point" | The commit_index recorded at the start of a linearizable read; the read must wait until apply_index reaches this value to ensure it sees all prior writes |
| CAS (compare-and-swap) | "conditional update" or "test-and-set" | An atomic operation that updates a key only if its current value matches an expected value; safe in Raft because the state machine applies commands sequentially |
| Leader redirect | "proxy" or "forwarding" | A follower responds to a client request with the leader's address; the client must reconnect to the leader — followers don't proxy writes |
| Snapshot | "checkpoint" or "log compaction" | A serialized state machine that replaces all Raft log entries up to `last_included_index`; needed because logs grow without bound |
| InstallSnapshot RPC | "catch-up snapshot" or "state transfer" | The RPC a leader sends to a lagging follower whose needed log entries have been truncated; the follower replaces its entire state with the snapshot |
| Joint consensus | "two-phase config change" or "C_old,new" | A configuration containing both old and new membership; any decision requires a majority from both configurations, preventing split-brain during transitions |
| Persistent state | "stable storage" or "durable state" | Raft state (term, vote, log) that must be written to stable storage before an RPC is acknowledged; without persistence, a crash can violate safety |
| MVCC | "multi-version concurrency control" | A storage model where each write creates a new revision; reads can target a specific revision for snapshot isolation (etcd uses this) |
| Trace ID | "correlation ID" or "request ID" | A unique identifier propagated through all layers of a distributed request, enabling log correlation across nodes and layers |
| WAL | "write-ahead log" or "transaction log" | A log where writes are appended before the operation is applied; on crash, the WAL is replayed to recover lost state |
| Quorum | "majority" or "voting set" | The minimum number of nodes that must acknowledge an operation for it to be committed; in Raft, ⌊N/2⌋+1 for an N-node cluster |

## Further Reading

- [In Search of an Understandable Consensus Algorithm](https://raft.github.io/raft.pdf) — Ongaro and Ousterhout, 2014. The Raft paper. Sections 6 (log compaction) and 8 (cluster membership changes) are directly relevant to this capstone.
- [etcd Documentation: Internals](https://etcd.io/docs/v3.5/learning/design/) — Walk through etcd's architecture: Raft layer, MVCC store, gRPC API. Compare each layer against our implementation.
- [Lin et al., "Linearizability vs Serializability"](https://www.cs.cmu.edu/~fp/courses/15411-f13/lectures/03-linearizability.pdf) — The classic distinction. Our read index protocol provides linearizability for reads; serializability would require multi-key transactions.
- [Raft Refloated](https://web.stanford.edu/~ouster/cgi-bin/papers/raft-refloated.pdf) — Clarifications to the original paper, including the "commit only from current term" rule and linearizable reads.
- [ZooKeeper: Wait-free coordination for distributed systems](https://www.usenix.org/legacy/event/usenix10/tech/full_papers/Hunt.pdf) — The original ZooKeeper paper. Compare ZAB vs Raft and see how a production system handles the same problems we solve.