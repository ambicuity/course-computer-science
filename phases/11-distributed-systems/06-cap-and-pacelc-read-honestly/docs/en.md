# CAP and PACELC — Read Honestly

> During a partition you choose; the rest of the time you compromise.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 11 lessons 01–05
**Time:** ~60 minutes

## Learning Objectives

- State the CAP theorem precisely: during a network partition, a system must choose between consistency and availability — you never "pick two of three."
- Explain PACELC: extend CAP to the non-partitioned case by making the latency-vs-consistency trade-off explicit.
- Classify real systems as CP or AP and explain what each choice sacrifices during a partition.
- Distinguish linearizability, sequential consistency, and eventual consistency — and identify which you get when you choose A over C.
- Argue why "CAP is an excuse to throw away consistency" is wrong: most systems operate in the "else" branch (E) most of the time.
- Run a CAP trade-off simulator that demonstrates CP rejection vs AP divergence under partition, and PACELC latency-vs-consistency trade-offs during normal operation.

## The Problem

You deploy a three-node database across two data centers. The network between DCs drops for 30 seconds. What happens?

If you reject all writes to the minority partition, your system is unavailable to anyone connected to that side — but the data stays consistent. If you accept writes on both sides, your system stays available — but you now have conflicting data that must be reconciled.

This isn't a bug. It's a theorem. Eric Brewer conjectured it in 2000, and Gilbert and Lynch proved it in 2002: **during a network partition, you must sacrifice either consistency or availability.** There is no third option.

