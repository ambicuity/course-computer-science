//! Shading Models — Lambert, Phong, Blinn-Phong
//! Phase 14 — Computer Graphics & Visualization
//!
//! Renders three spheres side-by-side as PPM, one per shading model,
//! so you can compare Lambert, Phong, and Blinn-Phong visually.

use std::fs::File;
use std::io::Write;

const WIDTH: usize = 600;
const HEIGHT: usize = 300;
const PI: f64 = std::f64::consts::PI;

#[derive(Clone, Copy)]
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
    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
    fn length(self) -> f64 {
        self.dot(self).sqrt()
    }
    fn normalize(self) -> Self {
        let l = self.length();
        if l > 0.0 {
            Self {
                x: self.x / l,
                y: self.y / l,
                z: self.z / l,
            }
        } else {
            Self::zero()
        }
    }
    fn reflect(self, n: Self) -> Self {
        Self {
            x: self.x - 2.0 * self.dot(n) * n.x,
            y: self.y - 2.0 * self.dot(n) * n.y,
            z: self.z - 2.0 * self.dot(n) * n.z,
        }
    }
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
    fn scale(self, s: f64) -> Self {
        Self {
            x: self.x * s,
            y: self.y * s,
            z: self.z * s,
        }
    }
    fn mul(self, other: Self) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
        }
    }
    fn clamp(self, lo: f64, hi: f64) -> Self {
        Self {
            x: self.x.max(lo).min(hi),
            y: self.y.max(lo).min(hi),
            z: self.z.max(lo).min(hi),
        }
    }
}

struct Sphere {
    center: Vec3,
    radius: f64,
    diffuse: Vec3,
    specular: Vec3,
    shininess: f64,
}

fn ray_sphere_intersect(origin: Vec3, dir: Vec3, sphere: &Sphere) -> Option<f64> {
    let oc = origin.sub(sphere.center);
    let a = dir.dot(dir);
    let b = 2.0 * oc.dot(dir);
    let c = oc.dot(oc) - sphere.radius * sphere.radius;
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }
    let sqrt_d = discriminant.sqrt();
    let t1 = (-b - sqrt_d) / (2.0 * a);
    let t2 = (-b + sqrt_d) / (2.0 * a);
    if t1 > 0.001 {
        Some(t1)
    } else if t2 > 0.001 {
        Some(t2)
    } else {
        None
    }
}

fn lambert(n: Vec3, l: Vec3, diffuse: Vec3, light_color: Vec3) -> Vec3 {
    let ndotl = n.dot(l).max(0.0);
    diffuse.mul(light_color).scale(ndotl / PI)
}

fn phong(n: Vec3, l: Vec3, v: Vec3, diffuse: Vec3, specular: Vec3, shininess: f64, ambient: Vec3, light_color: Vec3) -> Vec3 {
    let ambient_term = ambient.mul(diffuse);
    let ndotl = n.dot(l).max(0.0);
    let diffuse_term = diffuse.mul(light_color).scale(ndotl);
    let r = Vec3::reflect(l.scale(-1.0), n);
    let rdotv = r.dot(v).max(0.0);
    let spec_pow = rdotv.powf(shininess);
    let specular_term = specular.mul(light_color).scale(spec_pow);
    ambient_term.add(diffuse_term).add(specular_term)
}

fn blinn_phong(n: Vec3, l: Vec3, v: Vec3, diffuse: Vec3, specular: Vec3, shininess: f64, ambient: Vec3, light_color: Vec3) -> Vec3 {
    let ambient_term = ambient.mul(diffuse);
    let ndotl = n.dot(l).max(0.0);
    let diffuse_term = diffuse.mul(light_color).scale(ndotl);
    let h = l.add(v).normalize();
    let ndoth = n.dot(h).max(0.0);
    let spec_pow = ndoth.powf(shininess);
    let specular_term = specular.mul(light_color).scale(spec_pow);
    ambient_term.add(diffuse_term).add(specular_term)
}

#[derive(Clone, Copy)]
enum ShadingModel {
    Lambert,
    Phong,
    BlinnPhong,
}

fn shade_sphere(px: usize, py: usize, sphere: &Sphere, model: ShadingModel) -> Vec3 {
    let aspect = WIDTH as f64 / HEIGHT as f64;
    let fov = 1.0;
    let dir = Vec3::new(
        (2.0 * px as f64 / WIDTH as f64 - 1.0) * aspect * fov,
        (1.0 - 2.0 * py as f64 / HEIGHT as f64) * fov,
        -1.0,
    )
    .normalize();

    let cam = Vec3::new(0.0, 0.0, 3.0);
    let light_dir = Vec3::new(0.577, 0.577, 0.577).normalize();
    let light_color = Vec3::new(1.0, 1.0, 1.0);
    let ambient = Vec3::new(0.1, 0.1, 0.1);

    match ray_sphere_intersect(cam, dir, sphere) {
        Some(t) => {
            let hit = cam.add(dir.scale(t));
            let n = hit.sub(sphere.center).normalize();
            let v = cam.sub(hit).normalize();
            match model {
                ShadingModel::Lambert => lambert(n, light_dir, sphere.diffuse, light_color),
                ShadingModel::Phong => phong(n, light_dir, v, sphere.diffuse, sphere.specular, sphere.shininess, ambient, light_color),
                ShadingModel::BlinnPhong => blinn_phong(n, light_dir, v, sphere.diffuse, sphere.specular, sphere.shininess, ambient, light_color),
            }
        }
        None => Vec3::new(0.15, 0.15, 0.2),
    }
}

fn main() {
    let sphere = Sphere {
        center: Vec3::new(0.0, 0.0, 0.0),
        radius: 1.0,
        diffuse: Vec3::new(0.8, 0.2, 0.2),
        specular: Vec3::new(1.0, 1.0, 1.0),
        shininess: 32.0,
    };

    let models = [
        (ShadingModel::Lambert, "Lambert"),
        (ShadingModel::Phong, "Phong"),
        (ShadingModel::BlinnPhong, "Blinn-Phong"),
    ];

    let panel_w = WIDTH / 3;
    let mut pixels = vec![0u8; WIDTH * HEIGHT * 3];

    for py in 0..HEIGHT {
        for px in 0..WIDTH {
            let panel = if px < panel_w { 0 } else if px < 2 * panel_w { 1 } else { 2 };
            let local_px = px - panel * panel_w;
            let color = shade_sphere(local_px, py, &sphere, models[panel].0);
            let clamped = color.clamp(0.0, 1.0);
            let idx = (py * WIDTH + px) * 3;
            pixels[idx] = (clamped.x * 255.0) as u8;
            pixels[idx + 1] = (clamped.y * 255.0) as u8;
            pixels[idx + 2] = (clamped.z * 255.0) as u8;
        }
    }

    let mut file = File::create("shading_comparison.ppm").expect("Failed to create PPM file");
    write!(file, "P6\n{} {}\n255\n", WIDTH, HEIGHT).expect("Failed to write PPM header");
    file.write_all(&pixels).expect("Failed to write PPM data");

    println!("Wrote shading_comparison.ppm ({}x{})", WIDTH, HEIGHT);
    println!("Left:  Lambert (diffuse only, energy-conserving with 1/pi)");
    println!("Mid:   Phong    (ambient + diffuse + R·V specular)");
    println!("Right: Blinn-Phong (ambient + diffuse + N·H specular)");
}