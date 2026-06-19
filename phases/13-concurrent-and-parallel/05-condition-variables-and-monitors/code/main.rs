// Condition Variables and Monitors
// Phase 13 — Concurrent & Parallel Computing
//
// Build It, Steps 3 & 4:
//   3. Monitor Pattern — struct with Mutex + Condvar, safe API.
//      The monitor encapsulates locking so callers cannot forget to signal.
//   4. Multiple Conditions — bounded queue with separate "has data" /
//      "has space" Condvars.  Producers and consumers each wait on their
//      own CV, avoiding the thundering-herd problem.
//
// Run:  rustc main.rs && ./main     (or  cargo run)

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

// ==================================================================
//  Step 3 — Monitor Pattern: Unbounded Channel
// ==================================================================
//
// A monitor is a concurrency abstraction that bundles:
//   1. Shared state (protected by a mutex)
//   2. One or more condition variables
//   3. Methods that expose a safe API
//
// The key advantage: callers of send() and recv() cannot forget to lock
// the mutex or signal the CV.  The monitor's internal methods guarantee
// the protocol.
//
// Rust's Condvar::wait consumes the MutexGuard, so you *cannot* access
// the shared data while the thread is asleep — the guard is gone.
// When wait returns, the guard is re-acquired and returned as a new
// value.  This is the "Guard pattern."

pub struct Channel<T> {
    items: Mutex<Vec<T>>,
    ready: Condvar,
}

impl<T> Channel<T> {
    pub fn new() -> Self {
        Channel {
            items: Mutex::new(Vec::new()),
            ready: Condvar::new(),
        }
    }

    /// Send an item — pushes it into the queue and notifies one waiter.
    ///
    /// The signal happens *after* the push, while still holding the lock.
    /// The waker won't run until we release the lock (Mesa semantics).
    pub fn send(&self, msg: T) {
        let mut guard = self.items.lock().unwrap();
        guard.push(msg);
        // NOTE: notify_one is a no-op if nobody is waiting — that is fine.
        // The item is in the queue; the next recv() call will find it.
        self.ready.notify_one();
    }

    /// Receive an item — blocks until one is available.
    ///
    /// Uses the canonical while-loop pattern even though Condvar::wait
    /// tries to handle spurious wakeups.  The while loop covers:
    ///   1. Spurious wakeups — the OS may wake us for no reason.
    ///   2. Mesa semantics — another consumer may have grabbed the item
    ///      between the signal and our re-acquisition of the lock.
    ///
    /// Note how `guard` is rebound: wait() consumes the old guard and
    /// returns a new one with the lock re-acquired.
    pub fn recv(&self) -> T {
        let mut guard = self.items.lock().unwrap();
        while guard.is_empty() {
            // wait() consumes the guard, atomically unlocks + sleeps.
            // On wake it re-acquires the lock and returns a new guard.
            guard = self.ready.wait(guard).unwrap();
        }
        // guard is re-acquired — safe to access the vector.
        guard.remove(0)
    }
}

/// Drop implementation: nothing special needed since Mutex and Condvar
/// both implement Drop.  The monitor is RAII-clean.
impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        // In a production channel you might want to notify waiters that
        // the channel is closed so they can exit instead of blocking
        // forever.  For this lesson we accept the simple behavior.
    }
}

fn step3_monitor_pattern() {
    println!("=== Step 3: Monitor Pattern (Rust) ===");

    // A single-producer, single-consumer test.
    let chan = Arc::new(Channel::new());
    let mut handles = vec![];

    // Consumer — will block on recv() until the producer sends.
    let c = chan.clone();
    handles.push(thread::spawn(move || {
        println!("[Consumer] waiting for data...");
        let val = c.recv();
        println!("[Consumer] received {}", val);

        let val2 = c.recv();
        println!("[Consumer] received {}", val2);
    }));

    // Producer — sends two items with a delay.
    let c = chan.clone();
    handles.push(thread::spawn(move || {
        thread::sleep(Duration::from_millis(200));
        c.send(42);
        println!("[Producer] sent 42");

        thread::sleep(Duration::from_millis(100));
        c.send(99);
        println!("[Producer] sent 99");
    }));

    for h in handles {
        h.join().unwrap();
    }
    println!("[Monitor Pattern] OK — both items received.\n");
}

// ==================================================================
//  Step 4 — Multiple Conditions: Bounded Queue
// ==================================================================
//
// The bounded queue uses TWO condition variables:
//
//   can_read  — signaled when data becomes available (not empty)
//   can_write — signaled when space becomes available (not full)
//
// Producers wait on can_write; consumers wait on can_read.  Using
// separate CVs means a consumer never accidentally wakes a producer
// and vice versa.  This is more efficient than a single CV that
// requires broadcast (which wakes *everyone*, most of whom must
// immediately go back to sleep).
//
// The monitor pattern means all locking is internal: callers just
// call push() and pop().  They cannot forget to signal.

pub struct BoundedQueue<T> {
    inner: Mutex<Inner<T>>,
    can_read: Condvar,
    can_write: Condvar,
}

struct Inner<T> {
    data: VecDeque<T>,
    capacity: usize,
}

