//! Build a Software Rasterizer (in Rust)
//! Phase 14 — Computer Graphics & Visualization
//!
//! Renders three PPM images demonstrating the full rasterization pipeline:
//!   1. wireframe_cube.ppm   — Rotating wireframe cube (perspective)
//!   2. flat_cube.ppm        — Flat-shaded cube with Z-buffer
//!   3. lambert_scene.ppm    — Lambert-shaded scene with multiple objects

use std::fs::File;
use std::io::Write;

// ── Vector types ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn zero() -> Self { Self { x: 0.0, y: 0.0, z: 0.0 } }
    fn new(x: f32, y: f32, z: f32) -> Self { Self { x, y, z } }
    fn dot(self, o: Self) -> f32 { self.x * o.x + self.y * o.y + self.z * o.z }
    fn cross(self, o: Self) -> Self {
        Self {
            x: self.y * o.z - self.z * o.y,
            y: self.z * o.x - self.x * o.z,
            z: self.x * o.y - self.y * o.x,
        }
    }
    fn length(self) -> f32 { self.dot(self).sqrt() }
    fn normalized(self) -> Self {
        let l = self.length();
        if l < 1e-8 { Self::zero() } else { Self { x: self.x / l, y: self.y / l, z: self.z / l } }
    }
    fn sub(self, o: Self) -> Self { Self { x: self.x - o.x, y: self.y - o.y, z: self.z - o.z } }
    fn add(self, o: Self) -> Self { Self { x: self.x + o.x, y: self.y + o.y, z: self.z + o.z } }
    fn scale(self, s: f32) -> Self { Self { x: self.x * s, y: self.y * s, z: self.z * s } }
    fn lerp(self, o: Self, t: f32) -> Self { self.scale(1.0 - t).add(o.scale(t)) }
}

#[derive(Clone, Copy)]
struct Vec4 {
    x: f32, y: f32, z: f32, w: f32,
}

impl Vec4 {
    fn new(x: f32, y: f32, z: f32, w: f32) -> Self { Self { x, y, z, w } }
    fn from_vec3(v: Vec3, w: f32) -> Self { Self { x: v.x, y: v.y, z: v.z, w } }
    fn to_vec3(self) -> Vec3 {
        if self.w.abs() < 1e-8 { Vec3::zero() } else {
            Vec3 { x: self.x / self.w, y: self.y / self.w, z: self.z / self.w }
        }
    }
    fn perspective_divide(self) -> Vec3 {
        Vec3 { x: self.x / self.w, y: self.y / self.w, z: self.z / self.w }
    }
}

// ── Matrix ────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Mat4 {
    m: [f32; 16],
}

impl Mat4 {
    fn identity() -> Self {
        Self {
            m: [1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0],
        }
    }

    fn mul(self, o: Self) -> Self {
        let mut r = [0.0f32; 16];
        for row in 0..4 {
            for col in 0..4 {
                let mut sum = 0.0f32;
                for k in 0..4 {
                    sum += self.m[row * 4 + k] * o.m[k * 4 + col];
                }
                r[row * 4 + col] = sum;
            }
        }
        Self { m: r }
    }

    fn transform(self, v: Vec4) -> Vec4 {
        Vec4 {
            x: self.m[0]*v.x + self.m[1]*v.y + self.m[2]*v.z + self.m[3]*v.w,
            y: self.m[4]*v.x + self.m[5]*v.y + self.m[6]*v.z + self.m[7]*v.w,
            z: self.m[8]*v.x + self.m[9]*v.y + self.m[10]*v.z + self.m[11]*v.w,
            w: self.m[12]*v.x + self.m[13]*v.y + self.m[14]*v.z + self.m[15]*v.w,
        }
    }
}

// ── Camera / Projection matrices ──────────────────────────────────────

fn make_perspective(fov_rad: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let t = (fov_rad / 2.0).tan();
    let r = t * aspect;
    let depth = far - near;
    Mat4 {
        m: [
            1.0/r, 0.0,   0.0,                          0.0,
            0.0,   1.0/t, 0.0,                          0.0,
            0.0,   0.0,  -(far + near) / depth,        -2.0 * far * near / depth,
            0.0,   0.0,  -1.0,                          0.0,
        ],
    }
}

