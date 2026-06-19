# Vector Clocks and Causal Order

> Lamport clocks tell you "a happened before b" is *possible* — vector clocks tell you whether it's *true*.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 11 lessons 01–03
**Time:** ~75 minutes

## Learning Objectives

- Explain why `LC(a) < LC(b)` does **not** imply `a → b`, and why this matters for distributed systems.
- Implement a vector clock with increment, merge, and full comparison semantics (<, >, concurrent, equal).
- Describe the causal ordering guarantee and implement CBCAST (causal broadcast) that delays delivery until all causally preceding messages arrive.
- Detect concurrent updates (conflicts) by comparing incomparable vector clocks, and enumerate resolution strategies (last-writer-wins, application merge, CRDTs).
- Explain dotted version vectors and why they fix causality tracking in distributed storage.
- Distinguish Lamport clocks (total order that *refines* happens-before) from vector clocks (partial order that *captures* happens-before).

## The Problem

Three replicas store the same key `"balance"`. Replica A writes `balance = 200` and sends the update to B and C. Replica C independently writes `balance = 300`. Both updates arrive at B. Which one should B keep?

If you answered "the later one" — how does B know which came later? Physical clocks drift. Network delays reorder messages. Replica A's write might arrive at B *after* C's, even though A's happened first. Lamport timestamps don't help either: `LC(200-write) < LC(300-write)` doesn't mean the 200-write *happened before* the 300-write. They could be concurrent.

Without a way to capture **causality** — not just ordering — you cannot detect conflicts, you cannot enforce delivery order, and you cannot build eventually consistent storage that converges. Vector clocks solve exactly this.

## The Concept

### Lamport Clocks: The Gap

Recall from Lesson 03: a Lamport clock assigns each event a single integer. The rule is simple — on receiving a message, take `max(local, received) + 1`. This gives a **total order** that respects happens-before:

```
a → b  ⟹  LC(a) < LC(b)
```

But the converse is **false**:

```
LC(a) < LC(b)  ⟹  ??? (maybe a → b, maybe not)
```

Two events can have LC values 3 and 5, but be entirely independent. Lamport clocks compress all causal information into a single counter — you lose the *who knew what* that distinguishes "A's event 3 caused B's event 5" from "A and C did unrelated things that happened to get timestamps 3 and 5."

### Vector Clocks: Per-Process Counters

A **vector clock** (VC) gives each process its own counter. For N processes, a VC is a vector `[v₁, v₂, …, vₙ]` where `vᵢ` counts events that process `i` has observed (including indirectly via messages).

```
Process A [0,0,0]  ─── local event ──→  [1,0,0]  ─── send ──→  [2,0,0]
                                                                 │
                                                              msg carries [2,0,0]
                                                                 │
                                                                 ▼
Process B [0,0,0]  ─── receive ──→  max([0,1,0], [2,0,0]) + inc own = [2,2,0]
Process C [0,0,0]  ─── local ──→  [0,0,1]   ← concurrent with A's [2,0,0]!
```

The update rules:

| Event type | Action |
|-----------|--------|
| Local event | Increment own counter: `VC[i] += 1` |
| Send message | Increment own counter, attach VC to the message |
| Receive message | Element-wise max of local VC and message VC, then increment own counter |

### Comparison Semantics

Given two VCs `a` and `b`:

```
a < b      iff  ∀i: a[i] ≤ b[i]  AND  ∃j: a[j] < b[j]
a > b      iff  b < a
a || b     iff  ∃i: a[i] > b[i]  AND  ∃j: a[j] > b[j]   (incomparable → concurrent)
a == b     iff  ∀i: a[i] == b[i]
```

The critical property:

```
a → b  iff  VC(a) < VC(b)
```

This is the **converse** that Lamport clocks lack. Now `VC(a) < VC(b)` **does** imply `a → b`, and `VC(a) || VC(b)` implies `a ∥ b` (concurrent — no causal order between them).

### Causal Ordering and CBCAST

**Causal broadcast** (CBCAST) guarantees: if message `m₁ → m₂`, then every process delivers `m₁` before `m₂`. Out-of-order messages go into a **delay buffer** until their causal predecessors arrive.

