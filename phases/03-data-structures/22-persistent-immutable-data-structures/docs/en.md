# Persistent / Immutable Data Structures

> Update without destroying. Every version of the data lives forever. Path-copying gives O(log n) update + structural sharing. The shape of Clojure's collections, Git's history, and editor undo.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L02 (lists), L07 (tree recursion), L08 (BSTs)
**Time:** ~75 minutes

## Learning Objectives

- Implement a **persistent linked list** via cons cells with structural sharing.
- Implement a **persistent BST** using path-copying — O(log n) update, O(log n) space per version.
- Recognize the **HAMT** (hash array-mapped trie) — Clojure's PersistentMap, Scala's Vector.
- Understand the trade-off: every update is non-destructive but costs O(log n) memory.

## The Problem

Standard data structures are **ephemeral**: updates destroy the previous version. You can't ask "what did the map look like 10 operations ago?" without snapshotting (copy the whole thing — O(n)).

Persistent data structures preserve every version. After `v2 = v1.insert(k)`, both v1 (unchanged) and v2 (with k) coexist. Used in:

- **Functional languages** (Haskell, Clojure, Elixir): data is immutable by default.
- **Editors**: undo/redo as a tree of document states.
- **Git**: trees and commits are persistent — each commit references its parent.
- **Concurrent reads**: readers see a stable snapshot without locking.

The naïve approach (copy on every write) is O(n) per update — unusable. Persistent structures achieve **O(log n) update with structural sharing**.

## The Concept

### Persistent linked list (already exists!)

A cons cell `(head, tail)` is naturally persistent: `cons(x, list)` returns a new list whose tail is the old list. No copying needed; the old list still exists and is unchanged.

```c
typedef struct Node { int data; const struct Node *next; } Node;

const Node *cons(int x, const Node *t) {
    Node *n = malloc(sizeof(Node));
    n->data = x; n->next = t;
    return n;
}
```

Constants: list, push_front, pop_front. All operations O(1). Both old and new lists coexist with the older nodes shared.

### Path-copying BST

For a balanced BST: an update at position p modifies O(log n) nodes on the path from root to p. Copy those; leave the rest of the tree shared with the previous version.

```
Original v1:        After v2 = v1.insert(35):
        20                     20'        ← new node (root of v2)
       /  \                   /  \
     10    30               10    30'    ← new node (path)
          /  \                   /  \
         25  40                 25  40'  ← new node (path)
                                     /
                                    35   ← new
```

Nodes that aren't on the modified path (10, 25, the 30's-subtree minus the 40 branch) are **shared** between v1 and v2.

Each version v_i has O(log n) NEW nodes; total memory after k updates: O(k log n). Often much less than k full copies.

### HAMT (Hash Array-Mapped Trie)

For a persistent hash map, use a trie over the hash bits. Each level branches on 5 bits of hash → 32-way fan-out. Tree depth: ⌈log₃₂ n⌉ ≈ 5 for n=1M. Path-copy 5 nodes per update.

Bagwell's 2000 paper introduced HAMTs. They're the backbone of:

- Clojure `PersistentHashMap`
- Scala `HashMap`
- Haskell `Data.HashMap.Strict` (unordered-containers)
- Rust `im` crate

Each node uses a 32-bit "bitmap" to indicate which children exist (saves memory: most nodes are sparse).

### Performance vs ephemeral

For a HashMap or BTreeMap, ephemeral is ~5× faster than persistent. The wins of persistence:

1. Cheap snapshots (O(1) — just hold the root).
2. No copy needed for "save current state".
3. Lock-free readers see consistent snapshots.

If your workload is mostly reads with occasional snapshots (editor undo, version control, immutable data warehousing), persistent is right. For hot-loop updates, ephemeral wins.

### Path-copying in production

- **Git** uses path-copying conceptually: each commit's tree shares unchanged blobs with parent commits.
- **CRDTs** (Conflict-free Replicated Data Types) often use persistent maps internally for state.
- **Functional databases** (Datomic) store all versions of all data via path-copying.
- **MVCC databases** (Postgres, MySQL InnoDB) use a different (row-version) approach but the same principle: never destroy.

## Build It

`code/main.c`:

1. Persistent singly-linked list with cons.
2. Path-copying persistent BST: insert returns a new root sharing most of the old tree.
3. Demonstrate: build v1, v2, v3 — show all three coexist with shared structure.

`code/main.py` uses tuples for immutability.

`code/main.rs` uses `Arc<Node>` for shared ownership.

### Run

```sh
clang -O2 -fsanitize=address main.c -o pds && ./pds
```

## Use It

- **Clojure**: every collection (map, set, vector, list) is persistent.
- **Elm, F#, Haskell, Erlang/Elixir**: immutable by default.
- **Rust `im` crate**: production persistent data structures.
- **Scala `Map`, `Set`, `Vector`**: HAMT-based.
- **Redux (JavaScript)**: immutable state tree, conceptually persistent.

## Read the Source

- *Okasaki, Purely Functional Data Structures* (1998) — the canonical reference.
- *Bagwell, Ideal Hash Trees* (2000) — HAMT paper.
- [Rust `im` crate](https://github.com/aclysma/rs_imgui_demo) — production persistent structures in Rust.
- [Clojure source `clojure.lang.PersistentHashMap`](https://github.com/clojure/clojure/blob/master/src/jvm/clojure/lang/PersistentHashMap.java).

## Ship It

This lesson ships **`outputs/persistent_bst.h`** — path-copying persistent BST in C.

## Exercises

1. **Easy.** Build a list `[1, 2, 3]` as `v1`. Create `v2 = cons(0, v1)`. Verify both lists print correctly and share nodes.
2. **Medium.** Implement persistent BST delete via path-copying. The deleted node must be replaced (inorder successor) and the path copied; everything off-path remains shared.
3. **Hard.** Implement a HAMT (32-way trie). Each node uses a 32-bit bitmap to indicate which children exist. Use it as a persistent map.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Persistent | "Versioned" | Every update produces a new version; old versions remain valid |
| Structural sharing | "Shared subtree" | Two versions share nodes that didn't change |
| Path copying | "Copy on the path" | Updates allocate O(log n) new nodes along the modified path |
| HAMT | "Hash array-mapped trie" | 32-way persistent trie used for maps; ~O(1) ops |
| Ephemeral | "Standard mutable" | Updates destroy the previous version |

## Further Reading

- *Purely Functional Data Structures* by Okasaki (1998) — the textbook.
- *Ideal Hash Trees* by Bagwell (2000) — original HAMT paper.
- *Optimizing Hash-Array Mapped Tries for Fast and Lean Immutable JVM Collections* — modern improvements.
