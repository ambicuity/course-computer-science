//! Hashing in Algorithms — Rabin-Karp, Rolling Hashes
//! Phase 04 — Algorithms & Complexity Analysis

const BASE: u64 = 131;
const MOD: u64 = 1_000_000_007;
const BASE2: u64 = 137;
const MOD2: u64 = 1_000_000_009;

fn polynomial_hash(s: &[u8], base: u64, modulus: u64) -> u64 {
    let mut h: u64 = 0;
    for &ch in s {
        h = (h.wrapping_mul(base).wrapping_add(ch as u64)) % modulus;
    }
    h
}

fn rolling_hash_search(text: &str, pattern: &str) -> Vec<usize> {
    let (t, p) = (text.as_bytes(), pattern.as_bytes());
    let (n, m) = (t.len(), p.len());
    if m > n || m == 0 {
        return vec![];
    }
    let h_pat = polynomial_hash(p, BASE, MOD);
    let mut h_win = polynomial_hash(&t[..m], BASE, MOD);
    let power = mod_pow(BASE, (m - 1) as u32, MOD);
    let mut matches = Vec::new();
    for i in 0..=(n - m) {
        if h_win == h_pat && &t[i..i + m] == p {
            matches.push(i);
        }
        if i < n - m {
            h_win = (MOD + h_win
                - ((t[i] as u64).wrapping_mul(power) % MOD))
                .wrapping_mul(BASE)
                .wrapping_add(t[i + m] as u64)
                % MOD;
        }
    }
    matches
}

fn multi_pattern_search(text: &str, patterns: &[&str]) -> Vec<(usize, usize)> {
    let t = text.as_bytes();
    let mut results = Vec::new();
    let mut by_len: std::collections::HashMap<usize, std::collections::HashSet<u64>> =
        std::collections::HashMap::new();
    for &p in patterns {
        by_len
            .entry(p.len())
            .or_default()
            .insert(polynomial_hash(p.as_bytes(), BASE, MOD));
    }
    for (length, hash_set) in &by_len {
        if *length > t.len() {
            continue;
        }
        let power = mod_pow(BASE, (*length - 1) as u32, MOD);
        let mut h = polynomial_hash(&t[..*length], BASE, MOD);
        for i in 0..=(t.len() - length) {
            if hash_set.contains(&h) {
                results.push((*length, i));
            }
            if i < t.len() - length {
                h = (MOD + h
                    - ((t[i] as u64).wrapping_mul(power) % MOD))
                    .wrapping_mul(BASE)
                    .wrapping_add(t[i + length] as u64)
                    % MOD;
            }
        }
    }
    results
}

fn longest_common_substring(s1: &str, s2: &str) -> usize {
    let (b1, b2) = (s1.as_bytes(), s2.as_bytes());

    fn has_common(b1: &[u8], b2: &[u8], l: usize) -> bool {
        if l == 0 {
            return true;
        }
        let power = mod_pow(BASE, (l - 1) as u32, MOD);
        let mut hashes = std::collections::HashSet::new();
        let mut h = polynomial_hash(&b1[..l], BASE, MOD);
        hashes.insert(h);
        for i in 1..=(b1.len() - l) {
            h = (MOD + h
                - ((b1[i - 1] as u64).wrapping_mul(power) % MOD))
                .wrapping_mul(BASE)
                .wrapping_add(b1[i + l - 1] as u64)
                % MOD;
            hashes.insert(h);
        }
        h = polynomial_hash(&b2[..l], BASE, MOD);
        if hashes.contains(&h) {
            return true;
        }
        for i in 1..=(b2.len() - l) {
            h = (MOD + h
                - ((b2[i - 1] as u64).wrapping_mul(power) % MOD))
                .wrapping_mul(BASE)
                .wrapping_add(b2[i + l - 1] as u64)
                % MOD;
            if hashes.contains(&h) {
                return true;
            }
        }
        false
    }

    let mut lo = 0usize;
    let mut hi = b1.len().min(b2.len());
    let mut best = 0usize;
    while lo <= hi {
        let mid = (lo + hi) / 2;
        if has_common(b1, b2, mid) {
            best = mid;
            lo = mid + 1;
        } else if mid > 0 {
            hi = mid - 1;
        } else {
            break;
        }
    }
    best
}

