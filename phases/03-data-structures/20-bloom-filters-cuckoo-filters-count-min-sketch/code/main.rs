//! main.rs — Bloom filter + Count-Min sketch in Rust.

fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

pub struct Bloom { bits: Vec<u8>, m: usize, k: usize }

impl Bloom {
    pub fn new(m: usize, k: usize) -> Self {
        Bloom { bits: vec![0u8; (m + 7) / 8], m, k }
    }
    fn h(&self, x: u64, i: usize) -> usize {
        (mix64(x.wrapping_add((i as u64).wrapping_mul(0xdeadbeef))) as usize) % self.m
    }
    pub fn add(&mut self, x: u64) {
        for i in 0..self.k {
            let h = self.h(x, i);
            self.bits[h / 8] |= 1 << (h & 7);
        }
    }
    pub fn contains(&self, x: u64) -> bool {
        (0..self.k).all(|i| {
            let h = self.h(x, i);
            (self.bits[h / 8] >> (h & 7)) & 1 == 1
        })
    }
}

pub struct CountMin { t: Vec<Vec<i64>>, w: usize, d: usize }

impl CountMin {
    pub fn new(w: usize, d: usize) -> Self {
        CountMin { t: vec![vec![0i64; w]; d], w, d }
    }
    fn h(&self, x: u64, i: usize) -> usize {
        (mix64(x.wrapping_add((i as u64).wrapping_mul(0xcafef00d))) as usize) % self.w
    }
    pub fn add(&mut self, x: u64, c: i64) {
        for i in 0..self.d {
            let idx = self.h(x, i);
            self.t[i][idx] += c;
        }
    }
    pub fn estimate(&self, x: u64) -> i64 {
        (0..self.d).map(|i| self.t[i][self.h(x, i)]).min().unwrap()
    }
}

fn main() {
    let mut b = Bloom::new(96000, 7);
    let n_in: u64 = 10_000;
    let n_out: u64 = 100_000;
    for i in 0..n_in { b.add(i); }
    let fp = (n_in..n_in + n_out).filter(|&x| b.contains(x)).count();
    let fn_ = (0..n_in).filter(|&x| !b.contains(x)).count();
    let theoretical = (1.0 - (-(7.0 * n_in as f64) / 96000.0).exp()).powi(7);
    println!("Bloom (m=96000, k=7, n={n_in}):");
    println!("  false negatives: {fn_}");
    println!("  false positives: {fp} / {n_out} = {:.4}  (theoretical {theoretical:.4})",
             fp as f64 / n_out as f64);

    let mut cm = CountMin::new(256, 4);
    for _ in 0..1000 { cm.add(42, 1); }
    for i in 0..100u64 { for _ in 0..10 { cm.add(i, 1); } }
    println!("\nCount-Min:");
    println!("  estimate(42)  = {}", cm.estimate(42));
    println!("  estimate(7)   = {}", cm.estimate(7));
    println!("  estimate(999) = {}", cm.estimate(999));
}
