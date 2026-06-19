//! main.rs — character trie in Rust using HashMap<char, Box<TrieNode>>.

use std::collections::HashMap;

pub struct TrieNode {
    children: HashMap<char, Box<TrieNode>>,
    terminal: bool,
}

impl TrieNode {
    pub fn new() -> Self { TrieNode { children: HashMap::new(), terminal: false } }
}

pub struct Trie { root: TrieNode }

impl Trie {
    pub fn new() -> Self { Trie { root: TrieNode::new() } }

    pub fn insert(&mut self, word: &str) {
        let mut cur = &mut self.root;
        for c in word.chars() {
            cur = cur.children.entry(c).or_insert_with(|| Box::new(TrieNode::new()));
        }
        cur.terminal = true;
    }

    pub fn contains(&self, word: &str) -> bool {
        let mut cur = &self.root;
        for c in word.chars() {
            match cur.children.get(&c) {
                Some(n) => cur = n,
                None => return false,
            }
        }
        cur.terminal
    }

    pub fn prefix(&self, prefix: &str) -> Vec<String> {
        let mut cur = &self.root;
        for c in prefix.chars() {
            match cur.children.get(&c) {
                Some(n) => cur = n,
                None => return vec![],
            }
        }
        let mut out = vec![];
        Self::walk(cur, prefix.to_string(), &mut out);
        out
    }

    fn walk(n: &TrieNode, path: String, out: &mut Vec<String>) {
        if n.terminal { out.push(path.clone()); }
        for (c, child) in &n.children {
            let mut p = path.clone();
            p.push(*c);
            Self::walk(child, p, out);
        }
    }
}

fn main() {
    let mut t = Trie::new();
    for w in ["cat", "car", "card", "care", "careful", "core", "dog", "dot"] {
        t.insert(w);
    }
    println!("contains 'card': {}", t.contains("card"));
    println!("contains 'ca':   {}", t.contains("ca"));
    let mut ca = t.prefix("ca"); ca.sort();
    println!("prefix 'ca':     {:?}", ca);
}
