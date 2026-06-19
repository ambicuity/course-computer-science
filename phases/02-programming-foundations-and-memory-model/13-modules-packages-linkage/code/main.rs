//! Rust module visibility demo.
//! Build: rustc -O main.rs -o m && ./m

mod math {
    pub fn add(a: i32, b: i32) -> i32 {
        a + b
    }

    pub(crate) fn double(x: i32) -> i32 {
        x * 2
    }

    // Module-private — not visible from main.
    #[allow(dead_code)]
    fn secret(x: i32) -> i32 {
        x * 3
    }

    pub mod inner {
        // Accessible as math::inner::greet
        pub fn greet() -> &'static str {
            "hi from math::inner"
        }
    }
}

fn main() {
    println!("math::add(3, 4) = {}", math::add(3, 4));
    println!("math::double(7) = {}", math::double(7));
    println!("math::inner::greet() = {}", math::inner::greet());
    // Uncommenting next line fails to compile:
    //   error: function `secret` is private
    // println!("{}", math::secret(5));
}
