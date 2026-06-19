//! Memory Models — Sequential Consistency vs Relaxed
//! Phase 13 — Concurrent & Parallel Computing
//!
//! Litmus test suite in Rust demonstrating sequential consistency,
//! relaxed ordering, acquire/release, and memory fences.
//!
//! Run: rustc -O main.rs && ./main

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::Arc;
use std::thread;

// =============================================================================
// Part 1: Sequential Consistency — Dekker Pattern
// =============================================================================

fn part1_dekker_sc() {
    println!("=== Part 1: SC Dekker Pattern ===");
    let iterations = 50_000;

    let x = Arc::new(AtomicIsize::new(0));
    let y = Arc::new(AtomicIsize::new(0));

    let mut outcomes = [0usize; 4];

    for _ in 0..iterations {
        x.store(0, Ordering::SeqCst);
        y.store(0, Ordering::SeqCst);

        let x1 = Arc::clone(&x);
        let y1 = Arc::clone(&y);
        let t1 = thread::spawn(move || {
            x1.store(1, Ordering::SeqCst);
            y1.load(Ordering::SeqCst)
        });

        let x2 = Arc::clone(&x);
        let y2 = Arc::clone(&y);
        let t2 = thread::spawn(move || {
            y2.store(1, Ordering::SeqCst);
            x2.load(Ordering::SeqCst)
        });

        let r1 = t1.join().unwrap();
        let r2 = t2.join().unwrap();
        outcomes[(r1 * 2 + r2) as usize] += 1;
    }

    println!("SC Dekker outcomes ({} runs):", iterations);
    println!("  (0,0): {}  (should be 0)", outcomes[0]);
    println!("  (0,1): {}", outcomes[1]);
    println!("  (1,0): {}", outcomes[2]);
    println!("  (1,1): {}", outcomes[3]);

    if outcomes[0] == 0 {
        println!("  ✓ SC guarantee holds: (0,0) never observed");
    } else {
        println!("  ⚠ Unexpected (0,0) observed");
    }
}

// =============================================================================
// Part 2: Relaxed Atomics — Surprising Reordering
// =============================================================================

fn part2_relaxed_reordering() {
    println!("\n=== Part 2: Relaxed Atomics — Surprising Reordering ===");
    let iterations = 100_000;

    let a = Arc::new(AtomicIsize::new(0));
    let b = Arc::new(AtomicIsize::new(0));
    let mut observed = [0usize; 4];

    for _ in 0..iterations {
        a.store(0, Ordering::Relaxed);
        b.store(0, Ordering::Relaxed);

        let a_w = Arc::clone(&a);
        let b_w = Arc::clone(&b);
        let writer = thread::spawn(move || {
            a_w.store(1, Ordering::Relaxed);
            b_w.store(1, Ordering::Relaxed);
        });

        let a_r = Arc::clone(&a);
        let b_r = Arc::clone(&b);
        let reader = thread::spawn(move || {
            let i = b_r.load(Ordering::Relaxed);
            let j = a_r.load(Ordering::Relaxed);
            (i, j)
        });

        writer.join().unwrap();
        let (i, j) = reader.join().unwrap();
        observed[(i * 2 + j) as usize] += 1;
    }

    println!("Relaxed outcomes ({} runs):", iterations);
    println!("  (a=0,b=0): {}", observed[0]);
    println!("  (a=0,b=1): {}", observed[1]);
    println!("  (a=1,b=0): {}  (reordered!)", observed[2]);
    println!("  (a=1,b=1): {}", observed[3]);

    let pct = 100.0 * observed[2] as f64 / iterations as f64;
    println!("  → {:.1}% of runs showed reordering", pct);
}

// =============================================================================
// Part 3: Message Passing — Acquire/Release
// =============================================================================

