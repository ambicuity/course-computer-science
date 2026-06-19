# Hash Tables — Open Addressing vs Chaining

> The dict/map/HashMap you use every day. Two implementation families, three probing schemes, one constant tension: load factor.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L01–04, P01 L16 (modular arithmetic)
**Time:** ~90 minutes

## Learning Objectives

- Implement a hash table with both **chaining** (bucket → linked list) and **open addressing** (linear probing, Robin Hood).
- Define and tune the **load factor** α = n / m; explain why α > 0.7 destroys open-addressing performance and why α > 1 is fine for chaining.
- Resize at a chosen threshold; analyze why resize is amortized O(1) per insert.
- Identify the canonical hazards: cluster formation in linear probing, tombstones on delete, hash flooding by adversarial keys.

## The Problem

A dictionary maps keys to values. Many data structures support this — sorted arrays, balanced BSTs, tries — but only the hash table gives expected O(1) lookup and insert under reasonable assumptions, with a memory cost proportional to n.

The cost: it's all about the load factor, the probe sequence, the hash function, and the resize policy. Get one wrong and your "O(1)" becomes O(n) under load.

This lesson builds both major variants — chaining (Java's HashMap, Python ≤ 3.5) and open addressing (Python 3.6+, Rust's HashMap, Go's map). You'll feel where each one breaks.

## The Concept

### Chaining

Each slot in the table holds a linked list (or vector) of (key, value) pairs that hash to that bucket. Insert appends; lookup walks the list.

- **Load factor α = n/m** can be > 1; each bucket holds ⌈α⌉ entries on average.
- **Lookup**: O(1 + α) expected.
- **Resize** at α > 0.75 (typical): double cap, rehash all entries.
- **Strengths**: simple, deletion is cheap, high α tolerable.
- **Weaknesses**: cache locality is bad (chain nodes scattered), per-entry pointer overhead.

### Open addressing

Entries live directly in the array; collisions probe forward.

#### Linear probing

```c
i = hash(key) % cap
while (occupied[i] && keys[i] != key) i = (i + 1) % cap
```

- α capped at ~0.7 — past that, **primary clustering** ruins performance.
- Expected probe length: 1 / (1−α) for found, 1/(1−α)² for not-found.
- At α=0.9: 10 probes; at α=0.99: ~100.
- **Strength**: contiguous, cache-friendly.

#### Quadratic probing

```c
i = (hash(key) + k²) % cap
```

Avoids primary clustering, still has *secondary* clustering. Used in C++ stdlib `unordered_map` historically.

#### Double hashing

```c
i = (h1(key) + k * h2(key)) % cap
```

Best in theory — independent step size per key. Used in BSD `db`.

#### Robin Hood hashing

On insert, if the existing entry has a *shorter* probe distance than the incoming entry, **swap them** — the rich entry gives up its slot to the poor one. Empirically caps the maximum probe length at O(log n) with high probability, even at α=0.9. Used in Rust's `HashMap` pre-2018 and in `hashbrown`.

### Deletion in open addressing

Naive zero-delete breaks probe chains. Two fixes:

1. **Tombstones**: mark slot as DELETED. Lookups continue past tombstones; inserts can reuse them. Resize occasionally to clear accumulated tombstones.
2. **Back-shift** (Robin Hood-friendly): on delete, walk forward and shift entries back into the hole until you hit an empty slot or an entry with probe distance 0.

### Resize

When α exceeds the threshold, allocate a new table with 2× cap, rehash everything. Amortized O(1) per insert (banker's argument).

### Load factor by scheme

| Variant | Typical threshold | Why |
|---------|-------------------|-----|
| Chaining | 0.75 — 1.0 | Buckets can hold multiple entries |
| Linear probing | 0.5 — 0.7 | Probe length explodes near 1.0 |
| Quadratic probing | 0.5 — 0.7 | Same; small constant improvement |
| Robin Hood | 0.85 — 0.9 | Bounded probe length even at high α |

### Hash functions matter

A bad hash function creates pathological clusters. Production maps use:

- **Strong hashes**: Wyhash, SipHash, FxHash, AES-NI.
- **Hash randomization** (per-table seed): blocks **hash flooding** attacks where attackers send all-colliding keys to force O(N²) lookups. CVE-2011-4815.

## Build It

`code/main.c` implements:

1. **HashChain**: chaining with linked lists. Resize at α > 1.
2. **HashOpen**: open addressing with linear probing + tombstones.
3. **HashRH**: Robin Hood probing (no tombstones, back-shift delete).

The benchmark inserts 100K random uint64 keys and 100K lookups; reports avg/max probe length and ns/op for each.

`code/main.py` shows Python's built-in `dict` against a hand-rolled chaining dict.

`code/main.rs` uses `std::collections::HashMap` (SipHash) and a hand-rolled Robin Hood map.

### Run

```sh
clang -O2 main.c -o ht && ./ht
python3 main.py
```

## Use It

- **Python `dict`**: open addressing + perturbation (Objects/dictobject.c).
- **Rust `std::collections::HashMap`**: SwissTable + SipHash.
- **`hashbrown::HashMap`**: SwissTable + FxHash. Faster, not DOS-resistant.
- **Go `map`**: open addressing with chained overflow buckets; AES-NI on x86.
- **Java `HashMap`**: chaining; chains convert to red-black trees when long (since Java 8).

## Read the Source

- [Python `Objects/dictobject.c`](https://github.com/python/cpython/blob/main/Objects/dictobject.c)
- [Rust hashbrown](https://github.com/rust-lang/hashbrown) — SwissTable in Rust.
- [Go runtime/map.go](https://github.com/golang/go/blob/master/src/runtime/map.go)

## Ship It

This lesson ships **`outputs/hashmap.h`** — a single-header open-addressing hash map for uint64 → int, with Robin Hood probing and back-shift delete.

## Exercises

1. **Easy.** Add `contains_key`, `get`, `remove` to the chaining table.
2. **Medium.** Implement `Counter` (key → count) on top of your table. Find the most common word in a large text.
3. **Hard.** Implement an **open-addressing table that resizes without halting** ("incremental rehashing", à la Redis). Bound the per-insert latency, not the amortized.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Load factor (α) | "Fullness" | n entries / m slots |
| Chaining | "Bucket list" | Each slot is a linked list of colliders |
| Open addressing | "Probing" | Collisions probe forward through the array |
| Primary clustering | "Run formation" | Linear probing builds runs of occupied slots |
| Robin Hood | "Probe displacement balancing" | Swap entries on insert to equalize probe distances |
| Tombstone | "Deletion marker" | "Was occupied, lookups must continue past" |
| Hash flooding | "DOS attack" | Adversary crafts keys that all hash to one bucket |

## Further Reading

- *Introduction to Algorithms* (CLRS) Ch. 11.
- [SwissTable design (abseil)](https://abseil.io/about/design/swisstables).
- [Cliff Click: A Lock-Free Hash Table](https://www.youtube.com/watch?v=HJ-719EGIts).
