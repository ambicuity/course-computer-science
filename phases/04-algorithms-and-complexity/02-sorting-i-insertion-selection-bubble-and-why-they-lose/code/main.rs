//! Sorting I — Insertion, Selection, Bubble, and Why They Lose
//! Phase 04 — Algorithms & Complexity Analysis
//!
//! Generic sorts using Ord bound with comparison/swap counters.

use std::time::Instant;

struct Counters {
    comparisons: usize,
    swaps: usize,
}

impl Counters {
    fn new() -> Self {
        Counters {
            comparisons: 0,
            swaps: 0,
        }
    }
}

fn insertion_sort<T: Ord>(arr: &mut [T]) -> Counters {
    let mut c = Counters::new();
    for i in 1..arr.len() {
        let mut j = i;
        while j > 0 {
            c.comparisons += 1;
            if arr[j - 1] > arr[j] {
                arr.swap(j - 1, j);
                c.swaps += 1;
                j -= 1;
            } else {
                break;
            }
        }
    }
    c
}

fn selection_sort<T: Ord>(arr: &mut [T]) -> Counters {
    let mut c = Counters::new();
    let n = arr.len();
    for i in 0..n.saturating_sub(1) {
        let mut min_idx = i;
        for j in (i + 1)..n {
            c.comparisons += 1;
            if arr[j] < arr[min_idx] {
                min_idx = j;
            }
        }
        if min_idx != i {
            arr.swap(i, min_idx);
            c.swaps += 1;
        }
    }
    c
}

fn bubble_sort<T: Ord>(arr: &mut [T]) -> Counters {
    let mut c = Counters::new();
    let n = arr.len();
    for i in 0..n.saturating_sub(1) {
        let mut swapped = false;
        for j in 0..(n - 1 - i) {
            c.comparisons += 1;
            if arr[j] > arr[j + 1] {
                arr.swap(j, j + 1);
                c.swaps += 1;
                swapped = true;
            }
        }
        if !swapped {
            break;
        }
    }
    c
}

fn benchmark<F: Fn(&mut [i32]) -> Counters>(
    sort_fn: F,
    data: &[i32],
    name: &str,
    input_name: &str,
) {
    let mut arr: Vec<i32> = data.to_vec();
    let start = Instant::now();
    let c = sort_fn(&mut arr);
    let elapsed = start.elapsed();
    let is_sorted = arr.windows().all(|w| w[0] <= w[1]);
    assert!(is_sorted, "{name} produced wrong result");
    println!(
        "{name:<12} {input_name:<10} {:>12} {:>10} {:>10.2}",
        c.comparisons,
        c.swaps,
        elapsed.as_secs_f64() * 1000.0,
    );
}

fn generate_inputs(n: usize) -> Vec<(&'static str, Vec<i32>)> {
    // Simple deterministic RNG (xorshift)
    let mut seed: u64 = 42;
    let mut rand = || -> i32 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % (n as u64 * 10 + 1)) as i32
    };

    let random_data: Vec<i32> = (0..n).map(|_| rand()).collect();
    let sorted_data: Vec<i32> = (0..n).map(|i| i as i32).collect();
    let reverse_data: Vec<i32> = (0..n).map(|i| (n - i) as i32).collect();
    let sawtooth: Vec<i32> = (0..n).map(|i| (i % (n / 10 + 1)) as i32).collect();

    vec![
        ("random", random_data),
        ("sorted", sorted_data),
        ("reverse", reverse_data),
        ("sawtooth", sawtooth),
    ]
}

fn stability_check() {
    let pairs = vec![(3, "A"), (1, "B"), (3, "C"), (2, "D"), (3, "E")];

    // Insertion sort (stable)
    let mut data = pairs.clone();
    insertion_sort(&mut data);
    println!("Insertion: {:?}", data);

    // Selection sort (not stable)
    let mut data = pairs.clone();
    selection_sort(&mut data);
    println!("Selection: {:?}", data);

    // Bubble sort (stable)
    let mut data = pairs.clone();
    bubble_sort(&mut data);
    println!("Bubble:    {:?}", data);
}

fn main() {
    let sizes = [100, 1000, 5000];

    println!("{}", "=".repeat(78));
    println!("Sorting Benchmark — Insertion vs Selection vs Bubble");
    println!("{}", "=".repeat(78));

    for &n in &sizes {
        let inputs = generate_inputs(n);
        println!("\n--- n = {} ---", n);
        println!(
            "{:<12} {:<10} {:>12} {:>10} {:>10}",
            "Sort", "Input", "Comparisons", "Swaps", "Time (ms)"
        );
        println!("{}", "-".repeat(58));

        for (input_name, data) in &inputs {
            benchmark(insertion_sort, data, "Insertion", input_name);
            benchmark(selection_sort, data, "Selection", input_name);
            benchmark(bubble_sort, data, "Bubble   ", input_name);
        }
    }

    println!("\n--- Theoretical Reference ---");
    for &n in &sizes {
        println!("n={:>6}:  n(n-1)/2 = {:>12}", n, n * (n - 1) / 2);
    }

    println!("\n--- Stability Check ---");
    stability_check();
}
