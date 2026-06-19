package main

import (
	"context"
	"fmt"
	"math/rand"
	"net/http"
	"sort"
	"sync"
	"time"
)

type HealthStatus int

const (
	HealthUnknown HealthStatus = iota
	HealthHealthy
	HealthUnhealthy
)

func (s HealthStatus) String() string {
	switch s {
	case HealthHealthy:
		return "healthy"
	case HealthUnhealthy:
		return "unhealthy"
	default:
		return "unknown"
	}
}

type ServiceNode struct {
	ID       string
	Name     string
	Address  string
	Health   HealthStatus
	LastSeen time.Time
	TTL      time.Duration
	Metadata map[string]string
}

type ServiceRegistry struct {
	mu       sync.RWMutex
	nodes    map[string]*ServiceNode
	byName   map[string][]string
	quit     chan struct{}
	onEvict  func(nodeID string)
}

func NewServiceRegistry(checkInterval time.Duration) *ServiceRegistry {
	sr := &ServiceRegistry{
		nodes:  make(map[string]*ServiceNode),
		byName: make(map[string][]string),
		quit:   make(chan struct{}),
	}

	go sr.ttlChecker(checkInterval)
	go sr.activeHealthChecker(checkInterval)
	return sr
}

func (sr *ServiceRegistry) Stop() {
	close(sr.quit)
}

func (sr *ServiceRegistry) Register(node *ServiceNode) {
	sr.mu.Lock()
	defer sr.mu.Unlock()

	node.LastSeen = time.Now()
	node.Health = HealthHealthy
	sr.nodes[node.ID] = node
	sr.byName[node.Name] = append(sr.byName[node.Name], node.ID)
	fmt.Printf("  [registry] registered %s (%s) at %s with TTL=%v\n", node.ID, node.Name, node.Address, node.TTL)
}

func (sr *ServiceRegistry) Deregister(nodeID string) {
	sr.mu.Lock()
	defer sr.mu.Unlock()

	node, exists := sr.nodes[nodeID]
	if !exists {
		return
	}

	name := node.Name
	sr.removeNodeFromNameIndex(nodeID, name)
	delete(sr.nodes, nodeID)
	fmt.Printf("  [registry] deregistered %s (%s)\n", nodeID, name)
}

func (sr *ServiceRegistry) removeNodeFromNameIndex(nodeID, name string) {
	ids := sr.byName[name]
	for i, id := range ids {
		if id == nodeID {
			sr.byName[name] = append(ids[:i], ids[i+1:]...)
			break
		}
	}
	if len(sr.byName[name]) == 0 {
		delete(sr.byName, name)
	}
}

func (sr *ServiceRegistry) Heartbeat(nodeID string) bool {
	sr.mu.Lock()
	defer sr.mu.Unlock()

	node, exists := sr.nodes[nodeID]
	if !exists {
		return false
	}

	node.LastSeen = time.Now()
	node.Health = HealthHealthy
	return true
}

func (sr *ServiceRegistry) Discover(name string) []*ServiceNode {
	sr.mu.RLock()
	defer sr.mu.RUnlock()

	var result []*ServiceNode
	for _, id := range sr.byName[name] {
		if node, ok := sr.nodes[id]; ok && node.Health == HealthHealthy {
			result = append(result, node)
		}
	}
	return result
}

func (sr *ServiceRegistry) GetAllNodes() []*ServiceNode {
	sr.mu.RLock()
	defer sr.mu.RUnlock()

	var result []*ServiceNode
	for _, node := range sr.nodes {
		result = append(result, node)
	}
	return result
}

func (sr *ServiceRegistry) SetOnEvict(fn func(nodeID string)) {
	sr.onEvict = fn
}

func (sr *ServiceRegistry) ttlChecker(interval time.Duration) {
	ticker := time.NewTicker(interval)
	defer ticker.Stop()

	for {
		select {
		case <-sr.quit:
			return
		case <-ticker.C:
			sr.evictExpired()
		}
	}
}

func (sr *ServiceRegistry) evictExpired() {
	sr.mu.Lock()
	var expired []string
	now := time.Now()

	for id, node := range sr.nodes {
		if node.TTL > 0 && now.Sub(node.LastSeen) > node.TTL {
			expired = append(expired, id)
		}
	}

	for _, id := range expired {
		node := sr.nodes[id]
		name := node.Name
		sr.removeNodeFromNameIndex(id, name)
		delete(sr.nodes, id)
		fmt.Printf("  [registry] TTL expired for %s (%s)\n", id, name)
	}
	sr.mu.Unlock()

	for _, id := range expired {
		if sr.onEvict != nil {
			sr.onEvict(id)
		}
	}
}

