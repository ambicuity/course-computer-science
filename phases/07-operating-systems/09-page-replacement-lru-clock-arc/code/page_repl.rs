use std::collections::{HashMap, VecDeque};

/* ── LRU ──────────────────────────────────────────────── */

fn lru_replace(pages: &[i32], frames: usize) -> usize {
    let mut mem: Vec<i32> = Vec::with_capacity(frames);
    let mut order: VecDeque<i32> = VecDeque::new();
    let mut faults = 0;

    for &page in pages {
        if let Some(pos) = mem.iter().position(|&p| p == page) {
            /* Hit: move to front (most recent) */
            order.retain(|&p| p != page);
            order.push_front(page);
        } else {
            /* Miss */
            faults += 1;
            if mem.len() < frames {
                mem.push(page);
            } else {
                /* Evict LRU (back of order) */
                let victim = order.pop_back().unwrap();
                if let Some(pos) = mem.iter().position(|&p| p == victim) {
                    mem[pos] = page;
                }
            }
            order.push_front(page);
        }
    }
    faults
}

/* ── Clock ────────────────────────────────────────────── */

fn clock_replace(pages: &[i32], frames: usize) -> usize {
    let mut mem: Vec<i32> = vec![-1; frames];
    let mut ref_bits: Vec<bool> = vec![false; frames];
    let mut hand = 0usize;
    let mut faults = 0;

    for &page in pages {
        /* Check hit */
        if let Some(pos) = mem.iter().position(|&p| p == page) {
            ref_bits[pos] = true;
            continue;
        }

        /* Miss */
        faults += 1;

        /* Find empty frame */
        if let Some(empty) = mem.iter().position(|&p| p == -1) {
            mem[empty] = page;
            ref_bits[empty] = true;
            continue;
        }

        /* Clock sweep */
        while ref_bits[hand] {
            ref_bits[hand] = false;
            hand = (hand + 1) % frames;
        }
        mem[hand] = page;
        ref_bits[hand] = true;
        hand = (hand + 1) % frames;
    }
    faults
}

/* ── Simplified CFS-inspired (using frequency) ────────── */

fn lfu_replace(pages: &[i32], frames: usize) -> usize {
    let mut mem: Vec<i32> = Vec::with_capacity(frames);
    let mut freq: HashMap<i32, usize> = HashMap::new();
    let mut faults = 0;

    for &page in pages {
        *freq.entry(page).or_insert(0) += 1;

        if mem.contains(&page) {
            continue;
        }

        faults += 1;
        if mem.len() < frames {
            mem.push(page);
        } else {
            /* Evict least frequently used */
            let mut victim_idx = 0;
            let mut min_freq = usize::MAX;
            for (i, &p) in mem.iter().enumerate() {
                let f = *freq.get(&p).unwrap_or(&0);
                if f < min_freq {
                    min_freq = f;
                    victim_idx = i;
                }
            }
            mem[victim_idx] = page;
        }
    }
    faults
}

/* ── FIFO ─────────────────────────────────────────────── */

fn fifo_replace(pages: &[i32], frames: usize) -> usize {
    let mut mem: VecDeque<i32> = VecDeque::with_capacity(frames);
    let mut faults = 0;

    for &page in pages {
        if mem.contains(&page) {
            continue;
        }
        faults += 1;
        if mem.len() == frames {
            mem.pop_front();
        }
        mem.push_back(page);
    }
    faults
}

/* ── Optimal ──────────────────────────────────────────── */

fn optimal_replace(pages: &[i32], frames: usize) -> usize {
    let mut mem: Vec<i32> = Vec::with_capacity(frames);
    let mut faults = 0;

    for (i, &page) in pages.iter().enumerate() {
        if mem.contains(&page) {
            continue;
        }
        faults += 1;
        if mem.len() < frames {
            mem.push(page);
            continue;
        }
        /* Evict page used furthest in future (or never) */
        let mut evict_idx = 0;
        let mut farthest = usize::MAX;
        for (j, &mp) in mem.iter().enumerate() {
            let next_use = pages[i + 1..]
                .iter()
                .position(|&p| p == mp)
                .map(|pos| i + 1 + pos)
                .unwrap_or(usize::MAX);
            if next_use > farthest {
                farthest = next_use;
                evict_idx = j;
            }
        }
        mem[evict_idx] = page;
    }
    faults
}

/* ── Benchmark ────────────────────────────────────────── */

fn benchmark(label: &str, pages: &[i32], frames: usize) {
    println!("{:<12} frames={}  ref_len={}", label, frames, pages.len());
    println!("  Optimal:  {} faults", optimal_replace(pages, frames));
    println!("  FIFO:     {} faults", fifo_replace(pages, frames));
    println!("  LRU:      {} faults", lru_replace(pages, frames));
    println!("  Clock:    {} faults", clock_replace(pages, frames));
    println!("  LFU:      {} faults", lfu_replace(pages, frames));
    println!();
}

fn main() {
    println!("=== Page Replacement Simulator (Rust) ===\n");

    let ref1 = vec![1, 2, 3, 4, 1, 2, 5, 1, 2, 3, 4, 5];
    benchmark("Classic", &ref1, 3);
    benchmark("Classic", &ref1, 4);

    println!("Belady's Anomaly (FIFO):");
    println!("  3 frames: {} faults", fifo_replace(&ref1, 3));
    println!("  4 frames: {} faults", fifo_replace(&ref1, 4));
    println!();

    let ref2 = vec![1, 1, 1, 2, 2, 2, 3, 3, 3, 1, 1, 4, 4, 4, 2, 2, 5, 5, 5, 1, 1, 1];
    benchmark("Locality", &ref2, 3);

    let ref3: Vec<i32> = (0..50).map(|i| i % 10).collect();
    benchmark("Scan(10)", &ref3, 4);
}