fn make_view_matrix(eye: Vec3, center: Vec3, up: Vec3) -> Mat4 {
    let f = center.sub(eye).normalized();
    let s = f.cross(up).normalized();
    let u = s.cross(f);
    Mat4 {
        m: [
            s.x,  u.x, -f.x,  -(s.dot(eye)),
            s.y,  u.y, -f.y,  -(u.dot(eye)),
            s.z,  u.z, -f.z,   f.dot(eye),
            0.0,  0.0,  0.0,   1.0,
        ],
    }
}

fn make_model_matrix(rotation_y: f32, translation: Vec3) -> Mat4 {
    let cy = rotation_y.cos();
    let sy = rotation_y.sin();
    let rot_y = Mat4 {
        m: [
            cy,  0.0, sy,  0.0,
            0.0, 1.0, 0.0, 0.0,
           -sy,  0.0, cy,  0.0,
            0.0, 0.0, 0.0, 1.0,
        ],
    };
    let trans = Mat4 {
        m: [
            1.0, 0.0, 0.0, translation.x,
            0.0, 1.0, 0.0, translation.y,
            0.0, 0.0, 1.0, translation.z,
            0.0, 0.0, 0.0, 1.0,
        ],
    };
    trans.mul(rot_y)
}

// ── Vertex / Triangle / Scene ─────────────────────────────────────────

#[derive(Clone)]
struct Vertex {
    pos: Vec3,
    normal: Vec3,
    color: Vec3,
}

#[derive(Clone)]
struct Triangle {
    v: [Vertex; 3],
}

#[derive(Clone)]
struct Light {
    dir: Vec3,
    color: Vec3,
}

struct Scene {
    triangles: Vec<Triangle>,
}

// ── Framebuffer ───────────────────────────────────────────────────────

struct Framebuffer {
    width: usize,
    height: usize,
    color: Vec<[u8; 3]>,
    depth: Vec<f32>,
}

impl Framebuffer {
    fn new(w: usize, h: usize) -> Self {
        Self {
            width: w,
            height: h,
            color: vec![[0u8; 3]; w * h],
            depth: vec![f32::INFINITY; w * h],
        }
    }

    fn set_pixel(&mut self, x: i32, y: i32, c: [u8; 3]) {
        if x >= 0 && (x as usize) < self.width && y >= 0 && (y as usize) < self.height {
            let idx = (y as usize) * self.width + (x as usize);
            self.color[idx] = c;
        }
    }

    fn clear(&mut self, bg: [u8; 3]) {
        for p in self.color.iter_mut() { *p = bg; }
        for d in self.depth.iter_mut() { *d = f32::INFINITY; }
    }
}

// ── Bresenham line drawing (for wireframe) ────────────────────────────

fn draw_line(fb: &mut Framebuffer, x0: i32, y0: i32, x1: i32, y1: i32, c: [u8; 3]) {
    let mut dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx: i32 = if x0 < x1 { 1 } else { -1 };
    let sy: i32 = if y0 < y1 { 1 } else { -1 };
    let mut err = if dx > dy { dx / 2 } else { -dy / 2 };
    let mut cx = x0;
    let mut cy = y0;
    loop {
        fb.set_pixel(cx, cy, c);
        if cx == x1 && cy == y1 { break; }
        let e2 = err;
        if e2 > -dx { err -= dy; cx += sx; }
        if e2 < dy { err += dx; cy += sy; }
    }
}

// ── Barycentric coordinates ───────────────────────────────────────────

fn barycentric(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32) -> (f32, f32, f32) {
    let det = (by - cy) * (ax - cx) + (cx - bx) * (ay - cy);
    if det.abs() < 1e-8 { return (-1.0, -1.0, -1.0); }
    let u = ((by - cy) * (px - cx) + (cx - bx) * (py - cy)) / det;
    let v = ((cy - ay) * (px - cx) + (ax - cx) * (py - cy)) / det;
    let w = 1.0 - u - v;
    (u, v, w)
}

// ── MVP transform pipeline ────────────────────────────────────────────

struct ScreenVertex {
    sx: f32,
    sy: f32,
    sz: f32,
    inv_w: f32,
    world_pos: Vec3,
    world_normal: Vec3,
    color: Vec3,
}

