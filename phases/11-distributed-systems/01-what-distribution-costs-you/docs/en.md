# What Distribution Costs You

> Distribution is not a feature — it's a tax on every operation.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 10
**Time:** ~45 minutes

## Learning Objectives

- Recite the 8 fallacies of distributed computing and explain why each one makes distributed code harder than local code.
- Order common operations by latency (L1 cache → cross-region network) and estimate the cost of a request that crosses several layers.
- Compute the probability of at least K failures in an N-node cluster given a per-node failure rate.
- Explain partial failure — why sending a message and not hearing back is fundamentally ambiguous.
- State the CAP theorem and explain why you can't have consistency, availability, and partition tolerance simultaneously.
- Define idempotence and explain why every distributed operation must be idempotent.

## The Problem

You wrote a service that talks to a database. It works on your laptop. You deploy it to three nodes behind a load balancer and suddenly:

- Two nodes see different data after a network blip.
- A retry doubles a payment because the first request actually succeeded — you just didn't hear back.
- One node's outage cascades until the whole system is unreachable.

None of these are bugs in your code. They are the *cost of distribution*. Peter Deutsch and others at Sun Microsystems cataloged these surprises as the **8 Fallacies of Distributed Computing** — assumptions that every programmer makes when they first write networked code, and that are all false.

This lesson gives you the vocabulary and the numerical intuition to reason about that cost before you pay it.

## The Concept

### The 8 Fallacies of Distributed Computing

| # | Fallacy | Reality |
|---|---------|---------|
| 1 | The network is reliable | Cables get cut, switches fail, WiFi drops. Any message may never arrive. |
| 2 | Latency is zero | Every hop costs time. Even light in fiber takes ~5 ms per 1000 km. |
| 3 | Bandwidth is infinite | A single large payload can saturate a link. Netflix and your database share the same pipes. |
| 4 | The topology doesn't change | EC2 instances restart with new IPs. DNS entries change. Routers reroute. |
| 5 | There is one administrator | Your clusters span org boundaries with different policies, firewalls, and maintenance windows. |
| 6 | Transport cost is zero | Serialization, TLS handshakes, and kernel context switches all consume CPU and time. |
| 7 | The network is homogeneous | Mixed hardware, mixed firmware, mixed MTUs. TCP behaves differently across middleboxes. |
| 8 | **(Added by others)** The network is secure | Any node on the path can read, modify, or drop your packets. |

Every distributed bug traces back to at least one of these.

### Latency Numbers Every Programmer Should Know

These are orders of magnitude — memorize the ratios, not the exact numbers:

```
L1 cache reference              ~0.5 ns
Branch mispredict                ~5   ns
L2 cache reference               ~7   ns
Mutex lock/unlock               ~25   ns
Main memory reference          ~100   ns     (200× L1)
Compress 1 KB with Snappy     ~3,000 ns (3 μs)
Read 1 MB sequentially from SSD ~500 μs (0.5 ms)
Read 1 MB sequentially from HDD  ~10  ms
Send 1 KB over 1 Gbps network    ~10  μs
Round trip same datacenter      ~500  μs (0.5 ms)
Round trip cross-region        ~100    ms
Round trip cross-continent    ~150    ms
```

Key ratios to internalize:

- RAM is ~200× slower than L1.
- SSD is ~5,000× slower than RAM.
- Same-DC network is ~200× slower than SSD.
- Cross-region is ~200× slower than same-DC.

A single cross-region call costs the same as ~200,000 L1 reads. That's not free — that's your entire budget.

### Failure Modes

Distributed systems face four classes of failure:

| Mode | Behavior | Example |
|------|----------|---------|
| **Crash** | Node stops permanently | Kernel panic, power loss |
| **Hang** | Node stops temporarily | GC pause, stuck mutex |
| **Omission** | Node drops some messages | Network packet loss |
| **Byzantine** | Node behaves arbitrarily | Corrupted memory, bugs, malicious actor |

Crash and hang failures are indistinguishable remotely — you can't tell if a node is slow or dead. This is **partial failure**.

### Probability of Failure in Large Clusters

If each node fails independently with probability *p*, the probability that **at least one** node fails in a cluster of *N* nodes is:

```
P(at least 1 fail) = 1 - (1 - p)^N
```

With *p* = 0.01 and *N* = 100:

```
P(at least 1 fail) = 1 - 0.99^100 ≈ 0.634
```

There is a 63% chance that *some* node is down at any given time. For a quorum system (majority must be up), P(quorum lost) grows even faster.

### Partial Failure: The Fundamental Dilemma

You send a message to another node. The network silently drops it. Did the other node:

- Never receive it? (No work was done.)
- Receive it and succeed? (Work was done, you just don't know.)
- Receive it and crash halfway? (Partial work was done.)

You **cannot distinguish** these cases. This is the fundamental dilemma of distributed computing. Every retry risks double execution. Every timeout risks abandoning a completed operation.

### CAP Theorem (Preview)

```
        Consistency
           /\
          /  \
         /    \
        /______\
   Availability  Partition
                  Tolerance
```

A distributed system can provide **at most two** of:

- **Consistency (C):** Every read returns the most recent write.
- **Availability (A):** Every non-failing node responds (not an error).
- **Partition tolerance (P):** The system continues despite network splits.

Since partitions *will* happen (Fallacy #1), the real choice is **CP** (reject requests during partition) or **AP** (accept stale reads during partition). You cannot have all three.

### Network Partitions and Split-Brain

A **partition** splits the network into groups that can't communicate:

```
Group A: [N1] [N2] [N3]  ←→  Group B: [N4] [N5] [N6]
              ✕ network break ✕
```

Both groups may accept writes independently → **split-brain**. When the partition heals, you must reconcile conflicting updates.

Detection: heartbeat timeouts, phi accrual detectors (used by Akka, Cassandra). But you can never be certain a node is down — it might just be slow (Fallacy #2).

### Idempotence: Retries Are Inevitable

Because of partial failure, you **must** retry. Because you retry, your operations **must be idempotent**:

> An operation is **idempotent** if executing it once has the same effect as executing it any number of times.

Examples:
- `SET balance = 100` — idempotent
- `ADD 10 TO balance` — **not** idempotent (retry doubles the addition)
- `SET balance = 100 IF version = 5` — idempotent (conditional write)

Design every distributed operation so that a retry is safe.

## Build It

We'll build a latency and failure simulator in Python. Run it with `python3 main.py`.

### Step 1: Latency Reference Table

The `LatencyNumbers` class provides the reference latencies:

```
L1 cache       ~1 ns
Main memory   ~100 ns
SSD read     ~100 μs
Same-DC net  ~500 μs (0.5 ms)
Cross-region   ~100 ms
```

### Step 2: Latency Simulator

The `LatencySimulator` takes a sequence of operations (e.g., `["l1", "ram", "ssd", "network_dc"]`) and computes total latency, human-readable breakdown, and the ratio to a local-only request.

### Step 3: Failure Simulator

The `FailureSimulator` computes P(at least K fail) for N nodes with per-node failure probability p:

```
P(no failures) = (1 - p)^N
P(at least 1 failure) = 1 - (1 - p)^N
P(quorum lost) for majority = sum over k > N/2 of C(N,k) * p^k * (1-p)^(N-k)
```

### Step 4: Network Partition Simulation

The `NetworkPartition` class simulates two groups losing connectivity, showing split-brain writes and reconciliation costs.

### Step 5: Architecture Comparison

The demo compares monolith, microservice, and distributed request costs, showing how each distribution boundary adds latency tax.

## Use It

**etcd** (the key-value store powering Kubernetes) embodies every lesson here:

- It uses Raft for consensus (CAP: chose C+P, sacrifices A during partitions).
- Every write goes through the Raft log (idempotent by sequence number).
- It detects leader failures via heartbeat timeouts (partial failure detection).
- Code reference: [etcd server apply](https://github.com/etcd-io/etcd/blob/main/server/etcdserver/api/etcdhttp/serve.go) — notice the timeout and retry logic wrapping every operation.

Compare to our simulator: etcd's Raft consensus adds at least one same-DC round trip (~500 μs) per write before it returns success. That's the distribution tax.

## Read the Source

- [etcd/raft — state.go](https://github.com/etcd-io/etcd/blob/main/server/etcdserver/raft.go) — the Raft state machine that decides Consistency over Availability during partitions.

## Ship It

The reusable artifact is a **latency and failure simulator** CLI in `outputs/`. It ships as a single `main.py` that you can import as a library or run as a script:

- `python3 main.py` — full demo
- `from main import LatencyNumbers, LatencySimulator, FailureSimulator, NetworkPartition` — reuse in later phases

## Exercises

1. **Easy** — Run the simulator with a monolith request (L1 → RAM only). Compare the time to a cross-region request. Calculate the ratio.
2. **Medium** — Modify `FailureSimulator` to compute P(at least K fail) for K = 1, 2, 3 with N = 10, p = 0.05. At what cluster size does P(quorum lost) exceed 50% for a 5-node Raft group (quorum = 3)?
3. **Hard** — Extend `LatencySimulator` with a `retry_cost` method: given an operation that succeeds with probability *p* and retries on failure up to *n* times, compute the expected total latency. What's the expected cost of a same-DC RPC that has a 5% failure rate with 3 retries?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Latency | "It's slow" | Time for a single operation from request to response |
| Throughput | "It handles lots of requests" | Operations per unit time — independent of single-request latency |
| Partial failure | "The network dropped my message" | The fundamental ambiguity: you don't know if the other side processed your request or not |
| Idempotent | "Safe to retry" | An operation where f(x) = f(f(x)) — executing it N times has the same effect as executing it once |
| CAP theorem | "Pick two of C, A, P" | Given that partitions are inevitable, you must sacrifice either consistency or availability — you never get both during a partition |
| Split-brain | "Both sides think they're primary" | Two partitions both accept writes independently, producing conflicting state |
| Quorum | "Majority vote" | More than half of nodes must agree — the mechanism that prevents split-brain decisions |

## Further Reading

- [The 8 Fallacies of Distributed Computing](https://en.wikipedia.org/wiki/Fallacies_of_distributed_computing) — Peter Deutsch's original list, expanded by James Gosling and others at Sun.
- [Latency Numbers Every Programmer Should Know](https://colin-scott.github.io/blog/2012/12/05/latency-numbers-everyone-should-know/) — Jeff Dean's numbers, interactively visualized by Colin Scott.
- [CAP Twelve Years Later](https://www.infoq.com/articles/cap-twelve-years-later-how-the-rules-have-changed/) — Eric Brewer revisits the theorem and clarifies that the choice is only during partitions.
- [Designing Data-Intensive Applications](https://dataintensive.net/) — Martin Kleppmann's book, Chapters 8–9 cover distributed trouble in depth.