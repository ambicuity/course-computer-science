//! Rust assert/debug_assert demos.
//! Build:
//!   rustc -O main.rs -o m && ./m                # release; debug_assert! compiled out
//!   rustc    main.rs -o md && ./md              # debug; both fire

fn sum(arr: &[i32]) -> i32 {
    assert!(!arr.is_empty(), "sum requires a non-empty slice");
    arr.iter().sum()
}

fn binary_search(arr: &[i32], target: i32) -> Option<usize> {
    debug_assert!(arr.windows(2).all(|w| w[0] <= w[1]), "binary_search: array must be sorted");
    let (mut lo, mut hi) = (0, arr.len());
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

fn main() {
    let arr = [1, 3, 5, 7, 9];
    println!("== Happy path ==");
    println!("  sum(arr) = {}", sum(&arr));
    println!("  binary_search(arr, 7) = {:?}", binary_search(&arr, 7));

    println!("\n== assert! vs debug_assert! ==");
    println!("  assert!: always fires in any build mode");
    println!("  debug_assert!: fires only in debug builds (zero cost in --release)");

    // Demonstrate panic on bad input — only run via env to keep the demo non-crashing.
    if std::env::args().any(|a| a == "--crash-empty") {
        let _ = sum(&[]);    // assert! panics
    }
    if std::env::args().any(|a| a == "--crash-unsorted") {
        let _ = binary_search(&[5, 2, 8], 5);   // debug_assert! panics in debug build
    }
}
