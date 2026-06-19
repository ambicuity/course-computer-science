# Failure Models — Crash, Omission, Byzantine

> You can't build correct distributed algorithms without specifying what failures you tolerate.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 11 lesson 01
**Time:** ~60 minutes

## Learning Objectives

- Classify failures into crash-stop, crash-recovery, omission, timing, and Byzantine models and explain why the hierarchy matters
- Prove why 3 generals with 1 traitor cannot reach agreement (Byzantine Generals Problem)
- Explain why 3f+1 nodes are needed to tolerate f Byzantine faults and trace the PBFT protocol through its three phases
- Distinguish crash from omission failures and explain why timeouts are unreliable failure detectors
- Implement a Byzantine generals simulator demonstrating impossibility and PBFT consensus

## The Problem

You're building a distributed database. Node B stops responding. Is it crashed? Slow? Or lying about the data it holds? Your answer determines whether your system survives — or silently corrupts.

Most engineers assume "failure" means "the machine dies." In distributed systems that assumption kills you. A node that drops every third message is not the same as a dead node. A node that sends conflicting values to different peers is not the same as a slow node. If you design your consensus protocol for crash failures but your network suffers omission failures, your system breaks in ways you won't detect until production.

The lesson you're about to learn is: **every distributed algorithm is correct only under a specific failure model.** Use an algorithm outside its model and you get silent data loss — not an error message.

This lesson is the foundation for everything in Phase 11. Raft tolerates crash failures. PBFT tolerates Byzantine failures. You need to understand *why* before you can choose between them for the phase capstone.

## The Concept

### Why Failure Models Matter

A failure model specifies what can go wrong. It's not a theory exercise — it's the contract your algorithm relies on. An algorithm designed for crash-stop failures assumes:

- Failed nodes never send messages again
- Correct nodes only send truthful messages
- The network delivers messages eventually (no loss)

If any of those assumptions is violated, the algorithm's correctness proof falls apart. The algorithm doesn't "degrade gracefully" — it produces wrong results silently.

### The Failure Model Hierarchy

```
Byzantine (arbitrary behavior)
    ↑ generalizes
Omission (message loss)
    ↑ generalizes
Timing (slow responses)
    ↑ generalizes
Crash-recovery (stop, then restart)
    ↑ generalizes
Crash-stop (stop forever)
```

Each level **generalizes** the one below it. An algorithm that works under omission failures also works under crash-stop, but the reverse is not true. An algorithm for the crash-stop model will break under omission because it never retransmits lost messages.

### Crash-Stop Failure

A node halts and never recovers. This is the simplest model. Other nodes detect the failure via timeout: "if I haven't heard from you in T seconds, you're dead."

