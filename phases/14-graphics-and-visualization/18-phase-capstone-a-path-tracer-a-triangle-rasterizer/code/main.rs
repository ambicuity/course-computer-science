#![allow(dead_code)]
#![allow(unused_variables)]

use std::fs::File;
use std::io::Write;

// ============================================================
// Shared Math Foundation
// ============================================================

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
    fn one() -> Self {
        Self { x: 1.0, y: 1.0, z: 1.0 }
    }
    fn dot(self, o: Self) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    fn cross(self, o: Self) -> Self {
        Self::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    fn length(self) -> f64 {
        self.dot(self).sqrt()
    }
    fn length_sq(self) -> f64 {
        self.dot(self)
    }
    fn normalized(self) -> Self {
        let l = self.length();
        if l == 0.0 { return Self::zero(); }
        Self::new(self.x / l, self.y / l, self.z / l)
    }
    fn elem_mul(self, o: Self) -> Self {
        Self::new(self.x * o.x, self.y * o.y, self.z * o.z)
    }
    fn reflect(self, n: Self) -> Self {
        self - n * 2.0 * self.dot(n)
    }
    fn near_zero(self) -> bool {
        let e = 1e-8;
        self.x.abs() < e && self.y.abs() < e && self.z.abs() < e
    }
    fn clamp01(self) -> Self {
        Self::new(self.x.max(0.0).min(1.0), self.y.max(0.0).min(1.0), self.z.max(0.0).min(1.0))
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self { Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z) }
}
impl std::ops::Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self { Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z) }
}
impl std::ops::Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, s: f64) -> Self { Self::new(self.x * s, self.y * s, self.z * s) }
}
impl std::ops::Mul<Vec3> for f64 {
    type Output = Vec3;
    fn mul(self, v: Vec3) -> Vec3 { Vec3::new(self * v.x, self * v.y, self * v.z) }
}
impl std::ops::Div<f64> for Vec3 {
    type Output = Self;
    fn div(self, s: f64) -> Self { Self::new(self.x / s, self.y / s, self.z / s) }
}
impl std::ops::AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x; self.y += rhs.y; self.z += rhs.z;
    }
}

#[derive(Clone, Copy, Debug)]
struct Vec4 {
    x: f64, y: f64, z: f64, w: f64,
}

impl Vec4 {
    fn new(x: f64, y: f64, z: f64, w: f64) -> Self { Self { x, y, z, w } }
    fn from_point(v: Vec3) -> Self { Self { x: v.x, y: v.y, z: v.z, w: 1.0 } }
    fn from_dir(v: Vec3) -> Self { Self { x: v.x, y: v.y, z: v.z, w: 0.0 } }
    fn to_vec3(&self) -> Vec3 {
        if self.w.abs() < 1e-12 {
            Vec3::new(self.x, self.y, self.z)
        } else {
            Vec3::new(self.x / self.w, self.y / self.w, self.z / self.w)
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Mat4 {
    data: [[f64; 4]; 4],
}

impl Mat4 {
    fn identity() -> Self {
        let mut m = Self { data: [[0.0; 4]; 4] };
        m.data[0][0] = 1.0; m.data[1][1] = 1.0;
        m.data[2][2] = 1.0; m.data[3][3] = 1.0;
        m
    }

    fn mul(&self, o: &Self) -> Self {
        let mut r = Self { data: [[0.0; 4]; 4] };
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    r.data[i][j] += self.data[i][k] * o.data[k][j];
                }
            }
        }
        r
    }

    fn transform(&self, v: Vec4) -> Vec4 {
        Vec4::new(
            self.data[0][0]*v.x + self.data[0][1]*v.y + self.data[0][2]*v.z + self.data[0][3]*v.w,
            self.data[1][0]*v.x + self.data[1][1]*v.y + self.data[1][2]*v.z + self.data[1][3]*v.w,
            self.data[2][0]*v.x + self.data[2][1]*v.y + self.data[2][2]*v.z + self.data[2][3]*v.w,
            self.data[3][0]*v.x + self.data[3][1]*v.y + self.data[3][2]*v.z + self.data[3][3]*v.w,
        )
    }

    fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let fwd = (target - eye).normalized();
        let right = fwd.cross(up).normalized();
        let real_up = right.cross(fwd);
        let mut m = Self::identity();
        m.data[0][0] = right.x; m.data[0][1] = right.y; m.data[0][2] = right.z;
        m.data[0][3] = -(right.dot(eye));
        m.data[1][0] = real_up.x; m.data[1][1] = real_up.y; m.data[1][2] = real_up.z;
        m.data[1][3] = -(real_up.dot(eye));
        m.data[2][0] = -fwd.x; m.data[2][1] = -fwd.y; m.data[2][2] = -fwd.z;
        m.data[2][3] = fwd.dot(eye);
        m.data[3][3] = 1.0;
        m
    }

    fn perspective(fov_deg: f64, aspect: f64, near: f64, far: f64) -> Self {
        let fov_rad = fov_deg * std::f64::consts::PI / 180.0;
        let tan_half = (fov_rad / 2.0).tan();
        let mut m = Self { data: [[0.0; 4]; 4] };
        m.data[0][0] = 1.0 / (aspect * tan_half);
        m.data[1][1] = 1.0 / tan_half;
        m.data[2][2] = -(far + near) / (far - near);
        m.data[2][3] = -(2.0 * far * near) / (far - near);
        m.data[3][2] = -1.0;
        m
    }
}

impl std::ops::Mul for &Mat4 {
    type Output = Mat4;
    fn mul(self, rhs: Self) -> Mat4 { self.mul(rhs) }
}

#[derive(Clone, Copy, Debug)]
struct Ray {
    origin: Vec3,
    dir: Vec3,
}

impl Ray {
    fn new(origin: Vec3, dir: Vec3) -> Self { Self { origin, dir: dir.normalized() } }
    fn at(&self, t: f64) -> Vec3 { self.origin + self.dir * t }
}

// ============================================================
// Random Number Generation (simple LCG, no external deps)
// ============================================================

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self { Self { state: if seed == 0 { 1 } else { seed } } }
    fn gen_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }
    fn gen(&mut self) -> f64 {
        (self.gen_u32() as f64) / (u32::MAX as f64)
    }
    fn gen_range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.gen() * (hi - lo)
    }
}

fn cosine_hemisphere(rng: &mut SimpleRng) -> (Vec3, f64) {
    let r1 = rng.gen();
    let r2 = rng.gen();
    let phi = 2.0 * std::f64::consts::PI * r1;
    let cos_theta = r2.sqrt();
    let sin_theta = (1.0 - r2).sqrt();
    let x = sin_theta * phi.cos();
    let z = sin_theta * phi.sin();
    let pdf = cos_theta / std::f64::consts::PI;
    (Vec3::new(x, cos_theta, z), pdf)
}

fn local_to_world(normal: Vec3, local_dir: Vec3) -> Vec3 {
    let tangent = if normal.y.abs() > 0.999 {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0).cross(normal).normalized()
    };
    let bitangent = normal.cross(tangent).normalized();
    Vec3::new(
        tangent.x * local_dir.x + normal.x * local_dir.y + bitangent.x * local_dir.z,
        tangent.y * local_dir.x + normal.y * local_dir.y + bitangent.y * local_dir.z,
        tangent.z * local_dir.x + normal.z * local_dir.y + bitangent.z * local_dir.z,
    )
}

// ============================================================
// Scene Definition
// ============================================================

#[derive(Clone, Debug)]
struct Material {
    diffuse: Vec3,
    emissive: Vec3,
}

#[derive(Clone, Debug)]
struct Sphere {
    center: Vec3,
    radius: f64,
    material: usize,
}

#[derive(Clone, Debug)]
struct Triangle {
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    material: usize,
}

#[derive(Clone, Debug)]
struct Light {
    position: Vec3,
    color: Vec3,
    intensity: f64,
}

#[derive(Clone, Debug)]
struct Scene {
    spheres: Vec<Sphere>,
    triangles: Vec<Triangle>,
    lights: Vec<Light>,
    materials: Vec<Material>,
}

struct HitRecord {
    point: Vec3,
    normal: Vec3,
    t: f64,
    material: usize,
}

