# Eventual Consistency & Read-Repair

> Given no new updates, eventually all replicas converge — but "eventually" has no time bound.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 11 lessons 01–10 (especially lesson 04 on vector clocks and lesson 10 on replication/quorum)
**Time:** ~60 minutes

## Learning Objectives

- Define eventual consistency precisely: if no new updates are made, all replicas will eventually converge to the same state, with no guaranteed time bound.
- Classify workloads where eventual consistency is acceptable (feeds, DNS, caches) versus unacceptable (banking, inventory, config).
- Explain Dynamo-style versioning: vector clocks track causality, concurrent writes produce siblings, and conflict resolution happens at read time.
- Implement synchronous read-repair: on read, detect stale replicas and update them before returning results.
- Implement anti-entropy via Merkle trees: background process that finds divergent keys in O(log N) comparisons and syncs them.
- Implement hinted handoff: temporary write delegation when a replica is down, with delivery on recovery.
- Configure tunable consistency per operation using R and W quorum settings and reason about the trade-offs.

## The Problem

You run a key-value store replicated across three nodes. Node A receives a write for `key="user:42"` with value `{name: "Alice"}`. The write succeeds on A and B, but C is temporarily down. When C comes back, a read returns stale data — or no data at all. Two clients now see different answers for the same key. Without a repair mechanism, this divergence is permanent.

This is the fundamental challenge of eventually consistent systems: **replicas drift apart, and you need explicit mechanisms to bring them back together.** The Dynamo paper (2007) solved this with three repair strategies — read-repair, anti-entropy, and hinted handoff — and a tunable consistency model (R/W quorums) that lets callers decide how strict each operation should be.

Without understanding these mechanisms, you cannot reason about convergence guarantees, cannot build systems that heal from partitions, and cannot choose appropriate consistency levels for different operations in the same application.

## The Concept

### Eventual Consistency

**Definition:** A system is eventually consistent if, given no new updates, eventually all replicas converge to the same value. The key properties:

1. **Convergence** — replicas eventually agree.
2. **No time bound** — "eventually" could be milliseconds or hours.
3. **Availability** — reads and writes succeed even during partitions.
4. **Divergence** — concurrent writes can produce different values on different replicas until convergence.

```
          Partition begins          Partition heals
               │                        │
    A: x=1    ┤    x=2            x=2   │
    B: x=1    ┤    x=3            x=3   │ ← conflict!
    C: x=1    ┤    (down)         x=1   │ ← stale!
               │                        │
               ├──────── partition ──────┤
               │                        │
                                        └──→ read-repair / anti-entropy
                                             converges to x=2 or x=3
```

### When Is It Acceptable?

| Use case | Acceptable? | Why |
|----------|-------------|-----|
| Social media feeds | Yes | Stale posts are fine; users refresh |
| Product recommendations | Yes | Slightly different recs don't matter |
| DNS | Yes | TTL ensures eventual convergence; already designed for it |
| Cache invalidation | Yes | Stale cache entries expire |
| Banking balances | **No** | You cannot overdraw because a replica was stale |
| Inventory counts | **No** | Overselling is real money lost |
| Configuration management | **No** | Wrong config can take down production |

Rule of thumb: if the cost of a stale read is financial loss or safety risk, eventual consistency is the wrong model.

### Dynamo-Style Versioning

Amazon's Dynamo uses **vector clocks** (Lesson 04) to track causality per key:

```
Client writes x=v1 → Replica A
  → VC: {A:1}

Client writes x=v2 → Replica B (didn't see v1, or concurrent)
  → VC: {B:1}

Replica C reads x:
  → Gets v1 with VC={A:1} AND v2 with VC={B:1}
  → VCs are incomparable (concurrent) → SIBLINGS
  → Return both to the client for reconciliation
```

When vector clocks are **incomparable** (`a || b`), both versions are **siblings** — neither caused the other. The system doesn't silently drop one. Instead, it returns both and the client (or a merge function) resolves the conflict.

### Read-Repair

On every read, the coordinator contacts R replicas. If it finds divergent versions (siblings) or stale values, it pushes the most up-to-date version to the stale replicas.

**Synchronous read-repair:**
```
1. Client reads key K from R replicas
2. Replica 1 has VC={A:2}, value=v1
3. Replica 2 has VC={A:2,B:1}, value=v2  (newer)
4. Coordinator: "Replica 1 is stale" → update Replica 1 with v2
5. Return v2 to client
```

**Asynchronous read-repair:**
```
1. Client reads key K from R replicas
2. Find divergent versions
3. Return the best version to client immediately
4. Fire-and-forget update to stale replicas in the background
```

Synchronous is slower but guarantees the next read sees the repaired value. Asynchronous is faster but the stale value may survive until the background update completes.

