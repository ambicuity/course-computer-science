//! main.rs — std HashMap and a hand-rolled Robin Hood map for comparison.
//!
//! Build: `rustc -O main.rs && ./main`

use std::collections::HashMap;
use std::time::Instant;

// splitmix64
fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

// ---------- Robin Hood hash map ----------
#[derive(Clone, Copy)]
struct Slot { key: u64, val: i64, occupied: bool, dist: u32 }

pub struct RobinMap {
    slots: Vec<Slot>,
    cap: usize,
    len: usize,
}

impl RobinMap {
    pub fn new() -> Self {
        let cap = 16;
        RobinMap {
            slots: vec![Slot { key: 0, val: 0, occupied: false, dist: 0 }; cap],
            cap, len: 0,
        }
    }

    fn idx(&self, key: u64) -> usize { (mix64(key) as usize) & (self.cap - 1) }

    pub fn put(&mut self, key: u64, val: i64) {
        if self.len * 10 > self.cap * 9 { self.resize(); }
        let mut i = self.idx(key);
        let mut incoming = Slot { key, val, occupied: true, dist: 0 };
        loop {
            if !self.slots[i].occupied {
                self.slots[i] = incoming;
                self.len += 1;
                return;
            }
            if self.slots[i].key == incoming.key {
                self.slots[i].val = incoming.val;
                return;
            }
            if self.slots[i].dist < incoming.dist {
                std::mem::swap(&mut self.slots[i], &mut incoming);
            }
            incoming.dist += 1;
            i = (i + 1) & (self.cap - 1);
        }
    }

    pub fn get(&self, key: u64) -> Option<i64> {
        let mut i = self.idx(key);
        let mut dist: u32 = 0;
        loop {
            if !self.slots[i].occupied || self.slots[i].dist < dist { return None; }
            if self.slots[i].key == key { return Some(self.slots[i].val); }
            dist += 1;
            i = (i + 1) & (self.cap - 1);
        }
    }

    fn resize(&mut self) {
        let old = std::mem::take(&mut self.slots);
        let old_cap = self.cap;
        self.cap = old_cap * 2;
        self.slots = vec![Slot { key: 0, val: 0, occupied: false, dist: 0 }; self.cap];
        self.len = 0;
        for s in old {
            if s.occupied { self.put(s.key, s.val); }
        }
    }
}

fn main() {
    let n = 200_000;
    let keys: Vec<u64> = {
        let mut s: u64 = 12345;
        (0..n).map(|_| { s = mix64(s); s }).collect()
    };

    println!("== {n} inserts + {n} lookups ==\n");

    // std::collections::HashMap (SipHash)
    let t0 = Instant::now();
    let mut h: HashMap<u64, i64> = HashMap::with_capacity(n);
    for (i, &k) in keys.iter().enumerate() { h.insert(k, i as i64); }
    let t_ins = t0.elapsed();
    let t0 = Instant::now();
    let mut sum: i64 = 0;
    for &k in &keys { if let Some(v) = h.get(&k) { sum += v; } }
    let t_lk = t0.elapsed();
    println!("std HashMap (SipHash): insert {:>7.1?}  lookup {:>7.1?}  checksum={}",
             t_ins, t_lk, sum);

    // Robin Hood (no DoS resistance, just mix64)
    let t0 = Instant::now();
    let mut r = RobinMap::new();
    for (i, &k) in keys.iter().enumerate() { r.put(k, i as i64); }
    let t_ins = t0.elapsed();
    let t0 = Instant::now();
    let mut sum: i64 = 0;
    for &k in &keys { if let Some(v) = r.get(k) { sum += v; } }
    let t_lk = t0.elapsed();
    println!("RobinMap (mix64)     : insert {:>7.1?}  lookup {:>7.1?}  checksum={}",
             t_ins, t_lk, sum);
}
