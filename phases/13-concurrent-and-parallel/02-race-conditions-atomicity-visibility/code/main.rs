//! Race Conditions, Atomicity, Visibility
//! Phase 13 — Concurrent & Parallel Computing
//!
//! Race condition demos in Rust:
//!   1. Arc<Mutex<>> counter (correct, but slower)
//!   2. Arc<AtomicUsize> counter with Relaxed ordering
//!   3. Arc<AtomicUsize> counter with SeqCst ordering
//!   4. Visibility demo — acquire/release semantics
//!   5. Attempted data race (compile error — shown in comments)
//!
//! Run: rustc main.rs && ./main
//! Or:  cargo run  (if in a cargo project)

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const NUM_INCREMENTS: usize = 1_000_000;
const NUM_THREADS: usize = 2;

/* ────────────────────────────────────────────────────────
   Demo 1 — Arc<Mutex<>> Counter
   ────────────────────────────────────────────────────────
   Rust's type system forces us to use a Mutex (or similar)
   when sharing mutable state across threads. There is no
   way to write a data race in safe Rust.
   ──────────────────────────────────────────────────────── */

fn demo_mutex_counter() {
    println!("=== Demo 1: Arc<Mutex<>> Counter ===");
    let counter = Arc::new(Mutex::new(0_usize));
    let mut handles = vec![];

    for _ in 0..NUM_THREADS {
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..NUM_INCREMENTS {
                *c.lock().unwrap() += 1;
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("  Expected: {}", NUM_INCREMENTS * NUM_THREADS);
    println!("  Actual:   {}", *counter.lock().unwrap());
    println!("  (Mutex ensures mutual exclusion)\n");
}

/* ────────────────────────────────────────────────────────
   Demo 2 — AtomicUsize with Relaxed Ordering
   ────────────────────────────────────────────────────────
   fetch_add with Relaxed ordering guarantees atomicity
   of the counter itself, but no happens-before edges
   for other memory operations. For a simple counter
   where we only care about the final value being correct,
   this is sufficient. On x86, Relaxed compiles to the
   same `lock xadd` as SeqCst.
   ──────────────────────────────────────────────────────── */

fn demo_atomic_counter_relaxed() {
    println!("=== Demo 2: AtomicUsize (Relaxed) ===");
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for _ in 0..NUM_THREADS {
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..NUM_INCREMENTS {
                c.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("  Expected: {}", NUM_INCREMENTS * NUM_THREADS);
    println!("  Actual:   {}", counter.load(Ordering::Relaxed));
    println!("  (Atomic RMW prevents lost updates)\n");
}

/* ────────────────────────────────────────────────────────
   Demo 3 — AtomicUsize with SeqCst Ordering
   ────────────────────────────────────────────────────────
   SeqCst provides the strongest ordering guarantees.
   All SeqCst operations form a single total order that
   all threads agree on. This is the default ordering
   in C++ atomics and is usually what you want unless
   you have measured a performance need for weaker.
   ──────────────────────────────────────────────────────── */

fn demo_atomic_counter_seqcst() {
    println!("=== Demo 3: AtomicUsize (SeqCst) ===");
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for _ in 0..NUM_THREADS {
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..NUM_INCREMENTS {
                c.fetch_add(1, Ordering::SeqCst);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("  Expected: {}", NUM_INCREMENTS * NUM_THREADS);
    println!("  Actual:   {}", counter.load(Ordering::SeqCst));
    println!("  (SeqCst — strongest ordering, globally consistent)\n");
}

/* ────────────────────────────────────────────────────────
   Demo 4 — Visibility with Acquire/Release
   ────────────────────────────────────────────────────────
   Producer writes data (Relaxed), then sets flag (Release).
   Consumer waits on flag (Acquire), then reads data (Relaxed).
   The release-acquire pair establishes happens-before:
   the data write is guaranteed visible to the consumer.
   ──────────────────────────────────────────────────────── */

fn demo_visibility_ordering() {
    println!("=== Demo 4: Visibility — Acquire/Release ===");

    let ready = Arc::new(AtomicBool::new(false));
    let data = Arc::new(AtomicUsize::new(0));

    // Producer
    let r = Arc::clone(&ready);
    let d = Arc::clone(&data);
    let producer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        d.store(42, Ordering::Relaxed);
        r.store(true, Ordering::Release);
        println!("  producer: wrote data=42, set ready=true");
    });

    // Consumer
    let r = Arc::clone(&ready);
    let d = Arc::clone(&data);
    let consumer = thread::spawn(move || {
        while !r.load(Ordering::Acquire) {
            /* spin */
        }
        let val = d.load(Ordering::Relaxed);
        println!("  consumer: read data = {}", val);
        assert_eq!(val, 42, "Acquire/release guarantees visibility!");
    });

    producer.join().unwrap();
    consumer.join().unwrap();
    println!("  (Acquire/release guarantees data=42 is visible)\n");
}

/* ────────────────────────────────────────────────────────
   Demo 5 (commentary only) — Attempted Data Race
   ────────────────────────────────────────────────────────
   The following code would NOT compile in Rust.
   Uncomment it to see the compiler errors:

   #[allow(unused)]
   fn attempt_data_race() {
       let mut counter = 0;
       thread::spawn(move || {
           counter += 1;  // error[E0382]: use of moved value: `counter`
       });
   }

   Even with Arc (without interior mutability):
   #[allow(unused)]
   fn attempt_arc_race() {
       let counter = Arc::new(0_usize);
       let c = Arc::clone(&counter);
       thread::spawn(move || {
           // error: cannot assign to data in an Arc
           // *c += 1;
       });
   }

   Rust's ownership + Send/Sync traits guarantee that
   a data race is impossible in safe Rust. This is the
   key advantage over C/C++: the compiler catches it.
   ──────────────────────────────────────────────────────── */

fn demo_compile_time_safety() {
    println!("=== Demo 5: Rust's Compile-Time Data Race Prevention ===");
    println!("  // This does NOT compile in safe Rust:");
    println!("  let mut counter = 0;");
    println!("  thread::spawn(move || { counter += 1; });");
    println!("  // error[E0382]: use of moved value: `counter`");
    println!();
    println!("  Rust forces you to choose a synchronization primitive:");
    println!("  - Arc<Mutex<T>>    — mutual exclusion");
    println!("  - Arc<AtomicX>     — lock-free atomics");
    println!("  - Arc<RwLock<T>>   — read-write lock");
    println!("  - Arc<Barrier>     — rendezvous");
    println!();
    println!("  This is why Rust is the only systems language that");
    println!("  eliminates data races at compile time.\n");
}

/* ────────────────────────────────────────────────────────
   main
   ──────────────────────────────────────────────────────── */

fn main() {
    println!("══════════════════════════════════════════════");
    println!("  Race Conditions, Atomicity, Visibility");
    println!("══════════════════════════════════════════════\n");

    demo_mutex_counter();
    demo_atomic_counter_relaxed();
    demo_atomic_counter_seqcst();
    demo_visibility_ordering();
    demo_compile_time_safety();

    println!("All demos complete.");
}