### Anti-Entropy with Merkle Trees

Read-repair only fixes keys that are *read*. What about keys nobody reads? **Anti-entropy** is a background process that compares all data between replicas.

The naive approach — compare every key — is O(N) per comparison. Merkle trees bring this down to O(log N):

```
                root hash
               /        \
          hash_0        hash_1
         /      \      /      \
     hash_00 hash_01 hash_10 hash_11
       |       |      |       |
     K=A     K=B     K=C     K=D

If root hashes match → subtrees are identical → no sync needed.
If different → recurse down to find divergent leaves.
```

A **Merkle tree** is a hash tree where:
- **Leaves** are hashes of individual key-value pairs: `H(K || V)`.
- **Internal nodes** are hashes of their children: `H(left_child || right_child)`.
- **Root hash** uniquely identifies the entire dataset.

To compare two replicas:
1. Compare root hashes. If equal, done — no divergence.
2. If different, compare left and right child hashes (O(2) comparisons instead of O(N/2)).
3. Recurse until you find the divergent leaf keys. Sync only those.

For N keys, this requires O(log N) comparisons instead of O(N).

### Hinted Handoff

When a replica is down, writes intended for it are temporarily stored on another node (the "hint target"). When the down replica recovers, the hint target delivers the writes.

```
Replication factor N=3: replicas A, B, C for key K.

1. Write K=v1 with W=2
2. C is down
3. A writes K → success
4. B writes K → success
5. D (a neighboring node) stores a "hint" for C: "when C comes back, give it K=v1"
6. C recovers → D hands off the hint → C now has K=v1
```

Hinted handoff ensures availability during transient failures while maintaining the replication factor. The hints are not durable — if D also goes down before C recovers, the hints stored on D are lost. This is why hinted handoff is a *best-effort* mechanism that complements read-repair and anti-entropy.

### Tunable Consistency (R/W Quorums)

Dynamo-style systems let you choose per-operation trade-offs:

- **N** — replication factor (how many replicas store the key)
- **W** — write quorum (how many replicas must acknowledge a write)
- **R** — read quorum (how many replicas must respond to a read)

| Setting | Behavior | Guarantee |
|---------|----------|-----------|
| R=1, W=1 | Fast reads and writes | May see stale data |
| R=N/2+1, W=N/2+1 | Quorum reads and writes | Strong consistency (if used together) |
| R=N, W=N | All replicas must agree | Maximum consistency, minimum availability |

The math: if `R + W > N`, then any read quorum overlaps with any write quorum, guaranteeing you see at least one up-to-date value. This is the **quorum intersection** property.

For N=3, `R=2, W=2` gives quorum intersection: every set of 2 nodes overlaps with every other set of 2 nodes.

```
Write quorum = {A, B}   Read quorum = {B, C}
                      ↑ B is in both → B has the latest write
```

## Build It

We'll build a complete eventually consistent key-value store simulator in Python with vector clocks, read-repair, Merkle tree anti-entropy, and hinted handoff. Run it with `python3 main.py`.

### Step 1: Versioned Value and Vector Clocks

Each key stores a `VersionedValue` — a value paired with a vector clock. We reuse the vector clock logic from Lesson 04: `increment`, `merge`, and comparison (`<`, `>`, `||`, `==`).

### Step 2: Replica and DynamoCluster

A `Replica` stores key → list of versioned values (multiple values = siblings from concurrent writes). A `DynamoCluster` manages N replicas with configurable R and W per operation.

`put(key, value, W)` writes to at least W replicas, each extending the vector clock. `get(key, R)` reads from R replicas, collecting all siblings.

### Step 3: Read-Repair

On read, detect stale or missing replicas and synchronously push the most up-to-date version(s) to them. This is read-repair — the read path becomes a healing mechanism.

### Step 4: Merkle Tree Anti-Entropy

Each replica computes a Merkle tree over its keys. The anti-entropy process compares root hashes between replica pairs. If they differ, it drills down to find divergent keys and syncs them.

### Step 5: Hinted Handoff

When a replica is marked "down," writes targeting it are stored as hints on another replica. When the down replica recovers, hints are delivered.

### Step 6: Partition and Healing Demo

Write with W=2, create a partition, see divergence, heal the partition, and show convergence through read-repair and anti-entropy.

## Use It

**Apache Cassandra** (descended from Dynamo) uses all three repair mechanisms:

- **Read-repair**: On every read, `ReadRepairStrategy` compares digests (Merkle-tree-like hashes) of the requested data across replicas. If they differ, it issues background mutations to stale replicas (asynchronous by default, configurable to synchronous).
- **Anti-entropy**: The `nodetool repair` command triggers a full Merkle tree comparison between replicas. It's how you ensure convergence of data that nobody reads.
- **Hinted handoff**: When a replica is down, the coordinator stores hints locally (in `hints/` directory) and delivers them when the target comes back. `max_hint_window` controls how long hints are kept.

