use std::hint::black_box;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct BenchResult {
    name: String,
    min_ns: f64,
    median_ns: f64,
    mean_ns: f64,
    p99_ns: f64,
    max_ns: f64,
    stddev_ns: f64,
    iterations: usize,
    warmup: usize,
}

fn compute_stats(name: &str, warmup: usize, mut samples: Vec<f64>) -> BenchResult {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let n = samples.len();
    let min_val = samples[0];
    let max_val = samples[n - 1];
    let median_val = if n % 2 == 0 {
        (samples[n / 2 - 1] + samples[n / 2]) / 2.0
    } else {
        samples[n / 2]
    };
    let sum: f64 = samples.iter().sum();
    let mean_val = sum / n as f64;

    let p99_idx = 0.99 * (n as f64 - 1.0);
    let p99_lo = p99_idx.floor() as usize;
    let p99_hi = p99_idx.ceil() as usize;
    let frac = p99_idx - p99_lo as f64;
    let p99_val = samples[p99_lo] * (1.0 - frac) + samples[p99_hi] * frac;

    let variance: f64 = samples.iter().map(|s| (s - mean_val).powi(2)).sum::<f64>() / n as f64;
    let stddev_val = variance.sqrt();

    BenchResult {
        name: name.to_string(),
        min_ns: min_val,
        median_ns: median_val,
        mean_ns: mean_val,
        p99_ns: p99_val,
        max_ns: max_val,
        stddev_ns: stddev_val,
        iterations: n,
        warmup,
    }
}

fn format_ns(ns: f64) -> String {
    if ns < 1000.0 {
        format!("{:.1}ns", ns)
    } else if ns < 1_000_000.0 {
        format!("{:.2}us", ns / 1e3)
    } else {
        format!("{:.2}ms", ns / 1e6)
    }
}

fn run_benchmark<F>(name: &str, warmup: usize, measure: usize, mut f: F) -> BenchResult
where
    F: FnMut() -> i64,
{
    for _ in 0..warmup {
        black_box(f());
    }

    let mut samples = Vec::with_capacity(measure);
    for _ in 0..measure {
        let start = Instant::now();
        let result = f();
        black_box(result);
        let elapsed = start.elapsed().as_nanos() as f64;
        samples.push(elapsed);
    }

    compute_stats(name, warmup, samples)
}

fn print_header() {
    println!(
        "{:<28} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "Benchmark", "min", "median", "mean", "p99", "max", "stddev"
    );
    println!("{}", "-".repeat(88));
}

fn print_result(r: &BenchResult) {
    println!(
        "{:<28} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
        r.name,
        format_ns(r.min_ns),
        format_ns(r.median_ns),
        format_ns(r.mean_ns),
        format_ns(r.p99_ns),
        format_ns(r.max_ns),
        format_ns(r.stddev_ns)
    );
}

fn make_sequential_data(n: usize) -> Vec<i64> {
    (0..n as i64).collect()
}

fn make_random_data(n: usize, seed: u64) -> Vec<i64> {
    let mut data: Vec<i64> = (0..n as i64).collect();
    let mut rng = SimpleRng::new(seed);
    for i in (1..n).rev() {
        let j = rng.next() as usize % (i + 1);
        data.swap(i, j);
    }
    data
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
}

fn bench_sequential_access(data: &[i64]) -> i64 {
    let mut sum: i64 = 0;
    for &val in data {
        sum += black_box(val);
    }
    black_box(sum)
}

fn bench_random_access(indices: &[i64], data: &[i64]) -> i64 {
    let mut sum: i64 = 0;
    for &idx in indices {
        sum += black_box(data[idx as usize]);
    }
    black_box(sum)
}

fn bench_binary_search_sorted(data: &[i64], targets: &[i64]) -> i64 {
    let mut found: i64 = 0;
    for &t in targets {
        if data.binary_search(&t).is_ok() {
            found += 1;
        }
    }
    black_box(found)
}

fn bench_binary_search_random(random_data: &[i64], targets: &[i64]) -> i64 {
    let mut data_copy = random_data.to_vec();
    data_copy.sort();
    let mut found: i64 = 0;
    for &t in targets {
        if data_copy.binary_search(&t).is_ok() {
            found += 1;
        }
    }
    black_box(found)
}

