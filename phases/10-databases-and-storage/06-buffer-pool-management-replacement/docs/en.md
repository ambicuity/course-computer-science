# Buffer Pool Management & Replacement

> Disk is slow. The buffer pool makes it look fast.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 10 lessons 01–05 (page layout, disk manager, slotted pages)
**Time:** ~75 minutes

## Learning Objectives

- Explain why every database needs a buffer pool and what problem it solves.
- Implement a page table with pin counts, dirty flags, and latch semantics.
- Implement Clock (second-chance) replacement and compare it against LRU, ARC, and LFU.
- Trace a page access pattern through a buffer pool and compute hit ratio.
- Describe how PostgreSQL and InnoDB manage their buffer pools in production.

## The Problem

A single 4 KB page read from a modern SSD takes ~10–50 µs. A sequential read through 1 GB of data (262,144 pages) at those speeds takes 2.6–13 *seconds*. Direct I/O to the disk is the bottleneck in every data-intensive system — and the gap widens when the working set exceeds the page cache.

Without a buffer pool, every query pays the full disk tax. A B-tree index lookup touches 3–5 pages; without caching, that's 150–250 µs of I/O per lookup. With a warm buffer pool, it's zero — the pages are already in memory. The buffer pool is the mechanism that makes the "hot set" fast while letting the cold data stay on disk.

The hard part: when the working set is larger than memory, which pages do you evict? Pick wrong and you thrash — evicting a page only to read it back on the next access.

## The Concept

### Buffer Pool Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     Storage Engine                       │
│    (B-tree, heap, hash index — wants pages by page_id)   │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│                     BUFFER POOL                          │
│                                                          │
│  ┌──────────────────────────┐   ┌────────────────────┐  │
│  │      Page Table           │   │   Frame Array       │  │
│  │  page_id → frame_id       │   │                     │  │
│  │  pin_count                │   │  [frame 0: page 7]  │  │
│  │  dirty flag               │   │  [frame 1: page 3]  │  │
│  │  latch                    │   │  [frame 2: free  ]  │  │
│  │  LSN (log seq number)     │   │  [frame 3: page 9]  │  │
│  └──────────────────────────┘   └────────────────────┘  │
│                                                          │
│         Replacement Policy (Clock / LRU / ARC)           │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│                     DISK MANAGER                         │
│       (read page i from file → Vec<u8>, write back)      │
└─────────────────────────────────────────────────────────┘
```

**Page table**: a hash map from `page_id` → frame metadata. On every `fix(page_id)` call (also called `pin` or `read_page`), the buffer pool checks the page table. If present → increment pin count, return pointer. If absent → pick a victim frame, evict the old page (write it back if dirty), read the new page from disk, insert into page table.

**Pin count**: How many threads/operations are currently using this page. A page with pin_count > 0 cannot be evicted. This is the fundamental contract: the storage engine pins a page before touching it and unpins it when done.

**Dirty flag**: Set when the in-memory copy differs from disk. On eviction, dirty pages must be written back. Clean pages can be discarded.

**Latch**: A lightweight mutex protecting the page content during concurrent access. Not to be confused with the *database lock* (transactional isolation) — a latch is a short-lived, in-memory mutex held for microseconds.

### Pin/Unpin Protocol

```
fix(page_id=42) → Frame
  1. Look up page_id in page table
  2. If hit:
       pin_count += 1
       return frame_pointer
  3. If miss:
       victim = choose_victim()         // replacement policy
       if victim.is_dirty:
           disk.write(victim.page_id, victim.data)
       new_data = disk.read(page_id)
       page_table.insert(page_id, victim.frame_id)
       pin_count = 1
       return frame_pointer

unfix(page_id, is_dirty)
  1. Look up page_id in page table
  2. pin_count -= 1
  3. if is_dirty: set dirty flag
