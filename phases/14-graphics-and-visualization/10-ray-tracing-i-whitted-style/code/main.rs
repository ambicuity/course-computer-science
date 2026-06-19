use std::fs::File;
use std::io::Write;

#[derive(Clone, Copy, Default)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vec3 {
    fn new(x: f64, y: f64, z: f64) -> Self {
        Vec3 { x, y, z }
    }
    fn zero() -> Self {
        Vec3 { x: 0.0, y: 0.0, z: 0.0 }
    }
    fn add(&self, v: &Vec3) -> Vec3 {
        Vec3 { x: self.x + v.x, y: self.y + v.y, z: self.z + v.z }
    }
    fn sub(&self, v: &Vec3) -> Vec3 {
        Vec3 { x: self.x - v.x, y: self.y - v.y, z: self.z - v.z }
    }
    fn scale(&self, s: f64) -> Vec3 {
        Vec3 { x: self.x * s, y: self.y * s, z: self.z * s }
    }
    fn mul(&self, v: &Vec3) -> Vec3 {
        Vec3 { x: self.x * v.x, y: self.y * v.y, z: self.z * v.z }
    }
    fn dot(&self, v: &Vec3) -> f64 {
        self.x * v.x + self.y * v.y + self.z * v.z
    }
    fn cross(&self, v: &Vec3) -> Vec3 {
        Vec3 {
            x: self.y * v.z - self.z * v.y,
            y: self.z * v.x - self.x * v.z,
            z: self.x * v.y - self.y * v.x,
        }
    }
    fn length(&self) -> f64 {
        self.dot(self).sqrt()
    }
    fn normalized(&self) -> Vec3 {
        let l = self.length();
        self.scale(1.0 / l)
    }
    fn reflect(&self, n: &Vec3) -> Vec3 {
        self.sub(&n.scale(2.0 * self.dot(n)))
    }
    fn hadamard(&self, v: &Vec3) -> Vec3 {
        Vec3 { x: self.x * v.x, y: self.y * v.y, z: self.z * v.z }
    }
}

#[derive(Clone)]
struct Ray {
    origin: Vec3,
    direction: Vec3,
}

impl Ray {
    fn at(&self, t: f64) -> Vec3 {
        self.origin.add(&self.direction.scale(t))
    }
}

#[derive(Clone, Copy, Default)]
struct Material {
    color: Vec3,
    ambient: f64,
    diffuse: f64,
    specular: f64,
    shininess: f64,
    reflectivity: f64,
    transparency: f64,
    ior: f64,
}

#[derive(Clone)]
struct HitRecord {
    t: f64,
    point: Vec3,
    normal: Vec3,
    material: Material,
}

struct Sphere {
    center: Vec3,
    radius: f64,
    material: Material,
}

impl Sphere {
    fn intersect(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        let oc = ray.origin.sub(&self.center);
        let a = ray.direction.dot(&ray.direction);
        let half_b = oc.dot(&ray.direction);
        let c = oc.dot(&oc) - self.radius * self.radius;
        let discriminant = half_b * half_b - a * c;
        if discriminant < 0.0 {
            return None;
        }
        let sqrtd = discriminant.sqrt();
        let mut root = (-half_b - sqrtd) / a;
        if root < t_min || root > t_max {
            root = (-half_b + sqrtd) / a;
            if root < t_min || root > t_max {
                return None;
            }
        }
        let point = ray.at(root);
        let mut normal = point.sub(&self.center).scale(1.0 / self.radius);
        if ray.direction.dot(&normal) > 0.0 {
            normal = normal.scale(-1.0);
        }
        Some(HitRecord { t: root, point, normal, material: self.material })
    }
}

struct Plane {
    point: Vec3,
    normal: Vec3,
    material: Material,
}

