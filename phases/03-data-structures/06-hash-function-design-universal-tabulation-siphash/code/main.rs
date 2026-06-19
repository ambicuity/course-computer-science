//! main.rs — Rust's default Hasher (SipHash) and a fast FxHash for comparison.
//!
//! Build: `rustc -O main.rs && ./main`

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Instant;

/// FxHash from rustc / Firefox. ~2x faster than SipHash on x86-64 but NOT DOS-resistant.
#[derive(Default)]
struct FxHasher { h: u64 }

const FX_SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

impl Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.h = (self.h.rotate_left(5) ^ b as u64).wrapping_mul(FX_SEED);
        }
    }
    fn write_u64(&mut self, x: u64) {
        self.h = (self.h.rotate_left(5) ^ x).wrapping_mul(FX_SEED);
    }
    fn finish(&self) -> u64 { self.h }
}

fn bench<H: Hasher + Default>(label: &str, n: u64) {
    let t0 = Instant::now();
    let mut sink: u64 = 0;
    for i in 0..n {
        let mut h = H::default();
        i.hash(&mut h);
        sink ^= h.finish();
    }
    let dt = t0.elapsed();
    println!("  {:<22}  {:>5.1} ns/hash  ({} sink)", label,
             dt.as_secs_f64() * 1e9 / n as f64,
             sink & 0xffff);
}

fn main() {
    println!("== Hasher throughput (8-byte input, 10M iters) ==");
    bench::<DefaultHasher>("DefaultHasher (SipHash)", 10_000_000);
    bench::<FxHasher>("FxHasher",                10_000_000);

    println!("\n== Quick avalanche sanity ==");
    // Flip bit 0 of input; count output-bit changes.
    let probe = |mut h: Box<dyn Hasher>, x: u64| -> u64 {
        h.write_u64(x);
        h.finish()
    };
    let h1_a = probe(Box::new(DefaultHasher::new()), 0x1234_5678_9abc_def0);
    let h1_b = probe(Box::new(DefaultHasher::new()), 0x1234_5678_9abc_def1);
    let h2_a = probe(Box::new(FxHasher::default()),  0x1234_5678_9abc_def0);
    let h2_b = probe(Box::new(FxHasher::default()),  0x1234_5678_9abc_def1);
    println!("  SipHash flipped bits:  {}/64", (h1_a ^ h1_b).count_ones());
    println!("  FxHash  flipped bits:  {}/64", (h2_a ^ h2_b).count_ones());
}
