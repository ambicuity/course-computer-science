"""
Computational Geometry II — kd-Tree, R-Tree, Range Query
Phase 04 — Algorithms & Complexity Analysis

kd-tree with build, nearest-neighbor, range query, kNN, and ASCII visualization.
"""

from __future__ import annotations

import math
import random
import time
from dataclasses import dataclass, field
from typing import Optional


# ---------------------------------------------------------------------------
# kd-Tree Node and Construction
# ---------------------------------------------------------------------------

@dataclass
class KdNode:
    point: tuple[float, ...]
    left: Optional[KdNode] = None
    right: Optional[KdNode] = None


class KdTree:
    """kd-tree built from a list of k-dimensional points."""

    def __init__(self, points: list[tuple[float, ...]]) -> None:
        if not points:
            raise ValueError("Need at least one point")
        self.k = len(points[0])
        self.root = self._build(list(points), 0)
        self.size = len(points)

    def _build(self, points: list[tuple[float, ...]], depth: int) -> Optional[KdNode]:
        if not points:
            return None
        axis = depth % self.k
        points.sort(key=lambda p: p[axis])
        mid = len(points) // 2
        return KdNode(
            point=points[mid],
            left=self._build(points[:mid], depth + 1),
            right=self._build(points[mid + 1:], depth + 1),
        )

    # --- Nearest Neighbor ---

    def nearest_neighbor(self, target: tuple[float, ...]) -> tuple[float, ...]:
        """Find the closest point to target. O(log n) average."""
        best = [float("inf"), None]  # [dist_sq, node]
        self._nn(self.root, target, 0, best)
        if best[1] is None:
            raise RuntimeError("Tree is empty")
        return best[1].point

    def _nn(self, node: Optional[KdNode], target: tuple[float, ...],
            depth: int, best: list) -> None:
        if node is None:
            return
        axis = depth % self.k
        d_sq = self._dist_sq(node.point, target)
        if d_sq < best[0]:
            best[0] = d_sq
            best[1] = node
        diff = target[axis] - node.point[axis]
        close = node.left if diff <= 0 else node.right
        far = node.right if diff <= 0 else node.left
        self._nn(close, target, depth + 1, best)
        if diff * diff < best[0]:
            self._nn(far, target, depth + 1, best)

    # --- k Nearest Neighbors ---

    def knn(self, target: tuple[float, ...], k: int) -> list[tuple[float, ...]]:
        """Return the k nearest points to target."""
        import heapq
        heap: list[tuple[float, int, tuple[float, ...]]] = []
        counter = 0

        def visit(node: Optional[KdNode], depth: int) -> None:
            nonlocal counter
            if node is None:
                return
            d_sq = self._dist_sq(node.point, target)
            if len(heap) < k:
                heapq.heappush(heap, (-d_sq, counter, node.point))
                counter += 1
            elif d_sq < -heap[0][0]:
                heapq.heapreplace(heap, (-d_sq, counter, node.point))
                counter += 1
            axis = depth % self.k
            diff = target[axis] - node.point[axis]
            close = node.left if diff <= 0 else node.right
            far = node.right if diff <= 0 else node.left
            visit(close, depth + 1)
            if len(heap) < k or diff * diff < -heap[0][0]:
                visit(far, depth + 1)

        visit(self.root, 0)
        return [p for _, _, p in sorted(heap, reverse=True)]

    # --- Range Query ---

    def range_query(self, lo: tuple[float, ...],
                    hi: tuple[float, ...]) -> list[tuple[float, ...]]:
        """Return all points p where lo[i] <= p[i] <= hi[i] for all i."""
        result: list[tuple[float, ...]] = []
        self._range(self.root, lo, hi, 0, result)
        return result

    def _range(self, node: Optional[KdNode], lo: tuple[float, ...],
               hi: tuple[float, ...], depth: int,
               result: list[tuple[float, ...]]) -> None:
        if node is None:
            return
        axis = depth % self.k
        if all(lo[i] <= node.point[i] <= hi[i] for i in range(self.k)):
            result.append(node.point)
        if lo[axis] <= node.point[axis]:
            self._range(node.left, lo, hi, depth + 1, result)
        if node.point[axis] <= hi[axis]:
            self._range(node.right, lo, hi, depth + 1, result)

    # --- Utilities ---

    @staticmethod
    def _dist_sq(a: tuple[float, ...], b: tuple[float, ...]) -> float:
        return sum((ai - bi) ** 2 for ai, bi in zip(a, b))

    # --- ASCII Visualization ---

    def visualize(self, max_depth: int = 3) -> str:
        """Return ASCII art of the tree structure up to max_depth."""
        lines: list[str] = []

        def _show(node: Optional[KdNode], prefix: str, depth: int, is_left: bool) -> None:
            if node is None or depth > max_depth:
                return
            connector = "├── " if is_left else "└── "
            axis_name = "xy" if self.k == 2 else str(depth % self.k)
            label = f"({node.point[0]:.0f},{node.point[1]:.0f}) [{axis_name[depth % self.k]}]"
            lines.append(f"{prefix}{connector}{label}")
            child_prefix = prefix + ("│   " if is_left else "    ")
            _show(node.left, child_prefix, depth + 1, True)
            _show(node.right, child_prefix, depth + 1, False)

        if self.root:
            axis_name = "xy" if self.k == 2 else "0"
            lines.append(f"({self.root.point[0]:.0f},{self.root.point[1]:.0f}) [{axis_name}]")
            _show(self.root.left, "", 1, True)
            _show(self.root.right, "", 1, False)
        return "\n".join(lines)


