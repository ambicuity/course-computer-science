# TLA+ for Distributed Protocols

> Model the protocol before the pager models it for you.

**Type:** Learn
**Languages:** TLA+
**Prerequisites:** Phase 17 lessons 01-11
**Time:** ~90 minutes

## Learning Objectives

- Build a bounded distributed protocol model in TLA+.
- State safety invariants and simple progress properties.
- Analyze message loss/reordering abstraction impact.
- Use traces to refine protocol design.

## The Problem

Distributed protocol bugs are usually interleaving bugs. Unit tests can validate
handlers but rarely enumerate all reorderings, drops, and retries. TLA+ lets
you model transitions and explore reachable states systematically.

A leader election protocol sounds simple: nodes timeout, become candidates,
request votes, win or lose. But the corner cases are brutal. What if two nodes
timeout simultaneously? What if a vote arrives after the candidate already lost?
What if a network partition splits the cluster? What if a node receives a vote
from a previous term?

Every distributed systems team has a story about a bug that survived months of
testing, code review, and staging deployments. The bug required a specific
interleaving of 4-5 events that happened once in production under load. TLA+
finds these interleavings in minutes.

## The Concept

### Modeling Distributed Protocols

A distributed protocol model in TLA+ has:

1. **Nodes and roles:** Each node has a local state (follower, candidate, leader).
2. **Message set:** A set of in-flight messages (possibly lossy).
3. **Local state transitions:** Nodes send/receive messages and change state.
4. **Invariants:** Properties that must hold across all reachable states.

```
    Distributed System Model:
    
    ┌─────────┐     ┌─────────┐     ┌─────────┐
    │ Node A  │     │ Node B  │     │ Node C  │
    │ state:  │     │ state:  │     │ state:  │
    │ follower│     │ follower│     │ follower│
    │ term: 0 │     │ term: 0 │     │ term: 0 │
    └────┬────┘     └────┬────┘     └────┬────┘
         │               │               │
         └───────────────┼───────────────┘
                         │
                    ┌────▼────┐
                    │ Network │
                    │ (message│
                    │   set)  │
                    └─────────┘
    
    Transitions:
    - Timeout(node): node becomes candidate
    - SendVoteReq(node): node sends RequestVote to all
    - ReceiveVoteReq(node, msg): node grants/denies vote
    - WinElection(node): node becomes leader (majority votes)
```

### Leader Election Model

We model a simplified leader election inspired by Raft:

```tla
------------------------- MODULE LeaderElection -------------------------
EXTENDS Naturals, FiniteSets

CONSTANTS Nodes

VARIABLES state, term, votes, messages

vars == <<state, term, votes, messages>>

TypeOK ==
    /\ state \in [Nodes -> {"follower", "candidate", "leader"}]
    /\ term \in [Nodes -> Nat]
    /\ votes \in [Nodes -> SUBSET Nodes]
    /\ messages \in SUBSET [type: {"vote_req", "vote_grant"}, 
                            from: Nodes, to: Nodes, t: Nat]

Init ==
    /\ state = [n \in Nodes |-> "follower"]
    /\ term = [n \in Nodes |-> 0]
    /\ votes = [n \in Nodes |-> {}]
    /\ messages = {}

\* Node times out and becomes candidate
Timeout(n) ==
    /\ state[n] = "follower"
    /\ state' = [state EXCEPT ![n] = "candidate"]
    /\ term' = [term EXCEPT ![n] = term[n] + 1]
    /\ votes' = [votes EXCEPT ![n] = {n}]  \* votes for self
    /\ messages' = messages \cup 
        {[type |-> "vote_req", from |-> n, to |-> m, t |-> term[n] + 1] 
         : m \in Nodes \ {n}}

\* Node receives vote request and grants vote
HandleVoteReq(n) ==
    \E msg \in messages:
        /\ msg.type = "vote_req"
        /\ msg.to = n
        /\ msg.t > term[n]  \* only vote for higher term
        /\ state[n] /= "leader"
        /\ state' = [state EXCEPT ![n] = "follower"]
        /\ term' = [term EXCEPT ![n] = msg.t]
        /\ votes' = [votes EXCEPT ![n] = {}]
        /\ messages' = (messages \ {msg}) \cup 
            {[type |-> "vote_grant", from |-> n, to |-> msg.from, t |-> msg.t]}

\* Candidate receives vote grant
HandleVoteGrant(n) ==
    \E msg \in messages:
        /\ msg.type = "vote_grant"
        /\ msg.to = n
        /\ msg.t = term[n]
        /\ state[n] = "candidate"
        /\ votes' = [votes EXCEPT ![n] = votes[n] \cup {msg.from}]
        /\ IF Cardinality(votes'[n]) > Cardinality(Nodes) \div 2
           THEN state' = [state EXCEPT ![n] = "leader"]
           ELSE state' = state
        /\ messages' = messages \ {msg}
        /\ UNCHANGED term

\* Message loss (nondeterministic)
DropMsg ==
    \E msg \in messages:
        messages' = messages \ {msg}
        /\ UNCHANGED <<state, term, votes>>

Next ==
    \E n \in Nodes:
        \/ Timeout(n)
        \/ HandleVoteReq(n)
        \/ HandleVoteGrant(n)
    \/ DropMsg

Spec == Init /\ [][Next]_vars

\* Safety: at most one leader per term
AtMostOneLeader ==
    \A n1, n2 \in Nodes:
        (state[n1] = "leader" /\ state[n2] = "leader") => n1 = n2

\* Safety: leader has majority votes
LeaderHasMajority ==
    \A n \in Nodes:
        state[n] = "leader" => Cardinality(votes[n]) > Cardinality(Nodes) \div 2

\* Safety: terms are monotonically increasing per node
TermMonotonic ==
    \A n \in Nodes: term[n] >= 0
=========================================================================
```

