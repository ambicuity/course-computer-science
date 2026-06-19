//! Suffix Structures in Action — Aho-Corasick, LCP
//! Phase 04 — Algorithms & Complexity Analysis, Lesson 20
//!
//! Aho-Corasick multi-pattern matching automaton with trie + failure links.

use std::collections::{HashMap, VecDeque};

// ---------------------------------------------------------------------------
// Aho-Corasick
// ---------------------------------------------------------------------------

/// Aho-Corasick automaton for multi-pattern string matching.
pub struct AhoCorasick {
    trie: Vec<HashMap<u8, usize>>,
    output: Vec<Vec<usize>>,
    fail: Vec<usize>,
    patterns: Vec<Vec<u8>>,
}

impl AhoCorasick {
    /// Build the automaton from a list of pattern strings.
    pub fn new(patterns: &[&[u8]]) -> Self {
        let mut ac = AhoCorasick {
            trie: vec![HashMap::new()],
            output: vec![Vec::new()],
            fail: vec![0],
            patterns: patterns.iter().map(|p| p.to_vec()).collect(),
        };
        ac.build_trie();
        ac.build_fail();
        ac
    }

    fn build_trie(&mut self) {
        for (idx, pat) in self.patterns.clone().iter().enumerate() {
            let mut node = 0usize;
            for &ch in pat {
                if !self.trie[node].contains_key(&ch) {
                    let new_idx = self.trie.len();
                    self.trie[node].insert(ch, new_idx);
                    self.trie.push(HashMap::new());
                    self.output.push(Vec::new());
                    self.fail.push(0);
                }
                node = self.trie[node][&ch];
            }
            self.output[node].push(idx);
        }
    }

    fn build_fail(&mut self) {
        let mut queue: VecDeque<usize> = VecDeque::new();

        // Initialize: children of root
        let root_children: Vec<(u8, usize)> = self.trie[0].iter().map(|(&k, &v)| (k, v)).collect();
        for (_, child) in &root_children {
            self.fail[*child] = 0;
            queue.push_back(*child);
        }

        while let Some(u) = queue.pop_front() {
            let edges: Vec<(u8, usize)> = self.trie[u].iter().map(|(&k, &v)| (k, v)).collect();
            for (ch, v) in edges {
                queue.push_back(v);
                // Follow failure link chain
                let mut f = self.fail[u];
                while f != 0 && !self.trie[f].contains_key(&ch) {
                    f = self.fail[f];
                }
                self.fail[v] = *self.trie[f].get(&ch).unwrap_or(&0);
                // Propagate outputs
                let fail_outputs = self.output[self.fail[v]].clone();
                self.output[v].extend(fail_outputs);
            }
        }
    }

    /// Search text for all pattern occurrences.
    /// Returns (position, pattern_index) pairs.
    pub fn search(&self, text: &[u8]) -> Vec<(usize, usize)> {
        let mut matches = Vec::new();
        let mut node = 0usize;

        for (i, &ch) in text.iter().enumerate() {
            // Follow failure links until we can advance
            let mut current = node;
            while current != 0 && !self.trie[current].contains_key(&ch) {
                current = self.fail[current];
            }
            node = *self.trie[current].get(&ch).unwrap_or(&0);

            // Report all patterns ending here
            for &pat_idx in &self.output[node] {
                let pat_len = self.patterns[pat_idx].len();
                matches.push((i + 1 - pat_len, pat_idx));
            }
        }
        matches
    }
}

// ---------------------------------------------------------------------------
// Demo
// ---------------------------------------------------------------------------

fn main() {
    println!("=== Suffix Structures — Aho-Corasick (Rust) ===\n");

    // --- Basic multi-pattern search ---
    let pattern_strs = vec!["he", "she", "his", "hers"];
    let pattern_bytes: Vec<&[u8]> = pattern_strs.iter().map(|s| s.as_bytes()).collect();
    let ac = AhoCorasick::new(&pattern_bytes);

    let text = b"ushers";
    println!("Patterns: {:?}", pattern_strs);
    println!("Text:     {:?}", std::str::from_utf8(text).unwrap());

    let matches = ac.search(text);
    println!("Matches:");
    for (pos, idx) in &matches {
        println!("  '{}' at position {}", pattern_strs[*idx], pos);
    }

    // --- Genome search ---
    println!();
    let genome = b"ACGTACGTACGTAGCTAGCTAGCTACGT";
    let probe_strs = vec!["ACGT", "TAGC", "GCTA", "TACG"];
    let probe_bytes: Vec<&[u8]> = probe_strs.iter().map(|s| s.as_bytes()).collect();
    let ac2 = AhoCorasick::new(&probe_bytes);

    println!(
        "Genome:  {:?}",
        std::str::from_utf8(genome).unwrap()
    );
    println!("Probes:  {:?}", probe_strs);

    let matches = ac2.search(genome);
    println!("Matches ({} total):", matches.len());
    for (pos, idx) in &matches {
        println!("  '{}' at position {}", probe_strs[*idx], pos);
    }

    // --- Verify: no missed matches ---
    println!();
    println!("--- Verification: compare against brute force ---");
    let verify_text = b"ABCABCABC";
    let verify_pats = vec!["ABC", "BCA", "CAB", "ABCA"];
    let verify_bytes: Vec<&[u8]> = verify_pats.iter().map(|s| s.as_bytes()).collect();
    let ac3 = AhoCorasick::new(&verify_bytes);

    let ac_matches = ac3.search(verify_text);
    let mut brute_matches: Vec<(usize, usize)> = Vec::new();
    for (idx, pat) in verify_pats.iter().enumerate() {
        let p = pat.as_bytes();
        for i in 0..=verify_text.len() - p.len() {
            if &verify_text[i..i + p.len()] == p {
                brute_matches.push((i, idx));
            }
        }
    }

    // Sort both for comparison
    let mut ac_sorted = ac_matches.clone();
    let mut brute_sorted = brute_matches.clone();
    ac_sorted.sort();
    brute_sorted.sort();

    if ac_sorted == brute_sorted {
        println!("  Aho-Corasick matches brute-force on all test cases.");
    } else {
        println!("  MISMATCH!");
        println!("    Aho-Corasick: {:?}", ac_sorted);
        println!("    Brute-force:  {:?}", brute_sorted);
    }
}
