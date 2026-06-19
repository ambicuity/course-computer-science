//! Rust error-handling: Result + ?, Option, and when to panic.
//! Build: rustc -O main.rs -o m && ./m

use std::collections::HashMap;
use std::num::ParseIntError;

/// Domain-specific error type aggregating possible failures.
#[derive(Debug)]
enum AppError {
    BadFormat(ParseIntError),
    OutOfRange(i32),
    DbMiss(String),
}

impl From<ParseIntError> for AppError {
    fn from(e: ParseIntError) -> Self { AppError::BadFormat(e) }
}

fn parse_positive(s: &str) -> Result<i32, AppError> {
    let n: i32 = s.parse()?;                 // ? converts via From<ParseIntError>
    if n < 0 {
        return Err(AppError::OutOfRange(n));
    }
    Ok(n)
}

fn lookup(db: &HashMap<&str, i32>, key: &str) -> Result<i32, AppError> {
    db.get(key)
        .copied()
        .ok_or_else(|| AppError::DbMiss(key.to_string()))
}

fn main() {
    println!("== Result + ? operator ==");
    for input in ["42", "-7", "not_a_number"] {
        match parse_positive(input) {
            Ok(n)  => println!("  parse_positive({:?}) = Ok({})", input, n),
            Err(e) => println!("  parse_positive({:?}) = Err({:?})", input, e),
        }
    }

    println!("\n== Option<T> via HashMap::get ==");
    let mut db: HashMap<&str, i32> = HashMap::new();
    db.insert("alice", 30);
    db.insert("bob",   25);

    for key in ["alice", "carol"] {
        match lookup(&db, key) {
            Ok(age) => println!("  lookup({:?}) = {}", key, age),
            Err(e)  => println!("  lookup({:?}) = Err({:?})", key, e),
        }
    }

    println!("\n== When to panic vs return ==");
    let nums = vec![1, 2, 3];
    println!("  nums[1] = {}", nums[1]);       // panics on OOB — that's a bug
    // nums[10]                                 // would panic
    println!("  nums.get(10) = {:?}",  nums.get(10));   // returns Option — safe
}
