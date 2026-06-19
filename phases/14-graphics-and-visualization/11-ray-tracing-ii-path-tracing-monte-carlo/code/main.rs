use std::fs::File;
use std::io::Write;

const WIDTH: u32 = 400;
const HEIGHT: u32 = 300;
const SAMPLES_PER_PIXEL: u32 = 32;
const MAX_DEPTH: u32 = 8;
const RR_MIN_DEPTH: u32 = 3;
const EPSILON: f64 = 1e-4;

#[derive(Clone, Copy)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vec3 {
    fn zero() -> Self {
        Vec3 { x: 0.0, y: 0.0, z: 0.0 }
    }
    fn new(x: f64, y: f64, z: f64) -> Self {
        Vec3 { x, y, z }
    }
    fn add(self, o: Vec3) -> Vec3 {
        Vec3 { x: self.x + o.x, y: self.y + o.y, z: self.z + o.z }
    }
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3 { x: self.x - o.x, y: self.y - o.y, z: self.z - o.z }
    }
    fn mul(self, s: f64) -> Vec3 {
        Vec3 { x: self.x * s, y: self.y * s, z: self.z * s }
    }
    fn mul_vec(self, o: Vec3) -> Vec3 {
        Vec3 { x: self.x * o.x, y: self.y * o.y, z: self.z * o.z }
    }
    fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    fn len(self) -> f64 {
        self.dot(self).sqrt()
    }
    fn normalize(self) -> Vec3 {
        let l = self.len();
        if l > 0.0 { self.mul(1.0 / l) } else { self }
    }
    fn reflect(self, n: Vec3) -> Vec3 {
        self.sub(n.mul(2.0 * self.dot(n)))
    }
    fn max_component(self) -> f64 {
        self.x.max(self.y).max(self.z)
    }
    fn near_zero(self) -> bool {
        self.x.abs() < EPSILON && self.y.abs() < EPSILON && self.z.abs() < EPSILON
    }
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

#[derive(Clone, Copy)]
struct Ray {
    origin: Vec3,
    direction: Vec3,
}

impl Ray {
    fn new(origin: Vec3, direction: Vec3) -> Self {
        Ray { origin, direction: direction.normalize() }
    }
    fn at(self, t: f64) -> Vec3 {
        self.origin.add(self.direction.mul(t))
    }
}

#[derive(Clone, Copy)]
enum MaterialType {
    Diffuse,
    Mirror,
}

#[derive(Clone, Copy)]
struct Material {
    albedo: Vec3,
    emission: Vec3,
    mat_type: MaterialType,
}

impl Material {
    fn diffuse(albedo: Vec3) -> Self {
        Material { albedo, emission: Vec3::zero(), mat_type: MaterialType::Diffuse }
    }
    fn diffuse_emissive(albedo: Vec3, emission: Vec3) -> Self {
        Material { albedo, emission, mat_type: MaterialType::Diffuse }
    }
    fn mirror(albedo: Vec3) -> Self {
        Material { albedo, emission: Vec3::zero(), mat_type: MaterialType::Mirror }
    }
}

struct Hit {
    point: Vec3,
    normal: Vec3,
    t: f64,
    material: Material,
}

trait Hittable: Sync {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<Hit>;
}

struct Sphere {
    center: Vec3,
    radius: f64,
    material: Material,
}

impl Hittable for Sphere {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<Hit> {
        let oc = ray.origin.sub(self.center);
        let a = ray.direction.dot(ray.direction);
        let half_b = oc.dot(ray.direction);
        let c = oc.dot(oc) - self.radius * self.radius;
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
        let outward_normal = point.sub(self.center).mul(1.0 / self.radius);
        let normal = if ray.direction.dot(outward_normal) < 0.0 {
            outward_normal
        } else {
            outward_normal.mul(-1.0)
        };
        Some(Hit { point, normal, t: root, material: self.material })
    }
}

struct Plane {
    point: Vec3,
    normal: Vec3,
    material: Material,
}

impl Hittable for Plane {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<Hit> {
        let denom = ray.direction.dot(self.normal);
        if denom.abs() < EPSILON {
            return None;
        }
        let t = self.point.sub(ray.origin).dot(self.normal) / denom;
        if t < t_min || t > t_max {
            return None;
        }
        let point = ray.at(t);
        let normal = if denom < 0.0 { self.normal } else { self.normal.mul(-1.0) };
        Some(Hit { point, normal, t, material: self.material })
    }
}

struct Scene {
    objects: Vec<Box<dyn Hittable>>,
}

impl Scene {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<Hit> {
        let mut closest: Option<Hit> = None;
        let mut closest_t = t_max;
        for obj in &self.objects {
            if let Some(h) = obj.hit(ray, t_min, closest_t) {
                closest_t = h.t;
                closest = Some(h);
            }
        }
        closest
    }
}

struct Camera {
    origin: Vec3,
    lower_left: Vec3,
    horizontal: Vec3,
    vertical: Vec3,
}

