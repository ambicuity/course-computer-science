//! Minimum Spanning Trees — Prim, Kruskal, Borůvka
//! Phase 04 — Algorithms & Complexity Analysis
//!
//! Kruskal's and Prim's implementations in Rust.

use std::collections::BinaryHeap;
use std::cmp::Reverse;

// ---------------------------------------------------------------------------
// Union-Find with path compression + union by rank
// ---------------------------------------------------------------------------

struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u32>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    fn find(&mut self, x: usize) -> usize {
        let mut x = x;
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]]; // path splitting
            x = self.parent[x];
        }
        x
    }

    fn union(&mut self, x: usize, y: usize) -> bool {
        let mut rx = self.find(x);
        let mut ry = self.find(y);
        if rx == ry {
            return false;
        }
        if self.rank[rx] < self.rank[ry] {
            std::mem::swap(&mut rx, &mut ry);
        }
        self.parent[ry] = rx;
        if self.rank[rx] == self.rank[ry] {
            self.rank[rx] += 1;
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Kruskal's
// ---------------------------------------------------------------------------

type Edge = (usize, usize, i64); // (u, v, weight)

fn kruskal(edges: &mut [Edge], v: usize) -> (Vec<Edge>, i64) {
    edges.sort_by_key(|e| e.2);
    let mut uf = UnionFind::new(v);
    let mut mst: Vec<Edge> = Vec::new();
    let mut total: i64 = 0;

    for &(u, vtx, w) in edges.iter() {
        if uf.union(u, vtx) {
            mst.push((u, vtx, w));
            total += w;
            if mst.len() == v - 1 {
                break;
            }
        }
    }
    (mst, total)
}

// ---------------------------------------------------------------------------
// Prim's
// ---------------------------------------------------------------------------

fn prim(adj: &[Vec<(usize, i64)>], v: usize) -> (Vec<Edge>, i64) {
    let mut visited = vec![false; v];
    let mut heap: BinaryHeap<Reverse<(i64, usize, usize)>> = BinaryHeap::new();
    let mut mst: Vec<Edge> = Vec::new();
    let mut total: i64 = 0;

    visited[0] = true;
    for &(nb, w) in &adj[0] {
        heap.push(Reverse((w, 0, nb)));
    }

    while let Some(Reverse((w, u, vtx))) = heap.pop() {
        if visited[vtx] {
            continue;
        }
        visited[vtx] = true;
        mst.push((u, vtx, w));
        total += w;
        if mst.len() == v - 1 {
            break;
        }
        for &(nb, nw) in &adj[vtx] {
            if !visited[nb] {
                heap.push(Reverse((nw, vtx, nb)));
            }
        }
    }

    (mst, total)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn edges_to_adjacency(edges: &[Edge], v: usize) -> Vec<Vec<(usize, i64)>> {
    let mut adj: Vec<Vec<(usize, i64)>> = vec![Vec::new(); v];
    for &(u, vtx, w) in edges {
        adj[u].push((vtx, w));
        adj[vtx].push((u, w));
    }
    adj
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let mut raw_edges: Vec<Edge> = vec![
        (0, 1, 1), (0, 4, 3),
        (1, 2, 2), (1, 3, 4), (1, 5, 6),
        (2, 3, 5), (2, 5, 2),
        (3, 4, 7), (3, 5, 3),
        (4, 5, 5),
    ];
    let v = 6;

    println!("=== Kruskal's ===");
    let (mst_k, w_k) = kruskal(&mut raw_edges, v);
    for &(u, vtx, w) in &mst_k {
        println!("  {} -- {}  weight {}", u, vtx, w);
    }
    println!("  Total weight: {}\n", w_k);

    let adj = edges_to_adjacency(&raw_edges, v);

    println!("=== Prim's ===");
    let (mst_p, w_p) = prim(&adj, v);
    for &(u, vtx, w) in &mst_p {
        println!("  {} -- {}  weight {}", u, vtx, w);
    }
    println!("  Total weight: {}\n", w_p);

    assert_eq!(w_k, w_p, "Weight mismatch: kruskal={} prim={}", w_k, w_p);
    println!("Both algorithms agree: MST weight = {}", w_k);
}
