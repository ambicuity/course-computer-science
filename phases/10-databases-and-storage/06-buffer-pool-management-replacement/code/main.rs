//! Buffer Pool with Clock (Second-Chance) Replacement
//!
//! A complete, compilable buffer pool implementation for Phase 10 Lesson 06.
//! Run with: cargo build && cargo run
//!
//! Design:
//! - BufferPool: fixed-capacity array of frames, page table (page_id → frame_idx)
//! - Clock replacement: circular sweep with reference bits
//! - Pin/Unpin with reference counting
//! - Dirty flag tracking with flush support
//! - Simulated in-memory DiskManager

use std::collections::HashMap;
use std::collections::VecDeque;

const PAGE_SIZE: usize = 4096;
pub type PageData = [u8; PAGE_SIZE];

/// Trait abstracting the backing store. A real implementation would read from
/// a file descriptor; here we use an in-memory HashMap for testing.
pub trait DiskManager {
    fn read_page(&mut self, page_id: u64) -> PageData;
    fn write_page(&mut self, page_id: u64, data: &PageData);
}

/// In-memory disk simulates a storage device. Pages are stored in a HashMap.
pub struct MemoryDisk {
    pages: HashMap<u64, PageData>,
}

impl MemoryDisk {
    pub fn new() -> Self {
        Self {
            pages: HashMap::new(),
        }
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

/// Metadata for a single frame in the buffer pool.
#[derive(Clone, Debug)]
pub struct Frame {
    pub page_id: Option<u64>,
    pub data: PageData,
    pub pin_count: u32,
    pub dirty: bool,
    pub ref_bit: bool,
}

impl Frame {
    fn new() -> Self {
        Self {
            page_id: None,
            data: [0u8; PAGE_SIZE],
            pin_count: 0,
            dirty: false,
            ref_bit: false,
        }
    }
}

/// Tracks buffer pool performance.
#[derive(Debug, Clone)]
pub struct BufferPoolStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub flushes: u64,
}

impl BufferPoolStats {
    pub fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            evictions: 0,
            flushes: 0,
        }
    }

    pub fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        self.hits as f64 / total as f64
    }
}

/// The buffer pool: caches disk pages in memory with Clock replacement.
pub struct BufferPool {
    frames: Vec<Frame>,
    page_table: HashMap<u64, usize>,
    clock_hand: usize,
    stats: BufferPoolStats,
}

