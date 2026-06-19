//! Linear Algebra for Graphics — Transforms, Projections
//! Phase 14 — Computer Graphics & Visualization
//!
//! Implements Vec3, Vec4, Mat4 with operator overloading,
//! demonstrates TR vs RT, builds perspective projection,
//! and renders a wireframe cube to PPM.

use std::fmt;
use std::fs::File;
use std::io::Write;

#[derive(Clone, Copy)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vec3 {
    fn new(x: f64, y: f64, z: f64) -> Self {
        Vec3 { x, y, z }
    }

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn cross(self, other: Self) -> Self {
        Vec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    fn normalized(self) -> Self {
        let l = self.length();
        if l < 1e-12 {
            Vec3::new(0.0, 0.0, 0.0)
        } else {
            Vec3::new(self.x / l, self.y / l, self.z / l)
        }
    }

    fn to_vec4(self, w: f64) -> Vec4 {
        Vec4 {
            x: self.x,
            y: self.y,
            z: self.z,
            w,
        }
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, s: f64) -> Self {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

impl fmt::Display for Vec3 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({:.4}, {:.4}, {:.4})", self.x, self.y, self.z)
    }
}

#[derive(Clone, Copy)]
struct Vec4 {
    x: f64,
    y: f64,
    z: f64,
    w: f64,
}

impl Vec4 {
    fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Vec4 { x, y, z, w }
    }

    fn perspective_divide(self) -> Vec3 {
        if self.w.abs() < 1e-12 {
            Vec3::new(0.0, 0.0, 0.0)
        } else {
            Vec3::new(self.x / self.w, self.y / self.w, self.z / self.w)
        }
    }
}

impl fmt::Display for Vec4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "({:.4}, {:.4}, {:.4}, {:.4})",
            self.x, self.y, self.z, self.w
        )
    }
}

#[derive(Clone, Copy)]
struct Mat4 {
    m: [[f64; 4]; 4],
}

impl Mat4 {
    fn identity() -> Self {
        let mut m = [[0.0; 4]; 4];
        for i in 0..4 {
            m[i][i] = 1.0;
        }
        Mat4 { m }
    }

    fn translation(tx: f64, ty: f64, tz: f64) -> Self {
        let mut mat = Mat4::identity();
        mat.m[0][3] = tx;
        mat.m[1][3] = ty;
        mat.m[2][3] = tz;
        mat
    }

    #[allow(dead_code)]
    fn scaling(sx: f64, sy: f64, sz: f64) -> Self {
        let mut m = [[0.0; 4]; 4];
        m[0][0] = sx;
        m[1][1] = sy;
        m[2][2] = sz;
        m[3][3] = 1.0;
        Mat4 { m }
    }

    fn rotation_x(deg: f64) -> Self {
        let r = deg.to_radians();
        let (c, s) = (r.cos(), r.sin());
        let mut mat = Mat4::identity();
        mat.m[1][1] = c;
        mat.m[1][2] = -s;
        mat.m[2][1] = s;
        mat.m[2][2] = c;
        mat
    }

    fn rotation_y(deg: f64) -> Self {
        let r = deg.to_radians();
        let (c, s) = (r.cos(), r.sin());
        let mut mat = Mat4::identity();
        mat.m[0][0] = c;
        mat.m[0][2] = s;
        mat.m[2][0] = -s;
        mat.m[2][2] = c;
        mat
    }

    fn rotation_z(deg: f64) -> Self {
        let r = deg.to_radians();
        let (c, s) = (r.cos(), r.sin());
        let mut mat = Mat4::identity();
        mat.m[0][0] = c;
        mat.m[0][1] = -s;
        mat.m[1][0] = s;
        mat.m[1][1] = c;
        mat
    }

    fn perspective(fov_deg: f64, aspect: f64, near: f64, far: f64) -> Self {
        let fov_rad = fov_deg.to_radians();
        let f = 1.0 / (fov_rad / 2.0).tan();
        let mut m = [[0.0; 4]; 4];
        m[0][0] = f / aspect;
        m[1][1] = f;
        m[2][2] = -(far + near) / (far - near);
        m[2][3] = -(2.0 * far * near) / (far - near);
        m[3][2] = -1.0;
        Mat4 { m }
    }

    fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let forward = (target - eye).normalized();
        let right = forward.cross(up).normalized();
        let true_up = right.cross(forward);
        let mut mat = Mat4::identity();
        mat.m[0][0] = right.x;
        mat.m[0][1] = right.y;
        mat.m[0][2] = right.z;
        mat.m[0][3] = -right.dot(eye);
        mat.m[1][0] = true_up.x;
        mat.m[1][1] = true_up.y;
        mat.m[1][2] = true_up.z;
        mat.m[1][3] = -true_up.dot(eye);
        mat.m[2][0] = -forward.x;
        mat.m[2][1] = -forward.y;
        mat.m[2][2] = -forward.z;
        mat.m[2][3] = forward.dot(eye);
        mat
    }
}

impl std::ops::Mul for Mat4 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        let mut result = [[0.0; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    result[i][j] += self.m[i][k] * rhs.m[k][j];
                }
            }
        }
        Mat4 { m: result }
    }
}

impl std::ops::Mul<Vec4> for Mat4 {
    type Output = Vec4;
    fn mul(self, v: Vec4) -> Vec4 {
        let vals = [v.x, v.y, v.z, v.w];
        Vec4 {
            x: (0..4).map(|k| self.m[0][k] * vals[k]).sum(),
            y: (0..4).map(|k| self.m[1][k] * vals[k]).sum(),
            z: (0..4).map(|k| self.m[2][k] * vals[k]).sum(),
            w: (0..4).map(|k| self.m[3][k] * vals[k]).sum(),
        }
    }
}

impl fmt::Display for Mat4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for row in &self.m {
            writeln!(
                f,
                "  [{:8.4}, {:8.4}, {:8.4}, {:8.4}]",
                row[0], row[1], row[2], row[3]
            )?;
        }
        Ok(())
    }
}

const CUBE_VERTICES: [Vec3; 8] = [
    Vec3 { x: -1.0, y: -1.0, z: -1.0 },
    Vec3 { x:  1.0, y: -1.0, z: -1.0 },
    Vec3 { x:  1.0, y:  1.0, z: -1.0 },
    Vec3 { x: -1.0, y:  1.0, z: -1.0 },
    Vec3 { x: -1.0, y: -1.0, z:  1.0 },
    Vec3 { x:  1.0, y: -1.0, z:  1.0 },
    Vec3 { x:  1.0, y:  1.0, z:  1.0 },
    Vec3 { x: -1.0, y:  1.0, z:  1.0 },
];

const CUBE_EDGES: [(usize, usize); 12] = [
    (0, 1), (1, 2), (2, 3), (3, 0),
    (4, 5), (5, 6), (6, 7), (7, 4),
    (0, 4), (1, 5), (2, 6), (3, 7),
];

fn transform_vertex(
    v: Vec3,
    model: Mat4,
    view: Mat4,
    proj: Mat4,
    width: f64,
    height: f64,
) -> Option<(f64, f64)> {
    let clip = proj * view * model * v.to_vec4(1.0);
    if clip.w < 0.001 {
        return None;
    }
    let ndc = clip.perspective_divide();
    let sx = (ndc.x + 1.0) * 0.5 * width;
    let sy = (1.0 - ndc.y) * 0.5 * height;
    Some((sx, sy))
}

