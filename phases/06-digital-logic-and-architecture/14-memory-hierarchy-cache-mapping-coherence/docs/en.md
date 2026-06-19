# Lesson 14: Memory Hierarchy — Cache Mapping & Coherence

## The Memory Hierarchy

A CPU can execute an instruction every cycle, but DRAM takes 100–300 cycles to respond. Without bridging that gap, the processor would starve. The solution is a hierarchy of progressively larger, slower, cheaper storage layers:

| Level | Typical Size | Latency (cycles) | Managed By |
|-------|-------------|-------------------|------------|
| Registers | < 1 KB | 1 | Compiler / ISA |
| L1 Cache | 32–64 KB | 3–4 | Hardware |
| L2 Cache | 256 KB–1 MB | 10–20 | Hardware |
| L3 Cache | 4–64 MB | 30–50 | Hardware |
| DRAM | 8–128 GB | 100–300 | OS / Hardware |

The guiding principle: **smaller is faster and more expensive per byte**. Each level acts as a filter — the vast majority of accesses hit in L1; only misses cascade downward.

**Temporal locality**: if an address was accessed recently, it will likely be accessed again soon.
**Spatial locality**: if address *A* is accessed, nearby addresses will likely be accessed soon.

Caches exploit both. A cache *line* (or *block*) — typically 64 bytes — is the unit of transfer between levels. Loading one word pulls in its neighbors, exploiting spatial locality. Keeping the line around exploits temporal locality.

## Cache Structure and Address Decomposition

Every memory address is decomposed into three fields:

```
| tag  |  index  |  offset  |
```

- **Offset**: selects the byte within a cache line. For a 64-byte block, offset = 6 bits.
- **Index**: selects which *set* in the cache the address maps to.
- **Tag**: stored alongside the cached data; compared against the upper bits of the incoming address to verify a hit.

A **set** contains one or more **lines** (also called *ways*). The number of lines per set is the **associativity**.

### Example

A 32 KB L1 cache, 64-byte blocks, 8-way set-associative:

- Offset bits = log₂(64) = 6
- Number of sets = 32768 / (8 × 64) = 64 → index bits = 6
- Tag bits = 32 − 6 − 6 = 20

## Cache Mapping Strategies

### Direct-Mapped (1-way)

Each memory block maps to exactly one cache set (index = block_address mod num_sets). Simple and fast, but two frequently-accessed blocks that map to the same set will *thrash* — evicting each other on every access.

### Set-Associative (N-way)

Each set holds N lines. A block maps to a specific set but can occupy any of the N lines within it. 2-way and 4-way are common; 8-way is typical for L1 data caches. Higher associativity reduces conflict misses at the cost of more comparisons per lookup.

### Fully Associative

One set containing all lines. A block can go anywhere. Requires comparing the tag against every line in parallel (expensive in hardware). Used only for small structures like TLBs.

```
Associativity ↑  →  conflict misses ↓, comparison cost ↑, hit time ↑
```

## Replacement Policies

When a set is full and a new block must be loaded, one existing line must be evicted:

- **LRU (Least Recently Used)**: evict the line unused for the longest time. Optimal for temporal locality but expensive to track exactly at high associativity.
- **Pseudo-LRU**: approximate LRU using a tree of bits. Common in 4-way and 8-way caches.
- **Random**: pick a random line. Simple hardware, surprisingly effective (within a few percent of LRU on average).

## Write Policies

When the CPU writes to a cached address:

**Write-through**: every write updates both the cache and main memory immediately. Simple but generates heavy bus traffic.

**Write-back**: writes update only the cache; the line is marked *dirty*. The dirty line is written to memory only when evicted. Reduces bus traffic — the dominant policy in modern L1/L2 caches.

**Write-allocate**: on a write miss, fetch the block into cache, then write. Pairs naturally with write-back.

**No-write-allocate**: on a write miss, write directly to memory without loading the block. Pairs with write-through.

Modern processors: **write-back + write-allocate**.

## Cache Coherence and the MESI Protocol

In a multi-core system, each core has its own L1 (and possibly L2). If core 0 writes to address X, core 1's cached copy becomes stale. **Cache coherence** ensures all cores observe a consistent view of memory.

### MESI Protocol

Each cache line is in one of four states:

| State | Meaning |
|-------|---------|
| **Modified (M)** | Line is dirty; only copy in the system. Must be written back before eviction. |
| **Exclusive (E)** | Line is clean; only copy in the system. Can be silently upgraded to M on write. |
| **Shared (S)** | Line is clean; other caches may also hold it. |
| **Invalid (I)** | Line is empty or invalid. |

**State transitions** (simplified):

- **Read miss**: fetch from memory (or another cache). If no other cache has it → E. If another cache has it → S (other cache downgrades from E→S or M→S, writing back if M).
- **Write hit on S**: broadcast *BusRdX* (read-with-intent-to-modify). Other caches invalidate their copies → M.
- **Write hit on E**: silent upgrade to M (no bus transaction).
- **Write hit on M**: no transition needed; already exclusive and dirty.
- **Eviction of M**: write back to memory.
- **Eviction of E or S**: just discard (clean).

### Snooping vs Directory

- **Snooping-based**: every cache monitors (snoops) the bus. Simple, works well for small-scale shared-bus systems (2–8 cores). Used in most desktop/laptop CPUs.
- **Directory-based**: a centralized directory tracks which caches hold each line. Scales to many cores/nodes but adds latency. Used in large server and NUMA systems.

## Build It: Cache Simulator in C

The accompanying `main.c` implements a configurable cache simulator supporting direct-mapped and N-way set-associative modes with LRU replacement. It demonstrates how conflict misses in direct-mapped caches are eliminated by increasing associativity.

## Use It: Real-World Caches

Every modern CPU has at least:

- **L1I** (instruction) and **L1D** (data) — split Harvard-style, 32–64 KB each, 4–8 way, 3–4 cycle hit.
- **L2** — unified (instructions + data), 256 KB–1 MB, 8–16 way, 10–20 cycle hit.
- **L3** — shared across cores, 2–64 MB, 16+ way, 30–50 cycle hit. Acts as the coherence directory in some designs (AMD Infinity Cache, Intel LLC).

Intel's L1 uses an 8-way, 64-byte line, 32 KB design. AMD's Zen uses 32 KB 8-way L1D. Apple's M-series has 128–192 KB L1 with very low latency.

Understanding the mapping and coherence mechanisms helps you write cache-friendly code: avoid strided access patterns that map to the same set, prefer sequential access for spatial locality, and be aware that false sharing (different cores writing to different data in the same cache line) causes unnecessary coherence traffic.

## Ship It

The cache simulator compiles with `gcc -o cache_sim main.c` and runs two experiments:

1. A stride-4K access pattern that thrashes a direct-mapped 4 KB cache but hits in a 2-way set-associative cache.
2. Statistics output (hits, misses, hit rate) for each configuration.

## Exercises

1. **2-Way Set-Associative Cache**: Extend the simulator to support configurable associativity. Implement the `find_line_in_set()` logic: check all ways in the target set for a tag match. Verify that the stride-2K pattern produces fewer misses in 2-way vs direct-mapped.

2. **LRU vs Random Replacement**: Add a random replacement policy alongside LRU. Run the same access patterns on both and compare hit rates. Plot (or print) results for different working set sizes.

3. **MESI Protocol Simulation**: Model 4 cores, each with a small cache (4 lines). Simulate read and write operations from each core, tracking the MESI state of every cached line. Print the bus transactions (BusRd, BusRdX, BusUpgr) and state transitions. Verify that a write by core 0 invalidates the copy in core 1's cache.
