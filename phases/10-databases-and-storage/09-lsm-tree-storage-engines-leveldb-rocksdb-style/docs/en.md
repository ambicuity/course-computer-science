# LSM-Tree Storage Engines (LevelDB / RocksDB style)

> Turn random writes into sequential ones by staging data in memory before flushing it to disk in sorted immutable files.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 10 lessons 01–06 (pages, buffer pool, B-Trees), basic Rust
**Time:** ~90 minutes

## Learning Objectives

- Explain why LSM-Trees outperform B-Trees on write-heavy workloads
- Implement a complete LSM-Tree engine with memtable, SSTable, bloom filters, and compaction
- Contrast leveled vs size-tiered compaction and their impact on read/write/space amplification
- Trace the read path: memtable → immutable → L0 → L1 → ... with bloom filter short-circuits
- Distinguish LevelDB from RocksDB at the feature level

## The Problem

A database that handles 100k writes/second cannot afford a disk seek per write. B-Trees write to random pages scattered across disk — even with a buffer pool, write-heavy workloads (time-series ingestion, messaging queues, audit logs) trigger constant page evictions and random I/O. The result: throughput collapses to the disk's random-write speed (~200 IOPS for HDD, ~50k IOPS for SSD).

An LSM-Tree solves this with a radical trade: **accept that data arrives sorted, but defer the sorting cost to read time and background compaction.** All writes go to an in-memory sorted structure (memtable). When the memtable is full, it's flushed to disk as one contiguous sorted run — a single sequential write, regardless of how many keys it contains. Random writes become sequential flushes, and throughput is limited only by sequential write bandwidth.

The price? Reads must now check multiple places (memtable + immutable + sorted runs on disk). Compaction runs in the background to keep the number of runs bounded. The LSM-Tree design is a three-way trade between write amplification, read amplification, and space amplification — and you get to pick which two matter most.

## The Concept

### LSM-Tree Structure

```
Writes ──► MemTable (C0) ──full──► Immutable ──flush──► SSTable on disk (C1, C2, ...)
              │                                              │
              │                                              │
         (sorted in-memory)                          (sorted runs, levels)
```

The tree has these components:

| Component | Role | Data structure |
|-----------|------|---------------|
| MemTable (C0) | Absorbs all writes in memory | BTreeMap / skiplist |
| Immutable | Frozen MemTable waiting to flush | Same as above, read-only |
| L0 SSTables | Newest flushed runs | Sorted files on disk, may overlap |
| L1, L2, ... | Compacted runs | Sorted files, non-overlapping per level |

### Write Path

```
put("b", "x")   ──►   MemTable.insert("b", "x")
put("a", "y")   ──►   MemTable.insert("a", "y")
put("c", "z")   ──►   MemTable.insert("c", "z")

MemTable full? ──► Freeze (memtable → immutable)
                  ──► New MemTable for new writes
                  ──► Background: flush immutable → SSTable on disk (L0)
```

Details:
1. Every write (`put`) inserts into the MemTable, which keeps keys sorted (via BTreeMap or skiplist).
2. The MemTable tracks its approximate byte size. When it exceeds a threshold (typically 4–64 MB), it's **frozen**: the active MemTable becomes read-only (immutable), and a fresh MemTable takes writes.
3. A background thread **flushes** the immutable to disk as an SSTable — a contiguous, sorted file. This is purely sequential I/O.
4. After the flush, the immutable is freed, and the new SSTable appears in level 0 (L0).

### SSTable Format

An SSTable is an immutable, sorted file divided into blocks:

