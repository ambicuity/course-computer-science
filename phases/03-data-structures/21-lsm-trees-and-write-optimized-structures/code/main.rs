//! main.rs — toy LSM: memtable + sorted SSTables + per-SSTable Bloom.

use std::collections::BTreeMap;

fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

pub struct Bloom { bits: Vec<u8>, m: usize, k: usize }

impl Bloom {
    pub fn new(m: usize, k: usize) -> Self { Bloom { bits: vec![0u8; (m + 7) / 8], m, k } }
    pub fn add(&mut self, x: u64) {
        for i in 0..self.k {
            let h = (mix64(x.wrapping_add(i as u64 * 0xdeadbeef)) as usize) % self.m;
            self.bits[h / 8] |= 1 << (h & 7);
        }
    }
    pub fn check(&self, x: u64) -> bool {
        (0..self.k).all(|i| {
            let h = (mix64(x.wrapping_add(i as u64 * 0xdeadbeef)) as usize) % self.m;
            (self.bits[h / 8] >> (h & 7)) & 1 == 1
        })
    }
}

pub struct SSTable { entries: Vec<(u64, i64)>, bloom: Bloom }

impl SSTable {
    pub fn from_map(m: &BTreeMap<u64, i64>) -> Self {
        let entries: Vec<(u64, i64)> = m.iter().map(|(&k, &v)| (k, v)).collect();
        let mut bloom = Bloom::new(10 * entries.len(), 7);
        for &(k, _) in &entries { bloom.add(k); }
        SSTable { entries, bloom }
    }

    pub fn get(&self, k: u64) -> Option<i64> {
        if !self.bloom.check(k) { return None; }
        match self.entries.binary_search_by_key(&k, |&(k, _)| k) {
            Ok(i) => Some(self.entries[i].1),
            Err(_) => None,
        }
    }
}

pub struct LSM {
    memtable: BTreeMap<u64, i64>,
    ssts: Vec<SSTable>,
    memtable_limit: usize,
}

impl LSM {
    pub fn new(limit: usize) -> Self { LSM { memtable: BTreeMap::new(), ssts: vec![], memtable_limit: limit } }

    pub fn put(&mut self, k: u64, v: i64) {
        self.memtable.insert(k, v);
        if self.memtable.len() >= self.memtable_limit {
            self.flush();
        }
    }

    pub fn flush(&mut self) {
        if self.memtable.is_empty() { return; }
        let sst = SSTable::from_map(&self.memtable);
        self.ssts.insert(0, sst);                          // newest first
        self.memtable.clear();
    }

    pub fn get(&self, k: u64) -> Option<i64> {
        if let Some(&v) = self.memtable.get(&k) { return Some(v); }
        for sst in &self.ssts {
            if let Some(v) = sst.get(k) { return Some(v); }
        }
        None
    }
}

fn main() {
    let mut l = LSM::new(1000);
    for i in 0..10_000u64 { l.put(i * 7 + 1, i as i64); }
    l.flush();
    println!("LSM: {} SSTables", l.ssts.len());

    let mut hits = 0;
    for i in 0..10_000u64 {
        if l.get(i * 7 + 1) == Some(i as i64) { hits += 1; }
    }
    println!("reads: {hits} / 10000");
    println!("miss test: get(99999) = {:?}", l.get(99999));
}
