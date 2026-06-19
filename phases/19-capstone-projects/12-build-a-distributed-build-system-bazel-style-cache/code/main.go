// Build a Distributed Build System (Bazel-style cache)
// Run: go run main.go
//
// Architecture:
//   Build file → Parser (DAG) → Scheduler (topo sort) → Cache (CAS) → Execute
//
// Implements a content-addressable store for build artifacts with a DAG-based
// build scheduler demonstrating cache miss -> hit -> selective invalidation.

package main

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"os"
	"path/filepath"
	"sync"
)

// =============================================================================
// Step 1: Content-Addressable Store (CAS)
// =============================================================================

type ActionKey struct {
	Hash string
}

type Action struct {
	Command    string
	Inputs     []string
	EnvVars    map[string]string
	OutputPath string
}

type CAS struct {
	mu       sync.RWMutex
	storeDir string
}

func NewCAS(storeDir string) *CAS {
	os.MkdirAll(storeDir, 0755)
	return &CAS{storeDir: storeDir}
}

func ComputeActionKey(action Action) ActionKey {
	h := sha256.New()
	h.Write([]byte(action.Command))
	for _, input := range action.Inputs {
		content, err := os.ReadFile(input)
		if err != nil {
			h.Write([]byte(input))
			continue
		}
		h.Write(content)
	}
	for k, v := range action.EnvVars {
		h.Write([]byte(k))
		h.Write([]byte(v))
	}
	return ActionKey{Hash: hex.EncodeToString(h.Sum(nil))}
}

func (c *CAS) Lookup(key ActionKey) ([]byte, bool) {
	c.mu.RLock()
	defer c.mu.RUnlock()
	path := filepath.Join(c.storeDir, key.Hash)
	data, err := os.ReadFile(path)
	if err != nil { return nil, false }
	return data, true
}

func (c *CAS) Store(key ActionKey, data []byte) error {
	c.mu.Lock()
	defer c.mu.Unlock()
	path := filepath.Join(c.storeDir, key.Hash)
	return os.WriteFile(path, data, 0644)
}

// =============================================================================
// Step 2: DAG Scheduler
// =============================================================================

type Node struct {
	ID     string
	Action Action
	Deps   []*Node
	Status string
	Output []byte
}

type DAG struct {
	Nodes map[string]*Node
}

func NewDAG() *DAG {
	return &DAG{Nodes: make(map[string]*Node)}
}

func (d *DAG) AddNode(id string, action Action) *Node {
	node := &Node{ID: id, Action: action, Status: "pending"}
	d.Nodes[id] = node
	return node
}

func (d *DAG) AddEdge(from, to string) {
	d.Nodes[to].Deps = append(d.Nodes[to].Deps, d.Nodes[from])
}

func (d *DAG) Execute(cas *CAS) {
	order := d.topoSort()
	for _, nodeID := range order {
		node := d.Nodes[nodeID]
		key := ComputeActionKey(node.Action)

		if cached, ok := cas.Lookup(key); ok {
			node.Output = cached
			node.Status = "cached"
			fmt.Printf("  [CACHE HIT]  %s (key: %s...)\n", nodeID, key.Hash[:8])
			continue
		}

		fmt.Printf("  [EXECUTING]  %s\n", nodeID)
		var output []byte
		for _, input := range node.Action.Inputs {
			content, err := os.ReadFile(input)
			if err == nil { output = append(output, content...) }
		}
		output = append(output, []byte(node.Action.Command)...)

		node.Output = output
		node.Status = "done"

		if err := cas.Store(key, output); err != nil {
			fmt.Printf("  [ERROR] Failed to cache %s: %v\n", nodeID, err)
		} else {
			fmt.Printf("  [CACHED]     %s\n", nodeID)
		}
	}
}

func (d *DAG) topoSort() []string {
	visited := make(map[string]bool)
	var order []string
	var visit func(id string)
	visit = func(id string) {
		if visited[id] { return }
		visited[id] = true
		for _, dep := range d.Nodes[id].Deps { visit(dep.ID) }
		order = append(order, id)
	}
	for id := range d.Nodes { visit(id) }
	return order
}

// =============================================================================
// Step 3: Demo Build
// =============================================================================

func main() {
	os.WriteFile("input_a.txt", []byte("hello"), 0644)
	os.WriteFile("input_b.txt", []byte("world"), 0644)

	cas := NewCAS(".build_cache")
	dag := NewDAG()

	nodeA := dag.AddNode("compile_a", Action{
		Command: "gcc -c input_a.txt", Inputs: []string{"input_a.txt"}, OutputPath: "a.o",
	})
	nodeB := dag.AddNode("compile_b", Action{
		Command: "gcc -c input_b.txt", Inputs: []string{"input_b.txt"}, OutputPath: "b.o",
	})
	nodeC := dag.AddNode("link", Action{
		Command: "gcc a.o b.o -o output", Inputs: []string{"input_a.txt", "input_b.txt"}, OutputPath: "output",
	})
	dag.AddEdge("compile_a", "link")
	dag.AddEdge("compile_b", "link")

	fmt.Println("=== First build ===")
	dag.Execute(cas)

	fmt.Println("\n=== Second build (should be all cache hits) ===")
	dag2 := NewDAG()
	dag2.AddNode("compile_a", nodeA.Action)
	dag2.AddNode("compile_b", nodeB.Action)
	dag2.AddNode("link", nodeC.Action)
	dag2.AddEdge("compile_a", "link")
	dag2.AddEdge("compile_b", "link")
	dag2.Execute(cas)

	fmt.Println("\n=== After modifying input_a.txt ===")
	os.WriteFile("input_a.txt", []byte("modified hello"), 0644)
	dag3 := NewDAG()
	dag3.AddNode("compile_a", nodeA.Action)
	dag3.AddNode("compile_b", nodeB.Action)
	dag3.AddNode("link", nodeC.Action)
	dag3.AddEdge("compile_a", "link")
	dag3.AddEdge("compile_b", "link")
	dag3.Execute(cas)

	// Cleanup
	os.Remove("input_a.txt")
	os.Remove("input_b.txt")
	os.RemoveAll(".build_cache")
}