```

### Replacement Policies

**LRU (Least Recently Used):** Maintain a doubly-linked list. On access, move page to head. On eviction, remove from tail. Simple, but vulnerable to *sequential scans* that flush the entire warm cache. PostgreSQL famously does not use LRU for this reason.

**Clock (Second-Chance):** Pages live in a circular buffer with a *reference bit*. The clock hand sweeps: if ref_bit=1, clear it and move on (second chance). If ref_bit=0, evict. Efficient — no pointer shuffling on every access. Used by PostgreSQL (via `pg_buffercache`), and the basis of many OS page replacement algorithms.

```
Hand → ┌───┐     ┌───┐     ┌───┐     ┌───┐
        │ P1 │     │ P2 │     │ P3 │     │ P4 │
        │ref=1│    │ref=0│    │ref=1│    │ref=0│
        └───┘     └───┘     └───┘     └───┘
          ▲                               │
          └─────────── circular ──────────┘
  Hand sweeps: P1 ref→0 (skip), P2 ref=0 → EVICT
```

**CLOCK-Pro:** Extension of Clock that approximates LRU by tracking both *recency* (recent accesses) and *frequency* (hot pages get refills faster). Replaces the cold page with the earliest access time.

**ARC (Adaptive Replacement Cache):** IBM-patented. Splits the cache into two lists: *recent* (recency) and *frequent* (frequency), plus ghost entries for each. Dynamically adjusts the balance between recency and frequency based on observed access patterns. Used in ZFS, InnoDB (`innodb_adaptive_hash_index` part), and Varnish. Patent US6996676B2.

**LFU (Least Frequently Used):** Track access frequency. Evict the page with the lowest count. Problem: a page accessed 100 times in the past but never again clogs the cache. InnoDB uses a variant called *touch-count* with periodic aging to decay old frequencies.

**FIFO:** Evict the oldest page regardless of access. Simple but terrible — frequently accessed pages get evicted just because they arrived early.

### Double Buffering Problem

The OS page cache already caches file blocks. If the database buffer pool also caches pages, the *same* page exists in two places: the OS cache and the DB buffer pool. This wastes memory and creates two layers of eviction policy fighting each other.

Solutions:
- **Direct I/O (`O_DIRECT`):** Bypass the OS page cache entirely. The database owns the cache. Used by InnoDB, PostgreSQL (optionally), and most serious databases.
- **`posix_fadvise(DONTNEED):`** Tell the OS to drop pages after the DB has read them. Less aggressive than `O_DIRECT`.
- **Large pages / huge pages:** Reduce TLB pressure when the buffer pool is many GB.

## Build It

We'll build a buffer pool in Rust with Clock replacement, a page table, pin/unpin semantics, dirty tracking, and a stats counter.

### Step 1: Disk Manager Trait

First, an interface for reading/writing pages to a backing store. We'll implement it with an in-memory `HashMap` for testing.

```rust
/// A page is a fixed-size block of bytes.
pub const PAGE_SIZE: usize = 4096;

pub type PageData = [u8; PAGE_SIZE];

pub trait DiskManager {
    fn read_page(&mut self, page_id: u64) -> PageData;
    fn write_page(&mut self, page_id: u64, data: &PageData);
}
```

### Step 2: Frame Metadata

Each slot in the buffer pool is a *frame*. The metadata tracks which page occupies it, how many users have it pinned, whether it's dirty, and the reference bit for the Clock algorithm.

```rust
#[derive(Clone)]
pub struct Frame {
    pub page_id: Option<u64>,
    pub data: PageData,
    pub pin_count: u32,
    pub dirty: bool,
    pub ref_bit: bool,        // Clock second-chance bit
}
```

### Step 3: Clock Replacement Policy

The clock hand advances on eviction. On `pin`, we set the ref_bit. On eviction, we sweep until we find a frame with ref_bit=0 and pin_count=0.

```rust
pub struct ClockReplacer {
    frames: Vec<usize>,   // indices into the buffer pool's frames
    hand: usize,
    num_frames: usize,
}

