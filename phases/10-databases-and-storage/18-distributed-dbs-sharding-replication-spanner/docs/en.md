# Distributed DBs — Sharding, Replication, Spanner

> One machine is never enough.

**Type:** Build
**Languages:** Go
**Prerequisites:** Phase 10 lessons 01–17 (B-tree, LSM-tree, MVCC, transactions)
**Time:** ~75 minutes

## Learning Objectives

- Explain why a single machine can't serve a global application and what distributed databases do about it.
- Implement consistent hashing with virtual nodes for shard-aware key routing.
- Compare sharding strategies (hash, range, directory) and replication models (leader-follower, multi-leader, leaderless).
- Describe how Google Spanner uses TrueTime + 2PC + Paxos to achieve external consistency at planet scale.

## The Problem

You've built a fast single-node KV store with MVCC (lesson 17). It does 50k writes/sec and serves 100k reads/sec. Your startup takes off — users in New York, London, Tokyo, Sydney. Now one machine can't hold all the data, and when it goes down at 3 AM, the whole service is dark for hours.

Worse: a user in Tokyo hits 250 ms latency because every read goes to your us-east-1 server. You need the data *near* them. And you need it to survive a rack failure, a power outage, or someone tripping over the cable.

This is the fundamental distributed database problem: **scale** (more data than one disk), **availability** (no single point of failure), and **latency** (data close to users). You can't solve any of them on one node.

## The Concept

### Sharding (Horizontal Partitioning)

Sharding splits a dataset across N machines. Each machine owns a subset of the keyspace. Three common strategies:

| Strategy | How it works | Pro | Con |
|----------|-------------|-----|-----|
| **Hash-based** | `hash(key) % N` → node | Even distribution | Resharding moves almost all keys |
| **Range-based** | key `[a–m)` → node 1, `[m–z)` → node 2 | Range scans, prefix queries | Hot spots if writes cluster |
| **Directory-based** | Lookup service maps key → node | Flexible, tunable | Extra hop, SPOF |

**Consistent hashing** fixes the resharding problem: each key hashes to a point on a ring (0–2⁶⁴). Nodes also hash onto the ring. Each key goes to the *next* node clockwise. When a node joins or leaves, only its *neighbors* on the ring are affected — not all keys.

Virtual nodes (vnodes): each real node occupies multiple points on the ring. This smooths out distribution when nodes have different capacities.

### Replication

Replication copies data across multiple nodes for fault tolerance and read scaling.

- **Leader-follower (single-writer):** One leader handles all writes, replicates log to followers. Followers serve reads (stale reads allowed) or stay as standbys. Simple, no conflicts. Used by PostgreSQL, MySQL, Redis.
- **Multi-leader (multiple writers):** Several nodes accept writes, each propagates changes to others. Conflict resolution required (last-writer-wins, CRDTs, application merge). Used by Google Docs, MySQL Group Replication.
- **Leaderless (Dynamo-style):** Any node accepts writes. `W` nodes must acknowledge a write, `R` nodes for a read. Read repair and hinted handoff ensure eventual consistency. Used by Cassandra, DynamoDB.

### Consensus for Replication

**Raft / Paxos** give *strong consistency*: all nodes agree on the same log order. The write is committed only after a majority acknowledges. Etcd, Spanner, and CockroachDB use this. Latency is higher (need N/2 + 1 round trips), but the programmer sees a single, correct state.

**Eventual consistency** (Dynamo/Cassandra) gives higher availability: writes always succeed, conflicts resolved later. Reads might see stale data. The programmer must handle this (or use quorum `R + W > N`).

### Distributed Transactions

**2PC (Two-Phase Commit):**
- Phase 1 (Prepare): coordinator asks all participants "can you commit?"
- Phase 2 (Commit): if all say yes, coordinator says "commit"; otherwise "abort".
- Problem: if the coordinator crashes after Prepare, participants are *blocked* indefinitely holding locks.