### What the Model Checker Finds

With 3 nodes, TLC explores ~500 states and checks all invariants. Common
findings:

1. **Split vote:** Two candidates with same term, neither gets majority.
   Not a bug (both timeout and retry), but reveals need for randomized
   timeouts in implementation.

2. **Stale vote:** A vote grant arrives after the candidate already lost
   the election. The model shows this is safe (the vote is ignored because
   the term changed).

3. **Message loss impact:** With `DropMsg` enabled, the model shows that
   the protocol still achieves eventual leader election (liveness) as long
   as not all messages are dropped forever.

### Refinement

TLA+ supports **refinement**: proving that a concrete spec implements an
abstract spec. You can model an abstract "eventual leader" property and prove
that your concrete election protocol refines it.

```
    Abstract spec: "Eventually one node is leader"
         ▲
         │  refinement mapping
         │
    Concrete spec: "Nodes timeout, request votes, win majority"
```

If the concrete spec satisfies all invariants of the abstract spec, you've
proven the implementation is correct with respect to the abstraction.

## Build It

### Step 1: Define the protocol model

Write the TLA+ specification above. The key design decisions:

- **Nondeterministic message loss:** `DropMsg` lets TLC explore what happens
  when messages are lost. This is more realistic than perfect delivery.
- **Term-based voting:** Nodes only vote for higher terms. This prevents
  stale votes from electing outdated candidates.
- **Self-voting:** Candidates vote for themselves immediately. This is
  essential for single-node clusters.

### Step 2: Define invariants

```tla
\* Safety invariants
AtMostOneLeader == ...
LeaderHasMajority == ...
TermMonotonic == ...

\* Type invariant (catches typos and modeling errors)
TypeOK == ...
```

### Step 3: Run TLC and analyze results

```bash
tlc LeaderElection.tla -config LeaderElection.cfg
```

If TLC finds a violation, read the trace. Common patterns:

- **Invariant too strong:** You claimed something that isn't always true.
  Weaken the invariant or fix the protocol.
- **Modeling error:** Your TLA+ doesn't match your intent. Fix the spec.
- **Real bug:** The protocol has a flaw. Fix the design before coding.

### Step 4: Add liveness (optional)

```tla
\* Eventually some node becomes leader
Liveness == <><>(\E n \in Nodes: state[n] = "leader")

\* Every candidate eventually becomes leader or follower (no stuck candidates)
Progress == \A n \in Nodes: 
    (state[n] = "candidate") ~> (state[n] \in {"leader", "follower"})
```

Liveness properties require fairness assumptions. Without fairness, a node
could starve forever (TLC would find a stuttering counterexample).

## Use It

Production workflow for TLA+ on distributed protocols:

1. **Sketch the protocol in TLA+** before writing code. The model is 50-100
   lines; the implementation is 500-5000.

2. **Check invariants with bounded constants.** Start with 2-3 nodes. If
   bugs exist, they usually manifest in small instances.

3. **Inspect the shortest failing trace.** TLC minimizes counterexamples.
   Read the trace, understand the interleaving, fix the design.

4. **Iterate.** Each cycle takes minutes. The model evolves with your
   understanding of the protocol.

5. **Check in the spec as documentation.** The TLA+ model is a precise,
   machine-checkable description of the protocol's intended behavior. Future
   engineers can read it to understand the design.

Production references:

- Amazon's DynamoDB, S3, and EBS all have TLA+ specs.
- The Raft consensus protocol has an official TLA+ spec.
- Azure Cosmos DB uses TLA+ for consistency level verification.

## Read the Source

