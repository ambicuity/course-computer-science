# Distributed Caches — Memcached, Redis, consistent hashing

> A cache that doesn't understand distribution is just a cache — a distributed cache must survive nodes joining, leaving, and the thundering herd.

**Type:** Learn
**Languages:** Go
**Prerequisites:** Phase 11 lessons 01–17 (especially Lesson 10 — Replication and Lesson 13 — Gossip Protocols)
**Time:** ~60 minutes

## Learning Objectives

- Explain why caching matters: latency reduction, database load reduction, bandwidth savings — and why distribution adds complexity.
- Compare cache strategies: cache-aside (lazy loading), write-through, write-behind, and refresh-ahead — and know when each fails.
- Describe cache invalidation approaches: time-based (TTL), event-based (invalidate on write), and version-based.
- Contrast Memcached and Redis: threading models, data structures, persistence, replication, and clustering.
- Implement consistent hashing with virtual nodes (Karger et al. 1997) and explain why virtual nodes improve balance.
- Explain Rendezvous hashing (HRW) as an alternative to consistent hashing and compare their balance properties.
- Define cache stampede (thundering herd) and implement mitigations: lock-per-key, probabilistic early expiration, and request coalescing.
- Build a consistent-hash distributed cache in Go that distributes 1000 keys across 5 nodes, survives node additions with minimal remapping, and prevents stampedes.

## The Problem

Your database serves 10,000 reads per second. Ninety percent of those reads return the same popular items — product pages, user profiles, configuration flags. Each database query takes 5ms. That's 50 seconds of database time per second, which means you need 50 parallel connections just to keep up, and your p99 latency is climbing.

You add a cache. A single in-memory cache cuts latency from 5ms to 0.1ms for hot keys. Problem solved — until the cache process dies and 9,000 queries/sec slam the database simultaneously. Or the cache hits its memory limit and starts evicting. Or your service scales to 50 machines and each one has its own cache, so a key cached on machine A is a cold miss on machine B.

A **distributed cache** solves the last problem: all nodes in your cluster route requests for the same key to the same cache node, so every node benefits from every cached entry. But distributing a cache introduces a new problem — **which node owns which key?** A naive modulo assignment (`key % N`) works until you add or remove a node, at which point nearly every key remaps and your cache hit rate drops to zero.

This lesson builds the solution: consistent hashing to distribute keys with minimal remapping, cache strategies to keep data fresh, and stampede protection to survive the moments when caches fail.

## The Concept

### Why Cache?

Three forces push you toward caching:

| Force | Without cache | With cache |
|-------|--------------|------------|
| **Latency** | 5ms disk/DB read | 0.1ms memory read |
| **Load** | Every request hits DB | Only misses hit DB |
| **Bandwidth** | Large responses traversing network | Responses served locally |

Caching trades memory for speed. The question isn't whether to cache — it's how to manage the cache.

### Cache Strategies

```
Cache-Aside (Lazy Loading)
───────────────────────────
  Client → Check cache → Miss? → Read DB → Write cache → Return
                 ↓ Hit
               Return

Write-Through
─────────────
  Client → Write cache → Write DB → Return
  Client → Read cache → (always fresh if cache exists)

Write-Behind (Write-Back)
─────────────────────────
  Client → Write cache → Return (DB write deferred)
  Background: periodically flush cache → DB

Refresh-Ahead
─────────────
  Background: before TTL expires, refresh cache from DB
  Client → Read cache → (hot keys pre-warmed)
```

**Cache-aside** is the simplest and most common. The cache is never the source of truth — the database is. On a miss, you read from the DB and populate the cache. Stale data is only possible if data changes in the DB without the cache knowing.

**Write-through** keeps cache and DB consistent on every write, but every write now costs two operations (cache + DB). Reads are fast — cache always has the latest.

**Write-behind** writes to the cache and returns immediately; the DB is updated asynchronously. Faster writes, but you risk data loss if a cache node crashes before flushing. Used when write throughput matters more than durability.

