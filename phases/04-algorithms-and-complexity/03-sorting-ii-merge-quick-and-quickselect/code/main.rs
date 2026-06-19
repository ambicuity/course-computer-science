//! Sorting II — Merge, Quick (and Quickselect)
//! Phase 04 — Algorithms & Complexity Analysis
//!
//! Generics-based merge sort and quicksort implementations with step counting.

use std::fmt::Debug;

// ---------------------------------------------------------------------------
// Merge Sort
// ---------------------------------------------------------------------------

/// Sort a slice in place using merge sort. Returns comparison and move counts.
pub fn merge_sort<T: Ord + Copy>(arr: &mut [T]) -> (usize, usize) {
    let mut comparisons = 0usize;
    let mut moves = 0usize;
    let len = arr.len();
    if len <= 1 {
        return (0, 0);
    }
    let mut buf = vec![arr[0]; len]; // reuse buffer
    merge_sort_inner(arr, &mut buf, 0, len, &mut comparisons, &mut moves);
    (comparisons, moves)
}

fn merge_sort_inner<T: Ord + Copy>(
    arr: &mut [T],
    buf: &mut [T],
    lo: usize,
    hi: usize,
    comparisons: &mut usize,
    moves: &mut usize,
) {
    if hi - lo <= 1 {
        return;
    }
    let mid = lo + (hi - lo) / 2;
    merge_sort_inner(arr, buf, lo, mid, comparisons, moves);
    merge_sort_inner(arr, buf, mid, hi, comparisons, moves);
    // Merge arr[lo..mid] and arr[mid..hi] into buf[lo..hi], then copy back.
    let mut i = lo;
    let mut j = mid;
    let mut k = lo;
    while i < mid && j < hi {
        *comparisons += 1;
        if arr[i] <= arr[j] {
            buf[k] = arr[i];
            *moves += 1;
            i += 1;
        } else {
            buf[k] = arr[j];
            *moves += 1;
            j += 1;
        }
        k += 1;
    }
    while i < mid {
        buf[k] = arr[i];
        *moves += 1;
        i += 1;
        k += 1;
    }
    while j < hi {
        buf[k] = arr[j];
        *moves += 1;
        j += 1;
        k += 1;
    }
    arr[lo..hi].copy_from_slice(&buf[lo..hi]);
}

// ---------------------------------------------------------------------------
// Quicksort
// ---------------------------------------------------------------------------

pub enum PivotStrategy {
    First,
    Random,
    Median3,
}

pub struct QuickSortStats {
    pub comparisons: usize,
    pub swaps: usize,
}

pub fn quick_sort<T: Ord + Copy + Debug>(
    arr: &mut [T],
    strategy: &PivotStrategy,
) -> QuickSortStats {
    let mut stats = QuickSortStats {
        comparisons: 0,
        swaps: 0,
    };
    quick_sort_inner(arr, strategy, &mut stats);
    stats
}

fn quick_sort_inner<T: Ord + Copy>(
    arr: &mut [T],
    strategy: &PivotStrategy,
    stats: &mut QuickSortStats,
) {
    let len = arr.len();
    if len <= 1 {
        return;
    }
    let pivot_idx = choose_pivot(arr, strategy);
    arr.swap(0, pivot_idx);
    stats.swaps += 1;
    let p = partition_lomuto(arr, stats);
    quick_sort_inner(&mut arr[..p], strategy, stats);
    quick_sort_inner(&mut arr[p + 1..], strategy, stats);
}

fn choose_pivot<T: Ord + Copy>(arr: &[T], strategy: &PivotStrategy) -> usize {
    let hi = arr.len() - 1;
    match strategy {
        PivotStrategy::First => 0,
        PivotStrategy::Random => {
            // Simple deterministic-ish for demo; use rand crate in production.
            let mid = hi / 2;
            (arr[0] as usize ^ arr[mid] as usize ^ arr[hi] as usize) % arr.len()
        }
        PivotStrategy::Median3 => {
            let lo = 0;
            let mid = hi / 2;
            let a = arr[lo];
            let b = arr[mid];
            let c = arr[hi];
            if (a <= b && b <= c) || (c <= b && b <= a) {
                mid
            } else if (b <= a && a <= c) || (c <= a && a <= b) {
                lo
            } else {
                hi
            }
        }
    }
}

