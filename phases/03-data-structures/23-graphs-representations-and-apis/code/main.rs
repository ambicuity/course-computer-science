//! main.rs — adjacency list + CSR in Rust, with BFS.

use std::collections::VecDeque;

pub struct AdjList { adj: Vec<Vec<u32>> }

impl AdjList {
    pub fn new(n: usize) -> Self { AdjList { adj: vec![vec![]; n] } }
    pub fn add_edge(&mut self, u: u32, v: u32) { self.adj[u as usize].push(v); }
    pub fn bfs(&self, src: u32) -> usize {
        let n = self.adj.len();
        let mut dist = vec![-1i32; n];
        dist[src as usize] = 0;
        let mut q: VecDeque<u32> = VecDeque::new();
        q.push_back(src);
        let mut reached = 0;
        while let Some(u) = q.pop_front() {
            reached += 1;
            for &v in &self.adj[u as usize] {
                if dist[v as usize] == -1 {
                    dist[v as usize] = dist[u as usize] + 1;
                    q.push_back(v);
                }
            }
        }
        reached
    }
}

pub struct CSR { row_starts: Vec<u32>, neighbors: Vec<u32> }

impl CSR {
    pub fn from_adj(a: &AdjList) -> Self {
        let n = a.adj.len();
        let mut row_starts = Vec::with_capacity(n + 1);
        let mut total = 0u32;
        for nb in &a.adj { row_starts.push(total); total += nb.len() as u32; }
        row_starts.push(total);
        let mut neighbors = Vec::with_capacity(total as usize);
        for nb in &a.adj { neighbors.extend_from_slice(nb); }
        CSR { row_starts, neighbors }
    }

    pub fn bfs(&self, src: u32) -> usize {
        let n = self.row_starts.len() - 1;
        let mut dist = vec![-1i32; n];
        dist[src as usize] = 0;
        let mut q: VecDeque<u32> = VecDeque::new();
        q.push_back(src);
        let mut reached = 0;
        while let Some(u) = q.pop_front() {
            reached += 1;
            let start = self.row_starts[u as usize] as usize;
            let end = self.row_starts[u as usize + 1] as usize;
            for &v in &self.neighbors[start..end] {
                if dist[v as usize] == -1 {
                    dist[v as usize] = dist[u as usize] + 1;
                    q.push_back(v);
                }
            }
        }
        reached
    }
}

fn main() {
    let n: usize = 1000;
    let m = 8000;
    let mut a = AdjList::new(n);
    // Deterministic edges for reproducibility
    let mut s: u64 = 42;
    for _ in 0..m {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let u = ((s >> 32) as usize) % n;
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let v = ((s >> 32) as usize) % n;
        a.add_edge(u as u32, v as u32);
    }
    let csr = CSR::from_adj(&a);

    let t = std::time::Instant::now();
    let mut r = 0;
    for _ in 0..1000 { r = a.bfs(0); }
    println!("AdjList BFS: {:?} avg, reached {r}", t.elapsed() / 1000);

    let t = std::time::Instant::now();
    for _ in 0..1000 { r = csr.bfs(0); }
    println!("CSR BFS:    {:?} avg, reached {r}", t.elapsed() / 1000);
}
