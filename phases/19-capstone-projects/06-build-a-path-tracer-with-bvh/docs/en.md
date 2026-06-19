# Build a Path Tracer with BVH

> Rendering speed comes from pruning work before shading it.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 19 lessons 01-05
**Time:** ~600 minutes

## Learning Objectives

- Build core ray/object intersection flow.
- Add BVH acceleration structure reasoning and traversal.
- Validate image correctness against tiny scenes.
- Plan progressive optimizations and sampling improvements.

## The Problem

Naive path tracers test every ray against every primitive. For a scene with 10,000 triangles and 100 samples per pixel, that's 10,000 × 100 × (number of pixels) × (bounces per ray) intersection tests per frame. At 1920x1080 with 4 bounces, that's roughly 8 trillion intersection tests. Each test involves solving a quadratic equation and checking bounds. This is too slow for anything beyond toy scenes.

The solution: organize geometry into a hierarchy of bounding volumes. Before testing a ray against 1,000 triangles, first test it against a box that contains all 1,000 triangles. If the ray misses the box, skip all 1,000 tests. This is the Bounding Volume Hierarchy (BVH).

A BVH is a binary tree. Each internal node stores an axis-aligned bounding box (AABB) that contains all geometry in its subtree. Leaf nodes store a small number of primitives (typically 1-4). A ray traversal descends the tree, testing against node AABBs and skipping subtrees whose boxes the ray misses. For a well-balanced BVH, this reduces intersection tests from O(n) to O(log n) per ray.

## The Concept

The path tracing pipeline has four stages:

```
For each pixel (x, y):
  │
  ▼
┌──────────────────┐
│ 1. Camera ray     │  Generate ray from camera through pixel
└──────────────────┘
  │
  ▼
┌──────────────────┐
│ 2. Scene query    │  Find closest intersection (BVH traversal)
└──────────────────┘
  │
  ▼
┌──────────────────┐
│ 3. Shade          │  Compute color: emit + reflect + bounce
└──────────────────┘
  │
  ▼
┌──────────────────┐
│ 4. Accumulate     │  Add sample to pixel, average over samples
└──────────────────┘
```

The BVH structure:

```
          [AABB: entire scene]
         /                    \
  [AABB: left half]     [AABB: right half]
   /         \            /         \
[Leaf:      [Leaf:     [Leaf:      [Leaf:
 tri 0-3]   tri 4-7]   tri 8-11]  tri 12-15]
```

Build strategy: sort primitives along one axis (we pick the longest axis of the scene AABB), split at the median, recurse. This gives a balanced tree with O(log n) depth.

## Build It

### Step 1: Core Math Types

```rust
use std::f64::INFINITY;

#[derive(Debug, Clone, Copy)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vec3 {
    fn new(x: f64, y: f64, z: f64) -> Self { Vec3 { x, y, z } }
    fn add(self, o: Vec3) -> Vec3 { Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z) }
    fn sub(self, o: Vec3) -> Vec3 { Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z) }
    fn scale(self, t: f64) -> Vec3 { Vec3::new(self.x * t, self.y * t, self.z * t) }
    fn dot(self, o: Vec3) -> f64 { self.x * o.x + self.y * o.y + self.z * o.z }
    fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    fn length(self) -> f64 { self.dot(self).sqrt() }
    fn normalize(self) -> Vec3 { let l = self.length(); self.scale(1.0 / l) }
}

type Color = Vec3;

#[derive(Debug, Clone, Copy)]
struct Ray {
    origin: Vec3,
    direction: Vec3,
}

impl Ray {
    fn point_at(&self, t: f64) -> Vec3 {
        self.origin.add(self.direction.scale(t))
    }
}
```

### Step 2: Triangle Intersection and AABB

