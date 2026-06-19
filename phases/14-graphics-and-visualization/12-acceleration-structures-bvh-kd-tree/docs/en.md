# Acceleration Structures — BVH, kd-Tree

> Without acceleration structures, ray tracing scales O(n) per ray — checking every primitive.
> With them, it scales O(log n). That's the difference between seconds and hours.

**Type:** Learn
**Languages:** Rust
**Prerequisites:** Phase 14 lessons 01–11
**Time:** ~75 minutes

## Learning Objectives

- Explain why naive ray tracing is O(n) per ray and why that matters at scale.
- Implement axis-aligned bounding boxes (AABBs) and the slab-method intersection test.
- Build a Bounding Volume Hierarchy (BVH) using centroid-median splits and the Surface Area Heuristic (SAH).
- Traverse a BVH for ray-scene intersection, skipping subtrees when the ray misses an AABB.
- Explain kd-trees as a spatial partitioning alternative and compare them to BVHs.
- Know when acceleration structures become necessary (scenes with >1000 primitives).

## The Problem

Ray tracing casts a ray from the camera through each pixel and finds the closest surface it hits. The naive implementation checks every primitive in the scene for every ray:

```
for each pixel:
    ray = generate_ray(pixel)
    for each primitive in scene:        # O(n) per ray!
        if ray hits primitive:
            update closest hit
```

With 1,000 primitives and a 640×480 image, that's 307,200,000 intersection tests. With 1,000,000 primitives, it's 307.2 billion. Naive ray tracing becomes impractical fast.

The fix: **don't check every primitive**. Organize them into a hierarchical structure so you can reject large groups with a single test. That's what acceleration structures do.

## The Concept

### Two Philosophies: Object vs. Spatial

Acceleration structures split into two families:

| | Object Partitioning | Spatial Partitioning |
|---|---|---|
| **Idea** | Group primitives into bounding volumes | Subdivide space into regions |
| **Example** | BVH (Bounding Volume Hierarchy) | kd-Tree, Octree |
| **Primitives** | Stored once in one node | May appear in multiple cells |
| **Update** | Easy to refit (move objects) | Harder to update |
| **Overlap** | Sibling AABBs may overlap | Cells partition space (no overlap) |

BVHs and kd-trees are the two most common choices in production renderers. Let's understand both.

### Axis-Aligned Bounding Boxes (AABBs)

An AABB is the smallest box aligned to the x, y, z axes that contains a primitive (or group of primitives):

```
      ┌──────────┐  ← max_y
      │          │
      │  Sphere  │
      │          │
      └──────────┘  ← min_y
      ^          ^
   min_x      max_x
```

For a sphere at center (cx, cy, cz) with radius r:

```
min = (cx - r, cy - r, cz - r)
max = (cx + r, cy + r, cz + r)
```

AABBs are cheap to intersect — much cheaper than the primitives they contain. That's the key insight: test the cheap box first, skip the expensive primitive test if the box is missed.

### Slab Method: AABB–Ray Intersection

The **slab method** tests whether a ray passes through an AABB. For each axis, the ray enters and exits the box at specific t values. The box is hit if all three intervals overlap:

```
Axis intervals:
  x: [t_x_min, t_x_max]    ← ray enters/leaves the x-slab
  y: [t_y_min, t_y_max]    ← ray enters/leaves the y-slab
  z: [t_z_min, t_z_max]    ← ray enters/leaves the z-slab

  t_enter = max(t_x_min, t_y_min, t_z_min)
  t_exit  = min(t_x_max, t_y_max, t_z_max)

  Hit if: t_enter < t_exit AND t_exit > 0
```

**Worked example:**

```
Ray: origin = (0, 0, 0), direction = (1, 0, 0)
AABB: min = (2, -1, -1), max = (5, 1, 1)

  x-axis: t_x_min = (2-0)/1 = 2,  t_x_max = (5-0)/1 = 5
  y-axis: t_y_min = (-1-0)/0 → -inf (ray parallel, in slab), t_y_max = +inf
  z-axis: t_z_min = (-1-0)/0 → -inf, t_z_max = +inf

  t_enter = max(2, -inf, -inf) = 2
  t_exit  = min(5, +inf, +inf) = 5
  2 < 5 → HIT at t = 2
```

When the ray direction component is zero (parallel to a slab), the ray is either entirely inside or entirely outside that slab. We handle it by setting t_min = -inf and t_max = +inf when inside.

### Bounding Volume Hierarchy (BVH)

A BVH organizes primitives into a binary tree of AABBs:

```
           [Root AABB]
          /            \
    [Left AABB]      [Right AABB]
     /       \        /        \
  [Leaf]   [Leaf]  [Leaf]    [Leaf]
   p0,p1    p2       p3      p4,p5
```

Each internal node stores an AABB that bounds all primitives in its subtree. Each leaf stores a small number of primitives (typically 1–4).

**Traversal:** Cast ray against root AABB. If miss, done. If hit, recurse into children. At a leaf, test the ray against each primitive.

