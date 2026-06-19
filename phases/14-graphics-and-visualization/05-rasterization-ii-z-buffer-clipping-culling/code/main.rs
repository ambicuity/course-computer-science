use std::fs::File;
use std::io::Write;

#[derive(Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Add for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f32) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

#[derive(Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

struct Triangle {
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    color: Color,
    two_sided: bool,
}

fn signed_area_2d(a: &Vec3, b: &Vec3, c: &Vec3) -> f32 {
    0.5 * ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y))
}

fn rasterize_triangle(
    tri: &Triangle,
    fb: &mut [Color],
    zb: &mut [f32],
    w: usize,
    h: usize,
    enable_backface: bool,
) {
    let sa = signed_area_2d(&tri.v0, &tri.v1, &tri.v2);

    if enable_backface && !tri.two_sided && sa < 0.0 {
        return;
    }
    if sa.abs() < 1e-6 {
        return;
    }

    let min_x = (tri.v0.x.min(tri.v1.x).min(tri.v2.x).floor() as usize)
        .max(0);
    let max_x = (tri.v0.x.max(tri.v1.x).max(tri.v2.x).ceil() as usize)
        .min(w - 1);
    let min_y = (tri.v0.y.min(tri.v1.y).min(tri.v2.y).floor() as usize)
        .max(0);
    let max_y = (tri.v0.y.max(tri.v1.y).max(tri.v2.y).ceil() as usize)
        .min(h - 1);

    let inv_area = 1.0 / sa;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;

            let u0 = (tri.v1.x - px) * (tri.v2.y - py)
                - (tri.v2.x - px) * (tri.v1.y - py);
            let u1 = (tri.v2.x - px) * (tri.v0.y - py)
                - (tri.v0.x - px) * (tri.v2.y - py);
            let u2 = (tri.v0.x - px) * (tri.v1.y - py)
                - (tri.v1.x - px) * (tri.v0.y - py);

            let w0 = u0 * inv_area;
            let w1 = u1 * inv_area;
            let w2 = u2 * inv_area;

            if w0 < -0.001 || w1 < -0.001 || w2 < -0.001 {
                continue;
            }

            let depth = w0 * tri.v0.z + w1 * tri.v1.z + w2 * tri.v2.z;

            let idx = y * w + x;
            if depth < zb[idx] {
                zb[idx] = depth;
                fb[idx] = tri.color;
            }
        }
    }
}

fn rasterize_triangle_invz(
    tri: &Triangle,
    fb: &mut [Color],
    zb: &mut [f32],
    w: usize,
    h: usize,
) {
    let sa = signed_area_2d(&tri.v0, &tri.v1, &tri.v2);
    if sa.abs() < 1e-6 {
        return;
    }

    let min_x = (tri.v0.x.min(tri.v1.x).min(tri.v2.x).floor() as usize)
        .max(0);
    let max_x = (tri.v0.x.max(tri.v1.x).max(tri.v2.x).ceil() as usize)
        .min(w - 1);
    let min_y = (tri.v0.y.min(tri.v1.y).min(tri.v2.y).floor() as usize)
        .max(0);
    let max_y = (tri.v0.y.max(tri.v1.y).max(tri.v2.y).ceil() as usize)
        .min(h - 1);

    let inv_area = 1.0 / sa;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;

            let bw0 = (tri.v1.x - px) * (tri.v2.y - py)
                - (tri.v2.x - px) * (tri.v1.y - py);
            let bw1 = (tri.v2.x - px) * (tri.v0.y - py)
                - (tri.v0.x - px) * (tri.v2.y - py);
            let bw2 = (tri.v0.x - px) * (tri.v1.y - py)
                - (tri.v1.x - px) * (tri.v0.y - py);

            let w0 = bw0 * inv_area;
            let w1 = bw1 * inv_area;
            let w2 = bw2 * inv_area;

            if w0 < -0.001 || w1 < -0.001 || w2 < -0.001 {
                continue;
            }

            let inv_z_interp =
                w0 / tri.v0.z + w1 / tri.v1.z + w2 / tri.v2.z;
            let depth = 1.0 / inv_z_interp;

            let idx = y * w + x;
            if depth < zb[idx] {
                zb[idx] = depth;
                fb[idx] = tri.color;
            }
        }
    }
}

fn clip_edge_near(s: Vec3, e: Vec3, near_z: f32) -> Vec3 {
    let d_s = near_z - s.z;
    let d_e = near_z - e.z;
    let t = d_s / (d_s - d_e);
    Vec3::new(
        s.x + t * (e.x - s.x),
        s.y + t * (e.y - s.y),
        near_z,
    )
}

fn clip_triangle_near(tri: &Triangle, near_z: f32) -> Vec<Triangle> {
    let v0_in = tri.v0.z >= near_z;
    let v1_in = tri.v1.z >= near_z;
    let v2_in = tri.v2.z >= near_z;
    let inside = v0_in as u32 + v1_in as u32 + v2_in as u32;

    match inside {
        3 => vec![tri.clone()],
        0 => vec![],
        _ => {
            let verts_in = [v0_in, v1_in, v2_in];
            let vs = [tri.v0, tri.v1, tri.v2];
            let mut result = Vec::new();

            if inside == 1 {
                let i_in = verts_in.iter().position(|&v| v).unwrap();
                let i_out1 = (i_in + 1) % 3;
                let i_out2 = (i_in + 2) % 3;
                let a = clip_edge_near(vs[i_in], vs[i_out1], near_z);
                let b = clip_edge_near(vs[i_in], vs[i_out2], near_z);
                result.push(Triangle {
                    v0: vs[i_in],
                    v1: a,
                    v2: b,
                    color: tri.color,
                    two_sided: tri.two_sided,
                });
            } else {
                let i_out = verts_in.iter().position(|&v| !v).unwrap();
                let i_in1 = (i_out + 1) % 3;
                let i_in2 = (i_out + 2) % 3;
                let a = clip_edge_near(vs[i_in1], vs[i_out], near_z);
                let b = clip_edge_near(vs[i_in2], vs[i_out], near_z);
                result.push(Triangle {
                    v0: vs[i_in1],
                    v1: vs[i_in2],
                    v2: a,
                    color: tri.color,
                    two_sided: tri.two_sided,
                });
                result.push(Triangle {
                    v0: vs[i_in2],
                    v1: b,
                    v2: a,
                    color: tri.color,
                    two_sided: tri.two_sided,
                });
            }
            result
        }
    }
}

