//! main.rs — Use Rust's std::collections::BTreeMap (a production B-tree with m=6).
//! Re-implementing B-trees in safe Rust is tedious; in production you use BTreeMap directly.

use std::collections::BTreeMap;
use std::time::Instant;

fn main() {
    println!("== Rust's std::collections::BTreeMap (m=6 B-tree) ==\n");

    let mut m: BTreeMap<i32, String> = BTreeMap::new();
    for k in [10, 20, 30, 5, 15, 25, 35, 1, 7, 12, 17, 22, 27, 32, 37, 40, 45, 50] {
        m.insert(k, format!("v{}", k));
    }

    println!("get(17) = {:?}", m.get(&17));
    println!("get(99) = {:?}", m.get(&99));

    // Range query (BTreeMap's native API).
    let range: Vec<_> = m.range(15..=35).collect();
    println!("range [15, 35] = {:?}", range);

    // Bench against HashMap on a large workload.
    let n = 500_000;
    let mut bt: BTreeMap<i32, i32> = BTreeMap::new();
    let t0 = Instant::now();
    for i in 0..n { bt.insert(i, i * 2); }
    let t_bt_ins = t0.elapsed();

    let t0 = Instant::now();
    let mut sum: i64 = 0;
    for i in 0..n { if let Some(&v) = bt.get(&i) { sum += v as i64; } }
    let t_bt_get = t0.elapsed();

    let t0 = Instant::now();
    let mut count = 0;
    for (_, v) in bt.range(100_000..200_000) { count += 1; sum += *v as i64; }
    let t_bt_range = t0.elapsed();

    println!("\nBench (n={}):", n);
    println!("  BTreeMap insert : {:>7.1?}  ({:.1} ns/op)", t_bt_ins, t_bt_ins.as_secs_f64()*1e9/n as f64);
    println!("  BTreeMap get    : {:>7.1?}  ({:.1} ns/op)", t_bt_get, t_bt_get.as_secs_f64()*1e9/n as f64);
    println!("  BTreeMap range  : {:>7.1?}  ({} keys, range scan is the B-tree's killer feature)",
             t_bt_range, count);
    println!("  sink: {}", sum);
}
