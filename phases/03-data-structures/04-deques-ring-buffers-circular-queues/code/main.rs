//! main.rs — Rust deque + lock-free SPSC ring buffer with explicit memory orderings.
//!
//! Build: `rustc -O main.rs && ./main`

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

/// Lock-free SPSC ring buffer for i32. Power-of-2 capacity.
pub struct SpscRing {
    buf: Box<[std::cell::UnsafeCell<i32>]>,
    head: AtomicUsize,
    tail: AtomicUsize,
    mask: usize,
}

unsafe impl Sync for SpscRing {}

impl SpscRing {
    pub fn new(cap: usize) -> Self {
        assert!(cap.is_power_of_two(), "cap must be a power of 2");
        let buf: Vec<std::cell::UnsafeCell<i32>> =
            (0..cap).map(|_| std::cell::UnsafeCell::new(0)).collect();
        SpscRing {
            buf: buf.into_boxed_slice(),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            mask: cap - 1,
        }
    }

    /// Producer-side. Returns false if full.
    pub fn try_push(&self, x: i32) -> bool {
        let t = self.tail.load(Ordering::Relaxed);
        let h = self.head.load(Ordering::Acquire);
        if t.wrapping_sub(h) == self.mask + 1 { return false; }
        unsafe { *self.buf[t & self.mask].get() = x; }
        self.tail.store(t.wrapping_add(1), Ordering::Release);
        true
    }

    /// Consumer-side. Returns None if empty.
    pub fn try_pop(&self) -> Option<i32> {
        let h = self.head.load(Ordering::Relaxed);
        let t = self.tail.load(Ordering::Acquire);
        if h == t { return None; }
        let x = unsafe { *self.buf[h & self.mask].get() };
        self.head.store(h.wrapping_add(1), Ordering::Release);
        Some(x)
    }
}

fn main() {
    // VecDeque demo
    println!("== VecDeque (Rust's ring-buffer deque) ==");
    let mut d: VecDeque<i32> = VecDeque::new();
    d.push_back(1); d.push_back(2); d.push_back(3);
    d.push_front(0);
    println!("  after pushes: {:?}", d);
    println!("  pop_front -> {:?} (expect Some(0))", d.pop_front());
    println!("  pop_back  -> {:?} (expect Some(3))", d.pop_back());
    println!("  remaining: {:?}", d);

    // SPSC ring
    println!("\n== Lock-free SPSC ring buffer ==");
    const N: usize = 1_000_000;
    let q = Arc::new(SpscRing::new(1024));
    let qp = Arc::clone(&q);
    let qc = Arc::clone(&q);

    let t0 = std::time::Instant::now();
    let producer = thread::spawn(move || {
        let mut i: i32 = 0;
        while (i as usize) < N {
            if qp.try_push(i) { i += 1; }
        }
    });
    let consumer = thread::spawn(move || {
        let mut sum: i64 = 0;
        let mut count = 0;
        while count < N {
            if let Some(x) = qc.try_pop() {
                sum += x as i64;
                count += 1;
            }
        }
        sum
    });
    producer.join().unwrap();
    let sum = consumer.join().unwrap();
    let elapsed = t0.elapsed();

    let expected = ((N as i64 - 1) * N as i64) / 2;
    println!("  consumed {} items in {:.3?}  ({:.1} Mitems/s)",
             N, elapsed, N as f64 / 1e6 / elapsed.as_secs_f64());
    println!("  checksum {} (expected {}) — {}",
             sum, expected, if sum == expected { "OK" } else { "MISMATCH" });
}