```
┌────────────────────────────────────────────┐
│ Data Block 0  (4 KB)                       │
│   [key_a | val_a] [key_b | val_b] ...      │
├────────────────────────────────────────────┤
│ Data Block 1  (4 KB)                       │
│   [key_m | val_m] [key_n | val_n] ...      │
├────────────────────────────────────────────┤
│ ... (more data blocks)                     │
├────────────────────────────────────────────┤
│ Index Block                                │
│   [last_key_of_block_0 | offset_0]         │
│   [last_key_of_block_1 | offset_1]         │
│   ...                                      │
├────────────────────────────────────────────┤
│ Bloom Filter Block                         │
│   [bitset + num_hashes]                    │
├────────────────────────────────────────────┤
│ Footer (24 bytes)                          │
│   [bloom_offset | index_offset | magic]    │
└────────────────────────────────────────────┘
```

- **Data blocks**: Fixed-size (configurable, typically 4–16 KB) chunks of sorted key-value pairs. The last data block may be smaller.
- **Index block**: Maps each data block's last key to its offset. Enables binary search across blocks.
- **Bloom filter**: Probabilistic data structure that says "this key is definitely not in this SSTable" with configurable false-positive rate (~1%). Saves reading the entire SSTable for keys that don't exist.
- **Footer** : Fixed-size trailer pointing to bloom filter and index block offsets.

### Read Path

```
get("n") ──► 1. Check MemTable ("n" not found)
             2. Check Immutable ("n" not found)
             3. Check L0 SSTables (newest first):
                a. Check bloom filter → "n" might exist → read index → scan block → found!
```

Algorithm:
1. **MemTable** — O(log N) point query on the in-memory BTreeMap. Fastest check.
2. **Immutable** — Same as above, on the frozen MemTable.
3. **L0 SSTables** — Search newest file first (it has the most recent data). For each SSTable:
   a. **Bloom filter check** — if the filter says "not present", skip the entire file.
   b. **Index binary search** — find which data block could contain the key.
   c. **Block scan** — read the 4 KB data block and scan its entries.
4. **L1, L2, ...** — Same as step 3, level by level.

Since L0 files may overlap (flushes happen at different times), the read must search all L0 SSTables. In deeper levels, files are non-overlapping, so at most one file per level needs checking.

### Bloom Filters

A bloom filter is a bitset of `m` bits with `k` hash functions. To insert a key:

```
for i in 0..k:
    bit = hash_i(key) % m
    bitset[bit] = 1
```

To check membership:

```
for i in 0..k:
    bit = hash_i(key) % m
    if bitset[bit] == 0:
        return False   // definitely not present
return True            // probably present (false-positive rate ≈ (1 - e^{-kn/m})^k)
```

For `n` entries, optimal `m` and `k` are:
- `m = -n * ln(p) / (ln 2)^2` where `p` is the desired false-positive rate
- `k = (m/n) * ln 2`

In our engine, a key not in an SSTable costs one bloom filter check (a few hash computations and bit tests) instead of reading a 4 KB block and a disk seek.

### Compaction

Compaction merges multiple SSTables into fewer, larger ones. Two main strategies:

#### Size-Tiered Compaction (Cassandra, our implementation)

```
Level 0: [sst] [sst] [sst] [sst]   ← 4 files → trigger compaction
                           ↓
Level 1:       [merged sst]        ← 1 file (keep merging until next level fills)
```

When a level has N files (e.g., 4), all files in that level are merged into one or more files in the next level. The merge is a multi-way merge of sorted runs (like merge sort). Tombstones older than the oldest data in lower levels are dropped.

Pros: Simple, predictable write amplification. Cons: Read amplification grows if many levels have partial overlap.

#### Leveled Compaction (RocksDB)

```
L0: [sst] [sst] [sst] [sst]    ← overlapping
L1: [   ][   ][   ][   ][   ]  ← non-overlapping, each sst covers a key range
L2: 2x size of L1, same structure
```

Each level has a size limit (10× the previous level by default). When a level exceeds its limit, one SSTable is picked and merged with overlapping files in the next level. This keeps each level mostly non-overlapping.

Pros: Better read performance (at most one file per level to check). Cons: Higher write amplification (a key may be rewritten many times as it falls through levels).

### The Three Amplifications

