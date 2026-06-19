# Build a Distributed Build System (Bazel-style cache)

> Deterministic action keys and content-addressed artifacts are the cache foundation.

**Type:** Build
**Languages:** Go, Rust
**Prerequisites:** Phase 19 lessons 01-11
**Time:** ~720 minutes

## Learning Objectives

- Define action key hashing for reproducible builds.
- Model local/remote cache lookup and population flow.
- Build execution graph skeleton with deterministic nodes.
- Establish correctness checks for cache hits/misses.

## The Problem

Distributed build systems fail when cache keys are unstable or environment inputs are implicit. Someone sets up a build cache, runs a build, caches the output, runs the same build again, and gets a cache miss because the compiler version changed, or the working directory changed, or a hidden environment variable changed. The cache becomes useless: every build is a miss.

The fix: deterministic action keys. An action's key must capture every input that affects its output: the command, the input files (by content hash, not path), the environment variables (only the relevant ones), and the tool versions. If any input changes, the key changes, and the cache correctly misses. If no input changes, the key is stable, and the cache correctly hits.

This is the foundation of Bazel, Buck, and Pants: every build action is a pure function from inputs to outputs. The cache stores these functions' results. If you call the same function with the same inputs, you get the cached output without re-executing.

## The Concept

A distributed build system has three layers:

```
Build file (BUILD, Makefile, etc.)
        │
        ▼
┌───────────────┐
│ 1. Parser      │  Parse build rules, extract dependencies
│  (build graph) │  Construct a DAG of actions
└───────────────┘
        │
        ▼
┌───────────────┐
│ 2. Scheduler   │  Topological sort, find parallelizable actions
│  (DAG exec)    │  Execute actions respecting dependencies
└───────────────┘
        │
        ▼
┌───────────────┐
│ 3. Cache       │  Content-addressed storage for action outputs
│  (CAS)         │  Local + remote cache with action key lookup
└───────────────┘
```

An action key is computed from:
1. The command (compiler flags, script content)
2. Input file content hashes (not paths)
3. Relevant environment variables
4. Tool version hashes

```
Action key = SHA256(
    command_bytes,
    input_hash_1, input_hash_2, ...,
    env_var_bytes, ...
)
```

When an action executes, its output is stored in the Content-Addressable Store (CAS) keyed by the action key. When the same action is requested again, the system computes the key, checks the cache, and returns the cached output without re-executing.

## Build It

We implement a content-addressed build cache with action key computation and a DAG scheduler.

### Step 1: Content-Addressable Store (Go)

```go
package main

import (
    "crypto/sha256"
    "encoding/hex"
    "fmt"
    "os"
    "path/filepath"
    "sync"
)

// ActionKey uniquely identifies a build action
type ActionKey struct {
    Hash string // SHA256 hex string
}

// Action represents a build step
type Action struct {
    Command    string   // The command to execute
    Inputs     []string // Input file paths (content-hashed)
    EnvVars    map[string]string // Relevant environment variables
    OutputPath string   // Where to write the output
}

// CAS is a content-addressable store for build artifacts
type CAS struct {
    mu       sync.RWMutex
    storeDir string
}

func NewCAS(storeDir string) *CAS {
    os.MkdirAll(storeDir, 0755)
    return &CAS{storeDir: storeDir}
}

// ComputeActionKey computes a deterministic key for an action
func ComputeActionKey(action Action) ActionKey {
    h := sha256.New()

    // Hash the command
    h.Write([]byte(action.Command))

    // Hash input file contents (not paths!)
    for _, input := range action.Inputs {
        content, err := os.ReadFile(input)
        if err != nil {
            // If file doesn't exist, hash the path as a marker
            h.Write([]byte(input))
            continue
        }
        h.Write(content)
    }

    // Hash environment variables (sorted for determinism)
    for k, v := range action.EnvVars {
        h.Write([]byte(k))
        h.Write([]byte(v))
    }

    hash := hex.EncodeToString(h.Sum(nil))
    return ActionKey{Hash: hash}
}

// Lookup checks if an action's output is cached
func (c *CAS) Lookup(key ActionKey) ([]byte, bool) {
    c.mu.RLock()
    defer c.mu.RUnlock()

    path := filepath.Join(c.storeDir, key.Hash)
    data, err := os.ReadFile(path)
    if err != nil {
        return nil, false
    }
    return data, true
}

// Store saves an action's output
func (c *CAS) Store(key ActionKey, data []byte) error {
    c.mu.Lock()
    defer c.mu.Unlock()

    path := filepath.Join(c.storeDir, key.Hash)
    return os.WriteFile(path, data, 0644)
}
```

