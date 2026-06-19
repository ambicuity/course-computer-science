//! Parallel Patterns — Map, Reduce, Pipeline, Scan
//! Phase 13 — Concurrent & Parallel Computing
//!
//! Demonstrates four fundamental parallel patterns:
//!   1. Map — embarrassingly parallel element-wise transformation
//!   2. Reduce — parallel associative combination (tree reduction)
//!   3. Pipeline — task-parallel stages communicating via channels
//!   4. Scan (Prefix Sum) — parallel prefix computation (Hillis-Steele & Blelloch)
//!
//! Run: cargo run --release  (from this directory)

use rayon::prelude::*;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

const SMALL: usize = 100_000;
const MEDIUM: usize = 1_000_000;
const LARGE: usize = 10_000_000;

/// ========================================================================
/// STEP 1 — Parallel Map
///
/// Map applies a function f to every element of an array:
///   output[i] = f(input[i])
///
/// This is "embarrassingly parallel": each element is independent,
/// no communication, no synchronization.
///
/// Work:       W = n (same as sequential)
/// Span:       T = O(1) given enough processors
/// Speedup:    S ≤ P (perfect linear speedup achievable)
/// ========================================================================

fn step1_parallel_map() {
    println!("=== Step 1: Parallel Map ===");

    let data: Vec<i64> = (0..MEDIUM).map(|i| i as i64).collect();
    let f = |x: &i64| x * x + 2 * x + 1;

    let start = Instant::now();
    let seq: Vec<i64> = data.iter().map(f).collect();
    let seq_time = start.elapsed();

    let start = Instant::now();
    let par: Vec<i64> = data.par_iter().map(f).collect();
    let par_time = start.elapsed();

    assert_eq!(seq, par);
    println!("  f(x)=x²+2x+1, n={MEDIUM}");
    println!("  Sequential: {seq_time:?}  Parallel: {par_time:?}  Speedup: {:.2}x",
             seq_time.as_secs_f64() / par_time.as_secs_f64());

    let data: Vec<f64> = (0..LARGE).map(|i| i as f64).collect();
    let heavy_f = |x: &f64| x.sin().cos().atan().powf(1.5);

    let start = Instant::now();
    let _: Vec<f64> = data.iter().map(heavy_f).collect();
    let seq_heavy = start.elapsed();

    let start = Instant::now();
    let _: Vec<f64> = data.par_iter().map(heavy_f).collect();
    let par_heavy = start.elapsed();

    println!("  Heavy f(x)=sin(cos(atan(x)))¹·⁵, n={LARGE}");
    println!("  Sequential: {seq_heavy:?}  Parallel: {par_heavy:?}  Speedup: {:.2}x",
             seq_heavy.as_secs_f64() / par_heavy.as_secs_f64());
    println!();
}

/// ========================================================================
/// STEP 2 — Parallel Reduce
///
/// Reduce combines all elements with an associative binary operator:
///   result = input[0] ⊕ input[1] ⊕ ... ⊕ input[n-1]
///
/// Parallel reduction builds a tree:
///   Level 0: n elements  → n/2 partial results
///   Level 1: n/2         → n/4
///   ...
///   Level log₂n:         → 1 result
///
/// Work:       W = n-1 (same as sequential — tree doesn't add work)
/// Span:       T = O(log n)
/// Speedup:    S = W/T = (n-1)/log₂n
/// ========================================================================

fn step2_parallel_reduce() {
    println!("=== Step 2: Parallel Reduce ===");

    let data: Vec<i64> = (0..LARGE).map(|i| i as i64).collect();

    let start = Instant::now();
    let par_sum: i64 = data.par_iter().sum();
    let par_time = start.elapsed();

    let start = Instant::now();
    let seq_sum: i64 = data.iter().sum();
    let seq_time = start.elapsed();

    assert_eq!(par_sum, seq_sum);
    println!("  Sum (n={LARGE}): seq={seq_time:?} par={par_time:?} speedup={:.2}x",
             seq_time.as_secs_f64() / par_time.as_secs_f64());

    let par_min = data.par_iter().min().unwrap();
    let par_max = data.par_iter().max().unwrap();
    println!("  Min: {par_min}, Max: {par_max} (parallel)");

    let data2: Vec<i64> = (0..LARGE).rev().map(|i| i as i64).collect();
    let start = Instant::now();
    let dot: i64 = data.par_iter()
        .zip(data2.par_iter())
        .map(|(a, b)| a * b)
        .sum();
    let par_dot = start.elapsed();

    let start = Instant::now();
    let dot_seq: i64 = data.iter().zip(data2.iter()).map(|(a, b)| a * b).sum();
    let seq_dot = start.elapsed();

    assert_eq!(dot, dot_seq);
    println!("  Dot product: par={par_dot:?} seq={seq_dot:?} speedup={:.2}x",
             seq_dot.as_secs_f64() / par_dot.as_secs_f64());

    let small: Vec<i64> = (0..100_000).collect();
    let start = Instant::now();
    let manual: i64 = small.par_iter()
        .fold(|| 0i64, |a, &b| a + b)
        .reduce(|| 0i64, |a, b| a + b);
    let manual_time = start.elapsed();
    let builtin: i64 = small.par_iter().sum();
    assert_eq!(manual, builtin);
    println!("  fold+reduce: {manual_time:?}  built-in sum: equal (correctness OK)");
    println!();
}

