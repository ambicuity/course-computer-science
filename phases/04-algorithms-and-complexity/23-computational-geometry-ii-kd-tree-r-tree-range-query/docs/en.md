# Computational Geometry II — kd-Tree, R-Tree, Range Query

> Spatial data structures that turn "which points are near me?" from O(n) to O(log n).

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 04 lessons 01–22
**Time:** ~75 minutes

## Learning Objectives

- Build a kd-tree from scratch, understanding the alternating split dimension rule.
- Implement nearest-neighbor and range queries on a kd-tree.
- Explain R-tree structure and why spatial databases use bounding rectangles.
- Compare kd-tree nearest-neighbor search against brute force and measure the speedup.
- Identify real-world uses: game engines, GIS databases, kNN classifiers.

## The Problem

Lesson 22 covered convex hulls and sweep line — problems about *sets* of points processed once. But many applications need to *repeatedly* query a fixed set of points: "what's the closest point to x?", "which points fall inside this rectangle?" Brute force scans all n points per query — O(n). With millions of points and thousands of queries, that's unusable.

We need a spatial index — a data structure built once in O(n log n) that answers queries in O(log n) average time.

## The Concept

### kd-Tree: Binary Space Partition

A kd-tree is a binary tree that splits points by alternating dimensions at each level. At the root, split on x-coordinate; at depth 1, split on y; at depth 2, back to x; and so on.

**Construction (median split):**
1. Pick the dimension: `depth % k` (for 2D, alternate x and y).
2. Sort points by that dimension, take the median as the root.
3. Recurse: left subtree gets points below median, right gets points above.

```
Points: (2,3), (5,4), (9,6), (4,7), (8,1), (7,2)

            split on x
         (7,2)  ← median x
        /       \
   split on y  split on y
   (4,7)       (9,6)
   /   \           \
 (2,3) (5,4)      (8,1)
```

**Complexity:** Build O(n log n). Nearest-neighbor O(log n) average, O(n) worst. Range query O(√n + r) average in 2D.

### Nearest-Neighbor Search

The key insight is **pruning**: at each node, check whether the *opposite* subtree's bounding region could contain a closer point. If the current best distance is less than the perpendicular distance to the opposite split plane, skip that subtree entirely.

```
query = (6, 3)
1. Visit root (7,2). dist = √2. Best = (7,2).
2. Go left (split on y): (4,7). dist = √19. Best still (7,2).
   Prune left subtrees? Perpendicular distance to y-split = |7-3| = 4.
   √2 < 4, so prune (2,3) and (5,4)!
3. Go right: (9,6). dist = √10. Best still (7,2).
   Check (8,1). dist = √5. Best = (8,1), dist = √5 ≈ 2.24.
```

### R-Tree: Bounding Rectangles

An R-tree groups nearby objects into minimum bounding rectangles (MBRs). Each internal node stores an MBR covering all its children. Unlike kd-trees, R-trees are balanced — every leaf is at the same depth.

**Properties:** Each non-root node has m–M children (M typically 4–100). MBRs at the same level may overlap. Insert picks the leaf whose MBR needs least enlargement; on overflow, split using a heuristic.

**Split heuristics:** Linear split (O(n), max normalized separation), Quadratic split (O(n²), pick pair wasting most area), R\*-tree (forced reinsertion before splitting).

**Used in:** PostGIS (`GIST` index on geometry columns), SQLite R\*-Tree extension, MongoDB 2dsphere index.

### Range Queries

Given an axis-aligned rectangle [x1,x2] × [y1,y2], recurse only into subtrees whose bounding region intersects the query box. 1D range on a balanced BST: O(log n + r). 2D range on kd-tree: O(√n + r) average. On R-tree: O(log n + r) when overlap is low.

## Build It

### Step 1: Minimal kd-Tree

Build a kd-tree by sorting on the split dimension and taking the median.

```python
from __future__ import annotations
from dataclasses import dataclass

@dataclass
class Node:
    point: tuple[float, ...]
    left: Node | None = None
    right: Node | None = None

def build(points: list[tuple[float, ...]], depth: int = 0) -> Node | None:
    if not points:
        return None
    k = len(points[0])
    axis = depth % k
    points.sort(key=lambda p: p[axis])
    mid = len(points) // 2
    return Node(
        point=points[mid],
        left=build(points[:mid], depth + 1),
        right=build(points[mid + 1:], depth + 1),
    )
```