impl Scene {
    fn intersect(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        let mut closest: Option<HitRecord> = None;
        let mut closest_t = t_max;

        for sphere in &self.spheres {
            let oc = ray.origin - sphere.center;
            let a = ray.dir.dot(ray.dir);
            let half_b = oc.dot(ray.dir);
            let c = oc.dot(oc) - sphere.radius * sphere.radius;
            let disc = half_b * half_b - a * c;
            if disc < 0.0 { continue; }
            let sqrtd = disc.sqrt();
            let mut root = (-half_b - sqrtd) / a;
            if root < t_min || root > closest_t {
                root = (-half_b + sqrtd) / a;
            }
            if root < t_min || root > closest_t { continue; }
            closest_t = root;
            let point = ray.at(root);
            let normal = (point - sphere.center) / sphere.radius;
            closest = Some(HitRecord { point, normal, t: root, material: sphere.material });
        }

        for tri in &self.triangles {
            let edge1 = tri.v1 - tri.v0;
            let edge2 = tri.v2 - tri.v0;
            let h = ray.dir.cross(edge2);
            let a = edge1.dot(h);
            if a.abs() < 1e-10 { continue; }
            let f = 1.0 / a;
            let s = ray.origin - tri.v0;
            let u = f * s.dot(h);
            if u < 0.0 || u > 1.0 { continue; }
            let q = s.cross(edge1);
            let v = f * ray.dir.dot(q);
            if v < 0.0 || u + v > 1.0 { continue; }
            let t = f * edge2.dot(q);
            if t < t_min || t > closest_t { continue; }
            closest_t = t;
            let point = ray.at(t);
            let normal = edge1.cross(edge2).normalized();
            let normal = if normal.dot(ray.dir) > 0.0 { normal * -1.0 } else { normal };
            closest = Some(HitRecord { point, normal, t, material: tri.material });
        }

        closest
    }

    fn is_shadowed(&self, ray: &Ray, t_max: f64) -> bool {
        self.intersect(ray, 0.001, t_max).is_some()
    }
}

// ============================================================
// Camera
// ============================================================

struct Camera {
    eye: Vec3,
    target: Vec3,
    up: Vec3,
    fov: f64,
    aspect: f64,
    near: f64,
    far: f64,
}

impl Camera {
    fn new(eye: Vec3, target: Vec3, up: Vec3, fov: f64, aspect: f64) -> Self {
        Self { eye, target, up, fov, aspect, near: 0.1, far: 100.0 }
    }
    fn view(&self) -> Mat4 { Mat4::look_at(self.eye, self.target, self.up) }
    fn projection(&self) -> Mat4 { Mat4::perspective(self.fov, self.aspect, self.near, self.far) }
    fn mvp(&self) -> Mat4 { self.projection().mul(&self.view()) }

    fn get_ray(&self, u: f64, v: f64) -> Ray {
        let fov_rad = self.fov * std::f64::consts::PI / 180.0;
        let h = (fov_rad / 2.0).tan();
        let w = h * self.aspect;
        let fwd = (self.target - self.eye).normalized();
        let right = fwd.cross(self.up).normalized();
        let up = right.cross(fwd);
        let dir = (fwd + right * (u * 2.0 - 1.0) * w + up * (v * 2.0 - 1.0) * h).normalized();
        Ray::new(self.eye, dir)
    }
}

// ============================================================
// Framebuffer
// ============================================================

struct Framebuffer {
    width: usize,
    height: usize,
    color: Vec<Vec3>,
    depth: Vec<f64>,
}

impl Framebuffer {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width, height,
            color: vec![Vec3::zero(); width * height],
            depth: vec![f64::INFINITY; width * height],
        }
    }

    fn set_pixel(&mut self, x: usize, y: usize, c: Vec3) {
        if x < self.width && y < self.height {
            self.color[y * self.width + x] = c;
        }
    }

    fn set_depth(&mut self, x: usize, y: usize, d: f64) {
        if x < self.width && y < self.height {
            self.depth[y * self.width + x] = d;
        }
    }

    fn get_depth(&self, x: usize, y: usize) -> f64 {
        if x < self.width && y < self.height {
            self.depth[y * self.width + x]
        } else {
            f64::INFINITY
        }
    }

    fn accumulate(&mut self, x: usize, y: usize, c: Vec3) {
        if x < self.width && y < self.height {
            self.color[y * self.width + x] += c;
        }
    }
}

fn gamma_encode(v: Vec3) -> Vec3 {
    let inv_gamma = 1.0 / 2.2;
    Vec3::new(v.x.powf(inv_gamma), v.y.powf(inv_gamma), v.z.powf(inv_gamma))
}

fn save_ppm(fb: &Framebuffer, filename: &str) {
    let mut file = File::create(filename).expect("Failed to create PPM file");
    write!(file, "P3\n{} {}\n255\n", fb.width, fb.height).expect("Failed to write PPM header");
    for y in 0..fb.height {
        for x in 0..fb.width {
            let c = gamma_encode(fb.color[y * fb.width + x].clamp01());
            let r = (c.x * 255.0) as u8;
            let g = (c.y * 255.0) as u8;
            let b = (c.z * 255.0) as u8;
            write!(file, "{} {} {} ", r, g, b).expect("Failed to write pixel");
        }
        write!(file, "\n").expect("Failed to write newline");
    }
}