```
P1 sends m1 with VC=[1,0,0]    P2 sends m2 with VC=[1,1,0] (m2 depends on m1)
P3 receives m2 first (network reordering)
  → m2's VC says P1 should have delivered 1 message, but P3 has only seen 0 from P1
  → m2 goes into the delay buffer
P3 then receives m1
  → m1's VC=[1,0,0] — all dependencies satisfied (0 from P1 so far, now this is the 1st)
  → deliver m1, increment P1's counter in local VC to 1
  → check buffer: m2 now satisfies dependencies (P1 ≥ 1 ✓, P2 ≥ 0 ✓)
  → deliver m2
```

### Dotted Version Vectors

In distributed storage (Amazon Dynamo, Riak), replicas handle updates and replicas diverge. A naive vector clock approach loses information about *which replica* made a update when that replica's counter is merged.

**Dotted version vectors** (DVVs) add a "dot" — a `(replica_id, counter)` pair — on top of the base vector. The dot tracks the specific event that created the current version, while the vector tracks causal context. This prevents a well-known bug where two updates from the same replica look identical after a merge.

```
Regular VC:   [2, 3, 1]     — which of the 2 events from P0 is this? unclear.
DVV:  dot=(P0, 2), vc=[2, 3, 0]  — this is specifically P0's 2nd event, context includes P1 up to 3.
```

### Conflict Detection and Resolution

When two versions have incomparable VCs (`a || b`), they are **concurrent** — neither caused the other. This is a **conflict**. Resolution strategies:

| Strategy | How it works | Trade-off |
|----------|-------------|-----------|
| Last-writer-wins (LWW) | Attach a physical timestamp; higher timestamp wins | Simpler, but data loss if the "later" write was based on stale state |
| Application-specific merge | Domain logic merges both (e.g., set union, counter increment) | Correct but requires per-key-type logic |
| CRDTs | Data structures designed to merge without conflicts | Mathematically guaranteed convergence, but restricted operations |
| Manual resolution | Expose both versions to the user/application | Most flexible, worst UX |

CRDTs get a full lesson in Phase 11 Lesson 12. For now, treat them as a strategy you *know exists*.

## Build It

We'll build a complete vector clock system with causal broadcast and conflict detection in Python. Run it with `python3 main.py`.

### Step 1: VectorClock Class

A `VectorClock` tracks per-process counters. It supports `increment`, `merge` (element-wise max), and all four comparison operations.

```
vc_a = VectorClock(processes=["P0", "P1", "P2"])
vc_a.increment("P0")    # [1, 0, 0]
vc_a.increment("P0")    # [2, 0, 0]

vc_b = VectorClock(processes=["P0", "P1", "P2"])
vc_b.increment("P1")    # [0, 1, 0]
vc_b.merge(vc_a)        # [2, 1, 0]  (element-wise max)
vc_b.increment("P1")    # [2, 2, 0]

vc_a < vc_b?    → Yes: both components ≤ and at least one <
vc_a || vc_b?   → No: they are ordered
```

### Step 2: Process with Message Passing

A `Process` has a name, a vector clock, and can perform local events, send messages, and receive messages. The VC is updated per the rules in the concept section.

### Step 3: Causal Broadcast (CBCAST)

The `CausalBroadcast` class routes messages between processes. When an out-of-order message arrives, it goes into a delay buffer. Delivery happens only when all causally preceding messages have been delivered.

### Step 4: Conflict Detection

The `ConflictDetector` takes two versions (value + vector clock) and classifies their relationship: `before`, `after`, `concurrent`, or `identical`.

### Step 5: Simulation

Run three scenarios:
1. Three processes exchange messages — show VC evolution.
2. Concurrent updates to the same key — detect conflict.
3. Out-of-order delivery — CBCAST enforces causal order.

## Use It

**Riak** (the distributed database descended from Amazon Dynamo) uses vector clocks (and later dotted version vectors) to track causality across replicas:

- Each write extends the vector clock for the coordinating node.
- On read, if two siblings have incomparable VCs, Riak returns both to the client for application-specific merge.
- The default conflict resolution is "last sibling wins" (a form of LWW), but the recommended approach is CRDT-like merge.
- Source: [riak_object.erl](https://github.com/basho/riak_kv/blob/develop/src/riak_object.erl) — the `merge` function reconciles vector clock siblings.

**Amazon DynamoDB** descends from the same Dynamo paper but simplifies: it uses LWW for conflict resolution by default, trading consistency for simplicity. Dynamo's original vector clock approach lives on in open-source Riak and Apache Cassandra's lightweight transactions (which use Paxos, not VCs, for linearizability).

Compare to our implementation: Riak's DVV handles the "same replica, multiple updates" corner case that a plain VC misses. Our plain VC treats `P0:[2,0,0]` (P0 did 2 local events) as the same causal context regardless of which event created the current value — DVV's dot disambiguates this.

## Read the Source

- [Riak riak_object.erl](https://github.com/basho/riak_kv/blob/develop/src/riak_object.erl) — the `merge/2` function that reconciles siblings using vector clock comparison. Look at how it handles the `dominates` check (our `<` operator).
- [Akka DistributedData](https://github.com/akka/akka/blob/main/akka-distributed-data/src/main/scala/akka/cluster/ddata/ReplicatedData.scala) — CRDT implementations that use vector clocks for causal context. The `GCounter` is a grow-only counter that is essentially a vector clock.

## Ship It

The reusable artifact is a **vector clock library** in `outputs/`. It ships as a single Python module you can import or run:

- `python3 main.py` — full demo
- `from main import VectorClock, Process, CausalBroadcast, ConflictDetector` — reuse in later phases

## Exercises

1. **Easy** — Create two vector clocks: `vc_a = [3, 0, 1]` and `vc_b = [2, 4, 1]`. Classify their relationship (before, after, concurrent, identical).
2. **Medium** — Extend `CausalBroadcast` to handle a fourth process that joins mid-simulation. What happens to the vector clock dimensions? How does the delay buffer handle messages from a process the recipient has never seen?
3. **Hard** — Implement dotted version vectors on top of `VectorClock`. Add a `dot` field `(replica_id, counter)` to each version. Show that plain VCs lose information when the same replica makes two updates between merges, and that DVVs recover it. Write a test that fails with plain VCs and passes with DVVs.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Vector clock | "A fancy timestamp" | A per-process counter vector that captures the full happens-before relationship: `VC(a) < VC(b)` iff `a → b` |
| Causal order | "Message ordering" | Delivery ordering that respects happens-before: if `m₁ → m₂`, then `m₁` is delivered before `m₂` at every process |
| Concurrent | "At the same time" | Neither event caused the other (`a ∦ b` and `b ∦ a`); their VCs are incomparable — this is a conflict |
| CBCAST | "Ordered broadcast" | Causal broadcast — delivers messages in causal order by buffering out-of-order messages until their dependencies arrive |
| Dotted version vector | "A vector clock with a dot" | A VC plus a single `(replica, counter)` pair that identifies the exact event that created this version, preventing information loss on merge |
| Conflict resolution | "Fixing inconsistencies" | Choosing how to reconcile concurrent updates: LWW, application merge, CRDTs, or manual |
| Lamport clock vs vector clock | "They're both timestamps" | Lamport gives a single integer that refines happens-before into total order; vector clocks give a vector that *captures* happens-before exactly |

## Further Reading

- [Why Vector Clocks are Easy](https://blog.acolyer.org/2015/01/30/why-vector-clocks-are-easy/) — Adrian Colyer's summary of the "Why Vector Clocks Are Easy" paper, a clear rebuttal to "Why Vector Clocks Are Hard."
- [Why Vector Clocks are Hard](https://arxiv.org/abs/1806.03200) — Carlos Baquero's analysis of vector clock practical challenges (space, pruning, DVVs).
- [Dynamo: Amazon's Highly Available Key-value Store](https://www.allthingsdistributed.com/files/amazon-dynamo-sosp2007.pdf) — The original paper. Section 4.3 describes vector clocks for versioning.
- [Dotted Version Vectors](https://arxiv.org/abs/1011.5808) — The paper that introduces DVVs and explains why plain VCs lose information in distributed storage.