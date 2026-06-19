# Gossip Protocols & SWIM

> Epidemics are the fastest way to spread a rumor in a crowd — and also the fastest way to detect who's gone silent.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 11 lessons 01–12
**Time:** ~60 minutes

## Learning Objectives

- Explain why epidemic-style (gossip) protocols achieve O(log N) dissemination with constant fanout, and contrast rumor mongering with anti-entropy.
- Describe the SWIM protocol's three components — failure detection, membership dissemination, and suspicion — and how they eliminate the need for central coordinators or all-to-all heartbeats.
- Implement a SWIM membership simulation in Python: ping/ack with indirect probing, suspicion with incarnation-based refutation, and piggyback membership dissemination.
- Explain incarnation numbers: how they prevent stale information from resurrecting dead nodes, and how a wrongly-suspected node refutes suspicion.
- Compare SWIM's approach to production systems: Cassandra's gossip, Consul's Serf/SWIM, and HashiCorp's memberlist library.

## The Problem

You operate a 10,000-node cluster. Node 7,342 crashes. How does the rest of the cluster find out?

**Option A: Central coordinator.** Every node sends heartbeats to a leader. The leader broadcasts failures. This works — until the leader dies, or becomes a bottleneck, or the network partition isolates the leader from half the cluster.

**Option B: All-to-all heartbeats.** Every node pings every other node. This means N² messages per round. At 10,000 nodes, that's 100 million messages per round. You'll never scale.

**Option C: Gossip.** Node A notices node B is down. A tells K random peers. Those peers tell K more. Within O(log N) rounds, the entire cluster knows. No central point of failure. No N² message explosion. The cluster converges on the truth exactly as fast as a rumor spreads through a crowd.

This lesson covers **gossip protocols** for information dissemination and the **SWIM** (Scalable Weakly-consistent Infection-style Membership) protocol, which uses gossip principles to build a membership service that scales to arbitrarily large clusters.

## The Concept

### Gossip Protocols: Epidemic Dissemination

Gossip protocols borrow from epidemiology. In an epidemic:
- An infected individual contacts random others.
- Each new infection independently contacts more random individuals.
- The disease spreads exponentially until the population is saturated.

In a distributed system, the "disease" is *information* — a membership update, a configuration change, or a data mutation. The math is identical:

```
Round 0:   1 node knows         (the originator)
Round 1:   1 + K nodes know     (originator told K random peers)
Round 2:   (1 + K) + K·(1+K)   (each new knower tells K more)
...
Round r:   ≈ (1+K)^r nodes know

To reach N nodes: r ≈ log_K+1(N) = O(log N)
```

With fanout K=3 and N=10,000: `log_4(10000) ≈ 7` rounds. Seven rounds of each node messaging 3 peers, and the entire 10,000-node cluster converges.

### Rumor Mongering vs. Anti-Entropy

Gossip protocols come in two flavors:

| Property | Rumor Mongering | Anti-Entropy |
|----------|----------------|-------------|
| **Mechanism** | Node with new info tells K random peers | Periodic pairwise full-state sync (or digest exchange) |
| **Speed** | Fast: O(log N) rounds to reach all | Slower: depends on round interval and random pairing |
| **Reliability** | Probabilistic — a rumor may die out before reaching everyone | High — full state exchange ensures eventual consistency |
| **Message cost per update** | O(K) per round per new piece of info | O(state size) per round regardless of number of updates |
| **Best for** | Hot, frequent updates | Cold, infrequent updates; reconciliation |
| **Used by** | SWIM membership dissemination | Cassandra's anti-entropy repair |

**Rumor mongering** is like telling your friends a secret. If you tell 3 friends, and each tells 3 more, the secret spreads fast. But there's a small probability some people never hear it — the rumor "dies out" if nobody spreads it further. Systems that need guaranteed delivery add a background anti-entropy phase.

**Anti-entropy** is like periodically comparing your entire address book with a random colleague. It's slower per update but guarantees convergence. Cassandra uses this: every node periodically selects a random peer and exchanges Merkle tree digests to find and repair data inconsistencies.

