//! Join Algorithms — Nested Loop, Hash, Sort-Merge.
//! Phase 10 — Databases & Storage Systems.
//!
//! Implements Simple NLJ, Block NLJ, Grace Hash Join,
//! Hybrid Hash Join, Sort-Merge Join, and a benchmark
//! comparing algorithms on different input sizes.

use std::collections::HashMap;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Tuple representation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
struct Record {
    id: usize,
    key: u64,
    name: String,
}

fn generate_relation(name: &str, n: usize, key_range: u64) -> Vec<Record> {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let mut records = Vec::with_capacity(n);
    let mut rng = RandomState::new();
    for i in 0..n {
        let mut hasher = rng.build_hasher();
        hasher.write_usize(i);
        let key = hasher.finish() % key_range;
        records.push(Record {
            id: i,
            key,
            name: format!("{}_{}", name, i),
        });
    }
    records
}

// ---------------------------------------------------------------------------
// Trait: JoinAlgorithm
// ---------------------------------------------------------------------------

trait JoinAlgorithm {
    fn name(&self) -> &str;
    fn join(&self, r: &[Record], s: &[Record]) -> Vec<(Record, Record)>;
}

// ---------------------------------------------------------------------------
// 1. Simple Nested Loop Join
// ---------------------------------------------------------------------------

struct SimpleNestedLoopJoin;

impl JoinAlgorithm for SimpleNestedLoopJoin {
    fn name(&self) -> &str {
        "Simple NLJ"
    }

    fn join(&self, r: &[Record], s: &[Record]) -> Vec<(Record, Record)> {
        let mut result = Vec::new();
        for r_rec in r {
            for s_rec in s {
                if r_rec.key == s_rec.key {
                    result.push((r_rec.clone(), s_rec.clone()));
                }
            }
        }
        result
    }
}

// ---------------------------------------------------------------------------
// 2. Block Nested Loop Join
// ---------------------------------------------------------------------------

struct BlockNestedLoopJoin {
    page_size: usize,
}

impl BlockNestedLoopJoin {
    fn new(page_size: usize) -> Self {
        Self { page_size }
    }

    fn paginate(&self, rel: &[Record]) -> Vec<&[Record]> {
        rel.chunks(self.page_size).collect()
    }
}

impl JoinAlgorithm for BlockNestedLoopJoin {
    fn name(&self) -> &str {
        "Block NLJ"
    }

    fn join(&self, r: &[Record], s: &[Record]) -> Vec<(Record, Record)> {
        let r_pages = self.paginate(r);
        let s_pages = self.paginate(s);
        let mut result = Vec::new();
        for r_page in &r_pages {
            for s_page in &s_pages {
                for r_rec in *r_page {
                    for s_rec in *s_page {
                        if r_rec.key == s_rec.key {
                            result.push((r_rec.clone(), s_rec.clone()));
                        }
                    }
                }
            }
        }
        result
    }
}

// ---------------------------------------------------------------------------
// 3. Index Nested Loop Join (simulated with hash map)
// ---------------------------------------------------------------------------

struct IndexNestedLoopJoin;

impl JoinAlgorithm for IndexNestedLoopJoin {
    fn name(&self) -> &str {
        "Index NLJ"
    }

    fn join(&self, r: &[Record], s: &[Record]) -> Vec<(Record, Record)> {
        let mut idx: HashMap<u64, Vec<&Record>> = HashMap::new();
        for s_rec in s {
            idx.entry(s_rec.key).or_default().push(s_rec);
        }
        let mut result = Vec::new();
        for r_rec in r {
            if let Some(matches) = idx.get(&r_rec.key) {
                for s_rec in matches {
                    result.push((r_rec.clone(), (*s_rec).clone()));
                }
            }
        }
        result
    }
}

// ---------------------------------------------------------------------------
// 4. Grace Hash Join
// ---------------------------------------------------------------------------

struct GraceHashJoin {
    num_partitions: usize,
}

impl GraceHashJoin {
    fn new(num_partitions: usize) -> Self {
        Self { num_partitions }
    }
}

impl JoinAlgorithm for GraceHashJoin {
    fn name(&self) -> &str {
        "Grace Hash"
    }

    fn join(&self, r: &[Record], s: &[Record]) -> Vec<(Record, Record)> {
        let n = self.num_partitions;
        let mut part_r: Vec<Vec<&Record>> = (0..n).map(|_| Vec::new()).collect();
        let mut part_s: Vec<Vec<&Record>> = (0..n).map(|_| Vec::new()).collect();

        for rec in r {
            let p = (rec.key as usize) % n;
            part_r[p].push(rec);
        }
        for rec in s {
            let p = (rec.key as usize) % n;
            part_s[p].push(rec);
        }

        let mut result = Vec::new();
        for i in 0..n {
            // Choose smaller side as build
            let (build, probe) = if part_r[i].len() <= part_s[i].len() {
                (&part_r[i], &part_s[i])
            } else {
                (&part_s[i], &part_r[i])
            };
            let mut ht: HashMap<u64, Vec<&Record>> = HashMap::new();
            for rec in build {
                ht.entry(rec.key).or_default().push(rec);
            }
            for rec in probe {
                if let Some(matches) = ht.get(&rec.key) {
                    for m in matches {
                        result.push(((*rec).clone(), (*m).clone()));
                    }
                }
            }
        }
        result
    }
}

