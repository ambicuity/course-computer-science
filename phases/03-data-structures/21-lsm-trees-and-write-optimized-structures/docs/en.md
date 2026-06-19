# LSM Trees and Write-Optimized Structures

> The B+-tree is read-optimized; the LSM tree is write-optimized. Behind LevelDB, RocksDB, Cassandra, HBase, ScyllaDB, BigTable.

**Type:** Build
**Languages:** C, Rust
**Prerequisites:** P03 L12 (B-trees), L17 (segment tree), L20 (Bloom filter)
**Time:** ~75 minutes

## Learning Objectives

- Implement a toy **LSM tree**: in-memory memtable + flushed SSTables + merge on read.
- Apply **compaction** strategies (size-tiered vs leveled) — and their trade-offs.
- Use **Bloom filters per SSTable** to skip files on read.
- Understand the write-amplification cost vs B+ tree's read-amplification cost.

## The Problem

B+ trees give balanced random reads/writes but are read-optimized: every write does ~log n random page seeks + rewrites a leaf page. On modern SSDs/disks, **random writes are far more expensive than sequential writes** (10-100× factor).

LSM trees trade random reads for **sequential writes**:

- Writes go to a small in-memory sorted structure (memtable).
- When the memtable fills, dump it to disk as an immutable sorted file (SSTable). Sequential write.
- Background compaction merges SSTables to keep the number bounded.
- Reads check memtable first, then SSTables newest-to-oldest.

Net: writes are nearly sequential disk I/O (fast). Reads might check multiple SSTables (slower) — but Bloom filters short-circuit ~99% of negative file checks.

This is THE architecture of write-heavy databases: BigTable (Google), Cassandra, LevelDB, RocksDB, HBase, ScyllaDB.

## The Concept

### The pieces

- **Memtable**: in-memory sorted map (typically a skip list — Phase 3 L19). Bounded size (e.g., 64 MB).
- **WAL (write-ahead log)**: append-only log for crash recovery. Every memtable insert is also logged.
- **SSTable (Sorted String Table)**: immutable on-disk file of key-value pairs sorted by key. Created when memtable flushes.
- **Bloom filter**: one per SSTable; answers "is key likely here?" before opening the file.
- **Compaction**: background process that merges SSTables.

### Write path

1. Append to WAL (sequential disk write).
2. Insert into memtable (in-memory).
3. When memtable hits size limit: rename it to "immutable memtable", create new memtable.
4. Background thread flushes immutable memtable to a new SSTable file. Sequential write.

### Read path

1. Check memtable.
2. Check immutable memtable (if flush in progress).
3. For each SSTable, newest-to-oldest:
   - Check Bloom filter — if no, skip the file (~99% of misses).
   - Otherwise, open the file's index, binary search, read the block.
4. Return the first value found (newest version wins).

### Compaction strategies

- **Size-tiered (Cassandra default)**: SSTables grouped by size; when ≥4 of similar size, merge them into one. Fewer reads, more write amplification.
- **Leveled (LevelDB / RocksDB default)**: SSTables organized in L0 (allow overlap), L1, L2, ... (no overlap within a level; geometric size growth). Bounded read amplification: at most one file per level. More write amplification.

The literature has dozens of variants — Tombstone-aware, tiered+leveled hybrid, FIFO for time-series, universal in RocksDB. All trade write amplification ↔ read amplification ↔ space amplification.

### Tombstones

To delete a key, write a "tombstone" entry. Reads that hit a tombstone before any non-tombstone entry return "not found." Compaction eventually drops tombstones and the older entries they hide.

### Amplification

LSM has three:
- **Write amp**: bytes written / user bytes (typically 10-30 for leveled).
- **Read amp**: number of files checked per read (typically 2-10).
- **Space amp**: disk space used / live data size (1.1-2.0).

Tuning these against your workload is the LSM operator's job.

## Build It

`code/main.c`:

1. Memtable using a sorted map (we cheat with a sorted array; in production use skip list).
2. SSTable: an immutable sorted array on "disk" (in memory for the demo).
3. Per-SSTable Bloom filter.
4. Write 10K keys, flushing every 1K to a new SSTable. Verify reads return the latest value.
5. Demonstrate Bloom-skip: time reads with and without Bloom.

`code/main.rs` standard idiomatic Rust.

### Run

```sh
clang -O2 -lm main.c -o lsm && ./lsm
```

## Use It

- **LevelDB (Google, 2011)**: original open-source LSM. ~30K lines of C++.
- **RocksDB (Facebook)**: LevelDB fork with hundreds of tuning knobs.
- **Cassandra**: distributed LSM with size-tiered compaction (now also leveled).
- **HBase**: BigTable open-source clone.
- **ScyllaDB**: C++ rewrite of Cassandra, also LSM.
- **CockroachDB**: uses RocksDB (now Pebble, Go LSM) as storage engine.

Anywhere the workload is write-heavy at scale, LSM is the architecture.

## Read the Source

- [LevelDB source](https://github.com/google/leveldb) — start with `doc/impl.md`.
- [RocksDB wiki](https://github.com/facebook/rocksdb/wiki) — extensive design docs.
- *The Log-Structured Merge-Tree (LSM-Tree)* by O'Neil et al. (1996) — original paper.
- *Designing Data-Intensive Applications* by Kleppmann — Chapter 3 covers LSM.

## Ship It

This lesson ships **`outputs/lsm_toy.h`** — a 200-line LSM tree with memtable + SSTable + Bloom + compaction.

## Exercises

1. **Easy.** Add a `delete(key)` operation via tombstone. Verify read returns "not found" after delete.
2. **Medium.** Add leveled compaction: SSTables in L1 don't overlap; merge L0 → L1, L1 → L2, etc. Compare with the size-tiered version on write amplification.
3. **Hard.** Add a "block cache" of recently-read SSTable blocks; measure read amplification reduction on a Zipfian workload (80/20 hot/cold).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Memtable | "In-RAM sorted buffer" | The latest writes, sorted, before flush |
| SSTable | "Sorted file" | Immutable sorted key-value file on disk |
| WAL | "Write-ahead log" | Sequential durability log for crash recovery |
| Compaction | "Merge files" | Background merger of SSTables to keep file count low |
| Write amp | "Write amplification" | Bytes written / user bytes |
| Tombstone | "Delete marker" | Special SSTable entry indicating a deletion |

## Further Reading

- *Designing Data-Intensive Applications* Ch. 3.
- *Bigtable: A Distributed Storage System for Structured Data* (Chang et al., Google).
- *The Universal Storage Engine: a unified LSM-tree* (Y. Mei) — RocksDB's universal compaction.
