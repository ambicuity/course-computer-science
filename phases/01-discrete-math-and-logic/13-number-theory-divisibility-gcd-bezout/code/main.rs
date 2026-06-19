//! GCD + extended GCD + modular inverse in Rust.
//! Build: rustc -O main.rs -o nt && ./nt

fn gcd(mut a: i128, mut b: i128) -> i128 {
    a = a.abs(); b = b.abs();
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// Returns (g, x, y) with a*x + b*y = g = gcd(a, b).
fn extended_gcd(a: i128, b: i128) -> (i128, i128, i128) {
    if b == 0 {
        return (a.abs(), if a >= 0 { 1 } else { -1 }, 0);
    }
    let (g, x1, y1) = extended_gcd(b, a % b);
    (g, y1, x1 - (a / b) * y1)
}

fn modinv(a: i128, m: i128) -> Option<i128> {
    let (g, x, _) = extended_gcd(a, m);
    if g != 1 { return None; }
    Some(((x % m) + m) % m)
}

fn main() {
    println!("gcd(462, 1071) = {}", gcd(462, 1071));
    let (g, x, y) = extended_gcd(11, 13);
    println!("11·{} + 13·{} = {} = gcd", x, y, 11 * x + 13 * y);
    assert_eq!(11 * x + 13 * y, g);

    let inv = modinv(17, 1_000_000_007).expect("coprime");
    println!("17⁻¹ mod 1e9+7 = {}; verify (17·inv) mod = {}",
             inv, (17 * inv) % 1_000_000_007);
}
