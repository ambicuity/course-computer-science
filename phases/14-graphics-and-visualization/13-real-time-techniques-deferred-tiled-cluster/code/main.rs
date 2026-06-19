//! Real-Time Techniques — Deferred, Tiled, Cluster
//! Phase 14 — Computer Graphics & Visualization
//!
//! CPU simulation of deferred rendering concepts:
//! 1. Render geometry to G-buffer arrays (position, normal, albedo, depth)
//! 2. Lighting pass reads G-buffer and computes Blinn-Phong shading
//! 3. Compare forward (O(objects * lights)) vs deferred (O(objects + lights)) cost
//! 4. Output PPM images for forward and deferred results

use std::fs;

const WIDTH: usize = 320;
const HEIGHT: usize = 240;

#[derive(Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }
    fn zero() -> Self {
        Vec3 { x: 0.0, y: 0.0, z: 0.0 }
    }
    fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
    fn length(self) -> f32 {
        self.dot(self).sqrt()
    }
    fn normalized(self) -> Self {
        let len = self.length();
        if len > 1e-8 {
            Vec3 { x: self.x / len, y: self.y / len, z: self.z / len }
        } else {
            Vec3::zero()
        }
    }
    fn add(self, other: Self) -> Self {
        Vec3 { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
    }
    fn sub(self, other: Self) -> Self {
        Vec3 { x: self.x - other.x, y: self.y - other.y, z: self.z - other.z }
    }
    fn scale(self, s: f32) -> Self {
        Vec3 { x: self.x * s, y: self.y * s, z: self.z * s }
    }
    fn mul_elem(self, other: Self) -> Self {
        Vec3 { x: self.x * other.x, y: self.y * other.y, z: self.z * other.z }
    }
    fn clamp_color(self) -> (u8, u8, u8) {
        let r = (self.x.max(0.0).min(1.0) * 255.0) as u8;
        let g = (self.y.max(0.0).min(1.0) * 255.0) as u8;
        let b = (self.z.max(0.0).min(1.0) * 255.0) as u8;
        (r, g, b)
    }
}

#[derive(Clone, Copy)]
struct Sphere {
    center: Vec3,
    radius: f32,
    albedo: Vec3,
    specular: f32,
    roughness: f32,
}

#[derive(Clone, Copy)]
struct PointLight {
    position: Vec3,
    color: Vec3,
    radius: f32,
}

struct GBuffer {
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    albedos: Vec<Vec3>,
    speculars: Vec<f32>,
    roughnesses: Vec<f32>,
    depths: Vec<f32>,
    has_geometry: Vec<bool>,
}

impl GBuffer {
    fn new() -> Self {
        let n = WIDTH * HEIGHT;
        GBuffer {
            positions: vec![Vec3::zero(); n],
            normals: vec![Vec3::zero(); n],
            albedos: vec![Vec3::zero(); n],
            speculars: vec![0.0; n],
            roughnesses: vec![0.5; n],
            depths: vec![f32::INFINITY; n],
            has_geometry: vec![false; n],
        }
    }

    fn write_pixel(&mut self, x: usize, y: usize, pos: Vec3, normal: Vec3,
                   albedo: Vec3, spec: f32, rough: f32, depth: f32) {
        let idx = y * WIDTH + x;
        if depth < self.depths[idx] {
            self.positions[idx] = pos;
            self.normals[idx] = normal;
            self.albedos[idx] = albedo;
            self.speculars[idx] = spec;
            self.roughnesses[idx] = rough;
            self.depths[idx] = depth;
            self.has_geometry[idx] = true;
        }
    }
}

fn ray_sphere_intersect(origin: Vec3, direction: Vec3, sphere: &Sphere) -> Option<f32> {
    let oc = origin.sub(sphere.center);
    let a = direction.dot(direction);
    let b = 2.0 * oc.dot(direction);
    let c = oc.dot(oc) - sphere.radius * sphere.radius;
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }
    let sqrt_d = discriminant.sqrt();
    let t1 = (-b - sqrt_d) / (2.0 * a);
    let t2 = (-b + sqrt_d) / (2.0 * a);
    if t1 > 0.001 { return Some(t1); }
    if t2 > 0.001 { return Some(t2); }
    None
}