/// ========================================================================
/// STEP 3 — Pipeline
///
/// A pipeline connects stages where each stage processes data and
/// passes results to the next. Stages run concurrently, each on its
/// own thread, communicating via channels.
///
/// This is task parallelism (different operations on different data)
/// as opposed to data parallelism (same operation on different data).
///
/// Latency:     sum of all stage latencies
/// Throughput:  1 / max(stage latency)
/// Work:        sum of work per stage per item × n items
/// Span:        n × max(Lⱼ) (bottleneck dominates)
/// ========================================================================

fn step3_pipeline() {
    println!("=== Step 3: Pipeline ===");

    let items = 10_000;

    let (tx1, rx1) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();
    let (tx3, rx3) = mpsc::channel();

    let s1 = thread::spawn(move || {
        for i in 0..items {
            tx1.send(i).unwrap();
        }
        drop(tx1);
        println!("  Stage 1 (producer): sent {items} items");
    });

    let s2 = thread::spawn(move || {
        let mut count = 0;
        for val in rx1 {
            if val % 2 == 0 {
                tx2.send(val).unwrap();
                count += 1;
            }
        }
        drop(tx2);
        println!("  Stage 2 (filter): forwarded {count} even items");
    });

    let s3 = thread::spawn(move || {
        let mut count = 0;
        for val in rx2 {
            tx3.send(val * 2).unwrap();
            count += 1;
        }
        drop(tx3);
        println!("  Stage 3 (doubler): processed {count} items");
    });

    let s4 = thread::spawn(move || {
        let results: Vec<i32> = rx3.iter().collect();
        let sum: i32 = results.iter().sum();
        let expected: i32 = (0..items).filter(|i| i % 2 == 0).map(|i| i * 2).sum();
        assert_eq!(sum, expected, "Pipeline output mismatch!");
        println!("  Stage 4 (collector): {} items, sum={sum}, correct!",
                 results.len());
        results
    });

    s1.join().unwrap();
    s2.join().unwrap();
    s3.join().unwrap();
    let _results = s4.join().unwrap();
    println!();
}

/// ========================================================================
/// STEP 4 — Parallel Prefix Scan
///
/// Inclusive scan: output[i] = input[0] ⊕ input[1] ⊕ ... ⊕ input[i]
///
/// Two classic parallel algorithms with different work–span trade-offs:
///
/// Hillis-Steele:
///   W = n log₂ n  (work-inefficient — does redundant computation)
///   T = log₂ n    (fast span)
///   Good for small-to-medium arrays on many-core machines.
///
/// Blelloch:
///   W = 2n         (work-efficient — only 2× sequential work)
///   T = 2 log₂ n   (slightly longer span, better for large n)
///   Preferred for GPU implementations and large arrays.
/// ========================================================================

fn hillis_steele_inclusive(input: &[i64]) -> Vec<i64> {
    let n = input.len();
    if n <= 1 {
        return input.to_vec();
    }

    let mut old = input.to_vec();
    let mut new = vec![0i64; n];
    let mut d = 1_usize;

    while d < n {
        new.par_iter_mut()
            .enumerate()
            .for_each(|(i, v)| {
                *v = old[i];
                if i >= d {
                    *v += old[i - d];
                }
            });
        std::mem::swap(&mut old, &mut new);
        d <<= 1;
    }

    old
}

/// Blelloch exclusive scan (work-efficient).
/// Returns exclusive prefix sum: output[i] = sum(input[0..i]); output[0] = 0.
fn blelloch_exclusive(input: &[i64]) -> Vec<i64> {
    if input.is_empty() {
        return vec![];
    }
    if input.len() == 1 {
        return vec![0];
    }

    let n = input.len().next_power_of_two();
    let mut data: Vec<i64> = input.iter().copied()
        .chain(std::iter::repeat(0i64))
        .take(n)
        .collect();

    // Phase 1: Up-sweep (reduce tree)
    let log2n = (n as f64).log2() as usize;
    for d in 0..log2n {
        let stride = 1 << (d + 1);
        let half = 1 << d;
        data.par_chunks_mut(stride).for_each(|chunk| {
            debug_assert_eq!(chunk.len(), stride);
            let (first, rest) = chunk.split_at_mut(half);
            rest[stride - half - 1] += first[half - 1];
        });
    }

    // Phase 2: Down-sweep (distribute)
    data[n - 1] = 0;
    for d in (0..log2n).rev() {
        let stride = 1 << (d + 1);
        let half = 1 << d;
        data.par_chunks_mut(stride).for_each(|chunk| {
            debug_assert_eq!(chunk.len(), stride);
            let (first, rest) = chunk.split_at_mut(half);
            let left = &mut first[half - 1];
            let right = &mut rest[stride - half - 1];
            let lv = *left;
            let rv = *right;
            *left = rv;
            *right += lv;
        });
    }

    data.truncate(input.len());
    data
}