| Metric | Defines | B-Tree | LSM-Tree (size-tiered) | LSM-Tree (leveled) |
|--------|---------|--------|----------------------|-------------------|
| Write amplification | How many times each byte is written to disk | Low (~1) | Medium | High (10–40×) |
| Read amplification | How many I/Os per point lookup | Low (tree height) | High (check all levels) | Medium (one file per level) |
| Space amplification | Extra disk space for temporary data | Low | Medium (tombstones, old files) | Low |

LSM-Trees favor writes. Leveled compaction trades higher write amplification for better reads. Size-tiered compaction minimizes write amplification at the cost of reads.

### Tombstones

A `delete(key)` in an LSM-Tree doesn't erase data — it inserts a **tombstone** (a marker value like `u32::MAX` for value length). During compaction, if a tombstone is older than the oldest data in any lower level, it can be dropped (the key is effectively gone).

Without tombstones, a deleted key could reappear if a compacted file doesn't include the deletion but a lower-level file still has the old value.

### LevelDB vs RocksDB

| Feature | LevelDB | RocksDB |
|---------|---------|---------|
| Compaction | Leveled only | Leveled + size-tiered + universal + FIFO |
| Column families | No | Yes (shared WAL, separate memtables/SSTables) |
| Merge operators | No | Yes (update without read-modify-write) |
| Bloom filter per block | No (whole-file filter) | Yes (`format_version >= 5`) |
| Rate limiter | No | Yes (throttle compaction I/O) |
| Persistent cache | No | Yes (block cache on faster device) |
| Transactions | Basic | Optimistic + pessimistic + write-prepared |
| Backup | Snapshot-based | Checkpoint + backup engine |

RocksDB is a fork of LevelDB by Meta, designed for SSD-optimized, multi-threaded workloads. It adds column families (separate logical DBs within one WAL), merge operators (for counters, accumulators), and more compaction strategies to handle diverse workloads.

## Build It

We'll build a single-file LSM-Tree engine in Rust with memtable, SSTable I/O, bloom filters, size-tiered compaction, and point/range queries.

### Step 1: Project Setup

Create `code/Cargo.toml`:

```toml
[package]
name = "lsm-tree"
version = "0.1.0"
edition = "2021"
```

No external dependencies — everything uses `std::collections::BTreeMap` for the memtable and `std::hash` for bloom filter hashing.

### Step 2: MemTable (C0)

The memtable is a `BTreeMap<Vec<u8>, ValueEntry>` that tracks its approximate byte size:

```rust
enum ValueEntry { Live(Vec<u8>), Tombstone }

struct MemTable {
    map: BTreeMap<Vec<u8>, ValueEntry>,
    approx_size: usize,
}
```

- `put(key, value)` — inserts into the BTreeMap, updates size.
- `delete(key)` — inserts `Tombstone`, updates size.
- `is_full()` — returns true when `approx_size >= threshold`.
- When full: the memtable is **frozen** (moved to `immutable`), a new empty memtable takes writes, and the immutable is flushed to disk.

### Step 3: SSTable Binary Format

Each SSTable is written as:

```
[Data Block 0] [Data Block 1] ... [Index Block] [Bloom Block] [Footer]
```

- **Data block**: `num_entries(u32 LE) | (key_len: u32 | key: bytes | value_len: u32 | value: bytes)*`
  - Tombstones use `value_len = u32::MAX`.
  - Blocks are built up to `BLOCK_SIZE` (4 KB) before flushing.
- **Index block**: `num_blocks(u32 LE) | (last_key_len: u32 | last_key: bytes | block_offset: u64 LE)*`
  - One entry per data block, sorted by key.
- **Bloom block**: `num_hashes(u32 LE) | bits_len(u32 LE) | bits(u64 × bits_len LE)`
- **Footer** (24 bytes): `bloom_offset(u64 LE) | index_offset(u64 LE) | magic(u32 LE = 0xDEAD_BEEF)`

The builder accumulates entries and writes the file on `build()`:

```rust
struct SSTableBuilder {
    current_block: Vec<(Vec<u8>, ValueEntry)>,
    current_block_bytes: usize,
    blocks_data: Vec<Vec<u8>>,
    index: Vec<(Vec<u8>, u64)>,
    bloom: BloomFilter,
}
```

### Step 4: Bloom Filter

A bloom filter with `k` hash functions over a `m`-bit bitset. We derive multiple hash functions from `std::hash::DefaultHasher` by varying a seed:

```rust
fn hash_key(key: &[u8], seed: u64) -> u64 {
    let mut h = DefaultHasher::new();
    h.write_u64(seed);
    h.write(key);
    h.finish()
}
```

Optimal parameters for `n` entries and false-positive rate `p`:
- `m = -n * ln(p) / (ln 2)^2` bits
- `k = (m/n) * ln 2` hashes

### Step 5: Read Path

```
get(key) → check MemTable → check Immutable → check L0 → check L1 → ...
```

For each SSTable:
1. **Bloom filter** — if `!might_contain(key)`, skip this file.
2. **Index binary search** — find which data block could contain the key.
3. **Block scan** — read the block bytes and scan entries linearly.

### Step 6: Compaction (Size-Tiered)

When a level has ≥ `4^(level+1)` files, merge all files in that level into the next:

```
fn compact_level(&mut self, level: usize) -> io::Result<()> {
    let ssts = take all SSTables from levels[level];
    let all_entries: BTreeMap<Vec<u8>, ValueEntry> = merge all entries (latest version wins);
    write all_entries to a new SSTable in levels[level + 1];
    delete old SSTables;
    if levels[level + 1] is now over threshold {
        compact_level(level + 1)?;
    }
}
```

This is a full merge: read all entries, deduplicate by key (newest version wins), write back sorted. Tombstones are preserved unless they're for keys that don't appear in lower levels (to keep it simple, we always preserve them).

### Step 7: Demo

The engine exercises all paths:

```
fn main() -> io::Result<()> {
    let mut db = LSMTree::new("test_db")?;
    
    db.put(b"alpha", b"first")?;
    db.put(b"beta", b"second")?;
    db.put(b"gamma", b"third")?;
    
    assert_eq!(db.get(b"alpha")?, Some(b"first".to_vec()));
    
    db.delete(b"beta")?;
    assert_eq!(db.get(b"beta")?, None);
    
    // Force multiple flushes to trigger compaction
    for i in 0..1000 {
        let k = format!("key-{:04}", i);
        let v = format!("val-{:04}", i);
        db.put(k.into_bytes(), v.into_bytes())?;
    }
    
    for (k, v) in db.scan(b"key-0050", b"key-0060")? {
        println!("{} => {}", String::from_utf8_lossy(&k), String::from_utf8_lossy(&v));
    }
    
    db.close()?;
}
```

## Use It

### LevelDB

LevelDB (Google, 2011) is the original LSM-Tree library. It's a C++ library with a straightforward API:

```cpp
leveldb::DB* db;
leveldb::DB::Open(options, "/tmp/testdb", &db);
db->Put(leveldb::WriteOptions(), "key", "value");
db->Get(leveldb::ReadOptions(), "key", &value);
db->Delete(leveldb::WriteOptions(), "key");
```

LevelDB uses **leveled compaction** by default. Each level has a size limit (10× of the previous level). Files within a level don't overlap. A background thread picks one file from a level and merges it with overlapping files in the next level. This limits read amplification — at most `num_levels` SSTable reads per lookup.

What LevelDB has that ours doesn't:
- **WAL (Write-Ahead Log)** — Writes first go to a sequential log on disk for crash recovery. The memtable is rebuilt from the WAL on restart.
- **Snapshots** — Consistent read views using sequence numbers.
- **Cache** — LRU block cache for frequently accessed SSTable blocks.
- **Manifest** — Persistent metadata tracking which SSTables are in which level.

### RocksDB

