//! Phase 13 Capstone — Work-Stealing Scheduler + Lock-Free Queue
//!
//! Integrates:
//!   - Chase-Lev work-stealing deque (lock-free, atomic memory ordering)
//!   - Michael-Scott lock-free queue (external task submission)
//!   - Work-stealing thread pool with random victim selection
//!   - Benchmark suite: Fibonacci, Parallel Map, Tree Traversal
//!   - Comparison: work-stealing vs thread-per-task vs mutex pool

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, AtomicUsize, Ordering, fence};
use std::sync::{Arc, Barrier, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::marker::PhantomData;

// ============================================================================
// CONSTANTS
// ============================================================================

const DEQUE_CAPACITY: usize = 4096;

// ============================================================================
// XorShift64 — Fast PRNG for victim selection
// ============================================================================

struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        XorShift64 { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_usize(&mut self, range: usize) -> usize {
        if range == 0 { return 0; }
        (self.next_u64() as usize) % range
    }
}

// ============================================================================
// TASK TYPE
// ============================================================================

pub type Task = Box<dyn FnOnce() + Send>;

// ============================================================================
// CHASE-LEV WORK-STEALING DEQUE (lock-free)
// ============================================================================

pub struct ChaseLevDeque<T> {
    top: AtomicIsize,
    bottom: AtomicIsize,
    buffer: UnsafeCell<Box<[MaybeUninit<T>]>>,
    capacity: usize,
    mask: usize,
}

unsafe impl<T: Send> Send for ChaseLevDeque<T> {}
unsafe impl<T: Send> Sync for ChaseLevDeque<T> {}

impl<T> ChaseLevDeque<T> {
    pub fn new() -> Self {
        let cap = DEQUE_CAPACITY;
        let buf: Vec<MaybeUninit<T>> = (0..cap).map(|_| MaybeUninit::uninit()).collect();
        ChaseLevDeque {
            top: AtomicIsize::new(0),
            bottom: AtomicIsize::new(0),
            buffer: UnsafeCell::new(buf.into_boxed_slice()),
            capacity: cap,
            mask: cap - 1,
        }
    }

    pub fn push(&self, value: T) -> bool {
        let b = self.bottom.load(Ordering::Relaxed);
        let t = self.top.load(Ordering::Acquire);
        if b - t >= self.capacity as isize {
            return false;
        }
        let idx = (b as usize) & self.mask;
        let buf = unsafe { &mut *self.buffer.get() };
        buf[idx] = MaybeUninit::new(value);
        fence(Ordering::Release);
        self.bottom.store(b + 1, Ordering::Release);
        true
    }

    pub fn can_push(&self) -> bool {
        let b = self.bottom.load(Ordering::Relaxed);
        let t = self.top.load(Ordering::Relaxed);
        b - t < self.capacity as isize
    }

    pub fn pop(&self) -> Option<T> {
        let b = self.bottom.load(Ordering::Relaxed) - 1;
        self.bottom.store(b, Ordering::Relaxed);
        fence(Ordering::SeqCst);
        let t = self.top.load(Ordering::Relaxed);

        if t <= b {
            let idx = (b as usize) & self.mask;
            let buf = unsafe { &*self.buffer.get() };
            let val = unsafe { buf[idx].assume_init_read() };

            if t == b {
                if self
                    .top
                    .compare_exchange(t, t + 1, Ordering::SeqCst, Ordering::Relaxed)
                    .is_err()
                {
                    self.bottom.store(t + 1, Ordering::Relaxed);
                    return None;
                }
                self.bottom.store(t + 1, Ordering::Release);
            }
            Some(val)
        } else {
            self.bottom.store(t, Ordering::Relaxed);
            None
        }
    }

    pub fn steal(&self) -> Option<T> {
        let t = self.top.load(Ordering::Acquire);
        fence(Ordering::SeqCst);
        let b = self.bottom.load(Ordering::Acquire);

        if t < b {
            let idx = (t as usize) & self.mask;
            let buf = unsafe { &*self.buffer.get() };
            let val = unsafe { buf[idx].assume_init_read() };

            if self
                .top
                .compare_exchange(t, t + 1, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                Some(val)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bottom.load(Ordering::Relaxed) <= self.top.load(Ordering::Relaxed)
    }

    pub fn len(&self) -> isize {
        self.bottom.load(Ordering::Relaxed) - self.top.load(Ordering::Relaxed)
    }
}

// ============================================================================
// MICHAEL-SCOTT LOCK-FREE QUEUE
// ============================================================================

const TAG_SHIFT: usize = 48;

fn pack_ptr_tag<T>(ptr: *mut T, tag: usize) -> usize {
    (ptr as usize) | (tag << TAG_SHIFT)
}

fn unpack_ptr<T>(packed: usize) -> *mut T {
    (packed & ((1 << TAG_SHIFT) - 1)) as *mut T
}

fn _unpack_tag(packed: usize) -> usize {
    packed >> TAG_SHIFT
}

struct MSNode<T> {
    data: Option<T>,
    next: AtomicUsize,
}

struct MSQueue<T> {
    head: AtomicUsize,
    tail: AtomicUsize,
    _phantom: PhantomData<T>,
}

unsafe impl<T: Send> Send for MSQueue<T> {}
unsafe impl<T: Send> Sync for MSQueue<T> {}

impl<T> MSQueue<T> {
    fn new() -> Self {
        let dummy: *mut MSNode<T> = Box::into_raw(Box::new(MSNode {
            data: None,
            next: AtomicUsize::new(0),
        }));
        let addr = pack_ptr_tag(dummy, 0);
        MSQueue {
            head: AtomicUsize::new(addr),
            tail: AtomicUsize::new(addr),
            _phantom: PhantomData,
        }
    }

    fn enqueue(&self, data: T) {
        let node = Box::into_raw(Box::new(MSNode {
            data: Some(data),
            next: AtomicUsize::new(0),
        }));
        loop {
            let tail = self.tail.load(Ordering::Acquire);
            let tail_ptr: *mut MSNode<T> = unpack_ptr(tail);
            let next = unsafe { (*tail_ptr).next.load(Ordering::Acquire) };
            if tail != self.tail.load(Ordering::Relaxed) {
                continue;
            }
            if next != 0 {
                let _ = self.tail.compare_exchange(
                    tail, next, Ordering::Release, Ordering::Relaxed,
                );
                continue;
            }
            if unsafe {
                (*tail_ptr).next.compare_exchange(
                    0, pack_ptr_tag(node, 0), Ordering::Release, Ordering::Relaxed,
                ).is_ok()
            } {
                let _ = self.tail.compare_exchange(
                    tail, pack_ptr_tag(node, 0), Ordering::Release, Ordering::Relaxed,
                );
                break;
            }
        }
    }

    fn dequeue(&self) -> Option<T> {
        loop {
            let head = self.head.load(Ordering::Acquire);
            let head_ptr: *mut MSNode<T> = unpack_ptr(head);
            let tail = self.tail.load(Ordering::Acquire);
            let next = unsafe { (*head_ptr).next.load(Ordering::Acquire) };
            if head != self.head.load(Ordering::Relaxed) {
                continue;
            }
            let next_packed = next;
            if head == tail {
                if next_packed == 0 {
                    return None;
                }
                let _ = self.tail.compare_exchange(
                    tail, next_packed, Ordering::Release, Ordering::Relaxed,
                );
                continue;
            }
                if self.head.compare_exchange(
                head, next_packed, Ordering::Release, Ordering::Relaxed,
            ).is_ok() {
                let _old_dummy = unsafe { Box::from_raw(head_ptr) };
                let next_ptr: *mut MSNode<T> = unpack_ptr(next_packed);
                return unsafe { (*next_ptr).data.take() };
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let head_ptr: *mut MSNode<T> = unpack_ptr(head);
        let next = unsafe { (*head_ptr).next.load(Ordering::Acquire) };
        next == 0
    }
}

impl<T> Drop for MSQueue<T> {
    fn drop(&mut self) {
        self.drain()
    }
}

impl<T> MSQueue<T> {
    fn drain(&mut self) {
        loop {
            match self.dequeue() {
                Some(_) => continue,
                None => break,
            }
        }
        let head = self.head.load(Ordering::Relaxed);
        let head_ptr: *mut MSNode<T> = unpack_ptr(head);
        if !head_ptr.is_null() {
            unsafe { drop(Box::from_raw(head_ptr)); }
        }
    }
}

// ============================================================================
// THREAD STATE — for local-vs-external spawn detection
// ============================================================================

use std::cell::Cell;

struct ThreadState {
    is_worker: Cell<bool>,
    worker_id: Cell<usize>,
}

thread_local! {
    static THREAD_STATE: ThreadState = ThreadState {
        is_worker: Cell::new(false),
        worker_id: Cell::new(0),
    };
}

// ============================================================================
// WORKER STATISTICS
// ============================================================================

#[repr(align(64))]
struct AlignedAtomicU64(AtomicU64);

struct WorkerStats {
    tasks_local: Vec<AlignedAtomicU64>,
    tasks_stolen: Vec<AlignedAtomicU64>,
    tasks_external: Vec<AlignedAtomicU64>,
    steal_attempts: Vec<AlignedAtomicU64>,
    steal_successes: Vec<AlignedAtomicU64>,
}

impl WorkerStats {
    fn new(num_workers: usize) -> Self {
        let make_vec = || {
            (0..num_workers).map(|_| AlignedAtomicU64(AtomicU64::new(0))).collect()
        };
        WorkerStats {
            tasks_local: make_vec(),
            tasks_stolen: make_vec(),
            tasks_external: make_vec(),
            steal_attempts: make_vec(),
            steal_successes: make_vec(),
        }
    }

    #[allow(dead_code)]
    fn record_local(&self, id: usize) {
        self.tasks_local[id].0.fetch_add(1, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    fn record_stolen(&self, id: usize) {
        self.tasks_stolen[id].0.fetch_add(1, Ordering::Relaxed);
    }

    fn record_external(&self, id: usize) {
        self.tasks_external[id].0.fetch_add(1, Ordering::Relaxed);
    }

    fn record_steal_attempt(&self, id: usize) {
        self.steal_attempts[id].0.fetch_add(1, Ordering::Relaxed);
    }

    fn record_steal_success(&self, id: usize) {
        self.steal_successes[id].0.fetch_add(1, Ordering::Relaxed);
    }

    fn total(&self, field: &[AlignedAtomicU64]) -> u64 {
        field.iter().map(|a| a.0.load(Ordering::Relaxed)).sum()
    }

    fn report(&self) {
        let local = self.total(&self.tasks_local);
        let stolen = self.total(&self.tasks_stolen);
        let external = self.total(&self.tasks_external);
        let attempts = self.total(&self.steal_attempts);
        let successes = self.total(&self.steal_successes);
        let total = local + stolen + external;

        println!("  Tasks executed locally:    {:>8}", local);
        println!("  Tasks executed (stolen):   {:>8}", stolen);
        println!("  Tasks from submission q:   {:>8}", external);
        println!("  Steal attempts:            {:>8}", attempts);
        println!("  Steal successes:           {:>8}", successes);
        if attempts > 0 {
            println!("  Steal success rate:        {:>7.1}%", 100.0 * successes as f64 / attempts as f64);
        }
        if total > 0 {
            println!("  Stolen fraction:           {:>7.1}%", 100.0 * stolen as f64 / total as f64);
        }
    }
}

// ============================================================================
// WORK-STEALING THREAD POOL
// ============================================================================

pub struct WorkStealingPool {
    deques: Vec<Arc<ChaseLevDeque<Task>>>,
    submission_queue: Arc<MSQueue<Task>>,
    tasks_remaining: Arc<AtomicIsize>,
    shutdown: Arc<AtomicBool>,
    handles: Vec<JoinHandle<()>>,
    stats: Arc<WorkerStats>,
    num_workers: usize,
}

impl WorkStealingPool {
    pub fn new(num_workers: usize) -> Self {
        assert!(num_workers > 0, "need at least 1 worker");
        let deques: Vec<Arc<ChaseLevDeque<Task>>> = (0..num_workers)
            .map(|_| Arc::new(ChaseLevDeque::new()))
            .collect();
        let submission_queue = Arc::new(MSQueue::<Task>::new());
        let tasks_remaining = Arc::new(AtomicIsize::new(0));
        let shutdown = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(WorkerStats::new(num_workers));
        let mut handles = Vec::with_capacity(num_workers);

        for id in 0..num_workers {
            let deque = Arc::clone(&deques[id]);
            let all_deques: Vec<Arc<ChaseLevDeque<Task>>> = deques.iter().map(|d| Arc::clone(d)).collect();
            let sub_q = Arc::clone(&submission_queue);
            let shutdown = Arc::clone(&shutdown);
            let stats = Arc::clone(&stats);

            let handle = thread::Builder::new()
                .name(format!("ws-worker-{}", id))
                .spawn(move || {
                    THREAD_STATE.with(|state| {
                        state.is_worker.set(true);
                        state.worker_id.set(id);
                    });

                    let mut rng = XorShift64::new((id as u64 + 1) * 0x9e3779b97f4a7c15);
                    let n = all_deques.len();

                    while !shutdown.load(Ordering::Relaxed) {
                        let task = deque.pop().or_else(|| sub_q.dequeue());

                        if let Some(t) = task {
                            t();
                            continue;
                        }

                        // Steal from random victim
                        let victim = rng.next_usize(n);
                        if victim != id {
                            stats.record_steal_attempt(id);
                            if let Some(t) = all_deques[victim].steal() {
                                stats.record_steal_success(id);
                                t();
                                continue;
                            }
                        }

                        // Try once more before yielding
                        if !deque.is_empty() || !sub_q.is_empty() {
                            continue;
                        }
                        thread::yield_now();
                    }
                })
                .expect("failed to spawn worker thread");

            handles.push(handle);
        }

        WorkStealingPool {
            deques,
            submission_queue,
            tasks_remaining,
            shutdown,
            handles,
            stats,
            num_workers,
        }
    }

    pub fn spawn(self: &Arc<Self>, task: Task) {
        let remaining = Arc::clone(&self.tasks_remaining);
        let stats = Arc::clone(&self.stats);
        let mut task = Some(task);

        THREAD_STATE.with(|state| {
            if state.is_worker.get() {
                let id = state.worker_id.get();
                if id < self.deques.len() && self.deques[id].can_push() {
                    remaining.fetch_add(1, Ordering::Release);
                    let t = task.take().unwrap();
                    let r = Arc::clone(&remaining);
                    self.deques[id].push(Box::new(move || {
                        t();
                        r.fetch_sub(1, Ordering::Release);
                    }));
                    return;
                }
            }
            remaining.fetch_add(1, Ordering::Release);
            stats.record_external(0);
            let t = task.take().unwrap();
            let r = Arc::clone(&remaining);
            self.submission_queue.enqueue(Box::new(move || {
                t();
                r.fetch_sub(1, Ordering::Release);
            }));
        });
    }

    pub fn wait(&self) {
        let mut rng = XorShift64::new(42);
        let n = self.deques.len();
        let start = Instant::now();
        while self.tasks_remaining.load(Ordering::Acquire) > 0 {
            if start.elapsed() > Duration::from_secs(3) {
                eprintln!("    [dbg] wait timeout, remaining={}", self.tasks_remaining.load(Ordering::Relaxed));
                break;
            }
            if let Some(task) = self.submission_queue.dequeue() {
                task();
                continue;
            }
            let victim = rng.next_usize(n);
            if let Some(task) = self.deques[victim].steal() {
                task();
                continue;
            }
            thread::yield_now();
        }
    }

    pub(crate) fn num_workers(&self) -> usize {
        self.num_workers
    }

    pub(crate) fn stats(&self) -> &WorkerStats {
        &self.stats
    }
}

impl Drop for WorkStealingPool {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        for h in self.handles.drain(..) {
            let _: () = h.join().unwrap();
        }
    }
}

// ============================================================================
// SEQUENTIAL COMPUTATIONS
// ============================================================================

fn fib_seq(n: u64) -> u64 {
    if n <= 1 { return n; }
    fib_seq(n - 1) + fib_seq(n - 2)
}

fn map_transform(x: u64) -> u64 {
    x.wrapping_mul(x).wrapping_add(2 * x).wrapping_add(1)
}

fn map_seq(data: &[u64]) -> u64 {
    data.iter().map(|&x| map_transform(x)).sum()
}

struct TreeNode {
    value: u64,
    left: Option<Box<TreeNode>>,
    right: Option<Box<TreeNode>>,
}

impl TreeNode {
    fn new(depth: u64, seed: u64) -> Self {
        let mut rng = XorShift64::new(seed);
        Self::build(depth, &mut rng)
    }

    fn build(depth: u64, rng: &mut XorShift64) -> Self {
        let value = rng.next_u64() % 100;
        if depth == 0 {
            TreeNode { value, left: None, right: None }
        } else {
            TreeNode {
                value,
                left: Some(Box::new(Self::build(depth - 1, rng))),
                right: Some(Box::new(Self::build(depth - 1, rng))),
            }
        }
    }

    fn sum(&self) -> u64 {
        let mut s = self.value;
        if let Some(ref l) = self.left { s += l.sum(); }
        if let Some(ref r) = self.right { s += r.sum(); }
        s
    }
}

// ============================================================================
// BENCHMARK HELPERS
// ============================================================================

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs_f64();
    if secs < 1.0 {
        format!("{:.1} ms", secs * 1000.0)
    } else {
        format!("{:.3} s", secs)
    }
}

fn tasks_per_sec(d: Duration, count: usize) -> String {
    let secs = d.as_secs_f64();
    if secs > 0.0 {
        format!("{:.0}", count as f64 / secs)
    } else {
        "inf".to_string()
    }
}

fn run_benchmark<F>(name: &str, pool: &Arc<WorkStealingPool>, num_tasks: usize, make_task: F)
where
    F: Fn() -> Task,
{
    let start = Instant::now();
    for _ in 0..num_tasks {
        pool.spawn(make_task());
    }
    pool.wait();
    let elapsed = start.elapsed();
    println!(
        "  {:24} {:>10} ({}/s)",
        name,
        format_duration(elapsed),
        tasks_per_sec(elapsed, num_tasks)
    );
}

// ============================================================================
// BENCHMARK: THREAD-PER-TASK (baseline)
// ============================================================================

fn bench_thread_per_task<F>(name: &str, num_tasks: usize, make_task: F)
where
    F: Fn() -> Task + Send + Sync,
{
    let barrier = Arc::new(Barrier::new(num_tasks + 1));
    let start = Instant::now();

    let mut handles = Vec::with_capacity(num_tasks);
    for _ in 0..num_tasks {
        let task = make_task();
        let b = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            b.wait();
            task();
        }));
    }

    barrier.wait(); // all threads start simultaneously
    for h in handles {
        h.join().unwrap();
    }

    let elapsed = start.elapsed();
    println!(
        "  {:24} {:>10} ({}/s)",
        name,
        format_duration(elapsed),
        tasks_per_sec(elapsed, num_tasks)
    );
}

// ============================================================================
// BENCHMARK: MUTEX POOL (simple thread pool with shared queue)
// ============================================================================

fn bench_mutex_pool<F>(name: &str, num_workers: usize, num_tasks: usize, make_task: F)
where
    F: Fn() -> Task + Send + Sync + 'static,
{
    let queue: Arc<Mutex<Vec<Task>>> = Arc::new(Mutex::new(Vec::new()));
    let remaining = Arc::new(AtomicIsize::new(0));
    let shutdown = Arc::new(AtomicBool::new(false));

    let start = Instant::now();

    // Spawn tasks
    for _ in 0..num_tasks {
        queue.lock().unwrap().push(make_task());
    }
    remaining.store(num_tasks as isize, Ordering::Release);

    let mut handles = Vec::with_capacity(num_workers);
    for _ in 0..num_workers {
        let q = Arc::clone(&queue);
        let rem = Arc::clone(&remaining);
        let sd = Arc::clone(&shutdown);
        handles.push(thread::spawn(move || {
            while !sd.load(Ordering::Relaxed) {
                let task = q.lock().unwrap().pop();
                if let Some(t) = task {
                    t();
                    rem.fetch_sub(1, Ordering::Release);
                } else {
                    if rem.load(Ordering::Acquire) <= 0 {
                        break;
                    }
                    thread::yield_now();
                }
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let elapsed = start.elapsed();
    println!(
        "  {:24} {:>10} ({}/s)",
        name,
        format_duration(elapsed),
        tasks_per_sec(elapsed, num_tasks)
    );
}

// ============================================================================
// VERIFICATION TESTS
// ============================================================================

fn test_chase_lev() {
    let d = ChaseLevDeque::new();
    d.push(10);
    d.push(20);
    d.push(30);
    assert_eq!(d.pop(), Some(30));
    assert_eq!(d.pop(), Some(20));
    assert_eq!(d.pop(), Some(10));
    assert!(d.pop().is_none());
    assert!(d.is_empty());

    // Concurrent test: one owner pushes, multiple thieves steal
    let d = Arc::new(ChaseLevDeque::new());
    for j in 0..2000 {
        d.push(j);
    }
    assert_eq!(d.len(), 2000);

    let d = Arc::new(ChaseLevDeque::new());
    let mut handles = Vec::new();
    // Owner pushes 2000 items
    for j in 0..2000 {
        d.push(j);
    }
    // 4 thieves each steal ~500 items
    for _ in 0..4 {
        let deq = Arc::clone(&d);
        handles.push(thread::spawn(move || {
            let mut count = 0;
            for _ in 0..500 {
                if deq.steal().is_some() {
                    count += 1;
                }
            }
            count
        }));
    }
    let mut total = 0;
    for h in handles { total += h.join().unwrap(); }
    assert_eq!(total, 2000);
    println!("  Chase-Lev deque concurrent: OK (2000 items, {} stolen)", total);
}

fn test_ms_queue() {
    let q: MSQueue<i64> = MSQueue::new();
    q.enqueue(10);
    q.enqueue(20);
    q.enqueue(30);
    assert_eq!(q.dequeue(), Some(10));
    assert_eq!(q.dequeue(), Some(20));
    assert_eq!(q.dequeue(), Some(30));
    assert_eq!(q.dequeue(), None);
    assert!(q.is_empty());

    // Multi-producer, single consumer
    let q = Arc::new(MSQueue::<i64>::new());
    let mut handles = Vec::new();
    for i in 0..4 {
        let qq = Arc::clone(&q);
        handles.push(thread::spawn(move || {
            for j in 0..500 {
                qq.enqueue((i * 1000 + j) as i64);
            }
        }));
    }
    for h in handles { h.join().unwrap(); }

    // Single consumer
    let mut count = 0;
    loop {
        match q.dequeue() {
            Some(_) => count += 1,
            None => break,
        }
    }
    assert_eq!(count, 2000);
    println!("  MS queue concurrent: OK (2000 items, {} dequeued)", count);
}

fn test_pool() {
    let pool = Arc::new(WorkStealingPool::new(4));
    let counter = Arc::new(AtomicUsize::new(0));
    for _ in 0..100 {
        let c = Arc::clone(&counter);
        pool.spawn(Box::new(move || { c.fetch_add(1, Ordering::Relaxed); }));
    }
    pool.wait();
    assert_eq!(counter.load(Ordering::Relaxed), 100);
    println!("  Work-stealing pool: OK (100 tasks)");
}

// ============================================================================
// SEQUENTIAL BASELINES
// ============================================================================

fn bench_seq_fib(n: u64, iterations: usize) -> Duration {
    let start = Instant::now();
    for _ in 0..iterations {
        std::hint::black_box(fib_seq(n));
    }
    start.elapsed()
}

fn bench_seq_map(size: usize) -> Duration {
    let data: Vec<u64> = (0..size).map(|x| x as u64).collect();
    let start = Instant::now();
    std::hint::black_box(map_seq(&data));
    start.elapsed()
}

fn bench_seq_tree(depth: u64, num_trees: usize) -> Duration {
    let trees: Vec<TreeNode> = (0..num_trees)
        .map(|i| TreeNode::new(depth, i as u64))
        .collect();
    let start = Instant::now();
    for tree in &trees {
        std::hint::black_box(tree.sum());
    }
    start.elapsed()
}

// ============================================================================
// MAIN
// ============================================================================

fn main() {
    println!("\n═══ Phase 13 Capstone — Work-Stealing Scheduler + Lock-Free Queue ═══\n");
    println!("─── Verification ───\n");
    test_chase_lev();
    test_ms_queue();
    test_pool();

    println!("\n─── Benchmark: Fibonacci (fib(25), 512 tasks × 4 workers) ───\n");

    let num_workers = 4;
    let num_fib_tasks = 512;
    let fib_n = 25;

    // Sequential baseline
    let seq_time = bench_seq_fib(fib_n, num_fib_tasks);
    println!(
        "  {:<24} {:>10} ({}/s)",
        "Sequential",
        format_duration(seq_time),
        tasks_per_sec(seq_time, num_fib_tasks)
    );

    // Work-stealing pool
    let pool = Arc::new(WorkStealingPool::new(num_workers));
    run_benchmark("Work-Stealing Pool", &pool, num_fib_tasks, || {
        Box::new(move || { std::hint::black_box(fib_seq(fib_n)); })
    });

    // Thread-per-task
    bench_thread_per_task("Thread-per-Task", num_fib_tasks, || {
        Box::new(move || { std::hint::black_box(fib_seq(fib_n)); })
    });

    // Mutex pool
    bench_mutex_pool("Mutex Pool", num_workers, num_fib_tasks, move || {
        Box::new(move || { std::hint::black_box(fib_seq(fib_n)); })
    });

    println!("\n─── Benchmark: Parallel Map (size = 1,000,000, 4 chunks) ───\n");

    let map_size = 1_000_000usize;
    let seq_map_time = bench_seq_map(map_size);
    println!(
        "  {:<24} {:>10}",
        "Sequential",
        format_duration(seq_map_time)
    );

    let pool2 = Arc::new(WorkStealingPool::new(num_workers));
    let chunk_size = map_size / 4;
    let data: Arc<Vec<u64>> = Arc::new((0..map_size).map(|x| x as u64).collect());

    let map_start = Instant::now();
    for chunk_idx in 0..4 {
        let data = Arc::clone(&data);
        let start_idx = chunk_idx * chunk_size;
        let end_idx = if chunk_idx == 3 { map_size } else { start_idx + chunk_size };
        pool2.spawn(Box::new(move || {
            let mut sum = 0u64;
            for i in start_idx..end_idx {
                sum = sum.wrapping_add(map_transform(data[i]));
            }
            std::hint::black_box(sum);
        }));
    }
    pool2.wait();
    let map_elapsed = map_start.elapsed();
    println!(
        "  {:<24} {:>10}",
        "Work-Stealing Pool",
        format_duration(map_elapsed)
    );
    if seq_map_time.as_secs_f64() > 0.0 {
        println!(
            "  Speedup:                        {:>7.1}x",
            seq_map_time.as_secs_f64() / map_elapsed.as_secs_f64()
        );
    }

    println!("\n─── Benchmark: Tree Traversal (depth=20, 16 trees, 4 workers) ───\n");

    let tree_depth = 20;
    let num_trees = 16;
    // Pre-build trees so tasks only do traversal
    // Pre-build trees so tasks only do traversal (no construction cost)
    let trees: Arc<Vec<TreeNode>> = Arc::new(
        (0..num_trees).map(|i| TreeNode::new(tree_depth, i as u64)).collect()
    );
    let seq_tree_time = bench_seq_tree(tree_depth, num_trees);
    println!(
        "  {:<24} {:>10}",
        "Sequential",
        format_duration(seq_tree_time)
    );

    let pool3 = Arc::new(WorkStealingPool::new(num_workers));
    let tree_start = Instant::now();
    for i in 0..num_trees {
        let trees = Arc::clone(&trees);
        let pool3 = Arc::clone(&pool3);
        pool3.spawn(Box::new(move || {
            std::hint::black_box(trees[i].sum());
        }));
    }
    pool3.wait();
    let tree_elapsed = tree_start.elapsed();
    println!(
        "  {:<24} {:>10}",
        "Work-Stealing Pool",
        format_duration(tree_elapsed)
    );
    if seq_tree_time.as_secs_f64() > 0.0 {
        println!(
            "  Speedup:                        {:>7.1}x",
            seq_tree_time.as_secs_f64() / tree_elapsed.as_secs_f64()
        );
    }

    println!("\n─── Work-Stealing Statistics ───\n");
    pool.stats().report();
    pool2.stats().report();
    pool3.stats().report();

    println!("\n═══ Capstone complete ═══\n");
}