```rust
#[derive(Debug, Clone)]
struct Triangle {
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    color: Color,
}

impl Triangle {
    // Moller-Trumbore intersection algorithm
    fn intersect(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<f64> {
        let edge1 = self.v1.sub(self.v0);
        let edge2 = self.v2.sub(self.v0);
        let h = ray.direction.cross(edge2);
        let a = edge1.dot(h);

        if a > -1e-8 && a < 1e-8 {
            return None; // Ray parallel to triangle
        }

        let f = 1.0 / a;
        let s = ray.origin.sub(self.v0);
        let u = f * s.dot(h);
        if u < 0.0 || u > 1.0 {
            return None;
        }

        let q = s.cross(edge1);
        let v = f * ray.direction.dot(q);
        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let t = f * edge2.dot(q);
        if t >= t_min && t <= t_max {
            Some(t)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct AABB {
    min: Vec3,
    max: Vec3,
}

impl AABB {
    fn new(min: Vec3, max: Vec3) -> Self { AABB { min, max } }

    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> bool {
        // Slab test for each axis
        for axis in 0..3 {
            let (orig, dir, lo, hi) = match axis {
                0 => (ray.origin.x, ray.direction.x, self.min.x, self.max.x),
                1 => (ray.origin.y, ray.direction.y, self.min.y, self.max.y),
                _ => (ray.origin.z, ray.direction.z, self.min.z, self.max.z),
            };

            let inv_d = 1.0 / dir;
            let mut t0 = (lo - orig) * inv_d;
            let mut t1 = (hi - orig) * inv_d;
            if inv_d < 0.0 { std::mem::swap(&mut t0, &mut t1); }

            let t_min = t0.max(t_min);
            let t_max = t1.min(t_max);
            if t_max <= t_min { return false; }
        }
        true
    }

    fn surrounding_box(&self, other: &AABB) -> AABB {
        AABB::new(
            Vec3::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
                self.min.z.min(other.min.z),
            ),
            Vec3::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
                self.max.z.max(other.max.z),
            ),
        )
    }
}
```

### Step 3: BVH Construction and Traversal

```rust
enum BVHNode {
    Leaf {
        aabb: AABB,
        primitives: Vec<usize>, // Indices into scene's triangle list
    },
    Internal {
        aabb: AABB,
        left: Box<BVHNode>,
        right: Box<BVHNode>,
    },
}

struct BVH {
    root: BVHNode,
}

impl BVH {
    fn build(triangles: &[Triangle]) -> Self {
        let indices: Vec<usize> = (0..triangles.len()).collect();
        let root = Self::build_node(triangles, &indices);
        BVH { root }
    }

    fn build_node(triangles: &[Triangle], indices: &[usize]) -> BVHNode {
        // Compute AABB for all primitives in this node
        let mut aabb = Self::compute_aabb(triangles, indices);

        // Base case: small number of primitives
        if indices.len() <= 4 {
            return BVHNode::Leaf {
                aabb,
                primitives: indices.to_vec(),
            };
        }

        // Find longest axis
        let extent = aabb.max.sub(aabb.min);
        let axis = if extent.x > extent.y && extent.x > extent.z { 0 }
                   else if extent.y > extent.z { 1 } else { 2 };

        // Sort along longest axis
        let mut sorted = indices.to_vec();
        sorted.sort_by(|&a, &b| {
            let ca = Self::centroid(&triangles[a]);
            let cb = Self::centroid(&triangles[b]);
            let (va, vb) = match axis {
                0 => (ca.x, cb.x),
                1 => (ca.y, cb.y),
                _ => (ca.z, cb.z),
            };
            va.partial_cmp(&vb).unwrap()
        });

        // Split at median
        let mid = sorted.len() / 2;
        let left = Self::build_node(triangles, &sorted[..mid]);
        let right = Self::build_node(triangles, &sorted[mid..]);

        let left_aabb = Self::node_aabb(&left);
        let right_aabb = Self::node_aabb(&right);
        let combined = left_aabb.surrounding_box(&right_aabb);

        BVHNode::Internal {
            aabb: combined,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    fn centroid(tri: &Triangle) -> Vec3 {
        tri.v0.add(tri.v1).add(tri.v2).scale(1.0 / 3.0)
    }

    fn compute_aabb(triangles: &[Triangle], indices: &[usize]) -> AABB {
        let mut min = Vec3::new(INFINITY, INFINITY, INFINITY);
        let mut max = Vec3::new(-INFINITY, -INFINITY, -INFINITY);
        for &idx in indices {
            for v in &[triangles[idx].v0, triangles[idx].v1, triangles[idx].v2] {
                min = Vec3::new(min.x.min(v.x), min.y.min(v.y), min.z.min(v.z));
                max = Vec3::new(max.x.max(v.x), max.y.max(v.y), max.z.max(v.z));
            }
        }
        AABB::new(min, max)
    }

    fn node_aabb(node: &BVHNode) -> AABB {
        match node {
            BVHNode::Leaf { aabb, .. } => *aabb,
            BVHNode::Internal { aabb, .. } => *aabb,
        }
    }

    // Find closest intersection using BVH traversal
    fn intersect(&self, ray: &Ray, triangles: &[Triangle]) -> Option<(f64, usize)> {
        self.traverse(&self.root, ray, triangles, 0.001, INFINITY)
    }

    fn traverse(&self, node: &BVHNode, ray: &Ray, triangles: &[Triangle],
                t_min: f64, t_max: f64) -> Option<(f64, usize)>
    {
        match node {
            BVHNode::Leaf { aabb, primitives } => {
                if !aabb.hit(ray, t_min, t_max) { return None; }
                let mut closest = t_max;
                let mut hit_idx = None;
                for &idx in primitives {
                    if let Some(t) = triangles[idx].intersect(ray, t_min, closest) {
                        closest = t;
                        hit_idx = Some(idx);
                    }
                }
                hit_idx.map(|idx| (closest, idx))
            }
            BVHNode::Internal { aabb, left, right } => {
                if !aabb.hit(ray, t_min, t_max) { return None; }
                let hit_left = self.traverse(left, ray, triangles, t_min, t_max);
                let new_max = hit_left.map(|(t, _)| t).unwrap_or(t_max);
                let hit_right = self.traverse(right, ray, triangles, t_min, new_max);
                // Return the closer hit
                match (hit_left, hit_right) {
                    (Some(l), Some(r)) => if l.0 <= r.0 { Some(l) } else { Some(r) },
                    (Some(l), None) => Some(l),
                    (None, Some(r)) => Some(r),
                    (None, None) => None,
                }
            }
        }
    }
}
```