fn blinn_phong(pos: Vec3, normal: Vec3, albedo: Vec3, spec: f32,
               rough: f32, view_dir: Vec3, lights: &[PointLight]) -> Vec3 {
    let ambient = albedo.scale(0.05);
    let mut result = ambient;
    let shininess = 4.0 + (1.0 - rough) * 252.0;

    for light in lights {
        let light_vec = light.position.sub(pos);
        let dist = light_vec.length();
        if dist > light.radius {
            continue;
        }
        let light_dir = light_vec.scale(1.0 / dist);
        let attenuation = 1.0 / (1.0 + 0.09 * dist + 0.032 * dist * dist);
        let cutoff = 1.0 - ((dist - light.radius * 0.8) / (light.radius * 0.2)).max(0.0).min(1.0);

        let n_dot_l = normal.dot(light_dir).max(0.0);
        let diffuse = albedo.mul_elem(light.color).scale(n_dot_l);

        let half_vec = light_dir.add(view_dir).normalized();
        let n_dot_h = normal.dot(half_vec).max(0.0);
        let spec_intensity = n_dot_h.powf(shininess) * spec;
        let specular = light.color.scale(spec_intensity);

        result = result.add(diffuse.add(specular).scale(attenuation * cutoff));
    }
    result
}

fn geometry_pass(gbuf: &mut GBuffer, spheres: &[Sphere], camera_pos: Vec3) -> u64 {
    let mut ops = 0u64;
    let fov_scale = 1.0;

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let u = (2.0 * x as f32 / WIDTH as f32 - 1.0) * fov_scale * (WIDTH as f32 / HEIGHT as f32);
            let v = (1.0 - 2.0 * y as f32 / HEIGHT as f32) * fov_scale;
            let dir = Vec3::new(u, v, -1.0).normalized();

            let mut closest_t = f32::INFINITY;
            let mut closest_sphere: Option<&Sphere> = None;

            for sphere in spheres {
                if let Some(t) = ray_sphere_intersect(camera_pos, dir, sphere) {
                    if t < closest_t {
                        closest_t = t;
                        closest_sphere = Some(sphere);
                    }
                }
                ops += 1; // each intersection test counts
            }

            if let Some(sphere) = closest_sphere {
                let hit_pos = camera_pos.add(dir.scale(closest_t));
                let normal = hit_pos.sub(sphere.center).normalized();
                let depth = closest_t;
                gbuf.write_pixel(x, y, hit_pos, normal, sphere.albedo,
                                 sphere.specular, sphere.roughness, depth);
            }
        }
    }
    ops
}

fn lighting_pass_deferred(gbuf: &GBuffer, lights: &[PointLight], camera_pos: Vec3) -> (Vec<(u8, u8, u8)>, u64) {
    let mut pixels = vec![(0u8, 0u8, 0u8); WIDTH * HEIGHT];
    let mut ops = 0u64;

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let idx = y * WIDTH + x;
            if !gbuf.has_geometry[idx] {
                pixels[idx] = (10, 10, 20);
                continue;
            }
            let pos = gbuf.positions[idx];
            let normal = gbuf.normals[idx];
            let albedo = gbuf.albedos[idx];
            let spec = gbuf.speculars[idx];
            let rough = gbuf.roughnesses[idx];
            let view_dir = camera_pos.sub(pos).normalized();

            let color = blinn_phong(pos, normal, albedo, spec, rough, view_dir, lights);
            ops += lights.len() as u64;
            pixels[idx] = color.clamp_color();
        }
    }
    (pixels, ops)
}

fn forward_render(spheres: &[Sphere], lights: &[PointLight], camera_pos: Vec3) -> (Vec<(u8, u8, u8)>, u64) {
    let mut pixels = vec![(0u8, 0u8, 0u8); WIDTH * HEIGHT];
    let mut ops = 0u64;
    let fov_scale = 1.0;

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let u = (2.0 * x as f32 / WIDTH as f32 - 1.0) * fov_scale * (WIDTH as f32 / HEIGHT as f32);
            let v = (1.0 - 2.0 * y as f32 / HEIGHT as f32) * fov_scale;
            let dir = Vec3::new(u, v, -1.0).normalized();

            let mut closest_t = f32::INFINITY;
            let mut closest_sphere: Option<&Sphere> = None;

            for sphere in spheres {
                if let Some(t) = ray_sphere_intersect(camera_pos, dir, sphere) {
                    if t < closest_t {
                        closest_t = t;
                        closest_sphere = Some(sphere);
                    }
                }
                ops += 1;
            }

            if let Some(sphere) = closest_sphere {
                let hit_pos = camera_pos.add(dir.scale(closest_t));
                let normal = hit_pos.sub(sphere.center).normalized();
                let view_dir = camera_pos.sub(hit_pos).normalized();
                let color = blinn_phong(hit_pos, normal, sphere.albedo,
                                        sphere.specular, sphere.roughness, view_dir, lights);
                ops += lights.len() as u64;
                pixels[y * WIDTH + x] = color.clamp_color();
            } else {
                pixels[y * WIDTH + x] = (10, 10, 20);
            }
        }
    }
    (pixels, ops)
}

