use std::sync::atomic::{AtomicU64, Ordering};

struct LockFreeCounter {
    v: AtomicU64,
}

impl LockFreeCounter {
    fn new() -> Self {
        Self { v: AtomicU64::new(0) }
    }

    fn inc(&self) -> u64 {
        self.v.fetch_add(1, Ordering::Relaxed) + 1
    }

    fn get(&self) -> u64 {
        self.v.load(Ordering::Relaxed)
    }
}

fn main() {
    let c = LockFreeCounter::new();
    let mut last = c.get();
    for _ in 0..1000 {
        let now = c.inc();
        assert!(now >= last);
        last = now;
    }
    println!("final={}", c.get());
}
