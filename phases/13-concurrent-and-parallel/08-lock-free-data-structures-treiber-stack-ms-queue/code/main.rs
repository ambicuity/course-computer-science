// Lock-Free Data Structures — Treiber Stack, MS Queue
// Phase 13 — Concurrent & Parallel Computing
//
// Implements:
//   - Treiber stack (lock-free LIFO) with ABA-counter tag
//   - Michael-Scott queue (lock-free FIFO) with dummy node
//   - Mutex-based equivalents for performance comparison
//   - Multi-threaded benchmark harness

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

// ==========================================================================
//  UTILITY: pack a raw pointer + ABA tag into AtomicUsize
// ==========================================================================
// We reserve the high 16 bits for a monotonic tag.  The low 48 bits hold the
// pointer (more than enough on any current 64‑bit architecture; production
// code should use a platform‑specific tag width, e.g. the top 16 bits on x64).
const TAG_SHIFT: usize = 48;

fn pack_ptr_tag<T>(ptr: *mut T, tag: usize) -> usize {
    (ptr as usize) | (tag << TAG_SHIFT)
}

fn unpack_ptr<T>(packed: usize) -> *mut T {
    (packed & ((1 << TAG_SHIFT) - 1)) as *mut T
}

fn unpack_tag(packed: usize) -> usize {
    packed >> TAG_SHIFT
}

// ==========================================================================
//  TREIBER STACK  (lock‑free, with ABA counter)
// ==========================================================================

struct TreiberNode<T> {
    data: T,
    next: usize, // packed (ptr + tag) of the next node
}

pub struct TreiberStack<T> {
    head: AtomicUsize, // packed (ptr + tag) of the top node
}

unsafe impl<T: Send> Send for TreiberStack<T> {}
unsafe impl<T: Send> Sync for TreiberStack<T> {}

impl<T> TreiberStack<T> {
    pub fn new() -> Self {
        TreiberStack {
            head: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, data: T) {
        let node = Box::into_raw(Box::new(TreiberNode {
            data,
            next: 0,
        }));

        loop {
            let head = self.head.load(Ordering::Acquire);
            let tag = unpack_tag(head);
            // link the new node to the current top
            unsafe { (*node).next = head; }
            if self
                .head
                .compare_exchange(head, pack_ptr_tag(node, tag + 1),
                                  Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
            // CAS failed → another thread changed head; retry
        }
    }

    pub fn pop(&self) -> Option<T> {
        loop {
            let head = self.head.load(Ordering::Acquire);
            let head_ptr: *mut TreiberNode<T> = unpack_ptr(head);
            if head_ptr.is_null() {
                return None;
            }
            let next = unsafe { (*head_ptr).next };
            let tag = unpack_tag(head);
            if self
                .head
                .compare_exchange(head, pack_ptr_tag(unpack_ptr::<TreiberNode<T>>(next), tag + 1),
                                  Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                let node = unsafe { Box::from_raw(head_ptr) };
                return Some(node.data);
            }
            // CAS failed → retry
        }
    }

    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        unpack_ptr::<TreiberNode<T>>(head).is_null()
    }
}

// ==========================================================================
//  MICHAEL-SCOTT QUEUE  (lock‑free, with dummy node)
// ==========================================================================

struct MSNode<T> {
    data: Option<T>,
    next: AtomicUsize, // packed (ptr + tag)
}

pub struct MSQueue<T> {
    head: AtomicUsize, // always points to dummy
    tail: AtomicUsize, // points to last node (or dummy when empty)
}

unsafe impl<T: Send> Send for MSQueue<T> {}
unsafe impl<T: Send> Sync for MSQueue<T> {}

impl<T> MSQueue<T> {
    pub fn new() -> Self {
        let dummy = Box::into_raw(Box::new(MSNode {
            data: None,
            next: AtomicUsize::new(0),
        }));
        let addr = pack_ptr_tag(dummy, 0);
        MSQueue {
            head: AtomicUsize::new(addr),
            tail: AtomicUsize::new(addr),
        }
    }