impl BufferPool {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Buffer pool capacity must be > 0");
        let frames = (0..capacity).map(|_| Frame::new()).collect();
        Self {
            frames,
            page_table: HashMap::with_capacity(capacity),
            clock_hand: 0,
            stats: BufferPoolStats::new(),
        }
    }

    pub fn stats(&self) -> &BufferPoolStats {
        &self.stats
    }

    pub fn capacity(&self) -> usize {
        self.frames.len()
    }

    pub fn occupied_frames(&self) -> usize {
        self.frames.iter().filter(|f| f.page_id.is_some()).count()
    }

    /// Pin a page into the buffer pool.
    /// - If already resident: increment pin count, set ref bit, return data.
    /// - If not resident: evict a victim, read from disk, pin the new page.
    pub fn pin(&mut self, page_id: u64, disk: &mut dyn DiskManager) -> &mut PageData {
        if let Some(&idx) = self.page_table.get(&page_id) {
            self.stats.hits += 1;
            let f = &mut self.frames[idx];
            f.pin_count += 1;
            f.ref_bit = true;
            return &mut f.data;
        }

        self.stats.misses += 1;
        let idx = self.evict();

        if let Some(evicted_pid) = self.frames[idx].page_id {
            if self.frames[idx].dirty {
                disk.write_page(evicted_pid, &self.frames[idx].data);
                self.stats.flushes += 1;
            }
            self.page_table.remove(&evicted_pid);
        }

        let data = disk.read_page(page_id);
        let f = &mut self.frames[idx];
        f.page_id = Some(page_id);
        f.data = data;
        f.pin_count = 1;
        f.dirty = false;
        f.ref_bit = true;
        self.page_table.insert(page_id, idx);
        &mut f.data
    }

    /// Unpin a page. Mark it dirty if it was modified.
    pub fn unpin(&mut self, page_id: u64, dirty: bool) {
        if let Some(&idx) = self.page_table.get(&page_id) {
            let f = &mut self.frames[idx];
            if f.pin_count > 0 {
                f.pin_count -= 1;
            }
            if dirty {
                f.dirty = true;
            }
        }
    }

    /// Immediately write a specific dirty page to disk.
    pub fn flush_page(&mut self, page_id: u64, disk: &mut dyn DiskManager) {
        if let Some(&idx) = self.page_table.get(&page_id) {
            if self.frames[idx].dirty {
                if let Some(pid) = self.frames[idx].page_id {
                    disk.write_page(pid, &self.frames[idx].data);
                    self.stats.flushes += 1;
                }
                self.frames[idx].dirty = false;
            }
        }
    }

    /// Flush all dirty pages to disk.
    pub fn flush_all(&mut self, disk: &mut dyn DiskManager) {
        for i in 0..self.frames.len() {
            if let Some(pid) = self.frames[i].page_id {
                if self.frames[i].dirty {
                    disk.write_page(pid, &self.frames[i].data);
                    self.stats.flushes += 1;
                    self.frames[i].dirty = false;
                }
            }
        }
    }

    /// Evict a frame using the Clock (second-chance) algorithm.
    ///
    /// Sweeps the circular buffer. A frame with ref_bit=1 gets its bit cleared
    /// and survives (second chance). A frame with ref_bit=0 and pin_count=0
    /// is evicted. Pinned frames are always skipped.
    fn evict(&mut self) -> usize {
        let cap = self.frames.len();
        for _ in 0..cap * 2 {
            let idx = self.clock_hand;
            self.clock_hand = (self.clock_hand + 1) % cap;

            if self.frames[idx].pin_count > 0 {
                continue;
            }
            if self.frames[idx].ref_bit {
                self.frames[idx].ref_bit = false;
                continue;
            }
            self.stats.evictions += 1;
            return idx;
        }
        for i in 0..cap {
            if self.frames[i].pin_count == 0 {
                self.stats.evictions += 1;
                return i;
            }
        }
        panic!("Buffer pool deadlock: all {} frames are pinned", cap);
    }
}

// ---------------------------------------------------------------------------
// Comparison harness: Clock vs Naive LRU
// ---------------------------------------------------------------------------

fn run_comparison() {
    let pool_size = 10;
    let working_set = 50;
    let num_accesses = 1000;

    let (clock_hits, clock_misses) = simulate_clock(pool_size, working_set, num_accesses);
    let (lru_hits, lru_misses) = simulate_lru(pool_size, working_set, num_accesses);

    println!("\n── Comparison: Clock vs Naive LRU ──");
    println!("Pool size: {} frames, Working set: {} pages, Accesses: {}", 
             pool_size, working_set, num_accesses);
    println!("  Clock:     {} hits / {} misses  (hit ratio: {:.3})",
             clock_hits, clock_misses,
             clock_hits as f64 / (clock_hits + clock_misses) as f64);
    println!("  Naive LRU: {} hits / {} misses  (hit ratio: {:.3})",
             lru_hits, lru_misses,
             lru_hits as f64 / (lru_hits + lru_misses) as f64);
}

fn make_page(val: u8) -> PageData {
    [val; PAGE_SIZE]
}

fn simulate_clock(pool_size: usize, working_set: u64, num_accesses: u64) -> (u64, u64) {
    let mut pool = BufferPool::new(pool_size);
    let mut disk = MemoryDisk::new();
    for pid in 0..working_set {
        disk.preload(pid, make_page(pid as u8));
    }

    for i in 0..num_accesses {
        let pid = if i % 5 == 0 {
            i % working_set
        } else {
            (i * 7) % (working_set / 5)
        };
        pool.pin(pid, &mut disk);
        pool.unpin(pid, false);
    }

    let s = pool.stats();
    (s.hits, s.misses)
}