fn transform_vertex(v: &Vertex, mvp: &Mat4, viewport_w: f32, viewport_h: f32) -> Option<ScreenVertex> {
    let clip = mvp.transform(Vec4::from_vec3(v.pos, 1.0));
    if clip.w < 0.001 { return None; }
    let ndc = clip.perspective_divide();
    if ndc.z < -1.0 || ndc.z > 1.0 { return None; }
    let sx = (ndc.x + 1.0) * 0.5 * viewport_w;
    let sy = (1.0 - ndc.y) * 0.5 * viewport_h;
    let sz = ndc.z;
    Some(ScreenVertex {
        sx, sy, sz,
        inv_w: 1.0 / clip.w,
        world_pos: v.pos,
        world_normal: v.normal,
        color: v.color,
    })
}

// ── Lambert shading ───────────────────────────────────────────────────

fn lambert(normal: Vec3, lights: &[Light], base_color: Vec3) -> [u8; 3] {
    let mut r = 0.05f32;
    let mut g = 0.05f32;
    let mut b = 0.05f32;
    let n = normal.normalized();
    for light in lights {
        let ndotl = n.dot(light.dir.normalized()).max(0.0);
        r += base_color.x * light.color.x * ndotl;
        g += base_color.y * light.color.y * ndotl;
        b += base_color.z * light.color.z * ndotl;
    }
    [
        (r.min(1.0) * 255.0) as u8,
        (g.min(1.0) * 255.0) as u8,
        (b.min(1.0) * 255.0) as u8,
    ]
}

// ── Triangle rasterization with Z-buffer and barycentric interpolation ─

fn rasterize_triangle(fb: &mut Framebuffer, tri: &[ScreenVertex; 3], lights: &[Light]) {
    let min_x = tri[0].sx.min(tri[1].sx).min(tri[2].sx).floor() as i32;
    let min_y = tri[0].sy.min(tri[1].sy).min(tri[2].sy).floor() as i32;
    let max_x = tri[0].sx.max(tri[1].sx).max(tri[2].sx).ceil() as i32;
    let max_y = tri[0].sy.max(tri[1].sy).max(tri[2].sy).ceil() as i32;

    let min_x = min_x.max(0);
    let min_y = min_y.max(0);
    let max_x = max_x.min((fb.width - 1) as i32);
    let max_y = max_y.min((fb.height - 1) as i32);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let (u, v, w) = barycentric(
                px, py,
                tri[0].sx, tri[0].sy,
                tri[1].sx, tri[1].sy,
                tri[2].sx, tri[2].sy,
            );
            if u < 0.0 || v < 0.0 || w < 0.0 { continue; }

            let inv_denom = u * tri[0].inv_w + v * tri[1].inv_w + w * tri[2].inv_w;
            if inv_denom.abs() < 1e-10 { continue; }
            let denom = 1.0 / inv_denom;

            let z_ndc = (u * tri[0].sz * tri[0].inv_w + v * tri[1].sz * tri[1].inv_w + w * tri[2].sz * tri[2].inv_w) * denom;
            let depth = (z_ndc + 1.0) * 0.5;

            let idx = (y as usize) * fb.width + (x as usize);
            if depth < fb.depth[idx] {
                fb.depth[idx] = depth;

                let nx = (u * tri[0].world_normal.x * tri[0].inv_w
                        + v * tri[1].world_normal.x * tri[1].inv_w
                        + w * tri[2].world_normal.x * tri[2].inv_w) * denom;
                let ny = (u * tri[0].world_normal.y * tri[0].inv_w
                        + v * tri[1].world_normal.y * tri[1].inv_w
                        + w * tri[2].world_normal.y * tri[2].inv_w) * denom;
                let nz = (u * tri[0].world_normal.z * tri[0].inv_w
                        + v * tri[1].world_normal.z * tri[1].inv_w
                        + w * tri[2].world_normal.z * tri[2].inv_w) * denom;

                let cr = (u * tri[0].color.x * tri[0].inv_w
                        + v * tri[1].color.x * tri[1].inv_w
                        + w * tri[2].color.x * tri[2].inv_w) * denom;
                let cg = (u * tri[0].color.y * tri[0].inv_w
                        + v * tri[1].color.y * tri[1].inv_w
                        + w * tri[2].color.y * tri[2].inv_w) * denom;
                let cb = (u * tri[0].color.z * tri[0].inv_w
                        + v * tri[1].color.z * tri[1].inv_w
                        + w * tri[2].color.z * tri[2].inv_w) * denom;

                let interpolated_normal = Vec3::new(nx, ny, nz);
                let base_color = Vec3::new(cr, cg, cb);
                let color = lambert(interpolated_normal, lights, base_color);
                fb.color[idx] = color;
            }
        }
    }
}