**3PC (Three-Phase Commit):**
- Adds a "pre-commit" phase so participants can abort unilaterally after a timeout.
- Non-blocking in theory, but in practice network partitions cause liveness failures. Rarely used.

### Google Spanner

Spanner is Google's globally distributed SQL database. It provides **external consistency** (linearizability across the planet) using:

- **TrueTime API:** Exposes a time interval `[earliest, latest]` with bounded clock uncertainty (~7 ms). Spanner assigns commit timestamps that respect real-time ordering: if transaction A finishes before B starts, A's commit timestamp is strictly less than B's.
- **2PC + Paxos per shard:** Each shard (a Paxos group) runs 2PC for cross-shard transactions. Paxos handles group membership and fault tolerance. If the leader of a Paxos group fails, a new one is elected in seconds.
- **F1 RDBMS:** A SQL layer on top of Spanner that provides the relational model, schema, and joins.

Spanner architecture: data is partitioned into *directories* (key ranges). Directories are grouped into *tablets*. Each tablet is replicated across zones via Paxos. A *Paxos group* manages one tablet. Writes go through the Paxos leader to ensure ordering within the group. Cross-group transactions use 2PC with the Paxos leaders as participants.

### CockroachDB (Open-Source Spanner)

CockroachDB is inspired by Spanner but uses **Hybrid Logical Clocks (HLC)** instead of TrueTime. HLC is a software clock that combines physical time and a logical counter. It provides causally-consistent timestamps without atomic clocks or GPS receivers. CockroachDB also uses 2PC + Raft per range.

## Build It

We'll build three components in Go:

1. **ConsistentHashRing** — add/remove nodes with vnodes, route keys to the correct node.
2. **ShardedKV** — a distributed KV store that uses the hash ring to find the right node.
3. **Simplified Raft** — leader election, log replication, commit index.

### Step 1: Consistent Hash Ring with Virtual Nodes

```go
package main

import (
	"crypto/sha256"
	"encoding/binary"
	"fmt"
	"sort"
	"strconv"
)

type VNode struct {
	ID   uint64
	Node string
}

type ConsistentHashRing struct {
	vnodes  []VNode
	nodes   map[string]int
	replica int
}

func hash(key string) uint64 {
	h := sha256.Sum256([]byte(key))
	return binary.BigEndian.Uint64(h[:8])
}

func NewRing(replica int) *ConsistentHashRing {
	return &ConsistentHashRing{
		vnodes:  []VNode{},
		nodes:   make(map[string]int),
		replica: replica,
	}
}

func (r *ConsistentHashRing) AddNode(node string) {
	r.nodes[node] = 0
	for i := 0; i < r.replica; i++ {
		id := hash(node + ":" + strconv.Itoa(i))
		r.vnodes = append(r.vnodes, VNode{ID: id, Node: node})
	}
	sort.Slice(r.vnodes, func(i, j int) bool {
		return r.vnodes[i].ID < r.vnodes[j].ID
	})
}

func (r *ConsistentHashRing) RemoveNode(node string) {
	delete(r.nodes, node)
	filtered := []VNode{}
	for _, v := range r.vnodes {
		if v.Node != node {
			filtered = append(filtered, v)
		}
	}
	r.vnodes = filtered
}

func (r *ConsistentHashRing) GetNode(key string) string {
	if len(r.vnodes) == 0 {
		return ""
	}
	h := hash(key)
	idx := sort.Search(len(r.vnodes), func(i int) bool {
		return r.vnodes[i].ID >= h
	})
	if idx == len(r.vnodes) {
		idx = 0
	}
	return r.vnodes[idx].Node
}

func (r *ConsistentHashRing) Nodes() []string {
	ns := []string{}
	for n := range r.nodes {
		ns = append(ns, n)
	}
	sort.Strings(ns)
	return ns
}
```

### Step 2: Sharded KV Store

