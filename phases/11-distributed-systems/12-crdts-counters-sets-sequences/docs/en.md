# CRDTs — Counters, Sets, Sequences

> Merge without conflicts — because the data type itself guarantees convergence.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 11 lessons 01–11
**Time:** ~90 minutes

## Learning Objectives

- Explain why merging arbitrary data types in eventually consistent systems is hard, and how CRDTs eliminate the merge problem by construction.
- Distinguish CvRDTs (state-based) from CmRDTs (operation-based) and explain why the merge function must be commutative, associative, and idempotent.
- Implement G-Counter, PN-Counter, G-Set, OR-Set, and LWW-Register from scratch in Rust.
- Demonstrate that three replicas with concurrent updates converge to the same state after pairwise merges — without coordination.
- Identify the trade-offs of each CRDT variant (grow-only vs. add-remove, add-wins semantics, timestamp-based conflicts).

## The Problem

Three replicas store a shared counter. Replica A increments to 5. Replica B increments to 3. They've been operating independently — no coordination. Now they need to merge.

With a regular integer, what should the merged value be? 5? 3? 8? The "right" answer is 8 (both increments should count), but a plain integer can't tell you that. It only stores the final value, not the *history* of how it got there.

The same problem hits sets. Replica A adds `"alice"` to the set. Replica B removes `"alice"`. They merge. Is `"alice"` in the set or not? Without knowing the order of operations, you can't tell. You could say "add wins" or "remove wins," but without causal information, both answers are arbitrary.

This is the **merge problem**: in eventually consistent systems, replicas diverge, and merging arbitrary data types requires solving conflicts. The insight behind CRDTs is: what if the data type itself guarantees convergent merging? Then you never *have* a conflict to solve.

## The Concept

### CRDT Definition

A **Conflict-free Replicated Data Type** (CRDT) is a data structure designed to be replicated across multiple nodes, updated independently without coordination, and always merged in a consistent way. "Consistent" here means: regardless of the order in which merges happen, all replicas eventually converge to the same state.

The trick is encoding enough information in the data structure that merge becomes deterministic. A G-Counter doesn't store "5" — it stores `[A:5, B:3, C:0]`, a vector of per-replica increments. Merge takes the element-wise max, which always produces the same result regardless of order.

### Two Flavors: CvRDT and CmRDT

| Property | CvRDT (state-based) | CmRDT (operation-based) |
|----------|--------------------|--------------------------|
| What you send | Full state | Individual operations |
| Merge | `merge(a, b)` — a function on states | Apply each operation in causal order |
| Requirements | Merge is commutative, associative, idempotent | Operations are commutative, associative; delivery is exactly-once in causal order |
| Bandwidth | Higher (full state) | Lower (just ops) |
| Robustness | Tolerates duplicate and out-of-order delivery | Requires reliable causal broadcast |
| Simplicity | Easier to implement | More efficient, harder to guarantee delivery |

**CvRDT** (Convergent Replicated Data Type): you periodically gossip the entire state. The merge function takes two states and produces one. Because merge is commutative (`merge(a,b) = merge(b,a)`), associative (`merge(merge(a,b),c) = merge(a,merge(b,c))`), and idempotent (`merge(a,a) = a`), it doesn't matter what order merges arrive in or if they're duplicated.

**CmRDT** (Commutative Replicated Data Type): you broadcast operations. Each operation is designed so that applying them in any order gives the same result. This requires causal broadcast — if operations aren't delivered in causal order, you can break convergence.

We'll implement CvRDTs. They're simpler, more robust, and the ones you'll encounter most often in practice.

### G-Counter: Grow-Only Counter

A **G-Counter** can only increment. Each replica `i` maintains its own counter `c[i]`. To read the total, sum all entries. To merge, take the element-wise maximum.

```
Replica A: [5, 0, 0]    (A incremented 5 times)
Replica B: [0, 3, 0]    (B incremented 3 times)

merge(A, B) = [5, 3, 0]    (element-wise max)

Total = 5 + 3 + 0 = 8
```

Properties:
- **Commutative**: max(max(a,b), c) = max(a, max(b,c)) ✓
- **Associative**: max(a, max(b,c)) = max(max(a,b), c) ✓
- **Idempotent**: max(a, a) = a ✓
- **Cannot decrement**: this is a fundamental limitation. To support decrement, you need a PN-Counter.

### PN-Counter: Increment and Decrement

A **PN-Counter** is two G-Counters: one for increments (P) and one for decrements (N). Value = P - N.

