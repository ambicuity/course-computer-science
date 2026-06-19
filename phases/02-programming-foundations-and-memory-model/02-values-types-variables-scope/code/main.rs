//! Rust types + shadowing + copy/move demo.
//! Build:  rustc -O main.rs -o m && ./m

use std::mem::size_of;

fn main() {
    println!("== Rust primitive sizes (fixed by spec) ==");
    println!("  i8:    {} byte",  size_of::<i8>());
    println!("  i16:   {} bytes", size_of::<i16>());
    println!("  i32:   {} bytes", size_of::<i32>());
    println!("  i64:   {} bytes", size_of::<i64>());
    println!("  i128:  {} bytes", size_of::<i128>());
    println!("  f32:   {} bytes", size_of::<f32>());
    println!("  f64:   {} bytes", size_of::<f64>());
    println!("  bool:  {} byte",  size_of::<bool>());
    println!("  char:  {} bytes (a Unicode scalar value, not a byte!)", size_of::<char>());
    println!("  usize: {} bytes (pointer-sized)", size_of::<usize>());

    println!("\n== Shadowing with type change ==");
    let x = "42";                          // x: &str
    println!("  x = {:?} (type &str)", x);
    let x: i32 = x.parse().unwrap();       // shadows: x is now i32
    println!("  x = {:?} (type i32; original &str shadowed)", x);
    let x = x * 2;                         // shadow again
    println!("  x = {:?} (i32)", x);

    println!("\n== Copy vs move ==");
    // i32 implements Copy: assignment copies the bits.
    let a: i32 = 5;
    let b = a;
    println!("  i32: a={}, b={}  (both valid — i32 is Copy)", a, b);

    // String does NOT implement Copy: assignment moves.
    let s1 = String::from("hello");
    let s2 = s1;
    println!("  String: s2={:?}  (s1 has been moved; using it would be a compile error)", s2);
    // Uncommenting the next line would FAIL to compile:
    // println!("{}", s1);    // ← error: borrow of moved value
}