impl Plane {
    fn intersect(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        let denom = ray.direction.dot(&self.normal);
        if denom.abs() < 1e-8 {
            return None;
        }
        let t = self.point.sub(&ray.origin).dot(&self.normal) / denom;
        if t < t_min || t > t_max {
            return None;
        }
        let point = ray.at(t);
        let mut normal = self.normal;
        if denom > 0.0 {
            normal = normal.scale(-1.0);
        }
        let cx = point.x.floor() as i32;
        let cz = point.z.floor() as i32;
        let checkerboard = if (cx + cz) % 2 == 0 {
            Vec3::new(0.9, 0.9, 0.9)
        } else {
            Vec3::new(0.3, 0.3, 0.3)
        };
        let mat = Material {
            color: checkerboard,
            ambient: 0.05,
            diffuse: 0.6,
            specular: 0.2,
            shininess: 10.0,
            reflectivity: 0.2,
            transparency: 0.0,
            ior: 1.0,
        };
        Some(HitRecord { t, point, normal, material: mat })
    }
}

struct Light {
    position: Vec3,
    color: Vec3,
    intensity: f64,
}

struct Scene {
    spheres: Vec<Sphere>,
    planes: Vec<Plane>,
    lights: Vec<Light>,
    background: Vec3,
    max_depth: i32,
}

impl Scene {
    fn trace_ray(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        let mut closest = t_max;
        let mut result: Option<HitRecord> = None;
        for s in &self.spheres {
            if let Some(hit) = s.intersect(ray, t_min, closest) {
                closest = hit.t;
                result = Some(hit);
            }
        }
        for p in &self.planes {
            if let Some(hit) = p.intersect(ray, t_min, closest) {
                closest = hit.t;
                result = Some(hit);
            }
        }
        result
    }

    fn is_shadowed(&self, point: &Vec3, light_dir: &Vec3, light_dist: f64) -> bool {
        let shadow_ray = Ray { origin: *point, direction: *light_dir };
        self.trace_ray(&shadow_ray, 0.001, light_dist).is_some()
    }

    fn shade(&self, rec: &HitRecord, ray: &Ray, depth: i32) -> Vec3 {
        let mut result = Vec3::zero();
        result = result.add(&rec.material.color.scale(rec.material.ambient));

        for light in &self.lights {
            let to_light = light.position.sub(&rec.point);
            let dist = to_light.length();
            let l = to_light.scale(1.0 / dist);
            let shadow_origin = rec.point.add(&rec.normal.scale(0.001));

            if self.is_shadowed(&shadow_origin, &l, dist) {
                continue;
            }
            let diff = rec.normal.dot(&l).max(0.0);
            result = result.add(&rec.material.color.mul(&light.color).scale(
                rec.material.diffuse * diff * light.intensity,
            ));
            let r = l.scale(-1.0).reflect(&rec.normal);
            let v = ray.origin.sub(&rec.point).normalized();
            let spec = r.dot(&v).max(0.0).powf(rec.material.shininess);
            result = result.add(&light.color.scale(rec.material.specular * spec * light.intensity));
        }

        if rec.material.reflectivity > 0.0 && depth > 0 {
            let reflect_dir = ray.direction.reflect(&rec.normal);
            let reflect_ray = Ray {
                origin: rec.point.add(&rec.normal.scale(0.001)),
                direction: reflect_dir,
            };
            let reflect_color = self.compute_color(&reflect_ray, depth - 1);
            result = result.add(&reflect_color.scale(rec.material.reflectivity));
        }

        if rec.material.transparency > 0.0 && depth > 0 {
            let (outward_normal, ni_over_nt, _entering) = if ray.direction.dot(&rec.normal) < 0.0 {
                (rec.normal, 1.0 / rec.material.ior, true)
            } else {
                (rec.normal.scale(-1.0), rec.material.ior, false)
            };
            let cos_i = ray.direction.scale(-1.0).dot(&outward_normal).max(0.0);
            let sin2_t = ni_over_nt * ni_over_nt * (1.0 - cos_i * cos_i);

            if sin2_t <= 1.0 {
                let cos_t = (1.0 - sin2_t).sqrt();
                let refract_dir = ray.direction
                    .scale(ni_over_nt)
                    .add(&outward_normal.scale(ni_over_nt * cos_i - cos_t))
                    .normalized();
                let refract_ray = Ray {
                    origin: rec.point.sub(&outward_normal.scale(0.001)),
                    direction: refract_dir,
                };
                let refract_color = self.compute_color(&refract_ray, depth - 1);
                result = result.add(&refract_color.scale(rec.material.transparency));
            }
        }
        result
    }

