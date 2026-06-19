# Build a Distributed KV Store (Raft + MVCC)

> Consensus plus versioned storage yields predictable distributed state evolution.

**Type:** Build
**Languages:** Go, Rust
**Prerequisites:** Phase 19 lessons 01-02
**Time:** ~840 minutes

## Learning Objectives

- Decompose a distributed KV architecture into replication and storage layers.
- Model Raft log application flow into MVCC state.
- Build a thin vertical slice for put/get with version metadata.
- Define reliability and consistency validation gates.

## The Problem

Distributed storage capstones fail when consensus and storage semantics are intermingled too early. Someone starts building a replicated key-value store, puts the Raft election logic inside the put handler, discovers that a leader change corrupts the version counter, and can't tell whether the bug is in consensus or storage.

The root cause: consensus and storage are different problems with different invariants. Raft's invariant is "all nodes apply the same commands in the same order." MVCC's invariant is "each key retains a chain of versions, and reads at a given version see a consistent snapshot." When you blur these boundaries, a bug in version numbering looks like a consensus violation, and a bug in log application looks like a storage corruption.

Separating concerns enables incremental correctness. You can verify that Raft replicates commands correctly without caring about MVCC. You can verify that MVCC stores and retrieves versions correctly without caring about replication. The interface between them is the apply function: given a committed log entry, apply it to the MVCC store.

## The Concept

A distributed KV store has three layers, each with its own responsibilities:

```
Client request (PUT key=value / GET key)
        │
        ▼
┌───────────────────┐
│  Raft layer        │  Replicate command to majority
│  (consensus)       │  Ensure all nodes apply same order
└───────────────────┘
        │ committed entry
        ▼
┌───────────────────┐
│  Apply layer       │  Deterministic state machine transition
│  (state machine)   │  Assign version, execute operation
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  MVCC storage      │  Version chain per key
│  (storage engine)  │  Snapshot reads at any version
└───────────────────┘
```

**Raft layer**: accepts client commands, appends them to the leader's log, replicates to followers via AppendEntries, and commits when a majority acknowledges. Committed entries are delivered to the apply layer in log order.

**Apply layer**: receives committed entries and applies them to the MVCC store. Each apply increments a global version counter. PUT operations store the new value at the current version. GET operations read the latest value at or before the requested version.

**MVCC storage**: maintains a mapping from keys to version chains. Each version has a value and a creation timestamp (the global version number). Reads at a given version see a consistent snapshot: all writes with version <= the read version.

This decomposition mirrors production systems. etcd uses the same three-layer architecture: Raft for consensus, a key-index structure for MVCC, and BoltDB for storage. TiKV separates Raft replication from the RocksDB-backed MVCC engine.

## Build It

We implement a local single-node simulation that preserves the interfaces needed for multi-node extension. The Raft layer is simulated by a command log; the MVCC store is real.

### Step 1: MVCC Storage (Go)

```go
package main

import (
    "fmt"
    "sort"
    "strings"
    "sync"
)

// Version represents one version of a key's value
type Version struct {
    Value     string
    CreatedAt uint64 // Global version when this was written
    Deleted   bool   // Tombstone for deletes
}

// MVCCStore is a multi-version key-value store
type MVCCStore struct {
    mu       sync.RWMutex
    versions map[string][]Version // key -> sorted version chain
    globalVersion uint64
}

func NewMVCCStore() *MVCCStore {
    return &MVCCStore{
        versions:      make(map[string][]Version),
        globalVersion: 0,
    }
}

// ApplyPut stores a new version for the key at the current global version
func (s *MVCCStore) ApplyPut(key, value string) uint64 {
    s.mu.Lock()
    defer s.mu.Unlock()

    s.globalVersion++
    v := Version{
        Value:     value,
        CreatedAt: s.globalVersion,
    }
    s.versions[key] = append(s.versions[key], v)
    return s.globalVersion
}

// ApplyDelete stores a tombstone at the current global version
func (s *MVCCStore) ApplyDelete(key string) uint64 {
    s.mu.Lock()
    defer s.mu.Unlock()

    s.globalVersion++
    v := Version{
        Deleted:   true,
        CreatedAt: s.globalVersion,
    }
    s.versions[key] = append(s.versions[key], v)
    return s.globalVersion
}

// GetAt reads the value of a key at the given snapshot version.
// Returns the latest version with CreatedAt <= readVersion.
func (s *MVCCStore) GetAt(key string, readVersion uint64) (string, bool) {
    s.mu.RLock()
    defer s.mu.RUnlock()

    versions := s.versions[key]
    // Binary search for the latest version <= readVersion
    idx := sort.Search(len(versions), func(i int) bool {
        return versions[i].CreatedAt > readVersion
    })
    if idx == 0 {
        return "", false // No version exists at or before readVersion
    }
    v := versions[idx-1]
    if v.Deleted {
        return "", false
    }
    return v.Value, true
}

// GetLatest reads the most recent version of a key
func (s *MVCCStore) GetLatest(key string) (string, bool) {
    s.mu.RLock()
    defer s.mu.RUnlock()

    versions := s.versions[key]
    if len(versions) == 0 {
        return "", false
    }
    v := versions[len(versions)-1]
    if v.Deleted {
        return "", false
    }
    return v.Value, true
}

// CurrentVersion returns the latest committed version number
func (s *MVCCStore) CurrentVersion() uint64 {
    s.mu.RLock()
    defer s.mu.RUnlock()
    return s.globalVersion
}
```

