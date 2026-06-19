//! Rust analog of L03: branches, loops, recursion, plus match (modern switch) and
//! Iterator-style loops.
//!
//! Build:  rustc -O main.rs -o m && ./m

fn classify(x: i32) -> &'static str {
    match x.cmp(&0) {
        std::cmp::Ordering::Less    => "negative",
        std::cmp::Ordering::Equal   => "zero",
        std::cmp::Ordering::Greater => "positive",
    }
}

fn sum_for(n: i32) -> i32 {
    (1..=n).sum()      // iterator-form loop
}

fn binary_search(arr: &[i32], target: i32) -> Option<usize> {
    let (mut lo, mut hi) = (0usize, arr.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match arr[mid].cmp(&target) {
            std::cmp::Ordering::Equal   => return Some(mid),
            std::cmp::Ordering::Less    => lo = mid + 1,
            std::cmp::Ordering::Greater => hi = mid,
        }
    }
    None
}

fn fact_tail(n: u64, acc: u64) -> u64 {
    if n <= 1 { acc } else { fact_tail(n - 1, n * acc) }
}

fn fib_iter(n: u64) -> u64 {
    if n < 2 { return n; }
    let (mut a, mut b) = (0u64, 1u64);
    for _ in 2..=n {
        let c = a + b;
        a = b;
        b = c;
    }
    b
}

fn main() {
    println!("== Branches via match ==");
    for x in -3..=3 {
        println!("  classify({}) = {}", x, classify(x));
    }

    println!("\n== Loop: (1..=10).sum() = {} (expected 55) ==", sum_for(10));

    println!("\n== Binary search ==");
    let arr = [1, 3, 5, 7, 9, 11, 13, 15, 17, 19];
    for &t in &[1, 7, 19, 20, 0] {
        println!("  binary_search(arr, {}) = {:?}", t, binary_search(&arr, t));
    }

    println!("\n== Factorial (tail-recursive accumulator) ==");
    for i in 0..=12 {
        println!("  {}! = {}", i, fact_tail(i, 1));
    }

    println!("\n== Fibonacci (iterative) ==");
    for i in 0..=12 {
        println!("  F_{} = {}", i, fib_iter(i));
    }
    assert_eq!(fib_iter(10), 55);
}