fn part3_message_passing() {
    println!("\n=== Part 3: Message Passing with Acquire/Release ===");
    let iterations = 50_000;

    let ready = Arc::new(AtomicBool::new(false));
    let mut failures = 0usize;

    for i in 0..iterations {
        if failures >= 5 {
            println!("  (stopping early after {} failures)", failures);
            break;
        }

        let ready = Arc::new(AtomicBool::new(false));
        let data = Arc::new(AtomicIsize::new(0));

        let r_clone = Arc::clone(&ready);
        let d_clone = Arc::clone(&data);
        let producer = thread::spawn(move || {
            d_clone.store(42, Ordering::Relaxed);
            r_clone.store(true, Ordering::Release);
        });

        let r2 = Arc::clone(&ready);
        let d2 = Arc::clone(&data);
        let consumer = thread::spawn(move || {
            while !r2.load(Ordering::Acquire) {}
            let val = d2.load(Ordering::Relaxed);
            (val, val != 42)
        });

        producer.join().unwrap();
        let (val, failed) = consumer.join().unwrap();
        if failed {
            failures += 1;
            println!("  ⚠ Iteration {}: data = {}", i, val);
        }
    }

    if failures == 0 {
        println!("  ✓ Acquire/release: all iterations passed (0 failures)");
    }
}

// =============================================================================
// Part 4: Relaxed Message Passing (demonstrating the danger)
// =============================================================================

fn part4_message_passing_relaxed() {
    println!("\n=== Part 4: Message Passing with Relaxed (unsafe) ===");
    let iterations = 50_000;

    let mut failures = 0usize;

    for _ in 0..iterations {
        if failures >= 5 {
            break;
        }

        let ready = Arc::new(AtomicBool::new(false));
        let data = Arc::new(AtomicIsize::new(0));

        let r_clone = Arc::clone(&ready);
        let d_clone = Arc::clone(&data);
        let producer = thread::spawn(move || {
            d_clone.store(42, Ordering::Relaxed);
            r_clone.store(true, Ordering::Relaxed);
        });

        let r2 = Arc::clone(&ready);
        let d2 = Arc::clone(&data);
        let consumer = thread::spawn(move || {
            while !r2.load(Ordering::Relaxed) {}
            let val = d2.load(Ordering::Relaxed);
            (val, val != 42)
        });

        producer.join().unwrap();
        let (val, failed) = consumer.join().unwrap();
        if failed {
            failures += 1;
        }
    }

    if failures > 0 {
        println!("  ⚠ Failed {} times with relaxed ordering", failures);
    } else {
        println!("  No failures on this hardware (common on x86)");
    }
}

// =============================================================================
// Part 5: Memory Fences
// =============================================================================

fn part5_fences() {
    println!("\n=== Part 5: Memory Fences ===");
    let iterations = 50_000;

    let ready = Arc::new(AtomicBool::new(false));
    let mut failures = 0usize;

    for _ in 0..iterations {
        let ready = Arc::new(AtomicBool::new(false));
        let data = Arc::new(AtomicIsize::new(0));

        let r_clone = Arc::clone(&ready);
        let d_clone = Arc::clone(&data);
        let producer = thread::spawn(move || {
            d_clone.store(42, Ordering::Relaxed);
            std::sync::atomic::fence(Ordering::Release);
            r_clone.store(true, Ordering::Relaxed);
        });

        let r2 = Arc::clone(&ready);
        let d2 = Arc::clone(&data);
        let consumer = thread::spawn(move || {
            while !r2.load(Ordering::Relaxed) {}
            std::sync::atomic::fence(Ordering::Acquire);
            let val = d2.load(Ordering::Relaxed);
            (val, val != 42)
        });

        producer.join().unwrap();
        let (_, failed) = consumer.join().unwrap();
        if failed {
            failures += 1;
        }
    }

    if failures == 0 {
        println!("  ✓ Fence-based acquire/release: 0 failures");
    } else {
        println!("  ⚠ Fence failures: {}", failures);
    }
}

// =============================================================================
// Main
// =============================================================================

fn main() {
    println!("Memory Model Litmus Test Suite (Rust)");
    println!("=====================================");

    #[cfg(target_arch = "x86_64")]
    println!("Hardware: x86-64 (TSO)\n");
    #[cfg(target_arch = "aarch64")]
    println!("Hardware: ARM64 (Relaxed)\n");

    part1_dekker_sc();
    part2_relaxed_reordering();
    part3_message_passing();
    part4_message_passing_relaxed();
    part5_fences();

    println!("\nDone.");
}