impl Camera {
    fn new(look_from: Vec3, look_at: Vec3, vup: Vec3, vfov: f64, aspect: f64) -> Self {
        let theta = vfov * std::f64::consts::PI / 180.0;
        let h = (theta / 2.0).tan();
        let viewport_height = 2.0 * h;
        let viewport_width = aspect * viewport_height;
        let w = look_from.sub(look_at).normalize();
        let u = cross(vup, w).normalize();
        let v = cross(w, u);
        Camera {
            origin: look_from,
            lower_left: look_from
                .sub(u.mul(viewport_width / 2.0))
                .sub(v.mul(viewport_height / 2.0))
                .sub(w),
            horizontal: u.mul(viewport_width),
            vertical: v.mul(viewport_height),
        }
    }
    fn get_ray(&self, s: f64, t: f64) -> Ray {
        let direction = self.lower_left
            .add(self.horizontal.mul(s))
            .add(self.vertical.mul(t))
            .sub(self.origin);
        Ray::new(self.origin, direction)
    }
}

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng { state: seed }
    }
    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
    fn gen_float(&mut self) -> f64 {
        (self.next_u64() as f64) / (u64::MAX as f64)
    }
    fn gen_range(&mut self, min: f64, max: f64) -> f64 {
        min + (max - min) * self.gen_float()
    }
}

fn cosine_weighted_hemisphere(normal: Vec3, rng: &mut Rng) -> Vec3 {
    let r1 = rng.gen_float();
    let r2 = rng.gen_float();
    let phi = 2.0 * std::f64::consts::PI * r2;
    let sin_theta = r1.sqrt();
    let x = sin_theta * phi.cos();
    let y = sin_theta * phi.sin();
    let z = (1.0 - r1).sqrt();

    let local = Vec3::new(x, y, z);

    let w = if normal.z.abs() > 0.999 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        cross(normal, Vec3::new(0.0, 0.0, 1.0)).normalize()
    };
    let u = cross(w, normal).normalize();
    let v = cross(normal, u);

    u.mul(local.x).add(v.mul(local.y)).add(normal.mul(local.z)).normalize()
}

fn create_onb(normal: Vec3) -> (Vec3, Vec3, Vec3) {
    let w = normal.normalize();
    let u = if w.x.abs() > 0.9 {
        cross(Vec3::new(0.0, 1.0, 0.0), w).normalize()
    } else {
        cross(Vec3::new(1.0, 0.0, 0.0), w).normalize()
    };
    let v = cross(w, u).normalize();
    (u, v, w)
}

fn cosine_hemisphere_sample(normal: Vec3, rng: &mut Rng) -> Vec3 {
    let r1: f64 = rng.gen_float();
    let r2: f64 = rng.gen_float();
    let phi = 2.0 * std::f64::consts::PI * r2;
    let cos_theta = (1.0 - r1).sqrt();
    let sin_theta = r1.sqrt();
    let (u, v, w) = create_onb(normal);
    u.mul(sin_theta * phi.cos())
        .add(v.mul(sin_theta * phi.sin()))
        .add(w.mul(cos_theta))
}

fn path_trace(ray: &Ray, scene: &Scene, rng: &mut Rng, depth: u32) -> Vec3 {
    if depth > MAX_DEPTH {
        return Vec3::zero();
    }

    let hit = scene.hit(ray, EPSILON, f64::MAX);
    match hit {
        None => {
            let t = 0.5 * (ray.direction.normalize().y + 1.0);
            Vec3::new(1.0, 1.0, 1.0).mul(1.0 - t).add(Vec3::new(0.5, 0.7, 1.0).mul(t)).mul(0.15)
        }
        Some(h) => {
            let mut color = h.material.emission;

            match h.material.mat_type {
                MaterialType::Mirror => {
                    let reflected = ray.direction.reflect(h.normal);
                    let mirror_ray = Ray::new(
                        h.point.add(h.normal.mul(EPSILON)),
                        reflected,
                    );
                    color = color.add(h.material.albedo.mul_vec(path_trace(&mirror_ray, scene, rng, depth + 1)));
                }
                MaterialType::Diffuse => {
                    let p_continue = if depth >= RR_MIN_DEPTH {
                        h.material.albedo.max_component().min(1.0)
                    } else {
                        1.0
                    };

                    if depth >= RR_MIN_DEPTH && rng.gen_float() > p_continue {
                        return color;
                    }

                    let direction = cosine_hemisphere_sample(h.normal, rng);
                    if direction.near_zero() {
                        return color;
                    }

                    let bounce_ray = Ray::new(
                        h.point.add(h.normal.mul(EPSILON)),
                        direction,
                    );

                    let incoming = path_trace(&bounce_ray, scene, rng, depth + 1);

                    let contribution = h.material.albedo.mul_vec(incoming);

                    if depth >= RR_MIN_DEPTH {
                        color = color.add(contribution.mul(1.0 / p_continue));
                    } else {
                        color = color.add(contribution);
                    }
                }
            }

            color
        }
    }
}

