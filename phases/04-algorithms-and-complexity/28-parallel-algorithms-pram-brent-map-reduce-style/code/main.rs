//! Parallel Algorithms — PRAM, Brent, Map-Reduce style
//! Phase 04 — Algorithms & Complexity Analysis
//!
//! Parallel prefix sum, parallel merge sort, speedup benchmarks
//! using std::thread (no external crates).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

// ─── Parallel Prefix Sum (Blelloch scan) ──────────────────────────────────

/// Exclusive prefix sum using up-sweep + down-sweep.
///
/// Work:  O(n)
/// Depth: O(log n)
///
/// Uses std::thread to simulate PRAM concurrent access.
fn parallel_prefix_sum(arr: &[i64]) -> Vec<i64> {
    let n = arr.len();
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![0];
    }

    let m = n.next_power_of_two();
    let buf: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(
        arr.iter().copied().chain(std::iter::repeat(0)).take(m).collect(),
    ));

    // ── Up-sweep (reduce) ──
    let mut d: usize = 1;
    while d < m {
        let step = d;
        let indices: Vec<usize> = (d..m).step_by(2 * d).collect();
        let mut handles = Vec::new();
        for &i in &indices {
            let buf = Arc::clone(&buf);
            handles.push(thread::spawn(move || {
                let mut b = buf.lock().unwrap();
                b[i + step - 1] += b[i - 1];
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        d *= 2;
    }

    // Exclusive scan: set root to 0
    {
        let mut b = buf.lock().unwrap();
        b[m - 1] = 0;
    }

    // ── Down-sweep ──
    let mut d = m / 2;
    while d >= 1 {
        let step = d;
        let indices: Vec<usize> = (d..m).step_by(2 * d).collect();
        let mut handles = Vec::new();
        for &i in &indices {
            let buf = Arc::clone(&buf);
            handles.push(thread::spawn(move || {
                let mut b = buf.lock().unwrap();
                let old_left = b[i - 1];
                b[i - 1] = b[i + step - 1];
                b[i + step - 1] += old_left;
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        d /= 2;
    }

    let b = buf.lock().unwrap();
    b[..n].to_vec()
}

fn sequential_prefix_sum(arr: &[i64]) -> Vec<i64> {
    let mut out = Vec::with_capacity(arr.len());
    let mut acc: i64 = 0;
    for &x in arr {
        out.push(acc);
        acc += x;
    }
    out
}

// ─── Parallel Merge Sort ──────────────────────────────────────────────────

fn sequential_merge(left: &[i64], right: &[i64]) -> Vec<i64> {
    let mut result = Vec::with_capacity(left.len() + right.len());
    let mut i = 0;
    let mut j = 0;
    while i < left.len() && j < right.len() {
        if left[i] <= right[j] {
            result.push(left[i]);
            i += 1;
        } else {
            result.push(right[j]);
            j += 1;
        }
    }
    result.extend_from_slice(&left[i..]);
    result.extend_from_slice(&right[j..]);
    result
}

/// Fork-join parallel merge sort using std::thread.
///
/// Work:  O(n log n)
/// Depth: O(log^2 n) with parallel merge (sequential merge here → O(n) depth).
///
/// Limits recursion depth to avoid thread explosion.
fn parallel_merge_sort(arr: &[i64], depth: usize, max_parallel_depth: usize) -> Vec<i64> {
    if arr.len() <= 1 {
        return arr.to_vec();
    }

    let mid = arr.len() / 2;

    if depth < max_parallel_depth {
        let left_arr = arr[..mid].to_vec();
        let right_arr = arr[mid..].to_vec();

        let left_handle = thread::spawn(move || parallel_merge_sort(&left_arr, depth + 1, max_parallel_depth));
        let right_handle = thread::spawn(move || parallel_merge_sort(&right_arr, depth + 1, max_parallel_depth));

        let left = left_handle.join().unwrap();
        let right = right_handle.join().unwrap();

        sequential_merge(&left, &right)
    } else {
        let mut sorted = arr.to_vec();
        sorted.sort();
        sorted
    }
}

// ─── Speedup Measurement ─────────────────────────────────────────────────

fn measure_speedup<F, G, R>(label: &str, parallel_fn: F, sequential_fn: G, data: &[i64])
where
    F: Fn(&[i64]) -> R,
    G: Fn(&[i64]) -> R,
    R: PartialEq + std::fmt::Debug,
{
    let t0 = Instant::now();
    let seq_result = sequential_fn(data);
    let t_seq = t0.elapsed();

    let t0 = Instant::now();
    let par_result = parallel_fn(data);
    let t_par = t0.elapsed();

    assert_eq!(seq_result, par_result, "Results differ!");

    let speedup = if t_par.as_secs_f64() > 0.0 {
        t_seq.as_secs_f64() / t_par.as_secs_f64()
    } else {
        f64::INFINITY
    };

    println!("  [{}]", label);
    println!("    Sequential: {:.6}s", t_seq.as_secs_f64());
    println!("    Parallel:   {:.6}s", t_par.as_secs_f64());
    println!("    Speedup:    {:.2}x", speedup);
    println!();
}

// ─── Map-Reduce Style ────────────────────────────────────────────────────

/// Map-Reduce word count using threads.
///
/// Map:    each line → (word, 1) pairs  [parallel]
/// Shuffle: group by word
/// Reduce: sum counts per word          [parallel via Arc<Mutex>
fn map_reduce_word_count(lines: &[&str]) -> HashMap<String, i64> {
    // Map phase
    let mut handles = Vec::new();
    for &line in lines {
        handles.push(thread::spawn(move || {
            let mut pairs: Vec<(String, i64)> = Vec::new();
            for word in line.split_whitespace() {
                pairs.push((word.to_lowercase(), 1));
            }
            pairs
        }));
    }

    let mut all_pairs: Vec<(String, i64)> = Vec::new();
    for h in handles {
        all_pairs.extend(h.join().unwrap());
    }

    // Shuffle phase
    let mut groups: HashMap<String, Vec<i64>> = HashMap::new();
    for (word, count) in all_pairs {
        groups.entry(word).or_default().push(count);
    }

    // Reduce phase
    let groups_arc = Arc::new(Mutex::new(groups));
    let result = Arc::new(Mutex::new(HashMap::new()));
    let keys: Vec<String> = {
        let g = groups_arc.lock().unwrap();
        g.keys().cloned().collect()
    };

    let mut handles = Vec::new();
    for key in keys {
        let groups_ref = Arc::clone(&groups_arc);
        let result_ref = Arc::clone(&result);
        handles.push(thread::spawn(move || {
            let g = groups_ref.lock().unwrap();
            let total: i64 = g.get(&key).unwrap().iter().sum();
            let mut r = result_ref.lock().unwrap();
            r.insert(key, total);
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    Arc::try_unwrap(result).unwrap().into_inner().unwrap()
}

// ─── Brent's Theorem Demo ────────────────────────────────────────────────

fn brent_demo() {
    println!("=== Brent's Theorem ===");
    let t1: f64 = 1000.0;
    let t_inf: f64 = 10.0;

    for &p in &[1, 2, 4, 8, 16, 32, 64] {
        let lower = (t1 / p as f64).ceil() as i64;
        let upper = (t1 / p as f64).ceil() as i64 + t_inf as i64;
        println!(
            "  p={:3}:  T1/p = {:4}  ≤  T_p  ≤  {:4}  (= T1/p + T∞)",
            p, lower, upper
        );
    }
}

// ─── Simple PRNG (no external crate) ─────────────────────────────────────

fn simple_rand(seed: &mut u64) -> i64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    ((*seed >> 33) % 100) as i64 + 1
}

// ─── main ─────────────────────────────────────────────────────────────────

fn main() {
    let mut seed: u64 = 42;

    // ── Prefix Sum ──
    println!("=== Parallel Prefix Sum ===");
    for &size_exp in &[8, 12, 16] {
        let n = 1 << size_exp;
        let data: Vec<i64> = (0..n).map(|_| simple_rand(&mut seed)).collect();
        let log_n = (n as f64).log2() as usize;
        println!("  n={}: work=O({}), depth=O({})", n, n, log_n);

        measure_speedup(
            &format!("prefix_sum n={}", n),
            |arr: &[i64]| parallel_prefix_sum(arr),
            |arr: &[i64]| sequential_prefix_sum(arr),
            &data,
        );
    }

    // ── Merge Sort ──
    println!("=== Parallel Merge Sort ===");
    for &size_exp in &[10, 14] {
        let n = 1 << size_exp;
        let data: Vec<i64> = (0..n).map(|_| simple_rand(&mut seed) * 100).collect();
        let log_n = (n as f64).log2() as usize;
        println!(
            "  n={}: work=O(n log n) ≈ O({}), depth=O(log² n) ≈ O({})",
            n,
            n * log_n,
            log_n * log_n
        );

        measure_speedup(
            &format!("merge_sort n={}", n),
            |arr: &[i64]| parallel_merge_sort(arr, 0, 4),
            |arr: &[i64]| {
                let mut v = arr.to_vec();
                v.sort();
                v
            },
            &data,
        );
    }

    // ── Map-Reduce Word Count ──
    println!("=== Map-Reduce Word Count ===");
    let lines = [
        "the quick brown fox jumps over the lazy dog",
        "the lazy dog sleeps under the brown tree",
        "a quick fox and a lazy dog play together",
    ];
    let counts = map_reduce_word_count(&lines);
    let mut words: Vec<_> = counts.keys().cloned().collect();
    words.sort();
    for w in &words {
        println!("  {}: {}", w, counts[w]);
    }

    // ── Brent's Theorem ──
    println!();
    brent_demo();
}