// ============================================================
// The Triangle Rasterizer
// ============================================================

fn edge_function(a: Vec3, b: Vec3, c: Vec3) -> f64 {
    (c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x)
}

fn rasterize(fb: &mut Framebuffer, scene: &Scene, camera: &Camera) {
    let mvp = camera.mvp();
    let w = fb.width as f64;
    let h = fb.height as f64;

    for tri in &scene.triangles {
        let mat = &scene.materials[tri.material];

        let v0_clip = mvp.transform(Vec4::from_point(tri.v0));
        let v1_clip = mvp.transform(Vec4::from_point(tri.v1));
        let v2_clip = mvp.transform(Vec4::from_point(tri.v2));

        if v0_clip.w <= 0.0 && v1_clip.w <= 0.0 && v2_clip.w <= 0.0 { continue; }

        let v0_ndc = Vec3::new(v0_clip.x / v0_clip.w, v0_clip.y / v0_clip.w, v0_clip.z / v0_clip.w);
        let v1_ndc = Vec3::new(v1_clip.x / v1_clip.w, v1_clip.y / v1_clip.w, v1_clip.z / v1_clip.w);
        let v2_ndc = Vec3::new(v2_clip.x / v2_clip.w, v2_clip.y / v2_clip.w, v2_clip.z / v2_clip.w);

        let v0_screen = Vec3::new((v0_ndc.x + 1.0) * 0.5 * w, (1.0 - v0_ndc.y) * 0.5 * h, v0_ndc.z);
        let v1_screen = Vec3::new((v1_ndc.x + 1.0) * 0.5 * w, (1.0 - v1_ndc.y) * 0.5 * h, v1_ndc.z);
        let v2_screen = Vec3::new((v2_ndc.x + 1.0) * 0.5 * w, (1.0 - v2_ndc.y) * 0.5 * h, v2_ndc.z);

        let edge1 = tri.v1 - tri.v0;
        let edge2 = tri.v2 - tri.v0;
        let face_normal = edge1.cross(edge2).normalized();

        let min_x = v0_screen.x.min(v1_screen.x).min(v2_screen.x).max(0.0) as usize;
        let max_x = v0_screen.x.max(v1_screen.x).max(v2_screen.x).min(w - 1.0) as usize;
        let min_y = v0_screen.y.min(v1_screen.y).min(v2_screen.y).max(0.0) as usize;
        let max_y = v0_screen.y.max(v1_screen.y).max(v2_screen.y).min(h - 1.0) as usize;

        let area = edge_function(v0_screen, v1_screen, v2_screen);
        if area.abs() < 1e-10 { continue; }
        let inv_area = 1.0 / area;

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let p = Vec3::new(px as f64 + 0.5, py as f64 + 0.5, 0.0);
                let w0 = edge_function(v1_screen, v2_screen, p);
                let w1 = edge_function(v2_screen, v0_screen, p);
                let w2 = edge_function(v0_screen, v1_screen, p);

                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    let bary0 = w0 * inv_area;
                    let bary1 = w1 * inv_area;
                    let bary2 = w2 * inv_area;

                    let z = bary0 * v0_screen.z + bary1 * v1_screen.z + bary2 * v2_screen.z;

                    if z < fb.get_depth(px, py) {
                        let mut color = mat.emissive;

                        if mat.emissive.near_zero() {
                            let world_point = tri.v0 * bary0 + tri.v1 * bary1 + tri.v2 * bary2;

                            for light in &scene.lights {
                                let to_light = light.position - world_point;
                                let dist = to_light.length();
                                let light_dir = to_light / dist;

                                let shadow_origin = world_point + face_normal * 0.001;
                                let shadow_ray = Ray::new(shadow_origin, light_dir);
                                if scene.is_shadowed(&shadow_ray, dist) { continue; }

                                let n_dot_l = face_normal.dot(light_dir).max(0.0);
                                let attenuation = light.intensity / (dist * dist);
                                let diffuse = mat.diffuse * n_dot_l * attenuation;
                                color = color + diffuse.elem_mul(light.color);
                            }
                            let ambient = mat.diffuse * 0.05;
                            color = color + ambient;
                        }

                        fb.set_depth(px, py, z);
                        fb.set_pixel(px, py, color);
                    }
                }
            }
        }
    }
}

