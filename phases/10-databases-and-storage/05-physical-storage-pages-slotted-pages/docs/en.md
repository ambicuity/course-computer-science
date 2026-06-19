# Physical Storage — Pages, Slotted Pages

> A database that can't store data on disk is a very expensive calculator. Pages are how it survives a power cycle.

**Type:** Learn
**Languages:** Rust, C
**Prerequisites:** Phase 10 lessons 01–04 (relational model, SQL, normalization)
**Time:** ~60 minutes

## Learning Objectives

- Model the disk storage hierarchy and its latency/capacity tradeoffs from registers to HDD
- Explain why databases use fixed-size pages as the unit of I/O and how torn pages are prevented
- Implement a slotted page with variable-length record support from scratch in Rust and C
- Compare PostgreSQL's 8 KB page format with InnoDB's 16 KB format
- Build a heap file organizer that allocates new pages on demand when existing pages are full

## The Problem

You have 8 bytes of data — a single 64-bit integer — and you call `write(fd, &val, 8)`. That integer is now on a spinning platter or NAND flash cell. It works.

Now store a million records, each between 4 and 1024 bytes. Find record #582,391 and return it within 10 milliseconds. Your `write`-per-value approach fails: a single `fsync` costs 1–10 ms, so writing a million records one at a time takes hours. Reading them back by scanning byte-by-byte means traversing gigabytes to find one row.

Databases solve this with **pages**: fixed-size buffers (typically 4–16 KB) that are the atomic unit of I/O. Inside each page, a **slotted page** layout manages variable-length records. At the file level, a **heap file** organizes pages into a flat collection. Without this lesson, you cannot build the storage engine for the phase capstone — you will try to read single bytes from disk and wonder why your database is slower than a CSV file.

## The Concept

### The Storage Hierarchy

Every storage technology trades off capacity, latency, bandwidth, and cost-per-byte. The hierarchy exists because no single technology is simultaneously fast, cheap, and large.

| Level     | Capacity     | Latency    | Bandwidth   | Managed by  |
|-----------|-------------|------------|-------------|-------------|
| Register  | ~1 KB       | ~0.3 ns    | —           | Compiler    |
| L1 cache  | ~32 KB      | ~1 ns      | ~1 TB/s     | Hardware    |
| L2 cache  | ~256 KB     | ~4 ns      | ~500 GB/s   | Hardware    |
| L3 cache  | ~8–32 MB    | ~12 ns     | ~200 GB/s   | Hardware    |
| RAM       | 8–512 GB    | ~100 ns    | ~50 GB/s    | OS / DB     |
| NVMe SSD  | 256 GB–4 TB | ~10 µs     | 3–7 GB/s    | OS / DB     |
| SATA SSD  | 256 GB–4 TB | ~100 µs    | ~500 MB/s   | OS / DB     |
| HDD       | 1–20 TB     | ~5 ms      | ~200 MB/s   | OS / DB     |

The gap between RAM (~100 ns) and SSD (~10 µs) is **100×**. The gap between RAM and HDD (~5 ms) is **50,000×**. Databases bridge this gap by (1) batching operations into page-sized I/Os, (2) keeping hot pages in a buffer pool, and (3) using access patterns that maximize bandwidth per I/O (sequential scan, B-tree traversal).

### Pages as Unit of I/O

A page is a contiguous block that the database reads or writes atomically. Common sizes: 4 KB (MySQL with certain settings), 8 KB (PostgreSQL), 16 KB (InnoDB default), 32 KB (SQL Server).

Why not smaller? A 512-byte page means 8× more I/O operations for the same data — each I/O has fixed latency overhead. Why not larger? A 1 MB page wastes bandwidth when you only need one row and increases the chance of a partial write failure.

**Torn pages**: If the OS or hardware crashes while writing a page, only part of the page may be written — a torn page. Mitigations:

- **Atomic writes**: Some hardware guarantees 4 KB or 8 KB atomic sector writes
- **Double-write buffer**: InnoDB writes a page to a staging area before writing it in place
- **Page checksums**: Store a CRC-32 (or similar) in the page header; on read, recalculate and compare. A mismatch means the page is torn.
- **Full-page WAL**: PostgreSQL writes the entire page image to WAL on first modification after a checkpoint, enabling reconstruction of a torn page during recovery.

