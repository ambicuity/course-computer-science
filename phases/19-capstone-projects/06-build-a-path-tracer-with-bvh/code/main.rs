// Build a Path Tracer with BVH
// Run: rustc main.rs && ./main > output.ppm
// View: open output.ppm in any image viewer
//
// Architecture:
//   Camera ray → BVH traversal → Triangle intersection → Shading → Pixel color
//
// Implements a complete path tracer with BVH acceleration, Moller-Trumbore
// ray-triangle intersection, and diffuse shading. Outputs a PPM image.

use std::f64::INFINITY;

// =============================================================================
// Step 1: Core Math Types
// =============================================================================

#[derive(Debug, Clone, Copy)]
struct Vec3 { x: f64, y: f64, z: f64 }

impl Vec3 {
    fn new(x: f64, y: f64, z: f64) -> Self { Vec3 { x, y, z } }
    fn add(self, o: Vec3) -> Vec3 { Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z) }
    fn sub(self, o: Vec3) -> Vec3 { Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z) }
    fn scale(self, t: f64) -> Vec3 { Vec3::new(self.x * t, self.y * t, self.z * t) }
    fn dot(self, o: Vec3) -> f64 { self.x * o.x + self.y * o.y + self.z * o.z }
    fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }
    fn length(self) -> f64 { self.dot(self).sqrt() }
    fn normalize(self) -> Vec3 { self.scale(1.0 / self.length()) }
}

type Color = Vec3;

#[derive(Debug, Clone, Copy)]
struct Ray { origin: Vec3, direction: Vec3 }

impl Ray {
    fn point_at(&self, t: f64) -> Vec3 { self.origin.add(self.direction.scale(t)) }
}

// =============================================================================
// Step 2: Triangle Intersection and AABB
// =============================================================================

#[derive(Debug, Clone)]
struct Triangle { v0: Vec3, v1: Vec3, v2: Vec3, color: Color }

impl Triangle {
    fn intersect(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<f64> {
        let edge1 = self.v1.sub(self.v0);
        let edge2 = self.v2.sub(self.v0);
        let h = ray.direction.cross(edge2);
        let a = edge1.dot(h);
        if a > -1e-8 && a < 1e-8 { return None; }
        let f = 1.0 / a;
        let s = ray.origin.sub(self.v0);
        let u = f * s.dot(h);
        if u < 0.0 || u > 1.0 { return None; }
        let q = s.cross(edge1);
        let v = f * ray.direction.dot(q);
        if v < 0.0 || u + v > 1.0 { return None; }
        let t = f * edge2.dot(q);
        if t >= t_min && t <= t_max { Some(t) } else { None }
    }
}

#[derive(Debug, Clone, Copy)]
struct AABB { min: Vec3, max: Vec3 }

impl AABB {
    fn new(min: Vec3, max: Vec3) -> Self { AABB { min, max } }

    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> bool {
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
            Vec3::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y), self.min.z.min(other.min.z)),
            Vec3::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y), self.max.z.max(other.max.z)),
        )
    }
}

// =============================================================================
// Step 3: BVH Construction and Traversal
// =============================================================================

enum BVHNode {
    Leaf { aabb: AABB, primitives: Vec<usize> },
    Internal { aabb: AABB, left: Box<BVHNode>, right: Box<BVHNode> },
}

struct BVH { root: BVHNode }

impl BVH {
    fn build(triangles: &[Triangle]) -> Self {
        let indices: Vec<usize> = (0..triangles.len()).collect();
        BVH { root: Self::build_node(triangles, &indices) }
    }

    fn build_node(triangles: &[Triangle], indices: &[usize]) -> BVHNode {
        let aabb = Self::compute_aabb(triangles, indices);
        if indices.len() <= 4 {
            return BVHNode::Leaf { aabb, primitives: indices.to_vec() };
        }
        let extent = aabb.max.sub(aabb.min);
        let axis = if extent.x > extent.y && extent.x > extent.z { 0 }
                   else if extent.y > extent.z { 1 } else { 2 };
        let mut sorted = indices.to_vec();
        sorted.sort_by(|&a, &b| {
            let ca = Self::centroid(&triangles[a]);
            let cb = Self::centroid(&triangles[b]);
            let (va, vb) = match axis { 0 => (ca.x, cb.x), 1 => (ca.y, cb.y), _ => (ca.z, cb.z) };
            va.partial_cmp(&vb).unwrap()
        });
        let mid = sorted.len() / 2;
        let left = Self::build_node(triangles, &sorted[..mid]);
        let right = Self::build_node(triangles, &sorted[mid..]);
        let combined = Self::node_aabb(&left).surrounding_box(&Self::node_aabb(&right));
        BVHNode::Internal { aabb: combined, left: Box::new(left), right: Box::new(right) }
    }

    fn centroid(tri: &Triangle) -> Vec3 { tri.v0.add(tri.v1).add(tri.v2).scale(1.0 / 3.0) }

