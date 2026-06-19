# Service Discovery, Membership, Leader Election

> In a distributed system, services can't hard-code what they don't know — they must find each other and agree on who leads.

**Type:** Learn
**Languages:** Go
**Prerequisites:** Phase 11 lessons 01–16, especially lesson 09 (Raft consensus) and lesson 13 (Gossip/SWIM)
**Time:** ~60 minutes

## Learning Objectives

- Explain the service discovery problem: why hard-coded addresses fail in dynamic distributed systems.
- Compare DNS-based discovery vs. service registries (Consul, etcd, ZooKeeper, Eureka) along dimensions of freshness, health awareness, and consistency.
- Distinguish client-side discovery (client queries registry, load-balances) from server-side discovery (load balancer/router queries registry).
- Describe health checking strategies: TTL-based (heartbeats) vs. active (registry pings services), and the consequences of each for stale entries.
- Explain why leader election is needed (configuration consistency, distributed locks, task assignment).
- Implement the Bully algorithm: highest-ID node wins, failure triggers re-election.
- Explain ZooKeeper-style leader election using ephemeral nodes and watches.
- Explain etcd-style leader election using leases and key watches.
- Revisit Raft-based leader election (terms, timeouts, vote splitting) from lesson 09.
- Build a service registry with health checking and leader election in Go.

## The Problem

You run 15 instances of a payment service behind a load balancer. At 2 AM, three instances crash. The load balancer keeps routing traffic to them for 90 seconds — the DNS TTL hasn't expired yet. Customers see timeouts. By the time DNS updates, two more instances have been overwhelmed by the redirected load and crash too.

This is the service discovery problem: in a dynamic distributed system, how do services find each other when IPs change, instances start and stop, and failures are the norm? Hard-coding addresses doesn't work because the set of live instances is constantly changing.

Discovery is half the problem. The other half is **coordination**: among the live instances, which one should lead? Whether it's choosing a primary replica, acquiring a distributed lock, or assigning a task to exactly one worker, you need a mechanism to elect a leader — and re-elect when the leader fails.

Without service discovery and leader election, your distributed system either hard-codes brittle configuration (single point of failure) or descends into chaos (split-brain, duplicate work, lost updates).

## The Concept

### DNS-Based Discovery

The simplest approach to finding services is DNS. You give your service a domain name — `payment-service.internal` — and clients resolve it to an IP address.

```
Client → DNS lookup: payment-service.internal → 10.0.1.5
Client → connects to 10.0.1.5
```

DNS has three problems for dynamic systems:

1. **Stale data:** DNS records have a TTL (time-to-live). A typical TTL is 60–300 seconds. When an instance crashes, DNS returns its IP until the TTL expires. Clients connect to a dead service for up to the full TTL duration.
2. **No health checking:** DNS doesn't know whether the IP it returns responds to requests. It's a name-to-address mapping, not a liveness tracker.
3. **No semantic awareness:** DNS can't express "this instance handles read traffic but not writes" or "this instance is in zone us-east-1a." You get an IP, nothing more.

DNS works for relatively static infrastructure. For microservices that scale up and down by the minute, it's too slow and too blind.

### Service Registries

A **service registry** is a dedicated store where services register their existence and clients discover them. Unlike DNS, a registry tracks liveness and metadata.

```
┌──────────────────────────────────────────────────┐
│                  Service Registry                 │
│                                                   │
│  payment-service:                                 │
│    10.0.1.5:8080  [healthy]  zone=east  v2       │
│    10.0.1.6:8080  [healthy]  zone=west  v2       │
│    10.0.1.7:8080  [unhealthy]  → deregistering   │
│                                                   │
│  inventory-service:                              │
│    10.0.2.3:8081  [healthy]  zone=east  v1       │
└──────────────────────────────────────────────────┘
```

Key properties:

| | DNS | Service Registry |
|---|---|---|
| Freshness | Seconds to minutes | Near real-time (sub-second) |
| Health awareness | None | Active or TTL-based checking |
| Metadata | None (just IP) | Tags, weights, versions |
| Consistency | Eventually consistent (caches) | Configurable (strong via Raft, eventual via gossip) |

Production registries:

- **ZooKeeper** (2006): Hierarchical namespace, strong consistency via Zab protocol. Used by Kafka (historically), HBase, and many Hadoop-era systems.
- **etcd** (2013): Key-value store, strong consistency via Raft. The backing store for Kubernetes services.
- **Consul** (2014): Key-value + service discovery + health checking. Uses Raft for consensus + gossip for membership. Supports multi-datacenter.
- **Eureka** (2012): Netflix's eventually-consistent registry. Optimized for availability over consistency — nodes can serve stale data during partitions.

### Client-Side Discovery

In **client-side discovery**, the client queries the registry and decides which instance to call:

```
Client ──query──→ Registry: "give me payment-service instances"
Client ←──response── Registry: [10.0.1.5, 10.0.1.6, 10.0.1.8]

Client ──request──→ 10.0.1.6 (client picks via round-robin, random, weighted, etc.)
```

The client embeds a load-balancing library (Netflix Ribbon was the canonical example). The client is responsible for:
1. Querying the registry.
2. Caching the instance list.
3. Choosing which instance to call (load-balancing policy).
4. Retrying on failure.

**Pros:** No single point of failure — the client decides locally. Lower latency — no intermediary hop.

**Cons:** The client must speak the registry protocol and implement load balancing. Every language needs a client library. The client must handle registry failures gracefully (fall back to cache).

### Server-Side Discovery

In **server-side discovery**, the client talks to a load balancer or router, which queries the registry:

```
Client ──request──→ Load Balancer ──query──→ Registry
                                       ←──response── Registry: [10.0.1.5, 10.0.1.6]
                  Load Balancer → picks 10.0.1.5
                  Load Balancer ──request──→ 10.0.1.5:8080
Client ←──response── 10.0.1.5:8080
```

AWS Application Load Balancer with Consul, or Kubernetes Service + kube-proxy, are examples.

**Pros:** Client is simple — it just calls a single endpoint. No client library needed.

**Cons:** The load balancer is a potential bottleneck and single point of failure (mitigated by running multiple LBs). Every request adds one network hop.

### Health Checking

A registry is only as good as its health data. Two strategies:

**TTL-based (heartbeats):** The service periodically sends a heartbeat ("I'm alive") to the registry. If the registry doesn't receive a heartbeat within the TTL, the service is considered dead and deregistered.

```
Service ──heartbeat──→ Registry (TTL = 10s)
  ... 5 seconds later ...
Service ──heartbeat──→ Registry (TTL reset to 10s)
  ... 15 seconds later, no heartbeat ...
Registry: "TTL expired for 10.0.1.7" → deregister
```

Used by: Eureka, etcd leases.