Most production systems use **both**: rumor mongering for speed, anti-entropy as a reliable fallback.

### SWIM: Scalable Weakly-consistent Infection-style Membership

The SWIM protocol, introduced by Das, Chandra, and Vitek in 2002, solves the group membership problem: *who is currently alive in the cluster?*

The key insight: **membership information is itself disseminated via gossip**. Instead of every node reporting to a central server, or every node heartbeating every other node, SWIM detects failures locally and spreads the news epidemically.

SWIM has three interacting components:

#### 1. Failure Detection

Each round, every node picks a random target and sends it a **ping**:

```
Node A picks random target B
  ┌─── ping ───→ B
  │                │
  │            B responds
  │←── ack ────────┘
  ✓ B is alive
```

If B doesn't respond within a timeout, A doesn't immediately declare B dead. Instead, A performs an **indirect probe**:

```
Node A picks K random peers (not B)
  ┌─── ping-req(C, target=B) ───→ C
  │                                │
  │                            C pings B on A's behalf
  │                                │
  │           ┌──── B alive? ──────┤
  │           │                    │
  │←── ack (via C) ───────────────┘    OR    ←── no response (via C neither)
  │
  ✓ B is alive (indirect ack)          ✗ B is suspect
```

Indirect probing avoids false positives from transient network issues between A and B. If A can't reach B directly, but C *can* reach B, then B is alive — A and B just have a connectivity problem. Only if both direct and indirect probes fail does A suspect B.

#### 2. Membership Dissemination (Piggyback)

When A detects that B is alive or suspect, how does the rest of the cluster find out? SWIM **piggybacks** membership updates onto the ping/ack/ping-req messages that are already being sent:

```
Node A pings B:
  ┌─── ping [membership_delta: {C: suspect}] ───→ B

Node B acks A:
  ├──→ ack [membership_delta: {D: alive, E: alive}] ───→ A
```

No extra messages. Membership information rides for free on protocol messages that would be sent anyway. This is the "infection-style" part of SWIM — updates propagate like an infection, piggybacked on routine traffic.

Each message includes a small buffer of recent membership changes. Over O(log N) rounds, the entire cluster converges on the membership state.

#### 3. Suspicion Mechanism

A node that misses a ping isn't immediately declared dead. It enters a **Suspect** state:

```
Alive ──(missed ping, timeout)──→ Suspect ──(confirmation timeout)──→ Dead
  ↑                                  │
  └──(refutation: "I'm alive with inc+1!")──┘
```

When A suspects B, A gossips `{B: suspect}` throughout the cluster. If B is actually alive, B hears this suspicion (via piggyback gossip) and **refutes** it by incrementing its **incarnation number** and broadcasting `{B: alive, inc=B.inc+1}`.

**Incarnation numbers** work like this:
- Each member maintains an `incarnation` counter (starts at 0).
- When a member is wrongly suspected, it increments its incarnation and gossips `{self: alive, inc=new_value}`.
- Membership updates are ordered by `(state, incarnation)`:
  - `Alive` with higher incarnation beats `Suspect` with lower incarnation.
  - `Suspect` with higher incarnation beats `Alive` with lower incarnation.
  - `Dead` is permanent.

This prevents stale information from resurrecting truly dead nodes. If node B is suspected at incarnation 3, and later B's incarnation 4 alive message arrives, the cluster upgrades to alive. But if B is truly dead, no incarnation increment will ever appear, and the suspicion eventually confirms to dead.

### SWIM vs Traditional Failure Detection

| Property | All-to-all heartbeats | Central heartbeat server | SWIM |
|----------|----------------------|------------------------|------|
| **Messages per round** | O(N²) | O(N) to server + O(N) broadcast | O(N) total (each node sends 1 ping) |
| **False positive rate** | Low (direct connectivity) | Low (direct to server) | Low (indirect probing reduces false positives) |
| **Single point of failure** | No | Yes (the server) | No |
| **Convergence** | Immediate (after 1 timeout) | Fast but depends on server | O(log N) rounds |
| **Scalability** | Poor (N² messages) | Moderate (server bottleneck) | Excellent (linear message cost) |

