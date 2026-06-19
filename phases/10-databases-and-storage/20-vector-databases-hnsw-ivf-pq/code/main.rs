//! Vector Databases — HNSW, IVF, PQ
//! Phase 10 — Databases & Storage Systems
//!
//! HNSW (Hierarchical Navigable Small World) from scratch.
//!
//! ```
//! cargo run --release
//! ```

use rand::Rng;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::time::Instant;

type Distance = f32;

fn l2(a: &[f32], b: &[f32]) -> Distance {
    a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum()
}

struct HNSW {
    vectors: Vec<Vec<f32>>,
    layers: Vec<Vec<Vec<usize>>>,
    entry_point: Option<usize>,
    max_layer: usize,
    m: usize,
    mmax: usize,
    ef_con: usize,
    ef: usize,
    ml: f32,
}

impl HNSW {
    fn new(m: usize, ef_construction: usize, ef: usize) -> Self {
        let ml = 1.0 / (m as f32).ln();
        HNSW {
            vectors: Vec::new(),
            layers: Vec::new(),
            entry_point: None,
            max_layer: 0,
            m,
            mmax: m,
            ef_con: ef_construction,
            ef,
            ml,
        }
    }

    fn random_level(&self) -> usize {
        let mut rng = rand::thread_rng();
        let r: f32 = rng.gen();
        if r <= 0.0 || r >= 1.0 {
            return 0;
        }
        (-r.ln() * self.ml).floor() as usize
    }

    fn ensure_layer_capacity(&mut self, node_id: usize, level: usize) {
        while self.layers.len() <= level {
            self.layers.push(Vec::new());
        }
        for l in 0..=level {
            while self.layers[l].len() <= node_id {
                self.layers[l].push(Vec::new());
            }
        }
    }

    fn search_layer(
        &self,
        query: &[f32],
        entry: usize,
        ef: usize,
        layer: usize,
    ) -> Vec<(Distance, usize)> {
        let mut visited = HashSet::new();
        let mut candidates: BinaryHeap<ReverseItem> = BinaryHeap::new();
        let mut result: BinaryHeap<MaxItem> = BinaryHeap::new();

        visited.insert(entry);
        let d = l2(query, &self.vectors[entry]);
        candidates.push(ReverseItem(d, entry));
        result.push(MaxItem(d, entry));

        while let Some(ReverseItem(dc, c)) = candidates.pop() {
            if let Some(&MaxItem(df, _)) = result.peek() {
                if dc > df && result.len() >= ef {
                    break;
                }
            }
            for &n in &self.layers[layer][c] {
                if visited.insert(n) {
                    let dn = l2(query, &self.vectors[n]);
                    if result.len() < ef || dn < result.peek().unwrap().0 {
                        candidates.push(ReverseItem(dn, n));
                        result.push(MaxItem(dn, n));
                        if result.len() > ef {
                            result.pop();
                        }
                    }
                }
            }
        }

        result.into_sorted_vec().into_iter().map(|m| (m.0, m.1)).collect()
    }

    fn prune(&mut self, node: usize, layer: usize) {
        let query = self.vectors[node].clone();
        let neighbor_ids = self.layers[layer][node].clone();
        let mut dists: Vec<(Distance, usize)> = neighbor_ids
            .iter()
            .map(|&n| (l2(&query, &self.vectors[n]), n))
            .collect();
        dists.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        dists.truncate(self.mmax);
        self.layers[layer][node] = dists.into_iter().map(|(_, n)| n).collect();
    }

    fn insert(&mut self, vector: Vec<f32>) {
        let id = self.vectors.len();
        let level = self.random_level();
        self.ensure_layer_capacity(id, level);
        self.vectors.push(vector);

        if self.entry_point.is_none() {
            self.max_layer = level;
            self.entry_point = Some(id);
            return;
        }

        let ep = self.entry_point.unwrap();
        let query = self.vectors[id].clone();

        let mut curr_ep = ep;
        let mut curr_level = self.max_layer;
        while curr_level > level {
            let res = self.search_layer(&query, curr_ep, 1, curr_level);
            if let Some(&(_, next_ep)) = res.first() {
                curr_ep = next_ep;
            }
            if curr_level == 0 {
                break;
            }
            curr_level -= 1;
        }

        let top = level.min(self.max_layer);
        for l in (0..=top).rev() {
            let res = self.search_layer(&query, curr_ep, self.ef_con, l);
            let neighbors: Vec<usize> = res.iter().take(self.m).map(|&(_, n)| n).collect();

            for &n in &neighbors {
                self.layers[l][id].push(n);
                self.layers[l][n].push(id);
                if self.layers[l][n].len() > self.mmax {
                    self.prune(n, l);
                }
            }
            if !neighbors.is_empty() {
                curr_ep = neighbors[0];
            }
        }

        if level > self.max_layer {
            self.max_layer = level;
            self.entry_point = Some(id);
        }
    }