The problem is that most people stop there. They read "pick two of three" and conclude that since partitions are rare, they can just "pick AP" and move on. But partitions *are* going to happen (Fallacy #1), and the choice you make during a partition isn't the only choice that matters. Daniel Abadi's PACELC extension in 2010 makes this explicit: even when there's *no* partition, you're still making a trade-off — latency versus consistency. That's the trade-off that matters 99.9% of the time.

This lesson gives you the honest reading of CAP and PACELC so you never hide behind "eventual consistency is fine because CAP."

## The Concept

### CAP: The Precise Statement

The CAP theorem says that in a distributed system subject to network partitions:

- **Consistency (C):** Every read returns the most recent write or an error. This is linearizability — there exists a total order on all operations that matches real-time order.
- **Availability (A):** Every request to a non-failing node receives a non-error response (no timeouts, no "unavailable" errors).
- **Partition tolerance (P):** The system continues to operate despite network partitions.

The theorem proves you can have **at most two** of these simultaneously. But here's the honest reading: since network partitions *are* going to happen in any real system, P is mandatory. So the real choice is:

```
During a partition:
  ┌──────────────────────────────────────────┐
  │  CP: reject requests (sacrifice A)       │
  │  AP: serve stale data (sacrifice C)       │
  │  CA: impossible (denies P — untenable)    │
  └──────────────────────────────────────────┘
```

A "CA" system only works if the network never partitions. That never happens.

### What CAP Actually Says (and Doesn't Say)

CAP constrains behavior **only during a partition.** When the network is healthy, you can have both C and A. The theorem is silent about what happens when there's no partition.

Common misreadings:

| What people say | What CAP actually says |
|-----------------|----------------------|
| "You must pick two of C, A, P globally" | During a partition, you must sacrifice C or A. When there's no partition, you can have both. |
| "AP means eventual consistency is fine" | AP means you serve *some* response during a partition. The *quality* of that response (stale, wrong) is a separate concern. |
| "CP means strong consistency always" | CP means you reject requests during a partition. When there's no partition, you still choose your consistency level. |
| "CAP means I don't need to think about consistency" | CAP only constrains a tiny fraction of real-world operation time. |

### PACELC: Extending CAP to Normal Operation

Abadi (2010) extends CAP to ask: what about the 99.9% of the time when there's no partition?

```
PACELC:
  If there's a Partition: choose Availability or Consistency
  Else (normal operation): choose Latency or Consistency
```

Written as a diagram:

```
                    ┌─────────────────┐
                    │  Partition (P)?  │
                    └────────┬────────┘
                       Yes   │    No
                    ┌────────┴────────────────┐
                    │                         │
            Choose A or C              Choose L or C
         (availability vs          (latency vs
          consistency)              consistency)
```

This explains why systems that are "AP" under CAP still have real consistency bounds:

| System | Partition | Normal |
|--------|-----------|--------|
| MongoDB | PA (available) | EL (low latency) |
| Cassandra | PA (available) | EL (low latency) |
| Riak | PA (available) | EL (low latency) |
| HBase | PC (consistent) | EC (strong consistency) |
| etcd / ZooKeeper | PC (consistent) | EC (strong consistency) |

### Consistency Models: What You Get When You Choose A

When you sacrifice C for A during a partition (or L for C during normal operation), you get a weaker consistency model:

```
Strong ←————————————————————————————————→ Weak

Linearizability → Sequential → Causal → Eventual
```

- **Linearizability:** Every read sees the most recent write. Total order matches real time. This is what CP systems provide (when not partitioned).
- **Sequential consistency:** Operations appear in some total order that respects each client's program order, but reads may see stale data if they're not the latest write in real time.
- **Causal consistency:** Operations that are causally related appear in order. Concurrent writes can be seen in different orders by different clients.
- **Eventual consistency:** If no new writes happen, eventually all replicas converge. No guarantees about when, or how long stale reads can last.

AP systems typically provide eventual consistency or causal consistency during partitions. The gap between these models and linearizability is where bugs live.

### Why CAP Is Not a License to Throw Away Consistency

The critical insight: **partitions are rare.** Most of the time, the network is fine. So the choice that matters most is not P→A or P→C — it's E→L or E→C.

If you choose E→L (latency over consistency), every read might return stale data, even when nothing is wrong. You've made your normal-operation behavior worse for a benefit you only need during a rare partition.

Systems like etcd and ZooKeeper choose E→C (strong consistency during normal operation) and P→C (reject during partition). They're unavailable during partitions, but always consistent. Systems like Cassandra choose E→L and P→A — they're always available, but you must reason about stale reads at all times, not just during partitions.

There is no universally right choice. But claiming "CAP says I have to give up consistency" is wrong. CAP says you have to give up consistency *during a partition*. PACELC reminds you that you're also giving up consistency *during normal operation* if you choose latency — and that's a choice, not a theorem.

## Build It

We'll build a CAP trade-off simulator and PACELC explorer in Python. Run it with `python3 main.py`.

### Step 1: CAPSimulator — A 3-Node Cluster

The simulator models a 3-node cluster with a key-value store. In CP mode, the system rejects writes to the minority partition. In AP mode, it accepts writes on both sides, producing divergence.

### Step 2: Partition and Divergence

We introduce a network partition that splits the cluster into a majority (2 nodes) and minority (1 node). In CP mode, the minority node rejects all requests. In AP mode, both sides accept writes.

### Step 3: Healing and Reconciliation

When the partition heals, CP nodes have consistent data but lost availability. AP nodes have conflicting data that must be reconciled — we model last-write-wins and vector-clock reconciliation.

### Step 4: PACELCExplorer

Given a system configuration (PA/EL, PC/EC, etc.), show what happens during normal operation and during a partition, including latency measurements for reads with different consistency levels.

### Step 5: 5-Node Cluster Demo

Simulate a 5-node cluster: introduce a partition (3 vs 2), observe CP vs AP behavior, heal the partition, and observe reconciliation.

## Use It

**etcd** (CP/EC) and **Cassandra** (PA/EL) embody opposite ends of PACELC:

- etcd uses Raft for consensus. During a partition, the minority rejects all writes (CP). During normal operation, every write must be replicated to a quorum before acknowledgment — higher latency, but linearizable reads. Source: [etcd server apply](https://github.com/etcd-io/etcd/blob/main/server/etcdserver/raft.go) — the `processInternalRaftRequest` method forces every write through the Raft log.

- Cassandra uses tunable consistency. During a partition, each side serves reads from local replicas (PA). During normal operation, you choose `LOCAL_QUORUM` (low latency, eventual consistency across DCs) or `QUORUM` (higher latency, stronger consistency). Source: [Cassandra StorageProxy](https://github.com/apache/cassandra/blob/cassandra-5.0/src/java/org/apache/cassandra/service/StorageProxy.java) — the `apply` method handles consistency level per-request.

Compare: etcd's Raft log adds one round-trip per write (EC choice in E). Cassandra's eventual replication lets you read local replicas immediately (EL choice in E). The simulator in this lesson demonstrates exactly this trade-off.

**DynamoDB** offers both modes: `strong consistency` reads (EC) cost more and are slower; `eventually consistent` reads (EL) are faster but may be stale. That's PACELC in a single product's API.

## Read the Source

- [etcd/raft — raft.go](https://github.com/etcd-io/etcd/blob/main/server/etcdserver/raft.go) — the Raft state machine. During a partition, the leader on the minority side steps down; all writes to minority nodes are rejected (CP behavior).
- [Apache Cassandra — StorageProxy.java](https://github.com/apache/cassandra/blob/cassandra-5.0/src/java/org/apache/cassandra/service/StorageProxy.java) — the `apply` and `read` methods accept a `ConsistencyLevel` parameter. This is PACELC in action: `LOCAL_ONE` = EL, `QUORUM` = EC, `ALL` = EC with worst latency.

## Ship It

The reusable artifact is a **CAP and PACELC trade-off simulator** in `outputs/`. It ships as a single `main.py` that you can import as a library or run as a script:

- `python3 main.py` — full demo
- `from main import CAPSimulator, PACELCExplorer` — reuse in later phases (especially lesson 11 on eventual consistency and lesson 22 on the Raft KV capstone)

## Exercises

1. **Easy** — Run the simulator. Observe CP vs AP behavior during a partition. What data does each mode have after the partition heals?
2. **Medium** — Modify the PACELCExplorer to add a "PA/EC" system (available during partition, consistent during normal operation). What real system matches this profile? Hint: consider a system that relaxes consistency only during partitions.
3. **Hard** — Extend the CAPSimulator to support *tunable consistency* per request (like Cassandra's `ONE`, `QUORUM`, `ALL`). Implement a `read(consistency_level)` method that returns stale data for `ONE`, waits for quorum for `QUORUM`, and waits for all replicas for `ALL`. Measure latency for each.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| CAP theorem | "Pick two of C, A, P" | During a partition, you must sacrifice consistency or availability. P is mandatory, so the real choice is CP or AP. |
| Partition tolerance | "The system handles network splits" | The system continues operating during a network partition — it doesn't crash or pause entirely. All real systems must have this. |
| Linearizability | "Strong consistency" | Every read sees the most recent write. There exists a total order on all operations matching real time. |
| Eventual consistency | "It converges eventually" | If no new writes occur, all replicas will eventually return the same value. No bound on how long "eventually" is. |
| PACELC | "CAP extended" | During a Partition, choose Availability or Consistency; Else, choose Latency or Consistency. Makes the normal-operation trade-off explicit. |
| Split-brain | "Both sides think they're primary" | Two partitions both accept writes independently without coordination, producing conflicting state. |
| Tunable consistency | "Pick your own consistency level" | Per-request choice of how many replicas must acknowledge (ONE, QUORUM, ALL) — trading latency for consistency. |

## Further Reading

- [Brewer's Conjecture and the Feasibility of Consistent, Available, Fault-Tolerant Web Services](https://groups.csail.mit.edu/tds/papers/Gilbert/Brewer2.pdf) — Gilbert and Lynch's 2002 proof of CAP.
- [CAP Twelve Years Later: How the "Rules" Have Changed](https://www.infoq.com/articles/cap-twelve-years-later-how-the-rules-have-changed/) — Eric Brewer's 2012 reflection: the choice is only during partitions, and most systems operate in the "else" branch.
- [Consistency Tradeoffs in Modern Distributed Database System Design](https://www.cs.umd.edu/~abadi/papers/abadi-pacelc.pdf) — Daniel Abadi's 2010 PACELC paper, extending CAP to the normal-operation case.
- [Designing Data-Intensive Applications](https://dataintensive.net/) — Martin Kleppmann's book, Chapters 5 (Replication) and 9 (Consistency and Consensus), the most thorough treatment of consistency models.