```
Replica A increments 5 times, decrements 2 times:
  P = [5, 0, 0],  N = [2, 0, 0]  →  value = 5 - 2 = 3

Replica B increments 3 times:
  P = [0, 3, 0],  N = [0, 0, 0]  →  value = 3 - 0 = 3

Merge P: [5, 3, 0]   Merge N: [2, 0, 0]
Value = (5+3+0) - (2+0+0) = 8 - 2 = 6
```

### G-Set: Grow-Only Set

A **G-Set** is the simplest CRDT. Add elements. Merge = set union. Contains = membership test.

```
Replica A: {"alice", "bob"}
Replica B: {"bob", "carol"}

merge(A, B) = {"alice", "bob", "carol"}
```

The catch: **you cannot remove elements**. Once added, an element is in the set forever. Union is commutative, associative, and idempotent, so convergence is guaranteed — but you can't build a "remove" on top.

### OR-Set: Observed-Remove Set

An **OR-Set** supports both add and remove. The trick: each add operation tags the element with a unique identifier `(node_id, sequence_number)`. Remove only removes the tags it has *seen*.

```
Replica A: add("x") with tag (A,1)  →  {(x, (A,1))}
Replica B: add("x") with tag (B,1)  →  {(x, (B,1))}

After merge:  {(x, (A,1)), (x, (B,1))}  — "x" has two tags, both alive

Now Replica A removes "x":
  It can only remove tags it has seen: (A,1)
  Remaining: {(x, (B,1))}  — "x" is STILL in the set because tag (B,1) is alive

Concurrent add wins over remove. This is the "add-wins" semantics.
```

The OR-Set maintains two collections:
- `adds`: set of (element, tag) pairs
- `tombstones`: set of tags that have been removed

An element is present if it has at least one tag in `adds` that is not in `tombstones`.

Merge rule: union of adds, union of tombstones. Present elements = adds minus tombstoned tags.

### LWW-Register: Last-Writer-Wins Register

A **LWW-Register** stores a single value with a timestamp. On conflict, the value with the higher timestamp wins. Ties are broken by a deterministic rule (e.g., node_id comparison).

```
Replica A: value = "blue",  timestamp = 3, node = A
Replica B: value = "red",   timestamp = 5, node = B

Merge: "red" wins (timestamp 5 > 3)
```

Simple, but **data loss**: if two replicas concurrently write different values, one write is silently discarded. The "later" one wins, even if both were based on the same prior state.

### LWW-Element-Set

A **LWW-Element-Set** combines LWW with sets. For each element, track the timestamp of the add and the timestamp of the remove. An element is in the set if `add_timestamp > remove_timestamp`.

```
add_set    = {(alice, t=5), (bob, t=3)}
remove_set = {(alice, t=4)}

Is alice in the set?  add_t=5 > remove_t=4  → yes
Is bob in the set?    remove_t for bob = undefined → treat as 0  → add_t=3 > 0  → yes
```

### Sequence CRDTs (Brief Mention)

Counters, sets, and registers are "simple" CRDTs. **Sequence CRDTs** handle ordered sequences — think collaborative text editing. Two replicas concurrently insert characters at different positions; both insertions must be preserved on merge.

The key insight: assign each position a fractional index between two existing positions (like fractional indexing in Figma), or use a unique identifier that encodes enough ordering information. Algorithms like **RGA (Replicated Growing Array)** and **Treedoc** implement this, but they're complex enough for their own lesson. For now, know they exist and that they solve the "concurrent insert at the same position" problem.

## Build It

We'll implement all five CRDTs in Rust: G-Counter, PN-Counter, G-Set, OR-Set, and LWW-Register. Then we'll wrap them in a `Replica` struct and demonstrate convergence across three replicas with concurrent updates.

### Step 1: G-Counter and PN-Counter