func (sr *ServiceRegistry) activeHealthChecker(interval time.Duration) {
	ticker := time.NewTicker(interval)
	defer ticker.Stop()

	for {
		select {
		case <-sr.quit:
			return
		case <-ticker.C:
			sr.checkAllHealth()
		}
	}
}

func (sr *ServiceRegistry) checkAllHealth() {
	sr.mu.RLock()
	nodes := make([]*ServiceNode, 0, len(sr.nodes))
	for _, node := range sr.nodes {
		nodes = append(nodes, node)
	}
	sr.mu.RUnlock()

	for _, node := range nodes {
		if node.TTL > 0 {
			continue
		}

		healthy := sr.pingNode(node.Address)
		sr.mu.Lock()
		if n, ok := sr.nodes[node.ID]; ok {
			if healthy {
				n.Health = HealthHealthy
				n.LastSeen = time.Now()
			} else {
				n.Health = HealthUnhealthy
			}
		}
		sr.mu.Unlock()
	}
}

func (sr *ServiceRegistry) pingNode(address string) bool {
	client := http.Client{Timeout: 2 * time.Second}
	resp, err := client.Get("http://" + address + "/health")
	if err != nil {
		return false
	}
	defer resp.Body.Close()
	return resp.StatusCode == http.StatusOK
}

type ElectionMessage int

const (
	MsgElection ElectionMessage = iota
	MsgOK
	MsgCoordinator
)

type ElectionMsg struct {
	From    int
	To      int
	Type    ElectionMessage
}

type BullyNode struct {
	ID          int
	AllNodeIDs  []int
	IsLeader    bool
	LeaderID    int
	Alive       bool
	electionCh  chan ElectionMsg
	coordinatorCh chan int
}

func NewBullyNode(id int, allIDs []int) *BullyNode {
	return &BullyNode{
		ID:         id,
		AllNodeIDs: allIDs,
		Alive:      true,
		electionCh: make(chan ElectionMsg, 100),
	}
}

type BullyElection struct {
	mu    sync.Mutex
	nodes map[int]*BullyNode
}

func NewBullyElection(nodeIDs []int) *BullyElection {
	be := &BullyElection{
		nodes: make(map[int]*BullyNode),
	}
	for _, id := range nodeIDs {
		be.nodes[id] = NewBullyNode(id, nodeIDs)
	}
	return be
}

func (be *BullyElection) StartElection(initiatorID int) int {
	be.mu.Lock()
	defer be.mu.Unlock()

	node := be.nodes[initiatorID]
	if !node.Alive {
		return be.findLeader()
	}

	higherIDs := make([]int, 0)
	for _, id := range node.AllNodeIDs {
		if id > initiatorID {
			higherIDs = append(higherIDs, id)
		}
	}

	receivedOK := false
	for _, hid := range higherIDs {
		if higherNode, ok := be.nodes[hid]; ok && higherNode.Alive {
			receivedOK = true
			break
		}
	}

	if !receivedOK {
		node.IsLeader = true
		node.LeaderID = node.ID
		for _, id := range node.AllNodeIDs {
			if id != node.ID {
				if n, ok := be.nodes[id]; ok && n.Alive {
					n.IsLeader = false
					n.LeaderID = node.ID
				}
			}
		}
		return node.ID
	}

	var leaderID int
	for _, id := range node.AllNodeIDs {
		if higherNode, ok := be.nodes[id]; ok && higherNode.Alive {
			higherNode.IsLeader = false
		}
	}
	maxAliveID := -1
	for _, id := range node.AllNodeIDs {
		if n, ok := be.nodes[id]; ok && n.Alive && id > maxAliveID {
			maxAliveID = id
		}
	}
	if maxAliveID >= 0 {
		be.nodes[maxAliveID].IsLeader = true
		leaderID = maxAliveID
		for _, id := range node.AllNodeIDs {
			if n, ok := be.nodes[id]; ok && n.Alive {
				n.LeaderID = leaderID
			}
		}
	}

	return leaderID
}

func (be *BullyElection) findLeader() int {
	maxID := -1
	for id, node := range be.nodes {
		if node.Alive && id > maxID {
			maxID = id
		}
	}
	if maxID >= 0 {
		be.nodes[maxID].IsLeader = true
		for _, node := range be.nodes {
			if node.Alive {
				node.LeaderID = maxID
			}
		}
	}
	return maxID
}