// ── Scene construction helpers ────────────────────────────────────────

fn cube_triangles(offset: Vec3, scale: f32, color: Vec3) -> Vec<Triangle> {
    let s = scale;
    let faces: [[(Vec3, Vec3); 3]; 12] = [
        (Vec3::new(-s,s,s), Vec3::new(-s,-s,s), Vec3::new(s,s,s)),
        (Vec3::new(s,s,s), Vec3::new(-s,-s,s), Vec3::new(s,-s,s)),
        (Vec3::new(s,s,s), Vec3::new(s,-s,s), Vec3::new(s,s,-s)),
        (Vec3::new(s,s,-s), Vec3::new(s,-s,s), Vec3::new(s,-s,-s)),
        (Vec3::new(s,s,-s), Vec3::new(s,-s,-s), Vec3::new(-s,s,-s)),
        (Vec3::new(-s,s,-s), Vec3::new(s,-s,-s), Vec3::new(-s,-s,-s)),
        (Vec3::new(-s,s,-s), Vec3::new(-s,-s,-s), Vec3::new(-s,s,s)),
        (Vec3::new(-s,s,s), Vec3::new(-s,-s,-s), Vec3::new(-s,-s,s)),
        (Vec3::new(-s,s,s), Vec3::new(s,s,s), Vec3::new(-s,s,-s)),
        (Vec3::new(-s,s,-s), Vec3::new(s,s,s), Vec3::new(s,s,-s)),
        (Vec3::new(-s,-s,s), Vec3::new(-s,-s,-s), Vec3::new(s,-s,s)),
        (Vec3::new(s,-s,s), Vec3::new(-s,-s,-s), Vec3::new(s,-s,-s)),
    ];
    let normals = [
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
    ];
    faces.iter().zip(normals.iter()).map(|((a, b, c), n)| {
        let v0 = Vertex { pos: a.add(offset), normal: *n, color };
        let v1 = Vertex { pos: b.add(offset), normal: *n, color };
        let v2 = Vertex { pos: c.add(offset), normal: *n, color };
        Triangle { v: [v0, v1, v2] }
    }).collect()
}

fn sphere_triangles(center: Vec3, radius: f32, color: Vec3, rings: usize, sectors: usize) -> Vec<Triangle> {
    let mut tris = Vec::new();
    for i in 0..rings {
        let theta0 = std::f32::consts::PI * (i as f32) / (rings as f32) - std::f32::consts::FRAC_PI_2;
        let theta1 = std::f32::consts::PI * ((i + 1) as f32) / (rings as f32) - std::f32::consts::FRAC_PI_2;
        for j in 0..sectors {
            let phi0 = 2.0 * std::f32::consts::PI * (j as f32) / (sectors as f32);
            let phi1 = 2.0 * std::f32::consts::PI * ((j + 1) as f32) / (sectors as f32);

            let n00 = Vec3::new(theta0.cos()*phi0.cos(), theta0.sin(), theta0.cos()*phi0.sin());
            let n10 = Vec3::new(theta1.cos()*phi0.cos(), theta1.sin(), theta1.cos()*phi0.sin());
            let n01 = Vec3::new(theta0.cos()*phi1.cos(), theta0.sin(), theta0.cos()*phi1.sin());
            let n11 = Vec3::new(theta1.cos()*phi1.cos(), theta1.sin(), theta1.cos()*phi1.sin());

            let p00 = center.add(n00.scale(radius));
            let p10 = center.add(n10.scale(radius));
            let p01 = center.add(n01.scale(radius));
            let p11 = center.add(n11.scale(radius));

            tris.push(Triangle { v: [
                Vertex { pos: p00, normal: n00.normalized(), color },
                Vertex { pos: p10, normal: n10.normalized(), color },
                Vertex { pos: p11, normal: n11.normalized(), color },
            ]});
            tris.push(Triangle { v: [
                Vertex { pos: p00, normal: n00.normalized(), color },
                Vertex { pos: p11, normal: n11.normalized(), color },
                Vertex { pos: p01, normal: n01.normalized(), color },
            ]});
        }
    }
    tris
}