```rust
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
struct GCounter {
    counts: HashMap<String, u64>,
}

impl GCounter {
    fn new() -> Self {
        GCounter { counts: HashMap::new() }
    }

    fn increment(&mut self, node: &str, delta: u64) {
        *self.counts.entry(node.to_string()).or_insert(0) += delta;
    }

    fn value(&self) -> u64 {
        self.counts.values().sum()
    }

    fn merge(&self, other: &Self) -> Self {
        let mut result = self.counts.clone();
        for (node, count) in &other.counts {
            let entry = result.entry(node.clone()).or_insert(0);
            *entry = (*entry).max(*count);
        }
        GCounter { counts: result }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PNCounter {
    p: GCounter,
    n: GCounter,
}

impl PNCounter {
    fn new() -> Self {
        PNCounter { p: GCounter::new(), n: GCounter::new() }
    }

    fn increment(&mut self, node: &str, delta: u64) {
        self.p.increment(node, delta);
    }

    fn decrement(&mut self, node: &str, delta: u64) {
        self.n.increment(node, delta);
    }

    fn value(&self) -> i64 {
        self.p.value() as i64 - self.n.value() as i64
    }

    fn merge(&self, other: &Self) -> Self {
        PNCounter {
            p: self.p.merge(&other.p),
            n: self.n.merge(&other.n),
        }
    }
}
```

### Step 2: G-Set

```rust
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
struct GSet<T: Clone + Eq + std::hash::Hash> {
    elements: HashSet<T>,
}

impl<T: Clone + Eq + std::hash::Hash> GSet<T> {
    fn new() -> Self {
        GSet { elements: HashSet::new() }
    }

    fn add(&mut self, element: T) {
        self.elements.insert(element);
    }

    fn contains(&self, element: &T) -> bool {
        self.elements.contains(element)
    }

    fn merge(&self, other: &Self) -> Self {
        GSet {
            elements: self.elements.union(&other.elements).cloned().collect(),
        }
    }
}
```

### Step 3: OR-Set

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Tag(String, u64);

#[derive(Clone, Debug)]
struct ORSet<T: Clone + Eq + std::hash::Hash + std::fmt::Debug> {
    adds: HashSet<(T, Tag)>,
    tombstones: HashSet<Tag>,
    seq: u64,
    node: String,
}

impl<T: Clone + Eq + std::hash::Hash + std::fmt::Debug> ORSet<T> {
    fn new(node: &str) -> Self {
        ORSet {
            adds: HashSet::new(),
            tombstones: HashSet::new(),
            seq: 0,
            node: node.to_string(),
        }
    }

    fn add(&mut self, element: T) {
        let tag = Tag(self.node.clone(), self.seq);
        self.seq += 1;
        self.adds.insert((element, tag));
    }

    fn remove(&mut self, element: &T) {
        let to_remove: Vec<Tag> = self.adds
            .iter()
            .filter(|(e, _)| e == element)
            .map(|(_, t)| t.clone())
            .collect();
        for tag in to_remove {
            self.tombstones.insert(tag);
        }
    }

    fn contains(&self, element: &T) -> bool {
        self.adds.iter().any(|(e, t)| e == element && !self.tombstones.contains(t))
    }

    fn merge(&self, other: &Self) -> Self {
        let mut result = self.clone();
        for (element, tag) in &other.adds {
            result.adds.insert((element.clone(), tag.clone()));
            if other.tombstones.contains(tag) {
                result.tombstones.insert(tag.clone());
            }
        }
        for tag in &other.tombstones {
            result.tombstones.insert(tag.clone());
        }
        result.seq = result.seq.max(other.seq);
        result
    }
}
```

### Step 4: LWW-Register

```rust
#[derive(Clone, Debug, PartialEq)]
struct LWWRegister<T: Clone + std::fmt::Debug> {
    value: T,
    timestamp: u64,
    node: String,
}

impl<T: Clone + std::fmt::Debug> LWWRegister<T> {
    fn new(value: T, timestamp: u64, node: &str) -> Self {
        LWWRegister {
            value,
            timestamp,
            node: node.to_string(),
        }
    }

    fn set(&mut self, value: T, timestamp: u64) {
        if timestamp >= self.timestamp {
            self.value = value;
            self.timestamp = timestamp;
        }
    }

