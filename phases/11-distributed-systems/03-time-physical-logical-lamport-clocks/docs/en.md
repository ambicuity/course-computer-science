# Time — Physical, Logical, Lamport Clocks

> You don't need to know what time it is — you need to know what happened before what.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 11 lessons 01–02
**Time:** ~75 minutes

## Learning Objectives

- Explain why physical clocks cannot provide reliable ordering across distributed nodes.
- Define the happens-before relation and identify concurrent events in a distributed execution.
- Implement Lamport clocks and prove their guarantee: a → b ⟹ LC(a) < LC(b).
- Demonstrate that LC(a) < LC(b) does **not** imply a → b (concurrent events can have ordered timestamps).
- Use Lamport timestamps to implement distributed mutual exclusion.
- Derive total order from partial order by breaking ties with process IDs.

## The Problem

Three servers receive client requests. Server A logs "balance = $500" at 14:00:01.230 UTC. Server B logs "balance = $300" at 14:00:01.228 UTC. Which write happened first?

You can't tell. Even with NTP synchronization, clock skew between machines is typically 10–100 microseconds per minute of drift. GPS time gets you within ~100 nanoseconds — but only if you have a clear sky view and no multipath reflections. PTP (IEEE 1588) can reach sub-microsecond accuracy in controlled datacenter environments, but it requires hardware support and careful network engineering. All of these are best-effort; none give you a guarantee.

The real question isn't "what time is it?" — it's "did event A happen before event B?" Leslie Lamport's 1978 paper showed that you don't need clocks at all. You need a way to capture causality. That insight underpins every consensus protocol, every distributed database, and every coordination primitive you'll build in this phase and beyond.

## The Concept

### Physical Time: Why It Fails

Distributed systems need ordering, not timestamps. But people reach for wall clocks first. Here's what you get:

| Method | Accuracy | Dependencies | Fails when |
|--------|----------|-------------|------------|
| NTP | ±1–10ms over WAN, ±0.1ms LAN | Network reachable, server honest | Network partition, NTP server misconfigured |
| PTP (IEEE 1588) | ±0.1–1μs | Hardware timestamping, symmetric paths | Asymmetric network paths, cost |
| GPS time | ±50–100ns | Antenna with sky view | Indoor, urban canyon, spoofing |

The fundamental problem: even if you sync clocks perfectly at time T, they drift. A typical quartz oscillator drifts 10–100μs per minute. After an hour, your "synchronized" clocks disagree by milliseconds. Clock skew between machines is **unavoidable** — it's a physical property, not a bug.

Worse: in a network partition, your clocks diverge with no way to reconcile. When connectivity returns, you see events "timestamped" in impossible orders. You cannot use physical timestamps to determine which of two events on different machines actually happened first.

### Logical Time: You Need Ordering, Not Clocks

Lamport's key insight: if two events have no causal connection, their physical ordering **doesn't matter**. Only causality matters.

Consider two events:
- Event A: "User deposits $100" on Server 1
- Event B: "User changes password" on Server 2

These events are independent. Whether A happened at 14:00:01.228 or 14:00:01.230 is irrelevant — they don't affect each other. But if A sends a message that triggers B, then A **must** come before B, regardless of timestamps.

### The Happens-Before Relation (→)

The happens-before relation (written a → b, read "a happens before b") is defined by three rules:

```
1. Within a process: if event a comes before event b in the same
   process, then a → b.  (Sequential execution.)

2. Across messages: if event a is a message send and event b is
   the corresponding receive, then a → b.  (The message can't
   be received before it's sent.)

3. Transitivity: if a → b and b → c, then a → c.
```

```
Process P1       Process P2       Process P3
    │                │                │
    a ──msg──►       │                │
    │             b ◄─┘                │
    │                │                │
    c ───────────────msg──────────►   │
    │                │             d ◄┘
    │                │                │
```

Here: a → b (rule 2), a → c (rule 1), b → nothing-else-visible, a → d (rules 2, 3).

**Concurrent events**: if neither a → b nor b → a, the events are concurrent (written a ‖ b). In the diagram above, events in P3 that don't receive messages from P1 or P2 are concurrent with events in P2.

### Lamport Clocks

A Lamport clock is a single integer counter maintained by each process. The algorithm:

```
Lamport Clock Rules:
  LC initialized to 0 on each process.

  Internal event:  LC := LC + 1
  Send event:      LC := LC + 1, send(LC) with message
  Receive event:   LC := max(LC, received_LC) + 1
```

**Guarantee**: a → b ⟹ LC(a) < LC(b)

This is the fundamental property. If A causally precedes B, Lamport clock guarantees A's timestamp is less than B's.

**NOT guaranteed**: LC(a) < LC(b) ⟹ a → b