// ============================================================
// The Path Tracer
// ============================================================

fn trace(ray: &Ray, scene: &Scene, depth: u32, rng: &mut SimpleRng) -> Vec3 {
    if depth == 0 {
        return Vec3::zero();
    }

    let hit = match scene.intersect(ray, 0.001, f64::INFINITY) {
        Some(h) => h,
        None => return Vec3::zero(),
    };

    let mat = &scene.materials[hit.material];

    if !mat.emissive.near_zero() {
        return mat.emissive;
    }

    let rr_prob = 0.8;
    if rng.gen() > rr_prob {
        return Vec3::zero();
    }

    let (local_dir, pdf) = cosine_hemisphere(rng);
    let sample_dir = local_to_world(hit.normal, local_dir);

    if sample_dir.near_zero() {
        return Vec3::zero();
    }

    let cos_theta = sample_dir.dot(hit.normal).max(0.0);
    let brdf = mat.diffuse * (1.0 / std::f64::consts::PI);
    let incoming = trace(&Ray::new(hit.point + hit.normal * 0.001, sample_dir), scene, depth - 1, rng);

    let scatter = brdf.elem_mul(incoming) * (cos_theta / (pdf * rr_prob));

    let mut direct = Vec3::zero();
    for light in &scene.lights {
        let to_light = light.position - hit.point;
        let dist = to_light.length();
        let light_dir = to_light / dist;

        let shadow_origin = hit.point + hit.normal * 0.001;
        let shadow_ray = Ray::new(shadow_origin, light_dir);
        if scene.is_shadowed(&shadow_ray, dist - 0.001) { continue; }

        let n_dot_l = hit.normal.dot(light_dir).max(0.0);
        let attenuation = light.intensity / (dist * dist);
        direct = direct + mat.diffuse.elem_mul(light.color) * n_dot_l * attenuation;
    }

    direct + scatter
}

fn path_trace(scene: &Scene, camera: &Camera, samples: u32) -> Framebuffer {
    let width = 400;
    let height = 300;
    let mut fb = Framebuffer::new(width, height);
    let mut rng = SimpleRng::new(42);

    let inv_samples = 1.0 / samples as f64;

    for y in 0..height {
        for x in 0..width {
            let mut color = Vec3::zero();
            for _ in 0..samples {
                let jitter_x = rng.gen();
                let jitter_y = rng.gen();
                let u = (x as f64 + jitter_x) / width as f64;
                let v = (1.0 - (y as f64 + jitter_y) / height as f64);
                let ray = camera.get_ray(u, v);
                color = color + trace(&ray, scene, 8, &mut rng);
            }
            fb.set_pixel(x, y, color * inv_samples);
        }
        if y % 30 == 0 {
            eprintln!("Path tracer: row {}/{}", y, height);
        }
    }

    fb
}

// ============================================================
// Cornell Box Scene
// ============================================================

