//! Sieve of Eratosthenes + deterministic Miller-Rabin for u64.
//! Build: rustc -O main.rs -o primes && ./primes

fn sieve(n: usize) -> Vec<u64> {
    let mut is_prime = vec![true; n + 1];
    if n >= 0 { is_prime[0] = false; }
    if n >= 1 { is_prime[1] = false; }
    let mut i = 2;
    while i * i <= n {
        if is_prime[i] {
            let mut j = i * i;
            while j <= n {
                is_prime[j] = false;
                j += i;
            }
        }
        i += 1;
    }
    (0..=n as u64).filter(|&k| is_prime[k as usize]).collect()
}

/// Modular multiply using u128 to avoid u64 overflow.
fn mulmod(a: u64, b: u64, m: u64) -> u64 {
    (((a as u128) * (b as u128)) % (m as u128)) as u64
}

fn powmod(mut a: u64, mut e: u64, m: u64) -> u64 {
    if m == 1 { return 0; }
    let mut r = 1u64;
    a %= m;
    while e > 0 {
        if e & 1 == 1 { r = mulmod(r, a, m); }
        a = mulmod(a, a, m);
        e >>= 1;
    }
    r
}

/// Deterministic for all u64.
fn miller_rabin(n: u64) -> bool {
    if n < 2 { return false; }
    for &p in &[2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        if n == p { return true; }
        if n % p == 0 { return false; }
    }
    let mut d = n - 1;
    let mut s = 0u32;
    while d & 1 == 0 { d >>= 1; s += 1; }

    'witness: for &a in &[2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        if a >= n { continue; }
        let mut x = powmod(a, d, n);
        if x == 1 || x == n - 1 { continue; }
        for _ in 0..s - 1 {
            x = mulmod(x, x, n);
            if x == n - 1 { continue 'witness; }
        }
        return false;
    }
    true
}

fn main() {
    let primes_up_to_100 = sieve(100);
    println!("primes ≤ 100 ({} of them): {:?}", primes_up_to_100.len(), &primes_up_to_100[..10]);

    assert_eq!(miller_rabin(561), false);   // Carmichael
    println!("miller_rabin(561) = false  ✓");

    for &p in &[(1u64 << 31) - 1, (1u64 << 61) - 1] {
        println!("miller_rabin({p}) = {}", miller_rabin(p));
    }
}
