//! main.rs — stacks & queues, Rust standard-library and a hand-rolled ring buffer.
//!
//! Build: `rustc -O main.rs && ./main`

use std::collections::VecDeque;
use std::time::Instant;

/// Hand-rolled power-of-2 ring buffer.
pub struct RingBuf {
    buf: Vec<i32>,
    head: usize,
    tail: usize,
    mask: usize,
    len: usize,
}

impl RingBuf {
    pub fn new(min_cap: usize) -> Self {
        let mut cap = 1;
        while cap < min_cap { cap *= 2; }
        RingBuf {
            buf: vec![0; cap],
            head: 0, tail: 0, mask: cap - 1, len: 0,
        }
    }

    pub fn push(&mut self, x: i32) {
        if self.len == self.mask + 1 { self.grow(); }
        self.buf[self.tail] = x;
        self.tail = (self.tail + 1) & self.mask;
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<i32> {
        if self.len == 0 { return None; }
        let x = self.buf[self.head];
        self.head = (self.head + 1) & self.mask;
        self.len -= 1;
        Some(x)
    }

    fn grow(&mut self) {
        let new_cap = (self.mask + 1) * 2;
        let mut new_buf = vec![0i32; new_cap];
        for i in 0..self.len {
            new_buf[i] = self.buf[(self.head + i) & self.mask];
        }
        self.buf = new_buf;
        self.head = 0;
        self.tail = self.len;
        self.mask = new_cap - 1;
    }
}

fn bench(label: &str, f: impl FnOnce()) {
    let t = Instant::now();
    f();
    println!("  {:<28} {:>10.3} ms", label, t.elapsed().as_secs_f64() * 1000.0);
}

fn main() {
    let n = 1_000_000;
    let w = 10_000;

    println!("== Stack: Vec<i32> push/pop x {n} ==");
    bench("Vec::push + Vec::pop", || {
        let mut s: Vec<i32> = Vec::with_capacity(n);
        for i in 0..n as i32 { s.push(i); }
        while let Some(_) = s.pop() {}
    });

    println!("\n== Rolling-window queue (W={w}, R={n}) ==");
    bench("VecDeque (std ring buffer)", || {
        let mut q: VecDeque<i32> = (0..w as i32).collect();
        for i in 0..n as i32 {
            q.push_back(i);
            q.pop_front();
        }
    });
    bench("RingBuf (hand-rolled)", || {
        let mut q = RingBuf::new(w);
        for i in 0..w as i32 { q.push(i); }
        for i in 0..n as i32 {
            q.push(i);
            q.pop();
        }
    });
}