// ── Wireframe rendering ──────────────────────────────────────────────

fn cube_edges(offset: Vec3, scale: f32) -> Vec<(Vec3, Vec3)> {
    let s = scale;
    let corners = [
        Vec3::new(-s,s,s), Vec3::new(s,s,s), Vec3::new(s,s,-s), Vec3::new(-s,s,-s),
        Vec3::new(-s,-s,s), Vec3::new(s,-s,s), Vec3::new(s,-s,-s), Vec3::new(-s,-s,-s),
    ];
    let index_pairs: [(usize, usize); 12] = [
        (0,1),(1,2),(2,3),(3,0),
        (4,5),(5,6),(6,7),(7,4),
        (0,4),(1,5),(2,6),(3,7),
    ];
    index_pairs.iter().map(|(a, b)| (corners[*a].add(offset), corners[*b].add(offset))).collect()
}

fn project(v: Vec3, mvp: &Mat4, w: f32, h: f32) -> Option<(i32, i32)> {
    let clip = mvp.transform(Vec4::from_vec3(v, 1.0));
    if clip.w < 0.001 { return None; }
    let ndc = clip.perspective_divide();
    if ndc.z < -1.0 || ndc.z > 1.0 { return None; }
    Some(((ndc.x + 1.0) * 0.5 * w as f32) as i32,
         ((1.0 - ndc.y) * 0.5 * h as f32) as i32)
}

// ── PPM output ────────────────────────────────────────────────────────

fn save_ppm(fb: &Framebuffer, filename: &str) -> std::io::Result<()> {
    let mut f = File::create(filename)?;
    write!(f, "P6\n{} {}\n255\n", fb.width, fb.height)?;
    let bytes: Vec<u8> = fb.color.iter().flat_map(|c| [c[0], c[1], c[2]]).collect();
    f.write_all(&bytes)?;
    Ok(())
}

// ── Render functions ──────────────────────────────────────────────────

fn render_wireframe(fb: &mut Framebuffer, mvp: &Mat4, edges: &[(Vec3, Vec3)], color: [u8; 3]) {
    for (a, b) in edges {
        match (project(*a, mvp, fb.width as f32, fb.height as f32),
               project(*b, mvp, fb.width as f32, fb.height as f32)) {
            (Some(p0), Some(p1)) => draw_line(fb, p0.0, p0.1, p1.0, p1.1, color),
            _ => {}
        }
    }
}

fn render_scene(fb: &mut Framebuffer, scene: &Scene, mvp: &Mat4, lights: &[Light]) {
    for tri in &scene.triangles {
        let sv: [Option<ScreenVertex>; 3] = [
            transform_vertex(&tri.v[0], mvp, fb.width as f32, fb.height as f32),
            transform_vertex(&tri.v[1], mvp, fb.width as f32, fb.height as f32),
            transform_vertex(&tri.v[2], mvp, fb.width as f32, fb.height as f32),
        ];
        if sv[0].is_none() || sv[1].is_none() || sv[2].is_none() { continue; }
        let screen = [sv[0].unwrap(), sv[1].unwrap(), sv[2].unwrap()];
        rasterize_triangle(fb, &screen, lights);
    }
}

// ── Main ──────────────────────────────────────────────────────────────