func (be *BullyElection) KillNode(id int) {
	be.mu.Lock()
	defer be.mu.Unlock()
	if node, ok := be.nodes[id]; ok {
		node.Alive = false
		node.IsLeader = false
	}
}

func (be *BullyElection) ReviveNode(id int) {
	be.mu.Lock()
	defer be.mu.Unlock()
	if node, ok := be.nodes[id]; ok {
		node.Alive = true
	}
}

func (be *BullyElection) GetLeader() int {
	be.mu.Lock()
	defer be.mu.Unlock()
	for _, node := range be.nodes {
		if node.Alive && node.IsLeader {
			return node.ID
		}
	}
	return -1
}

func (be *BullyElection) PrintState() {
	be.mu.Lock()
	defer be.mu.Unlock()

	ids := make([]int, 0, len(be.nodes))
	for id := range be.nodes {
		ids = append(ids, id)
	}
	sort.Ints(ids)

	leaderID := -1
	for _, node := range be.nodes {
		if node.Alive && node.IsLeader {
			leaderID = node.ID
		}
	}

	fmt.Printf("    Leader: Node %d | ", leaderID)
	for _, id := range ids {
		n := be.nodes[id]
		status := "alive"
		if !n.Alive {
			status = "DEAD"
		}
		fmt.Printf("Node%d=%s  ", id, status)
	}
	fmt.Println()
}

type ZKCandidate struct {
	ID       string
	SeqNum   int
	IsActive bool
}

type ZKElection struct {
	mu         sync.Mutex
	candidates []*ZKCandidate
	nextSeq    int
	leaderID   string
	watchers   map[int]chan string
}

func NewZKElection() *ZKElection {
	return &ZKElection{
		watchers: make(map[int]chan string),
	}
}

func (zk *ZKElection) CreateEphemeralSequential(candidateID string) int {
	zk.mu.Lock()
	defer zk.mu.Unlock()

	seq := zk.nextSeq
	zk.nextSeq++

	candidate := &ZKCandidate{
		ID:       candidateID,
		SeqNum:   seq,
		IsActive: true,
	}
	zk.candidates = append(zk.candidates, candidate)

	fmt.Printf("  [zk] %s created ephemeral node with sequence %010d\n", candidateID, seq)

	if seq > 0 {
		prevSeq := seq - 1
		ch := make(chan string, 1)
		zk.watchers[prevSeq] = ch

		go func(cID string, pSeq int) {
			<-ch
			fmt.Printf("  [zk] %s notified: previous node %d deleted, checking leadership...\n", cID, pSeq)
		}(candidateID, prevSeq)
	}

	zk.determineLeader()
	return seq
}

func (zk *ZKElection) determineLeader() {
	zk.leaderID = ""
	for _, c := range zk.candidates {
		if c.IsActive {
			zk.leaderID = c.ID
			break
		}
	}
}

func (zk *ZKElection) DeleteEphemeral(seqNum int) {
	zk.mu.Lock()
	var removed *ZKCandidate
	for i, c := range zk.candidates {
		if c.SeqNum == seqNum {
			removed = c
			c.IsActive = false
			zk.candidates = append(zk.candidates[:i], zk.candidates[i+1:]...)
			break
		}
	}
	zk.determineLeader()
	zk.mu.Unlock()

	if removed != nil {
		fmt.Printf("  [zk] %s's ephemeral node deleted (session expired)\n", removed.ID)

		zk.mu.Lock()
		if ch, ok := zk.watchers[seqNum]; ok {
			ch <- "deleted"
			delete(zk.watchers, seqNum)
		}
		zk.mu.Unlock()
	}
}

func (zk *ZKElection) GetLeader() string {
	zk.mu.Lock()
	defer zk.mu.Unlock()
	return zk.leaderID
}

func (zk *ZKElection) PrintState() {
	zk.mu.Lock()
	defer zk.mu.Unlock()

	fmt.Printf("    Leader: %s | Candidates: ", zk.leaderID)
	for _, c := range zk.candidates {
		status := "active"
		if !c.IsActive {
			status = "INACTIVE"
		}
		fmt.Printf("%s(seq=%d,%s) ", c.ID, c.SeqNum, status)
	}
	fmt.Println()
}

