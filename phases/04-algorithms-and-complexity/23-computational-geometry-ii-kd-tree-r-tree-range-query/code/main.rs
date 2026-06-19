//! Computational Geometry II — kd-Tree, R-Tree, Range Query
//! Phase 04 — Algorithms & Complexity Analysis
//!
//! kd-tree with build, nearest-neighbor, and range query.

use std::cmp::Ordering;
use std::time::Instant;

// ---------------------------------------------------------------------------
// kd-Tree
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct KdNode {
    point: [f64; 2],
    left: Option<Box<KdNode>>,
    right: Option<Box<KdNode>>,
}

pub struct KdTree {
    root: Option<Box<KdNode>>,
    size: usize,
}

impl KdTree {
    pub fn new(points: &[[f64; 2]]) -> Self {
        if points.is_empty() {
            return KdTree { root: None, size: 0 };
        }
        let mut pts = points.to_vec();
        let size = pts.len();
        let root = Self::build(&mut pts, 0);
        KdTree { root, size }
    }

    fn build(points: &mut [[f64; 2]], depth: usize) -> Option<Box<KdNode>> {
        if points.is_empty() {
            return None;
        }
        let axis = depth % 2;
        points.sort_by(|a, b| a[axis].partial_cmp(&b[axis]).unwrap_or(Ordering::Equal));
        let mid = points.len() / 2;
        let left = Self::build(&mut points[..mid].to_vec(), depth + 1);
        let right = Self::build(&mut points[mid + 1..].to_vec(), depth + 1);
        Some(Box::new(KdNode {
            point: points[mid],
            left,
            right,
        }))
    }

    /// Nearest-neighbor query. Returns the closest point and its squared distance.
    pub fn nearest_neighbor(&self, target: &[f64; 2]) -> Option<([f64; 2], f64)> {
        let mut best = [f64::MAX, 0.0, 0.0]; // [dist_sq, x, y]
        self.nn_inner(&self.root, target, 0, &mut best);
        if best[0] == f64::MAX {
            None
        } else {
            Some(([best[1], best[2]], best[0]))
        }
    }

    fn nn_inner(
        &self,
        node: &Option<Box<KdNode>>,
        target: &[f64; 2],
        depth: usize,
        best: &mut [f64; 3],
    ) {
        let n = match node {
            Some(n) => n,
            None => return,
        };
        let axis = depth % 2;
        let d_sq = dist_sq(&n.point, target);
        if d_sq < best[0] {
            best[0] = d_sq;
            best[1] = n.point[0];
            best[2] = n.point[1];
        }
        let diff = target[axis] - n.point[axis];
        let (close, far) = if diff <= 0.0 {
            (&n.left, &n.right)
        } else {
            (&n.right, &n.left)
        };
        self.nn_inner(close, target, depth + 1, best);
        if diff * diff < best[0] {
            self.nn_inner(far, target, depth + 1, best);
        }
    }

    /// Range query: return all points in [lo, hi] (axis-aligned rectangle).
    pub fn range_query(&self, lo: &[f64; 2], hi: &[f64; 2]) -> Vec<[f64; 2]> {
        let mut result = Vec::new();
        self.range_inner(&self.root, lo, hi, 0, &mut result);
        result
    }

    fn range_inner(
        &self,
        node: &Option<Box<KdNode>>,
        lo: &[f64; 2],
        hi: &[f64; 2],
        depth: usize,
        result: &mut Vec<[f64; 2]>,
    ) {
        let n = match node {
            Some(n) => n,
            None => return,
        };
        let axis = depth % 2;
        if (0..2).all(|i| lo[i] <= n.point[i] && n.point[i] <= hi[i]) {
            result.push(n.point);
        }
        if lo[axis] <= n.point[axis] {
            self.range_inner(&n.left, lo, hi, depth + 1, result);
        }
        if n.point[axis] <= hi[axis] {
            self.range_inner(&n.right, lo, hi, depth + 1, result);
        }
    }
}

// ---------------------------------------------------------------------------
// Brute-force NN
// ---------------------------------------------------------------------------

