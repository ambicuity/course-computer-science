//! main.rs — naive suffix array + Kasai LCP in Rust.

pub fn build_sa(s: &[u8]) -> Vec<usize> {
    let mut sa: Vec<usize> = (0..s.len()).collect();
    sa.sort_by(|&a, &b| s[a..].cmp(&s[b..]));
    sa
}

pub fn build_lcp(s: &[u8], sa: &[usize]) -> Vec<usize> {
    let n = s.len();
    let mut isa = vec![0usize; n];
    for (i, &p) in sa.iter().enumerate() { isa[p] = i; }
    let mut lcp = vec![0usize; n];
    let mut h = 0usize;
    for i in 0..n {
        if isa[i] > 0 {
            let j = sa[isa[i] - 1];
            while i + h < n && j + h < n && s[i + h] == s[j + h] { h += 1; }
            lcp[isa[i]] = h;
            if h > 0 { h -= 1; }
        }
    }
    lcp
}

pub fn longest_repeat(s: &[u8]) -> &[u8] {
    let sa = build_sa(s);
    let lcp = build_lcp(s, &sa);
    let mut best = 0usize;
    let mut at = 0usize;
    for i in 1..lcp.len() {
        if lcp[i] > best { best = lcp[i]; at = sa[i]; }
    }
    &s[at..at + best]
}

fn main() {
    let text = b"the quick brown fox jumps over the lazy dog. the quick fox is quick.";
    let sa = build_sa(text);
    println!("first 5 sorted suffixes:");
    for i in 0..5 {
        let start = sa[i];
        let end = (start + 30).min(text.len());
        println!("  SA[{i}]={:3}: {:?}", start, std::str::from_utf8(&text[start..end]).unwrap());
    }

    let lr = longest_repeat(text);
    println!("\nlongest repeated substring: {:?}", std::str::from_utf8(lr).unwrap());
}