func main() {
	rand.Seed(time.Now().UnixNano())

	fmt.Println("╔══════════════════════════════════════════════════════════════╗")
	fmt.Println("║  Service Discovery, Membership & Leader Election            ║")
	fmt.Println("╚══════════════════════════════════════════════════════════════╝")

	demoServiceRegistry()
	demoHealthCheckAndDeregistration()
	demoBullyElection()
	demoZKElection()
	demoLeaderElectionComparison()
}

func demoServiceRegistry() {
	fmt.Println("\n━━━ Demo 1: Service Registry ━━━")

	registry := NewServiceRegistry(5 * time.Second)
	defer registry.Stop()

	nodes := []*ServiceNode{
		{ID: "pay-1", Name: "payment-service", Address: "10.0.1.5:8080", TTL: 10 * time.Second, Metadata: map[string]string{"zone": "east", "version": "v2"}},
		{ID: "pay-2", Name: "payment-service", Address: "10.0.1.6:8080", TTL: 10 * time.Second, Metadata: map[string]string{"zone": "west", "version": "v2"}},
		{ID: "inv-1", Name: "inventory-service", Address: "10.0.2.3:8081", TTL: 10 * time.Second, Metadata: map[string]string{"zone": "east", "version": "v1"}},
	}

	for _, node := range nodes {
		registry.Register(node)
	}

	fmt.Println("\n  Discovering payment-service:")
	found := registry.Discover("payment-service")
	for _, n := range found {
		fmt.Printf("    → %s at %s (zone=%s, version=%s)\n", n.ID, n.Address, n.Metadata["zone"], n.Metadata["version"])
	}

	fmt.Println("\n  Discovering inventory-service:")
	found = registry.Discover("inventory-service")
	for _, n := range found {
		fmt.Printf("    → %s at %s\n", n.ID, n.Address)
	}

	fmt.Println("\n  Discovering unknown-service:")
	found = registry.Discover("unknown-service")
	fmt.Printf("    → %d instances found\n", len(found))
}

func demoHealthCheckAndDeregistration() {
	fmt.Println("\n━━━ Demo 2: Health Checking & Deregistration ━━━")

	registry := NewServiceRegistry(1 * time.Second)
	defer registry.Stop()

	nodes := []*ServiceNode{
		{ID: "svc-1", Name: "api-gateway", Address: "10.0.1.1:8080", TTL: 5 * time.Second},
		{ID: "svc-2", Name: "api-gateway", Address: "10.0.1.2:8080", TTL: 5 * time.Second},
		{ID: "svc-3", Name: "api-gateway", Address: "10.0.1.3:8080", TTL: 5 * time.Second},
	}

	for _, node := range nodes {
		registry.Register(node)
	}

	fmt.Println("\n  All nodes sending heartbeats normally:")
	for i := 0; i < 3; i++ {
		registry.Heartbeat("svc-1")
		registry.Heartbeat("svc-2")
		registry.Heartbeat("svc-3")
		time.Sleep(500 * time.Millisecond)
	}

	fmt.Println("\n  Discovering api-gateway (all healthy):")
	found := registry.Discover("api-gateway")
	fmt.Printf("    → %d healthy instances\n", len(found))

	fmt.Println("\n  Simulating svc-2 crash (no more heartbeats)...")
	fmt.Println("  svc-1 and svc-3 continue sending heartbeats...")
	fmt.Println("  Waiting for svc-2 TTL expiry (5 seconds)...")

	heartbeatCtx, heartbeatCancel := context.WithCancel(context.Background())
	go func() {
		ticker := time.NewTicker(1 * time.Second)
		defer ticker.Stop()
		for {
			select {
			case <-heartbeatCtx.Done():
				return
			case <-ticker.C:
				registry.Heartbeat("svc-1")
				registry.Heartbeat("svc-3")
			}
		}
	}()

	time.Sleep(6 * time.Second)
	heartbeatCancel()

	fmt.Println("\n  Discovering api-gateway (after svc-2 TTL expiry):")
	found = registry.Discover("api-gateway")
	fmt.Printf("    → %d healthy instances\n", len(found))
	for _, n := range found {
		fmt.Printf("    → %s at %s (%s)\n", n.ID, n.Address, n.Health)
	}

	fmt.Println("\n  Explicitly deregistering svc-3...")
	registry.Deregister("svc-3")

	fmt.Println("\n  Discovering api-gateway (after explicit deregister):")
	found = registry.Discover("api-gateway")
	fmt.Printf("    → %d healthy instances\n", len(found))
	for _, n := range found {
		fmt.Printf("    → %s at %s\n", n.ID, n.Address)
	}
}