    fn search(&self, query: &[f32], k: usize) -> Vec<(Distance, usize)> {
        if self.entry_point.is_none() {
            return Vec::new();
        }

        let ep = self.entry_point.unwrap();
        let mut curr_ep = ep;
        let mut curr_level = self.max_layer;
        while curr_level > 0 {
            let res = self.search_layer(query, curr_ep, 1, curr_level);
            if let Some(&(_, next_ep)) = res.first() {
                curr_ep = next_ep;
            }
            curr_level -= 1;
        }

        let res = self.search_layer(query, curr_ep, self.ef, 0);
        res.into_iter().take(k).collect()
    }
}

// Min-heap item: smaller distance = higher priority for candidates
#[derive(Clone, Copy, Debug)]
struct ReverseItem(Distance, usize);

impl PartialEq for ReverseItem {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for ReverseItem {}
impl PartialOrd for ReverseItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.0.partial_cmp(&self.0)
    }
}
impl Ord for ReverseItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

// Max-heap item: larger distance = higher priority for result (for eviction)
#[derive(Clone, Copy, Debug)]
struct MaxItem(Distance, usize);

impl PartialEq for MaxItem {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for MaxItem {}
impl PartialOrd for MaxItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
impl Ord for MaxItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

fn brute_force(database: &[Vec<f32>], query: &[f32], k: usize) -> Vec<(Distance, usize)> {
    let mut dists: Vec<(Distance, usize)> = database
        .iter()
        .enumerate()
        .map(|(i, v)| (l2(query, v), i))
        .collect();
    dists.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    dists.truncate(k);
    dists
}

fn recall_at_k(exact: &[(Distance, usize)], approx: &[(Distance, usize)], k: usize) -> f64 {
    let exact_set: HashSet<usize> = exact.iter().take(k).map(|&(_, id)| id).collect();
    let approx_set: HashSet<usize> = approx.iter().take(k).map(|&(_, id)| id).collect();
    let intersection = exact_set.intersection(&approx_set).count();
    if exact_set.is_empty() {
        return 1.0;
    }
    intersection as f64 / exact_set.len() as f64
}

fn main() {
    let mut rng = rand::thread_rng();
    let (n, d, k) = (2000usize, 128, 10usize);
    let nq = 50usize;

    println!("Generating {} random {d}-dim vectors...", n);
    let vectors: Vec<Vec<f32>> = (0..n)
        .map(|_| (0..d).map(|_| rng.gen::<f32>()).collect())
        .collect();

    let queries: Vec<Vec<f32>> = (0..nq)
        .map(|_| (0..d).map(|_| rng.gen::<f32>()).collect())
        .collect();

    // --- Build HNSW index ---
    let mut hnsw = HNSW::new(16, 200, 50);
    let t0 = Instant::now();
    for v in &vectors {
        hnsw.insert(v.clone());
    }
    let build_time = t0.elapsed();
    println!(
        "HNSW build:  {}.{:03}s  ({} nodes)",
        build_time.as_secs(),
        build_time.subsec_millis(),
        hnsw.vectors.len()
    );

    // --- Brute-force baseline ---
    let t0 = Instant::now();
    let exact_results: Vec<Vec<(Distance, usize)>> = queries
        .iter()
        .map(|q| brute_force(&vectors, q, k))
        .collect();
    let exact_time = t0.elapsed();

    // --- HNSW search ---
    let t0 = Instant::now();
    let hnsw_results: Vec<Vec<(Distance, usize)>> = queries
        .iter()
        .map(|q| hnsw.search(q, k))
        .collect();
    let hnsw_time = t0.elapsed();

    // --- Results ---
    let recall: f64 = (0..nq)
        .map(|i| recall_at_k(&exact_results[i], &hnsw_results[i], k))
        .sum::<f64>()
        / nq as f64;

    println!(
        "Brute force:  {}.{:03}s  ({} queries × {} vectors)",
        exact_time.as_secs(),
        exact_time.subsec_millis(),
        nq,
        n
    );
    println!(
        "HNSW search:  {}.{:03}s  (ef = {})",
        hnsw_time.as_secs(),
        hnsw_time.subsec_millis(),
        hnsw.ef
    );
    println!("Recall@{}:     {:.3}", k, recall);
    println!(
        "Speedup:       {:.1}x",
        exact_time.as_secs_f64() / hnsw_time.as_secs_f64()
    );
}