```go
type ShardedKV struct {
	ring  *ConsistentHashRing
	stores map[string]map[string]string
}

func NewShardedKV(replica int) *ShardedKV {
	return &ShardedKV{
		ring:   NewRing(replica),
		stores: make(map[string]map[string]string),
	}
}

func (kv *ShardedKV) AddNode(node string) {
	kv.ring.AddNode(node)
	kv.stores[node] = make(map[string]string)
}

func (kv *ShardedKV) RemoveNode(node string) {
	kv.ring.RemoveNode(node)
	delete(kv.stores, node)
}

func (kv *ShardedKV) Put(key, value string) {
	node := kv.ring.GetNode(key)
	kv.stores[node][key] = value
}

func (kv *ShardedKV) Get(key string) (string, bool) {
	node := kv.ring.GetNode(key)
	v, ok := kv.stores[node][key]
	return v, ok
}

func (kv *ShardedKV) Dump() {
	for _, node := range kv.ring.Nodes() {
		fmt.Printf("  %s (%d keys):\n", node, len(kv.stores[node]))
		for k, v := range kv.stores[node] {
			fmt.Printf("    %s → %s\n", k, v)
		}
	}
}
```

### Step 3: Simplified Raft (Leader Election + Log Replication)

This is a pedagogical version — it captures the core protocol without the full production complexity.

```go
import (
	"math/rand"
	"sync"
	"time"
)

type LogEntry struct {
	Term    int
	Command string
}

type RaftNode struct {
	mu       sync.Mutex
	id       string
	peers    []string
	state    string // follower, candidate, leader
	term     int
	votedFor string
	log      []LogEntry
	commit   int
	// leader-only state
	nextIndex  map[string]int
	matchIndex map[string]int
	// channels
	heartbeat chan struct{}
	stop      chan struct{}
}

func NewRaftNode(id string, peers []string) *RaftNode {
	return &RaftNode{
		id:        id,
		peers:     peers,
		state:     "follower",
		term:      0,
		votedFor:  "",
		log:       []LogEntry{},
		commit:    -1,
		nextIndex: make(map[string]int),
		matchIndex: make(map[string]int),
		heartbeat: make(chan struct{}, 100),
		stop:      make(chan struct{}),
	}
}

func (n *RaftNode) Run() {
	for {
		select {
		case <-n.stop:
			return
		default:
		}
		n.mu.Lock()
		state := n.state
		n.mu.Unlock()

		switch state {
		case "follower":
			n.runFollower()
		case "candidate":
			n.runCandidate()
		case "leader":
			n.runLeader()
		}
	}
}

func (n *RaftNode) runFollower() {
	timer := time.NewTimer(randomElectionTimeout())
	select {
	case <-n.heartbeat:
		timer.Stop()
	case <-timer.C:
		n.mu.Lock()
		n.state = "candidate"
		n.mu.Unlock()
	case <-n.stop:
		timer.Stop()
	}
}

func (n *RaftNode) runCandidate() {
	n.mu.Lock()
	n.term++
	n.votedFor = n.id
	n.mu.Unlock()

	votes := 1
	total := len(n.peers) + 1
	majority := total/2 + 1

	timer := time.NewTimer(randomElectionTimeout())
	for _, peer := range n.peers {
		go func(p string) {
			if n.requestVote(p) {
				n.mu.Lock()
				votes++
				if votes >= majority && n.state == "candidate" {
					n.state = "leader"
					n.nextIndex = make(map[string]int)
					for _, peer := range n.peers {
						n.nextIndex[peer] = len(n.log)
					}
				}
				n.mu.Unlock()
			}
		}(peer)
	}

	<-timer.C
	// If we didn't become leader, go back to follower
	n.mu.Lock()
	if n.state == "candidate" {
		n.state = "follower"
	}
	n.mu.Unlock()
}

func (n *RaftNode) runLeader() {
	// Send heartbeats / append entries to all followers
	for _, peer := range n.peers {
		go func(p string) {
			n.appendEntries(p)
		}(peer)
	}

	ticker := time.NewTicker(50 * time.Millisecond)
	defer ticker.Stop()

	select {
	case <-ticker.C:
		// Send periodic heartbeats
	case <-n.stop:
		return
	}
	// Check for leader step-down (higher term discovered)
}

// Simplified — in production these would be RPCs
func (n *RaftNode) requestVote(peer string) bool {
	return true // simplified
}

func (n *RaftNode) appendEntries(peer string) {
	// Simplified — in production this is an RPC
}

func (n *RaftNode) Propose(command string) {
	n.mu.Lock()
	defer n.mu.Unlock()
	n.log = append(n.log, LogEntry{Term: n.term, Command: command})
	// Leader commits immediately in simplified version
	n.commit = len(n.log) - 1
}

func (n *RaftNode) CommitIndex() int {
	n.mu.Lock()
	defer n.mu.Unlock()
	return n.commit
}

func (n *RaftNode) LastApplied() []LogEntry {
	n.mu.Lock()
	defer n.mu.Unlock()
	if n.commit < 0 {
		return nil
	}
	return n.log[:n.commit+1]
}

func randomElectionTimeout() time.Duration {
	return time.Duration(150+rand.Intn(150)) * time.Millisecond
}
```