### Step 2: Command Log (Raft simulation)

```go
// CommandType represents the type of KV operation
type CommandType int

const (
    CmdPut CommandType = iota
    CmdDelete
    CmdGet
)

// Command is a replicated log entry
type Command struct {
    Type  CommandType
    Key   string
    Value string // Only used for PUT
}

// ReplicatedLog simulates Raft's replicated log
// In a real system, this would be replicated across nodes via AppendEntries
type ReplicatedLog struct {
    mu       sync.Mutex
    entries  []Command
    commitIdx int
}

func NewReplicatedLog() *ReplicatedLog {
    return &ReplicatedLog{}
}

// Append adds a command to the log (simulates leader append)
func (l *ReplicatedLog) Append(cmd Command) int {
    l.mu.Lock()
    defer l.mu.Unlock()
    l.entries = append(l.entries, cmd)
    return len(l.entries) - 1
}

// Commit marks all entries up to idx as committed (simulates majority ack)
func (l *ReplicatedLog) Commit(upToIdx int) {
    l.mu.Lock()
    defer l.mu.Unlock()
    if upToIdx > l.commitIdx {
        l.commitIdx = upToIdx
    }
}

// ApplyCommitted applies all newly committed entries to the MVCC store
func (l *ReplicatedLog) ApplyCommitted(store *MVCCStore) {
    l.mu.Lock()
    entries := make([]Command, len(l.entries))
    copy(entries, l.entries)
    commitIdx := l.commitIdx
    l.mu.Unlock()

    for i := 0; i <= commitIdx; i++ {
        cmd := entries[i]
        switch cmd.Type {
        case CmdPut:
            store.ApplyPut(cmd.Key, cmd.Value)
        case CmdDelete:
            store.ApplyDelete(cmd.Key)
        }
    }
}
```

### Step 3: Putting It Together

```go
func main() {
    store := NewMVCCStore()
    log := NewReplicatedLog()

    // Simulate: client sends PUT commands through Raft
    commands := []Command{
        {CmdPut, "name", "Alice"},
        {CmdPut, "age", "30"},
        {CmdPut, "name", "Bob"},  // Overwrite
        {CmdDelete, "age", ""},
        {CmdPut, "city", "Portland"},
    }

    // Phase 1: Append all commands to the log
    for _, cmd := range commands {
        idx := log.Append(cmd)
        log.Commit(idx) // In real Raft, this happens after majority ack
    }

    // Phase 2: Apply committed entries to the MVCC store
    log.ApplyCommitted(store)

    // Demonstrate MVCC reads
    fmt.Println("=== Latest values ===")
    for _, key := range []string{"name", "age", "city"} {
        if val, ok := store.GetLatest(key); ok {
            fmt.Printf("  %s = %s\n", key, val)
        } else {
            fmt.Printf("  %s = <not found>\n", key)
        }
    }

    // Demonstrate snapshot reads at historical versions
    fmt.Println("\n=== Snapshot at version 2 ===")
    snapshotVersion := uint64(2)
    for _, key := range []string{"name", "age", "city"} {
        if val, ok := store.GetAt(key, snapshotVersion); ok {
            fmt.Printf("  %s = %s\n", key, val)
        } else {
            fmt.Printf("  %s = <not found>\n", key)
        }
    }

    fmt.Printf("\nCurrent global version: %d\n", store.CurrentVersion())
}
```

Expected output:

```
=== Latest values ===
  name = Bob
  age = <not found>
  city = Portland

=== Snapshot at version 2 ===
  name = Alice
  age = 30
  city = <not found>

Current global version: 5
```

### Step 4: Rust Equivalence Sketch

```rust
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug)]
struct Version {
    value: Option<String>, // None = tombstone
    created_at: u64,
}

struct MVCCStore {
    versions: RwLock<BTreeMap<String, Vec<Version>>>,
    global_version: RwLock<u64>,
}

impl MVCCStore {
    fn new() -> Self {
        MVCCStore {
            versions: RwLock::new(BTreeMap::new()),
            global_version: RwLock::new(0),
        }
    }

    fn apply_put(&self, key: &str, value: &str) -> u64 {
        let mut gv = self.global_version.write().unwrap();
        *gv += 1;
        let v = Version {
            value: Some(value.to_string()),
            created_at: *gv,
        };
        let mut versions = self.versions.write().unwrap();
        versions.entry(key.to_string()).or_default().push(v);
        *gv
    }

    fn apply_delete(&self, key: &str) -> u64 {
        let mut gv = self.global_version.write().unwrap();
        *gv += 1;
        let v = Version {
            value: None,
            created_at: *gv,
        };
        let mut versions = self.versions.write().unwrap();
        versions.entry(key.to_string()).or_default().push(v);
        *gv
    }

    fn get_at(&self, key: &str, read_version: u64) -> Option<String> {
        let versions = self.versions.read().unwrap();
        let chain = versions.get(key)?;
        // Binary search for latest version <= read_version
        let idx = chain.partition_point(|v| v.created_at <= read_version);
        if idx == 0 { return None; }
        chain[idx - 1].value.clone()
    }
}

fn main() {
    let store = MVCCStore::new();
    store.apply_put("name", "Alice");
    store.apply_put("age", "30");
    store.apply_put("name", "Bob");
    store.apply_delete("age");

    println!("Latest name: {:?}", store.get_at("name", u64::MAX));
    println!("Name at v1: {:?}", store.get_at("name", 1));
    println!("Age at v2: {:?}", store.get_at("age", 2));
}
```

