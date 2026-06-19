# Skip Lists

> A randomized linked-list-of-linked-lists. Expected O(log n) for search/insert/delete with no rotations. Redis SortedSet's underlying structure.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L02 (linked lists), L11 (treap intuition for randomization)
**Time:** ~60 minutes

## Learning Objectives

- Implement a **skip list**: layered linked lists where each higher layer skips ~2× elements.
- Use **randomized levels**: each new node tossed up coin-flip-many times.
- Achieve expected O(log n) per op without rotations, with the simple code of linked lists.
- Recognize where used: Redis SortedSet/SortedDict, LevelDB's memtable, ConcurrentSkipListMap.

## The Problem

We've seen balanced BSTs (AVL, RB) and randomized BSTs (treaps) that achieve O(log n) per op. Skip lists are a different approach: randomized linked lists.

The pitch: same asymptotic guarantees, no rotations, no balance invariant to maintain. Insert and delete are just standard linked-list pointer updates at each level. Code is shorter and arguably easier to verify than RB tree's 6-case delete.

The price: randomized (small probability of bad cases) and worse constant-factor cache behavior than B-trees.

## The Concept

### Structure

Layer 0 is a doubly-linked list of all n elements in sorted order. Layer 1 is a linked list of ~n/2 elements (every node from layer 0 that "flipped heads"). Layer 2 is ~n/4. And so on, up to layer log₂ n.

```
Level 3:                      [HEAD] ────────────────────────── [NIL]
Level 2:                      [HEAD] ─── 17 ───────── 42 ────── [NIL]
Level 1:                      [HEAD] ─── 17 ─── 31 ── 42 ── 53 ─ [NIL]
Level 0: [HEAD] 5 ── 11 ── 17 ─ 23 ─ 31 ─ 38 ─ 42 ─ 47 ─ 53 ─── [NIL]
```

### Search

Start at the top-left. Walk right while the next key ≤ target. When you can't go right (next key > target or NIL), drop down a level. Repeat. Total moves: expected O(log n).

```c
Node *search(SkipList *s, int key) {
    Node *cur = s->head;
    for (int lvl = s->max_level; lvl >= 0; --lvl) {
        while (cur->next[lvl] && cur->next[lvl]->key < key) cur = cur->next[lvl];
    }
    cur = cur->next[0];
    return (cur && cur->key == key) ? cur : NULL;
}
```

### Insert

1. Find the position (search-like walk, remembering predecessors at each level in `update[]`).
2. Toss a coin: flip until tails to decide the new node's level. Pr[level ≥ k] = 1/2^k.
3. Allocate a node with that level; rewire pointers at each affected level using `update[]`.

```c
int random_level(void) {
    int lvl = 1;
    while (rand() & 1) lvl++;     /* geometric distribution */
    return lvl;
}
```

### Delete

Like insert but in reverse: find, then unlink at each level.

### Why expected O(log n)

A node is in level k with probability 1/2^k. Expected number of nodes in level k: n/2^k. So expected max level is ~log₂ n. Searches walk one node per level (expected) → O(log n) expected.

The bad case: extreme luck (very tall tower of randomness) — possible but exponentially rare.

### vs balanced BST

| | Skip list | Red-black tree |
|---|-----------|----------------|
| Expected ops | O(log n) | O(log n) |
| Worst-case | O(n) (vanishingly rare) | O(log n) |
| Code length | ~50 lines | ~300 lines |
| Concurrent | Easy (lock-free variants) | Hard (rotations conflict) |
| Cache | Bad (pointer chase) | Worse (pointer chase + rotations) |
| Memory | ~1.5n pointers | 2-3n pointers + color |

The killer feature of skip lists: **concurrent-friendly**. Pugh's original paper noted that locking small spans of the list is easy; lock-free skip-list versions are simpler than lock-free trees. ConcurrentSkipListMap is the standard "ordered concurrent map" in Java.

## Build It

`code/main.c`:

1. Skip list with insert, search, delete.
2. Verify search returns correct results on 10K random insert/delete.
3. Measure observed expected levels vs predicted log₂ n.

`code/main.py` mirrors with cleaner code.

`code/main.rs` simplified for safety.

### Run

```sh
clang -O2 -fsanitize=address main.c -o sl && ./sl
```

## Use It

- **Redis SortedSet** (`ZADD`/`ZRANGE`): skip list + hash table dual. The skip list provides ordered iteration; the hash table provides O(1) member-existence.
- **LevelDB / RocksDB memtable**: skip list as in-memory sorted buffer before flushing to SSTable.
- **Java `ConcurrentSkipListMap`**: lock-free ordered map.
- **`std::list` is NOT a skip list** — common confusion. C++ `std::list` is a doubly-linked list.

## Read the Source

- *Pugh's original 1990 paper* — Skip Lists: A Probabilistic Alternative to Balanced Trees.
- [Redis `t_zset.c`](https://github.com/redis/redis/blob/unstable/src/t_zset.c) — production skip list with span field for ranking.
- [LevelDB `memtable.cc`](https://github.com/google/leveldb/blob/main/db/memtable.cc) — lock-free skip list using atomic pointers.

## Ship It

This lesson ships **`outputs/skiplist.h`** — single-header skip list.

## Exercises

1. **Easy.** Print the levels of a skip list after 1000 random inserts. Verify max level ≈ log₂ 1000 ≈ 10.
2. **Medium.** Add a `span` field per pointer: number of elements at level 0 spanned by that link. Then `rank(key)` answers "what's the index of key" in O(log n). Used by Redis.
3. **Hard.** Implement a lock-free skip list using `atomic_compare_exchange_strong` for each level's next pointer. Test under 4 threads inserting 100K elements each; verify all elements present and order preserved.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Skip list | "Probabilistic balanced list" | Multi-level linked list with random heights |
| Coin flip | "Geometric height" | Each new node's level = number of consecutive heads + 1 |
| Span | "Distance" | Number of bottom-level nodes a link covers; enables O(log n) rank |
| Lock-free | "CAS-based" | Multiple threads update via atomic compare-exchange |
| Expected vs worst-case | "Randomized analysis" | O(log n) with high probability; rare bad cases possible |

## Further Reading

- Pugh 1990 — *Skip Lists: A Probabilistic Alternative to Balanced Trees*.
- *Concurrent Skip Lists* (Herlihy & Lea) — lock-free designs.
- [Redis: 'Why I write the new redis-cluster-proxy'](http://antirez.com/news/132) — antirez on choosing skip lists.