The problem: **timeouts lie.** A timeout tells you "I haven't received a message in T seconds." It does not tell you why. The node could be:
- Permanently crashed
- Temporarily slow (GC pause, network congestion)
- Partitioned from you (it's fine, just unreachable)

```
            timeout fires
Node A --------X---------> "B must be dead"
                              |
                              +---> B is actually fine, just slow
                              +---> B is crashed
                              +---> Network partition
```

You cannot distinguish these cases in an asynchronous system. This is why distributed systems use *suspect* mechanisms, not *detect* mechanisms. FLP impossibility (lesson 01) means no deterministic failure detector exists.

### Crash-Recovery Failure

A node crashes, then recovers. This is strictly harder than crash-stop because:

1. You must distinguish a recovering node from an imposter ("Is this really node B, or a new node pretending?")
2. Recovering nodes have stale state — they don't know what they missed
3. You need **stable storage** (write-ahead log) so the node can replay its actions after recovery

Without stable storage, a recovering node might double-commit or lose commits it acknowledged before crashing.

### Omission Failure

A node drops messages. There are two subtypes:

- **Send omission**: The node sends a message, but it never reaches the network (e.g., NIC buffer overflow)
- **Receive omission**: A message arrives at the node's NIC, but the node never processes it (e.g., kernel buffer full)

```
Node A ---[msg]---> network ---[X]---> Node B (receive omission)
Node A ---[msg]---> [X] (send omission)           Node B
```

Network partitions are **systematic omission failures** — a set of nodes consistently drops all messages to/from another set. The CAP theorem (lesson 01) is fundamentally about omission: during a partition, you cannot both maintain consistency and availability.

### Timing Failure

A node responds, but too slowly. In a synchronous system, you have known bounds on message delay and processing time. In an **asynchronous system** — which the internet effectively is — there are no timing guarantees. A 10ms response and a 10-second response are both valid.

Timing failures are insidious because they look like omission failures to the observer. "No response in T seconds" could be omission or just slowness.

### Byzantine Failure

The most general — and most dangerous — model. A Byzantine node behaves **arbitrarily**. It can:

- Send conflicting messages to different peers ("The value is 5" to A, "The value is 7" to B)
- Lie about its state
- Collude with other Byzantine nodes
- Selectively drop messages
- Pretend to be a different node

Byzantine failures model everything: buggy software, corrupted memory, compromised nodes, malicious actors. They are also the most expensive to tolerate.

### The Byzantine Generals Problem

Lamport, Shostak, and Pease (1982) proved: **with 3 generals and 1 traitor, agreement is impossible.**

Here's why. Three generals — one Commander (C) and two Lieutenants (L1, L2) — must agree on a single value (attack or retreat).

**Case 1: Commander is the traitor**

```
C (traitor) ---- "ATTACK" ----> L1
              ---- "RETREAT" ---> L2
```

L1 receives "ATTACK", L2 receives "RETREAT". L1 and L2 exchange messages:

- L1 tells L2: "The Commander said ATTACK"
- L2 tells L1: "The Commander said RETREAT"

L2 now hears: "Commander said RETREAT" (from C directly) and "Commander said ATTACK" (from L1). This is identical to the scenario where the Commander is honest and L1 is the traitor. L2 cannot distinguish the two cases.

**Case 2: Lieutenant is the traitor**

```
C (honest) ---- "ATTACK" ----> L1 (traitor)
            ---- "ATTACK" ---> L2
```

L1 (traitor) tells L2: "The Commander said RETREAT". L2 hears "ATTACK" from C but "RETREAT" from L1. This is the same evidence L2 had in Case 1.

The two cases are **indistinguishable** to L2. If L2 follows the honest-commander rule, it must choose ATTACK in Case 2. If L2 follows the majority rule, it gets ATTACK in Case 2 but also ATTACK in Case 1 (where the correct answer depends on C's intent, which is ill-defined since C is lying). There is no deterministic strategy that works.

**The general result**: To tolerate f Byzantine faults, you need **3f + 1** nodes.

Why? Each honest node must outvote the f traitors. With n nodes, f traitors, and n-f honest nodes, honest nodes must be able to reach agreement without the traitors. The critical threshold: each honest node receives messages from n-1 others. Of those, up to f could be from traitors. For the honest majority among the n-1 messages to exceed the traitors: (n-1-f) > f, which gives n > 2f, meaning n ≥ 2f+1. But that's for one round. Byzantine agreement requires relaying — so the traitors can lie about what others said. After relaying, we need n ≥ 3f+1.

### Practical Byzantine Fault Tolerance (PBFT)

Castro and Liskov (1999) created the first practical protocol for Byzantine fault tolerance. It handles 3f+1 replicas tolerating f Byzantine faults, with 2f+1 forming a quorum.

PBFT has three phases after a client request:

**Phase 1: Pre-prepare**

```
Client --[request]--> Primary
Primary --[pre-prepare(seq#, digest)]--> All replicas
```

The primary (leader) assigns a sequence number and broadcasts a pre-prepare message with the request digest. This establishes the ordering.

**Phase 2: Prepare**

```
Replica_i --[prepare(seq#, digest, view#)]--> All other replicas
```

Each replica broadcasts a prepare message. A replica enters the **prepared** state when it collects 2f matching prepare messages (plus its own pre-prepare). That's 2f+1 total — enough to guarantee that at least f+1 honest nodes agree on this sequence number and digest.

**Phase 3: Commit**

```
Replica_i --[commit(seq#, digest, view#)]--> All other replicas
```

Once prepared, each replica broadcasts a commit message. A replica **commits** when it collects 2f+1 matching commit messages. At this point, f+1 honest nodes have committed, guaranteeing that even if f nodes fail, at least one honest node knows the value and can inform others.

```
Time ─────────────────────────────────────────────>

Client     Primary     Replica 1     Replica 2     Replica 3
  |            |            |             |             |
  |--request-->|            |             |             |
  |            |--pre-prepare-->|          |             |
  |            |--pre-prepare------------>|             |
  |            |--pre-prepare-------------------------->|
  |            |            |             |             |
  |            |<--prepare--|             |             |
  |            |<--prepare----------------|             |
  |            |<--prepare------------------------------|
  |            |  (2f+1 prepared)         |             |
  |            |            |             |             |
  |            |<--commit---|             |             |
  |            |<--commit----------------|             |
  |            |<--commit------------------------------|
  |            |  (2f+1 committed)        |             |
  |<--reply----|            |             |             |
```

**Why three rounds?** Pre-prepare establishes order. Prepare ensures enough honest nodes saw the same value. Commit ensures enough honest nodes recorded it permanently. Without commit, two different values could be "prepared" at different honest nodes during a view change.

## Build It

We'll build a Byzantine generals simulator in Python that demonstrates:

1. The impossibility of 3 generals / 1 traitor
2. PBFT-style consensus with 4 nodes / 1 traitor
3. A failure simulator comparing crash, omission, and Byzantine behavior

### Step 1: Byzantine Generals — The Impossible Case

```python
import random

class General:
    def __init__(self, name, is_traitor=False):
        self.name = name
        self.is_traitor = is_traitor

    def send_order(self, order, recipient_name):
        if self.is_traitor:
            return random.choice(["ATTACK", "RETREAT"])
        return order

def simulate_3_generals(commander_is_traitor):
    commander = General("Commander", is_traitor=commander_is_traitor)
    l1 = General("Lieutenant1")
    l2 = General("Lieutenant2")

    true_order = "ATTACK"
    l1_hears = commander.send_order(true_order, "L1")
    l2_hears = commander.send_order(true_order, "L2")

    l1_relay = l1.send_order(l1_hears, "L2")
    l2_relay = l2.send_order(l2_hears, "L1")

    l1_evidence = [l1_hears, l2_relay]
    l2_evidence = [l2_hears, l1_relay]

    l1_decision = majority(l1_evidence)
    l2_decision = majority(l2_evidence)

    return l1_decision, l2_decision, l1_evidence, l2_evidence

def majority(votes):
    a = votes.count("ATTACK")
    r = votes.count("RETREAT")
    return "ATTACK" if a >= r else "RETREAT"
```

Run this 10 times with the commander as traitor — L1 and L2 will disagree often, and even when they agree, they agree on the wrong value.

### Step 2: PBFT-Style Consensus with 4 Nodes

```python
class PBFTNode:
    def __init__(self, name, is_byzantine=False):
        self.name = name
        self.is_byzantine = is_byzantine
        self.pre_prepare_msgs = []
        self.prepare_msgs = []
        self.commit_msgs = []
        self.committed_value = None

    def pre_prepare(self, value, seq_num):
        if self.is_byzantine:
            return ("PRE-PREPARE", seq_num, random.choice(["A", "B"]))
        return ("PRE-PREPARE", seq_num, value)

    def prepare(self, pre_prepare_msg):
        if self.is_byzantine:
            return ("PREPARE", pre_prepare_msg[1], random.choice(["A", "B"]),
                    self.name)
        return ("PREPARE", pre_prepare_msg[1], pre_prepare_msg[2], self.name)

    def commit(self, prepared_value, seq_num):
        if self.is_byzantine:
            return ("COMMIT", seq_num, random.choice(["A", "B"]), self.name)
        return ("COMMIT", seq_num, prepared_value, self.name)

def run_pbft(nodes, value, f):
    n = len(nodes)
    primary = nodes[0]

    pp_msg = primary.pre_prepare(value, 0)

    prepare_msgs = []
    for node in nodes:
        p_msg = node.prepare(pp_msg)
        prepare_msgs.append(p_msg)

    honest_prepares = [m for m in prepare_msgs if not nodes[m[3] == primary.name and 0 or 1].is_byzantine]
    prepared_value = value
    if len(prepare_msgs) >= 2 * f + 1:
        values = [m[2] for m in prepare_msgs]
        from collections import Counter
        counts = Counter(values)
        prepared_value = counts.most_common(1)[0][0]

    commit_msgs = []
    for i, node in enumerate(nodes):
        c_msg = node.commit(prepared_value, 0)
        commit_msgs.append(c_msg)

    if len(commit_msgs) >= 2 * f + 1:
        values = [m[2] for m in commit_msgs]
        from collections import Counter
        counts = Counter(values)
        return counts.most_common(1)[0][0]

    return None
```

With 4 nodes and 1 traitor, the 3 honest nodes outvote the traitor in every phase.

### Step 3: Full Simulator

The complete `main.py` in `code/` ties all of this together with a clean CLI interface, runs the impossibility demonstration, the PBFT consensus, and a failure model comparison.

## Use It

**Raft** (the consensus protocol you'll build in the capstone) is designed for crash-stop failures. It uses leader election with timeouts and requires a majority quorum. Look at how Raft handles a crashed leader:

- etcd's Raft implementation: [`etcd/raft/node.go`](https://github.com/etcd-io/etcd/blob/main/raft/node.go) — the election timeout mechanism that detects leader failure
- Raft assumes nodes either respond correctly or not at all — no omission, no lying

**PBFT** is used in systems that must tolerate Byzantine faults:

- Hyperledger Fabric's consensus: [`fabric/orderer/consensus/bft`](https://github.com/hyperledger/fabric/tree/main/orderer/consensus/bft) — production PBFT implementation
- Tendermint (Cosmos): uses a BFT consensus protocol with 3 rounds analogous to PBFT's pre-prepare/prepare/commit

**Key difference**: Raft's 2f+1 nodes tolerate f crash faults. PBFT's 3f+1 nodes tolerate f Byzantine faults. The 50% overhead is the price of Byzantine tolerance.

## Read the Source

- [`etcd/raft/raft.go`](https://github.com/etcd-io/etcd/blob/main/raft/raft.go) — Step function handles message types; note how it treats all messages as truthful (crash-stop assumption)
- [`tendermint/internal/consensus/state.go`](https://github.com/cometbft/cometbft/blob/main/internal/consensus/state.go) — three-phase vote (prevote, precommit, commit) mirrors PBFT's prepare/commit

## Ship It

The reusable artifact is a Byzantine generals simulator (`code/main.py`) that you can run to:

- Demonstrate impossibility with 3 generals / 1 traitor
- Demonstrate PBFT consensus with 4 nodes / 1 traitor
- Compare crash, omission, and Byzantine failure models with configurable parameters

You'll reuse these failure models in later lessons on Raft, leader election, and the capstone KV store.

## Exercises

1. **Easy** — Modify the simulator to use 7 generals with 2 traitors. Run it 100 times. Does honest agreement always hold? Why?
2. **Medium** — Add crash-recovery to the simulator. A node crashes mid-protocol, then recovers with its persistent log. Show that stable storage is necessary — without it, the recovering node violates safety.
3. **Hard** — Implement view changes in the PBFT simulator. When the primary is suspected faulty (timeout), the protocol must re-elect a leader without losing committed values. This is the hardest part of PBFT and the reason most production systems use crash-fault consensus instead.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Crash-stop | "The server died" | Node halts permanently and never sends another message |
| Crash-recovery | "The server restarted" | Node halts then recovers; needs stable storage to resume safely |
| Omission | "The network lost the packet" | Node drops messages — can be send-side or receive-side; partitions are systematic omission |
| Timing failure | "It's fast, then slow" | Node responds correctly but outside expected time bounds; indistinguishable from omission in async networks |
| Byzantine | "The node is malicious" | Node behaves arbitrarily — conflicting messages, lies, collusion; the strongest failure model |
| 3f+1 | "You need 4 nodes for 1 fault" | Minimum nodes to tolerate f Byzantine faults; each honest node must outvote f traitors even with f missing messages |
| PBFT | "Three-phase consensus" | Practical Byzantine Fault Tolerance — pre-prepare, prepare, commit phases with 2f+1 quorum at each stage |
| Failure detector | "Heartbeat timeout" | An unreliable oracle that *suspects* failures but cannot *detect* them in async networks (FLP) |

## Further Reading

- [The Byzantine Generals Problem](https://lamport.azurewebsites.net/byz/byz.pdf) (Lamport, Shostak, Pease, 1982) — the original impossibility proof and the 3f+1 algorithm
- [Practical Byzantine Fault Tolerance](https://pmg.csail.mit.edu/papers/osdi99.pdf) (Castro, Liskov, 1999) — the PBFT protocol that made Byzantine tolerance practical
- [FLP Impossibility](https://groups.csail.mit.edu/tds/papers/Lynch/jacm85.pdf) (Fischer, Lynch, Paterson, 1985) — why deterministic consensus is impossible in async systems with one crash fault
- [Unreliable Failure Detectors](https://www.cs.yale.edu/homes/aspnes/papers/jaakola-freund-schapire.pdf) (Chandra, Toueg, 1996) — how *eventually* strong failure detectors enable consensus despite FLP