fn build_cornell_box() -> Scene {
    let white = Material { diffuse: Vec3::new(0.73, 0.73, 0.73), emissive: Vec3::zero() };
    let red   = Material { diffuse: Vec3::new(0.63, 0.06, 0.06), emissive: Vec3::zero() };
    let green = Material { diffuse: Vec3::new(0.14, 0.45, 0.09), emissive: Vec3::zero() };
    let light_mat = Material { diffuse: Vec3::zero(), emissive: Vec3::new(15.0, 15.0, 10.0) };
    let sphere_red = Material { diffuse: Vec3::new(0.8, 0.1, 0.1), emissive: Vec3::zero() };
    let sphere_blue = Material { diffuse: Vec3::new(0.1, 0.1, 0.8), emissive: Vec3::zero() };

    let materials = vec![white, red, green, light_mat, sphere_red, sphere_blue];
    // indices: 0=white, 1=red, 2=green, 3=light, 4=sphere_red, 5=sphere_blue

    let floor = Triangle { v0: Vec3::new(-2.0, -1.0, -2.0), v1: Vec3::new(2.0, -1.0, -2.0), v2: Vec3::new(-2.0, -1.0, 2.0), material: 0 };
    let floor2 = Triangle { v0: Vec3::new(2.0, -1.0, -2.0), v1: Vec3::new(2.0, -1.0, 2.0), v2: Vec3::new(-2.0, -1.0, 2.0), material: 0 };

    let ceiling = Triangle { v0: Vec3::new(-2.0, 2.0, -2.0), v1: Vec3::new(-2.0, 2.0, 2.0), v2: Vec3::new(2.0, 2.0, -2.0), material: 0 };
    let ceiling2 = Triangle { v0: Vec3::new(2.0, 2.0, -2.0), v1: Vec3::new(-2.0, 2.0, 2.0), v2: Vec3::new(2.0, 2.0, 2.0), material: 0 };

    let back = Triangle { v0: Vec3::new(-2.0, -1.0, -2.0), v1: Vec3::new(-2.0, 2.0, -2.0), v2: Vec3::new(2.0, -1.0, -2.0), material: 0 };
    let back2 = Triangle { v0: Vec3::new(2.0, -1.0, -2.0), v1: Vec3::new(-2.0, 2.0, -2.0), v2: Vec3::new(2.0, 2.0, -2.0), material: 0 };

    let left_wall = Triangle { v0: Vec3::new(-2.0, -1.0, -2.0), v1: Vec3::new(-2.0, -1.0, 2.0), v2: Vec3::new(-2.0, 2.0, -2.0), material: 2 }; // green
    let left_wall2 = Triangle { v0: Vec3::new(-2.0, -1.0, 2.0), v1: Vec3::new(-2.0, 2.0, 2.0), v2: Vec3::new(-2.0, 2.0, -2.0), material: 2 };

    let right_wall = Triangle { v0: Vec3::new(2.0, -1.0, -2.0), v1: Vec3::new(2.0, 2.0, -2.0), v2: Vec3::new(2.0, -1.0, 2.0), material: 1 }; // red
    let right_wall2 = Triangle { v0: Vec3::new(2.0, -1.0, 2.0), v1: Vec3::new(2.0, 2.0, -2.0), v2: Vec3::new(2.0, 2.0, 2.0), material: 1 };

    let light_a = Triangle { v0: Vec3::new(-0.5, 1.98, -0.5), v1: Vec3::new(-0.5, 1.98, 0.5), v2: Vec3::new(0.5, 1.98, -0.5), material: 3 };
    let light_b = Triangle { v0: Vec3::new(-0.5, 1.98, 0.5), v1: Vec3::new(0.5, 1.98, 0.5), v2: Vec3::new(0.5, 1.98, -0.5), material: 3 };

    let triangles = vec![
        floor, floor2, ceiling, ceiling2,
        back, back2,
        left_wall, left_wall2,
        right_wall, right_wall2,
        light_a, light_b,
    ];

    let sphere1 = Sphere { center: Vec3::new(-0.5, -0.35, 0.3), radius: 0.65, material: 4 };
    let sphere2 = Sphere { center: Vec3::new(0.7, -0.55, -0.5), radius: 0.45, material: 5 };
    let spheres = vec![sphere1, sphere2];

    let light = Light {
        position: Vec3::new(0.0, 1.95, 0.0),
        color: Vec3::new(1.0, 1.0, 0.9),
        intensity: 8.0,
    };
    let lights = vec![light];

    Scene { spheres, triangles, lights, materials }
}

// ============================================================
// Main
// ============================================================

fn main() {
    eprintln!("=== Phase 14 Capstone: Dual Renderer ===");
    eprintln!("Building Cornell Box scene...");

    let scene = build_cornell_box();
    let camera = Camera::new(
        Vec3::new(0.0, 0.5, 3.5),
        Vec3::new(0.0, 0.3, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        60.0,
        400.0 / 300.0,
    );

    eprintln!("\n--- Rasterizer ---");
    let start = std::time::Instant::now();
    let mut fb_rast = Framebuffer::new(400, 300);
    rasterize(&mut fb_rast, &scene, &camera);
    let rast_time = start.elapsed();
    save_ppm(&fb_rast, "rasterizer.ppm");
    eprintln!("Rasterizer: 400x300 in {:?}", rast_time);
    eprintln!("Output: rasterizer.ppm");

    eprintln!("\n--- Path Tracer ---");
    let start = std::time::Instant::now();
    let fb_path = path_trace(&scene, &camera, 32);
    let path_time = start.elapsed();
    save_ppm(&fb_path, "pathtracer.ppm");
    eprintln!("Path tracer: 400x300 x 32 samples in {:?}", path_time);
    eprintln!("Output: pathtracer.ppm");

    eprintln!("\n=== Comparison ===");
    eprintln!("Rasterizer: deterministic, no noise, direct lighting only, hard shadows");
    eprintln!("Path tracer: stochastic, some noise, global illumination, soft shadows, color bleeding");
    eprintln!("Look for: red/green color bleeding on the floor, soft shadow penumbrae");
}