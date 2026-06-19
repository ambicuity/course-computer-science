//! Rust macros via `macro_rules!`.
//! Build: rustc -O main.rs -o m && ./m

// Hygienic: introduces a fresh binding `a, b` that won't collide with caller's scope.
macro_rules! max {
    ($a:expr, $b:expr) => {{
        let a = $a;
        let b = $b;
        if a > b { a } else { b }
    }};
}

// Recursive over a list, like a variadic function.
macro_rules! sum {
    ($x:expr) => { $x };
    ($x:expr, $($rest:expr),+) => { $x + sum!($($rest),+) };
}

// A type-parameterized vector constructor (subset of std::vec!).
macro_rules! vec_of {
    ( $( $x:expr ),* $(,)? ) => {{
        let mut v = Vec::new();
        $( v.push($x); )*
        v
    }};
}

// Trace with file:line auto-captured (similar to C's __FILE__/__LINE__).
macro_rules! trace {
    ($msg:expr) => {
        println!("[{}:{}] {}", file!(), line!(), $msg)
    };
}

fn main() {
    println!("== max! ==");
    let mut i = 0;
    let m = max!({ i += 1; i + 4 }, 5);  // bumped once
    println!("  max!({{ i+=1; i+4 }}, 5) = {}, i = {}  (no multiple-evaluation)", m, i);

    println!("\n== Recursive sum! ==");
    println!("  sum!(1, 2, 3, 4, 5) = {}", sum!(1, 2, 3, 4, 5));

    println!("\n== vec_of! ==");
    let v: Vec<i32> = vec_of![1, 2, 3, 4, 5];
    println!("  vec_of![1..5] = {:?}", v);

    println!("\n== trace! captures file:line ==");
    trace!("hello, hygienic macros");
}