fn build_scene() -> Scene {
    let ground = Plane {
        point: Vec3::new(0.0, -0.5, 0.0),
        normal: Vec3::new(0.0, 1.0, 0.0),
        material: Material::diffuse(Vec3::new(0.8, 0.8, 0.8)),
    };

    let red_sphere = Sphere {
        center: Vec3::new(-1.0, 0.0, 3.0),
        radius: 0.7,
        material: Material::diffuse(Vec3::new(0.9, 0.1, 0.1)),
    };

    let blue_sphere = Sphere {
        center: Vec3::new(1.0, 0.0, 3.0),
        radius: 0.7,
        material: Material::diffuse(Vec3::new(0.1, 0.2, 0.9)),
    };

    let mirror_sphere = Sphere {
        center: Vec3::new(0.0, 0.0, 6.0),
        radius: 0.7,
        material: Material::mirror(Vec3::new(0.95, 0.95, 0.95)),
    };

    let light_sphere = Sphere {
        center: Vec3::new(0.0, 3.5, 3.0),
        radius: 1.0,
        material: Material::diffuse_emissive(
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(6.0, 6.0, 5.0),
        ),
    };

    let ceiling_light = Sphere {
        center: Vec3::new(-0.5, 3.5, 5.5),
        radius: 0.6,
        material: Material::diffuse_emissive(
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(4.0, 4.0, 4.5),
        ),
    };

    let back_wall = Plane {
        point: Vec3::new(0.0, 0.0, 15.0),
        normal: Vec3::new(0.0, 0.0, -1.0),
        material: Material::diffuse(Vec3::new(0.6, 0.6, 0.7)),
    };

    Scene {
        objects: vec![
            Box::new(ground),
            Box::new(red_sphere),
            Box::new(blue_sphere),
            Box::new(mirror_sphere),
            Box::new(light_sphere),
            Box::new(ceiling_light),
            Box::new(back_wall),
        ],
    }
}

fn clamp(x: f64, min: f64, max: f64) -> f64 {
    if x < min { min } else if x > max { max } else { x }
}

fn tone_map(color: Vec3) -> Vec3 {
    let mapped = Vec3::new(
        color.x / (color.x + 1.0),
        color.y / (color.y + 1.0),
        color.z / (color.z + 1.0),
    );
    Vec3::new(
        mapped.x.powf(1.0 / 2.2),
        mapped.y.powf(1.0 / 2.2),
        mapped.z.powf(1.0 / 2.2),
    )
}

fn to_byte(v: f64) -> u8 {
    (clamp(v, 0.0, 1.0) * 255.0) as u8
}

fn render_pixel(x: u32, y: u32, scene: &Scene, camera: &Camera, base_seed: u64) -> Vec3 {
    let mut color = Vec3::zero();
    let pixel_seed = ((y as u64) * (WIDTH as u64) + (x as u64)) * 10007 + base_seed;

    for s in 0..SAMPLES_PER_PIXEL {
        let mut rng = Rng::new(pixel_seed + (s as u64) * 7919 + 1);
        let u = (x as f64 + rng.gen_float()) / (WIDTH as f64);
        let v = ((HEIGHT - 1 - y) as f64 + rng.gen_float()) / (HEIGHT as f64);
        let ray = camera.get_ray(u, v);
        color = color.add(path_trace(&ray, scene, &mut rng, 0));
    }

    color.mul(1.0 / (SAMPLES_PER_PIXEL as f64))
}

fn main() {
    let scene = build_scene();

    let camera = Camera::new(
        Vec3::new(0.0, 1.5, -1.0),
        Vec3::new(0.0, 0.0, 3.0),
        Vec3::new(0.0, 1.0, 0.0),
        50.0,
        WIDTH as f64 / HEIGHT as f64,
    );

    let mut pixels: Vec<u8> = vec![0; (WIDTH * HEIGHT * 3) as usize];
    let base_seed: u64 = 42;

    eprintln!("Path tracer: {}x{} with {} samples/pixel", WIDTH, HEIGHT, SAMPLES_PER_PIXEL);
    eprintln!("Rendering {} pixels...", WIDTH * HEIGHT);

    for y in 0..HEIGHT {
        if y % 30 == 0 {
            eprintln!("  Row {}/{}", y, HEIGHT);
        }
        for x in 0..WIDTH {
            let color = render_pixel(x, y, &scene, &camera, base_seed);
            let mapped = tone_map(color);
            let idx = ((y * WIDTH + x) * 3) as usize;
            pixels[idx] = to_byte(mapped.x);
            pixels[idx + 1] = to_byte(mapped.y);
            pixels[idx + 2] = to_byte(mapped.z);
        }
    }

    let mut file = File::create("output.ppm").expect("Failed to create output.ppm");
    write!(file, "P3\n{} {}\n255\n", WIDTH, HEIGHT).expect("Failed to write header");
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let idx = ((y * WIDTH + x) * 3) as usize;
            write!(
                file,
                "{} {} {} ",
                pixels[idx], pixels[idx + 1], pixels[idx + 2]
            ).expect("Failed to write pixel");
        }
        writeln!(file).expect("Failed to write newline");
    }

    eprintln!("Done! Wrote output.ppm");
}