fn step4_parallel_scan() {
    println!("=== Step 4: Parallel Prefix Scan ===");

    let n = SMALL;
    let input: Vec<i64> = (1..=n as i64).collect();

    let start = Instant::now();
    let seq: Vec<i64> = input.iter().scan(0i64, |acc, &x| {
        *acc += x;
        Some(*acc)
    }).collect();
    let seq_time = start.elapsed();

    let start = Instant::now();
    let hs = hillis_steele_inclusive(&input);
    let hs_time = start.elapsed();
    assert_eq!(hs, seq, "Hillis-Steele result mismatch!");

    let start = Instant::now();
    let bl_excl = blelloch_exclusive(&input);
    let bl_time = start.elapsed();
    let bl: Vec<i64> = bl_excl.iter()
        .zip(input.iter())
        .map(|(&p, &x)| p + x)
        .collect();
    assert_eq!(bl, seq, "Blelloch result mismatch!");

    println!("  n = {n}");
    println!("  Sequential:  {seq_time:?}  (baseline)");
    println!("  Hillis-Steele: {hs_time:?}  (W=n log n, T=log n)");
    println!("  Blelloch:    {bl_time:?}  (W=2n, T=2 log n)");

    if hs_time < seq_time {
        println!("  Hillis-Steele speedup: {:.2}x",
                 seq_time.as_secs_f64() / hs_time.as_secs_f64());
    }
    if bl_time < seq_time {
        println!("  Blelloch speedup: {:.2}x",
                 seq_time.as_secs_f64() / bl_time.as_secs_f64());
    }
    println!("  Correctness: ✓ both algorithms match sequential scan");
    println!();
}

/// ========================================================================
/// STEP 5 — Benchmark Suite
///
/// Compare sequential vs parallel speedup across all patterns at
/// multiple data sizes.
/// ========================================================================

fn run_benchmark() {
    println!("=== Benchmark: All Patterns ===");
    println!();
    println!("  {:<15} {:<10} {:<12} {:<12} {:<8}", "Pattern", "Size", "Seq (s)", "Par (s)", "Speedup");
    println!("  {}", "-".repeat(60));

    for &size in &[SMALL, MEDIUM] {
        let data: Vec<i64> = (0..size as i64).collect();

        // Map
        let t0 = Instant::now();
        let _: Vec<i64> = data.iter().map(|&x| x * x).collect();
        let t_seq = t0.elapsed();
        let t0 = Instant::now();
        let _: Vec<i64> = data.par_iter().map(|&x| x * x).collect();
        let t_par = t0.elapsed();
        let sp = t_seq.as_secs_f64() / t_par.as_secs_f64();
        println!("  {:<15} {:<10} {:<12.6} {:<12.6} {:<7.2}x",
                 "Map", size, t_seq.as_secs_f64(), t_par.as_secs_f64(), sp);

        // Reduce
        let t0 = Instant::now();
        let _: i64 = data.iter().sum();
        let t_seq = t0.elapsed();
        let t0 = Instant::now();
        let _: i64 = data.par_iter().sum();
        let t_par = t0.elapsed();
        let sp = t_seq.as_secs_f64() / t_par.as_secs_f64();
        println!("  {:<15} {:<10} {:<12.6} {:<12.6} {:<7.2}x",
                 "Reduce (sum)", size, t_seq.as_secs_f64(), t_par.as_secs_f64(), sp);

        // Scan (Hillis-Steele)
        let t0 = Instant::now();
        let _: Vec<i64> = data.iter().scan(0i64, |acc, &x| { *acc += x; Some(*acc) }).collect();
        let t_seq = t0.elapsed();
        let t0 = Instant::now();
        let _ = hillis_steele_inclusive(&data);
        let t_par = t0.elapsed();
        let sp = t_seq.as_secs_f64() / t_par.as_secs_f64();
        println!("  {:<15} {:<10} {:<12.6} {:<12.6} {:<7.2}x",
                 "Scan (HS)", size, t_seq.as_secs_f64(), t_par.as_secs_f64(), sp);
    }
    println!();
}

fn main() {
    println!("═══════════════════════════════════════════════════");
    println!("  Parallel Patterns — Map, Reduce, Pipeline, Scan");
    println!("═══════════════════════════════════════════════════\n");

    step1_parallel_map();
    step2_parallel_reduce();
    step3_pipeline();
    step4_parallel_scan();
    run_benchmark();

    println!("All demos complete.");
}
