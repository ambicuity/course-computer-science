# Replication — Leader/Follower, Quorum

> One copy is no copy. Quorum is how many copies you must hear from before you believe what you read.

**Type:** Learn
**Languages:** Rust
**Prerequisites:** Phase 11 lessons 01–09
**Time:** ~75 minutes

## Learning Objectives

- Explain why replication exists: availability (survive node failures), durability (data survives disk failures), latency (read from a nearby replica).
- Compare single-leader, multi-leader, and leaderless replication architectures — their consistency guarantees, failure modes, and trade-offs.
- Distinguish synchronous, asynchronous, and semi-synchronous replication — what each sacrifices and what each guarantees.
- Derive the quorum condition R + W > N and explain why it ensures every read sees the latest write.
- Implement primary-backup replication with configurable quorum writes and quorum reads, including read-repair for stale replicas.
- Demonstrate fault tolerance: write with W=3 quorum in a 5-node cluster, kill one node, show reads still succeed.

## The Problem

You have one database server. It holds every user's data. One morning the disk fails. Every write from the last 12 hours is gone. Users are outraged. You add a second server as a backup — but now you have a new problem: when you write to the primary, when do you tell the user "done"? After the local disk write? After the backup confirms? What if the backup is slow? What if it's down?

This is the replication problem, and it's harder than it looks. Every answer trades off consistency, availability, and latency. Replication is the mechanism that makes distributed databases possible — and also the mechanism that makes them hard.

Without understanding replication modes and quorums, you cannot build the Raft-replicated KV store that is this phase's capstone. You also can't reason about why your PostgreSQL streaming replica is behind, why Cassandra returns stale data, or why DynamoDB offers "strong" and "eventually consistent" reads — those are all replication choices expressed as API parameters.

## The Concept

### Why Replicate?

Three reasons, none of which is "because distributed systems are cool":

| Goal | What breaks without replication | What replication gives you |
|------|-------------------------------|---------------------------|
| **Availability** | One node fails → entire system is down | Read and write from surviving nodes |
| **Durability** | One disk fails → data is lost | Data exists on multiple disks |
| **Latency** | Users far from the single node see slow reads | Read from the nearest replica |

Availability and durability are about **surviving failures**. Latency is about **performance**. Replication helps with all three, but each benefit comes with a consistency cost.

### Single-Leader Replication

The simplest model: one **primary** (leader) accepts all writes. The primary appends each write to its **replication log** and sends log entries to one or more **followers** (replicas). Followers apply log entries in order.

```
Client writes        Replication log
  │                     ┌──────────────┐
  ▼                     │ index │ op   │
┌────────┐    ┌─────►  │  1    │ SET x=1 │
│Primary │    │         │  2    │ SET y=3 │
│(leader)│────┤         │  3    │ SET x=2 │
└────────┘    │         └──────────────┘
              │              │         │
              ▼              ▼         ▼
         ┌─────────┐  ┌─────────┐  ┌─────────┐
         │Follower1│  │Follower2│  │Follower3│
         └─────────┘  └─────────┘  └─────────┘
```

Reads can go to the primary (strong consistency — you always see the latest write) or to followers (eventual consistency — you might see stale data).

The primary is a **single point of failure for writes**. If it goes down, no writes are possible until a new leader is elected (or the old one recovers). Leader election is covered in Lessons 07–09 (Paxos/Raft).

### Log-Based Replication

The primary writes each operation to a **write-ahead log** (WAL) — an ordered, numbered sequence of entries. Followers receive log entries in order and apply them sequentially.

```
Primary log:   [1: SET x=1] [2: SET y=3] [3: SET x=2] [4: DEL z]
                        │                         │
Follower A:   [1: SET x=1] [2: SET y=3] [3: SET x=2]  ← caught up
Follower B:   [1: SET x=1] [2: SET y=3]                ← behind
Follower C:   [1: SET x=1]                              ← far behind
```