```
BVH_intersect(ray, node):
    if ray misses node.aabb:
        return miss
    if node is leaf:
        return closest intersection with node.primitives
    hit_left  = BVH_intersect(ray, node.left)
    hit_right = BVH_intersect(ray, node.right)
    return closer of hit_left, hit_right
```

**Result:** Instead of O(n) per ray, you get O(log n) in the best case, O(n) worst (ray hits everything), and roughly O(log n) in practice.

### BVH Construction: Middle Split (Centroid Median)

The simplestBVH construction strategy:

1. Choose an axis (round-robin: x, y, z, x, ...)
2. Sort primitives by centroid along that axis
3. Split at the median — left half gets the first n/2 primitives
4. Recurse on each half

```
Before split (8 primitives along x):
  p1  p3  p2  p5  p4  p8  p6  p7
  ─────────────────────────────────→ x

After median split:
  Left:  p1, p3, p2, p4    Right: p5, p8, p6, p7
```

This is fast to build (O(n log n)) but doesn't produce optimal trees. Overlapping sibling AABBs mean some rays visit both children.

### BVH Construction: Surface Area Heuristic (SAH)

The SAH estimates the expected cost of a split. The intuition: a split that creates small, well-separated subtrees is better because rays are less likely to hit both.

**SAH cost model:**

```
C(split) = C_traversal
         + P(hit_left)  × C_left
         + P(hit_right) × C_right
```

Where:
- `C_traversal` is a fixed cost per node (~1.0)
- `P(hit_left) = surface_area(left_child) / surface_area(parent)`
- `C_left` is the cost of the left subtree (number of primitives if leaf)

**Surface area of an AABB:**

```
SA = 2×((max_x - min_x)(max_y - min_y) 
      + (max_y - min_y)(max_z - min_z)
      + (max_z - min_z)(max_x - min_x))
```

To build with SAH:
1. Try several split positions along each axis
2. Compute the SAH cost for each
3. Choose the split with the lowest cost
4. If the best SAH cost exceeds the cost of making this node a leaf, stop splitting

### kd-Tree: Spatial Partitioning

A kd-tree recursively splits space along alternating axes:

```
        ┌─────────────────┐
        │       │         │
        │  Left │  Right  │
        │       │         │
        └─────────────────┘
             split x=5

        ┌──────┬──────────┐
        │  L   │    R     │
        │      │  ┌──────┐│
        │      │  │  RR  ││
        │      │  │      ││
        │      │  └──────┘│
        └──────┴──────────┘
           x=5    y=3
```

Each node stores a splitting plane (e.g., "x = 5"). The left subtree contains everything to the left; the right subtree, everything to the right.

**Key difference from BVH:** kd-tree cells partition space — no overlap. A ray can definitively skip one side. But primitives that straddle the splitting plane must appear in both children.

### kd-Tree Ray Traversal

The classic algorithm (Havran, 2000) walks the tree in ray order:

```
kd_intersect(ray, node):
    t_split = (node.split_pos - ray.origin[node.axis]) / ray.direction[node.axis]
    if ray.direction[node.axis] > 0:
        first = node.left, second = node.right
    else:
        first = node.right, second = node.left

    if t_split >= ray.t_max:
        # ray ends before split — only visit first child
        return kd_intersect(ray, first)
    elif t_split <= ray.t_min:
        # ray starts after split — only visit second child
        return kd_intersect(ray, second)
    else:
        # ray crosses split — visit both, near side first
        hit = kd_intersect(ray, first)
        if hit and hit.t < t_split:
            return hit
        return kd_intersect(ray, second)
```

### BVH vs. kd-Tree: Comparison

| Property | BVH | kd-Tree |
|----------|-----|---------|
| **Partitioning** | Object (primitives in one node) | Spatial (space split, primitives may overlap) |
| **Construction** | O(n log n) simple, O(n log n) SAH | O(n log² n) with SAH |
| **Refitting** | Easy — update AABBs after transform | Hard — must rebuild |
| **Dynamic scenes** | Good — refit or rebuild per frame | Poor — rebuild needed |
| **Memory** | Lower (each primitive stored once) | Higher (primitives may be in multiple leaves) |
| **Query quality** | Slightly worse (overlapping nodes) | Better (spatial partitioning, no overlap) |
| **GPU traversal** | Stack-friendly, widely used | Harder to implement on GPU |
| **Standard in** | Embree, OptiX, Blender Cycles | PBRT (optional), older renderers |

**Takeaway:** BVHs dominate in modern real-time and GPU ray tracing because they're easier to update and traverse on GPU. kd-trees can produce slightly better traversal for static scenes but are rarely used in production today.

### When Acceleration Matters

| Scene size | Brute-force (ray) | BVH (ray) | Speedup |
|------------|-------------------|-----------|---------|
| 10 primitives | 10 tests | ~4 tests | ~2.5× |
| 100 primitives | 100 tests | ~7 tests | ~14× |
| 1,000 primitives | 1,000 tests | ~10 tests | ~100× |
| 1,000,000 primitives | 1,000,000 tests | ~20 tests | ~50,000× |

