//! Physically Based Rendering — BRDF, Microfacet
//! Phase 14 — Computer Graphics & Visualization
//!
//! Evaluates Cook-Torrance BRDF on CPU and renders a PBR-lit sphere to PPM.

use std::fs::File;
use std::io::Write;

const PI: f64 = std::f64::consts::PI;

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

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    fn normalized(self) -> Self {
        let len = self.length();
        if len < 1e-10 {
            Self::new(0.0, 0.0, 0.0)
        } else {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
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

    fn mix(self, other: Self, t: f64) -> Self {
        self.scale(1.0 - t).add(other.scale(t))
    }

    fn clamp01(self) -> Self {
        Self {
            x: self.x.clamp(0.0, 1.0),
            y: self.y.clamp(0.0, 1.0),
            z: self.z.clamp(0.0, 1.0),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Material {
    albedo: Vec3,
    metallic: f64,
    roughness: f64,
    ao: f64,
}

fn ggx_ndf(n_dot_h: f64, roughness: f64) -> f64 {
    let a = roughness * roughness;
    let a2 = a * a;
    let n_dot_h2 = n_dot_h * n_dot_h;
    let denom = n_dot_h2 * (a2 - 1.0) + 1.0;
    a2 / (PI * denom * denom).max(1e-8)
}

fn smith_ggx(n_dot_v: f64, n_dot_l: f64, roughness: f64) -> f64 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    let g1_v = n_dot_v / (n_dot_v * (1.0 - k) + k).max(1e-8);
    let g1_l = n_dot_l / (n_dot_l * (1.0 - k) + k).max(1e-8);
    g1_v * g1_l
}

fn schlick_fresnel(cos_theta: f64, f0: Vec3) -> Vec3 {
    let t = (1.0 - cos_theta).clamp(0.0, 1.0);
    let t5 = t * t * t * t * t;
    f0.add(Vec3::new(1.0, 1.0, 1.0).sub(f0).scale(t5))
}

fn cook_torrance_brdf(
    n: Vec3,
    v: Vec3,
    l: Vec3,
    mat: Material,
) -> Vec3 {
    let h = v.add(l).normalized();
    let n_dot_v = (n.dot(v)).max(0.0);
    let n_dot_l = (n.dot(l)).max(0.0);
    let n_dot_h = (n.dot(h)).max(0.0);
    let v_dot_h = (v.dot(h)).max(0.0);

    let f0 = Vec3::new(0.04, 0.04, 0.04).mix(mat.albedo, mat.metallic);

    let d = ggx_ndf(n_dot_h, mat.roughness);
    let g = smith_ggx(n_dot_v, n_dot_l, mat.roughness);
    let f = schlick_fresnel(v_dot_h, f0);

    let numerator = d * g;
    let denominator = (4.0 * n_dot_v.max(0.001) * n_dot_l.max(0.001)).max(1e-8);
    let specular = f.scale(numerator / denominator);

    let kd = Vec3::new(1.0, 1.0, 1.0).sub(f).scale(1.0 - mat.metallic);
    let diffuse = kd.mul(mat.albedo).scale(1.0 / PI);

    diffuse.add(specular)
}

fn reinhard_tonemap(color: Vec3) -> Vec3 {
    color.mul(Vec3::new(1.0, 1.0, 1.0).add(color)).scale(1.0)
        .add(color.recip())
}

fn gamma_correct(c: f64) -> u8 {
    let linear = c.clamp(0.0, 1.0);
    let srgb = linear.powf(1.0 / 2.2);
    (srgb * 255.0).round() as u8
}

fn to_srgb(v: Vec3) -> (u8, u8, u8) {
    let tm = reinhard_tonemap(v.clamp01());
    let gc = Vec3::new(
        gamma_correct(tm.x),
        gamma_correct(tm.y),
        gamma_correct(tm.z),
    );
    // We'll reinterpret below
    (gc.x as u8, gc.y as u8, gc.z as u8)
}

fn main() {
    let width = 512;
    let height = 512;

    let materials = vec![
        ("Gold",    Material { albedo: Vec3::new(1.0, 0.76, 0.34), metallic: 1.0, roughness: 0.3, ao: 1.0 }),
        ("Chrome",  Material { albedo: Vec3::new(0.55, 0.55, 0.55), metallic: 1.0, roughness: 0.15, ao: 1.0 }),
        ("Plastic", Material { albedo: Vec3::new(0.8, 0.1, 0.1), metallic: 0.0, roughness: 0.4, ao: 1.0 }),
        ("Rubber",  Material { albedo: Vec3::new(0.2, 0.2, 0.2), metallic: 0.0, roughness: 0.9, ao: 1.0 }),
    ];

    let light_pos = Vec3::new(2.0, 3.0, 4.0);
    let light_color = Vec3::new(5.0, 5.0, 5.0);
    let cam_pos = Vec3::new(0.0, 0.0, 4.0);

    let tile_w = width / 2;
    let tile_h = height / 2;

    let mut pixels: Vec<u8> = vec![0; width * height * 3];

    for (idx, (_name, mat)) in materials.iter().enumerate() {
        let ox = (idx % 2) * tile_w;
        let oy = (idx / 2) * tile_h;
        let cx = tile_w / 2;
        let cy = tile_h / 2;
        let radius = (tile_w.min(tile_h) as f64) * 0.38;

        for py in 0..tile_h {
            for px in 0..tile_w {
                let sx = px as f64 - cx as f64;
                let sy = -(py as f64 - cy as f64);
                let dist2 = sx * sx + sy * sy;
                let r2 = radius * radius;

                let px_abs = ox + px;
                let py_abs = oy + py;

                if dist2 > r2 {
                    let off = (py_abs * width + px_abs) * 3;
                    pixels[off] = 30;
                    pixels[off + 1] = 30;
                    pixels[off + 2] = 36;
                    continue;
                }

                let zn = (1.0 - dist2 / r2).sqrt();
                let n = Vec3::new(sx / radius, sy / radius, zn).normalized();
                let v = cam_pos.normalized();
                let l = light_pos.normalized();
                let h = v.add(l).normalized();

                let n_dot_l = n.dot(l).max(0.0);
                let brdf = cook_torrance_brdf(n, v, l, *mat);

                let dist = light_pos.length();
                let attenuation = 1.0 / (dist * dist);
                let radiance = light_color.scale(attenuation);
                let color = brdf.mul(radiance).scale(n_dot_l);
                let ambient = mat.albedo.scale(0.03);
                let final_color = color.add(ambient);

                let (r, g, b) = to_srgb(final_color);
                let off = (py_abs * width + px_abs) * 3;
                pixels[off] = r;
                pixels[off + 1] = g;
                pixels[off + 2] = b;
            }
        }
    }

    let mut file = File::create("pbr_sphere.ppm").expect("Failed to create PPM file");
    write!(file, "P3\n{} {}\n255\n", width, height).expect("Failed to write header");
    for y in 0..height {
        for x in 0..width {
            let off = (y * width + x) * 3;
            write!(file, "{} {} {} ", pixels[off], pixels[off + 1], pixels[off + 2])
                .expect("Failed to write pixel");
        }
        writeln!(file).expect("Failed to write newline");
    }

    eprintln!("Wrote pbr_sphere.ppm ({}x{}) with 4 materials", width, height);
    eprintln!("Materials: Gold, Chrome, Plastic (Red), Rubber");

    let angles = [0.0_f64, 0.3, 0.6, 0.85];
    eprintln!("\n--- BRDF Debug: Schlick Fresnel for dielectric (F0=0.04) ---");
    for &angle in &angles {
        let cos_t = angle.cos();
        let f = schlick_fresnel(cos_t, Vec3::new(0.04, 0.04, 0.04));
        eprintln!("  theta={:.2}  cos={:.4}  F=({:.4}, {:.4}, {:.4})", angle, cos_t, f.x, f.y, f.z);
    }

    eprintln!("\n--- BRDF Debug: GGX NDF for various roughness (NdotH=0.707) ---");
    for &roughness in &[0.1_f64, 0.3, 0.5, 0.8] {
        let d = ggx_ndf(0.707, roughness);
        eprintln!("  roughness={:.1}  D={:.6}", roughness, d);
    }

    eprintln!("\n--- BRDF Debug: Smith G for various roughness (NdotV=0.7, NdotL=0.5) ---");
    for &roughness in &[0.1_f64, 0.3, 0.5, 0.8] {
        let g = smith_ggx(0.7, 0.5, roughness);
        eprintln!("  roughness={:.1}  G={:.6}", roughness, g);
    }
}