fn rabin_karp_double(text: &str, pattern: &str) -> Vec<usize> {
    let (t, p) = (text.as_bytes(), pattern.as_bytes());
    let (n, m) = (t.len(), p.len());
    if m > n || m == 0 {
        return vec![];
    }
    let (hp1, hp2) = (
        polynomial_hash(p, BASE, MOD),
        polynomial_hash(p, BASE2, MOD2),
    );
    let mut hw1 = polynomial_hash(&t[..m], BASE, MOD);
    let mut hw2 = polynomial_hash(&t[..m], BASE2, MOD2);
    let pw1 = mod_pow(BASE, (m - 1) as u32, MOD);
    let pw2 = mod_pow(BASE2, (m - 1) as u32, MOD2);
    let mut matches = Vec::new();
    for i in 0..=(n - m) {
        if hw1 == hp1 && hw2 == hp2 && &t[i..i + m] == p {
            matches.push(i);
        }
        if i < n - m {
            hw1 = (MOD + hw1
                - ((t[i] as u64).wrapping_mul(pw1) % MOD))
                .wrapping_mul(BASE)
                .wrapping_add(t[i + m] as u64)
                % MOD;
            hw2 = (MOD2 + hw2
                - ((t[i] as u64).wrapping_mul(pw2) % MOD2))
                .wrapping_mul(BASE2)
                .wrapping_add(t[i + m] as u64)
                % MOD2;
        }
    }
    matches
}

fn mod_pow(mut base: u64, mut exp: u32, modulus: u64) -> u64 {
    let mut result = 1u64;
    base %= modulus;
    while exp > 0 {
        if exp & 1 == 1 {
            result = result.wrapping_mul(base) % modulus;
        }
        exp >>= 1;
        base = base.wrapping_mul(base) % modulus;
    }
    result
}

fn count_distinct_substrings(s: &str) -> usize {
    let b = s.as_bytes();
    let mut seen = std::collections::HashSet::new();
    for length in 1..=b.len() {
        let power = mod_pow(BASE, (length - 1) as u32, MOD);
        let mut h = polynomial_hash(&b[..length], BASE, MOD);
        seen.insert(h);
        for i in 1..=(b.len() - length) {
            h = (MOD + h
                - ((b[i - 1] as u64).wrapping_mul(power) % MOD))
                .wrapping_mul(BASE)
                .wrapping_add(b[i + length - 1] as u64)
                % MOD;
            seen.insert(h);
        }
    }
    seen.len()
}

fn brute_lcs(a: &str, b: &str) -> usize {
    let mut best = 0usize;
    for i in 0..a.len() {
        for j in (i + 1)..=a.len() {
            if b.contains(&a[i..j]) {
                best = best.max(j - i);
            }
        }
    }
    best
}

fn main() {
    let text = "abcabcabc";
    let pattern = "abc";
    println!("Rabin-Karp search for '{pattern}' in '{text}':");
    println!("  Single pattern: {:?}", rolling_hash_search(text, pattern));
    println!("  Double hash:    {:?}", rabin_karp_double(text, pattern));

    let patterns = ["abc", "bca", "xyz"];
    println!("\nMulti-pattern search {patterns:?}:");
    println!("  {:?}", multi_pattern_search(text, &patterns));

    let s1 = "banana";
    let s2 = "canaan";
    let lcs = longest_common_substring(s1, s2);
    println!("\nLongest common substring of '{s1}' and '{s2}': {lcs}");
    assert_eq!(lcs, brute_lcs(s1, s2));

    let distinct = count_distinct_substrings("aba");
    println!("Distinct substrings of 'aba': {distinct} (expected: 5)");
    assert_eq!(distinct, 5);

    println!("\nAll assertions passed.");
}