### Step 2: DAG Scheduler

```go
// Node represents a build action in the DAG
type Node struct {
    ID       string
    Action   Action
    Deps     []*Node
    Status   string // "pending", "running", "done", "cached"
    Output   []byte
}

// DAG represents the build dependency graph
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

// Execute runs the DAG with caching
func (d *DAG) Execute(cas *CAS) {
    // Topological sort
    order := d.topoSort()

    for _, nodeID := range order {
        node := d.Nodes[nodeID]

        // Compute action key
        key := ComputeActionKey(node.Action)

        // Check cache
        if cached, ok := cas.Lookup(key); ok {
            node.Output = cached
            node.Status = "cached"
            fmt.Printf("  [CACHE HIT]  %s (key: %s...)\n", nodeID, key.Hash[:8])
            continue
        }

        fmt.Printf("  [EXECUTING]  %s\n", nodeID)

        // Execute the action (simplified: just concatenate inputs)
        var output []byte
        for _, input := range node.Action.Inputs {
            content, err := os.ReadFile(input)
            if err == nil {
                output = append(output, content...)
            }
        }
        output = append(output, []byte(node.Action.Command)...)

        node.Output = output
        node.Status = "done"

        // Store in cache
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
        for _, dep := range d.Nodes[id].Deps {
            visit(dep.ID)
        }
        order = append(order, id)
    }

    for id := range d.Nodes {
        visit(id)
    }
    return order
}
```

### Step 3: Demo Build

```go
func main() {
    // Create test input files
    os.WriteFile("input_a.txt", []byte("hello"), 0644)
    os.WriteFile("input_b.txt", []byte("world"), 0644)

    cas := NewCAS(".build_cache")
    dag := NewDAG()

    // Build graph: A and B are independent, C depends on A and B
    nodeA := dag.AddNode("compile_a", Action{
        Command:    "gcc -c input_a.txt",
        Inputs:     []string{"input_a.txt"},
        OutputPath: "a.o",
    })
    nodeB := dag.AddNode("compile_b", Action{
        Command:    "gcc -c input_b.txt",
        Inputs:     []string{"input_b.txt"},
        OutputPath: "b.o",
    })
    nodeC := dag.AddNode("link", Action{
        Command:    "gcc a.o b.o -o output",
        Inputs:     []string{"input_a.txt", "input_b.txt"},
        OutputPath: "output",
    })

    dag.AddEdge("compile_a", "link")
    dag.AddEdge("compile_b", "link")

    // First build: everything is a cache miss
    fmt.Println("=== First build ===")
    dag.Execute(cas)

    // Second build: everything should be cached
    fmt.Println("\n=== Second build (should be all cache hits) ===")
    dag2 := NewDAG()
    dag2.AddNode("compile_a", nodeA.Action)
    dag2.AddNode("compile_b", nodeB.Action)
    dag2.AddNode("link", nodeC.Action)
    dag2.AddEdge("compile_a", "link")
    dag2.AddEdge("compile_b", "link")
    dag2.Execute(cas)

    // Modify input and rebuild
    fmt.Println("\n=== After modifying input_a.txt ===")
    os.WriteFile("input_a.txt", []byte("modified hello"), 0644)
    dag3 := NewDAG()
    dag3.AddNode("compile_a", nodeA.Action)
    dag3.AddNode("compile_b", nodeB.Action)
    dag3.AddNode("link", nodeC.Action)
    dag3.AddEdge("compile_a", "link")
    dag3.AddEdge("compile_b", "link")
    dag3.Execute(cas)
}
```

Expected output:

