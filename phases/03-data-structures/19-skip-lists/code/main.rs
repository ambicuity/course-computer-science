//! main.rs — skip list in Rust.

const MAX_LEVEL: usize = 16;

struct Node {
    key: i32,
    next: Vec<Option<Box<Node>>>,        // owned chain
}

// Note: this implementation uses indices/positions via a Vec-of-nodes pattern
// to avoid the multi-owner pointer headaches in safe Rust. For production use
// `std::collections::BTreeSet` or `crossbeam-skiplist`.

pub struct SkipList {
    levels: Vec<Vec<i32>>,                // sorted entries per level
}

impl SkipList {
    pub fn new() -> Self {
        SkipList { levels: vec![vec![]; MAX_LEVEL] }
    }

    fn random_level(seed: &mut u64) -> usize {
        let mut lvl = 1;
        loop {
            *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            if (*seed >> 32) & 1 == 0 || lvl >= MAX_LEVEL { return lvl; }
            lvl += 1;
        }
    }

    pub fn insert(&mut self, key: i32, seed: &mut u64) {
        let lvl = Self::random_level(seed);
        for l in 0..lvl {
            match self.levels[l].binary_search(&key) {
                Ok(_) => return,                                  // duplicate
                Err(pos) => self.levels[l].insert(pos, key),
            }
        }
    }

    pub fn contains(&self, key: i32) -> bool {
        self.levels[0].binary_search(&key).is_ok()
    }

    pub fn level_counts(&self) -> Vec<usize> {
        self.levels.iter().map(|l| l.len()).collect()
    }
}

fn main() {
    let mut sl = SkipList::new();
    let mut seed: u64 = 42;
    for k in 0..1000 { sl.insert(k, &mut seed); }
    println!("Skip list level counts:");
    for (i, c) in sl.level_counts().iter().enumerate() {
        if *c == 0 { break; }
        println!("  level {i}: {c} nodes");
    }
    println!("\ncontains(500) = {}", sl.contains(500));
    println!("contains(9999) = {}", sl.contains(9999));
}