// ---------------------------------------------------------------------------
// 5. Hybrid Hash Join
// ---------------------------------------------------------------------------

struct HybridHashJoin {
    num_partitions: usize,
}

impl HybridHashJoin {
    fn new(num_partitions: usize) -> Self {
        Self { num_partitions }
    }
}

impl JoinAlgorithm for HybridHashJoin {
    fn name(&self) -> &str {
        "Hybrid Hash"
    }

    fn join(&self, r: &[Record], s: &[Record]) -> Vec<(Record, Record)> {
        let n = self.num_partitions;
        // Partition 0 stays in memory; rest go to "disk" (Vec)
        let mut kept_r: Vec<&Record> = Vec::new();
        let mut kept_s: Vec<&Record> = Vec::new();
        let mut disk_r: Vec<Vec<&Record>> = (0..n - 1).map(|_| Vec::new()).collect();
        let mut disk_s: Vec<Vec<&Record>> = (0..n - 1).map(|_| Vec::new()).collect();

        for rec in r {
            let p = (rec.key as usize) % n;
            if p == 0 {
                kept_r.push(rec);
            } else {
                disk_r[p - 1].push(rec);
            }
        }
        for rec in s {
            let p = (rec.key as usize) % n;
            if p == 0 {
                kept_s.push(rec);
            } else {
                disk_s[p - 1].push(rec);
            }
        }

        let mut result = Vec::new();

        // Process kept partition (in-memory, no I/O)
        let (build, probe) = if kept_r.len() <= kept_s.len() {
            (&kept_r, &kept_s)
        } else {
            (&kept_s, &kept_r)
        };
        let mut ht: HashMap<u64, Vec<&Record>> = HashMap::new();
        for rec in build {
            ht.entry(rec.key).or_default().push(rec);
        }
        for rec in probe {
            if let Some(matches) = ht.get(&rec.key) {
                for m in matches {
                    result.push(((*rec).clone(), (*m).clone()));
                }
            }
        }

        // Process disk partitions
        for i in 0..n - 1 {
            let (build, probe) = if disk_r[i].len() <= disk_s[i].len() {
                (&disk_r[i], &disk_s[i])
            } else {
                (&disk_s[i], &disk_r[i])
            };
            let mut ht: HashMap<u64, Vec<&Record>> = HashMap::new();
            for rec in build {
                ht.entry(rec.key).or_default().push(rec);
            }
            for rec in probe {
                if let Some(matches) = ht.get(&rec.key) {
                    for m in matches {
                        result.push(((*rec).clone(), (*m).clone()));
                    }
                }
            }
        }

        result
    }
}

// ---------------------------------------------------------------------------
// 6. Sort-Merge Join
// ---------------------------------------------------------------------------

struct SortMergeJoin;

impl JoinAlgorithm for SortMergeJoin {
    fn name(&self) -> &str {
        "Sort-Merge"
    }

    fn join(&self, r: &[Record], s: &[Record]) -> Vec<(Record, Record)> {
        let mut r_sorted = r.to_vec();
        let mut s_sorted = s.to_vec();
        r_sorted.sort_by_key(|rec| rec.key);
        s_sorted.sort_by_key(|rec| rec.key);

        let mut result = Vec::new();
        let mut i = 0;
        let mut j = 0;

        while i < r_sorted.len() && j < s_sorted.len() {
            let rk = r_sorted[i].key;
            let sk = s_sorted[j].key;
            if rk == sk {
                let j_start = j;
                while j < s_sorted.len() && s_sorted[j].key == rk {
                    j += 1;
                }
                let mut k = i;
                while k < r_sorted.len() && r_sorted[k].key == rk {
                    for m in j_start..j {
                        result.push((r_sorted[k].clone(), s_sorted[m].clone()));
                    }
                    k += 1;
                }
                i = k;
            } else if rk < sk {
                i += 1;
            } else {
                j += 1;
            }
        }
        result
    }
}

// ---------------------------------------------------------------------------
// Correctness verification
// ---------------------------------------------------------------------------