## Use It

This decomposition mirrors practical systems design in production distributed databases:

- **etcd**: the Kubernetes backing store uses Raft for consensus and an MVCC key-index backed by BoltDB. Every key revision is a version; `etcdctl get --rev=N` reads at a specific version. The apply function is in `server/etcdserver/server.go`.
- **TiKV**: a distributed transactional KV store that separates Raft replication (via the `raftstore` crate) from the MVCC-aware storage engine (backed by RocksDB). The `Storage` struct handles MVCC logic; Raft handles replication.
- **CockroachDB**: uses Raft for replication and MVCC for transactional isolation. Every key-value pair is stored with a timestamp (hybrid logical clock). Reads at a given timestamp see a consistent snapshot.

The key production lesson: **the apply function is the contract between consensus and storage**. In etcd, this is `apply()` in the Raft state machine. In TiKV, it's the `apply` callback in the Raft worker. If this function is deterministic (same input produces same state), then all nodes converge to the same state. If it's not, consensus doesn't help you.

## Read the Source

- [Raft paper](https://raft.github.io/raft.pdf) — Ongaro and Ousterhout, 2014. The consensus protocol that orders commands. Read sections 5-7 for the core algorithm.
- [etcd's MVCC implementation](https://github.com/etcd-io/etcd/blob/main/server/storage/mvcc/) — The `kv.go` and `store.go` files show how etcd implements version chains with a B-tree index.
- [TiKV's storage engine](https://github.com/tikv/tikv/tree/master/src/storage/) — The `txn` module implements MVCC on top of RocksDB. The `mvcc` module shows version management with min-commit-ts and read timestamps.
- [MVCC overview on Wikipedia](https://en.wikipedia.org/wiki/Multiversion_concurrency_control) — Background on multi-version concurrency control theory.

## Ship It

- `code/main.go`: command log + MVCC simulation in Go, demonstrating put/get/delete with snapshot reads.
- `code/main.rs`: equivalent Rust sketch with thread-safe MVCC store.
- `outputs/README.md`: distributed KV milestone checklist covering consensus integration, MVCC correctness, and snapshot reads.

## Exercises

1. **Easy** — Add range reads by version snapshot. Implement `ScanRange(startKey, endKey, readVersion)` that returns all key-value pairs in the given key range at the given snapshot version. Use the BTreeMap's range iterator.
2. **Medium** — Add conflict checks with compare-and-set semantics. Implement `CAS(key, expectedValue, newValue)` that only writes if the current latest value matches `expectedValue`. Return success/failure. Think about what happens when two nodes try CAS on the same key simultaneously.
3. **Hard** — Simulate follower catch-up from log prefix. When a new node joins the cluster, it needs to replay the committed log from the beginning. Implement `CatchUp(newNode, log, store)` that transfers all committed entries and rebuilds the MVCC state on the new node.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Consensus log | "ordered commands" | A replicated sequence of commands agreed upon by a majority of nodes. Each entry has a term and index. All nodes apply entries in the same order. |
| State machine apply | "commit step" | The deterministic function that takes a committed log entry and updates the storage state. If all nodes apply the same entries in the same order, they converge to the same state. |
| MVCC | "multi-version storage" | A storage technique where each write creates a new version rather than overwriting. Reads at a given timestamp see a consistent snapshot. Enables lock-free reads and temporal queries. |
| Linearizability | "real-time consistency" | The strongest single-object consistency model. Each operation appears to take effect atomically at some point between its invocation and response. Requires consensus to implement. |
| Tombstone | "delete marker" | A special version entry indicating a key was deleted. Without tombstones, you can't distinguish "key was never written" from "key was deleted." Tombstones are compacted during garbage collection. |

## Further Reading

- [Raft paper](https://raft.github.io/raft.pdf) — The consensus protocol that orders commands in our distributed KV store.
- [MVCC overview](https://en.wikipedia.org/wiki/Multiversion_concurrency_control) — Background on multi-version concurrency control.
- [Designing Data-Intensive Applications](https://dataintensive.net/) — Kleppmann. Chapter 7 (Transactions) and Chapter 9 (Consistency) cover MVCC and consensus in depth.
- [etcd architecture](https://etcd.io/docs/v3.5/learning/architecture/) — How etcd puts Raft, MVCC, and BoltDB together.