    pub fn enqueue(&self, data: T) {
        let node = Box::into_raw(Box::new(MSNode {
            data: Some(data),
            next: AtomicUsize::new(0),
        }));
        loop {
            let tail = self.tail.load(Ordering::Acquire);
            let tail_ptr: *mut MSNode<T> = unpack_ptr(tail);
            let next = unsafe { (*tail_ptr).next.load(Ordering::Acquire) };
            // Guard: if tail has been updated since we loaded it, retry
            if tail != self.tail.load(Ordering::Relaxed) {
                continue;
            }
            if next != 0 {
                // tail is lagging; help advance it
                let _ = self.tail.compare_exchange(
                    tail, next, Ordering::Release, Ordering::Relaxed,
                );
                continue;
            }
            // Try to link new node at tail->next
            if unsafe { (*tail_ptr).next.compare_exchange(
                0, pack_ptr_tag(node, 0), Ordering::Release, Ordering::Relaxed,
            ).is_ok() } {
                // Advance tail (best‑effort – may fail, next enqueuer will fix)
                let _ = self.tail.compare_exchange(
                    tail, pack_ptr_tag(node, 0), Ordering::Release, Ordering::Relaxed,
                );
                break;
            }
        }
    }

    pub fn dequeue(&self) -> Option<T> {
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
                    return None; // empty
                }
                // tail is lagging; advance it (best‑effort)
                let _ = self.tail.compare_exchange(
                    tail, next_packed, Ordering::Release, Ordering::Relaxed,
                );
                continue;
            }
            let next_ptr: *mut MSNode<T> = unpack_ptr(next_packed);
            let data = unsafe { (*next_ptr).data.take() };
            if self.head.compare_exchange(
                head, next_packed, Ordering::Release, Ordering::Relaxed,
            ).is_ok() {
                let _old_dummy = unsafe { Box::from_raw(head_ptr) };
                return data;
            }
            // CAS failed; restore data and retry
            unsafe { (*next_ptr).data = data; }
        }
    }

    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let head_ptr: *mut MSNode<T> = unpack_ptr(head);
        let next = unsafe { (*head_ptr).next.load(Ordering::Acquire) };
        next == 0
    }
}

// ==========================================================================
//  MUTEX-BASED STACK  (for comparison)
// ==========================================================================

pub struct MutexStack<T> {
    inner: Mutex<Vec<T>>,
}

impl<T> MutexStack<T> {
    pub fn new() -> Self {
        MutexStack { inner: Mutex::new(Vec::new()) }
    }

    pub fn push(&self, data: T) {
        self.inner.lock().unwrap().push(data);
    }

    pub fn pop(&self) -> Option<T> {
        self.inner.lock().unwrap().pop()
    }
}

// ==========================================================================
//  MUTEX-BASED QUEUE  (for comparison)
// ==========================================================================

pub struct MutexQueue<T> {
    inner: Mutex<Vec<T>>,
}

impl<T> MutexQueue<T> {
    pub fn new() -> Self {
        MutexQueue { inner: Mutex::new(Vec::new()) }
    }

    pub fn enqueue(&self, data: T) {
        self.inner.lock().unwrap().push(data);
    }

    pub fn dequeue(&self) -> Option<T> {
        let mut guard = self.inner.lock().unwrap();
        if guard.is_empty() {
            None
        } else {
            Some(guard.remove(0))
        }
    }
}

// ==========================================================================
//  HELPERS FOR CONCURRENT TESTING
// ==========================================================================

const NUM_THREADS: usize = 4;
const OPS_PER_THREAD: usize = 50_000;