fn brute_force_nn(points: &[[f64; 2]], target: &[f64; 2]) -> ([f64; 2], f64) {
    points
        .iter()
        .map(|&p| (p, dist_sq(&p, target)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
        .unwrap()
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

fn dist_sq(a: &[f64; 2], b: &[f64; 2]) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    println!("=== Computational Geometry II — kd-Tree (Rust) ===\n");

    // Build
    let points = [
        [2.0, 3.0], [5.0, 4.0], [9.0, 6.0], [4.0, 7.0], [8.0, 1.0], [7.0, 2.0],
    ];
    let tree = KdTree::new(&points);
    println!("Built kd-tree with {} points.", tree.size);

    // Nearest neighbor
    let queries = [[6.0, 3.0], [1.0, 8.0], [9.0, 1.0]];
    println!("\nNearest neighbor queries:");
    for q in &queries {
        let (pt, d_sq) = tree.nearest_neighbor(q).unwrap();
        println!(
            "  ({:.0},{:.0}) -> ({:.0},{:.0})  dist={:.2}",
            q[0], q[1], pt[0], pt[1], d_sq.sqrt()
        );
    }

    // Range query
    let lo = [3.0, 1.0];
    let hi = [8.0, 5.0];
    let inside = tree.range_query(&lo, &hi);
    println!(
        "\nPoints in [{:.0},{:.0}]×[{:.0},{:.0}]: {:?}",
        lo[0], lo[1], hi[0], hi[1],
        inside.iter().map(|p| (p[0] as i64, p[1] as i64)).collect::<Vec<_>>()
    );

    // Verify against brute force
    let n = 500;
    let pts: Vec<[f64; 2]> = (0..n)
        .map(|i| {
            let s = i as f64 * 7.13 + 3.71;
            [(s * 1.31).fract() * 100.0, (s * 2.17).fract() * 100.0]
        })
        .collect();
    let tree2 = KdTree::new(&pts);

    let mut ok = 0usize;
    for i in 0..20 {
        let q = [(i as f64 * 13.7 + 1.1) % 100.0, (i as f64 * 7.3 + 2.9) % 100.0];
        let (kd_pt, kd_d) = tree2.nearest_neighbor(&q).unwrap();
        let (bf_pt, bf_d) = brute_force_nn(&pts, &q);
        if (kd_d - bf_d).abs() < 1e-9 {
            ok += 1;
        } else {
            println!(
                "  MISMATCH q={:?} kd={:?} bf={:?}",
                q, kd_pt, bf_pt
            );
        }
    }
    println!("\nVerification: {ok}/20 queries match brute force.");

    // Benchmark
    println!(
        "\n{:<10} {:>14} {:>14} {:>8}",
        "n", "kd-tree (ms)", "brute (ms)", "speedup"
    );
    println!("{}", "-".repeat(50));

    for &n in &[10_000, 100_000] {
        let pts: Vec<[f64; 2]> = (0..n)
            .map(|i| {
                let s = i as f64 * 7.13 + 3.71;
                [(s * 1.31).fract() * 1000.0, (s * 2.17).fract() * 1000.0]
            })
            .collect();
        let tree = KdTree::new(&pts);

        let queries: Vec<[f64; 2]> = (0..100)
            .map(|i| {
                let s = i as f64 * 3.31 + 1.17;
                [(s * 1.91).fract() * 1000.0, (s * 2.73).fract() * 1000.0]
            })
            .collect();

        let t0 = Instant::now();
        for q in &queries {
            let _ = tree.nearest_neighbor(q);
        }
        let t_kd = t0.elapsed().as_secs_f64() * 1000.0;

        let t0 = Instant::now();
        for q in &queries {
            let _ = brute_force_nn(&pts, q);
        }
        let t_bf = t0.elapsed().as_secs_f64() * 1000.0;

        let speedup = if t_kd > 0.0 { t_bf / t_kd } else { f64::INFINITY };
        println!("{n:<10} {t_kd:>13.2}  {t_bf:>13.2}  {speedup:>7.1}x");
    }
}