fn simulate_lru(pool_size: usize, working_set: u64, num_accesses: u64) -> (u64, u64) {
    let mut cache: HashMap<u64, PageData> = HashMap::new();
    let mut order: VecDeque<u64> = VecDeque::new();
    let mut disk = MemoryDisk::new();
    for pid in 0..working_set {
        disk.preload(pid, make_page(pid as u8));
    }

    let mut hits = 0u64;
    let mut misses = 0u64;

    for i in 0..num_accesses {
        let pid = if i % 5 == 0 {
            i % working_set
        } else {
            (i * 7) % (working_set / 5)
        };

        if cache.contains_key(&pid) {
            hits += 1;
            if let Some(pos) = order.iter().position(|&x| x == pid) {
                order.remove(pos);
            }
            order.push_back(pid);
        } else {
            misses += 1;
            let data = disk.read_page(pid);
            if cache.len() >= pool_size {
                if let Some(lru_pid) = order.pop_front() {
                    cache.remove(&lru_pid);
                }
            }
            cache.insert(pid, data);
            order.push_back(pid);
        }
    }

    (hits, misses)
}

// ---------------------------------------------------------------------------

fn main() {
    println!("=== Buffer Pool with Clock Replacement ===\n");

    let mut pool = BufferPool::new(4);
    let mut disk = MemoryDisk::new();

    for i in 0..10u64 {
        disk.preload(i, make_page(i as u8));
    }

    // Access pattern to demonstrate Clock behavior
    let access_pattern = vec![
        0, 1, 2, 3,
        4, 5, 6, 7,
        0, 1, 2, 3,
        0, 0, 0, 0,
        1, 1, 1, 1,
        8, 9,
        0, 1,
    ];

    for &pid in &access_pattern {
        pool.pin(pid, &mut disk);
        pool.unpin(pid, false);
    }

    let s = pool.stats();
    println!("Basic test results:");
    println!("  Hits:      {}", s.hits);
    println!("  Misses:    {}", s.misses);
    println!("  Evictions: {}", s.evictions);
    println!("  Flushes:   {}", s.flushes);
    println!("  Hit ratio: {:.3}", s.hit_ratio());
    println!("  Occupied frames: {}/{}", pool.occupied_frames(), pool.capacity());

    // --- Dirty page test ---
    println!("\n── Dirty Page Test ──");
    let mut pool2 = BufferPool::new(3);
    let mut disk2 = MemoryDisk::new();
    for i in 0..5u64 {
        disk2.preload(i, make_page(i as u8));
    }

    let data = pool2.pin(0, &mut disk2);
    data[0..4].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
    pool2.unpin(0, true);

    pool2.pin(1, &mut disk2);
    pool2.unpin(1, false);

    let data = pool2.pin(2, &mut disk2);
    data[0..4].copy_from_slice(&[0xCA, 0xFE, 0xBA, 0xBE]);
    pool2.unpin(2, true);

    // Pin page 3 — should evict a clean page (page 1) rather than a dirty one
    pool2.pin(3, &mut disk2);
    pool2.unpin(3, false);

    let s2 = pool2.stats();
    println!("  Flushes after evicting for page 3: {} (should be 0 — clean page 1 evicted)", s2.flushes);

    pool2.flush_page(0, &mut disk2);
    pool2.flush_page(2, &mut disk2);

    let s2 = pool2.stats();
    println!("  After explicit flush: {} total flushes", s2.flushes);

    let page0_data = disk2.read_page(0);
    let page2_data = disk2.read_page(2);
    assert_eq!(&page0_data[0..4], &[0xDE, 0xAD, 0xBE, 0xEF], "Page 0 flush corrupted");
    assert_eq!(&page2_data[0..4], &[0xCA, 0xFE, 0xBA, 0xBE], "Page 2 flush corrupted");
    println!("  Flush data verified OK");

    run_comparison();
}