fn render_cube_ppm(filename: &str, angle_deg: f64) {
    let width = 400usize;
    let height = 400usize;
    let model = Mat4::rotation_y(angle_deg) * Mat4::rotation_x(15.0);
    let eye = Vec3::new(0.0, 2.0, 6.0);
    let target = Vec3::new(0.0, 0.0, 0.0);
    let up = Vec3::new(0.0, 1.0, 0.0);
    let view = Mat4::look_at(eye, target, up);
    let proj = Mat4::perspective(60.0, width as f64 / height as f64, 0.1, 100.0);

    let mut screen_pts: [Option<(f64, f64)>; 8] = [None; 8];
    for (i, v) in CUBE_VERTICES.iter().enumerate() {
        screen_pts[i] = transform_vertex(*v, model, view, proj, width as f64, height as f64);
    }

    let mut pixels = vec![[15u8, 15u8, 25u8]; width * height];

    for &(i, j) in &CUBE_EDGES {
        let (a, b) = match (screen_pts[i], screen_pts[j]) {
            (Some(a), Some(b)) => (a, b),
            _ => continue,
        };
        let mut x0 = a.0 as i32;
        let mut y0 = a.1 as i32;
        let x1 = b.0 as i32;
        let y1 = b.1 as i32;
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        loop {
            if x0 >= 0 && x0 < width as i32 && y0 >= 0 && y0 < height as i32 {
                pixels[(y0 as usize) * width + (x0 as usize)] = [0, 255, 160];
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x0 += sx;
            }
            if e2 < dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    for (_idx, pt) in screen_pts.iter().enumerate() {
        if let Some((cx, cy)) = pt {
            let cx = *cx as i32;
            let cy = *cy as i32;
            for ddx in -2..=2 {
                for ddy in -2..=2 {
                    let nx = cx + ddx;
                    let ny = cy + ddy;
                    if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                        pixels[(ny as usize) * width + (nx as usize)] = [255, 220, 50];
                    }
                }
            }
        }
    }

    let mut file = File::create(filename).expect("Failed to create PPM");
    write!(file, "P3\n{} {}\n255\n", width, height).unwrap();
    for row in 0..height {
        for col in 0..width {
            let [r, g, b] = pixels[row * width + col];
            write!(file, "{} {} {} ", r, g, b).unwrap();
        }
        writeln!(file).unwrap();
    }
}

fn demo_tr_vs_rt() {
    println!("=== TR vs RT Demo ===");
    let v = Vec3::new(1.0, 0.0, 0.0);
    let t = Mat4::translation(5.0, 0.0, 0.0);
    let r = Mat4::rotation_z(90.0);
    let clip_tr = t * r * v.to_vec4(1.0);
    let result_tr = clip_tr.perspective_divide();
    let clip_rt = r * t * v.to_vec4(1.0);
    let result_rt = clip_rt.perspective_divide();
    println!("Point v = {}", v);
    println!("T*R*v = {} (TR: translate after rotate)", result_tr);
    println!("R*T*v = {} (RT: rotate after translate)", result_rt);
    println!("They differ! TR gives (5,1,0) vs RT gives (0,6,0)");
    println!();
}

fn demo_perspective_divide() {
    println!("=== Perspective Divide Demo ===");
    let clip_near = Vec4::new(0.0, 0.5, -1.0, 1.0);
    let clip_far = Vec4::new(0.0, 0.5, -10.0, 10.0);
    println!(
        "Near point (z={:.1}, w={:.1}): ndc_y = {:.4}",
        clip_near.z,
        clip_near.w,
        clip_near.y / clip_near.w
    );
    println!(
        "Far point  (z={:.1}, w={:.1}): ndc_y = {:.4}",
        clip_far.z,
        clip_far.w,
        clip_far.y / clip_far.w
    );
    println!("Far point has smaller ndc_y — appears closer to center = 'smaller'");
    println!();
}

fn demo_projection_matrix() {
    println!("=== Perspective Projection Matrix ===");
    let p = Mat4::perspective(60.0, 1.0, 0.1, 100.0);
    println!("Perspective matrix (fov=60, aspect=1, near=0.1, far=100):");
    println!("{}", p);
    let v_eye = Vec4::new(0.0, 0.0, -5.0, 1.0);
    let v_clip = p * v_eye;
    println!("Eye-space point: {}", v_eye);
    println!("Clip-space: {}", v_clip);
    let ndc = v_clip.perspective_divide();
    println!("After perspective divide: {}", ndc);
    println!();
}

fn main() {
    println!("Lesson 14.03: Linear Algebra for Graphics — Transforms, Projections");
    println!("=================================================================\n");
    demo_tr_vs_rt();
    demo_perspective_divide();
    demo_projection_matrix();
    println!("=== Rendering Wireframe Cube ===");
    render_cube_ppm("cube_wireframe.ppm", 30.0);
    println!("Wrote cube_wireframe.ppm (rotate by changing angle_deg)");
    render_cube_ppm("cube_wireframe_0deg.ppm", 0.0);
    println!("Wrote cube_wireframe_0deg.ppm");
    render_cube_ppm("cube_wireframe_60deg.ppm", 60.0);
    println!("Wrote cube_wireframe_60deg.ppm");
    println!();
    println!("Key takeaway: TR and RT give different results — order matters!");
    println!("The perspective divide (÷w) is what makes far things small.");
}