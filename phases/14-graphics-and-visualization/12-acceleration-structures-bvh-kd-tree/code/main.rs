use std::cmp::Ordering;
use std::f64;
use std::io::Write;
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vec3 {
    fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    fn zero() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0 }
    }
    fn dot(&self, other: &Vec3) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
    fn sub(&self, other: &Vec3) -> Vec3 {
        Vec3::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
    fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }
}

struct Ray {
    origin: Vec3,
    direction: Vec3,
}

impl Ray {
    fn new(origin: Vec3, direction: Vec3) -> Self {
        Self { origin, direction }
    }
    fn at(&self, t: f64) -> Vec3 {
        Vec3::new(
            self.origin.x + t * self.direction.x,
            self.origin.y + t * self.direction.y,
            self.origin.z + t * self.direction.z,
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct Aabb {
    min: Vec3,
    max: Vec3,
}

impl Aabb {
    fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }
    fn empty() -> Self {
        Self {
            min: Vec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            max: Vec3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }
    fn union(&self, other: &Aabb) -> Aabb {
        Aabb::new(
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
    fn surround_point(&self, p: &Vec3) -> Aabb {
        Aabb::new(
            Vec3::new(
                self.min.x.min(p.x),
                self.min.y.min(p.y),
                self.min.z.min(p.z),
            ),
            Vec3::new(
                self.max.x.max(p.x),
                self.max.y.max(p.y),
                self.max.z.max(p.z),
            ),
        )
    }
    fn centroid(&self) -> Vec3 {
        Vec3::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
            (self.min.z + self.max.z) * 0.5,
        )
    }
    fn surface_area(&self) -> f64 {
        let dx = self.max.x - self.min.x;
        let dy = self.max.y - self.min.y;
        let dz = self.max.z - self.min.z;
        2.0 * (dx * dy + dy * dz + dz * dx)
    }
    fn intersect(&self, ray: &Ray, t_min: f64, t_max: f64) -> bool {
        let mut t_enter = t_min;
        let mut t_exit = t_max;
        let axes = [(ray.origin.x, ray.direction.x, self.min.x, self.max.x),
                     (ray.origin.y, ray.direction.y, self.min.y, self.max.y),
                     (ray.origin.z, ray.direction.z, self.min.z, self.max.z)];
        for (o, d, bmin, bmax) in axes {
            let (t0, t1) = if d.abs() < 1e-12 {
                if o < bmin || o > bmax {
                    return false;
                }
                (f64::NEG_INFINITY, f64::INFINITY)
            } else {
                let inv_d = 1.0 / d;
                let mut t0 = (bmin - o) * inv_d;
                let mut t1 = (bmax - o) * inv_d;
                if t0 > t1 {
                    std::mem::swap(&mut t0, &mut t1);
                }
                (t0, t1)
            };
            t_enter = t_enter.max(t0);
            t_exit = t_exit.min(t1);
            if t_enter > t_exit {
                return false;
            }
        }
        t_exit > 0.0
    }
}

#[derive(Clone)]
struct Sphere {
    center: Vec3,
    radius: f64,
}

impl Sphere {
    fn new(center: Vec3, radius: f64) -> Self {
        Self { center, radius }
    }
    fn bounding_box(&self) -> Aabb {
        Aabb::new(
            Vec3::new(
                self.center.x - self.radius,
                self.center.y - self.radius,
                self.center.z - self.radius,
            ),
            Vec3::new(
                self.center.x + self.radius,
                self.center.y + self.radius,
                self.center.z + self.radius,
            ),
        )
    }
    fn intersect(&self, ray: &Ray) -> Option<f64> {
        let oc = ray.origin.sub(&self.center);
        let a = ray.direction.length_squared();
        let half_b = oc.dot(&ray.direction);
        let c = oc.length_squared() - self.radius * self.radius;
        let discriminant = half_b * half_b - a * c;
        if discriminant < 0.0 {
            return None;
        }
        let sqrt_d = discriminant.sqrt();
        let mut t = (-half_b - sqrt_d) / a;
        if t < 0.001 {
            t = (-half_b + sqrt_d) / a;
            if t < 0.001 {
                return None;
            }
        }
        Some(t)
    }
}

#[derive(Clone)]
struct HitRecord {
    t: f64,
    point: Vec3,
    normal: Vec3,
}

enum BvhNode {
    Leaf {
        bounds: Aabb,
        first: usize,
        count: usize,
    },
    Inner {
        bounds: Aabb,
        left: usize,
        right: usize,
    },
}

struct Bvh {
    nodes: Vec<BvhNode>,
}

impl Bvh {
    fn build(spheres: &mut [Sphere]) -> Self {
        let mut nodes = Vec::new();
        if spheres.is_empty() {
            return Self { nodes };
        }
        Self::build_recursive(spheres, &mut nodes, 0);
        Self { nodes }
    }

    fn build_recursive(spheres: &mut [Sphere], nodes: &mut Vec<BvhNode>, depth: usize) -> usize {
        let n = spheres.len();
        let bounds = spheres.iter().fold(Aabb::empty(), |b, s| {
            b.union(&s.bounding_box())
        });
        let leaf_size = 4;
        if n <= leaf_size {
            let idx = nodes.len();
            let first = spheres.as_ptr() as usize;
            nodes.push(BvhNode::Leaf { bounds, first, count: n });
            return idx;
        }

        let axis = depth % 3;
        spheres.sort_by(|a, b| {
            let ca = a.bounding_box().centroid();
            let cb = b.bounding_box().centroid();
            let va = match axis { 0 => ca.x, 1 => ca.y, _ => ca.z };
            let vb = match axis { 0 => cb.x, 1 => cb.y, _ => cb.z };
            va.partial_cmp(&vb).unwrap_or(Ordering::Equal)
        });

        let mid = Self::sah_split(spheres, &bounds, axis);
        let (left_spheres, right_spheres) = spheres.split_at_mut(mid);

        let idx = nodes.len();
        nodes.push(BvhNode::Inner { bounds, left: 0, right: 0 });

        let left_idx = Self::build_recursive(left_spheres, nodes, depth + 1);
        let right_idx = Self::build_recursive(right_spheres, nodes, depth + 1);

        if let BvhNode::Inner { left, right, .. } = &mut nodes[idx] {
            *left = left_idx;
            *right = right_idx;
        }

        idx
    }

    fn sah_split(spheres: &[Sphere], parent_bounds: &Aabb, axis: usize) -> usize {
        let n = spheres.len();
        if n <= 2 {
            return n / 2;
        }
        let num_buckets = 12;
        let parent_sa = parent_bounds.surface_area();
        if parent_sa <= 0.0 {
            return n / 2;
        }
        let mut buckets_count = vec![0u32; num_buckets];
        let mut buckets_bounds = vec![Aabb::empty(); num_buckets];

        let (pmin, pmax) = match axis {
            0 => (parent_bounds.min.x, parent_bounds.max.x),
            1 => (parent_bounds.min.y, parent_bounds.max.y),
            _ => (parent_bounds.min.z, parent_bounds.max.z),
        };
        let extent = pmax - pmin;
        if extent <= 0.0 {
            return n / 2;
        }

        for s in spheres {
            let c = s.bounding_box().centroid();
            let v = match axis { 0 => c.x, 1 => c.y, _ => c.z };
            let mut b = (((v - pmin) / extent) * num_buckets as f64) as usize;
            if b >= num_buckets { b = num_buckets - 1; }
            buckets_count[b] += 1;
            buckets_bounds[b] = buckets_bounds[b].union(&s.bounding_box());
        }

        let mut left_count = 0u32;
        let mut left_bounds = Aabb::empty();
        let mut best_cost = f64::INFINITY;
        let mut best_split = n / 2;
        let c_trav = 1.0;
        let c_isect = 1.0;

        for i in 0..(num_buckets - 1) {
            left_count += buckets_count[i];
            left_bounds = left_bounds.union(&buckets_bounds[i]);
            let mut right_count = 0u32;
            let mut right_bounds = Aabb::empty();
            for j in (i + 1)..num_buckets {
                right_count += buckets_count[j];
                right_bounds = right_bounds.union(&buckets_bounds[j]);
            }
            if left_count == 0 || right_count == 0 {
                continue;
            }
            let left_sa = left_bounds.surface_area();
            let right_sa = right_bounds.surface_area();
            let cost = c_trav
                + (left_sa / parent_sa) * left_count as f64 * c_isect
                + (right_sa / parent_sa) * right_count as f64 * c_isect;
            if cost < best_cost {
                best_cost = cost;
                best_split = left_count as usize;
            }
        }

        if best_cost >= n as f64 * c_isect {
            return n;
        }
        if best_split == 0 { best_split = 1; }
        if best_split >= n { best_split = n - 1; }
        best_split
    }

    fn intersect(&self, ray: &Ray, spheres: &[Sphere]) -> Option<HitRecord> {
        if self.nodes.is_empty() {
            return None;
        }
        let mut closest_t = f64::INFINITY;
        let mut closest_hit: Option<HitRecord> = None;
        let mut stack = vec![0usize];
        while let Some(node_idx) = stack.pop() {
            let node = &self.nodes[node_idx];
            match node {
                BvhNode::Leaf { bounds, first, count } => {
                    if !bounds.intersect(ray, 0.001, closest_t) {
                        continue;
                    }
                    let base = *first;
                    let len = *count;
                    let ptr = base as *const Sphere;
                    for i in 0..len {
                        unsafe {
                            let s = &*ptr.add(i);
                            if let Some(t) = s.intersect(ray) {
                                if t < closest_t {
                                    closest_t = t;
                                    let point = ray.at(t);
                                    let normal = point.sub(&s.center);
                                    let normal_len = normal.length_squared().sqrt();
                                    closest_hit = Some(HitRecord {
                                        t,
                                        point,
                                        normal: Vec3::new(
                                            normal.x / normal_len,
                                            normal.y / normal_len,
                                            normal.z / normal_len,
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }
                BvhNode::Inner { bounds, left, right } => {
                    if !bounds.intersect(ray, 0.001, closest_t) {
                        continue;
                    }
                    stack.push(*left);
                    stack.push(*right);
                }
            }
        }
        closest_hit
    }
}

fn brute_force_intersect(ray: &Ray, spheres: &[Sphere]) -> Option<HitRecord> {
    let mut closest_t = f64::INFINITY;
    let mut closest_hit: Option<HitRecord> = None;
    for s in spheres {
        if let Some(t) = s.intersect(ray) {
            if t < closest_t {
                closest_t = t;
                let point = ray.at(t);
                let normal = point.sub(&s.center);
                let normal_len = normal.length_squared().sqrt();
                closest_hit = Some(HitRecord {
                    t,
                    point,
                    normal: Vec3::new(
                        normal.x / normal_len,
                        normal.y / normal_len,
                        normal.z / normal_len,
                    ),
                });
            }
        }
    }
    closest_hit
}

fn simple_shade(normal: &Vec3) -> f64 {
    let light = Vec3::new(0.4, 0.8, 0.6);
    let len = (light.x * light.x + light.y * light.y + light.z * light.z).sqrt();
    let dot = (normal.x * light.x + normal.y * light.y + normal.z * light.z) / len;
    dot.max(0.0) * 0.7 + 0.2
}

fn render_scene(
    width: usize,
    height: usize,
    spheres: &[Sphere],
    use_bvh: bool,
) -> Vec<u8> {
    let mut pixels = vec![0u8; width * height * 3];
    if use_bvh {
        let mut spheres_mut = spheres.to_vec();
        let bvh = Bvh::build(&mut spheres_mut);
        for y in 0..height {
            for x in 0..width {
                let u = (x as f64) / (width as f64 - 1.0);
                let v = ((height - 1 - y) as f64) / (height as f64 - 1.0);
                let dir = Vec3::new(-1.0 + 2.0 * u, -1.0 + 2.0 * v, -1.0);
                let dir_len = dir.length_squared().sqrt();
                let ray = Ray::new(
                    Vec3::new(0.0, 0.0, 4.0),
                    Vec3::new(dir.x / dir_len, dir.y / dir_len, dir.z / dir_len),
                );
                let idx = (y * width + x) * 3;
                if let Some(hit) = bvh.intersect(&ray, spheres) {
                    let shade = simple_shade(&hit.normal);
                    pixels[idx] = (shade * 180.0) as u8;
                    pixels[idx + 1] = (shade * 200.0) as u8;
                    pixels[idx + 2] = (shade * 255.0) as u8;
                }
            }
        }
    } else {
        for y in 0..height {
            for x in 0..width {
                let u = (x as f64) / (width as f64 - 1.0);
                let v = ((height - 1 - y) as f64) / (height as f64 - 1.0);
                let dir = Vec3::new(-1.0 + 2.0 * u, -1.0 + 2.0 * v, -1.0);
                let dir_len = dir.length_squared().sqrt();
                let ray = Ray::new(
                    Vec3::new(0.0, 0.0, 4.0),
                    Vec3::new(dir.x / dir_len, dir.y / dir_len, dir.z / dir_len),
                );
                let idx = (y * width + x) * 3;
                if let Some(hit) = brute_force_intersect(&ray, spheres) {
                    let shade = simple_shade(&hit.normal);
                    pixels[idx] = (shade * 180.0) as u8;
                    pixels[idx + 1] = (shade * 200.0) as u8;
                    pixels[idx + 2] = (shade * 255.0) as u8;
                }
            }
        }
    }
    pixels
}

fn write_ppm(filename: &str, width: usize, height: usize, pixels: &[u8]) -> std::io::Result<()> {
    let mut file = std::fs::File::create(filename)?;
    write!(file, "P3\n{} {}\n255\n", width, height)?;
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 3;
            write!(file, "{} {} {} ", pixels[idx], pixels[idx + 1], pixels[idx + 2])?;
        }
        writeln!(file)?;
    }
    Ok(())
}

fn generate_scene(num_spheres: usize) -> Vec<Sphere> {
    let mut spheres = Vec::with_capacity(num_spheres + 1);
    spheres.push(Sphere::new(Vec3::new(0.0, -1002.0, 0.0), 1000.0));
    let mut seed: u64 = 12345;
    let pseudo_rand = |seed: &mut u64| -> f64 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let x = *seed;
        let x = ((x >> 16) ^ x) as u32;
        (x as f64) / (u32::MAX as f64)
    };
    for _ in 0..num_spheres {
        let x = (pseudo_rand(&mut seed) - 0.5) * 6.0;
        let y = (pseudo_rand(&mut seed) - 0.5) * 6.0;
        let z = (pseudo_rand(&mut seed) - 0.5) * 6.0 - 3.0;
        let r = 0.1 + pseudo_rand(&mut seed) * 0.3;
        spheres.push(Sphere::new(Vec3::new(x, y, z), r));
    }
    spheres
}

fn main() {
    let width = 320;
    let height = 240;
    let num_spheres = 200;

    println!("=== Acceleration Structures: BVH Benchmark ===\n");
    println!("Scene: {} spheres + ground plane", num_spheres);
    println!("Image: {}x{} pixels\n", width, height);

    let spheres = generate_scene(num_spheres);

    let brute_start = Instant::now();
    let brute_pixels = render_scene(width, height, &spheres, false);
    let brute_elapsed = brute_start.elapsed();

    println!("Brute-force ray tracing: {:?}", brute_elapsed);

    let bvh_start = Instant::now();
    let bvh_pixels = render_scene(width, height, &spheres, true);
    let bvh_elapsed = bvh_start.elapsed();

    println!("BVH-accelerated ray tracing: {:?}", bvh_elapsed);

    let speedup = brute_elapsed.as_secs_f64() / bvh_elapsed.as_secs_f64();
    println!("\nSpeedup: {:.2}x", speedup);

    match write_ppm("output_brute.ppm", width, height, &brute_pixels) {
        Ok(_) => println!("\nWrote output_brute.ppm"),
        Err(e) => eprintln!("Error writing brute force PPM: {}", e),
    }
    match write_ppm("output_bvh.ppm", width, height, &bvh_pixels) {
        Ok(_) => println!("Wrote output_bvh.ppm"),
        Err(e) => eprintln!("Error writing BVH PPM: {}", e),
    }

    println!("\n--- AABB Slab Method Validation ---");
    let test_box = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
    let ray_hit = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
    let ray_miss = Ray::new(Vec3::new(5.0, 5.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
    println!("Ray toward box origin: hit = {} (expected true)", test_box.intersect(&ray_hit, 0.0, f64::INFINITY));
    println!("Ray away from box:     hit = {} (expected false)", test_box.intersect(&ray_miss, 0.0, f64::INFINITY));

    println!("\n--- SAH Cost Explanation ---");
    println!("SAH cost = C_trav + (SA_left/SA_parent) * N_left * C_isect + (SA_right/SA_parent) * N_right * C_isect");
    println!("A good split minimizes this cost by making child AABBs small relative to the parent.");
}