RocksDB (Meta, 2013) forked LevelDB and optimized it for SSD hardware with multi-threaded compaction, column families, and merge operators. Read its source at:

- `db/db_impl.cc` — `DBImpl::Put`, `DBImpl::Get`, compaction orchestration.
- `table/block_based_table_builder.cc` — `BlockBasedTableBuilder::Add` — SSTable building with optional delta-encoding and dictionary compression.
- `table/block_based_table_reader.cc` — `BlockBasedTableIterator` — block iteration with prefetching.

RocksDB's key innovation: **column families** let you partition data within one DB, each with its own memtable and SSTables but sharing a WAL. This enables workloads like "put hot data in CF1 with fast SSD, cold data in CF2 on HDD" within the same process.

## Read the Source

- **LevelDB**: `db/db_impl.cc` — `DBImpl::BackgroundCompaction` shows the compaction loop.
- **RocksDB**: `db/db_impl/db_impl_compaction_flush.cc` — `DBImpl::BackgroundCompaction` with multiple compaction strategies.
- **RocksDB SSTable builder**: `table/block_based_table_builder.cc` — how data blocks, filter blocks, index blocks, and footer are written.
- **Wisckey paper (LSM-tree storage + SSDs)**: Section 3 of "WiscKey: Separating Keys from Values in SSD-Conscious Storage" — explains why value separation reduces write amplification.

## Ship It

The reusable artifact is the LSM-Tree engine at `code/` (both the library and the `main.rs` demo). It can be extracted into its own crate and reused whenever a write-optimized KV store is needed — including the Phase 10 capstone (MVCC KV store) where the LSM-Tree serves as the underlying storage engine.

## Exercises

1. **Easy** — Add a `flush()` method that forces the memtable to disk. Measure the write throughput before and after with a microbenchmark.
2. **Medium** — Implement WAL recovery: write each `put` to a sequential log file before applying to the memtable. On startup, replay the log to rebuild the memtable.
3. **Hard** — Replace the full-merge compaction with a proper multi-way merge that streams entries through a min-heap (avoiding loading all entries into memory at once).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| LSM-Tree | A tree that stages writes in memory | A log-structured merge tree that batches random writes into sequential flushes, trading read efficiency for write throughput. |
| MemTable | The in-memory write buffer | A sorted data structure (skiplist / BTreeMap) that absorbs all writes before flushing to disk. |
| SSTable | A sorted file on disk | An immutable, sorted, block-indexed file with a bloom filter footer. Multiple SSTables at the same level may overlap (L0) or be disjoint (L1+). |
| Compaction | Merging SSTables | The background process that merges sorted runs to bound the number of files a read must check. |
| Write amplification | The write multiplier | Each byte from the application results in W bytes written to disk (flushes + compaction rewrites). Leveled compaction has higher WA than size-tiered. |
| Tombstone | A deleted-key marker | A special entry that shadows older values for the same key. Dropped during compaction when no lower level has the key. |
| Bloom filter | A "maybe" set | A probabilistic data structure that says "definitely not present" or "maybe present". Saves SSTable reads for nonexistent keys. |

## Further Reading

- [LevelDB GitHub](https://github.com/google/leveldb) — The original LSM implementation, ~10k lines of C++.
- [RocksDB GitHub](https://github.com/facebook/rocksdb) — Feature-rich fork with column families, merge operators, multiple compaction strategies.
- [Designing Data-Intensive Applications (Kleppmann)](https://dataintensive.net) — Chapter 3 covers LSM-Trees vs B-Trees with clear trade-off analysis.
- [CMU 15-445: LSM-Trees Lecture](https://www.youtube.com/watch?v=AmdY4LGT2HY) — Andy Pavlo walks through LSM-Tree internals with SQLite and RocksDB examples.
- [WiscKey paper](https://www.usenix.org/conference/fast16/technical-sessions/presentation/lu) — Separating keys from values to reduce write amplification on SSDs.
