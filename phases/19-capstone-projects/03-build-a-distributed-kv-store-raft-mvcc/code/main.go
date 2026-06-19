// Build a Distributed KV Store with Raft + MVCC
// Run: go run main.go
//
// Architecture:
//   Client → Raft (replicated log) → Apply (state machine) → MVCC (versioned storage)
//
// This implements a multi-version key-value store with a simulated replicated log,
// demonstrating snapshot reads at any historical version.

package main

import (
	"fmt"
	"sort"
	"sync"
)

// =============================================================================
// Step 1: MVCC Storage — multi-version key-value store
// =============================================================================

// Version represents one version of a key's value
type Version struct {
	Value     string
	CreatedAt uint64 // Global version when this was written
	Deleted   bool   // Tombstone for deletes
}

// MVCCStore is a multi-version key-value store
type MVCCStore struct {
	mu            sync.RWMutex
	versions      map[string][]Version // key -> sorted version chain
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
		return "", false
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

// =============================================================================
// Step 2: Command Log / Raft Simulation
// =============================================================================

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
	Value string
}

// ReplicatedLog simulates Raft's replicated log
type ReplicatedLog struct {
	mu        sync.Mutex
	entries   []Command
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

// =============================================================================
// Step 3: Main — wire everything together
// =============================================================================

func main() {
	store := NewMVCCStore()
	log := NewReplicatedLog()

	// Simulate: client sends PUT commands through Raft
	commands := []Command{
		{CmdPut, "name", "Alice"},
		{CmdPut, "age", "30"},
		{CmdPut, "name", "Bob"}, // Overwrite
		{CmdDelete, "age", ""},
		{CmdPut, "city", "Portland"},
	}

	// Phase 1: Append all commands to the log
	for _, cmd := range commands {
		idx := log.Append(cmd)
		log.Commit(idx)
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
