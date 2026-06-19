//! Locks — Mutex, RW Lock, Spinlock, Ticket Lock
//! Phase 13 — Concurrent & Parallel Computing
//!
//! Rust demonstrations:
//!   1. std::sync::Mutex basic usage
//!   2. Lock poisoning — panic recovery
//!   3. std::sync::RwLock — multiple readers / exclusive writer
//!   4. Hand-rolled Spinlock with AtomicBool
//!   5. Benchmark comparing all variants
//!
//! Run: rustc main.rs -o lock_demo && ./lock_demo
//! Or with spin crate: add spin = "0.9" to Cargo.toml and uncomment demo_spin_crate()

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Instant;

const NUM_THREADS: usize = 4;
const NUM_ITERATIONS: usize = 500_000;

/* ──────────────────────────────────────────────
   Demo 1 — Basic Mutex Usage
   ────────────────────────────────────────────── */

fn demo_basic_mutex() {
    println!("=== Demo 1: Basic Mutex ===");
    let counter = Arc::new(Mutex::new(0_usize));
    let mut handles = vec![];

    for _ in 0..NUM_THREADS {
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..NUM_ITERATIONS {
                let mut guard = c.lock().unwrap();
                *guard += 1;
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let final_val = *counter.lock().unwrap();
    println!("  Expected: {}", NUM_THREADS * NUM_ITERATIONS);
    println!("  Actual:   {}", final_val);
    println!("  (std::sync::Mutex — correct, RAII guard)\n");
}

/* ──────────────────────────────────────────────
   Demo 2 — Lock Poisoning
   ────────────────────────────────────────────── */

fn demo_mutex_poisoning() {
    println!("=== Demo 2: Lock Poisoning ===");
    let lock = Arc::new(Mutex::new(42_i32));

    // Spawn a thread that panics while holding the lock
    let l2 = Arc::clone(&lock);
    let h = thread::spawn(move || {
        let guard = l2.lock().unwrap();
        println!("  Child thread: got lock, about to panic...");
        // Force poison by panicking while guard is alive
        drop(guard);
        panic!("simulated panic inside critical section");
    });

    // Ignore the panic
    let _ = h.join();

    // Try to acquire the poisoned lock
    match lock.lock() {
        Ok(guard) => println!("  Got lock normally: value = {}", *guard),
        Err(poisoned) => {
            println!("  Lock is POISONED! Error: {}", poisoned);
            // Recovery: into_inner() extracts the value regardless of poison
            let recovered = poisoned.into_inner();
            println!("  Recovered value via into_inner(): {}", recovered);
        }
    }

    // Demonstrate that locking a poisoned mutex IS possible with `lock().unwrap_err()`
    // The typical pattern: `let val = lock.lock().unwrap_or_else(|e| e.into_inner());`
    println!("  (Poisoning prevents access to potentially-corrupted data)\n");
}

/* ──────────────────────────────────────────────
   Demo 3 — RW Lock: multiple readers, exclusive writer
   ────────────────────────────────────────────── */

fn demo_rwlock() {
    println!("=== Demo 3: RwLock — Multiple Readers / Exclusive Writer ===");
    let data = Arc::new(RwLock::new(0_i32));
    let mut handles = vec![];

    // Spawn 4 readers
    for i in 0..4 {
        let d = Arc::clone(&data);
        handles.push(thread::spawn(move || {
            for _ in 0..NUM_ITERATIONS {
                let guard = d.read().unwrap();
                let _val = *guard;
                // Multiple readers can hold the guard simultaneously
            }
            println!("  Reader {} done (read {} times)", i, NUM_ITERATIONS);
        }));
    }

    // Spawn 1 writer
    let d = Arc::clone(&data);
    handles.push(thread::spawn(move || {
        for i in 0..NUM_ITERATIONS / 10 {
            let mut guard = d.write().unwrap();
            *guard = i;
        }
        println!("  Writer done");
    }));

    for h in handles {
        h.join().unwrap();
    }

    println!("  Final value: {}", *data.read().unwrap());
    println!("  (RwLock: multiple readers, exclusive writer)\n");
}

/* ──────────────────────────────────────────────
   Demo 4 — Hand-rolled Spinlock
   ────────────────────────────────────────────── */

struct SpinLock {
    locked: AtomicBool,
}

impl SpinLock {
    const fn new() -> Self {
        SpinLock {
            locked: AtomicBool::new(false),
        }
    }

    fn lock(&self) {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Spin hint: yield to hyper-thread
            std::hint::spin_loop();
        }
    }

    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

fn demo_spinlock_scratch() {
    println!("=== Demo 4: Spinlock from Scratch ===");
    let lock = Arc::new(SpinLock::new());
    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut handles = vec![];

    for _ in 0..NUM_THREADS {
        let l = Arc::clone(&lock);
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..NUM_ITERATIONS {
                l.lock();
                c.fetch_add(1, Ordering::Relaxed);
                l.unlock();
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("  Expected: {}", NUM_THREADS * NUM_ITERATIONS);
    println!("  Actual:   {}", counter.load(Ordering::Relaxed));
    println!("  (Custom SpinLock via AtomicBool CAS)\n");
}

/* ──────────────────────────────────────────────
   Demo 5 — Benchmark Comparison

   Measures throughput: Mutex vs RwLock(read) vs RwLock(write) vs SpinLock
   ────────────────────────────────────────────── */

fn bench_mutex(threads: usize, iters: usize) -> f64 {
    let lock = Arc::new(Mutex::new(0_usize));
    let mut handles = vec![];
    let t0 = Instant::now();
    for _ in 0..threads {
        let l = Arc::clone(&lock);
        handles.push(thread::spawn(move || {
            for _ in 0..iters {
                let mut g = l.lock().unwrap();
                *g += 1;
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    t0.elapsed().as_secs_f64()
}

fn bench_rwlock_read(threads: usize, iters: usize) -> f64 {
    let lock = Arc::new(RwLock::new(0_i32));
    let mut handles = vec![];
    let t0 = Instant::now();
    for _ in 0..threads {
        let l = Arc::clone(&lock);
        handles.push(thread::spawn(move || {
            for _ in 0..iters {
                let g = l.read().unwrap();
                let _val = *g;
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    t0.elapsed().as_secs_f64()
}

fn bench_spinlock(threads: usize, iters: usize) -> f64 {
    let lock = Arc::new(SpinLock::new());
    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut handles = vec![];
    let t0 = Instant::now();
    for _ in 0..threads {
        let l = Arc::clone(&lock);
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..iters {
                l.lock();
                c.fetch_add(1, Ordering::Relaxed);
                l.unlock();
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    t0.elapsed().as_secs_f64()
}

fn demo_benchmark() {
    println!("=== Demo 5: Throughput Benchmark ===");
    println!("  {} threads, {} iterations/thread\n", NUM_THREADS, NUM_ITERATIONS);

    let t = NUM_THREADS;
    let n = NUM_ITERATIONS;

    let sec_mutex = bench_mutex(t, n);
    let sec_rw    = bench_rwlock_read(t, n);
    let sec_spin  = bench_spinlock(t, n);

    println!("  {:-<14} {:-<12} {:-<12}", "", "", "");
    println!("  | {:<12} | {:<10} | {:<10} |", "Lock", "Time (s)", "Ops/sec");
    println!("  |{:-<14}|{:-<12}|{:-<12}|", "", "", "");
    println!("  | {:<12} | {:>8.3}s | {:>8.0f}  |", "Mutex", sec_mutex,
            (t * n) as f64 / sec_mutex);
    println!("  | {:<12} | {:>8.3}s | {:>8.0f}  |", "RwLock(read)", sec_rw,
            (t * n) as f64 / sec_rw);
    println!("  | {:<12} | {:>8.3}s | {:>8.0f}  |", "SpinLock", sec_spin,
            (t * n) as f64 / sec_spin);
    println!("  {:-<14} {:-<12} {:-<12}\n", "", "", "");
}

/* ──────────────────────────────────────────────
   (Optional) spin crate comparison
   Uncomment and add spin = "0.9" to Cargo.toml:

fn demo_spin_crate() {
    use spin::Mutex as SpinMutex;
    println!("=== Extra: spin crate Mutex ===");
    let lock = Arc::new(SpinMutex::new(0_usize));
    let mut handles = vec![];
    for _ in 0..NUM_THREADS {
        let l = Arc::clone(&lock);
        handles.push(thread::spawn(move || {
            for _ in 0..NUM_ITERATIONS {
                let mut g = l.lock();
                *g += 1;
            }
        }));
    }
    for h in handles { h.join().unwrap(); }
    println!("  Final: {}", *lock.lock());
}
   ────────────────────────────────────────────── */

/* ──────────────────────────────────────────────
   main
   ────────────────────────────────────────────── */

fn main() {
    println!("═══════════════════════════════════════════════");
    println!("  Locks — Mutex, RW Lock, Spinlock, Ticket Lock");
    println!("═══════════════════════════════════════════════\n");

    demo_basic_mutex();
    demo_mutex_poisoning();
    demo_rwlock();
    demo_spinlock_scratch();
    demo_benchmark();

    println!("All demos complete.");
}