fn demonstrate_dce() {
    println!("=== Dead Code Elimination Demo ===\n");

    let data: Vec<i64> = (0..10000).collect();

    let naive = || -> i64 {
        let mut sum: i64 = 0;
        for &val in &data {
            sum += val;
        }
        sum
    };

    let protected = || -> i64 {
        let mut sum: i64 = 0;
        for &val in &data {
            sum += black_box(val);
        }
        black_box(sum)
    };

    let r_naive = run_benchmark("naive_loop", 10, 100, naive);
    let r_protected = run_benchmark("protected_loop", 10, 100, protected);

    print_header();
    print_result(&r_naive);
    print_result(&r_protected);

    if r_naive.mean_ns < r_protected.mean_ns * 0.5 {
        println!("\nWARNING: Naive loop appears much faster — likely DCE!");
        println!("The compiler probably removed the computation because the result was unused.");
    }
    println!();
}

fn demonstrate_warm_cache() {
    println!("=== Cache Warmup Demo ===\n");

    let data = make_sequential_data(100_000);
    let indices = make_random_data(100_000, 42);

    let mut first_10: Vec<f64> = Vec::new();
    let mut later_10: Vec<f64> = Vec::new();

    for i in 0..20 {
        let start = Instant::now();
        let mut sum: i64 = 0;
        for &idx in &indices {
            sum += data[idx as usize];
        }
        black_box(sum);
        let elapsed = start.elapsed().as_nanos() as f64;
        if i < 10 {
            first_10.push(elapsed);
        } else {
            later_10.push(elapsed);
        }
    }

    let avg = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let first_avg = avg(&first_10);
    let later_avg = avg(&later_10);

    println!("First 10 iterations avg:  {:.0} ns", first_avg);
    println!("Later 10 iterations avg:  {:.0} ns", later_avg);
    println!(
        "Ratio (cold/warm):       {:.2}x",
        first_avg / later_avg
    );
    println!();
    println!("The first iterations are slower because data must be fetched from DRAM.");
    println!("After warming the cache, accesses hit L1/L2 instead.");
    println!();
}

fn main() {
    const WARMUP: usize = 20;
    const MEASURE: usize = 200;
    const DATA_SIZE: usize = 100_000;
    const SMALL_SIZE: usize = 10_000;

    println!("Measurement Discipline — Benchmarks That Don't Lie");
    println!("{}", "=".repeat(60));
    println!();

    demonstrate_dce();

    demonstrate_warm_cache();

    println!("=== Main Benchmarks ===");
    println!("Data size: {} elements", DATA_SIZE);
    println!(
        "Warmup: {} iterations, Measure: {} iterations\n",
        WARMUP, MEASURE
    );

    let seq_data = make_sequential_data(DATA_SIZE);
    let rand_indices = make_random_data(DATA_SIZE, 12345);
    let sorted_data = make_sequential_data(SMALL_SIZE);
    let shuffled_data = make_random_data(SMALL_SIZE, 999);
    let search_targets = make_random_data(SMALL_SIZE, 777);

    let mut results: Vec<BenchResult> = Vec::new();

    results.push(run_benchmark("seq_access", WARMUP, MEASURE, || {
        bench_sequential_access(&seq_data)
    }));

    results.push(run_benchmark("random_access", WARMUP, MEASURE, || {
        bench_random_access(&rand_indices, &seq_data)
    }));

    results.push(run_benchmark("bin_search_sorted", WARMUP, MEASURE, || {
        bench_binary_search_sorted(&sorted_data, &search_targets)
    }));

    results.push(run_benchmark("bin_search_random", WARMUP, MEASURE, || {
        bench_binary_search_random(&shuffled_data, &search_targets)
    }));

    print_header();
    for r in &results {
        print_result(r);
    }

    println!("\n=== Analysis ===\n");

    let seq_median = results[0].median_ns;
    let rand_median = results[1].median_ns;
    let sorted_median = results[2].median_ns;
    let rand_bs_median = results[3].median_ns;

    println!("Sequential vs Random access:");
    println!(
        "  Random is {:.1}x slower (cache + TLB effects)\n",
        rand_median / seq_median
    );

    println!("Sorted vs Random binary search:");
    println!(
        "  Random layout is {:.1}x slower (branch prediction + cache)\n",
        rand_bs_median / sorted_median
    );

    let mean_ratio = results[1].mean_ns / results[1].median_ns;
    println!(
        "Random access mean/median ratio: {:.2}",
        mean_ratio
    );
    if mean_ratio > 1.3 {
        println!("  Mean is significantly above median — heavy tail (outliers from OS noise)");
        println!("  → Report median, not mean, for typical-case performance");
    }
}