**Refresh-ahead** pre-warms popular keys before they expire, so clients never see a miss for hot data. Requires predicting which keys will be accessed, and can waste resources refreshing keys nobody reads.

### Cache Invalidation

There are only two hard problems in computer science: cache invalidation and naming things. — Phil Karlton

| Approach | Mechanism | Trade-off |
|----------|-----------|-----------|
| **TTL** | Expire entries after a fixed time | Simple, but stale for the entire TTL window |
| **Event-based** | Invalidate on write/update | Always fresh, but requires write path awareness |
| **Version-based** | Key includes version (`user:42:v3`) | No invalidation needed, but old versions accumulate |

TTL is the baseline — every cache system supports it. Event-based invalidation is more correct but requires the cache to be wired into the write path. Version-based caching avoids invalidation entirely by making each version a new key, at the cost of storage growth.

### Memcached vs. Redis

| Property | Memcached | Redis |
|----------|-----------|-------|
| **Thread model** | Multi-threaded (no locks per thread) | Single-threaded (event loop) |
| **Data structures** | Strings only (key → blob) | Strings, lists, sets, sorted sets, hashes, streams |
| **Eviction** | LRU | LRU, LFU, volatile-ttl, noeviction |
| **Persistence** | None (purely volatile) | RDB snapshots + AOF append-only log |
| **Replication** | None | Primary-replica async replication |
| **Clustering** | Client-side consistent hashing | Hash slots (16,384 slots across nodes) |
| **Memory allocator** | Slab allocator (pre-sized chunks) | jemalloc |

Memcached is a **dumb, fast bucket**. It does one thing well: store key-value pairs in memory and evict the least recently used when full. Multi-threading means it can serve requests across CPU cores without lock contention. No persistence — restart means cold cache.

Redis is a **data structure server**. Single-threaded event loop sounds like a disadvantage, but it eliminates lock overhead and makes operations atomic. Rich data structures mean you can do atomic increments, sorted-set rankings, pub/sub, and streams — things that would require multiple round-trips in Memcached. Redis Cluster distributes 16,384 hash slots across nodes; each key maps to a slot via `CRC16(key) % 16384`.

### Consistent Hashing

The core problem: map keys to cache nodes so that adding or removing a node remaps the minimum number of keys.

**Naive approach:** `node = hash(key) % N`. When N changes (add/remove a node), nearly every key remaps. With 5 nodes, adding a 6th remaps 83% of keys.

**Consistent hashing** (Karger et al., 1997): arrange hash values on a ring from 0 to 2^32-1. Each node occupies positions on the ring determined by `hash(node_id)`. A key maps to the first node clockwise from `hash(key)`.

```
    0
    │
  Node A ─── hash("A") = 2,847,310
    │
  Node B ─── hash("B") = 9,562,104
    │
  Node C ─── hash("C") = 3,421,009,877
    │
    2^32 - 1

  Key K with hash(K) = 5,000,000 → belongs to Node B
  (first node clockwise from 5,000,000)
```

When you add Node D, only the keys between Node D's position and its predecessor remap to D. All other keys stay where they are. With K nodes, adding one remaps roughly 1/K of the keys — not 100%.

**The balance problem:** With few nodes, the ring distribution is uneven. Node A might own 50% of the ring while Node B owns 10%.

**Virtual nodes (vnodes):** Each physical node maps to many positions on the ring (typically 100–200). Node A becomes `hash("A#0")`, `hash("A#1")`, ..., `hash("A#149")`. The law of large numbers smooths the distribution. With 150 vnodes per physical node and 5 nodes, each physical node owns roughly 20% ± 2% of the ring.

### Rendezvous Hashing (HRW)

An alternative to consistent hashing. Given a key K and a set of nodes, compute `hash(K, node_i)` for every node and pick the node with the highest hash value. To find which node owns K, you compute one hash per node — no ring needed.

