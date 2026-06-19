//! Graph Algorithms II — Dijkstra, Bellman-Ford, A*
//! Phase 04 — Algorithms & Complexity Analysis
//!
//! Dijkstra and Bellman-Ford with path reconstruction using BinaryHeap.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

// ---------------------------------------------------------------------------
// Dijkstra
// ---------------------------------------------------------------------------

#[derive(Eq, PartialEq)]
struct State {
    cost: i64,
    node: usize,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost) // min-heap via reversed ordering
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn dijkstra(adj: &[Vec<(usize, i64)>], src: usize) -> (Vec<i64>, Vec<Option<usize>>) {
    let n = adj.len();
    let mut dist = vec![i64::MAX; n];
    let mut prev = vec![None; n];
    dist[src] = 0;

    let mut pq = BinaryHeap::new();
    pq.push(State { cost: 0, node: src });

    while let Some(State { cost, node }) = pq.pop() {
        if cost > dist[node] {
            continue;
        }
        for &(v, w) in &adj[node] {
            let nd = cost + w;
            if nd < dist[v] {
                dist[v] = nd;
                prev[v] = Some(node);
                pq.push(State { cost: nd, node: v });
            }
        }
    }

    (dist, prev)
}

pub fn reconstruct_path(prev: &[Option<usize>], target: usize) -> Vec<usize> {
    let mut path = Vec::new();
    let mut cur = Some(target);
    while let Some(node) = cur {
        path.push(node);
        cur = prev[node];
    }
    path.reverse();
    path
}

// ---------------------------------------------------------------------------
// Bellman-Ford
// ---------------------------------------------------------------------------

pub fn bellman_ford(
    edges: &[(usize, usize, i64)],
    v: usize,
    src: usize,
) -> (Vec<i64>, Vec<Option<usize>>, Option<Vec<usize>>) {
    let mut dist = vec![i64::MAX; v];
    let mut prev = vec![None; v];
    dist[src] = 0;

    for _ in 0..(v - 1) {
        let mut updated = false;
        for &(u, vtx, w) in edges {
            if dist[u] != i64::MAX && dist[u] + w < dist[vtx] {
                dist[vtx] = dist[u] + w;
                prev[vtx] = Some(u);
                updated = true;
            }
        }
        if !updated {
            break;
        }
    }

    // Negative cycle detection
    for &(u, vtx, w) in edges {
        if dist[u] != i64::MAX && dist[u] + w < dist[vtx] {
            let mut cycle_node = vtx;
            for _ in 0..v {
                cycle_node = prev[cycle_node].unwrap_or(0);
            }
            let mut cycle = Vec::new();
            let mut node = cycle_node;
            loop {
                cycle.push(node);
                node = prev[node].unwrap_or(0);
                if node == cycle_node {
                    cycle.push(node);
                    break;
                }
            }
            cycle.reverse();
            return (dist, prev, Some(cycle));
        }
    }

    (dist, prev, None)
}

// ---------------------------------------------------------------------------
// Demo
// ---------------------------------------------------------------------------