### Production Gossip Implementations

**Cassandra** uses a gossip protocol for cluster membership and state dissemination. Every second, each node gossips with 1–3 random peers. The gossip payload includes: node state (alive/leaving/left), generation number (like incarnation), application state (schema versions, ring positions). Cassandra combines rumor mongering (for speed) with anti-entropy (for reliability).

**Consul** (via Serf) uses SWIM for its LAN gossip pool. Serf is HashiCorp's implementation of SWIM with Lifeguard extensions that reduce false positives further. Consul uses gossip for: cluster membership, failure detection, and event broadcast (custom application messages).

**HashiCorp's memberlist** is the Go library that powers Serf, Consul, and Nomad's cluster membership. It implements SWIM with several extensions: compound messages (batching multiple small messages), push-pull state sync (anti-entropy), and custom awareness of local health (the Lifeguard enhancements).

## Build It

We'll build a SWIM membership protocol simulation in Python. Run it with `python3 main.py`.

### Step 1: SWIMMember — Node State

Each member has an ID, an incarnation number, and a state (`Alive`, `Suspect`, or `Dead`). Incarnation numbers increment on refutation.

### Step 2: SWIMCluster — Network Simulation

A `SWIMCluster` manages N members and simulates the network with a configurable message drop rate. It supports: killing a node (truly dead), slowing a node (misses pings but eventually responds), and running failure detection rounds.

### Step 3: Failure Detection — Ping/ACK

In each round, every alive node picks a random target and sends a ping. If the ping times out (network drop or dead target), the pinger performs an indirect probe: it asks K other members to ping the target on its behalf.

### Step 4: Suspicion

When a target fails both direct and indirect probes, the pinger marks it `Suspect`. If the target is actually alive, it hears the suspicion (via piggyback gossip) and refutes by incrementing its incarnation number.

### Step 5: Membership Dissemination

Every protocol message (ping, ack, ping-req, indirect-ack) carries a piggyback buffer of recent membership changes. Recipients merge this information into their local membership table, preferring higher incarnation numbers and more severe states.

### Step 6: Anti-Entropy Gossip

Periodically, each node performs a full state sync with a random peer — this is the anti-entropy mechanism that ensures convergence even if rumor mongering missed someone.

### Step 7: Demo Scenarios

The simulation runs two scenarios:
1. **Node death**: 10-node cluster, kill node 7, observe suspicion spreading through the cluster, then confirmation of death.
2. **False positive refutation**: 10-node cluster, node 3 has a slow network (drops some pings). Other nodes suspect it, but node 3 refutes with a higher incarnation number and re-joins as fully alive.

## Use It

**HashiCorp's memberlist** library (`github.com/hashicorp/memberlist`) is the production reference for SWIM:

- It implements the full SWIM protocol with indirect probing, suspicion, and incarnation-based refutation.
- It adds Lifeguard enhancements: local health awareness (nodes under load defer suspicion), broadcast awareness (nodes track who they've broadcast to), and probe awareness (nodes rate-limit outgoing probes when local health degrades).
- The `InternalHandler` processes incoming ping/ack/ping-req messages.
- State synchronization uses push-pull: two nodes exchange full state digests over a dedicated TCP connection, then reconcile differences.

Compare to our simulation:
- Our simulation runs in discrete rounds. Memberlist operates in real-time with configurable intervals (default: probe interval 1s, gossip interval 200ms, push-pull interval 30s).
- Memberlist batches gossip messages into compound messages for efficiency. We piggyback on each individual message.
- Memberlist uses both UDP (for ping/ack/gossip) and TCP (for full state sync). We simulate everything in-process.

**Apache Cassandra's** gossip runs in `org.apache.cassandra.gms.Gossiper`. Each node gossips every second with up to 3 peers. The gossip digest includes: node endpoint, generation (epoch of node start, similar to incarnation), and max-version (highest application state version seen). Cassandra's gossip also handles schema disagreement, ring position changes, and decommissioning — all piggybacked on the same gossip messages.

## Read the Source

- [HashiCorp memberlist — `delegate.go`](https://github.com/hashiCorp/memberlist/blob/main/delegate.go) — the interface that applications implement to receive gossip messages. Look at how `NotifyMsg` delivers application-level broadcast messages alongside SWIM's internal membership data.
- [Cassandra Gossiper — `Gossiper.java`](https://github.com/apache/cassandra/blob/cassandra-5.0/src/java/org/apache/cassandra/gms/Gossiper.java) — the full gossip implementation. The `applyStateLocally` method shows how incoming gossip digests are merged into local state with generation numbers acting as incarnation numbers.
- [SWIM paper](https://www.cs.cornell.edu/projects/Quicksilver/public_pdfs/SWIM.pdf) — Das, Chandra, Vitek (2002). The original specification. Section 3 describes the three-component decomposition; Section 4 analyzes protocol correctness.

## Ship It

The reusable artifact is a **SWIM membership protocol simulation** in `code/main.py`. Run it with:

- `python3 main.py` — full demo with both scenarios
- Import `SWIMMember`, `SWIMCluster` for use in later phases and exercises

## Exercises

1. **Easy** — Modify the fanout K in the simulation from 3 to 1. Run the death scenario and count how many more rounds it takes for the cluster to converge compared to K=3. Explain why in terms of O(log N).
2. **Medium** — Add a network partition to the simulation: split the 10-node cluster into two groups of 5 that cannot communicate. In one group, kill a node. What does the other group observe? How does the suspicion timeout affect the outcome?
3. **Hard** — Implement the Lifeguard extension from HashiCorp's memberlist: local health awareness. Track each node's recent probe success rate. Nodes with low success rates should defer expressing suspicion. Show that this reduces false positives in clusters with high network loss.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Gossip protocol | "A broadcast protocol" | An epidemic-style protocol where each node shares information with K random peers, achieving O(log N) dissemination without central coordination |
| Fanout | "How many messages per round" | The number K of random peers each node contacts per gossip round; determines the exponential spread rate |
| Rumor mongering | "Fire-and-forget gossip" | A gossip mode where new information is actively spread for a limited number of rounds, then forgotten — fast but probabilistic |
| Anti-entropy | "Background sync" | Periodic pairwise full-state comparison (or digest exchange) — slower than rumor mongering but guarantees convergence |
| SWIM | "A membership protocol" | Scalable Weakly-consistent Infection-style Membership: a protocol that detects failures locally and disseminates membership changes via piggyback gossip |
| Indirect probe | "Asking someone else to ping" | When a direct ping fails, the pinger asks K other members to ping the target — reduces false positives from point-to-point network issues |
| Incarnation number | "A version number for a node" | A counter that a member increments when refuting a false suspicion; higher incarnation always beats lower, preventing stale suspicions from lingering |
| Suspect state | "Probably dead" | An intermediate state between Alive and Dead — a suspected node can refute by broadcasting a higher incarnation number |
| Piggyback | "Free rider messages" | Attaching membership updates to protocol messages (ping/ack) that would be sent anyway, avoiding dedicated membership broadcast messages |

## Further Reading

- [SWIM: Scalable Weakly-consistent Infection-style Process Group Membership Protocol](https://www.cs.cornell.edu/projects/Quicksilver/public_pdfs/SWIM.pdf) — The original paper by Das, Chandra, and Vitek (2002). The foundation of everything in this lesson.
- [Lifeguard: Local Health Awareness for More Resilient Gossip](https://arxiv.org/abs/2004.14590) — HashiCorp's extension to SWIM that reduces false positives by making nodes aware of their own local health (probe success rate, broadcast reach).
- [Cassandra Architecture — Gossip](https://cassandra.apache.org/doc/latest/cassandra/architecture/gossip.html) — Official documentation on how Cassandra uses gossip for cluster membership and failure detection.
- [Serf: Gossip-based membership](https://www.serf.io/docs/internals/gossip.html) — Serf's documentation on its SWIM implementation, including how it handles suspicion, refutation, and graceful leaves.