fn write_ppm(filename: &str, fb: &[Color], w: usize, h: usize) -> std::io::Result<()> {
    let mut f = File::create(filename)?;
    write!(f, "P6\n{} {}\n255\n", w, h)?;
    for row in (0..h).rev() {
        for col in 0..w {
            let c = fb[row * w + col];
            f.write_all(&[c.r, c.g, c.b])?;
        }
    }
    println!("Wrote {}", filename);
    Ok(())
}

fn render_scene(
    tris: &[Triangle],
    filename: &str,
    w: usize,
    h: usize,
    backface: bool,
    use_invz: bool,
) {
    let mut fb = vec![Color::new(30, 30, 30); w * h];
    let mut zb = vec![f32::MAX; w * h];

    for tri in tris {
        let clipped = clip_triangle_near(tri, 0.1);
        for ct in &clipped {
            if use_invz {
                rasterize_triangle_invz(ct, &mut fb, &mut zb, w, h);
            } else {
                rasterize_triangle(ct, &mut fb, &mut zb, w, h, backface);
            }
        }
    }
    write_ppm(filename, &fb, w, h).expect("Failed to write PPM");
}

fn main() {
    const W: usize = 400;
    const H: usize = 400;

    // Scene 1: Two overlapping triangles with correct depth ordering
    let scene1 = vec![
        Triangle {
            v0: Vec3::new(100.0, 50.0, 0.5),
            v1: Vec3::new(350.0, 50.0, 0.4),
            v2: Vec3::new(200.0, 300.0, 0.6),
            color: Color::new(220, 50, 50),
            two_sided: false,
        },
        Triangle {
            v0: Vec3::new(150.0, 100.0, 0.3),
            v1: Vec3::new(350.0, 200.0, 0.35),
            v2: Vec3::new(100.0, 350.0, 0.45),
            color: Color::new(50, 180, 50),
            two_sided: false,
        },
    ];
    render_scene(&scene1, "zbuffer_overlapping.ppm", W, H, true, false);

    // Scene 2: Z-fighting — two nearly co-planar triangles
    let scene2 = vec![
        Triangle {
            v0: Vec3::new(80.0, 80.0, 0.500),
            v1: Vec3::new(320.0, 80.0, 0.500),
            v2: Vec3::new(200.0, 320.0, 0.500),
            color: Color::new(200, 60, 60),
            two_sided: false,
        },
        Triangle {
            v0: Vec3::new(100.0, 120.0, 0.5001),
            v1: Vec3::new(300.0, 120.0, 0.5001),
            v2: Vec3::new(200.0, 300.0, 0.5001),
            color: Color::new(60, 60, 200),
            two_sided: false,
        },
    ];
    render_scene(&scene2, "zbuffer_zfighting.ppm", W, H, true, false);

    // Scene 3: Backface culling demo
    let scene3 = vec![
        // CCW (front-facing) — rendered
        Triangle {
            v0: Vec3::new(50.0, 50.0, 0.5),
            v1: Vec3::new(200.0, 50.0, 0.5),
            v2: Vec3::new(125.0, 200.0, 0.5),
            color: Color::new(200, 50, 50),
            two_sided: false,
        },
        // CW (back-facing) — culled
        Triangle {
            v0: Vec3::new(250.0, 50.0, 0.4),
            v1: Vec3::new(250.0, 200.0, 0.4),
            v2: Vec3::new(400.0, 200.0, 0.4),
            color: Color::new(50, 200, 50),
            two_sided: false,
        },
        // CW but two_sided — rendered regardless
        Triangle {
            v0: Vec3::new(250.0, 50.0, 0.3),
            v1: Vec3::new(400.0, 200.0, 0.3),
            v2: Vec3::new(250.0, 200.0, 0.3),
            color: Color::new(50, 50, 200),
            two_sided: true,
        },
    ];
    render_scene(&scene3, "zbuffer_backface.ppm", W, H, true, false);

    // Scene 4: 1/z interpolation demo
    let scene4 = vec![
        Triangle {
            v0: Vec3::new(100.0, 50.0, 5.0),
            v1: Vec3::new(350.0, 50.0, 50.0),
            v2: Vec3::new(200.0, 300.0, 5.0),
            color: Color::new(220, 50, 50),
            two_sided: false,
        },
        Triangle {
            v0: Vec3::new(150.0, 150.0, 3.0),
            v1: Vec3::new(350.0, 200.0, 30.0),
            v2: Vec3::new(100.0, 350.0, 10.0),
            color: Color::new(50, 180, 50),
            two_sided: false,
        },
    ];
    render_scene(&scene4, "zbuffer_invz.ppm", W, H, false, true);

    println!("Z-buffer rasterizer complete. Check .ppm output files.");
}