    fn merge(&self, other: &Self) -> Self {
        if other.timestamp > self.timestamp
            || (other.timestamp == self.timestamp && other.node > self.node)
        {
            other.clone()
        } else {
            self.clone()
        }
    }
}
```

### Step 5: Replica and Convergence Demo

The `Replica` struct owns all CRDTs. It can merge state from another replica. The demo shows three replicas making concurrent updates, then merging pairwise, and converging to the same state.

See `code/main.rs` for the full implementation and demo.

## Use It

**Riak** (Basho's distributed database) ships built-in CRDT datatypes — counters, sets, maps, and registers — called "Riak Data Types." They use the same CvRDT approach we've implemented: counters are PN-Counters, sets are OR-Sets, registers are LWW-Registers. The key difference from our implementation: Riak's CRDTs are backed by a deterministic pseudo-random tag generator instead of a simple sequence number, and they support garbage collection of tombstones through dotted version vectors.

**Redis** exposes approximate counters and sets, but these are not CRDTs — they require single-leader一致性. If you need CRDT semantics in Redis, you need CRDT Redis (a research prototype by the SyncFree project).

**Automerge** and **Yjs** are CRDT libraries for collaborative editing. They implement sequence CRDTs (RGA-like) on top of the same principles we've built here. The key addition: they assign unique identifiers to each character in a text sequence, enabling concurrent inserts to merge deterministically.

Compare our G-Counter to Riak's: Riak stores the counter vector in a special `riak_dt_map` that can hold nested CRDTs (a map of counters, sets, etc.). Our G-Counter is a flat `HashMap<String, u64>`. The merge semantics are identical — element-wise max — but Riak's version handles schema evolution and type checking.

## Read the Source

- [Riak `riak_dt_pncounter.erl`](https://github.com/basho/riak_kv/blob/develop/src/riak_dt_pncounter.erl) — Production PN-Counter implementation. Look at `merge/2` to see element-wise max on two G-Counters, and `value/1` to see P - N.
- [Automerge `counter.ts`](https://github.com/automerge/automerge/blob/main/rust/automerge/src/counter.rs) — A CRDT counter in Rust used by Automerge. Compare to our G-Counter: Automerge uses a single increment/decrement representation rather than per-node vectors.

## Ship It

The reusable artifact for this lesson is in `outputs/`:

- **A self-contained CRDT library** — G-Counter, PN-Counter, G-Set, OR-Set, and LWW-Register implementations you can reuse in later phases for distributed data structures.

## Exercises

1. **Easy** — Implement a G-Counter from scratch without looking at the lesson code. Verify that merging `[A:3, B:0]` with `[A:0, B:5]` yields `[A:3, B:5]` = 8.
2. **Medium** — Implement an OR-Set where concurrent remove wins (instead of concurrent add wins). Show that this changes the semantics: if replica A adds "x" and replica B removes "x" concurrently, "x" is absent. Discuss why add-wins is the more common choice in practice.
3. **Hard** — Implement a 2P-Set (Two-Phase Set): a G-Set for additions and a G-Set for removals (tombstones). An element is in the set if it's in the add set but not in the remove set. Show that once an element is removed, it can never be added again (the "unre-addable" problem). Then implement OR-Set as the fix.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| CRDT | "A mergeable data structure" | A data type whose operations are designed so that any number of merges in any order converge to the same state — no coordination needed |
| CvRDT | "State-based CRDT" | Convergent Replicated Data Type — you gossip full state; merge function is commutative, associative, idempotent |
| CmRDT | "Operation-based CRDT" | Commutative Replicated Data Type — you broadcast operations; they must be commutative and associative, delivered in causal order exactly once |
| G-Counter | "A distributed counter" | Grow-only counter: each node owns its slot, merge = element-wise max, value = sum of all slots. Cannot decrement. |
| PN-Counter | "An increment-decrement counter" | Two G-Counters (P for positive, N for negative). Value = P - N. Supports both directions but uses twice the space. |
| OR-Set | "A set you can add and remove" | Observed-Remove Set: each add gets a unique tag; remove only kills observed tags. Concurrent add wins over remove. |
| LWW-Register | "The latest write wins" | A register where the value with the highest timestamp (plus tiebreaker) wins on merge. Simple but silently drops concurrent writes. |
| Idempotent merge | "You can merge twice safely" | merge(a, a) = a — receiving the same state update twice has no effect. Essential for CvRDTs in unreliable networks. |

## Further Reading

- [A Comprehensive Study of Convergent and Commutative Replicated Data Types](https://hal.inria.fr/inria-00555588/document) — Marc Shapiro et al., the definitive CRDT survey paper. Defines CvRDT and CmRDT formally, catalogs all known CRDTs.
- [CRDTs: The Hard Parts](https://www.youtube.com/watch?v=qb4PnRWnpRg) — Martin Kleppmann's talk on why CRDTs are harder to get right than they look, covering edge cases in OR-Set and sequence CRDTs.
- [Automerge: A New CRDT for Building Collaborative Applications](https://arxiv.org/abs/2307.05453) — The paper behind Automerge, showing how CRDTs enable local-first software.
- [Riak Data Types](https://docs.riak.com/riak/kv/2.2.3/using/reference/crdts/index.html) — Production CRDT documentation: counters, sets, maps, and flags as first-class distributed datatypes.