### Slotted Page Layout

A slotted page organizes variable-length records within a fixed-size buffer:

```
 0 ┌──────────────────────────────────────┐
   │ Page Header (24 bytes)               │
   │  - page_id, free_start, data_end,    │
   │    slot_count, flags, checksum       │
   ├──────────────────────────────────────┤
   │ Slot 0: offset=4060, length=12       │
   │ Slot 1: offset=4040, length=16       │
   │ Slot 2: offset=4016, length=20       │
   ├──────────────────────────────────────┤
   │         free space                   │
   │    (grows/shrinks as needed)         │
   ├──────────────────────────────────────┤
   │ Record 2 (20 bytes)                  │
   │ Record 1 (16 bytes)                  │
   │ Record 0 (12 bytes)                  │
   └──────────────────────────────────────┘
4096
```

The **page header** stores metadata: page ID, LSN (for WAL recovery), checksum, `free_start` (end of slot array), `data_end` (start of record data growing backward from page end), and slot count.

The **slot array** grows forward from the header. Each entry is (offset, length) — a pointer into the data area.

The **data area** grows backward from the end. Records are appended from the end, so the two regions grow toward each other, consolidating free space in the middle.

**Operations**:

- **Insert**: Write record data at `data_end - record_length`, append a slot entry. Both `free_start` and `data_end` adjust inward.
- **Get**: Read slot entry at `slot`, return `buffer[offset..offset+length]`.
- **Delete**: Set slot entry to (0, 0) tombstone. Data area is not reclaimed until defrag.
- **Update (shrink)**: Copy new data in place, update slot length. Zero the tail.
- **Update (grow)**: Tombstone the old slot, allocate new space at the end. May trigger defrag if space is tight.
- **Defrag**: Compact all live records to the end of the page, update offsets in slot entries. Slot numbers remain stable.

### Record IDs (RID)

A RID is `(page_id, slot_number)` — a physical identifier encoding the exact disk location. B-tree leaf pages store RIDs; to fetch a record the database (1) locates the page via page directory, (2) reads it (or finds it in the buffer pool), (3) indexes into the slot array. RIDs are stable for a record's lifetime even through defrag — only the offset in the slot entry changes.

### Heap File vs Sorted File Organization

**Heap file**: Records are placed in any page with free space. New pages are appended as needed. Insertion is O(1) amortized. No ordering — range queries require a full scan or an index.

**Sorted file**: Records are kept in key order within pages (and pages in order). Range scans are trivially fast, but insertion is expensive (may require shifting records/pages).

Most production databases use heap files for base tables and rely on separate B-tree indexes for sorted access.

### PostgreSQL vs InnoDB Page Formats

**PostgreSQL (8 KB pages, `src/include/storage/bufpage.h`)**:

```
┌──────────────────────────────────┐
│ PageHeaderData (24 bytes):       │
│  pd_lsn, pd_checksum, pd_flags  │
│  pd_lower (free start),          │
│  pd_upper (data end),            │
│  pd_special, pd_pagesize_version │
│  pd_prune_xid                    │
├──────────────────────────────────┤
│ ItemIdData[] — slot array        │
│  (each: lp_off + lp_flags + len) │
├──────────────────────────────────┤
│ Free space                       │
├──────────────────────────────────┤
│ Tuple data (rows)                │
├──────────────────────────────────┤
│ Special space (index-specific)   │
└──────────────────────────────────┘
```

`pd_lower` is the end of the slot array (first byte of free space). `pd_upper` is the start of the data area (last byte of free space + 1). ItemId entries have `lp_flags` for state: `LP_UNUSED`, `LP_NORMAL`, `LP_REDIRECT`, `LP_DEAD`. Tuples have their own headers with `t_xmin`/`t_xmax` for MVCC.

**InnoDB (16 KB default)**:

```
┌──────────────────────────────────┐
│ FIL Header (38 bytes)            │
│ Page Header (56 bytes)           │
├──────────────────────────────────┤
│ Infimum + Supremum records       │
├──────────────────────────────────┤
│ User records (sorted linked list │
│ via next_record offset)          │
├──────────────────────────────────┤
│ Free space                       │
├──────────────────────────────────┤
│ Page Directory (directory slots  │
│ for binary search)               │
├──────────────────────────────────┤
│ FIL Trailer (8 bytes)            │
└──────────────────────────────────┘
```