### Step 4: Simple Path Tracer

```rust
fn ray_color(ray: &Ray, bvh: &BVH, triangles: &[Triangle], depth: i32) -> Color {
    if depth <= 0 {
        return Vec3::new(0.0, 0.0, 0.0); // Black for max bounces
    }

    if let Some((t, idx)) = bvh.intersect(ray, triangles) {
        let hit_point = ray.point_at(t);
        let tri = &triangles[idx];

        // Simple diffuse shading with a fixed light direction
        let light_dir = Vec3::new(0.5, 1.0, 0.3).normalize();
        let edge1 = tri.v1.sub(tri.v0);
        let edge2 = tri.v2.sub(tri.v0);
        let normal = edge1.cross(edge2).normalize();
        let ndotl = normal.dot(light_dir).max(0.0);

        // Ambient + diffuse
        let ambient = tri.color.scale(0.15);
        let diffuse = tri.color.scale(ndotl * 0.85);
        ambient.add(diffuse)
    } else {
        // Sky gradient
        let unit_dir = ray.direction.normalize();
        let t = 0.5 * (unit_dir.y + 1.0);
        Vec3::new(1.0, 1.0, 1.0).scale(1.0 - t).add(Vec3::new(0.5, 0.7, 1.0).scale(t))
    }
}

fn main() {
    // Image
    let width = 400;
    let height = 300;
    let samples_per_pixel = 16;

    // Camera
    let origin = Vec3::new(0.0, 0.0, 3.0);
    let lower_left = Vec3::new(-2.0, -1.5, 1.0);
    let horizontal = Vec3::new(4.0, 0.0, 0.0);
    let vertical = Vec3::new(0.0, 3.0, 0.0);

    // Scene: a few colored triangles
    let triangles = vec![
        Triangle { v0: Vec3::new(-1.0, -1.0, 0.0), v1: Vec3::new(1.0, -1.0, 0.0),
                   v2: Vec3::new(0.0, 1.0, 0.0), color: Vec3::new(0.9, 0.2, 0.2) },
        Triangle { v0: Vec3::new(-2.0, -1.0, -1.0), v1: Vec3::new(2.0, -1.0, -1.0),
                   v2: Vec3::new(0.0, -1.0, 2.0), color: Vec3::new(0.2, 0.8, 0.2) },
        Triangle { v0: Vec3::new(0.5, -0.5, 0.5), v1: Vec3::new(1.5, -0.5, 0.5),
                   v2: Vec3::new(1.0, 0.5, 0.5), color: Vec3::new(0.2, 0.2, 0.9) },
    ];

    let bvh = BVH::build(&triangles);
    println!("BVH built for {} triangles", triangles.len());

    // Render
    println!("P3\n{} {}\n255", width, height);
    for y in (0..height).rev() {
        for x in 0..width {
            let mut color = Vec3::new(0.0, 0.0, 0.0);
            for _ in 0..samples_per_pixel {
                let u = (x as f64) / (width as f64);
                let v = (y as f64) / (height as f64);
                let direction = lower_left.add(horizontal.scale(u)).add(vertical.scale(v)).sub(origin);
                let ray = Ray { origin, direction: direction.normalize() };
                color = color.add(ray_color(&ray, &bvh, &triangles, 5));
            }
            color = color.scale(1.0 / samples_per_pixel as f64);

            let r = (color.x.sqrt() * 255.0) as i32;
            let g = (color.y.sqrt() * 255.0) as i32;
            let b = (color.z.sqrt() * 255.0) as i32;
            println!("{} {} {}", r.min(255), g.min(255), b.min(255));
        }
    }
    eprintln!("Done.");
}
```