func demoBullyElection() {
	fmt.Println("\n━━━ Demo 3: Bully Algorithm Leader Election ━━━")

	nodeIDs := []int{1, 2, 3, 4, 5}
	election := NewBullyElection(nodeIDs)

	fmt.Println("\n  Initial election (all 5 nodes alive):")
	leader := election.StartElection(1)
	fmt.Printf("  Elected leader: Node %d\n", leader)
	election.PrintState()

	fmt.Println("\n  Killing leader (Node 5)...")
	election.KillNode(5)

	fmt.Println("  Node 3 detects failure, starts election:")
	leader = election.StartElection(3)
	fmt.Printf("  New leader: Node %d\n", leader)
	election.PrintState()

	fmt.Println("\n  Killing Node 4 (current leader)...")
	election.KillNode(4)

	fmt.Println("  Node 2 detects failure, starts election:")
	leader = election.StartElection(2)
	fmt.Printf("  New leader: Node %d\n", leader)
	election.PrintState()

	fmt.Println("\n  Reviving Node 5 (highest-ID node)...")
	election.ReviveNode(5)

	fmt.Println("  Node 5 starts election (highest ID wins):")
	leader = election.StartElection(5)
	fmt.Printf("  New leader: Node %d\n", leader)
	election.PrintState()
}

func demoZKElection() {
	fmt.Println("\n━━━ Demo 4: ZooKeeper-Style Leader Election ━━━")

	zk := NewZKElection()

	fmt.Println("\n  Three candidates join sequentially:")
	seqA := zk.CreateEphemeralSequential("Node-A")
	seqB := zk.CreateEphemeralSequential("Node-B")
	seqC := zk.CreateEphemeralSequential("Node-C")

	zk.PrintState()

	fmt.Println("\n  Lowest sequence number is leader:")
	fmt.Printf("  Leader: %s\n", zk.GetLeader())

	fmt.Println("\n  Node-A (leader) crashes → ephemeral node deleted:")
	zk.DeleteEphemeral(seqA)
	zk.PrintState()

	fmt.Println("\n  Node-B now has lowest sequence → becomes leader:")
	fmt.Printf("  Leader: %s\n", zk.GetLeader())

	fmt.Println("\n  Node-B also crashes → only Node-C remains:")
	zk.DeleteEphemeral(seqB)
	zk.PrintState()

	fmt.Println("\n  Node-C is now leader:")
	fmt.Printf("  Leader: %s\n", zk.GetLeader())

	fmt.Println("\n  New nodes join → they watch the previous node:")
	seqD := zk.CreateEphemeralSequential("Node-D")
	seqE := zk.CreateEphemeralSequential("Node-E")
	zk.PrintState()
	_ = seqC
	_ = seqD
	_ = seqE

	fmt.Printf("  Leader: %s (still Node-C with lowest active seq)\n", zk.GetLeader())
}

func demoLeaderElectionComparison() {
	fmt.Println("\n━━━ Demo 5: Comparison of Leader Election Approaches ━━━")

	fmt.Println(`
  ┌─────────────────┬────────────────┬─────────────────┬─────────────────┐
  │     Property     │     Bully       │   ZK Ephemeral  │   etcd Lease    │
  ├─────────────────┼────────────────┼─────────────────┼─────────────────┤
  │ Deterministic?   │ Yes (highest ID)│ No (create race)│ No (create race)│
  │ External service?│ No              │ Yes (ZooKeeper) │ Yes (etcd)      │
  │ Split-brain?     │ Possible        │ No (ZK is CP)   │ No (etcd is CP) │
  │ Auto cleanup?    │ No              │ Yes (ephemeral) │ Yes (lease TTL) │
  │ Complexity       │ O(N²) messages  │ O(N) per elect  │ O(N) per elect  │
  │ Herd effect?     │ Yes (broadcasts)│ No (watch prev) │ No (watch key)  │
  └─────────────────┴────────────────┴─────────────────┴─────────────────┘`)

	fmt.Println("\n  Key insight: Bully is simple but dangerous in production.")
	fmt.Println("  ZK ephemeral nodes and etcd leases avoid split-brain by relying")
	fmt.Println("  on a strongly consistent coordination service.")
	fmt.Println()
	fmt.Println("  In practice, most production systems use Raft-based election")
	fmt.Println("  (built into the consensus protocol) rather than running a separate")
	fmt.Println("  leader election algorithm. etcd, Consul, and CockroachDB all use")
	fmt.Println("  Raft — the leader of the consensus group IS the service leader.")
}