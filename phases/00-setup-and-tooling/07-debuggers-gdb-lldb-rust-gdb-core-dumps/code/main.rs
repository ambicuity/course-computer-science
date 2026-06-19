//! Rust drill program for rust-gdb / rust-lldb.
//!
//! Build:  rustc -g main.rs -o main_rs
//! Run:    ./main_rs 5       (normal)
//!         ./main_rs 999     (panic — produces a backtrace)
//!
//! `accumulate` builds a Vec and computes a sum; perfect for showing off
//! rust-gdb's pretty-printer for Vec<i32>.

use std::env;

fn accumulate(n: i32) -> (Vec<i32>, i32) {
    let mut v: Vec<i32> = Vec::with_capacity(n.max(0) as usize);
    let mut sum = 0;
    for i in 1..=n {
        v.push(i);
        sum += i;
    }
    (v, sum)
}

fn main() {
    let n: i32 = env::args()
        .nth(1)
        .expect("usage: main_rs N")
        .parse()
        .expect("N must be an integer");

    if n == 999 {
        panic!("deliberate panic for backtrace demo");
    }

    let (v, sum) = accumulate(n);
    println!("v = {v:?}");
    println!("sum = {sum}");
}