fn test_treiber_stack() {
    let stack = Arc::new(TreiberStack::new());
    let mut handles = vec![];

    for _ in 0..NUM_THREADS {
        let s = Arc::clone(&stack);
        handles.push(thread::spawn(move || {
            for i in 0..OPS_PER_THREAD {
                s.push(i);
            }
            for _ in 0..OPS_PER_THREAD {
                loop {
                    if s.pop().is_some() {
                        break;
                    }
                    // spin until we get something (handles concurrent pops)
                }
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
    assert!(stack.is_empty());
    println!("  Treiber stack: passed");
}

fn test_ms_queue() {
    let queue = Arc::new(MSQueue::new());
    let mut handles = vec![];

    for _ in 0..NUM_THREADS {
        let q = Arc::clone(&queue);
        handles.push(thread::spawn(move || {
            for i in 0..OPS_PER_THREAD {
                q.enqueue(i);
            }
            for _ in 0..OPS_PER_THREAD {
                loop {
                    if q.dequeue().is_some() {
                        break;
                    }
                }
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
    assert!(queue.is_empty());
    println!("  MS queue: passed");
}

// ==========================================================================
//  BENCHMARKS
// ==========================================================================

fn bench_treiber_vs_mutex() {
    let n = NUM_THREADS;
    let ops = OPS_PER_THREAD;

    // Lock-free
    let stack = Arc::new(TreiberStack::new());
    let start = Instant::now();
    let mut handles = vec![];
    for _ in 0..n {
        let s = Arc::clone(&stack);
        handles.push(thread::spawn(move || {
            for i in 0..ops { s.push(i); }
            for _ in 0..ops {
                loop { if s.pop().is_some() { break; } }
            }
        }));
    }
    for h in handles { h.join().unwrap(); }
    let lf_elapsed = start.elapsed();
    let lf_throughput = (n * ops * 2) as f64 / lf_elapsed.as_secs_f64();

    // Mutex-based (must spin like lock-free version for fair comparison)
    let mstack = Arc::new(MutexStack::new());
    let start = Instant::now();
    let mut handles = vec![];
    for _ in 0..n {
        let s = Arc::clone(&mstack);
        handles.push(thread::spawn(move || {
            for i in 0..ops { s.push(i); }
            for _ in 0..ops { loop { if s.pop().is_some() { break; } } }
        }));
    }
    for h in handles { h.join().unwrap(); }
    let mx_elapsed = start.elapsed();
    let mx_throughput = (n * ops * 2) as f64 / mx_elapsed.as_secs_f64();

    println!(
        "  Treiber stack: {:>10.0} ops/s  |  Mutex stack: {:>10.0} ops/s  |  Speedup: {:.1}x",
        lf_throughput, mx_throughput, lf_throughput / mx_throughput
    );
}

fn bench_ms_vs_mutex() {
    let n = NUM_THREADS;
    let ops = OPS_PER_THREAD;

    // Lock-free
    let queue = Arc::new(MSQueue::new());
    let start = Instant::now();
    let mut handles = vec![];
    for _ in 0..n {
        let q = Arc::clone(&queue);
        handles.push(thread::spawn(move || {
            for i in 0..ops { q.enqueue(i); }
            for _ in 0..ops {
                loop { if q.dequeue().is_some() { break; } }
            }
        }));
    }
    for h in handles { h.join().unwrap(); }
    let lf_elapsed = start.elapsed();
    let lf_throughput = (n * ops * 2) as f64 / lf_elapsed.as_secs_f64();

    // Mutex-based (must spin like lock-free version for fair comparison)
    let mq = Arc::new(MutexQueue::new());
    let start = Instant::now();
    let mut handles = vec![];
    for _ in 0..n {
        let q = Arc::clone(&mq);
        handles.push(thread::spawn(move || {
            for i in 0..ops { q.enqueue(i); }
            for _ in 0..ops { loop { if q.dequeue().is_some() { break; } } }
        }));
    }
    for h in handles { h.join().unwrap(); }
    let mx_elapsed = start.elapsed();
    let mx_throughput = (n * ops * 2) as f64 / mx_elapsed.as_secs_f64();

    println!(
        "  MS queue:      {:>10.0} ops/s  |  Mutex queue: {:>10.0} ops/s  |  Speedup: {:.1}x",
        lf_throughput, mx_throughput, lf_throughput / mx_throughput
    );
}

// ==========================================================================
//  MAIN
// ==========================================================================

fn main() {
    println!("═══ Lock-Free Data Structures — Treiber Stack & MS Queue ═══\n");

    println!("─── Correctness tests ───");
    test_treiber_stack();
    test_ms_queue();
    println!();

    println!("─── Performance benchmarks ({} threads, {} ops/thread) ───",
             NUM_THREADS, OPS_PER_THREAD);
    bench_treiber_vs_mutex();
    bench_ms_vs_mutex();
    println!();

    println!("─── Demonstration of lock-free push/pop patterns ───\n");

    // Demonstrate Treiber stack with single thread
    let stack = TreiberStack::new();
    stack.push(10);
    stack.push(20);
    stack.push(30);
    assert_eq!(stack.pop(), Some(30));
    assert_eq!(stack.pop(), Some(20));
    assert_eq!(stack.pop(), Some(10));
    assert_eq!(stack.pop(), None);
    println!("  Treiber stack sequential demo: OK (30, 20, 10, empty)");

    // Demonstrate Michael-Scott queue with single thread
    let queue = MSQueue::new();
    queue.enqueue("a");
    queue.enqueue("b");
    queue.enqueue("c");
    assert_eq!(queue.dequeue(), Some("a"));
    assert_eq!(queue.dequeue(), Some("b"));
    assert_eq!(queue.dequeue(), Some("c"));
    assert_eq!(queue.dequeue(), None);
    println!("  MS queue sequential demo: OK (a, b, c, empty)\n");

    println!("═══ All checks passed ═══");
}