fn main() {
    let w = 640;
    let h = 480;

    // ── Demo 1: Wireframe rotating cube ───────────────────────────────
    {
        let mut fb = Framebuffer::new(w, h);
        fb.clear([20, 20, 40]);
        let proj = make_perspective(std::f32::consts::FRAC_PI_3, w as f32 / h as f32, 0.1, 100.0);
        let view = make_view_matrix(Vec3::new(3.0, 2.0, 5.0), Vec3::zero(), Vec3::new(0.0, 1.0, 0.0));
        let model = make_model_matrix(0.6, Vec3::zero());
        let mvp = proj.mul(view).mul(model);
        let edges = cube_edges(Vec3::zero(), 1.0);
        render_wireframe(&mut fb, &mvp, &edges, [0, 255, 180]);
        let proj2 = make_perspective(std::f32::consts::FRAC_PI_3, w as f32 / h as f32, 0.1, 100.0);
        let view2 = make_view_matrix(Vec3::new(3.0, 2.0, 5.0), Vec3::zero(), Vec3::new(0.0, 1.0, 0.0));
        let model2 = make_model_matrix(0.6 + 0.3, Vec3::zero());
        let mvp2 = proj2.mul(view2).mul(model2);
        let edges2 = cube_edges(Vec3::zero(), 1.0);
        render_wireframe(&mut fb, &mvp2, &edges2, [255, 100, 50]);
        save_ppm(&fb, "wireframe_cube.ppm").expect("Failed to write wireframe_cube.ppm");
        println!("Wrote wireframe_cube.ppm");
    }

    // ── Demo 2: Flat-shaded cube with Z-buffer ────────────────────────
    {
        let mut fb = Framebuffer::new(w, h);
        fb.clear([20, 20, 40]);
        let proj = make_perspective(std::f32::consts::FRAC_PI_3, w as f32 / h as f32, 0.1, 100.0);
        let view = make_view_matrix(Vec3::new(3.0, 3.0, 5.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
        let model = make_model_matrix(0.4, Vec3::zero());
        let mvp = proj.mul(view).mul(model);
        let lights = [
            Light { dir: Vec3::new(0.5, 1.0, 0.5), color: Vec3::new(1.0, 1.0, 1.0) },
        ];
        let cube_tris = cube_triangles(Vec3::zero(), 1.2, Vec3::new(0.8, 0.3, 0.2));
        let scene = Scene { triangles: cube_tris };
        render_scene(&mut fb, &scene, &mvp, &lights);
        save_ppm(&fb, "flat_cube.ppm").expect("Failed to write flat_cube.ppm");
        println!("Wrote flat_cube.ppm");
    }

    // ── Demo 3: Lambert-shaded scene (cube + two spheres) ──────────────
    {
        let mut fb = Framebuffer::new(w, h);
        fb.clear([20, 20, 40]);
        let proj = make_perspective(std::f32::consts::FRAC_PI_3, w as f32 / h as f32, 0.1, 100.0);
        let view = make_view_matrix(Vec3::new(0.0, 4.0, 8.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
        let mvp_ident = proj.mul(view);

        let mut all_tris: Vec<Triangle> = Vec::new();
        all_tris.extend(cube_triangles(Vec3::new(-2.5, 0.5, 0.0), 1.0, Vec3::new(0.2, 0.6, 0.9)));
        all_tris.extend(sphere_triangles(Vec3::new(1.0, 0.5, 0.0), 1.2, Vec3::new(0.9, 0.3, 0.2), 12, 16));
        all_tris.extend(sphere_triangles(Vec3::new(0.0, 1.8, -1.5), 0.7, Vec3::new(0.3, 0.9, 0.4), 10, 14));

        let ground_y = -0.5f32;
        let gs = 4.0f32;
        let gn = Vec3::new(0.0, 1.0, 0.0);
        let gc = Vec3::new(0.5, 0.5, 0.5);
        let ground_tris: Vec<Triangle> = [
            (Vec3::new(-gs, ground_y, -gs), Vec3::new(-gs, ground_y, gs), Vec3::new(gs, ground_y, gs)),
            (Vec3::new(-gs, ground_y, -gs), Vec3::new(gs, ground_y, gs), Vec3::new(gs, ground_y, -gs)),
        ].iter().map(|&(a, b, c)| {
            Triangle { v: [
                Vertex { pos: a, normal: gn, color: gc },
                Vertex { pos: b, normal: gn, color: gc },
                Vertex { pos: c, normal: gn, color: gc },
            ]}
        }).collect();
        all_tris.extend(ground_tris);

        let scene = Scene { triangles: all_tris };
        let lights = [
            Light { dir: Vec3::new(0.5, 1.0, 0.3), color: Vec3::new(1.0, 0.95, 0.9) },
            Light { dir: Vec3::new(-0.6, 0.3, -0.5), color: Vec3::new(0.3, 0.3, 0.5) },
        ];
        render_scene(&mut fb, &scene, &mvp_ident, &lights);
        save_ppm(&fb, "lambert_scene.ppm").expect("Failed to write lambert_scene.ppm");
        println!("Wrote lambert_scene.ppm");
    }

    println!("\nAll renders complete. View the .ppm files in an image viewer.");
}