InnoDB records within a page form a singly-linked list sorted by primary key. The page directory stores pointers to every Nth record for binary search within the page. The infimum and supremum are sentinel records. The page header tracks `heap_top` (the boundary between used and free space, equivalent to `pd_upper`).

## Build It

We implement a slotted page in Rust and C with a 4096-byte buffer, a slot array at the front, and records packed from the end. Both implementations share the same design:

```
Header (24 bytes): page_id | free_start | data_end | slot_count | reserved
Slot entry (4 bytes): offset (u16) | length (u16)
Data area: grows backward from PAGE_SIZE
```

Full code listings are in `code/main.rs` and `code/main.c`. The key operations:

### Insert

Write record data at `data_end - len`, then write a slot entry (offset, length) at `free_start`. Advance both boundaries:

```rust
pub fn insert_record(&mut self, data: &[u8]) -> Result<u16, &str> {
    let needed = data.len() + SLOT_ENTRY_SIZE;
    if needed > self.free_space() {
        self.defragment();
        if needed > self.free_space() {
            return Err("page full");
        }
    }
    let slot = self.slot_count();
    let new_de = self.data_end() - data.len() as u16;
    self.buffer[new_de as usize..][..data.len()].copy_from_slice(data);
    self.set_slot_off(slot, new_de);
    self.set_slot_len(slot, data.len() as u16);
    self.set_data_end(new_de);
    self.set_slot_count(slot + 1);
    self.set_free_start(HEADER_SIZE as u16 + self.slot_count() * SLOT_ENTRY_SIZE as u16);
    Ok(slot)
}
```

### Get

Bounds-check the slot, read the offset/length from the slot entry, return a slice:

```rust
pub fn get_record(&self, slot: u16) -> Option<&[u8]> {
    if slot >= self.slot_count() { return None; }
    let off = self.slot_off(slot);
    let len = self.slot_len(slot);
    if off == 0 && len == 0 { return None; }
    Some(&self.buffer[off as usize..][..len as usize])
}
```

### Update (growing)

When a record grows beyond its current space, we tombstone the old slot and allocate new space at the end. The old data remains in the buffer — it will be reclaimed by the next defrag:

```rust
// Grow: relocate to end
self.set_slot_off(slot, 0);
self.set_slot_len(slot, 0);
let needed = data.len() + SLOT_ENTRY_SIZE;
if needed > self.free_space() {
    self.defragment();
    if needed > self.free_space() {
        return Err("page full after defrag");
    }
}
let new_de = self.data_end() - data.len() as u16;
self.buffer[new_de as usize..][..data.len()].copy_from_slice(data);
self.set_slot_off(slot, new_de);
self.set_slot_len(slot, data.len() as u16);
self.set_data_end(new_de);
```

### Defrag

Compact all live records to the end of the page. Slot numbers are preserved — only the offsets in each slot entry change:

```rust
pub fn defragment(&mut self) {
    let count = self.slot_count() as usize;
    // Collect live records, reset data_end, rewrite from end
    let mut live: Vec<Rec> = /* collect non-deleted records */;
    self.set_data_end(PAGE_SIZE as u16);
    for rec in &live {
        let new_off = self.data_end() - rec.data.len() as u16;
        self.set_slot_off(rec.slot as u16, new_off);
        self.buffer[new_off as usize..][..rec.data.len()].copy_from_slice(&rec.data);
        self.set_data_end(new_off);
    }
}
```

### HeapFile

A flat collection of pages. On insert, try each existing page; if all are full, allocate a new one:

```rust
pub fn insert_record(&mut self, data: &[u8]) -> (u32, u16) {
    for page in self.pages.iter_mut() {
        if let Ok(slot) = page.insert_record(data) {
            return (page.page_id(), slot);
        }
    }
    let mut page = SlottedPage::new(self.next_page_id);
    self.next_page_id += 1;
    let slot = page.insert_record(data).unwrap();
    let pid = page.page_id();
    self.pages.push(page);
    (pid, slot)
}
```

Run the implementations:

```
# Rust
cd code && cargo build && cargo run && cargo test

# C
cd code && gcc -Wall -Wextra -o main main.c && ./main
```

## Use It