### Step 4: Complete Program (main.go)

```go
package main

import (
	"fmt"
	"math/rand"
	"time"
)

func main() {
	rand.Seed(time.Now().UnixNano())

	fmt.Println("=== Consistent Hash Ring ===")
	ring := NewRing(3)
	ring.AddNode("node-a")
	ring.AddNode("node-b")
	ring.AddNode("node-c")

	keys := []string{"alice", "bob", "charlie", "dave", "eve", "frank", "grace", "heidi"}
	for _, k := range keys {
		fmt.Printf("  %s → %s\n", k, ring.GetNode(k))
	}

	fmt.Println("\n=== Sharded KV Store ===")
	kv := NewShardedKV(3)
	kv.AddNode("server-1")
	kv.AddNode("server-2")
	kv.AddNode("server-3")

	for i := 0; i < 20; i++ {
		key := fmt.Sprintf("user:%d", i)
		val := fmt.Sprintf("val-%d", i)
		kv.Put(key, val)
	}
	kv.Dump()

	fmt.Println("\n  Reading back:")
	for _, key := range []string{"user:0", "user:5", "user:10", "user:15"} {
		if v, ok := kv.Get(key); ok {
			fmt.Printf("    %s → %s\n", key, v)
		}
	}

	fmt.Println("\n=== Adding server-4 (rebalancing) ===")
	kv.AddNode("server-4")
	fmt.Printf("  Key 'user:0' now on: %s (was server-2 before rebalance)\n", kv.ring.GetNode("user:0"))

	fmt.Println("\n=== Simplified Raft ===")
	nodes := []string{"n1", "n2", "n3"}
	for _, id := range nodes {
		peers := []string{}
		for _, p := range nodes {
			if p != id {
				peers = append(peers, p)
			}
		}
		n := NewRaftNode(id, peers)
		go n.Run()

		n.mu.Lock()
		n.state = "leader" // force leader for demo
		n.mu.Unlock()

		n.Propose(fmt.Sprintf("SET x=%d", i))
	}

	// Show last committed
	fmt.Println("\n  Raft log for n1:")
	log := NewRaftNode("n1", []string{"n2", "n3"}).LastApplied()
	for _, entry := range log {
		fmt.Printf("    [term %d] %s\n", entry.Term, entry.Command)
	}
	// Note: actual output would show committed entries from the real leader
}
```

## Use It

**Google Spanner** is the production reference. The key differences between our toy and Spanner:

- Our ring doesn't do range-based sharding (Spanner uses key-range splits called *splits*).
- Our raft is single-node only; Spanner runs Paxos per shard with full leader election, log compaction (snapshotting), membership changes, and pipeline replication.
- Spanner's TrueTime provides bounded clock uncertainty; our vnode hash doesn't deal with time at all.
- Spanner's 2PC handles cross-shard transactions atomically; our KV store routes each key independently.
- Spanner serves F1 SQL queries on top; ours is just a key-value store.

