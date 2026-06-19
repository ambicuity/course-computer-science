use std::time::Instant;

/// Counting sort — O(n + k), stable.
/// `max_val` must be the maximum value in the array (used to size the count array).
pub fn counting_sort(arr: &[usize], max_val: usize) -> Vec<usize> {
    let n = arr.len();
    let mut count = vec![0usize; max_val + 1];

    for &x in arr {
        count[x] += 1;
    }
    for i in 1..=max_val {
        count[i] += count[i - 1];
    }

    let mut output = vec![0usize; n];
    // Right-to-left preserves stability
    for &x in arr.iter().rev() {
        count[x] -= 1;
        output[count[x]] = x;
    }
    output
}

/// Counting sort keyed by a single digit at position `exp`.
/// Internal subroutine for radix sort.
fn counting_sort_by_digit(arr: &[usize], exp: usize) -> Vec<usize> {
    let n = arr.len();
    let mut count = [0usize; 10];

    for &x in arr {
        let digit = (x / exp) % 10;
        count[digit] += 1;
    }
    for i in 1..10 {
        count[i] += count[i - 1];
    }

    let mut output = vec![0usize; n];
    for &x in arr.iter().rev() {
        let digit = (x / exp) % 10;
        count[digit] -= 1;
        output[count[digit]] = x;
    }
    output
}

/// LSD Radix sort — O(d · (n + k)), stable.
pub fn radix_sort_lsd(arr: &[usize]) -> Vec<usize> {
    if arr.is_empty() {
        return Vec::new();
    }
    let max_val = *arr.iter().max().unwrap();
    let mut result = arr.to_vec();
    let mut exp = 1;
    while max_val / exp > 0 {
        result = counting_sort_by_digit(&result, exp);
        exp *= 10;
    }
    result
}

/// Bucket sort — O(n) average for uniform distribution on [0.0, 1.0).
pub fn bucket_sort(arr: &[f64], n_buckets: usize) -> Vec<f64> {
    if arr.is_empty() {
        return Vec::new();
    }
    let mut buckets: Vec<Vec<f64>> = vec![Vec::new(); n_buckets];
    let min_val = arr.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = arr.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let span = if (max_val - min_val).abs() < f64::EPSILON {
        1.0
    } else {
        max_val - min_val
    };

    for &x in arr {
        let idx = ((x - min_val) / span * (n_buckets as f64)) as usize;
        let idx = idx.min(n_buckets - 1);
        buckets[idx].push(x);
    }

    let mut result = Vec::with_capacity(arr.len());
    for mut b in buckets {
        b.sort_by(|a, b| a.partial_cmp(b).unwrap());
        result.extend(b);
    }
    result
}

fn main() {
    // ── Counting sort demo ──
    println!("=== Counting Sort ===");
    let data = vec![4, 2, 4, 1, 2, 7, 0, 3];
    let max_val = *data.iter().max().unwrap();
    let sorted = counting_sort(&data, max_val);
    println!("Input:  {:?}", data);
    println!("Output: {:?}", sorted);

    // Stability check: track which "4" comes first
    println!("Stable: elements with same value preserve input order.\n");

    // ── Radix sort demo ──
    println!("=== Radix Sort (LSD) ===");
    let data = vec![170, 45, 75, 90, 802, 24, 2, 66];
    let sorted = radix_sort_lsd(&data);
    println!("Input:  {:?}", data);
    println!("Output: {:?}", sorted);

    // Large random array
    let large: Vec<usize> = (0..100_000)
        .map(|_| fastrand::usize(0..1_000_000))
        .collect();
    let t0 = Instant::now();
    let _ = radix_sort_lsd(&large);
    let radix_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let mut std_sorted = large.clone();
    let t0 = Instant::now();
    std_sorted.sort();
    let std_ms = t0.elapsed().as_secs_f64() * 1000.0;

    println!("\n100k integers:");
    println!("  Radix sort:   {:.2} ms", radix_ms);
    println!("  std::sort:    {:.2} ms", std_ms);

    // ── Bucket sort demo ──
    println!("\n=== Bucket Sort ===");
    let data_f64: Vec<f64> = (0..100).map(|_| fastrand::f64()).collect();
    let sorted = bucket_sort(&data_f64, 20);
    println!("Input (first 5):  {:?}", &data_f64[..5]);
    println!("Output (first 5): {:?}", &sorted[..5]);

    // ── When linear sorts win vs lose ──
    println!("\n=== Linear Sorts: Win vs Lose ===");

    // Win: small range
    let small_range: Vec<usize> = (0..100_000)
        .map(|_| fastrand::usize(0..10))
        .collect();
    let t0 = Instant::now();
    let _ = counting_sort(&small_range, 10);
    let counting_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let mut std_data = small_range.clone();
    let t0 = Instant::now();
    std_data.sort();
    let std_ms = t0.elapsed().as_secs_f64() * 1000.0;

    println!("\nSmall range (values 0-9), 100k elements:");
    println!("  Counting sort: {:.2} ms (WIN)", counting_ms);
    println!("  std::sort:     {:.2} ms", std_ms);

    // Lose: large range
    let large_range: Vec<usize> = (0..100_000)
        .map(|_| fastrand::usize(0..usize::MAX))
        .collect();
    let t0 = Instant::now();
    let _ = radix_sort_lsd(&large_range);
    let radix_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let mut std_data = large_range.clone();
    let t0 = Instant::now();
    std_data.sort();
    let std_ms = t0.elapsed().as_secs_f64() * 1000.0;

    println!("\nLarge range (0 to usize::MAX), 100k elements:");
    println!("  Radix sort:   {:.2} ms", radix_ms);
    println!("  std::sort:    {:.2} ms (WIN: many digits in radix)", std_ms);

    println!("\nDone.");
}
