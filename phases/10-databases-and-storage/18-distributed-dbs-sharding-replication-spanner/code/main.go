package main

import (
	"crypto/sha256"
	"encoding/binary"
	"fmt"
	"math/rand"
	"sort"
	"strconv"
	"sync"
	"time"
)

// ── Consistent Hash Ring ──────────────────────────────────────────────────

type VNode struct {
	ID   uint64
	Node string
}

type ConsistentHashRing struct {
	vnodes   []VNode
	nodes    map[string]int
	replicas int
}

func hashKey(key string) uint64 {
	h := sha256.Sum256([]byte(key))
	return binary.BigEndian.Uint64(h[:8])
}

func NewRing(replicas int) *ConsistentHashRing {
	return &ConsistentHashRing{
		vnodes:   []VNode{},
		nodes:    make(map[string]int),
		replicas: replicas,
	}
}

func (r *ConsistentHashRing) AddNode(node string) {
	r.nodes[node] = 0
	for i := 0; i < r.replicas; i++ {
		id := hashKey(node + ":" + strconv.Itoa(i))
		r.vnodes = append(r.vnodes, VNode{ID: id, Node: node})
	}
	sort.Slice(r.vnodes, func(i, j int) bool {
		return r.vnodes[i].ID < r.vnodes[j].ID
	})
}

func (r *ConsistentHashRing) RemoveNode(node string) {
	delete(r.nodes, node)
	filtered := make([]VNode, 0, len(r.vnodes))
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
	h := hashKey(key)
	idx := sort.Search(len(r.vnodes), func(i int) bool {
		return r.vnodes[i].ID >= h
	})
	if idx == len(r.vnodes) {
		idx = 0
	}
	return r.vnodes[idx].Node
}

func (r *ConsistentHashRing) NodeList() []string {
	ns := make([]string, 0, len(r.nodes))
	for n := range r.nodes {
		ns = append(ns, n)
	}
	sort.Strings(ns)
	return ns
}

// ── Sharded KV Store ──────────────────────────────────────────────────────

type ShardedKV struct {
	ring   *ConsistentHashRing
	stores map[string]map[string]string
}

func NewShardedKV(replicas int) *ShardedKV {
	return &ShardedKV{
		ring:   NewRing(replicas),
		stores: make(map[string]map[string]string),
	}
}