Properties:
- Adding a node: only keys where the new node wins the rendezvous remap.
- Removing a node: keys redistribute to their second-highest node.
- Better balance than bare consistent hashing (no vnodes needed), but O(N) per lookup vs O(log N) for the ring with vnodes.

In practice, consistent hashing with vnodes dominates because the O(log N) ring lookup is faster for large clusters, and vnodes solve the balance problem adequately.

### Cache Stampede (Thundering Herd)

When a popular key expires (or a cache node fails), every concurrent request sees a miss and tries to fetch from the database simultaneously.

```
  Cache miss for key "popular-item"
  ┌─────────┐ ┌─────────┐ ┌─────────┐     ┌─────────┐
  │ Req 1   │ │ Req 2   │ │ Req 3   │ ... │ Req N   │
  └────┬────┘ └────┬────┘ └────┬────┘     └────┬────┘
       │            │            │               │
       ▼            ▼            ▼               ▼
  ┌─────────────────────────────────────────────────┐
  │              DATABASE (N simultaneous queries)   │
  └─────────────────────────────────────────────────┘
```

Three mitigations:

1. **Lock per key:** The first request to see a miss acquires a lock for that key. Other requests wait for the lock. One DB query instead of N.

2. **Probabilistic early expiration:** Before the TTL expires, requests randomly decide to refresh early based on `curr_time > expire_time - random(0, TTL * beta)`. This spreads refreshes across time instead of clustering them at the exact TTL boundary.

3. **Request coalescing (singleflight):** When multiple requests ask for the same key, they share the result of the first request. Go's `golang.org/x/sync/singleflight` implements this pattern.

## Build It

### Step 1: Consistent Hash Ring with Virtual Nodes

The ring maps keys to nodes. Each physical node gets 150 virtual nodes. Keys are assigned to the first vnode clockwise from their hash position.

```go
package main

import (
    "crypto/sha256"
    "fmt"
    "math"
    "sort"
    "sync"
    "time"
)

func sha256Hash(data string) uint32 {
    h := sha256.Sum256([]byte(data))
    return uint32(h[0])<<24 | uint32(h[1])<<16 | uint32(h[2])<<8 | uint32(h[3])
}

type VirtualNode struct {
    hash     uint32
    nodeID   string
}

type ConsistentHashRing struct {
    vnodes   []VirtualNode
    numVNodes int
    mu        sync.RWMutex
}

func NewConsistentHashRing(numVNodes int) *ConsistentHashRing {
    return &ConsistentHashRing{numVNodes: numVNodes}
}

func (r *ConsistentHashRing) AddNode(nodeID string) {
    r.mu.Lock()
    defer r.mu.Unlock()
    for i := 0; i < r.numVNodes; i++ {
        vnodeKey := fmt.Sprintf("%s#%d", nodeID, i)
        r.vnodes = append(r.vnodes, VirtualNode{
            hash:   sha256Hash(vnodeKey),
            nodeID: nodeID,
        })
    }
    sort.Slice(r.vnodes, func(i, j int) bool {
        return r.vnodes[i].hash < r.vnodes[j].hash
    })
}

func (r *ConsistentHashRing) RemoveNode(nodeID string) {
    r.mu.Lock()
    defer r.mu.Unlock()
    filtered := r.vnodes[:0]
    for _, vn := range r.vnodes {
        if vn.nodeID != nodeID {
            filtered = append(filtered, vn)
        }
    }
    r.vnodes = filtered
}

func (r *ConsistentHashRing) GetNode(key string) string {
    r.mu.RLock()
    defer r.mu.RUnlock()
    if len(r.vnodes) == 0 {
        return ""
    }
    h := sha256Hash(key)
    idx := sort.Search(len(r.vnodes), func(i int) bool {
        return r.vnodes[i].hash >= h
    })
    if idx == len(r.vnodes) {
        idx = 0
    }
    return r.vnodes[idx].nodeID
}

func (r *ConsistentHashRing) NodeCount() int {
    r.mu.RLock()
    defer r.mu.RUnlock()
    seen := make(map[string]bool)
    for _, vn := range r.vnodes {
        seen[vn.nodeID] = true
    }
    return len(seen)
}
```