The log index is the key to ordering. A follower at index 3 has seen entries 1–3 and is "caught up." A follower at index 1 has seen only one entry. The gap between a follower's index and the primary's index is the **replication lag**.

### Synchronous vs Asynchronous Replication

When the primary receives a write, does it wait for followers to confirm before acknowledging the client?

| Mode | Behavior | Consistency | Latency | Data loss risk |
|------|----------|-------------|---------|---------------|
| **Synchronous** | Primary waits for all followers to confirm before acking client | Strong: if primary dies, any follower has all data | High: each write pays the round-trip to every follower | None (all replicas have the data) |
| **Asynchronous** | Primary acks client immediately, sends to followers in background | Weak: followers may be behind | Low: write is fast | High: if primary dies, unreplicated writes are lost |
| **Semi-synchronous** | Primary waits for at least one follower to confirm before acking | Moderate: at least one confirmed copy exists | Medium: one round-trip, not all | Low: at least one follower has the data |

```
Synchronous:       Client ──write──► Primary ──replicate──► All Followers ──ack──► Primary ──ack──► Client
Asynchronous:      Client ──write──► Primary ──ack──► Client    (replicate in background)
Semi-synchronous:  Client ──write──► Primary ──replicate──► ≥1 Follower ──ack──► Primary ──ack──► Client
```

PostgreSQL defaults to asynchronous streaming replication. MySQL's semi-synchronous plugin waits for at least one replica ack. These are not theoretical choices — they are configuration knobs with real consequences.

### Multi-Leader Replication

What if multiple nodes accept writes? This is **multi-leader replication**. Each leader replicates its writes to all other leaders.

```
┌─────────────┐              ┌─────────────┐
│   Leader 1  │◄────────────►│   Leader 2  │
│  (DC East)  │              │  (DC West)  │
└─────────────┘              └─────────────┘
      │                            │
   ┌─────┐                    ┌─────┐
   │Rep A│                    │Rep B│
   └─────┘                    └─────┘
```

The advantage: writes are fast in each datacenter (no cross-DC latency). The problem: **conflict resolution**. If Leader 1 writes `x = 1` and Leader 2 simultaneously writes `x = 2`, both writes must be reconciled. Strategies include last-writer-wins (LWW), custom conflict handlers, and CRDTs (Lesson 12).

Multi-leader is used in multi-datacenter deployments (Cassandra, BDR for PostgreSQL, multi-master MySQL). It trades consistency for write availability.

### Leaderless Replication

In leaderless (Dynamo-style) replication, **any node can accept writes**. There is no primary. The client (or a coordinating node) sends each write to N replicas and waits for W acknowledgments. Reads contact R replicas and return the latest value.

```
Client writes x=5 with W=2:

  N1: receives x=5 ✓    (ack)
  N2: receives x=5 ✓    (ack)
  N3: receives x=5 ✗    (down)

  Client gets 2 acks → write succeeds (W=2 of N=3)
```

This is how Amazon Dynamo, Apache Cassandra, and Riak work. The key property is the **quorum**: as long as enough replicas respond, the system is available despite failures.

### Quorums: R + W > N

**The fundamental theorem of quorum replication:**

> If every read contacts R replicas and every write is confirmed by W replicas, and R + W > N, then every read will see at least one replica with the latest write.

Proof: R replicas overlap with the W replicas that saw the latest write by at least R + W − N replicas. Since R + W > N, there is at least 1 replica in the intersection that has the latest value.

```
N = 3, W = 2, R = 2  →  R + W = 4 > 3 ✓

  Write x=5 to 2 of 3 replicas:    [N1: x=5] [N2: x=5] [N3: x=old]
  Read from 2 of 3 replicas:        [N1: x=5] [N2: x=5]
  → At least one of the R replicas has x=5 ✓
```

Common quorum settings:

| Setting | R | W | N | Properties | Trade-off |
|---------|---|---|---|------------|-----------|
| Write-all | 1 | N | N | Very fast reads, slow writes, any single failure kills writes | Maximizes read availability |
| Read-all | N | 1 | N | Very fast writes, slow reads, any single failure kills reads | Maximizes write availability |
| Majority | ⌈(N+1)/2⌉ | ⌈(N+1)/2⌉ | N | Balanced, tolerates minority failures | Standard compromise |
| Dynamo-style | 2 | 2 | 3 | Reads and writes succeed if 2 of 3 nodes are up | N=3 with majority quorum |

For N = 5, majority quorum is R = 3, W = 3. The system tolerates 2 failures.

### Read-Repair and Anti-Entropy

When R replicas disagree during a read, the client sees stale data on some replicas. **Read-repair** fixes this: after returning the latest value to the client, propagate that value to the stale replicas.

```
Read from 3 of 5 replicas (R=3):
  N1: x=5  (latest)     ← value returned to client
  N3: x=5  (latest)
  N4: x=3  (stale)      ← read-repair writes x=5 to N4

Later: all 5 replicas converge to x=5.
```

**Anti-entropy** (background sync) is a separate mechanism: a background process compares replicas periodically and reconciles differences. Read-repair fixes stale data at read time; anti-entropy fixes it asynchronously. Both are needed: read-repair handles hot keys; anti-entropy handles cold keys that are never read.

```
Read-repair:    Fixes stale data when a read happens        (synchronous, per-read)
Anti-entropy:   Fixes stale data in the background           (asynchronous, periodic)
Hinted handoff: Stores writes for a down node, replays later (asynchronous, on node recovery)
```

## Build It

We'll build primary-backup replication with configurable quorum in Rust. Run it with `cargo run`.

### Step 1: Log Entry and QuorumConfig