Both PostgreSQL and InnoDB use slotted page layouts that are direct evolutions of our implementation:

**PostgreSQL** (`src/include/storage/bufpage.h`):
- `PageHeaderData.pd_lower` = our `free_start`, `pd_upper` = our `data_end`
- `ItemIdData` = our slot entries + `lp_flags` (UNUSED, NORMAL, REDIRECT, DEAD)
- `PageAddItemExtended()` = our insert with more safety checks
- `PageRepairFragmentation()` = our defrag, but also can prune unused line pointers

**InnoDB**:
- Records are linked via `next_record` offsets (sorted by key), not a flat slot array
- The page directory stores pointers for binary search within the page
- The infimum/supremum records are sentinel list terminators
- Page compaction happens during B-tree splits and merges

Our implementation is simplified (no MVCC, no binary search directory, no linked lists), but the core concept — separate metadata headers and records packed from opposite ends — is identical.

## Read the Source

- **PostgreSQL bufpage.h**: `src/include/storage/bufpage.h` in the PostgreSQL source — defines `PageHeaderData` and `ItemIdData`; the canonical reference for slotted page layout
- **PostgreSQL bufpage.c**: `src/backend/storage/page/bufpage.c` — `PageAddItemExtended`, `PageRepairFragmentation`
- **InnoDB page0page.cc**: MySQL source — `page_cur_insert_rec_low` and `page_compact` for the InnoDB page format
- **Architecture of a Database System** (Hellerstein, Stonebraker, Hamilton) — Foundations section on storage management

## Ship It

The reusable artifact is a self-contained slotted page library in Rust and C, saved in `outputs/`. Reuse it in:
- **Lesson 06 (Buffer Pool)**: the slotted page is the data structure the buffer pool manages
- **Lesson 07 (B-Tree Indexing)**: B-tree leaf and internal pages use the slotted page layout
- **Phase Capstone (MVCC KV Store)**: the storage engine uses this as its page format

## Exercises

1. **Easy** — Extend `utilization()` to print a histogram of record sizes. Run random insert/delete patterns and observe fragmentation.
2. **Medium** — Implement "redirect" slots: when a record is updated and moved, leave a forwarding pointer in the old slot so existing RIDs still resolve. (PostgreSQL uses this for HOT updates.)
3. **Hard** — Add concurrent access. In Rust: wrap with `RwLock<SlottedPage>` and verify no reader observes a partially written slot entry. In C: use `pthread_rwlock_t`.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Page | "The unit of I/O" | A fixed-size buffer (4–16 KB) that the database reads or writes atomically. All storage engine operations work on pages. |
| Slotted page | "A page with variable-length records" | A page layout where a slot array grows from the header and records grow from the end, allowing variable-length records without pre-allocating fixed slots. |
| Slot array | "The record index within a page" | An array of (offset, length) pairs at the start of free space. Slot N points to record N; slot numbers are stable within a page. |
| RID | "Record ID" | A `(page_id, slot_number)` pair that uniquely identifies a record. Physical addressing — it encodes disk location. |
| Heap file | "Unordered file of pages" | A file organization where new pages are appended and records go to any page with free space. |
| Torn page | "A half-written page" | A page partially written when a crash occurred. Detected via checksums; prevented via double-write buffer or full-page WAL. |
| LSN | "Log Sequence Number" | A counter in the page header used by WAL recovery to determine which REDO/UNDO operations apply. |
| Defrag | "Compacting a page" | Moving all live records to the end to consolidate free space, updating slot offsets. Slot numbers remain the same. |

## Further Reading

- **Database Systems: The Complete Book** (Garcia-Molina, Ullman, Widom), Chapter 13 — Disk storage and file organization
- [PostgreSQL bufpage.h](https://git.postgresql.org/gitweb/?p=postgresql.git;a=blob;f=src/include/storage/bufpage.h) — The canonical C implementation of slotted pages
- [InnoDB Page Layout](https://dev.mysql.com/doc/refman/8.0/en/innodb-page-layout.html) — MySQL documentation with diagrams
- *Let's Build a Simple Database* (cstack.github.io) — Tutorial series building a slotted page from scratch in C
- **Architecture of a Database System** (Hellerstein, Stonebraker, Hamilton) — Foundations and Architecture section on storage management