### Step 2: Distributed Cache with Cache Strategies

Each physical node has its own local cache. The distributed cache routes requests to the correct node using the consistent hash ring. We implement cache-aside, write-through, and TTL-based eviction.

```go
type CacheEntry struct {
    value     string
    expiresAt time.Time
}

type LocalCache struct {
    items map[string]CacheEntry
    mu    sync.RWMutex
}

func NewLocalCache() *LocalCache {
    return &LocalCache{items: make(map[string]CacheEntry)}
}

func (lc *LocalCache) Set(key, value string, ttl time.Duration) {
    lc.mu.Lock()
    defer lc.mu.Unlock()
    lc.items[key] = CacheEntry{
        value:     value,
        expiresAt: time.Now().Add(ttl),
    }
}

func (lc *LocalCache) Get(key string) (string, bool) {
    lc.mu.RLock()
    defer lc.mu.RUnlock()
    entry, ok := lc.items[key]
    if !ok {
        return "", false
    }
    if time.Now().After(entry.expiresAt) {
        return "", false
    }
    return entry.value, true
}

func (lc *LocalCache) Delete(key string) {
    lc.mu.Lock()
    defer lc.mu.Unlock()
    delete(lc.items, key)
}

func (lc *LocalCache) Len() int {
    lc.mu.RLock()
    defer lc.mu.RUnlock()
    count := 0
    for _, entry := range lc.items {
        if time.Now().Before(entry.expiresAt) {
            count++
        }
    }
    return count
}

type Database struct {
    data map[string]string
    mu   sync.RWMutex
}

func NewDatabase() *Database {
    return &Database{data: make(map[string]string)}
}

func (db *Database) Get(key string) (string, bool) {
    db.mu.RLock()
    defer db.mu.RUnlock()
    v, ok := db.data[key]
    return v, ok
}

func (db *Database) Set(key, value string) {
    db.mu.Lock()
    defer db.mu.Unlock()
    db.data[key] = value
}

func (db *Database) ReadCount() int { return 0 }

type DistributedCache struct {
    ring   *ConsistentHashRing
    nodes  map[string]*LocalCache
    db     *Database
    dbReads int64
    mu     sync.Mutex
}

func NewDistributedCache(ring *ConsistentHashRing, db *Database) *DistributedCache {
    return &DistributedCache{
        ring:  ring,
        nodes: make(map[string]*LocalCache),
        db:    db,
    }
}

func (dc *DistributedCache) AddNode(nodeID string) {
    dc.ring.AddNode(nodeID)
    dc.nodes[nodeID] = NewLocalCache()
}
```

### Step 3: Cache-Aside and Write-Through Strategies

```go
func (dc *DistributedCache) CacheAsideGet(key string) string {
    nodeID := dc.ring.GetNode(key)
    node := dc.nodes[nodeID]
    if v, ok := node.Get(key); ok {
        return v
    }
    dc.mu.Lock()
    dc.dbReads++
    dc.mu.Unlock()
    v, _ := dc.db.Get(key)
    node.Set(key, v, 30*time.Second)
    return v
}

func (dc *DistributedCache) CacheAsideSet(key, value string) {
    dc.db.Set(key, value)
    nodeID := dc.ring.GetNode(key)
    dc.nodes[nodeID].Set(key, value, 30*time.Second)
}

func (dc *DistributedCache) WriteThroughGet(key string) string {
    nodeID := dc.ring.GetNode(key)
    node := dc.nodes[nodeID]
    if v, ok := node.Get(key); ok {
        return v
    }
    dc.mu.Lock()
    dc.dbReads++
    dc.mu.Unlock()
    v, _ := dc.db.Get(key)
    node.Set(key, v, 30*time.Second)
    return v
}

func (dc *DistributedCache) WriteThroughSet(key, value string) {
    dc.db.Set(key, value)
    nodeID := dc.ring.GetNode(key)
    dc.nodes[nodeID].Set(key, value, 30*time.Second)
}
```