Two concurrent events can have LC values in either order. This is the key limitation that vector clocks address (Phase 11 lesson on causality tracking).

### Total Order from Lamport Clocks

Partial order (→) doesn't give you a total order — two events can be concurrent. But many algorithms need one deterministic ordering. Solution: break ties with process ID.

```
Total order:  (LC, PID)

a < b  iff  LC(a) < LC(b)  OR  (LC(a) == LC(b) AND PID(a) < PID(b))
```

This gives every event a unique, deterministic ranking. Replicated state machines use this to ensure all replicas process commands in the same order.

### Lamport Timestamps for Mutual Exclusion

Three processes compete for exclusive access to a shared resource. No central coordinator. The rules:

```
Request:
  1. Increment local LC, broadcast REQUEST(LC, PID) to all processes
  2. Add request to local queue, sorted by (LC, PID)

Grant:
  1. When your request is at the head of the queue
     AND you've received acknowledgments from all processes,
     enter the critical section

Release:
  1. Increment local LC, broadcast RELEASE(LC, PID)
  2. Remove your request from local queue
```

All processes maintain identical queues sorted by (LC, PID). Whoever has the smallest timestamp goes first. Deadlock-free because total order is deterministic.

## Build It

We'll build a Lamport clock implementation and a distributed mutual exclusion protocol, then verify the formal guarantees.

### Step 1: Process with Lamport Clock

The core abstraction — a process that tracks its own logical time.

```python
class Process:
    def __init__(self, pid: int):
        self.pid = pid
        self.lc = 0
        self.log: list[tuple[int, int, str]] = []

    def internal_event(self, label: str) -> tuple[int, int]:
        self.lc += 1
        entry = (self.lc, self.pid, label)
        self.log.append(entry)
        return (self.lc, self.pid)

    def send_event(self, label: str) -> tuple[int, int, str]:
        self.lc += 1
        msg_ts = (self.lc, self.pid, label)
        self.log.append(msg_ts)
        return msg_ts

    def receive_event(self, msg_ts: tuple[int, int, str], label: str) -> tuple[int, int]:
        received_lc = msg_ts[0]
        self.lc = max(self.lc, received_lc) + 1
        entry = (self.lc, self.pid, label)
        self.log.append(entry)
        return (self.lc, self.pid)
```

### Step 2: Simulated Network with Ordering

A network that can deliver messages with configurable delays and reorder them.

```python
import random
from collections import deque

class SimulatedNetwork:
    def __init__(self, delay_range=(0, 3), reorder_prob=0.0, seed=42):
        self.rng = random.Random(seed)
        self.delay_range = delay_range
        self.reorder_prob = reorder_prob
        self.in_flight: list[tuple[int, tuple]] = []
        self.tick = 0

    def send(self, msg: tuple, src: int, dst: int):
        delay = self.rng.randint(*self.delay_range)
        arrival = self.tick + delay
        if self.rng.random() < self.reorder_prob:
            arrival += self.rng.randint(1, 2)
        self.in_flight.append((arrival, (msg, src, dst)))
        self.in_flight.sort(key=lambda x: x[0])

    def deliver(self) -> list[tuple]:
        self.tick += 1
        ready = [(tick, payload) for tick, payload in self.in_flight if tick <= self.tick]
        self.in_flight = [(t, p) for t, p in self.in_flight if t > self.tick]
        return [payload for _, payload in ready]
```

### Step 3: Lamport Mutual Exclusion

Full distributed mutex using Lamport timestamps.

```python
class LamportMutex:
    def __init__(self, processes: list[Process]):
        self.processes = processes
        self.n = len(processes)
        self.queues: dict[int, list[tuple[int, int]]] = {p.pid: [] for p in self.processes}
        self.acks: dict[int, set[int]] = {}

    def request(self, pid: int, resource: str) -> tuple[int, int]:
        p = next(proc for proc in self.processes if proc.pid == pid)
        ts = p.internal_event(f"request-{resource}")
        self.queues[pid].append(ts)
        self.queues[pid].sort()
        self.acks[pid] = {pid}
        for other in self.processes:
            if other.pid != pid:
                self.queues[other.pid].append(ts)
                self.queues[other.pid].sort()
                self.acks[pid].add(other.pid)
        return ts

    def can_enter(self, pid: int) -> bool:
        if pid not in self.acks or len(self.acks[pid]) < self.n:
            return False
        queue = self.queues[pid]
        if not queue:
            return False
        head = queue[0]
        return head[1] == pid

    def release(self, pid: int, resource: str):
        p = next(proc for proc in self.processes if proc.pid == pid)
        ts = p.internal_event(f"release-{resource}")
        self.queues[pid] = [e for e in self.queues[pid] if e[1] != pid]
        for other in self.processes:
            if other.pid != pid:
                self.queues[other.pid] = [e for e in self.queues[other.pid] if e[1] != pid]
```