A `LogEntry` records each write: an index, a key, a value, and a term (used for leader identification — we'll keep it simple). A `QuorumConfig` validates that R + W > N.

### Step 2: ReplicaNode — Primary and Followers

A `ReplicaNode` holds a write-ahead log and a key-value store derived from that log. The primary accepts writes, replicates log entries to followers, and waits for a quorum of acknowledgments. Followers apply log entries in order.

### Step 3: PrimaryBack — Write with W-Quorum, Read with R-Quorum

The `PrimaryBack` struct manages a cluster of N replica nodes. A write replicates to W nodes before succeeding. A read queries R nodes and returns the latest value, performing read-repair on stale replicas.

### Step 4: Fault Tolerance — Kill a Node, Read Still Works

Demonstrate that with N=5, W=3, R=3, the system tolerates one node failure: write succeeds with 3 of 5 nodes, read succeeds with 3 of 4 remaining nodes, and still sees the latest value.

### Step 5: Read-Repair Demo

Write a value, let one follower fall behind, then perform a read with R=3. Show that the stale replica is repaired.

## Use It

**etcd** uses single-leader replication with Raft (a form of synchronous replication to a quorum). Every write must be replicated to a majority before the leader acknowledges it. This gives etcd strong consistency at the cost of write latency (one round-trip to a quorum). Source: [etcd server apply](https://github.com/etcd-io/etcd/blob/main/server/etcdserver/raft.go) — the `processInternalRaftRequest` method.

**Cassandra** uses leaderless replication with configurable quorums. You choose `QUORUM`, `LOCAL_QUORUM`, `ONE`, or `ALL` per request. Write with `QUORUM` and read with `QUORUM` gives you R + W > N. Write with `ONE` and read with `ONE` does not — you may see stale data. Source: [Cassandra AbstractReplicationStrategy](https://github.com/apache/cassandra/blob/cassandra-5.0/src/java/org/apache/cassandra/locator/AbstractReplicationStrategy.java).

**PostgreSQL** streaming replication uses single-leader asynchronous replication by default, with optional synchronous mode (`synchronous_standby_names`). In synchronous mode, writes block until the specified standbys confirm — this is semi-synchronous replication. Source: [PostgreSQL walsender](https://github.com/postgres/postgres/blob/master/src/backend/replication/walsender.c).

The difference between our implementation and production: production systems handle network partitions, leader elections, log truncation, snapshotting, and membership changes. Our implementation assumes a stable primary and focuses on the quorum mechanics — the part that determines whether your reads see your writes.

## Read the Source

- [etcd/raft — raft.go](https://github.com/etcd-io/etcd/blob/main/server/etcdserver/raft.go) — the Raft state machine. Every write goes through `processInternalRaftRequest`, which replicates to a quorum before responding. This is synchronous quorum replication.
- [Cassandra — StorageProxy.java](https://github.com/apache/cassandra/blob/cassandra-5.0/src/java/org/apache/cassandra/service/StorageProxy.java) — the `apply` method sends mutations to N replicas and waits for W acks. This is leaderless quorum replication with configurable R/W.

## Ship It

The reusable artifact is a **primary-backup replication library** with configurable quorum in `outputs/`. It ships as a Rust crate you can run as a demo or import as a library:

- `cargo run` — full demo (5-node cluster, quorum writes, node failure, read-repair)
- `use replication::{QuorumConfig, PrimaryBack, ReplicaNode}` — reuse in later phases (especially the Raft capstone)

## Exercises

1. **Easy** — Run the demo. Verify that with N=5, W=3, R=3, the system tolerates one node failure and reads still return the latest value. What happens if you kill two nodes?
2. **Medium** — Implement a `multi_leader_write` method that accepts writes on any node and detects conflicts using version vectors (from Lesson 04). When two nodes write the same key concurrently, flag the conflict.
3. **Hard** — Extend `PrimaryBack` with **anti-entropy**: a background task that periodically compares each follower's log index against the primary's and sends missing entries. Add a `partition` method that isolates a follower for several writes, then heals the partition and observes anti-entropy bringing the follower up to date.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Replication | "Copying data" | Maintaining identical copies of data across multiple nodes, with a defined consistency model governing how and when copies converge |
| Primary/Leader | "The master" | The single node that accepts writes and propagates them to followers. A single point of failure for writes. |
| Synchronous replication | "Wait for everyone" | The primary waits for all (or a quorum of) followers to acknowledge before confirming the write to the client. Guarantees no data loss but increases write latency. |
| Asynchronous replication | "Fire and forget" | The primary confirms the write immediately and replicates in the background. Fast writes but data loss if the primary crashes before replication completes. |
| Quorum | "Majority vote" | Reading from R replicas and writing to W replicas where R + W > N guarantees that at least one replica in the read set has the latest write |
| Read-repair | "Fix stale reads" | Propagating the latest value to stale replicas during a read operation, fixing divergence at read time |
| Anti-entropy | "Background sync" | A background process that compares replicas and reconciles differences, fixing divergence for data that isn't actively being read |
| Semi-synchronous | "Wait for one" | The primary waits for at least one follower to acknowledge before confirming the write. A compromise between synchronous and asynchronous. |

## Further Reading

- [Designing Data-Intensive Applications](https://dataintensive.net/) — Martin Kleppmann, Chapters 5 (Replication) and 9 (Consistency and Consensus). The best single treatment of replication modes, quorums, and consistency.
- [Dynamo: Amazon's Highly Available Key-value Store](https://www.allthingsdistributed.com/files/amazon-dynamo-sosp2007.pdf) — The paper that introduced leaderless replication with tunable quorums. Sections 4–5 cover the quorum mechanics and read-repair.
- [PostgreSQL Streaming Replication](https://www.postgresql.org/docs/current/warm-standby.html) — Production single-leader asynchronous replication. The `synchronous_standby_names` parameter controls synchronous vs asynchronous behavior.
- [Cassandra Architecture](https://cassandra.apache.org/doc/latest/cassandra/architecture/overview.html) — Leaderless replication with tunable consistency. The `QUORUM`/`LOCAL_QUORUM`/`ALL` settings directly map to the R and W values discussed here.