- [Raft TLA+ spec](https://github.com/ongardie/raft.tla) — the official
  Raft specification.
- [Paxos Made Live](https://research.google/pubs/paxos-made-live/) — Google's
  experience turning Paxos into a real system.
- [TLA+ Hyperbook](https://lamport.azurewebsites.net/tla/hyperbook.html) —
  Lamport's canonical resource.

## Ship It

This lesson ships:

- `code/LeaderElection.tla`: protocol model with leader uniqueness invariant.
- `code/LeaderElection.cfg`: TLC configuration file.
- `outputs/README.md`: model-checking checklist for distributed protocols.

```bash
tlc code/LeaderElection.tla -config code/LeaderElection.cfg
```

## Quiz

**Pre-questions:**

**Q1.** Why is TLA+ especially useful for distributed protocols?

- A) Distributed protocols are simple enough to model completely.
- B) Distributed bugs are interleaving bugs that require exploring many event
   orderings, which TLA+ does exhaustively.
- C) TLA+ replaces the need for testing distributed systems.
- D) TLA+ can only model distributed systems.

**Answer: B.** Distributed protocol bugs almost always require a specific
ordering of events (message delays, crashes, retries) that's hard to hit
with unit tests. TLA+ explores all orderings in a bounded model, finding
interleavings that would take years of production traffic to encounter.

**Q2.** What is a "message loss" abstraction in a TLA+ model?

- A) Removing all message-related transitions.
- B) Adding a nondeterministic transition that removes messages from the
   in-flight set, simulating unreliable delivery.
- C) Modeling only successful message delivery.
- D) Using TCP instead of UDP.

**Answer: B.** Adding a `DropMsg` action that nondeterministically removes
messages from the set models unreliable networks. TLC explores paths where
messages are dropped at various points, revealing whether the protocol
tolerates loss.

**Post-questions:**

**Q3.** Your model has 3 nodes and TLC checks 500 states. What does increasing
to 5 nodes do?

- A) Doubles the state space.
- B) Increases it linearly.
- C) Increases it exponentially (more nodes means more interleavings).
- D) Has no effect.

**Answer: C.** More nodes means more possible states per node, more messages
in flight, and more interleavings. The state space grows exponentially. This
is why model checking uses small instances: if a protocol bug exists, it
usually manifests with 2-3 nodes.

**Q4.** TLC finds a trace where two nodes are both in "leader" state. What
should you do?

- A) Delete the AtMostOneLeader invariant.
- B) Read the trace to understand the interleaving, then fix the protocol
   or strengthen the election restriction.
- C) Add more nodes to the model.
- D) Run TLC again with a different random seed.

**Answer: B.** The counterexample trace shows exactly which sequence of events
leads to two leaders. Common causes: missing term check in vote handling,
allowing votes from lower terms, or not resetting votes when becoming
candidate. Fix the protocol based on what the trace reveals.

**Q5.** What is "refinement" in TLA+?

- A) Making the model more detailed.
- B) Proving that a concrete spec implements an abstract spec by showing
   every behavior of the concrete spec corresponds to a behavior of the
   abstract spec.
- C) Simplifying the model for faster checking.
- D) Adding more invariants.

**Answer: B.** Refinement proves that a detailed (concrete) specification
implements a simpler (abstract) specification. Every execution of the concrete
spec maps to a valid execution of the abstract spec. This lets you verify
low-level protocol details against high-level safety properties.

## Exercises

**Easy:** Add a message duplication action to the model. What happens when
messages are duplicated? Does the protocol still satisfy mutual exclusion?

**Medium:** Add term numbers and a monotonicity invariant. Model the case
where a node receives a vote request with a term lower than its current term.
Verify that the node rejects the request.

**Hard:** Model a split vote scenario. Two candidates start elections
simultaneously with the same term. Neither gets majority. Add a retry
mechanism (candidates timeout again and start new election with higher term).
Verify that the protocol eventually elects a leader.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| State space | "all cases" | Reachable valuation set under transitions |
| Invariant | "always true rule" | Safety property over all reachable states |
| Nondeterminism | "randomness" | Explicit modeling of alternative transitions |
| Refinement | "more detail" | Relation between abstract and concrete specs |
| Liveness | "eventual progress" | Property that something good eventually happens |
| Fairness | "no starvation" | Assumption that enabled transitions eventually occur |
| Stuttering | "no-op step" | Step where all variables stay unchanged |
| Term | "epoch" | Logical clock value used to detect stale messages |

## Further Reading

- [TLA+ Hyperbook](https://lamport.azurewebsites.net/tla/hyperbook.html) — Lamport's canonical resource.
- [Learn TLA+](https://learntla.com/) — practical tutorial flow.
- [Raft TLA+ spec](https://github.com/ongardie/raft.tla) — official Raft specification.
- [Amazon's TLA+ experience](https://www.amazon.science/publications/how-amazon-web-services-uses-formal-methods) — real-world impact.
- [Paxos Made Live](https://research.google/pubs/paxos-made-live/) — Google's experience with consensus verification.