## Use It

This structure scales into production renderers and real-time ray tracing systems:

- **PBRT (Physically Based Rendering Toolkit)**: the academic reference implementation of a path tracer. Its BVH implementation (`Accelerator` class) uses the Surface Area Heuristic (SAH) for optimal split placement. Our median-split BVH is simpler but follows the same traversal logic.
- **Intel Embree**: a production ray tracing kernel used in film rendering. Embree's BVH uses SIMD-optimized traversal, multi-branch trees (8-wide BVH for AVX), and specialized intersection kernels for triangles, curves, and volumes.
- **NVIDIA RTX hardware**: real-time ray tracing GPUs implement BVH traversal in fixed-function hardware. The BVH is built by the driver and stored in a format optimized for the hardware traversal unit. Our software BVH is the conceptual equivalent.
- **Ray Tracing in One Weekend**: Peter Shirley's book series builds a path tracer incrementally, starting with spheres and adding BVH in the second book. Our approach mirrors this progression.

The key production lesson: **BVH quality determines render performance more than anything else**. A bad BVH (poor split placement) can make a scene 10x slower to render. Production systems use the Surface Area Heuristic (SAH): split at the position that minimizes the expected number of ray-AABB tests, weighted by the surface area of each child's bounding box.

## Read the Source

- [PBRT](https://pbr-book.org/) — Pharr, Jakob, Humphreys. The definitive reference for physically based rendering. Chapter 4 (Primitives and Intersection Acceleration) covers BVH construction and traversal in detail.
- [Ray Tracing in One Weekend](https://raytracing.github.io/) — Shirley. Book 2 covers BVH implementation. The progression from brute-force to BVH-accelerated ray tracing mirrors this lesson.
- [Embree documentation](https://www.embree.org/) — Intel's production ray tracing kernel. The API docs show how production BVH structures are built and traversed.

## Ship It

- `code/main.rs`: complete path tracer with BVH acceleration, triangle intersection, and PPM output.
- `outputs/README.md`: path tracer capstone checklist covering BVH construction, intersection, shading, and output validation.

## Exercises

1. **Easy** — Add sphere primitive support. Implement ray-sphere intersection (solving the quadratic equation) and add spheres to the BVH. The BVH AABB for a sphere is straightforward: center ± radius in each axis.
2. **Medium** — Add Russian roulette termination. Instead of a fixed bounce limit, randomly terminate rays at each bounce with probability proportional to the throughput. This gives unbiased results with fewer bounces on average, improving performance for scenes with many diffuse bounces.
3. **Hard** — Add multi-threaded tile rendering. Divide the image into tiles (e.g., 16x16 pixels) and render each tile on a separate thread using a thread pool. Each thread needs its own random number generator (or a lock-free RNG). Measure speedup on a multi-core machine.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| BVH | "acceleration tree" | A binary tree of axis-aligned bounding boxes (AABBs) used to accelerate ray-scene intersection. Each node's AABB contains all geometry in its subtree. Rays skip entire subtrees by testing against the AABB first. |
| Ray hit | "intersection" | The point where a ray meets a geometric surface. The closest hit is the one visible to the camera. For triangles, the Moller-Trumbore algorithm computes the intersection efficiently using cross products. |
| Throughput | "path contribution" | The multiplicative color weight of a ray as it bounces through the scene. Each bounce multiplies the throughput by the surface's reflectance. When throughput is low, the ray contributes little to the final color. |
| Sample count | "anti-noise budget" | The number of random rays cast per pixel. More samples reduce variance (noise) but increase render time. The noise decreases as 1/sqrt(N) where N is the sample count. |
| AABB | "bounding box" | Axis-Aligned Bounding Box: the smallest box with faces parallel to the coordinate axes that contains a set of points. Used in BVH nodes and broad-phase collision detection. |

## Further Reading

- [PBRT](https://pbr-book.org/) — The definitive reference for physically based rendering.
- [Ray Tracing in One Weekend](https://raytracing.github.io/) — Build a path tracer from scratch in a weekend.
- [Scratchapixel](https://www.scratchapixel.com/) — Detailed tutorials on ray tracing, BVH, and rendering math.
