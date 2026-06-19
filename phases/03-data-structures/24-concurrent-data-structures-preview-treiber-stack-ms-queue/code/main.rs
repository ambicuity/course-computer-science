//! main.rs — Treiber stack in Rust using AtomicPtr.
//!
//! NB: We leak popped nodes intentionally (no hazard pointers). Production
//! lock-free in Rust: use `crossbeam_epoch` for safe memory reclamation.

use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

pub struct Node { value: i32, next: *mut Node }

pub struct Treiber { head: AtomicPtr<Node> }

unsafe impl Send for Treiber {}
unsafe impl Sync for Treiber {}

impl Treiber {
    pub fn new() -> Self { Treiber { head: AtomicPtr::new(ptr::null_mut()) } }

    pub fn push(&self, value: i32) {
        let n = Box::into_raw(Box::new(Node { value, next: ptr::null_mut() }));
        let mut old = self.head.load(Ordering::Relaxed);
        loop {
            unsafe { (*n).next = old; }
            match self.head.compare_exchange_weak(old, n, Ordering::Release, Ordering::Relaxed) {
                Ok(_) => return,
                Err(observed) => old = observed,
            }
        }
    }

    pub fn pop(&self) -> Option<i32> {
        let mut old = self.head.load(Ordering::Acquire);
        loop {
            if old.is_null() { return None; }
            let next = unsafe { (*old).next };
            match self.head.compare_exchange_weak(old, next, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => {
                    let v = unsafe { (*old).value };
                    /* LEAK: hazard pointers omitted for clarity */
                    return Some(v);
                }
                Err(observed) => old = observed,
            }
        }
    }
}

fn main() {
    use std::sync::Arc;
    use std::thread;

    let s = Arc::new(Treiber::new());
    let n_threads = 4;
    let n_ops = 100_000;

    let mut handles = vec![];
    for tid in 0..n_threads {
        let s = Arc::clone(&s);
        handles.push(thread::spawn(move || {
            let mut popped_sum: i64 = 0;
            let mut pushed_sum: i64 = 0;
            for i in 0..n_ops {
                let v = (tid as i32) * 1_000_000 + i;
                s.push(v);
                pushed_sum += v as i64;
                if let Some(p) = s.pop() { popped_sum += p as i64; }
            }
            while let Some(p) = s.pop() { popped_sum += p as i64; }
            pushed_sum - popped_sum
        }));
    }
    let diff: i64 = handles.into_iter().map(|h| h.join().unwrap()).sum();
    println!("checksum diff = {diff} (should be 0)");
}
