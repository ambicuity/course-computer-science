package main

import (
	"crypto/sha256"
	"fmt"
	"math"
	"sort"
	"sync"
	"sync/atomic"
	"time"
)

func sha256Hash(data string) uint32 {
	h := sha256.Sum256([]byte(data))
	return uint32(h[0])<<24 | uint32(h[1])<<16 | uint32(h[2])<<8 | uint32(h[3])
}

type VirtualNode struct {
	hash   uint32
	nodeID string
}

type ConsistentHashRing struct {
	vnodes    []VirtualNode
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

func (r *ConsistentHashRing) GetNodeMapping(keys []string) map[string]string {
	mapping := make(map[string]string, len(keys))
	for _, k := range keys {
		mapping[k] = r.GetNode(k)
	}
	return mapping
}

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
	data    map[string]string
	mu      sync.RWMutex
	readCnt int64
}

func NewDatabase() *Database {
	return &Database{data: make(map[string]string)}
}

func (db *Database) Get(key string) (string, bool) {
	atomic.AddInt64(&db.readCnt, 1)
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

func (db *Database) ReadCount() int64 {
	return atomic.LoadInt64(&db.readCnt)
}

type DistributedCache struct {
	ring     *ConsistentHashRing
	nodes    map[string]*LocalCache
	db       *Database
	dbReads  int64
	mu       sync.Mutex
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

type StampedeProtector struct {
	locks map[string]*sync.Mutex
	mu    sync.Mutex
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

func main() {
	fmt.Println("========================================")
	fmt.Println(" Distributed Caches — Consistent Hashing")
	fmt.Println("========================================")

	ring := NewConsistentHashRing(150)
	db := NewDatabase()
	for i := 0; i < 1000; i++ {
		db.Set(fmt.Sprintf("key:%d", i), fmt.Sprintf("value:%d", i))
	}

	nodeIDs := []string{"node-A", "node-B", "node-C", "node-D", "node-E"}
	dc := NewDistributedCache(ring, db)
	for _, id := range nodeIDs {
		dc.AddNode(id)
	}

	fmt.Println("\n--- Distribution of 1000 keys across 5 nodes ---")
	counts := make(map[string]int)
	for i := 0; i < 1000; i++ {
		nodeID := ring.GetNode(fmt.Sprintf("key:%d", i))
		counts[nodeID]++
	}
	minCount, maxCount := math.MaxInt32, 0
	for _, id := range nodeIDs {
		fmt.Printf("  %s: %d keys\n", id, counts[id])
		if counts[id] < minCount {
			minCount = counts[id]
		}
		if counts[id] > maxCount {
			maxCount = counts[id]
		}
	}
	ideal := 1000 / len(nodeIDs)
	fmt.Printf("  Min: %d, Max: %d, Ideal: %d, Spread: %.1f%%\n",
		minCount, maxCount, ideal, float64(maxCount-minCount)/float64(ideal)*100)

	fmt.Println("\n--- Adding node-F (6th node) ---")
	originalMapping := make(map[string]string)
	for i := 0; i < 1000; i++ {
		key := fmt.Sprintf("key:%d", i)
		originalMapping[key] = ring.GetNode(key)
	}
	dc.AddNode("node-F")
	remapped := 0
	for i := 0; i < 1000; i++ {
		key := fmt.Sprintf("key:%d", i)
		if ring.GetNode(key) != originalMapping[key] {
			remapped++
		}
	}
	fmt.Printf("  Keys remapped: %d / 1000 (%.1f%%)\n", remapped, float64(remapped)/10.0)
	fmt.Printf("  Ideal for 5→6 nodes: ~16.7%% (1/6 of keys)\n")
	fmt.Printf("  Naive mod (hash%%5→hash%%6): ~83%% remap\n")

	fmt.Println("\n--- Removing node-C (back to 5 nodes) ---")
	beforeRemove := make(map[string]string)
	for i := 0; i < 1000; i++ {
		key := fmt.Sprintf("key:%d", i)
		beforeRemove[key] = ring.GetNode(key)
	}
	ring.RemoveNode("node-C")
	remapOnRemove := 0
	for i := 0; i < 1000; i++ {
		key := fmt.Sprintf("key:%d", i)
		if ring.GetNode(key) != beforeRemove[key] {
			remapOnRemove++
		}
	}
	fmt.Printf("  Keys remapped: %d / 1000 (%.1f%%)\n", remapOnRemove, float64(remapOnRemove)/10.0)
	fmt.Printf("  Only node-C's keys moved to other nodes\n")

	fmt.Println("\n--- Cache-aside strategy ---")
	ring2 := NewConsistentHashRing(150)
	db2 := NewDatabase()
	db2.Set("user:42", "Alice")
	db2.Set("user:99", "Bob")
	dc2 := NewDistributedCache(ring2, db2)
	for _, id := range []string{"nA", "nB", "nC", "nD", "nE"} {
		dc2.AddNode(id)
	}
	v := dc2.CacheAsideGet("user:42")
	fmt.Printf("  First get (miss): value=%s, DB reads=%d\n", v, dc2.dbReads)
	v = dc2.CacheAsideGet("user:42")
	fmt.Printf("  Second get (hit):  value=%s, DB reads=%d\n", v, dc2.dbReads)

	fmt.Println("\n--- Write-through strategy ---")
	dc2.WriteThroughSet("user:42", "Alice-Updated")
	v = dc2.WriteThroughGet("user:42")
	fmt.Printf("  After write-then-read: value=%s, DB reads=%d\n", v, dc2.dbReads)

	fmt.Println("\n--- Cache stampede simulation ---")
	stampedeDemo()
}

type StampedeCache struct {
	ring  *ConsistentHashRing
	nodes map[string]*LocalCache
	db    *Database
}

func NewStampedeCache(ring *ConsistentHashRing, db *Database) *StampedeCache {
	return &StampedeCache{ring: ring, nodes: make(map[string]*LocalCache), db: db}
}

func (sc *StampedeCache) AddNode(nodeID string) {
	sc.ring.AddNode(nodeID)
	sc.nodes[nodeID] = NewLocalCache()
}

func (sc *StampedeCache) getUnprotected(key string) {
	nodeID := sc.ring.GetNode(key)
	node := sc.nodes[nodeID]
	if _, ok := node.Get(key); ok {
		return
	}
	time.Sleep(5 * time.Millisecond)
	sc.db.Get(key)
	node.Set(key, "value", 30*time.Second)
}

func (sc *StampedeCache) getProtected(key string, sp *StampedeProtector) string {
	nodeID := sc.ring.GetNode(key)
	node := sc.nodes[nodeID]
	if v, ok := node.Get(key); ok {
		return v
	}
	lock := sp.Acquire(key)
	lock.Lock()
	defer lock.Unlock()
	if v, ok := node.Get(key); ok {
		return v
	}
	v, _ := sc.db.Get(key)
	node.Set(key, v, 30*time.Second)
	return v
}

func stampedeDemo() {
	fmt.Println("  Simulating 100 concurrent requests for the same cold key")

	ringU := NewConsistentHashRing(150)
	dbU := NewDatabase()
	dbU.Set("hot-key", "hot-value")
	scU := NewStampedeCache(ringU, dbU)
	for _, id := range []string{"sA", "sB", "sC"} {
		scU.AddNode(id)
	}

	var wg sync.WaitGroup
	for i := 0; i < 100; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			scU.getUnprotected("hot-key")
		}()
	}
	wg.Wait()
	fmt.Printf("  WITHOUT protection: %d DB reads (many goroutines hit DB before cache fills)\n", dbU.ReadCount())

	ringP := NewConsistentHashRing(150)
	dbP := NewDatabase()
	dbP.Set("hot-key", "hot-value")
	scP := NewStampedeCache(ringP, dbP)
	for _, id := range []string{"pA", "pB", "pC"} {
		scP.AddNode(id)
	}
	sp := NewStampedeProtector()

	var wg2 sync.WaitGroup
	for i := 0; i < 100; i++ {
		wg2.Add(1)
		go func() {
			defer wg2.Done()
			scP.getProtected("hot-key", sp)
		}()
	}
	wg2.Wait()
	fmt.Printf("  WITH protection:    %d DB read  (only 1 request hits DB)\n", dbP.ReadCount())
}