### Step 4: Stampede Protection

```go
type StampedeProtector struct {
    locks   map[string]*sync.Mutex
    mu      sync.Mutex
}

func NewStampedeProtector() *StampedeProtector {
    return &StampedeProtector{locks: make(map[string]*sync.Mutex)}
}

func (sp *StampedeProtector) Acquire(key string) *sync.Mutex {
    sp.mu.Lock()
    defer sp.mu.Unlock()
    if _, ok := sp.locks[key]; !ok {
        sp.locks[key] = &sync.Mutex{}
    }
    return sp.locks[key]
}

func (dc *DistributedCache) CacheAsideGetProtected(key string, sp *StampedeProtector) string {
    nodeID := dc.ring.GetNode(key)
    node := dc.nodes[nodeID]
    if v, ok := node.Get(key); ok {
        return v
    }
    lock := sp.Acquire(key)
    lock.Lock()
    defer lock.Unlock()
    if v, ok := node.Get(key); ok {
        return v
    }
    dc.mu.Lock()
    dc.dbReads++
    dc.mu.Unlock()
    v, _ := dc.db.Get(key)
    node.Set(key, v, 30*time.Second)
    return v
}
```

### Step 5: Demo — Balanced Distribution and Minimal Remapping

The full demo puts it all together: distribute 1000 keys across 5 nodes, verify balance, add a 6th node, and show less than 20% of keys remap.

```go
func demo() {
    ring := NewConsistentHashRing(150)
    db := NewDatabase()
    dc := NewDistributedCache(ring, db)

    nodeIDs := []string{"node-A", "node-B", "node-C", "node-D", "node-E"}
    for _, id := range nodeIDs {
        dc.AddNode(id)
    }

    for i := 0; i < 1000; i++ {
        key := fmt.Sprintf("key:%d", i)
        db.Set(key, fmt.Sprintf("value:%d", i))
    }

    fmt.Println("=== Distribution across 5 nodes ===")
    counts := make(map[string]int)
    for i := 0; i < 1000; i++ {
        nodeID := ring.GetNode(fmt.Sprintf("key:%d", i))
        counts[nodeID]++
    }
    for _, id := range nodeIDs {
        fmt.Printf("  %s: %d keys\n", id, counts[id])
    }

    minCount, maxCount := math.MaxInt32, 0
    for _, c := range counts {
        if c < minCount { minCount = c }
        if c > maxCount { maxCount = c }
    }
    fmt.Printf("  Min: %d, Max: %d, Ideal: 200, Spread: %.1f%%\n",
        minCount, maxCount, float64(maxCount-minCount)/200.0*100)

    fmt.Println("\n=== Adding node-F (6th node) ===")
    originalAssignment := make(map[string]string)
    for i := 0; i < 1000; i++ {
        key := fmt.Sprintf("key:%d", i)
        originalAssignment[key] = ring.GetNode(key)
    }

    dc.AddNode("node-F")
    remapped := 0
    for i := 0; i < 1000; i++ {
        key := fmt.Sprintf("key:%d", i)
        if ring.GetNode(key) != originalAssignment[key] {
            remapped++
        }
    }
    fmt.Printf("  Keys remapped: %d / 1000 (%.1f%%)\n", remapped, float64(remapped)/10.0)
    fmt.Printf("  (Ideal: ~16.7%%, i.e. 1/6 of keys, naive mod would remap ~83%%)\n")

    fmt.Println("\n=== Cache-aside strategy ===")
    dc2 := NewDistributedCache(NewConsistentHashRing(150), db)
    for _, id := range []string{"nA", "nB", "nC", "nD", "nE"} {
        dc2.AddNode(id)
    }
    v := dc2.CacheAsideGet("key:42")
    fmt.Printf("  First get (miss): %s (DB reads: %d)\n", v, dc2.dbReads)
    v = dc2.CacheAsideGet("key:42")
    fmt.Printf("  Second get (hit):  %s (DB reads: %d)\n", v, dc2.dbReads)

    fmt.Println("\n=== Write-through strategy ===")
    dc2.WriteThroughSet("key:99", "fresh-value")
    v = dc2.WriteThroughGet("key:99")
    fmt.Printf("  Write-then-read: %s (DB reads: %d)\n", v, dc2.dbReads)

    fmt.Println("\n=== Cache stampede simulation ===")
    stampedeDemo()
}
```