**etcd** uses Raft for its consensus layer. Reading etcd's raft package (`go.etcd.io/raft/v3`) shows how production Raft handles leader election with pre-vote, check-quorum, and dynamic membership — all omitted from our simplified version.

**CockroachDB** uses Raft per range and HLC clocks. Their `pkg/kv/kvserver/` directory shows how a shard (range) is a Raft group. CockroachDB's HLC implementation is in `pkg/util/hlc/`.

## Read the Source

- [Google Spanner paper](https://research.google/pubs/pub39966/) — The original 2012 paper explaining TrueTime, 2PC + Paxos, and external consistency. Start with §2 (TrueTime) and §4.2 (read-write transactions).
- [etcd raft](https://github.com/etcd-io/raft) — `raft.go` (the core protocol loop), `log.go` (the log implementation). Look at `Step()` to see how messages drive state transitions.
- [CockroachDB range](https://github.com/cockroachdb/cockroach/tree/master/pkg/kv/kvserver) — `replica.go` implements the Raft-backed range. `Store` manages all ranges on a node.
- [Cassandra partitioner](https://github.com/apache/cassandra/blob/trunk/src/java/org/apache/cassandra/dht/Murmur3Partitioner.java) — Murmur3 consistent hasher in production use.

## Ship It

The reusable artifact is in `outputs/` — a self-contained Go package with `ConsistentHashRing` and `ShardedKV` that you can import into future projects.

## Exercises

1. **Easy** — Add a `GetNodeReplicas(key string, count int) []string` method that returns the next N distinct nodes for the key (for N-way replication).
2. **Medium** — Implement key migration: when a node is removed, copy its keys to the node that takes over its hash range.
3. **Hard** — Implement a real Raft leader election with RPC simulation (channels) and log replication (Note: This is already done above. Expand it to handle network partitions and term checks.).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Sharding | "Splitting data across machines" | Horizontal partitioning where each node owns a subset of the keyspace; combined with replication for fault tolerance |
| Consistent hashing | "A ring of hashes" | A hash function maps both keys and nodes to a circular space; each key is assigned to the nearest node clockwise |
| Raft | "A consensus algorithm" | A protocol where a leader replicates a log across a majority of nodes; all nodes agree on the log order and which entries are committed |
| TrueTime | "Google's atomic clock service" | A time API returning `[earliest, latest]` intervals with bounded uncertainty (~7 ms); enables external consistency without centralized clock authority |
| 2PC | "Two-phase commit" | Prepare all participants; if any fails, abort all; coordinator crash after prepare blocks participants indefinitely |
| HLC | "Hybrid logical clock" | Combines physical wall clock + logical counter to provide causally-consistent timestamps without GPS/atomic clocks |

## Further Reading

- [Spanner: Google's Globally-Distributed Database (2012)](https://research.google/pubs/pub39966/) — The original paper. Still the clearest explanation of external consistency.
- [Spanner, TrueTime & the CAP Theorem (2017)](https://static.googleusercontent.com/media/research.google.com/en//pubs/archive/45855.pdf) — How Spanner provides CP with availability despite partitions.
- [CockroachDB Design](https://www.cockroachlabs.com/docs/stable/architecture/overview.html) — Spanner-inspired design with HLC instead of TrueTime.
- [Dynamo: Amazon's Highly Available Key-value Store](https://www.allthingsdistributed.com/files/amazon-dynamo-sosp2007.pdf) — The original leaderless replication paper (read repair, hinted handoff, vector clocks).
- [Raft: In Search of an Understandable Consensus Algorithm](https://raft.github.io/raft.pdf) — The Raft paper. Read sections 5.1–5.4 for leader election and log replication.
- [The Part-Time Parliament (Paxos)](https://lamport.azurewebsites.net/pubs/lamport-paxos.pdf) — Lamport's original Paxos paper (wrapped in a Greek allegory).