fn partition_lomuto<T: Ord + Copy>(arr: &mut [T], stats: &mut QuickSortStats) -> usize {
    let pivot = arr[0];
    let mut i = 1;
    for j in 1..arr.len() {
        stats.comparisons += 1;
        if arr[j] < pivot {
            arr.swap(i, j);
            stats.swaps += 1;
            i += 1;
        }
    }
    arr.swap(0, i - 1);
    stats.swaps += 1;
    i - 1
}

// ---------------------------------------------------------------------------
// Quickselect
// ---------------------------------------------------------------------------

pub fn quickselect<T: Ord + Copy>(arr: &mut [T], k: usize) -> T {
    assert!(k < arr.len(), "k out of bounds");
    let mut lo = 0usize;
    let mut hi = arr.len() - 1;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        // Median-of-3 pivot selection
        let pivot_idx = {
            let (a, b, c) = (arr[lo], arr[mid], arr[hi]);
            if (a <= b && b <= c) || (c <= b && b <= a) {
                mid
            } else if (b <= a && a <= c) || (c <= a && a <= b) {
                lo
            } else {
                hi
            }
        };
        arr.swap(lo, pivot_idx);
        let pivot = arr[lo];
        let mut i = lo + 1;
        for j in lo + 1..=hi {
            if arr[j] < pivot {
                arr.swap(i, j);
                i += 1;
            }
        }
        arr.swap(lo, i - 1);
        let p = i - 1;
        if p == k {
            return arr[p];
        } else if p < k {
            lo = p + 1;
        } else {
            hi = p - 1;
        }
    }
    arr[lo]
}

// ---------------------------------------------------------------------------
// Main — demos and benchmarks
// ---------------------------------------------------------------------------

fn main() {
    println!("=== Sorting II — Merge, Quick, Quickselect ===\n");

    // --- Merge sort demo ---
    let mut data = vec![38, 27, 43, 3, 9, 82, 10];
    let (cmp, mov) = merge_sort(&mut data);
    println!("Merge sort:  {:?}", data);
    println!("  comparisons={cmp}, moves={mov}\n");

    // --- Quicksort strategies demo ---
    for (name, strat) in [
        ("first", PivotStrategy::First),
        ("median3", PivotStrategy::Median3),
    ] {
        let mut arr = vec![38, 27, 43, 3, 9, 82, 10];
        let stats = quick_sort(&mut arr, &strat);
        println!(
            "Quicksort ({:>7}): {:?}  comparisons={}, swaps={}",
            name, arr, stats.comparisons, stats.swaps
        );
    }

    // --- Quickselect demo ---
    let mut arr = vec![38, 27, 43, 3, 9, 82, 10];
    for k in [0, 2, 6] {
        let val = quickselect(&mut arr.clone(), k);
        println!("quickselect(arr, k={k}) = {val}");
    }
    println!();

    // --- Benchmark ---
    println!(
        "{:<25} {:>10} {:>10} {:>10}",
        "Algorithm", "n=1000", "n=5000", "n=10000"
    );
    println!("{}", "-".repeat(58));

    let sizes = [1000, 5000, 10000];
    let mut merge_times = Vec::new();
    let mut qs_times = Vec::new();

    for &size in &sizes {
        let mut data: Vec<i32> = (0..size).map(|i| (i * 7 + 13) % size as i32).collect();

        // Merge sort
        let mut d1 = data.clone();
        let start = std::time::Instant::now();
        merge_sort(&mut d1);
        merge_times.push(start.elapsed().as_secs_f64() * 1000.0);

        // Quicksort (median3)
        let mut d2 = data.clone();
        let start = std::time::Instant::now();
        quick_sort(&mut d2, &PivotStrategy::Median3);
        qs_times.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    println!(
        "{:<25} {:>9.2}ms {:>9.2}ms {:>9.2}ms", "merge_sort",
        merge_times[0], merge_times[1], merge_times[2]
    );
    println!(
        "{:<25} {:>9.2}ms {:>9.2}ms {:>9.2}ms", "quick_median3",
        qs_times[0], qs_times[1], qs_times[2]
    );
}