fn verify_correctness() -> bool {
    let r = generate_relation("R", 20, 5);
    let s = generate_relation("S", 15, 5);

    let algos: Vec<Box<dyn JoinAlgorithm>> = vec![
        Box::new(SimpleNestedLoopJoin),
        Box::new(BlockNestedLoopJoin::new(4)),
        Box::new(IndexNestedLoopJoin),
        Box::new(GraceHashJoin::new(4)),
        Box::new(HybridHashJoin::new(4)),
        Box::new(SortMergeJoin),
    ];

    // Reference: simple NLJ
    let reference = SimpleNestedLoopJoin.join(&r, &s);

    let mut all_pass = true;
    println!("--- Correctness (20 x 15, key range 5) ---");
    for algo in &algos {
        let result = algo.join(&r, &s);
        // Compare as sets of (id, id) pairs
        let ref_pairs: std::collections::HashSet<(usize, usize)> = reference
            .iter()
            .map(|(a, b)| (a.id, b.id))
            .collect();
        let res_pairs: std::collections::HashSet<(usize, usize)> = result
            .iter()
            .map(|(a, b)| (a.id, b.id))
            .collect();
        let ok = ref_pairs == res_pairs;
        println!(
            "  {:15s}: {:4d} rows  {}",
            algo.name(),
            result.len(),
            if ok { "✓" } else { "✗ MISMATCH" }
        );
        if !ok {
            all_pass = false;
        }
    }
    all_pass
}

// ---------------------------------------------------------------------------
// Benchmark
// ---------------------------------------------------------------------------

fn run_benchmark() {
    println!("\n--- Benchmark ---");
    let sizes = [
        ("Tiny   ", 100, 80, 20),
        ("Small  ", 500, 300, 50),
        ("Medium ", 2000, 1000, 100),
    ];

    for (label, r_size, s_size, key_range) in &sizes {
        let r = generate_relation("R", *r_size, *key_range);
        let s = generate_relation("S", *s_size, *key_range);

        println!(
            "\n  {} R={} S={} key_range={}",
            label, r_size, s_size, key_range
        );

        let algos: Vec<Box<dyn JoinAlgorithm>> = vec![
            Box::new(SimpleNestedLoopJoin),
            Box::new(BlockNestedLoopJoin::new(4)),
            Box::new(IndexNestedLoopJoin),
            Box::new(GraceHashJoin::new(4)),
            Box::new(HybridHashJoin::new(4)),
            Box::new(SortMergeJoin),
        ];

        for algo in &algos {
            let start = Instant::now();
            let result = algo.join(&r, &s);
            let elapsed = start.elapsed();
            println!(
                "    {:15s}: {:7} rows in {:>10?}",
                algo.name(),
                result.len(),
                elapsed
            );
        }
    }
}

// ---------------------------------------------------------------------------
// I/O Cost Estimation (simplified textbook model)
// ---------------------------------------------------------------------------

fn estimate_costs(
    pages_r: f64, pages_s: f64,
    tuples_r: f64, _tuples_s: f64,
    memory_pages: f64,
    has_index: bool,
) {
    let b = memory_pages.max(1.0);

    let simple = pages_r + tuples_r * pages_s;
    let block_size = (b - 1.0).max(1.0);
    let block = pages_r + (pages_r / block_size).ceil() * pages_s;
    let index = if has_index {
        pages_r + tuples_r * 3.0
    } else {
        f64::INFINITY
    };
    let grace = 3.0 * (pages_r + pages_s);
    let hybrid = grace - 2.0 * (pages_r + pages_s) / b.min(16.0);

    fn sort_cost(p: f64, b: f64) -> f64 {
        if p <= b { 2.0 * p } else {
            let passes = (p.log(b)).ceil();
            2.0 * p * (1.0 + passes)
        }
    }
    let sm = sort_cost(pages_r, b) + sort_cost(pages_s, b) + pages_r + pages_s;

    println!("\n  Cost estimates (page I/Os):");
    println!("    Simple NLJ:   {:>12,.0}", simple);
    println!("    Block NLJ:    {:>12,.0}", block);
    println!("    Index NLJ:    {:>12,.0}", index);
    println!("    Grace Hash:   {:>12,.0}", grace);
    println!("    Hybrid Hash:  {:>12,.0}", hybrid);
    println!("    Sort-Merge:   {:>12,.0}", sm);

    let costs = [
        ("Simple NLJ", simple),
        ("Block NLJ", block),
        ("Index NLJ", index),
        ("Grace Hash", grace),
        ("Hybrid Hash", hybrid),
        ("Sort-Merge", sm),
    ];
    let best = costs.iter().min_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap();
    println!("  Optimal: {} ({:.0} I/Os)", best.0, best.1);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    println!("{}", "=".repeat(72));
    println!("Join Algorithm Benchmark (Rust)");
    println!("{}", "=".repeat(72));

    verify_correctness();
    run_benchmark();

    println!("\n--- Cost Estimation ---");
    let scenarios = [
        ("Tiny",    5.0,   3.0,   100.0,   60.0),
        ("Small",  50.0,  20.0,  1000.0,  500.0),
        ("Medium", 500.0, 200.0, 10000.0, 5000.0),
    ];
    for (name, pr, ps, tr, ts) in &scenarios {
        println!("\n--- {} (R: {} pages, {} tuples | S: {} pages, {} tuples) ---",
                 name, pr, tr, ps, ts);
        estimate_costs(*pr, *ps, *tr, *ts, 256.0, false);
    }
}
