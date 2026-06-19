//! main.rs — hand-rolled binary min-heap + Rust std BinaryHeap (max).

use std::cmp::Reverse;
use std::collections::BinaryHeap;

pub struct MinHeap { a: Vec<i32> }

impl MinHeap {
    pub fn new() -> Self { MinHeap { a: vec![] } }

    pub fn push(&mut self, x: i32) {
        self.a.push(x);
        let mut i = self.a.len() - 1;
        while i > 0 {
            let p = (i - 1) / 2;
            if self.a[p] <= self.a[i] { break; }
            self.a.swap(p, i);
            i = p;
        }
    }

    pub fn pop(&mut self) -> Option<i32> {
        if self.a.is_empty() { return None; }
        let top = self.a[0];
        let last = self.a.pop().unwrap();
        if !self.a.is_empty() {
            self.a[0] = last;
            self.sift_down(0);
        }
        Some(top)
    }

    fn sift_down(&mut self, mut i: usize) {
        let n = self.a.len();
        loop {
            let l = 2 * i + 1;
            let r = 2 * i + 2;
            let mut smallest = i;
            if l < n && self.a[l] < self.a[smallest] { smallest = l; }
            if r < n && self.a[r] < self.a[smallest] { smallest = r; }
            if smallest == i { return; }
            self.a.swap(i, smallest);
            i = smallest;
        }
    }
}

fn main() {
    println!("== Hand-rolled min-heap ==");
    let mut h = MinHeap::new();
    for x in [5, 1, 9, 3, 7, 2, 8, 4, 6] { h.push(x); }
    let mut out = vec![];
    while let Some(x) = h.pop() { out.push(x); }
    println!("  pops: {:?}", out);

    println!("\n== std BinaryHeap (max-heap; use Reverse<T> for min) ==");
    let mut std_min: BinaryHeap<Reverse<i32>> = BinaryHeap::new();
    for x in [5, 1, 9, 3, 7, 2, 8, 4, 6] { std_min.push(Reverse(x)); }
    let mut out = vec![];
    while let Some(Reverse(x)) = std_min.pop() { out.push(x); }
    println!("  pops: {:?}", out);
}