func (kv *ShardedKV) AddNode(node string) {
	kv.ring.AddNode(node)
	if kv.stores[node] == nil {
		kv.stores[node] = make(map[string]string)
	}
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
	for _, node := range kv.ring.NodeList() {
		fmt.Printf("  %s (%d keys):\n", node, len(kv.stores[node]))
		keys := make([]string, 0, len(kv.stores[node]))
		for k := range kv.stores[node] {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		for _, k := range keys {
			fmt.Printf("    %s → %s\n", k, kv.stores[node][k])
		}
	}
}

// ── Simplified Raft ───────────────────────────────────────────────────────

type LogEntry struct {
	Term    int
	Command string
}

type RaftNode struct {
	mu       sync.Mutex
	id       string
	peers    []string
	state    string
	term     int
	votedFor string
	log      []LogEntry
	commit   int
	stop     chan struct{}
}

func NewRaftNode(id string, peers []string) *RaftNode {
	return &RaftNode{
		id:       id,
		peers:    peers,
		state:    "follower",
		term:     0,
		votedFor: "",
		log:      []LogEntry{},
		commit:   -1,
		stop:     make(chan struct{}),
	}
}

func (n *RaftNode) Run(wg *sync.WaitGroup) {
	defer wg.Done()
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
	timer := time.NewTimer(randomTimeout())
	select {
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
	term := n.term
	n.mu.Unlock()

	votes := 1
	majority := (len(n.peers)+1)/2 + 1

	for _, peer := range n.peers {
		go func(p string) {
			if n.requestVote(p, term) {
				n.mu.Lock()
				votes++
				if votes >= majority && n.state == "candidate" {
					n.state = "leader"
					fmt.Printf("  [raft] %s elected leader for term %d\n", n.id, n.term)
				}
				n.mu.Unlock()
			}
		}(peer)
	}

	timer := time.NewTimer(randomTimeout())
	<-timer.C
	n.mu.Lock()
	if n.state == "candidate" {
		n.state = "follower"
	}
	n.mu.Unlock()
}

func (n *RaftNode) runLeader() {
	for _, peer := range n.peers {
		go n.appendEntries(peer)
	}
	ticker := time.NewTicker(100 * time.Millisecond)
	defer ticker.Stop()
	select {
	case <-ticker.C:
	case <-n.stop:
	}
}

func (n *RaftNode) requestVote(peer string, term int) bool {
	return true
}

func (n *RaftNode) appendEntries(peer string) {}

func (n *RaftNode) Propose(command string) {
	n.mu.Lock()
	defer n.mu.Unlock()
	if n.state != "leader" {
		return
	}
	n.log = append(n.log, LogEntry{Term: n.term, Command: command})
	n.commit = len(n.log) - 1
	fmt.Printf("  [raft] %s committed [%d] %s (commit=%d)\n", n.id, n.term, command, n.commit)
}

func (n *RaftNode) GetLog() []LogEntry {
	n.mu.Lock()
	defer n.mu.Unlock()
	return n.log
}

func (n *RaftNode) Stop() {
	close(n.stop)
}

func randomTimeout() time.Duration {
	return time.Duration(150+rand.Intn(150)) * time.Millisecond
}

// ── Data distribution report ──────────────────────────────────────────────

func distributionReport(kv *ShardedKV, keys []string) {
	counts := make(map[string]int)
	for _, k := range keys {
		node := kv.ring.GetNode(k)
		counts[node]++
	}
	total := len(keys)
	fmt.Println("  Distribution:")
	for _, node := range kv.ring.NodeList() {
		pct := float64(counts[node]) / float64(total) * 100
		fmt.Printf("    %s: %d/%d (%.1f%%)\n", node, counts[node], total, pct)
	}
}

// ── Main ──────────────────────────────────────────────────────────────────

func main() {
	rand.Seed(time.Now().UnixNano())

	fmt.Println("╔══════════════════════════════════════════════════════╗")
	fmt.Println("║  10.18  Distributed DBs — Sharding, Replication     ║")
	fmt.Println("╚══════════════════════════════════════════════════════╝")

	// ── Part 1: Consistent Hash Ring ──────────────────────────────────
	fmt.Println("\n── Part 1: Consistent Hash Ring ──")
	ring := NewRing(3)
	ring.AddNode("node-a")
	ring.AddNode("node-b")
	ring.AddNode("node-c")

	keys := []string{"alice", "bob", "charlie", "dave", "eve", "frank",
		"grace", "heidi", "ivan", "judy", "kira", "leo"}
	for _, k := range keys {
		fmt.Printf("  %-8s → %s\n", k, ring.GetNode(k))
	}

	// ── Part 2: Sharded KV Store ──────────────────────────────────────
	fmt.Println("\n── Part 2: Sharded KV Store ──")
	kv := NewShardedKV(5)
	kv.AddNode("server-1")
	kv.AddNode("server-2")
	kv.AddNode("server-3")

	insertKeys := make([]string, 30)
	for i := 0; i < 30; i++ {
		key := fmt.Sprintf("user:%03d", i)
		insertKeys[i] = key
		kv.Put(key, fmt.Sprintf("val-%d", i))
	}

	distributionReport(kv, insertKeys)

	fmt.Println("\n  Reads:")
	for _, key := range []string{"user:000", "user:010", "user:020"} {
		if v, ok := kv.Get(key); ok {
			fmt.Printf("    %s → %s\n", key, v)
		}
	}

	// ── Part 3: Rebalance (add node) ──────────────────────────────────
	fmt.Println("\n── Part 3: Rebalance — Add server-4 ──")
	kv.AddNode("server-4")
	distributionReport(kv, insertKeys)

	fmt.Println("\n── Part 4: Rebalance — Remove server-2 ──")
	kv.RemoveNode("server-2")
	distributionReport(kv, insertKeys)

	// Data still readable after rebalance (keys don't move automatically
	// in this simplified version — real systems migrate them)
	for _, key := range []string{"user:000", "user:010", "user:020"} {
		if v, ok := kv.Get(key); ok {
			fmt.Printf("    %s → %s (still readable via hash ring)\n", key, v)
		} else {
			fmt.Printf("    %s → NOT FOUND (key on removed node)\n", key)
		}
	}

	// ── Part 5: Simplified Raft ───────────────────────────────────────
	fmt.Println("\n── Part 5: Simplified Raft ──")
	var wg sync.WaitGroup

	n1 := NewRaftNode("n1", []string{"n2", "n3"})
	n2 := NewRaftNode("n2", []string{"n1", "n3"})
	n3 := NewRaftNode("n3", []string{"n1", "n2"})

	wg.Add(3)
	go n1.Run(&wg)
	go n2.Run(&wg)
	go n3.Run(&wg)

	// Force n1 as leader for demo clarity
	n1.mu.Lock()
	n1.state = "leader"
	n1.term = 1
	n1.mu.Unlock()

	n1.Propose("SET x = 42")
	n1.Propose("SET y = hello")
	n1.Propose("SET z = 3.14")

	time.Sleep(200 * time.Millisecond)

	n1.Stop()
	n2.Stop()
	n3.Stop()

	fmt.Println("\n  n1 committed log:")
	for _, e := range n1.GetLog() {
		fmt.Printf("    [term %d] %s\n", e.Term, e.Command)
	}

	fmt.Println("\n╔══════════════════════════════════════════════════════╗")
	fmt.Println("║  Done. See docs/en.md for the full lesson.           ║")
	fmt.Println("╚══════════════════════════════════════════════════════╝")
}