impl ClockReplacer {
    pub fn new(num_frames: usize) -> Self {
        Self {
            frames: (0..num_frames).collect(),
            hand: 0,
            num_frames,
        }
    }

    /// Find a victim frame. Returns None if all frames are pinned.
    pub fn victim(&mut self, pool: &[Frame]) -> Option<usize> {
        for _ in 0..self.num_frames * 2 {
            let idx = self.frames[self.hand];
            self.hand = (self.hand + 1) % self.num_frames;

            if pool[idx].pin_count > 0 {
                continue;
            }
            if pool[idx].ref_bit {
                // Second chance: clear bit, move on
                // SAFETY: interior mutability via RefCell or atomic would be better
                // but for this exercise we'll handle it through the pool ref
                continue; // ref_bit cleared in the loop below via a second pass
            }
            return Some(idx);
        }
        None
    }
}
```

We'll refine this in the full implementation below.

### Step 4: Full Buffer Pool

```rust
use std::collections::HashMap;

const PAGE_SIZE: usize = 4096;
pub type PageData = [u8; PAGE_SIZE];

pub trait DiskManager {
    fn read_page(&mut self, page_id: u64) -> PageData;
    fn write_page(&mut self, page_id: u64, data: &PageData);
}

/// In-memory disk: stores pages in a HashMap for testing.
pub struct MemoryDisk {
    pages: HashMap<u64, PageData>,
}

impl MemoryDisk {
    pub fn new() -> Self {
        Self { pages: HashMap::new() }
    }
    pub fn preload(&mut self, page_id: u64, data: PageData) {
        self.pages.insert(page_id, data);
    }
}

impl DiskManager for MemoryDisk {
    fn read_page(&mut self, page_id: u64) -> PageData {
        self.pages.get(&page_id).copied().unwrap_or([0u8; PAGE_SIZE])
    }
    fn write_page(&mut self, page_id: u64, data: &PageData) {
        self.pages.insert(page_id, *data);
    }
}

pub struct Frame {
    pub page_id: Option<u64>,
    pub data: PageData,
    pub pin_count: u32,
    pub dirty: bool,
    pub ref_bit: bool,
}

pub struct BufferPoolStats {
    pub hits: u64,
    pub misses: u64,
}

impl BufferPoolStats {
    pub fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { return 0.0; }
        self.hits as f64 / total as f64
    }
}

pub struct BufferPool {
    frames: Vec<Frame>,
    page_table: HashMap<u64, usize>,  // page_id → frame index
    clock_hand: usize,
    stats: BufferPoolStats,
}

impl BufferPool {
    pub fn new(capacity: usize) -> Self {
        let frames = (0..capacity)
            .map(|_| Frame {
                page_id: None,
                data: [0u8; PAGE_SIZE],
                pin_count: 0,
                dirty: false,
                ref_bit: false,
            })
            .collect();
        Self {
            frames,
            page_table: HashMap::new(),
            clock_hand: 0,
            stats: BufferPoolStats { hits: 0, misses: 0 },
        }
    }

    pub fn stats(&self) -> &BufferPoolStats {
        &self.stats
    }

    /// Pin a page. Returns a mutable reference to its frame's data.
    pub fn pin(&mut self, page_id: u64, disk: &mut dyn DiskManager) -> &mut PageData {
        // Fast path: already in buffer pool
        if let Some(&frame_idx) = self.page_table.get(&page_id) {
            self.stats.hits += 1;
            let frame = &mut self.frames[frame_idx];
            frame.pin_count += 1;
            frame.ref_bit = true;
            return &mut frame.data;
        }

        // Slow path: need to read from disk
        self.stats.misses += 1;

        // Find victim frame
        let victim_idx = self.evict();
        let frame = &mut self.frames[victim_idx];

        // Flush victim if dirty
        if let Some(victim_pid) = frame.page_id {
            if frame.dirty {
                disk.write_page(victim_pid, &frame.data);
            }
            self.page_table.remove(&victim_pid);
        }

        // Read new page
        let data = disk.read_page(page_id);
        frame.page_id = Some(page_id);
        frame.data = data;
        frame.pin_count = 1;
        frame.dirty = false;
        frame.ref_bit = true;
        self.page_table.insert(page_id, victim_idx);
        &mut self.frames[victim_idx].data
    }