**Pros:** Service controls when it's considered dead. Simple to reason about.
**Cons:** Network partitions can cause false positives (service is alive but heartbeats aren't reaching the registry). Also, stale registrations persist if the registry crashes.

**Active (registry pings services):** The registry periodically pings each registered service. If a service doesn't respond, it's marked unhealthy.

```
Registry ──HTTP GET /health──→ Service on 10.0.1.7
  Service responds 200 OK → still healthy
Registry ──HTTP GET /health──→ Service on 10.0.1.7
  No response (timeout) → marked unhealthy
  ... next check ...
Registry ──HTTP GET /health──→ Service on 10.0.1.7
  No response → deregistered
```

Used by: Consul (TCP/HTTP/gRPC health checks).

**Pros:** Registry is in control — it decides when a service is dead. Can check application-level health (e.g., `/health` endpoint verifies database connectivity, not just process liveness).
**Cons:** Registry must know how to health-check each service type. Scales poorly — a registry tracking 10,000 services with 5-second intervals sends 2,000 health-check requests per second.

Most production systems use a hybrid: TTL heartbeats for basic liveness + active checks for application-level health.

### Leader Election

Many distributed systems need exactly one node to act as the **leader**: the primary replica, the lock holder, the task coordinator. Without a leader, you risk split-brain (two nodes think they're in charge) or no coordination (nobody decides).

Why you need a single leader:

1. **Configuration consistency:** Only the leader can update shared configuration, preventing conflicting updates.
2. **Distributed lock:** The leader holds a lock, ensuring exclusive access to a resource (e.g., only one node processes a queue).
3. **Task assignment:** The leader assigns work, preventing duplicate processing.

### Bully Algorithm

The **Bully algorithm** is a deterministic leader election protocol for systems where every node knows every other node's ID:

```
Node IDs: 1, 2, 3, 4, 5  (higher ID = higher priority)

Current leader: Node 5

Node 5 crashes.
Node 3 notices (timeout on heartbeat from 5).

Election:
  Node 3 sends ELECTION to all higher-ID nodes: 4, 5
  Node 4 responds OK (Node 5 doesn't respond — it's dead)
  Node 4 sends ELECTION to Node 5 (no response)
  Node 4 gets no OK responses → it becomes leader
  Node 4 broadcasts COORDINATOR(4) to all nodes

Result: Node 4 is the new leader.
```

Rules:
1. When a node detects the leader has failed, it starts an election by sending an `ELECTION` message to all nodes with higher IDs.
2. If any higher-ID node responds with `OK`, the initiating node waits for a `COORDINATOR` message — a higher-ID node will become the leader.
3. If no higher-ID node responds (they're all dead or unreachable), the initiating node becomes the leader and broadcasts `COORDINATOR(self)`.
4. When a node receives a `COORDINATOR(p)` message, it accepts `p` as the leader.

The name "Bully" comes from the fact that the highest-ID node always wins — it "bullies" lower-ID nodes into accepting its leadership.

**Properties:**
- **Deterministic:** The highest live node always becomes the leader. No randomization needed.
- **Message complexity:** O(N²) in the worst case (every node detects failure simultaneously and sends messages to all higher nodes).
- **Requires:** Every node knows every other node's ID. Not suitable for large, dynamic clusters.
- **Failure assumption:** Assumes reliable links between live nodes. If a higher-ID node is network-partitioned but alive, it will become leader when the partition heals — potentially causing split-brain.

### ZooKeeper-Style Leader Election

ZooKeeper provides a coordination service with strong consistency (Zab protocol). Leader election uses **ephemeral nodes** and **watches**:

```
Step 1: Each candidate creates an ephemeral sequential node:
  /election/candidate_0000000001  ← created by Node A
  /election/candidate_0000000002  ← created by Node B
  /election/candidate_0000000003  ← created by Node C

Step 2: The candidate with the lowest sequence number becomes leader.
  → Node A is the leader (sequence 0000000001)

Step 3: Each non-leader watches the node just before it:
  Node B watches /election/candidate_0000000001
  Node C watches /election/candidate_0000000002

Step 4: If Node A (leader) crashes:
  - Its ephemeral node is automatically deleted (ZooKeeper deletes ephemeral nodes when the session that created them disconnects)
  - Node B is notified via its watch
  - Node B checks if it now has the lowest sequence number → it becomes leader
```

**Why ephemeral nodes?** An ephemeral node exists only as long as the ZooKeeper session that created it is alive. If the process crashes or the network connection drops, the node disappears automatically — no manual cleanup needed. This is the key primitive that makes ZK leader election elegant.

**Why sequential?** ZooKeeper appends a monotonically increasing counter to the node path. This guarantees a total ordering — the lowest-numbered node is unambiguously the leader.

**Why watch the previous node?** Watching only the node immediately before yours creates a **herd effect** avoidance chain. If all non-leaders watched the leader, the leader's crash would trigger all N-1 nodes simultaneously. By watching only the previous node, exactly one node is notified when the leader fails.

Properties:
- **Strong consistency:** ZooKeeper uses the Zab consensus protocol, guaranteeing that all nodes see the same sequence of ephemeral nodes.
- **Automatic cleanup:** Ephemeral nodes vanish when sessions expire. No stale leader registrations.
- **Herd effect avoidance:** Only one node transitions at a time.
- **Drawback:** ZooKeeper is a separate infrastructure dependency. Its coordination API is powerful but complex.

### etcd-Style Leader Election

etcd uses **leases** for leader election, which is conceptually similar to ZooKeeper's ephemeral nodes but built on a simpler key-value primitive:

```
Step 1: Node A creates a lease with TTL = 10 seconds:
  etcdctl lease grant 10
  Lease ID: 758790

Step 2: Node A attempts to create the leader key with its lease:
  etcdctl put /leader nodeA --lease=758790
  (Succeeds if key doesn't exist → Node A is leader)

Step 3: Other nodes watch the leader key:
  etcdctl watch /leader

Step 4: Node A keeps its lease alive by periodically renewing it:
  etcdctl lease keep-alive 758790

Step 5: If Node A crashes, its lease expires → the key is automatically deleted.
  Nodes watching the key are notified.
  They race to create the key with their own lease.
  First to create wins.
```

The lease is etcd's analog of ZooKeeper's ephemeral node — it's a time-limited claim that expires if the owner stops renewing it. Combined with the `Create` conditional (create the key only if it doesn't exist), this gives a clean leader election primitive.

### Raft-Based Leader Election (Revisited from Lesson 09)

In lesson 09, you implemented Raft's leader election. Here's the recap in the context of service coordination:

Raft elects a leader using **terms**, **randomized timeouts**, and **majority voting**:
1. A follower that doesn't hear from the current leader within the election timeout becomes a candidate.
2. The candidate increments its term, votes for itself, and sends `RequestVote` RPCs to all other nodes.
3. If a candidate receives votes from a majority, it becomes the leader.
4. Split votes are resolved by randomized timeouts — the candidate with the shorter timeout restarts first and collects votes before others wake up.

Raft election is built into systems like etcd and Consul. You don't run a separate leader election algorithm — the consensus protocol itself produces a leader.

**Comparison of leader election approaches:**

| | Bully | ZK Ephemeral | etcd Lease | Raft (built-in) |
|---|---|---|---|---|
| Deterministic | Yes (highest ID) | No (first to create node) | No (first to create key) | No (first to win majority) |
| Requires external service | No | Yes (ZooKeeper) | Yes (etcd) | Built into consensus |
| Split-brain risk | Yes (if network partition) | No (ZK is CP) | No (etcd is CP) | No (majority prevents) |
| Message complexity | O(N²) worst case | O(N) per election | O(N) per election | O(N) per election |
| Automatic cleanup | No | Yes (ephemeral nodes) | Yes (lease expiry) | Yes (term expiry) |

## Build It

We'll build a service registry with health checking and two leader election algorithms (Bully and simulated ZooKeeper-style) in Go. See `code/main.go` for the full implementation.

### Step 1: Service Node and Registry

Define `ServiceNode` (ID, address, health status, TTL) and `ServiceRegistry` (register, deregister, discover by name).

### Step 2: Health Checking

Implement TTL-based health checking: nodes send heartbeats, and a background goroutine expires nodes whose TTL has lapsed. Also implement active health checking: the registry pings each service's `/health` endpoint.

### Step 3: Bully Algorithm

Implement the Bully algorithm: nodes have IDs, an election is triggered when the current leader is detected as failed, and the highest-ID live node becomes the new leader.

### Step 4: ZooKeeper-Style Leader Election

Simulate ZooKeeper's ephemeral sequential nodes: candidates create sequential entries, the lowest-numbered entry wins, and nodes watch the entry immediately before theirs.

### Step 5: Demo

Register three services, discover them by name, run health checks, deregister unhealthy services, and elect a leader using both the Bully algorithm and the ZK-style approach.

## Use It

**HashiCorp Consul** is the most full-featured production service discovery system. It combines:
- A strongly consistent key-value store (Raft) for configuration and leader election.
- Gossip-based membership (SWIM) for cluster health.
- TTL and active health checking (HTTP, TCP, gRPC, Docker).
- Multi-datacenter support via WAN gossip.

Compare our registry against Consul:

| Our Registry | Consul |
|---|---|
| In-memory | Persistent (Raft log + snapshots) |
| TTL + simulated HTTP check | TTL + HTTP/TCP/gRPC/Docker checks |
| Single-node | Multi-node Raft cluster |
| Bully + ZK-style election | Built-in Raft election (from consensus protocol) |
| No multi-datacenter | WAN gossip federation |

**Kubernetes** uses etcd as its backing store. Services are defined as `Service` objects, and `kube-proxy` on each node watches etcd for changes and updates iptables/IPVS rules locally. This is server-side discovery: the client connects to a ClusterIP, and kube-proxy routes to a healthy pod.

**Netflix Eureka** represents the other extreme: eventually consistent, peer-to-peer replication. Each Eureka node can serve stale data. This means Eureka never blocks during a partition — but clients might route to dead instances. Netflix designed it this way because their microservices are designed to tolerate stale registry data (circuit breakers, retries).

## Read the Source

- [Consul agent/agent.go](https://github.com/hashicorp/consul/blob/main/agent/agent.go) — the Consul agent that manages service registration, health checking, and coordination. Look at how it orchestrates local state with Raft consensus.
- [etcd clientv3/concurrency/election.go](https://github.com/etcd-io/etcd/blob/main/client/v3/concurrency/election.go) — etcd's built-in leader election using leases. Look at `Campaign()` to see how it creates a key with a lease.
- [ZooKeeper Recipes: Leader Election](https://zookeeper.apache.org/doc/current/recipes.html#sc_leaderElection) — the official ZooKeeper recipe. Compare the ephemeral sequential approach against our implementation.

## Ship It

The reusable artifact lives in `outputs/`: a Go service registry with TTL-based health checking, active health checks, Bully algorithm leader election, and ZK-style leader election.

## Exercises

1. **Easy** — Add weights to service nodes (e.g., `weight: 3` gets 3x the traffic). Implement weighted round-robin in `Discover()`.
2. **Medium** — Implement the ring hash (consistent hashing) service selection from lesson 18 in this registry. Clients hash the request key to a ring position and select the nearest healthy node.
3. **Hard** — Implement a simple Raft-based leader election that uses the registry itself as the coordination store. The leader writes its ID to a key with a lease, and other nodes watch the key. When the lease expires (leader crashed), re-election happens. How does this differ from the ZK ephemeral-node approach?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Service discovery | "finding services" | The mechanism by which services locate each other in a dynamic system — either via DNS (slow, no health), a registry (fast, health-aware), or sidecar proxies |
| Service registry | "a DNS for microservices" | A centralized or distributed store where services register themselves and discover others (Consul, etcd, ZooKeeper, Eureka) |
| Client-side discovery | "client picks" | The client queries the registry and load-balances itself. Requires a client library. No intermediary hop. |
| Server-side discovery | "load balancer picks" | The client talks to a load balancer/router, which queries the registry and forwards. Simple client, extra hop. |
| TTL-based health check | "heartbeat" | The service sends periodic heartbeats. If the registry doesn't receive one within the TTL, the service is deregistered. Fast detection, but false positives during partitions. |
| Active health check | "registry pings service" | The registry probes each service's health endpoint. Application-level health (e.g., database connectivity). Scales poorly — O(N) probes. |
| Ephemeral node | "temporary znode" | A ZooKeeper node that is automatically deleted when the session that created it disconnects. The key primitive for ZK leader election and distributed locks. |
| Bully algorithm | "highest ID wins" | A deterministic leader election algorithm where the node with the highest ID among live nodes becomes leader. O(N²) message complexity. Vulnerable to split-brain. |
| Lease | "time-limited lock" | A time-bounded claim on a resource (key, lock, leadership). If the holder crashes, the lease expires and the resource is released automatically. Used by etcd for leader election. |

## Further Reading

- [ZooKeeper Wait-free Coordination](https://zookeeper.apache.org/doc/current/zookeeperOver.html) — the ZooKeeper overview. Read for the ephemeral node and watch primitives.
- [etcd Lease API](https://etcd.io/docs/v3.5/learning/api/#lease-api) — how etcd leases work for leader election and distributed locks.
- [Netflix Eureka](https://github.com/Netflix/eureka/wiki) — the eventually-consistent service registry. Read for the trade-off between availability and consistency in service discovery.
- [Consul Service Discovery](https://developer.hashicorp.com/consul/docs/discovery) — how Consul combines Raft (strong consistency for KV) with SWIM gossip (eventual consistency for membership).
- [Bully Algorithm — Garcia-Molina, 1982](https://en.wikipedia.org/wiki/Bully_algorithm) — the original paper. Read for the correctness proof and the "election" vs "coordinator" message distinction.