//! String Matching — KMP, Z, Boyer-Moore
//! Phase 04 — Algorithms & Complexity Analysis, Lesson 19
//!
//! Generic implementations of KMP and Boyer-Moore for any byte-slice pattern/text.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// 1. KMP
// ---------------------------------------------------------------------------

/// Build the longest-proper-prefix-suffix (failure) array for `pattern`.
fn build_lps<T: Eq>(pattern: &[T]) -> Vec<usize> {
    let m = pattern.len();
    let mut lps = vec![0usize; m];
    let mut length = 0usize;
    let mut i = 1;
    while i < m {
        if pattern[i] == pattern[length] {
            length += 1;
            lps[i] = length;
            i += 1;
        } else if length != 0 {
            length = lps[length - 1];
        } else {
            lps[i] = 0;
            i += 1;
        }
    }
    lps
}

/// KMP search: returns (match_positions, comparison_count).
fn kmp_search<T: Eq>(text: &[T], pattern: &[T]) -> (Vec<usize>, usize) {
    let n = text.len();
    let m = pattern.len();
    if m == 0 {
        return ((0..=n).collect(), 0);
    }
    let lps = build_lps(pattern);
    let mut matches = Vec::new();
    let mut cmps = 0usize;
    let mut i = 0usize;
    let mut j = 0usize;
    while i < n {
        cmps += 1;
        if text[i] == pattern[j] {
            i += 1;
            j += 1;
        }
        if j == m {
            matches.push(i - j);
            j = lps[j - 1];
        } else if i < n && text[i] != pattern[j] {
            if j != 0 {
                j = lps[j - 1];
            } else {
                i += 1;
            }
        }
    }
    (matches, cmps)
}

// ---------------------------------------------------------------------------
// 2. Boyer-Moore
// ---------------------------------------------------------------------------

/// Bad-character heuristic table: maps each byte to its rightmost index in the pattern.
fn bad_char_table(pattern: &[u8]) -> HashMap<u8, usize> {
    let mut table = HashMap::new();
    for (i, &b) in pattern.iter().enumerate() {
        table.insert(b, i);
    }
    table
}

/// Good-suffix shift table.
fn good_suffix_table(pattern: &[u8]) -> Vec<usize> {
    let m = pattern.len();
    let mut gs = vec![0usize; m + 1];
    let mut border = vec![0usize; m + 1];

    // Phase 1
    let mut i = m;
    let mut j = m + 1;
    border[i] = j;
    while i > 0 {
        while j <= m && pattern[i - 1] != pattern[j - 1] {
            if gs[j] == 0 {
                gs[j] = j - i;
            }
            j = border[j];
        }
        i -= 1;
        j -= 1;
        border[i] = j;
    }

    // Phase 2
    j = border[0];
    for i in 0..=m {
        if gs[i] == 0 {
            gs[i] = j;
        }
        if i == j {
            j = border[j];
        }
    }
    gs
}

/// Boyer-Moore search (bytes only for simplicity). Returns (matches, comparisons).
fn boyer_moore(text: &[u8], pattern: &[u8]) -> (Vec<usize>, usize) {
    let n = text.len();
    let m = pattern.len();
    if m == 0 {
        return ((0..=n).collect(), 0);
    }
    if m > n {
        return (Vec::new(), 0);
    }

    let bc = bad_char_table(pattern);
    let gs = good_suffix_table(pattern);

    let mut matches = Vec::new();
    let mut cmps = 0usize;
    let mut i = 0usize;

    while i <= n - m {
        let mut j = m - 1;
        loop {
            cmps += 1;
            if pattern[j] != text[i + j] {
                break;
            }
            if j == 0 {
                matches.push(i);
                break;
            }
            j -= 1;
        }

        if j == 0 && pattern[0] == text[i] {
            // Matched — shift by good-suffix[0]
            i += gs[0];
        } else {
            let bc_shift = match bc.get(&text[i + j]) {
                Some(&pos) => j.saturating_sub(pos),
                None => j + 1,
            };
            let gs_shift = gs[j + 1];
            i += bc_shift.max(gs_shift);
        }
    }
    (matches, cmps)
}

// ---------------------------------------------------------------------------
// 3. Benchmarks & demo
// ---------------------------------------------------------------------------

fn main() {
    println!("=== String Matching — KMP, Boyer-Moore (Rust) ===\n");

    let text = b"ababcababcabc";
    let pattern = b"abcab";

    println!("Text:    {:?}", std::str::from_utf8(text).unwrap());
    println!("Pattern: {:?}", std::str::from_utf8(pattern).unwrap());

    let (m1, c1) = kmp_search(text, pattern);
    println!("\nKMP:        matches={:?}  comparisons={}", m1, c1);

    let (m2, c2) = boyer_moore(text, pattern);
    println!("Boyer-Moore: matches={:?}  comparisons={}", m2, c2);

    // Failure function demo
    println!(
        "\nKMP failure function for 'abcab': {:?}",
        build_lps(pattern)
    );

    // Benchmark: worst case for naive
    println!("\n--- Benchmark: 10 000 'a' + 'b', pattern='a...ab' (len 21) ---");
    let mut worst_text = vec![b'a'; 10_000];
    worst_text.push(b'b');
    let mut worst_pat = vec![b'a'; 20];
    worst_pat.push(b'b');

    let (_, c) = kmp_search(&worst_text, &worst_pat);
    println!("  KMP:        comparisons={:>10}", c);
    let (_, c) = boyer_moore(&worst_text, &worst_pat);
    println!("  Boyer-Moore: comparisons={:>10}", c);

    // Benchmark: DNA-like
    println!("\n--- Benchmark: 50 000 random ACGT, pattern='ACGTACGT' ---");
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let dna: Vec<u8> = (0u64..50_000)
        .map(|i| {
            let mut h = DefaultHasher::new();
            i.hash(&mut h);
            b"ACGT"[(h.finish() % 4) as usize]
        })
        .collect();
    let dna_pat = b"ACGTACGT";

    let (_, c) = kmp_search(&dna, dna_pat);
    println!("  KMP:        comparisons={:>10}", c);
    let (_, c) = boyer_moore(&dna, dna_pat);
    println!("  Boyer-Moore: comparisons={:>10}", c);

    // Correctness: verify KMP and Boyer-Moore agree
    println!("\n--- Correctness ---");
    let test_cases: Vec<(&[u8], &[u8])> = vec![
        (b"", b"a"),
        (b"a", b""),
        (b"abc", b"abc"),
        (b"aaaaaa", b"aa"),
        (b"mississippi", b"issi"),
    ];
    let mut all_ok = true;
    for (t, p) in &test_cases {
        let kmp_r = kmp_search(t, p).0;
        let bm_r = boyer_moore(t, p).0;
        if kmp_r != bm_r {
            println!(
                "  FAIL: text={:?} pat={:?}  kmp={:?} bm={:?}",
                std::str::from_utf8(t).unwrap(),
                std::str::from_utf8(p).unwrap(),
                kmp_r,
                bm_r
            );
            all_ok = false;
        }
    }
    if all_ok {
        println!("  All algorithms agree on all test cases.");
    }
}
