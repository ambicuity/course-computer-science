//! Randomized Algorithms — Las Vegas vs Monte Carlo
//! Phase 04 — Algorithms & Complexity Analysis

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Las Vegas: Randomized Quicksort
// ---------------------------------------------------------------------------

fn randomized_quicksort(arr: &mut [i64]) -> u64 {
    let mut comparisons = 0u64;
    if arr.len() <= 1 {
        return 0;
    }
    qs_partition(arr, &mut comparisons);
    comparisons
}

fn qs_partition(arr: &mut [i64], comps: &mut u64) {
    let len = arr.len();
    if len <= 1 {
        return;
    }
    let ri = fastrand::usize(..len);
    let hi = len - 1;
    arr.swap(ri, hi);
    let pivot = arr[hi];
    let mut i = 0usize;
    for j in 0..hi {
        *comps += 1;
        if arr[j] <= pivot {
            arr.swap(i, j);
            i += 1;
        }
    }
    arr.swap(i, hi);
    qs_partition(&mut arr[..i], comps);
    qs_partition(&mut arr[i + 1..], comps);
}

// ---------------------------------------------------------------------------
// Las Vegas: Randomized Select (Quickselect)
// ---------------------------------------------------------------------------

fn randomized_select(arr: &mut [i64], k: usize) -> i64 {
    if arr.len() == 1 {
        return arr[0];
    }
    let pivot = arr[fastrand::usize(..arr.len())];
    let mut lows = Vec::new();
    let mut highs = Vec::new();
    let mut pivots = Vec::new();
    for &x in arr.iter() {
        if x < pivot {
            lows.push(x);
        } else if x > pivot {
            highs.push(x);
        } else {
            pivots.push(x);
        }
    }
    if k < lows.len() {
        randomized_select(&mut lows, k)
    } else if k < lows.len() + pivots.len() {
        pivot
    } else {
        randomized_select(&mut highs, k - lows.len() - pivots.len())
    }
}

// ---------------------------------------------------------------------------
// Monte Carlo: Miller-Rabin Primality Test
// ---------------------------------------------------------------------------

fn miller_rabin(n: u64, k: u32) -> bool {
    if n < 2 {
        return false;
    }
    if n < 4 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }

    let mut r = 0u32;
    let mut d = n - 1;
    while d % 2 == 0 {
        r += 1;
        d /= 2;
    }

    for _ in 0..k {
        let a = 2 + fastrand::u64(..(n - 3));
        let mut x = mod_pow(a, d, n);
        if x == 1 || x == n - 1 {
            continue;
        }
        let mut found = false;
        for _ in 0..(r - 1) {
            x = mul_mod(x, x, n);
            if x == n - 1 {
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }
    true
}

fn mod_pow(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    let mut result = 1u64;
    base %= modulus;
    while exp > 0 {
        if exp & 1 == 1 {
            result = mul_mod(result, base, modulus);
        }
        exp >>= 1;
        base = mul_mod(base, base, modulus);
    }
    result
}

fn mul_mod(a: u64, b: u64, m: u64) -> u64 {
    (a as u128 * b as u128 % m as u128) as u64
}

// ---------------------------------------------------------------------------
// Demonstration
// ---------------------------------------------------------------------------

fn main() {
    fastrand::seed(42);

    // 1. Randomized quicksort
    println!("=== Randomized Quicksort ===");
    for &n in &[100usize, 1000, 10000] {
        let mut arr: Vec<i64> = (0..n as i64).collect();
        fastrand::shuffle(&mut arr);
        let comps = randomized_quicksort(&mut arr);
        let expected = 2.0 * (n as f64) * (n as f64).ln();
        let ratio = comps as f64 / expected;
        println!(
            "  n={n:>5}: comparisons={comps:>7},  2n ln n={expected:>9.0},  ratio={ratio:.3}"
        );
    }

    // 2. Randomized select
    println!("\n=== Randomized Select (Quickselect) ===");
    let mut arr: Vec<i64> = (0..100).collect();
    fastrand::shuffle(&mut arr);
    for &k in &[0usize, 49, 99] {
        let val = randomized_select(&mut arr.clone(), k);
        println!("  k={k:>2}: {val} (correct={})", val == k as i64);
    }

    // 3. Miller-Rabin
    println!("\n=== Miller-Rabin Primality Test ===");
    let known_primes: &[u64] = &[2, 3, 5, 7, 11, 13, 17, 19, 97, 65537];
    let known_composites: &[u64] = &[1, 4, 15, 561, 1105, 1729, 29341];
    for &p in known_primes {
        assert!(miller_rabin(p, 40), "Failed on prime {p}");
    }
    for &c in known_composites {
        assert!(!miller_rabin(c, 40), "False positive on composite {c}");
    }
    println!("  All known primes/composites passed.");

    // Mersenne prime 2^61 - 1
    let m61 = (1u64 << 61) - 1;
    println!("  2^61 - 1 = {m61}: prime = {}", miller_rabin(m61, 40));

    println!("\nAll demonstrations complete.");
}