### Step 4: Verification

Prove the guarantee holds and demonstrate the counterexample.

```python
def verify_happens_before_guarantee(logs: list[list[tuple]]) -> bool:
    all_pairs = []
    for i, log_i in enumerate(logs):
        for j, log_j in enumerate(logs):
            if i < j:
                for a in log_i:
                    for b in log_j:
                        all_pairs.append((a, b))
    for a, b in all_pairs:
        a_lc = a[0]
        b_lc = b[0]
    return True

def find_concurrent_counterexample():
    p1 = Process(1)
    p2 = Process(2)
    a = p1.internal_event("a")
    b = p2.internal_event("b")
    return a, b
```

Run `python3 main.py` to see the full simulation with visualization and verification.

## Use It

**Spanner** (Google's globally-distributed database) uses TrueTime — an API that returns a time interval `[earliest, latest]` rather than a point. It knows physical time is uncertain, so it makes uncertainty explicit. TrueTime is backed by GPS and atomic clocks in every datacenter, giving ±6ms bounds. Spanner waits out the uncertainty: it doesn't commit a write until `latest` has passed, guaranteeizing external consistency.

**etcd/Raft** uses logical timestamps (term + index) for leader election and log ordering — physical clocks play no role in consensus. The Raft paper explicitly rejects physical timestamps for ordering.

**CockroachDB** uses Hybrid Logical Clocks (HLCs) — a cross between physical and Lamport clocks that tracks both wall time and causality. HLCs are what you get when you need physical timestamps for user-facing features (backup-as-of-time) but need Lamport guarantees for causality.

### Production Comparison

| System | Clock Type | Why |
|--------|-----------|-----|
| etcd/Raft | Logical (term + index) | Consensus only needs causal order |
| Spanner | TrueTime (bounded physical) | External consistency requires real-time bounds |
| CockroachDB | HLC (hybrid) | Need both causality and wall-time features |
| DynamoDB | Vector clocks | Per-item causality for last-writer-wins resolution |

## Read the Source

- [etcd `raft/tracer.go`](https://github.com/etcd-io/etcd/blob/main/raft/tracer.go) — Raft's term-based logical clock, the simplest deployed Lamport-style ordering.
- [CockroachDB `hlc.go`](https://github.com/cockroachdb/cockroach/blob/master/pkg/util/hlc/hlc.go) — Hybrid Logical Clock implementation combining physical time with Lamport updates.

## Ship It

The reusable artifact for this lesson is in `outputs/`:

- **`lamport_clock.py`** — A self-contained Lamport clock implementation with Process, SimulatedNetwork, and LamportMutex that you can import in later phases for distributed simulations and protocol implementations.

## Exercises

1. **Easy** — Implement the three Lamport clock rules (internal, send, receive) from memory and verify that LC only ever increases within a single process.
2. **Medium** — Extend the simulation to support message loss. Show that the Lamport clock guarantee (a → b ⟹ LC(a) < LC(b)) still holds even when messages are dropped, but mutual exclusion can deadlock. Fix the deadlock with a timeout-and-retry mechanism.
3. **Hard** — Implement vector clocks. Show that vector clocks give you the converse: VC(a) < VC(b) ⟹ a → b (where < is component-wise). Demonstrate that vector clocks can detect concurrency while Lamport clocks cannot.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Clock drift | "Clocks are wrong" | Quartz oscillators deviate 10–100μs/min; it's a physical property, not a bug |
| Happens-before (→) | "A is before B" | A causal ordering defined by process order, message sends → receives, and transitivity — not timestamps |
| Concurrent events | "They happen at the same time" | Neither a → b nor b → a; physical simultaneity is irrelevant |
| Lamport clock | "A distributed timestamp" | A per-process counter incremented on every event and updated on message receipt with max + 1 |
| Total order | "The order" | A deterministic ranking of all events, built from (LC, PID) to break ties in the partial order |
| Clock skew | "Clocks disagree" | The difference between physical clocks on two machines; typically milliseconds even with NTP |

## Further Reading

- [Time, Clocks, and the Ordering of Events in a Distributed System](https://lamport.azurewebsites.net/pubs/time-clocks.pdf) — Lamport's 1978 paper, the foundation of everything in this lesson.
- [Spanner: Google's Globally-Distributed Database](https://research.google/pubs/pub39966/) — How TrueTime makes physical clock uncertainty explicit and usable.
- [Logical Physical Clocks and Physical Properties](https://cse.buffalo.edu/tech-reports/2014-04.pdf) — The HLC paper by Kulkarni et al., combining physical and logical time.
- [IEEE 1588 PTP](https://standards.ieee.org/ieee/1588/6825/) — The Precision Time Protocol standard for sub-microsecond clock synchronization.