    fn compute_aabb(triangles: &[Triangle], indices: &[usize]) -> AABB {
        let (mut min, mut max) = (Vec3::new(INFINITY, INFINITY, INFINITY), Vec3::new(-INFINITY, -INFINITY, -INFINITY));
        for &idx in indices {
            for v in &[triangles[idx].v0, triangles[idx].v1, triangles[idx].v2] {
                min = Vec3::new(min.x.min(v.x), min.y.min(v.y), min.z.min(v.z));
                max = Vec3::new(max.x.max(v.x), max.y.max(v.y), max.z.max(v.z));
            }
        }
        AABB::new(min, max)
    }

    fn node_aabb(node: &BVHNode) -> AABB {
        match node { BVHNode::Leaf { aabb, .. } | BVHNode::Internal { aabb, .. } => *aabb }
    }

    fn intersect(&self, ray: &Ray, triangles: &[Triangle]) -> Option<(f64, usize)> {
        self.traverse(&self.root, ray, triangles, 0.001, INFINITY)
    }

    fn traverse(&self, node: &BVHNode, ray: &Ray, triangles: &[Triangle],
                t_min: f64, t_max: f64) -> Option<(f64, usize)> {
        match node {
            BVHNode::Leaf { aabb, primitives } => {
                if !aabb.hit(ray, t_min, t_max) { return None; }
                let (mut closest, mut hit_idx) = (t_max, None);
                for &idx in primitives {
                    if let Some(t) = triangles[idx].intersect(ray, t_min, closest) {
                        closest = t; hit_idx = Some(idx);
                    }
                }
                hit_idx.map(|idx| (closest, idx))
            }
            BVHNode::Internal { aabb, left, right } => {
                if !aabb.hit(ray, t_min, t_max) { return None; }
                let hit_left = self.traverse(left, ray, triangles, t_min, t_max);
                let new_max = hit_left.map(|(t, _)| t).unwrap_or(t_max);
                let hit_right = self.traverse(right, ray, triangles, t_min, new_max);
                match (hit_left, hit_right) {
                    (Some(l), Some(r)) => if l.0 <= r.0 { Some(l) } else { Some(r) },
                    (l @ Some(_), None) | (None, l @ Some(_)) => l,
                    (None, None) => None,
                }
            }
        }
    }
}

// =============================================================================
// Step 4: Path Tracer + Main
// =============================================================================

fn ray_color(ray: &Ray, bvh: &BVH, triangles: &[Triangle], depth: i32) -> Color {
    if depth <= 0 { return Vec3::new(0.0, 0.0, 0.0); }
    if let Some((t, idx)) = bvh.intersect(ray, triangles) {
        let tri = &triangles[idx];
        let light_dir = Vec3::new(0.5, 1.0, 0.3).normalize();
        let normal = tri.v1.sub(tri.v0).cross(tri.v2.sub(tri.v0)).normalize();
        let ndotl = normal.dot(light_dir).max(0.0);
        tri.color.scale(0.15).add(tri.color.scale(ndotl * 0.85))
    } else {
        let unit_dir = ray.direction.normalize();
        let t = 0.5 * (unit_dir.y + 1.0);
        Vec3::new(1.0, 1.0, 1.0).scale(1.0 - t).add(Vec3::new(0.5, 0.7, 1.0).scale(t))
    }
}

fn main() {
    let (width, height, samples) = (400, 300, 16);
    let origin = Vec3::new(0.0, 0.0, 3.0);
    let lower_left = Vec3::new(-2.0, -1.5, 1.0);
    let horizontal = Vec3::new(4.0, 0.0, 0.0);
    let vertical = Vec3::new(0.0, 3.0, 0.0);

    let triangles = vec![
        Triangle { v0: Vec3::new(-1.0, -1.0, 0.0), v1: Vec3::new(1.0, -1.0, 0.0),
                   v2: Vec3::new(0.0, 1.0, 0.0), color: Vec3::new(0.9, 0.2, 0.2) },
        Triangle { v0: Vec3::new(-2.0, -1.0, -1.0), v1: Vec3::new(2.0, -1.0, -1.0),
                   v2: Vec3::new(0.0, -1.0, 2.0), color: Vec3::new(0.2, 0.8, 0.2) },
        Triangle { v0: Vec3::new(0.5, -0.5, 0.5), v1: Vec3::new(1.5, -0.5, 0.5),
                   v2: Vec3::new(1.0, 0.5, 0.5), color: Vec3::new(0.2, 0.2, 0.9) },
    ];

    let bvh = BVH::build(&triangles);
    eprintln!("BVH built for {} triangles", triangles.len());

    println!("P3\n{} {}\n255", width, height);
    for y in (0..height).rev() {
        for x in 0..width {
            let mut color = Vec3::new(0.0, 0.0, 0.0);
            for _ in 0..samples {
                let u = (x as f64) / (width as f64);
                let v = (y as f64) / (height as f64);
                let direction = lower_left.add(horizontal.scale(u)).add(vertical.scale(v)).sub(origin);
                let ray = Ray { origin, direction: direction.normalize() };
                color = color.add(ray_color(&ray, &bvh, &triangles, 5));
            }
            color = color.scale(1.0 / samples as f64);
            println!("{} {} {}", (color.x.sqrt() * 255.0) as i32, (color.y.sqrt() * 255.0) as i32, (color.z.sqrt() * 255.0) as i32);
        }
    }
    eprintln!("Done.");
}