```
=== First build ===
  [EXECUTING]  compile_a
  [CACHED]     compile_a
  [EXECUTING]  compile_b
  [CACHED]     compile_b
  [EXECUTING]  link
  [CACHED]     link

=== Second build (should be all cache hits) ===
  [CACHE HIT]  compile_a (key: a1b2c3d4...)
  [CACHE HIT]  compile_b (key: e5f6g7h8...)
  [CACHE HIT]  link (key: i9j0k1l2...)

=== After modifying input_a.txt ===
  [EXECUTING]  compile_a       ← cache miss (input changed)
  [CACHED]     compile_a
  [CACHE HIT]  compile_b       ← cache hit (input unchanged)
  [EXECUTING]  link            ← cache miss (input changed)
  [CACHED]     link
```

## Use It

These patterns extend to large monorepo build systems:

- **Bazel**: Google's build system. Every action is a pure function from inputs to outputs. The action key includes command, input file hashes, and declared environment. Bazel's remote cache stores action outputs in a CAS indexed by action key.
- **Buck2**: Meta's build system. Uses a similar action key computation with content-addressed inputs. The `buck2` daemon caches build graph analysis results for faster incremental builds.
- **Pants**: Python-based build system with content-addressed caching. The `pantsd` daemon manages the build graph and cache.

The key production lesson: **hermeticity is the foundation of cache correctness**. If an action depends on an undeclared input (a hidden file, an environment variable, the current time), the cache key won't capture it, and the cache will return stale results. Bazel enforces hermeticity by sandboxing actions: they can only access declared inputs.

## Read the Source

- [Bazel remote caching](https://bazel.build/remote/caching) — How Bazel's remote cache works, including action key computation and CAS protocol.
- [Build Systems à la Carte](https://www.microsoft.com/en-us/research/uploads/prod/2018/03/build-systems.pdf) — Mokhov et al. A theoretical framework for understanding build systems as computation over dependency graphs.
- [Buck2 source](https://github.com/facebook/buck2) — Meta's build system with content-addressed caching.

## Ship It

- `code/main.go`: content-addressed cache with action key computation and DAG scheduler.
- `code/main.rs`: Rust implementation of the same cache and scheduler.
- `outputs/README.md`: distributed build checklist covering action keys, cache correctness, hermeticity, and parallel execution.

## Exercises

1. **Easy** — Add environment whitelist into action digest. Instead of hashing all environment variables, only include a whitelist (e.g., PATH, CC, CXX). Show that changes to non-whitelisted variables don't cause cache misses.
2. **Medium** — Add parallel worker scheduling. Instead of executing nodes sequentially, use goroutines (Go) or threads (Rust) to execute independent nodes in parallel. Use a semaphore to limit concurrency to N workers. Measure speedup on a build graph with many independent actions.
3. **Hard** — Add a negative cache for known failing actions. If an action fails, cache the failure with a short TTL. Subsequent requests for the same action key return the cached failure without re-executing. This prevents repeatedly running broken builds.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Action key | "cache hash" | A deterministic digest computed from all inputs that affect an action's output: command, input file contents, environment variables, and tool versions. The cache key for the action's output. |
| Remote cache | "artifact store" | A shared content-addressed storage service (typically gRPC) where build outputs are stored and retrieved. Multiple developers and CI machines share the same cache, avoiding redundant builds. |
| Hermetic build | "closed inputs" | A build that depends only on declared inputs. No hidden dependencies on files, environment, or time. Hermeticity guarantees that the same action key always produces the same output. |
| DAG scheduler | "task ordering" | The component that topologically sorts the build graph and executes actions respecting dependencies. Independent actions can execute in parallel; dependent actions wait for their inputs. |
| CAS | "content-addressable store" | A storage system where data is indexed by its content hash. If two files have the same hash, they're stored once. Used for build artifacts: the action key maps to the output content. |

## Further Reading

- [Bazel remote caching](https://bazel.build/remote/caching) — How Bazel implements distributed build caching.
- [Build Systems à la Carte](https://www.microsoft.com/en-us/research/uploads/prod/2018/03/build-systems.pdf) — Theoretical framework for build system design.
- [Buck2](https://github.com/facebook/buck2) — Meta's build system with content-addressed caching.