Source: `StorageService.java` and `HintsService.java` in the Cassandra codebase.

**Riak** (another Dynamo descendant) exposes tunable consistency directly in the API:
- `r=1, w=1` — fast, eventually consistent
- `r=quorum, w=quorum` — Dynamo-style quorum (default)
- `r=all, w=all` —strong consistency (but no availability during partitions)

Compare to our implementation: our Merkle tree uses a simple binary tree with per-key leaf hashes, which is a simplified version of Cassandra's Merkle tree over token ranges. Production systems partition the key space into ranges and compute per-range Merkle trees, reducing the comparison granularity.

## Read the Source

- [Cassandra AntiEntropyService.java](https://github.com/apache/cassandra/blob/cassandra-5.0/src/java/org/apache/cassandra/service/AntiEntropyService.java) — the Merkle tree comparison and repair process. Look at `getDifference()` for how it walks mismatched subtrees.
- [Riak riak_kv_vnode.erl](https://github.com/basho/riak_kv/blob/develop/src/riak_kv_vnode.erl) — per-key vector clock handling and sibling management in a Dynamo-style store.
- [Dynamo paper](https://www.allthingsdistributed.com/files/amazon-dynamo-sosp2007.pdf) — Section 4.5 (read-repair), Section 4.6 (anti-entropy with Merkle trees), Section 4.7 (hinted handoff). The original source for all three mechanisms.

## Ship It

The reusable artifact is an **eventual consistency simulator** in `outputs/`. It ships as a single Python module:

- `python3 main.py` — full demo with partition, divergence, and convergence
- `from main import Replica, DynamoCluster, MerkleTree, AntiEntropy, HintedHandoff` — reuse in later phases

## Exercises

1. **Easy** — Create a 3-replica DynamoCluster. Write with W=2. Read with R=1 from one replica, then R=3. Show that R=1 can return stale data while R=3 triggers read-repair and returns the latest value.
2. **Medium** — Add asynchronous read-repair mode to the cluster. In this mode, reads return immediately without repairing stale replicas. Compare convergence time (measured in number of operations) between synchronous and asynchronous modes.
3. **Hard** — Implement a "last-writer-wins" conflict resolution strategy using physical timestamps attached to each VersionedValue. Compare its behavior against sibling-based resolution: construct a scenario where LWW silently drops a write that sibling-based resolution would preserve.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Eventual consistency | "It's weak consistency" | A guarantee that replicas converge given no new updates, with no time bound on convergence — not a guarantee of staleness, but an absence of guaranteed recency |
| Siblings | "Conflicting values" | Multiple valid versions of a key whose vector clocks are incomparable — neither caused the other, so both are legitimate |
| Read-repair | "Fixing data on reads" | Detecting stale or missing data during a read operation and pushing updates to replicas that need them — reads heal the system |
| Anti-entropy | "Background sync" | A Merkle-tree-driven background process that compares replicas and syncs divergent keys — fixes keys that nobody reads |
| Merkle tree | "A hash tree" | A binary tree where leaves hash individual key-value pairs and internal nodes hash their children; O(log N) comparison to find divergent keys |
| Hinted handoff | "Write forwarding" | Storing writes destined for a down replica on a healthy neighbor; delivering them when the target recovers — best-effort, not durable |
| Tunable consistency | "R and W knobs" | Per-operation quorum settings where R=read quorum, W=write quorum, N=replication factor; R+W>N guarantees you see the latest write |
| Quorum intersection | "Overlapping sets" | Any read quorum and write quorum share at least one node when R+W>N, guaranteeing at least one up-to-date value in the read result |

## Further Reading

- [Dynamo: Amazon's Highly Available Key-value Store](https://www.allthingsdistributed.com/files/amazon-dynamo-sosp2007.pdf) — The original paper. Sections 4.5–4.7 cover read-repair, anti-entropy, and hinted handoff.
- [Cassandra Architecture: Repair](https://cassandra.apache.org/doc/latest/cassandra/architecture/repair.html) — How Apache Cassandra implements Merkle tree-based anti-entropy repair.
- [Eventually Consistent — Werner Vogels](https://www.allthingsdistributed.com/2008/12/eventually_consistent.html) — Amazon CTO's clarification of eventual consistency: convergence, staleness windows, and when it's appropriate.
- [Merkle Trees in Distributed Systems](https://en.wikipedia.org/wiki/Merkle_tree) — The general technique used by Git, BitTorrent, Cassandra, and others for efficient set reconciliation.