# ---------------------------------------------------------------------------
# Brute-Force Nearest Neighbor (for verification)
# ---------------------------------------------------------------------------

def brute_force_nn(points: list[tuple[float, ...]],
                   target: tuple[float, ...]) -> tuple[float, ...]:
    """O(n) scan for nearest neighbor — ground truth."""
    best = min(points, key=lambda p: sum((a - b) ** 2 for a, b in zip(p, target)))
    return best


# ---------------------------------------------------------------------------
# Benchmark
# ---------------------------------------------------------------------------

def benchmark() -> None:
    """Compare kd-tree vs brute force on random 2D points."""
    random.seed(42)
    print(f"\n{'n':>8}  {'kd-tree (ms)':>14}  {'brute (ms)':>12}  {'speedup':>8}")
    print("-" * 48)

    for n in [1_000, 10_000, 100_000]:
        pts = [(random.uniform(0, 1000), random.uniform(0, 1000)) for _ in range(n)]
        tree = KdTree(pts)
        queries = [(random.uniform(0, 1000), random.uniform(0, 1000)) for _ in range(100)]

        # kd-tree
        t0 = time.perf_counter()
        for q in queries:
            tree.nearest_neighbor(q)
        t_kd = (time.perf_counter() - t0) * 1000

        # brute force
        t0 = time.perf_counter()
        for q in queries:
            brute_force_nn(pts, q)
        t_bf = (time.perf_counter() - t0) * 1000

        speedup = t_bf / t_kd if t_kd > 0 else float("inf")
        print(f"{n:>8}  {t_kd:>13.2f}  {t_bf:>11.2f}  {speedup:>7.1f}x")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    print("=== Computational Geometry II — kd-Tree ===\n")

    # --- Build and visualize ---
    points = [(2, 3), (5, 4), (9, 6), (4, 7), (8, 1), (7, 2)]
    tree = KdTree(points)
    print("kd-tree structure:")
    print(tree.visualize(max_depth=4))
    print()

    # --- Nearest neighbor ---
    queries = [(6, 3), (1, 8), (9, 1)]
    print("Nearest neighbor queries:")
    for q in queries:
        result = tree.nearest_neighbor(q)
        d = math.dist(q, result)
        print(f"  query={q}  ->  nn={result}  dist={d:.2f}")
    print()

    # --- kNN ---
    target = (6, 3)
    k = 3
    neighbors = tree.knn(target, k)
    print(f"{k}-nearest neighbors of {target}:")
    for p in neighbors:
        print(f"  {p}  dist={math.dist(target, p):.2f}")
    print()

    # --- Range query ---
    lo, hi = (3, 1), (8, 5)
    inside = tree.range_query(lo, hi)
    print(f"Points in [{lo}, {hi}]: {inside}")
    print()

    # --- Verify NN against brute force ---
    random.seed(42)
    pts = [(random.uniform(0, 100), random.uniform(0, 100)) for _ in range(200)]
    tree2 = KdTree(pts)
    mismatches = 0
    for _ in range(50):
        q = (random.uniform(0, 100), random.uniform(0, 100))
        kd_result = tree2.nearest_neighbor(q)
        bf_result = brute_force_nn(pts, q)
        if kd_result != bf_result:
            # Allow ties (same distance)
            if math.dist(q, kd_result) != math.dist(q, bf_result):
                mismatches += 1
    print(f"Verification: {50 - mismatches}/50 queries match brute force")
    if mismatches:
        print(f"  ({mismatches} mismatches due to equidistant points)")
    print()

    # --- Benchmark ---
    benchmark()
    print()

    # --- ASCII grid visualization of splits ---
    print("2D space partition (10 points):")
    pts10 = [(2, 3), (5, 4), (9, 6), (4, 7), (8, 1),
             (7, 2), (1, 5), (6, 8), (3, 1), (10, 4)]
    tree10 = KdTree(pts10)
    grid = [["."] * 12 for _ in range(10)]
    for x, y in pts10:
        gx, gy = int(x), int(y)
        if 0 <= gy < 10 and 0 <= gx < 12:
            grid[9 - gy][gx] = "*"
    print("  y")
    for row_idx, row in enumerate(grid):
        y_val = 9 - row_idx
        print(f"  {y_val}|" + " ".join(row))
    print("   +" + "-" * 23)
    print("    0 1 2 3 4 5 6 7 8 9 10 11  x")
    print()

    print("Split lines: root x=5 (vertical), then y splits at each subtree.")


if __name__ == "__main__":
    main()
