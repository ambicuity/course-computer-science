//! Streaming Algorithms — Frequency, Quantiles, HyperLogLog
//! Phase 04 — Algorithms & Complexity Analysis
//!
//! HyperLogLog and Count-Min Sketch implementations in Rust.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

// ---------------------------------------------------------------------------
// HyperLogLog
// ---------------------------------------------------------------------------

/// HyperLogLog cardinality estimator.
///
/// Uses `m = 2^p` registers.  Relative error ≈ 1.04 / √m.
/// Space: m bytes (1 byte per register, max leading-zero count ≤ 64).
pub struct HyperLogLog {
    p: u8,
    m: usize,
    registers: Vec<u8>,
    alpha: f64,
}

impl HyperLogLog {
    pub fn new(p: u8) -> Self {
        let m = 1usize << p;
        let alpha = if m >= 128 {
            0.7213 / (1.0 + 1.079 / m as f64)
        } else if m >= 64 {
            0.7093 / (1.0 + 1.079 / m as f64)
        } else if m >= 32 {
            0.697 / (1.0 + 1.079 / m as f64)
        } else {
            0.673
        };
        Self {
            p,
            m,
            registers: vec![0u8; m],
            alpha,
        }
    }

    /// Hash an element to a u64 using std's default hasher.
    fn hash_element<T: Hash>(element: &T) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        element.hash(&mut hasher);
        hasher.finish()
    }

    /// rho(w) = position of the least significant 1-bit + 1.
    fn rho(w: u64) -> u8 {
        if w == 0 {
            65
        } else {
            (w ^ (w - 1)).leading_zeros() as u8 + 1
        }
    }

    /// Insert an element into the sketch.
    pub fn add<T: Hash>(&mut self, element: &T) {
        let x = Self::hash_element(element);
        let j = (x as usize) & (self.m - 1);
        let w = x >> self.p;
        let rho = Self::rho(w);
        if rho > self.registers[j] {
            self.registers[j] = rho;
        }
    }

    /// Return the estimated number of distinct elements.
    pub fn count(&self) -> u64 {
        let z_inv: f64 = self
            .registers
            .iter()
            .map(|&r| 2.0f64.powi(-(r as i32)))
            .sum();
        let raw = self.alpha * (self.m as f64) * (self.m as f64) / z_inv;

        // Small-range correction
        let estimate = if raw <= 2.5 * self.m as f64 {
            let v = self.registers.iter().filter(|&&r| r == 0).count();
            if v > 0 {
                (self.m as f64) * ((self.m as f64) / v as f64).ln()
            } else {
                raw
            }
        } else {
            raw
        };

        estimate as u64
    }
}

// ---------------------------------------------------------------------------
// Count-Min Sketch
// ---------------------------------------------------------------------------

/// Count-Min Sketch with ε-δ guarantees.
///
/// Space: O(w · d) where w = ⌈e/ε⌉, d = ⌈ln(1/δ)⌉.
pub struct CountMinSketch {
    w: usize,
    d: usize,
    counters: Vec<Vec<u64>>,
    seeds: Vec<u64>,
    total: u64,
}

impl CountMinSketch {
    pub fn new(epsilon: f64, delta: f64) -> Self {
        let w = (std::f64::consts::E / epsilon).ceil() as usize;
        let d = (1.0_f64 / delta).ln().ceil() as usize;
        let counters = vec![vec![0u64; w]; d];
        // Deterministic seeds from index
        let seeds: Vec<u64> = (0..d)
            .map(|i| {
                let mut h = std::collections::hash_map::DefaultHasher::new();
                (i as u64 + 0x9E3779B97F4A7C15).hash(&mut h);
                h.finish()
            })
            .collect();
        Self {
            w,
            d,
            counters,
            seeds,
            total: 0,
        }
    }

    fn _hash(element: &str, seed: u64, w: usize) -> usize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        seed.hash(&mut hasher);
        element.hash(&mut hasher);
        (hasher.finish() as usize) % w
    }

    pub fn add(&mut self, element: &str) {
        self.total += 1;
        for i in 0..self.d {
            let j = Self::_hash(element, self.seeds[i], self.w);
            self.counters[i][j] += 1;
        }
    }

    pub fn estimate(&self, element: &str) -> u64 {
        (0..self.d)
            .map(|i| {
                let j = Self::_hash(element, self.seeds[i], self.w);
                self.counters[i][j]
            })
            .min()
            .unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Demo
// ---------------------------------------------------------------------------

fn random_url() -> String {
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz".chars().collect();
    let path: String = (0..8)
        .map(|_| chars[rand_usize() % chars.len()])
        .collect();
    format!("https://example.com/{}", path)
}

/// Simple xorshift PRNG (no external crate dependency).
fn rand_usize() -> usize {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u64> = Cell::new(0xDEAD_BEEF_CAFE_1234);
    }
    STATE.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        x as usize
    })
}

fn demo_hyperloglog() {
    println!("{}", "=".repeat(60));
    println!("HyperLogLog");
    println!("{}", "=".repeat(60));

    let mut exact: HashMap<String, bool> = HashMap::new();
    let mut hll_14 = HyperLogLog::new(14);
    let n = 500_000;

    for _ in 0..n {
        let url = random_url();
        exact.insert(url.clone(), true);
        hll_14.add(&url);
    }

    let exact_count = exact.len();
    let est = hll_14.count();
    let err = (est as f64 - exact_count as f64).abs() / exact_count as f64 * 100.0;
    let expected = 104.0 / (1u64 << 14) as f64.sqrt();

    println!("Exact: {} unique URLs", exact_count);
    println!("HLL (p=14): {} estimated", est);
    println!("Error: {:.2}%  (expected ~{:.2}%)", err, expected);
    println!();
}

fn demo_count_min_sketch() {
    println!("{}", "=".repeat(60));
    println!("Count-Min Sketch");
    println!("{}", "=".repeat(60));

    let mut cms = CountMinSketch::new(0.001, 0.01);
    let mut exact: HashMap<String, u64> = HashMap::new();

    for _ in 0..500_000 {
        let url = random_url();
        cms.add(&url);
        *exact.entry(url).or_default() += 1;
    }

    // Show top-5 elements
    let mut sorted: Vec<_> = exact.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    println!(
        "Sketch: {} rows x {} cols = {} counters",
        cms.d,
        cms.w,
        cms.d * cms.w
    );
    println!(
        "{:<20} {:>8} {:>8} {:>8}",
        "Element", "Exact", "Estimate", "Error"
    );
    println!("{}", "-".repeat(46));
    for (elem, true_count) in sorted.iter().take(5) {
        let est = cms.estimate(elem);
        let err = est as i64 - *true_count as i64;
        println!("{:<20} {:>8} {:>8} {:>+8}", elem, true_count, est, err);
    }
    println!();
}

fn main() {
    demo_hyperloglog();
    demo_count_min_sketch();
}