### Step 6: Cache Stampede Simulation

100 concurrent goroutines all request the same cold key. Without protection, every goroutine hits the database. With lock-per-key, only one does.

```go
func stampedeDemo() {
    ring := NewConsistentHashRing(150)
    db := NewDatabase()
    db.Set("hot-key", "hot-value")

    dcUnprotected := NewDistributedCache(ring, db)
    for _, id := range []string{"sA", "sB", "sC"} {
        dcUnprotected.AddNode(id)
    }

    var wg sync.WaitGroup
    var dbReadsUnprotected int64
    for i := 0; i < 100; i++ {
        wg.Add(1)
        go func() {
            defer wg.Done()
            dcUnprotected.CacheAsideGet("hot-key")
            atomic.AddInt64(&dbReadsUnprotected, dcUnprotected.dbReads)
        }()
    }
    wg.Wait()
    fmt.Printf("  WITHOUT stampede protection: %d DB reads for 100 concurrent requests\n",
        dcUnprotected.dbReads)

    ring2 := NewConsistentHashRing(150)
    db2 := NewDatabase()
    db2.Set("hot-key", "hot-value")
    dcProtected := NewDistributedCache(ring2, db2)
    for _, id := range []string{"pA", "pB", "pC"} {
        dcProtected.AddNode(id)
    }
    sp := NewStampedeProtector()

    var wg2 sync.WaitGroup
    for i := 0; i < 100; i++ {
        wg2.Add(1)
        go func() {
            defer wg2.Done()
            dcProtected.CacheAsideGetProtected("hot-key", sp)
        }()
    }
    wg2.Wait()
    fmt.Printf("  WITH stampede protection:    %d DB reads for 100 concurrent requests\n",
        dcProtected.dbReads)
}
```

```go
func main() {
    demo()
}
```

## Use It

### Redis Clustering: Hash Slots

Redis Cluster doesn't use consistent hashing — it uses **hash slots**. The keyspace is divided into 16,384 slots. Each key maps to a slot via `CRC16(key) % 16384`. Slots are assigned to nodes; reshuffling means moving slots, not rehashing every key. This gives Redis an advantage: moving a slot moves a contiguous range of keys, which is efficient to implement as a bulk operation.

The tradeoff: hash slots require a fixed number of slots (16,384), and adding nodes beyond that limit is impossible. 16,384 is chosen to allow up to ~1,000 nodes with a reasonable number of slots per node. Consistent hashing has no such hard limit but has less predictable slot boundaries.

### Memcached's Client-Side Distribution

Memcached itself is unaware of other nodes — it's a simple key-value store. Distribution is handled entirely by the client library. Early clients used modulo hashing (`hash(key) % N`), which Karger's consistent hashing paper explicitly shows is poor under node changes. Modern clients use consistent hashing with Ketama (a specific consistent hashing algorithm with vnodes).

### Dynamo's Use of Consistent Hashing

Amazon's Dynamo (the system behind DynamoDB) uses consistent hashing with virtual nodes, closely matching the design you just built. Each node is assigned to multiple positions on the ring, and data is replicated to the next N-1 nodes clockwise. This is also the approach used by Apache Cassandra and Riak.

## Read the Source