fn main() {
    // --- Dijkstra demo ---
    println!("{}", "=".repeat(60));
    println!("DIJKSTRA'S ALGORITHM");
    println!("{}", "=".repeat(60));

    let adj = vec![
        vec![(1, 4), (2, 1)],
        vec![(3, 1)],
        vec![(1, 2), (3, 5)],
        vec![(4, 3)],
        vec![],
    ];

    let (dist, prev) = dijkstra(&adj, 0);
    println!("Shortest distances from 0: {:?}", dist);
    let path = reconstruct_path(&prev, 4);
    println!("Path 0 -> 4: {:?}", path);
    let path1 = reconstruct_path(&prev, 1);
    println!("Path 0 -> 1: {:?}", path1);

    // --- Bellman-Ford demo ---
    println!();
    println!("{}", "=".repeat(60));
    println!("BELLMAN-FORD ALGORITHM");
    println!("{}", "=".repeat(60));

    let edges = vec![
        (0, 1, 4), (0, 2, 1),
        (1, 3, 1), (2, 1, 2),
        (2, 3, 5), (3, 4, 3),
    ];
    let (dist_bf, prev_bf, cycle) = bellman_ford(&edges, 5, 0);
    println!("Shortest distances from 0: {:?}", dist_bf);
    let path_bf = reconstruct_path(&prev_bf, 4);
    println!("Path 0 -> 4: {:?}", path_bf);

    // Negative cycle demo
    let neg_edges = vec![
        (0, 1, 1), (1, 2, -3),
        (2, 3, -1), (3, 1, 2),
    ];
    let (_, _, cycle) = bellman_ford(&neg_edges, 4, 0);
    println!("Negative cycle detected: {:?}", cycle);

    // No negative cycle
    let safe_edges = vec![
        (0, 1, 1), (1, 2, 2),
        (2, 3, 3),
    ];
    let (_, _, cycle) = bellman_ford(&safe_edges, 4, 0);
    println!("Negative cycle (should be None): {:?}", cycle);

    // --- Correctness checks ---
    println!();
    println!("{}", "=".repeat(60));
    println!("CORRECTNESS CHECKS");
    println!("{}", "=".repeat(60));

    // Verify Dijkstra == Bellman-Ford on non-negative graph
    let v = 50;
    let test_edges = vec![
        (0, 1, 4), (0, 2, 1), (1, 3, 1), (2, 1, 2),
        (2, 3, 5), (3, 4, 3), (4, 5, 2), (1, 5, 10),
    ];
    let mut test_adj = vec![vec![]; v];
    for &(u, vtx, w) in &test_edges {
        test_adj[u].push((vtx, w));
    }
    let (d_dij, _) = dijkstra(&test_adj, 0);
    let (d_bf, _, _) = bellman_ford(&test_edges, v, 0);
    let match_ok = d_dij.iter().zip(d_bf.iter()).all(|(a, b)| a == b);
    println!("Dijkstra == Bellman-Ford on non-negative: {}", match_ok);

    println!();
    println!("All demos complete.");
    println!("\nRun `cargo test` to verify correctness.");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dijkstra_simple() {
        let adj = vec![
            vec![(1, 4), (2, 1)],
            vec![(3, 1)],
            vec![(1, 2), (3, 5)],
            vec![(4, 3)],
            vec![],
        ];
        let (dist, prev) = dijkstra(&adj, 0);
        assert_eq!(dist[0], 0);
        assert_eq!(dist[1], 3);
        assert_eq!(dist[2], 1);
        assert_eq!(dist[3], 4);
        assert_eq!(dist[4], 7);

        let path = reconstruct_path(&prev, 4);
        assert_eq!(path, vec![0, 1, 3, 4]);
    }

    #[test]
    fn dijkstra_disconnected() {
        let adj = vec![vec![(1, 5)], vec![], vec![]];
        let (dist, _) = dijkstra(&adj, 0);
        assert_eq!(dist[0], 0);
        assert_eq!(dist[1], 5);
        assert_eq!(dist[2], i64::MAX);
    }

    #[test]
    fn bellman_ford_no_negative() {
        let edges = vec![
            (0, 1, 4), (0, 2, 1),
            (1, 3, 1), (2, 1, 2),
            (2, 3, 5), (3, 4, 3),
        ];
        let (dist, prev, cycle) = bellman_ford(&edges, 5, 0);
        assert!(cycle.is_none());
        assert_eq!(dist[4], 8);

        let path = reconstruct_path(&prev, 4);
        assert_eq!(path, vec![0, 1, 3, 4]);
    }

    #[test]
    fn bellman_ford_negative_cycle() {
        let edges = vec![
            (0, 1, 1), (1, 2, -3),
            (2, 3, -1), (3, 1, 2),
        ];
        let (_, _, cycle) = bellman_ford(&edges, 4, 0);
        assert!(cycle.is_some());
        let c = cycle.unwrap();
        assert!(c.len() >= 2);
    }

    #[test]
    fn bellman_ford_no_cycle_safe() {
        let edges = vec![
            (0, 1, 1), (1, 2, 2),
            (2, 3, 3),
        ];
        let (_, _, cycle) = bellman_ford(&edges, 4, 0);
        assert!(cycle.is_none());
    }

    #[test]
    fn dijkstra_matches_bellman_ford() {
        let edges = vec![
            (0, 1, 4), (0, 2, 1), (1, 3, 1), (2, 1, 2),
            (2, 3, 5), (3, 4, 3), (4, 5, 2), (1, 5, 10),
        ];
        let mut adj = vec![vec![]; 6];
        for &(u, v, w) in &edges {
            adj[u].push((v, w));
        }
        let (d_dij, _) = dijkstra(&adj, 0);
        let (d_bf, _, cycle) = bellman_ford(&edges, 6, 0);
        assert!(cycle.is_none());
        for i in 0..6 {
            assert_eq!(d_dij[i], d_bf[i], "mismatch at vertex {}", i);
        }
    }
}
