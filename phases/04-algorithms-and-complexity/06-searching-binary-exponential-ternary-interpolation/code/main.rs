//! Searching — Binary, Exponential, Ternary, Interpolation
//! Phase 04 — Algorithms & Complexity Analysis
//!
//! Generic binary search variants with Rust's trait bounds.

use std::time::Instant;

// ─── Binary Search ───────────────────────────────────────────────────────────

/// Iterative binary search on a sorted slice. Returns index of target or None.
pub fn binary_search<T: Ord>(arr: &[T], target: &T) -> Option<usize> {
    let mut lo = 0usize;
    let mut hi = arr.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2; // never (lo + hi) / 2
        if arr[mid] < *target {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo < arr.len() && arr[lo] == *target {
        Some(lo)
    } else {
        None
    }
}

/// Recursive binary search on a sorted slice.
pub fn binary_search_rec<T: Ord>(arr: &[T], target: &T) -> Option<usize> {
    fn helper<T: Ord>(arr: &[T], target: &T, lo: usize, hi: usize) -> Option<usize> {
        if lo >= hi {
            return if lo < arr.len() && arr[lo] == *target {
                Some(lo)
            } else {
                None
            };
        }
        let mid = lo + (hi - lo) / 2;
        if arr[mid] < *target {
            helper(arr, target, mid + 1, hi)
        } else {
            helper(arr, target, lo, mid)
        }
    }
    helper(arr, target, 0, arr.len())
}

// ─── Lower / Upper Bound ────────────────────────────────────────────────────

/// First index i where arr[i] >= target (equivalent to Rust's partition_point
/// with predicate `|x| x < target`).
pub fn lower_bound<T: Ord>(arr: &[T], target: &T) -> usize {
    let mut lo = 0usize;
    let mut hi = arr.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if arr[mid] < *target {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// First index i where arr[i] > target.
pub fn upper_bound<T: Ord>(arr: &[T], target: &T) -> usize {
    let mut lo = 0usize;
    let mut hi = arr.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if arr[mid] <= *target {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

// ─── Exponential Search ─────────────────────────────────────────────────────

/// Exponential search for unbounded/infinite sorted sequences. O(log i).
pub fn exponential_search<T: Ord>(arr: &[T], target: &T) -> Option<usize> {
    if arr.is_empty() {
        return None;
    }
    if arr[0] == *target {
        return Some(0);
    }
    let mut bound = 1usize;
    while bound < arr.len() && arr[bound] < *target {
        bound *= 2;
    }
    let lo = bound / 2;
    let hi = (bound + 1).min(arr.len());
    let idx = lower_bound(&arr[lo..hi], target) + lo;
    if idx < arr.len() && arr[idx] == *target {
        Some(idx)
    } else {
        None
    }
}

// ─── Ternary Search (Unimodal Function Maximum) ─────────────────────────────

/// Find the maximum of a unimodal function f on [lo, hi] using ternary search.
pub fn ternary_search<F: Fn(f64) -> f64>(f: F, lo: f64, hi: f64, eps: f64) -> f64 {
    let mut lo = lo;
    let mut hi = hi;
    while hi - lo > eps {
        let m1 = lo + (hi - lo) / 3.0;
        let m2 = hi - (hi - lo) / 3.0;
        if f(m1) < f(m2) {
            lo = m1;
        } else {
            hi = m2;
        }
    }
    (lo + hi) / 2.0
}

// ─── Interpolation Search ───────────────────────────────────────────────────

/// Interpolation search on uniformly distributed data. O(log log n) average.
pub fn interpolation_search(arr: &[i64], target: i64) -> Option<usize> {
    if arr.is_empty() {
        return None;
    }
    let mut lo = 0usize;
    let mut hi = arr.len() - 1;
    while lo <= hi && arr[lo] <= target && target <= arr[hi] {
        if arr[lo] == arr[hi] {
            if arr[lo] == target {
                return Some(lo);
            }
            break;
        }
        let range = arr[hi] - arr[lo];
        let offset = target - arr[lo];
        let pos = lo + ((offset as usize) * (hi - lo)) / (range as usize);
        if arr[pos] == target {
            return Some(pos);
        } else if arr[pos] < target {
            lo = pos + 1;
        } else {
            if pos == 0 {
                break;
            }
            hi = pos - 1;
        }
    }
    None
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_search() {
        let arr = [1, 3, 5, 7, 9, 11, 13];
        assert_eq!(binary_search(&arr, &7), Some(3));
        assert_eq!(binary_search(&arr, &1), Some(0));
        assert_eq!(binary_search(&arr, &13), Some(6));
        assert_eq!(binary_search(&arr, &6), None);
        assert_eq!(binary_search(&[] as &[i32], &5), None);
    }

    #[test]
    fn test_binary_search_rec() {
        let arr = [1, 3, 5, 7, 9, 11, 13];
        assert_eq!(binary_search_rec(&arr, &7), Some(3));
        assert_eq!(binary_search_rec(&arr, &6), None);
    }

    #[test]
    fn test_lower_upper_bound() {
        let arr = [1, 3, 5, 7, 7, 7, 9, 11];
        assert_eq!(lower_bound(&arr, &7), 3);
        assert_eq!(upper_bound(&arr, &7), 6);
        assert_eq!(lower_bound(&arr, &6), 3);
        assert_eq!(upper_bound(&arr, &6), 3);
        assert_eq!(lower_bound(&arr, &0), 0);
        assert_eq!(upper_bound(&arr, &12), 8);

        // Compare against Rust's built-in partition_point
        assert_eq!(lower_bound(&arr, &7), arr.partition_point(|&x| x < 7));
        assert_eq!(upper_bound(&arr, &7), arr.partition_point(|&x| x <= 7));
    }

    #[test]
    fn test_exponential_search() {
        let arr = [1, 3, 5, 7, 9, 11, 13];
        assert_eq!(exponential_search(&arr, &7), Some(3));
        assert_eq!(exponential_search(&arr, &1), Some(0));
        assert_eq!(exponential_search(&arr, &13), Some(6));
        assert_eq!(exponential_search(&arr, &6), None);
        assert_eq!(exponential_search(&[] as &[i32], &5), None);
    }

    #[test]
    fn test_ternary_search() {
        // Maximize f(x) = -(x-3)^2 + 10, peak at x=3
        let f = |x: f64| -(x - 3.0).powi(2) + 10.0;
        let peak = ternary_search(f, 0.0, 6.0, 1e-9);
        assert!((peak - 3.0).abs() < 1e-6, "found {peak}, expected ~3.0");
    }

    #[test]
    fn test_interpolation_search() {
        let uniform = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
        assert_eq!(interpolation_search(&uniform, 50), Some(4));
        assert_eq!(interpolation_search(&uniform, 10), Some(0));
        assert_eq!(interpolation_search(&uniform, 100), Some(9));
        assert_eq!(interpolation_search(&uniform, 55), None);
        assert_eq!(interpolation_search(&[] as &[i64], 5), None);
    }

    #[test]
    fn test_edge_cases() {
        assert_eq!(binary_search(&[5], &5), Some(0));
        assert_eq!(binary_search(&[5], &3), None);
        assert_eq!(binary_search(&[1, 2], &2), Some(1));
        assert_eq!(lower_bound(&[1], &0), 0);
        assert_eq!(lower_bound(&[1], &2), 1);
    }
}

// ─── Benchmark ──────────────────────────────────────────────────────────────

fn benchmark() {
    println!(
        "\n{:>12} {:>12} {:>12} {:>12} {:>14}",
        "n", "binary", "lower_bnd", "exponential", "interpolation"
    );
    println!("{}", "-".repeat(66));

    for &n in &[1_000usize, 10_000, 100_000, 1_000_000, 10_000_000] {
        let arr: Vec<i64> = (0..n).map(|i| (i as i64) * 2).collect(); // [0, 2, 4, ...]
        let target = arr[n / 2];

        let trials = 1000;

        // Binary search
        let t0 = Instant::now();
        for _ in 0..trials {
            let _ = binary_search(&arr, &target);
        }
        let t_binary = t0.elapsed().as_secs_f64() / trials as f64;

        // Lower bound
        let t0 = Instant::now();
        for _ in 0..trials {
            let _ = lower_bound(&arr, &target);
        }
        let t_lower = t0.elapsed().as_secs_f64() / trials as f64;

        // Exponential search
        let t0 = Instant::now();
        for _ in 0..trials {
            let _ = exponential_search(&arr, &target);
        }
        let t_exp = t0.elapsed().as_secs_f64() / trials as f64;

        // Interpolation search
        let t0 = Instant::now();
        for _ in 0..trials {
            let _ = interpolation_search(&arr, target);
        }
        let t_interp = t0.elapsed().as_secs_f64() / trials as f64;

        println!(
            "{n:>12} {t_binary:>11.7}s {t_lower:>11.7}s {t_exp:>11.7}s {t_interp:>13.7}s"
        );
    }
}

fn main() {
    // Run: cargo test  (for correctness)
    // Run: cargo run   (for benchmark)

    benchmark();

    // Quick demo
    let arr = [1, 3, 5, 7, 7, 7, 9, 11];
    println!("\narr = {:?}", arr);
    println!("binary_search(&arr, 7)  = {:?}", binary_search(&arr, &7));
    println!("lower_bound(&arr, 7)    = {}", lower_bound(&arr, &7));
    println!("upper_bound(&arr, 7)    = {}", upper_bound(&arr, &7));
    println!(
        "partition_point(|x| x < 7) = {} (Rust stdlib)",
        arr.partition_point(|&x| x < 7)
    );

    // Ternary search demo
    let f = |x: f64| -(x - 3.0).powi(2) + 10.0;
    let peak = ternary_search(f, 0.0, 6.0, 1e-9);
    println!("ternary_search peak = {peak:.6} (expected 3.0)");
}
