//! Pascal's triangle in Rust + a checked C(n, k) using u128.
//!
//! Build:  rustc -O main.rs -o pascal && ./pascal

fn n_choose_k(n: u32, k: u32) -> u128 {
    if k > n { return 0; }
    let k = k.min(n - k);
    let mut num: u128 = 1;
    let mut den: u128 = 1;
    for i in 0..k {
        num = num.checked_mul((n - i) as u128).expect("overflow");
        den = den.checked_mul((i + 1) as u128).expect("overflow");
    }
    num / den
}

fn pascal(rows: usize) -> Vec<Vec<u128>> {
    let mut out: Vec<Vec<u128>> = vec![vec![1]];
    for _ in 1..rows {
        let prev = out.last().unwrap().clone();
        let mut row = Vec::with_capacity(prev.len() + 1);
        row.push(1u128);
        for i in 0..prev.len() - 1 {
            row.push(prev[i] + prev[i + 1]);
        }
        row.push(1u128);
        out.push(row);
    }
    out
}

fn main() {
    println!("== n_choose_k ==");
    let cases = [(10u32, 3u32), (52, 5), (49, 6), (100, 50)];
    for (n, k) in cases {
        println!("  C({n}, {k}) = {}", n_choose_k(n, k));
    }

    println!("\n== Pascal rows 0..8 ==");
    for row in pascal(9) {
        println!("  {:?}", row);
    }

    // Cross-check against Pascal's rule
    let tri = pascal(15);
    for n in 1..tri.len() {
        for k in 1..tri[n].len() - 1 {
            assert_eq!(tri[n][k], tri[n - 1][k - 1] + tri[n - 1][k]);
            assert_eq!(tri[n][k], n_choose_k(n as u32, k as u32));
        }
    }
    println!("\n  ✓ Pascal's rule verified for n ∈ [1, 14]");
}