    /// Unpin a page. If `dirty` is true, mark the page for writeback on eviction.
    pub fn unpin(&mut self, page_id: u64, dirty: bool) {
        if let Some(&frame_idx) = self.page_table.get(&page_id) {
            let frame = &mut self.frames[frame_idx];
            if frame.pin_count > 0 {
                frame.pin_count -= 1;
            }
            if dirty {
                frame.dirty = true;
            }
        }
    }

    /// Flush a specific dirty page to disk immediately.
    pub fn flush_page(&mut self, page_id: u64, disk: &mut dyn DiskManager) {
        if let Some(&frame_idx) = self.page_table.get(&page_id) {
            let frame = &self.frames[frame_idx];
            if frame.dirty {
                if let Some(pid) = frame.page_id {
                    disk.write_page(pid, &frame.data);
                }
            }
            // Safe borrow: we reborrow mutably after reading frame
            self.frames[frame_idx].dirty = false;
        }
    }

    /// Flush all dirty pages to disk.
    pub fn flush_all(&mut self, disk: &mut dyn DiskManager) {
        for i in 0..self.frames.len() {
            if let Some(pid) = self.frames[i].page_id {
                if self.frames[i].dirty {
                    disk.write_page(pid, &self.frames[i].data);
                    self.frames[i].dirty = false;
                }
            }
        }
    }

    // ----- internal helpers -----