- [Redis Cluster hash slot implementation (`cluster.c`)](https://github.com/redis/redis/blob/unstable/src/cluster.c) — Look at `keyHashSlot()` which determines which of the 16,384 slots a key belongs to, and `clusterGetNodeByKey()` for routing.
- [Memcached slab allocator (`slabs.c`)](https://github.com/memcached/memcached/blob/master/slabs.c) — Shows how Memcached pre-allocates memory in slab classes to eliminate per-allocation fragmentation overhead.
- [HashRing Java implementation (Ketama)](https://github.com/RJ/ketama) — The original Ketama consistent hashing library that popularized virtual nodes in the Memcached ecosystem.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A Go package providing `ConsistentHashRing` and `DistributedCache`** — importable in later phases for distributing cache traffic across nodes, useful in the Phase 11 capstone (a Raft-replicated KV store).

## Exercises

1. **Easy** — Modify the demo to remove a node (instead of adding one) and verify that only ~1/K of keys remap to other nodes, with the remaining keys staying put.
2. **Medium** — Implement Rendezvous hashing (HRW) alongside the consistent hash ring. Compare the key distribution balance of both approaches with 3, 5, and 10 nodes using coefficient of variation. Which produces more even distributions?
3. **Hard** — Implement probabilistic early expiration: when a cached entry's remaining TTL is less than `random(0, TTL * beta)` where `beta` is a tunable parameter, refresh it from the database before it expires. Simulate thundering herds with and without this optimization and measure the difference in peak database load.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Consistent hashing | "A hash that doesn't change" | A ring-based key-to-node mapping where adding/removing a node only remaps keys in the affected ring segment (~1/K of keys) |
| Virtual nodes (vnodes) | "Extra copies of nodes" | Multiple hash positions per physical node on the ring, improving distribution balance without adding real capacity |
| Cache-aside | "The normal caching pattern" | Lazy-loading: the cache is never the source of truth; on miss, read from DB then populate cache |
| Write-through | "Write to cache and DB" | Every write goes to both cache and DB before returning — guarantees consistency at the cost of write latency |
| Cache stampede / thundering herd | "Too many cache misses" | When a hot key expires, thousands of concurrent requests bypass the cache and hammer the DB simultaneously |
| Rendezvous hashing (HRW) | "Another way to hash" | Highest Random Weight — compute hash(key, node) for all nodes, pick the highest; O(N) lookup but perfectly balanced without vnodes |
| TTL | "How long until it goes away" | Time-To-Live — absolute expiration time after which a cached entry is stale and must be refreshed |
| Slab allocator | "Memcached's memory thing" | Memory divided into pre-sized chunk classes (e.g. 64B, 128B, 256B) to eliminate malloc/free fragmentation at the cost of internal waste per item |

## Further Reading

- [Consistent Hashing and Random Trees (Karger et al., 1997)](https://www.akamai.com/us/en/multimedia/documents/technical-publication/consistent-hashing-and-random-trees-distributed-caching-protocol-for-relieving-hot-spots-on-the-world-wide-web-technical-publication.pdf) — The original paper that introduced consistent hashing for distributing web traffic across caches.
- [Web Caching with Consistent Hashing (Karger et al.)](https://www.cs.princeton.edu/courses/archive/fall09/cos518/papers/consistent-hashing-web.pdf) — Shorter, practical version of the paper with explicit treatment of virtual nodes.
- [Redis Cluster Specification](https://redis.io/docs/reference/cluster-spec/) — How Redis distributes keys across 16,384 hash slots and handles resharding, failover, and redirection.
- [Memcached: A Distributed Memory Object Caching System](https://memcached.org/) — The original Memcached paper and documentation. Read the protocol spec for how multi-get works across nodes.
- [Preventing Cache Stampedes with Probabilistic Early Expiration (Fan et al.)](https://cseweb.ucsd.edu//~ravikanth/CSE224S18/Preventing-Cache-Stampedes-Eurosys12.pdf) — The paper that formalized probabilistic early expiration as a solution to thundering herds.