Acceleration structures matter when you have hundreds of primitives or more. For tiny scenes, brute force is fine — the overhead of BVH traversal can actually be slower.

## Build It

### Step 1: Minimal AABB and Ray

```rust
struct Vec3 { x: f64, y: f64, z: f64 }
struct Ray { origin: Vec3, direction: Vec3 }
struct Aabb { min: Vec3, max: Vec3 }
```

The slab method: compute entry/exit t values for all three axes, take the max of entries and min of exits. Hit if overlap exists.

### Step 2: BVH Construction

Start with middle split (centroid median). Then upgrade to SAH:

```rust
fn build_bvh(primitives: &[Sphere], depth: usize) -> BvhNode {
    if primitives.len() <= 2 {
        return BvhNode::Leaf { primitives, bounds };
    }
    let axis = depth % 3;
    let mid = primitives.len() / 2;
    // Sort by centroid along axis, split at mid
    let left = build_bvh(&primitives[..mid], depth + 1);
    let right = build_bvh(&primitives[mid..], depth + 1);
    BvhNode::Inner { left, right, bounds: union(left.bounds, right.bounds) }
}
```

### Step 3: BVH Traversal

Walk the tree, pruning subtrees whose AABB the ray misses:

```rust
fn intersect_bvh(ray: &Ray, node: &BvhNode) -> Option<Hit> {
    if !node.bounds.intersect(ray) { return None; }
    match node {
        BvhNode::Leaf { primitives } => intersect_all(ray, primitives),
        BvhNode::Inner { left, right, .. } => {
            let l = intersect_bvh(ray, left);
            let r = intersect_bvh(ray, right);
            closest(l, r)
        }
    }
}
```

### Step 4: Benchmark

Compare BVH-accelerated vs. brute-force on a scene with 100+ spheres. Print timing info.
Render a PPM image comparing results.

## Use It

Production renderers use BVHs extensively:

- **Embree** (Intel): Uses a two-level BVH with SAH-based construction. The inner nodes store compressed AABBs (8-bit quantized). Source: `kernel/bvh/` directory.
- **OptiX** (NVIDIA): GPU BVH traversal with hardware acceleration on RTX cards. The BVH is built on the GPU using morton codes (LBVH algorithm).
- **PBRT** (pbrt.org): Supports both BVH and kd-tree. The BVH implementation uses SAH with a binned approximation for O(n) construction. See `src/pbrt/accelerators/`.

These production implementations add:
- Quantized AABBs for cache efficiency
- Binning to approximate SAH in O(n) per node
- Morton-code-based (LBVH) construction for GPU
- Refitting: updating AABBs without rebuilding the tree

## Read the Source

- **PBRT v4** — `src/pbrt/accelerators/bvh.cpp`: SAH-based BVH construction with binned splitting. Look at `buildRecursive()` for the core logic.
- **Embree** — `kernels/bvh/bvh_builder.cpp`: Production BVH builder with SIMD-optimized AABB tests.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`bvh_reference.md`** — A reference card with BVH construction pseudocode, AABB intersection formula, SAH cost function, and BVH vs. kd-tree comparison table.

## Exercises

1. **Easy** — Reimplement the AABB slab-method intersection from memory. Test it against known ray-box pairs.
2. **Medium** — Add SAH-based splitting to the BVH constructor. Compare tree quality (number of nodes visited per ray) against the median-split version.
3. **Hard** — Implement a kd-tree with SAH-based splitting and empty-space optimization. Benchmark it against the BVH on the same scene. For extra credit, implement the Havran traversal algorithm.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| AABB | "axis-aligned bounding box" | The tightest box aligned to x/y/z axes that contains a shape |
| BVH | "bounding volume hierarchy" | A tree where each node's AABB bounds all primitives below it |
| kd-Tree | "k-dimensional tree" | A binary tree that recursively splits space along alternating axes |
| SAH | "surface area heuristic" | Cost model: prefer splits that minimize the expected number of ray-AABB tests |
| Slab method | "slab test" | Ray-AABB intersection by finding overlap of interval projections on each axis |
| Refitting | "updating the BVH" | Recomputing AABB bounds after objects move, without rebuilding the tree topology |
| Morton code | "Z-order curve" | An integer encoding of 3D position that preserves spatial locality — used for fast BVH construction |

## Further Reading

- **PBRT** — Chapter 4.3 (BVH) and 4.4 (kd-tree) in *Physically Based Rendering* by Pharr, Jakob, and Hanika.
- **Embree** — Intel's open-source ray tracing kernel library: github.com/RenderingResearch-Projects/Embree
- **Havran, 2000** — *Heuristic Ray Shooting Algorithms* — the definitive thesis on kd-tree traversal.
- **Karras, 2012** — *Maximizing Parallelism in the Construction of BVHs, Octrees, and kd-Trees* — the LBVH/morton-code paper used by OptiX and Embree.