### Step 2: Nearest-Neighbor with Pruning

```python
import math

def nn(node: Node | None, target: tuple[float, ...], depth: int = 0,
       best: tuple[float, Node | None] = (float('inf'), None)) -> tuple[float, ...]:
    if node is None:
        return best[1].point if best[1] else None
    k = len(target)
    axis = depth % k
    d = math.dist(node.point, target)
    if d < best[0]:
        best = (d, node)
    diff = target[axis] - node.point[axis]
    close, far = (node.left, node.right) if diff <= 0 else (node.right, node.left)
    best = _update_best(nn(close, target, depth + 1, best), best)
    if abs(diff) < best[0]:
        best = _update_best(nn(far, target, depth + 1, best), best)
    return best[1].point if best[1] else None
```

### Step 3: Range Query

```python
def range_query(node: Node | None, lo: tuple[float, ...], hi: tuple[float, ...],
                depth: int = 0, result: list | None = None) -> list[tuple[float, ...]]:
    if result is None:
        result = []
    if node is None:
        return result
    k = len(lo)
    axis = depth % k
    if all(lo[i] <= node.point[i] <= hi[i] for i in range(k)):
        result.append(node.point)
    if lo[axis] <= node.point[axis]:
        range_query(node.left, lo, hi, depth + 1, result)
    if node.point[axis] <= hi[axis]:
        range_query(node.right, lo, hi, depth + 1, result)
    return result
```

## Use It

- **SciPy:** `scipy.spatial.KDTree` — C-backed, supports k-nearest-neighbor and radius queries.
- **nanoflann** (C++): Header-only kd-tree used in ROS, Point Cloud Library.
- **SQLite:** `CREATE VIRTUAL TABLE ... USING rtree(...)` — R\*-Tree for geospatial queries.
- **PostGIS:** `CREATE INDEX ... USING GIST(geom)` — GiST index (generalized R-tree).
- **Game engines:** kd-trees for ray tracing, R-trees for collision detection broad phase.
- **sklearn:** `KDTree` and `BallTree` back the kNN classifier.

Production kd-trees add: leaf-size cutoff (8–32 points, scan linearly) and cache-friendly memory layout to avoid sqrt.

## Read the Source

- **SciPy KDTree:** `scipy/spatial/_ckdtree.pyx` — Cython kd-tree with kNN and box queries.
- **SQLite R-Tree:** `ext/rtree/rtree.c` — R\*-tree module, ~3000 lines.

## Ship It

`outputs/` contains **a kd-tree module with build, NN, range query, and kNN** — reuse in later phases for spatial indexing and kNN classification.

## Exercises

1. **Easy** — Implement k-nearest neighbors (kNN) using the kd-tree: return the k closest points to a query. Use a max-heap of size k to track candidates.
2. **Medium** — Generate 10,000 random 2D points. Benchmark kd-tree NN vs brute-force NN. Report speedup ratio. At what n does kd-tree start winning?
3. **Hard** — Implement R-tree insert with quadratic split: on node overflow, pick the two seed entries (max wasted area), distribute remaining entries to the MBR needing least enlargement.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| kd-tree | "k-dimensional tree" | Binary tree splitting on alternating axes at each level |
| Bounding rectangle | "MBR" | Minimum axis-aligned box enclosing all child entries |
| Pruning | "Skip branches that can't help" | If split-plane distance > best distance, skip opposite subtree |
| R-tree | "Spatial index" | Balanced tree grouping objects into overlapping MBRs |
| Range query | "Find points in a box" | Report all points inside [lo, hi]^k in sublinear time |

## Further Reading

- de Berg et al., *Computational Geometry*, Ch. 5 — kd-trees and range trees.
- Guttman, "R-Trees: A Dynamic Index Structure for Spatial Searching," 1984.
- Bentley, "Multidimensional Binary Search Trees Used for Associative Searching," 1975.