    fn compute_color(&self, ray: &Ray, depth: i32) -> Vec3 {
        if depth <= 0 {
            return self.background;
        }
        match self.trace_ray(ray, 0.001, 1e9) {
            Some(rec) => self.shade(&rec, ray, depth),
            None => self.background,
        }
    }
}

fn clamp01(v: f64) -> i32 {
    (v * 255.0).min(255.0).max(0.0) as i32
}

fn main() {
    let w = 800;
    let h = 600;

    let red_mat = Material {
        color: Vec3::new(0.8, 0.2, 0.2), ambient: 0.1, diffuse: 0.6,
        specular: 0.3, shininess: 50.0, reflectivity: 0.3, transparency: 0.0, ior: 1.0,
    };
    let blue_mat = Material {
        color: Vec3::new(0.2, 0.2, 0.8), ambient: 0.1, diffuse: 0.6,
        specular: 0.3, shininess: 50.0, reflectivity: 0.3, transparency: 0.0, ior: 1.0,
    };
    let mirror_mat = Material {
        color: Vec3::new(0.9, 0.9, 0.9), ambient: 0.05, diffuse: 0.2,
        specular: 0.8, shininess: 200.0, reflectivity: 0.8, transparency: 0.0, ior: 1.0,
    };
    let glass_mat = Material {
        color: Vec3::new(1.0, 1.0, 1.0), ambient: 0.05, diffuse: 0.1,
        specular: 0.3, shininess: 50.0, reflectivity: 0.1, transparency: 0.8, ior: 1.5,
    };

    let scene = Scene {
        spheres: vec![
            Sphere { center: Vec3::new(0.0, 1.0, -4.0), radius: 1.0, material: red_mat },
            Sphere { center: Vec3::new(-2.5, 0.7, -3.0), radius: 0.7, material: mirror_mat },
            Sphere { center: Vec3::new(2.5, 1.0, -5.0), radius: 1.0, material: glass_mat },
            Sphere { center: Vec3::new(1.2, 0.5, -2.0), radius: 0.5, material: blue_mat },
        ],
        planes: vec![
            Plane { point: Vec3::new(0.0, 0.0, 0.0), normal: Vec3::new(0.0, 1.0, 0.0), material: Material::default() },
        ],
        lights: vec![
            Light { position: Vec3::new(-5.0, 8.0, -2.0), color: Vec3::new(1.0, 1.0, 1.0), intensity: 1.0 },
            Light { position: Vec3::new(5.0, 6.0, 1.0), color: Vec3::new(0.8, 0.8, 1.0), intensity: 0.6 },
        ],
        background: Vec3::new(0.2, 0.3, 0.5),
        max_depth: 5,
    };

    let cam_pos = Vec3::new(0.0, 2.0, 2.0);
    let cam_target = Vec3::new(0.0, 1.0, -3.0);
    let cam_up = Vec3::new(0.0, 1.0, 0.0);
    let forward = cam_target.sub(&cam_pos).normalized();
    let right = forward.cross(&cam_up).normalized();
    let up = right.cross(&forward);
    let fov = 60.0_f64;
    let aspect = w as f64 / h as f64;
    let half_h = (fov * 0.5 * std::f64::consts::PI / 180.0).tan();
    let half_w = half_h * aspect;

    let mut file = File::create("output.ppm").expect("create output.ppm");
    write!(file, "P3\n{} {}\n255\n", w, h).unwrap();
    for j in 0..h {
        for i in 0..w {
            let u = (2.0 * ((i as f64) + 0.5) / (w as f64) - 1.0) * half_w;
            let v = (1.0 - 2.0 * ((j as f64) + 0.5) / (h as f64)) * half_h;
            let dir = forward.add(&right.scale(u)).add(&up.scale(v)).normalized();
            let ray = Ray { origin: cam_pos, direction: dir };
            let col = scene.compute_color(&ray, scene.max_depth);
            write!(
                file,
                "{} {} {}\n",
                clamp01(col.x),
                clamp01(col.y),
                clamp01(col.z)
            ).unwrap();
        }
    }
    eprintln!("Rendered {}x{} to output.ppm", w, h);
}