fn write_ppm(filename: &str, pixels: &[(u8, u8, u8)]) {
    let mut content = format!("P3\n{} {}\n255\n", WIDTH, HEIGHT);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let (r, g, b) = pixels[y * WIDTH + x];
            content.push_str(&format!("{} {} {} ", r, g, b));
        }
        content.push('\n');
    }
    fs::write(filename, content).expect("Failed to write PPM");
}

fn main() {
    let camera_pos = Vec3::new(0.0, 0.0, 5.0);

    let spheres = vec![
        Sphere { center: Vec3::new(-1.5, 0.0, -2.0), radius: 1.0,
                 albedo: Vec3::new(0.8, 0.2, 0.2), specular: 0.5, roughness: 0.3 },
        Sphere { center: Vec3::new(0.0, 0.0, -3.0), radius: 1.2,
                 albedo: Vec3::new(0.2, 0.8, 0.2), specular: 0.8, roughness: 0.1 },
        Sphere { center: Vec3::new(1.5, 0.0, -1.5), radius: 1.0,
                 albedo: Vec3::new(0.2, 0.2, 0.8), specular: 0.3, roughness: 0.7 },
        Sphere { center: Vec3::new(0.0, -1.5, -2.5), radius: 0.8,
                 albedo: Vec3::new(0.9, 0.9, 0.9), specular: 0.1, roughness: 0.9 },
    ];

    let num_lights = 50;
    let mut lights = Vec::with_capacity(num_lights);
    for i in 0..num_lights {
        let angle = i as f32 * 2.0 * std::f32::consts::PI / num_lights as f32;
        let radius = 4.0;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius - 3.0;
        let y = (i as f32 / num_lights as f32 * 2.0 - 1.0) * 2.0;
        let r = ((i * 3) % 255) as f32 / 255.0;
        let g = ((i * 7 + 50) % 255) as f32 / 255.0;
        let b = ((i * 11 + 100) % 255) as f32 / 255.0;
        lights.push(PointLight {
            position: Vec3::new(x, y, z),
            color: Vec3::new(r.max(0.3), g.max(0.3), b.max(0.3)),
            radius: 5.0,
        });
    }

    println!("=== Real-Time Techniques: Deferred vs Forward ===");
    println!("Scene: {} spheres, {} lights, {}x{} pixels\n",
             spheres.len(), lights.len(), WIDTH, HEIGHT);

    // --- Deferred Rendering ---
    let mut gbuf = GBuffer::new();
    let geom_ops = geometry_pass(&mut gbuf, &spheres, camera_pos);
    let (deferred_pixels, light_ops) = lighting_pass_deferred(&gbuf, &lights, camera_pos);
    let deferred_total = geom_ops + light_ops;

    println!("Deferred Rendering:");
    println!("  Geometry pass operations: {}", geom_ops);
    println!("  Lighting pass operations: {}", light_ops);
    println!("  Total operations:         {}", deferred_total);
    println!("  Cost model: O(objects + pixels × lights) = O({} + {} × {}) = {}",
             spheres.len(), WIDTH * HEIGHT, lights.len(), deferred_total);

    // --- Forward Rendering ---
    let (forward_pixels, forward_ops) = forward_render(&spheres, &lights, camera_pos);

    println!("\nForward Rendering:");
    println!("  Total operations: {}", forward_ops);
    println!("  Cost model: O(pixels × objects × lights) = {} × {} × {} = {}",
             WIDTH * HEIGHT, spheres.len(), lights.len(),
             (WIDTH * HEIGHT) as u64 * spheres.len() as u64 * lights.len() as u64);

    println!("\nCost comparison:");
    println!("  Deferred total ops:  {:>10}", deferred_total);
    println!("  Forward total ops:   {:>10}", forward_ops);
    println!("  Ratio (forward/deferred): {:.2}x", forward_ops as f64 / deferred_total as f64);

    println!("\nKey insight: Deferred rendering decouples geometry from lighting.");
    println!("  Adding more objects → only geometry pass gets slower.");
    println!("  Adding more lights → only lighting pass gets slower.");
    println!("  Forward couples them: every additional light costs O(objects × pixels).");

    write_ppm("output_deferred.ppm", &deferred_pixels);
    write_ppm("output_forward.ppm", &forward_pixels);

    println!("\nWritten: output_deferred.ppm, output_forward.ppm");
}