//! main.rs — runnable demo for the Rust toolchain lesson.
//!
//! Single-file build (no Cargo project required):
//!     rustc main.rs -O -o demo && ./demo 5
//!
//! Inside a cargo project you'd put this at src/main.rs and run `cargo run -- 5`.

use std::env;

fn factorial(n: u64) -> u128 {
    (1..=n as u128).product()
}

fn parse_arg() -> Result<u64, String> {
    let arg = env::args().nth(1).ok_or("usage: demo <n>")?;
    arg.parse::<u64>().map_err(|e| format!("not a number: {e}"))
}

fn main() {
    match parse_arg() {
        Ok(n) if n > 34 => {
            eprintln!("n={n} would overflow u128; pick n <= 34");
            std::process::exit(2);
        }
        Ok(n) => {
            let f = factorial(n);
            println!("{n}! = {f}");
        }
        Err(msg) => {
            eprintln!("error: {msg}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factorial_small() {
        assert_eq!(factorial(0), 1);
        assert_eq!(factorial(1), 1);
        assert_eq!(factorial(5), 120);
    }

    #[test]
    fn factorial_larger() {
        assert_eq!(factorial(20), 2_432_902_008_176_640_000);
    }
}