    fn evict(&mut self) -> usize {
        let cap = self.frames.len();
        for _ in 0..cap * 2 {
            let idx = self.clock_hand;
            self.clock_hand = (self.clock_hand + 1) % cap;

            if self.frames[idx].pin_count > 0 {
                continue;
            }
            if self.frames[idx].ref_bit {
                // Second chance: clear ref bit and keep going
                self.frames[idx].ref_bit = false;
                continue;
            }
            // This frame is unpinned and has ref_bit=0 → evict
            return idx;
        }
        // Fallback: take the first unpinned frame
        for i in 0..cap {
            if self.frames[i].pin_count == 0 {
                return i;
            }
        }
        panic!("All frames are pinned — cannot evict");
    }
}
```

### Step 5: Test Harness — LRU vs Clock

The full `main.rs` (in `code/`) includes a test harness that compares LRU and Clock behavior with a controlled access pattern. The key insight: a sequential scan through more pages than the cache size will *flush* an LRU cache but leave a Clock cache's most popular pages intact (if access frequency varies).

## Use It

### PostgreSQL Buffer Manager

PostgreSQL's buffer manager lives in `src/backend/storage/buffer/`. Key parameters:

- **`shared_buffers`**: Size of the buffer pool (typically 25% of RAM). Default is 128 MB — ridiculously small for production.
- **Clock sweep**: PostgreSQL uses a Clock-like algorithm but calls it *clock sweep* (see `bufmgr.c:StrategyClockSweep`). The `usage_count` field replaces the single reference bit — each pin increments usage_count up to a max of 5.
- **Background writer (`bgwriter`)**: Periodically writes dirty pages to disk to keep eviction fast. Runs every `bgwriter_delay` ms.
- **Checkpointer**: Writes a checkpoint — forces *all* dirty pages to disk and writes a checkpoint WAL record. After a crash, recovery only needs to replay WAL from the last checkpoint.

### InnoDB Buffer Pool

InnoDB's buffer pool in MySQL/MariaDB is more sophisticated:

- **Adaptive hash index**: If InnoDB notices a B-tree page is being accessed via repeated lookups (not scans), it builds a hash index in memory to speed up future lookups — stored *inside* the buffer pool frames.
- **Change buffer**: Instead of reading a page just to modify a secondary index entry, InnoDB records the change in the change buffer and merges it lazily when the page is next read. Avoids random I/O for secondary index writes.
- **Page cleaner threads**: Dedicated threads that flush dirty pages in the background (analogous to PostgreSQL's bgwriter but with more parallelism).
- **LRU with midpoint insertion**: New pages are inserted at the midpoint of the LRU list, not the head. A page only moves to the head if accessed twice within a window. This protects against one-shot sequential scans polluting the hot set.

### Direct I/O and the Double Buffer

InnoDB uses `O_DIRECT` on the ibdata / ibd files by default. PostgreSQL historically avoided `O_DIRECT` (relying on `posix_fadvise`), but recent versions support it via `effective_io_concurrency` tuning. The tradeoff: `O_DIRECT` gives the DB full control over eviction but forces the DB to manage its own read-ahead and alignment.

## Read the Source

- **PostgreSQL `bufmgr.c`**: `src/backend/storage/buffer/bufmgr.c` — the heart of the buffer manager. Look at `PinBuffer`, `UnpinBuffer`, and `StrategyClockSweep`.
- **InnoDB `buf0buf.cc`**: MySQL source, `storage/innobase/buf/buf0buf.cc` — `BufPool::read_page` and the LRU midpoint insertion logic.
- **Rust `arc` replacement**: The `arcache` crate implements ARC on crates.io.

## Ship It

The reusable artifact is `code/main.rs` — a self-contained buffer pool library with Clock replacement, page table, stats, and a disk manager trait. Copy it into your own database project as a starting point.

## Exercises

1. **Easy** — Extend the test harness to log hit ratio after every 10 accesses. Run with a working set of 100 pages and a pool of 10 frames. What happens after the first 10 distinct pages?

2. **Medium** — Replace the Clock policy with LRU (using a `VecDeque` or a linked list). Compare hit ratios on a repeated 80/20 access pattern (80% of accesses to 20% of pages).

3. **Hard** — Add a `prefetch(page_id)` method that reads a page into the buffer pool without pinning it. If the hand sweeps past it before it gets pinned, it gets evicted — but if the storage engine accesses it soon after, it's a hit. This is how read-ahead works.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Buffer pool | Just a cache for disk pages | The central memory structure that all page access goes through — every read and write touches it. Cache misses block the query. |
| Pin | "I need this page" | A reference count that prevents eviction. Must be paired with unpin — leaks cause the buffer pool to fill with pinned, unflushable pages. |
| Clock sweep | A replacement algorithm | Circular scan with reference bits. Each full sweep clears one bit; a page survives as many sweeps as its access frequency. |
| Dirty page | A page that has been modified in memory | Must be written to disk before its frame can be reused. Crash + dirty unflushed page = corrupted data if no WAL. |
| Double buffering | The OS also caches file data | The same page sits in both the OS page cache and the DB buffer pool. Wastes memory and creates competing eviction policies. |

## Further Reading

- [The PostgreSQL Buffer Manager](https://www.postgresql.org/docs/current/storage-buffer-manager.html) — official docs with parameter reference.
- [InnoDB Buffer Pool](https://dev.mysql.com/doc/refman/8.0/en/innodb-buffer-pool.html) — MySQL docs on the LRU midpoint, adaptive hash index, and change buffer.
- [ARC: A Self-Tuning, Low Overhead Replacement Cache](https://www.usenix.org/legacy/events/fast03/tech/03-megiddo.pdf) — Megiddo & Modha, USENIX FAST '03. The paper behind the IBM patent.
- [CLOCK-Pro: An Effective Improvement of the CLOCK Replacement](https://www.usenix.org/legacy/events/usenix05/tech/general/full_papers/jiang/jiang.pdf) — Jiang & Zhang, USENIX '05.
- [Operating Systems: Three Easy Pieces — Paging Policies](https://pages.cs.wisc.edu/~remzi/OSTEP/vm-beyondphys-policy.pdf) — Chapter on replacement policies from the OSTEP book.