impl<T> BoundedQueue<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "BoundedQueue capacity must be at least 1");
        BoundedQueue {
            inner: Mutex::new(Inner {
                data: VecDeque::with_capacity(capacity),
                capacity,
            }),
            can_read: Condvar::new(),
            can_write: Condvar::new(),
        }
    }

    /// Push an item into the queue, blocking if the queue is full.
    ///
    /// The while-loop handles spurious wakeups and Mesa semantics:
    /// even though we waited on can_write, another producer may have
    /// filled the buffer before we re-acquired the lock.
    pub fn push(&self, item: T) {
        let mut guard = self.inner.lock().unwrap();

        while guard.data.len() == guard.capacity {
            // Wait on the "space available" CV.
            guard = self.can_write.wait(guard).unwrap();
        }

        guard.data.push_back(item);

        // Wake ONE consumer that there's data to read.
        // (notify_all would wake ALL consumers — thundering herd)
        self.can_read.notify_one();
    }

    /// Pop an item from the queue, blocking if the queue is empty.
    pub fn pop(&self) -> T {
        let mut guard = self.inner.lock().unwrap();

        while guard.data.is_empty() {
            // Wait on the "data available" CV.
            guard = self.can_read.wait(guard).unwrap();
        }

        let item = guard.data.pop_front().unwrap();

        // Wake ONE producer that there's space to write.
        self.can_write.notify_one();
        item
    }

    /// Non-blocking try-pop.  Returns None if the queue is empty.
    /// Useful when you want to poll without blocking (e.g., in a UI
    /// event loop).
    pub fn try_pop(&self) -> Option<T> {
        let mut guard = self.inner.lock().unwrap();
        let item = guard.data.pop_front();
        if item.is_some() {
            self.can_write.notify_one();
        }
        item
    }

    /// Return the current length without blocking.
    pub fn len(&self) -> usize {
        let guard = self.inner.lock().unwrap();
        guard.data.len()
    }

    /// Return the capacity.
    pub fn capacity(&self) -> usize {
        let guard = self.inner.lock().unwrap();
        guard.capacity
    }

    /// Block until the queue has at least `n` items, then return them
    /// as a batch.  This reduces lock acquisitions for bulk consumers.
    pub fn pop_batch(&self, n: usize) -> Vec<T> {
        let mut guard = self.inner.lock().unwrap();
        // Wait until we have at least n items.
        while guard.data.len() < n {
            guard = self.can_read.wait(guard).unwrap();
        }
        let mut batch: Vec<T> = Vec::with_capacity(n);
        for _ in 0..n {
            batch.push(guard.data.pop_front().unwrap());
        }
        // Signal producers that space freed up.
        self.can_write.notify_one();
        batch
    }
}

fn step4_multiple_conditions() {
    println!("=== Step 4: Multiple Conditions (BoundedQueue) ===");

    let queue = Arc::new(BoundedQueue::new(3)); // tiny capacity → pressure
    let mut handles = vec![];

    // ==============================================================
    //  2 producers, each pushing 5 items  (10 total)
    // ==============================================================
    for id in 0..2 {
        let q = queue.clone();
        handles.push(thread::spawn(move || {
            for i in 0..5 {
                let val = id * 100 + i;
                q.push(val);
                println!("[Producer {}] pushed {}", id, val);
                thread::sleep(Duration::from_millis(10));
            }
            println!("[Producer {}] done", id);
        }));
    }

    // ==============================================================
    //  3 consumers, each popping 4 or 3 items  (10 total)
    // ==============================================================
    for id in 0..3 {
        let q = queue.clone();
        let count = if id == 0 { 4 } else { 3 };
        handles.push(thread::spawn(move || {
            for _ in 0..count {
                let val = q.pop();
                println!("[Consumer {}] popped {}", id, val);
                thread::sleep(Duration::from_millis(15));
            }
            println!("[Consumer {}] done", id);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
    println!("[BoundedQueue] All 10 items produced and consumed.");

    // ==============================================================
    //  Demonstrate try_pop and pop_batch
    // ==============================================================
    println!("\n--- try_pop and pop_batch demo ---");

    let q = BoundedQueue::new(10);

    // Queue starts empty → try_pop returns None.
    assert!(q.try_pop().is_none());
    println!("try_pop on empty queue: None (correct)");

    // Push 5 items.
    for i in 0..5 {
        q.push(i);
    }
    println!("After pushing 5 items, len = {}", q.len());

    // try_pop now succeeds.
    let val = q.try_pop().unwrap();
    println!("try_pop returned Some({})", val);
    println!("len after try_pop = {}", q.len());

    // pop_batch(3) — blocks until 3 items available (they already are).
    let batch = q.pop_batch(3);
    println!("pop_batch(3) returned: {:?}", batch);
    println!("len after pop_batch = {}", q.len());

    println!("\n[BoundedQueue] All extra demos passed.\n");
}

// ==================================================================
//  Main
// ==================================================================

fn main() {
    println!("=== Condition Variables and Monitors (Rust) ===\n");

    step3_monitor_pattern();
    step4_multiple_conditions();

    println!("=== Summary ===");
    println!("  Step 3: Monitor pattern — safe send/recv API with Condvar.");
    println!("           Guard pattern: wait() consumes + returns MutexGuard.");
    println!("  Step 4: BoundedQueue with separate can_read / can_write Condvars.");
    println!("           try_pop() and pop_batch() for flexible use.");
    println!("  Key rule: ALWAYS loop, even with Rust's type-safe Condvar.");
}
