// Rasterization I — Lines and Triangles
// Phase 14 — Computer Graphics & Visualization
//
// Renders a PPM image (output.ppm) demonstrating:
//   1. Bresenham line drawing (several lines of varying slope)
//   2. A filled triangle with barycentric color interpolation
//   3. Two overlapping triangles showing rasterization rules

use std::fs::File;
use std::io::Write;

#[derive(Clone, Copy)]
struct Vec2 {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    const fn new(r: u8, g: u8, b: u8) -> Self { Self { r, g, b } }
}

const BLACK: Color = Color::new(0, 0, 0);
const WHITE: Color = Color::new(255, 255, 255);
const RED: Color = Color::new(255, 0, 0);
const GREEN: Color = Color::new(0, 255, 0);
const BLUE: Color = Color::new(0, 0, 255);

struct Image {
    w: usize,
    h: usize,
    pixels: Vec<Color>,
}

impl Image {
    fn new(w: usize, h: usize) -> Self {
        Self { w, h, pixels: vec![BLACK; w * h] }
    }

    fn set(&mut self, x: i32, y: i32, c: Color) {
        if x >= 0 && (x as usize) < self.w && y >= 0 && (y as usize) < self.h {
            self.pixels[(y as usize) * self.w + (x as usize)] = c;
        }
    }

    fn write_ppm(&self, filename: &str) -> std::io::Result<()> {
        let mut f = File::create(filename)?;
        write!(f, "P6\n{} {}\n255\n", self.w, self.h)?;
        let bytes: Vec<u8> = self.pixels.iter().flat_map(|c| [c.r, c.g, c.b]).collect();
        f.write_all(&bytes)?;
        Ok(())
    }
}

fn edge_function(a: Vec2, b: Vec2, p: Vec2) -> f32 {
    (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
}

fn bresenham(x0: i32, y0: i32, x1: i32, y1: i32, img: &mut Image, c: Color) {
    let (mut x0, mut y0, mut x1, mut y1) = (x0, y0, x1, y1);
    let steep = (y1 - y0).abs() > (x1 - x0).abs();
    if steep {
        std::mem::swap(&mut x0, &mut y0);
        std::mem::swap(&mut x1, &mut y1);
    }
    if x0 > x1 {
        std::mem::swap(&mut x0, &mut x1);
        std::mem::swap(&mut y0, &mut y1);
    }
    let dx = x1 - x0;
    let dy = (y1 - y0).abs();
    let mut err = dx / 2;
    let ystep: i32 = if y0 < y1 { 1 } else { -1 };
    let mut y = y0;
    for x in x0..=x1 {
        if steep {
            img.set(y, x, c);
        } else {
            img.set(x, y, c);
        }
        err -= dy;
        if err < 0 {
            y += ystep;
            err += dx;
        }
    }
}

fn rasterize_triangle(
    v0: Vec2, v1: Vec2, v2: Vec2,
    c0: Color, c1: Color, c2: Color,
    img: &mut Image,
) {
    let min_x = v0.x.min(v1.x).min(v2.x).floor() as i32;
    let max_x = v0.x.max(v1.x).max(v2.x).ceil() as i32;
    let min_y = v0.y.min(v1.y).min(v2.y).floor() as i32;
    let max_y = v0.y.max(v1.y).max(v2.y).ceil() as i32;

    let min_x = min_x.max(0);
    let max_x = max_x.min((img.w - 1) as i32);
    let min_y = min_y.max(0);
    let max_y = max_y.min((img.h - 1) as i32);

    let area = edge_function(v0, v1, v2);
    if area.abs() < 1e-6 { return; }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = Vec2 { x: x as f32 + 0.5, y: y as f32 + 0.5 };
            let w0 = edge_function(p, v1, v2);
            let w1 = edge_function(p, v2, v0);
            let w2 = edge_function(p, v0, v1);
            let inside = if area > 0.0 {
                w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0
            } else {
                w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0
            };
            if inside {
                let inv = 1.0 / area;
                let w0 = w0 * inv;
                let w1 = w1 * inv;
                let w2 = w2 * inv;
                let r = (w0 * c0.r as f32 + w1 * c1.r as f32 + w2 * c2.r as f32).min(255.0) as u8;
                let g = (w0 * c0.g as f32 + w1 * c1.g as f32 + w2 * c2.g as f32).min(255.0) as u8;
                let b = (w0 * c0.b as f32 + w1 * c1.b as f32 + w2 * c2.b as f32).min(255.0) as u8;
                img.set(x, y, Color::new(r, g, b));
            }
        }
    }
}

fn rasterize_triangle_flat(v0: Vec2, v1: Vec2, v2: Vec2, c: Color, img: &mut Image) {
    rasterize_triangle(v0, v1, v2, c, c, c, img);
}

fn draw_bresenham_demo(img: &mut Image) {
    let cx = (img.w / 2) as i32;
    let cy = 90;
    let len = 60;
    bresenham(cx, cy, cx + len, cy, img, WHITE);
    bresenham(cx, cy, cx + len, cy + len / 3, img, WHITE);
    bresenham(cx, cy, cx + len / 3, cy + len, img, WHITE);
    bresenham(cx, cy, cx, cy + len, img, WHITE);
    bresenham(cx, cy, cx - len / 3, cy + len, img, WHITE);
    bresenham(cx, cy, cx - len, cy + len / 3, img, WHITE);
    bresenham(cx, cy, cx - len, cy, img, WHITE);
    bresenham(cx, cy, cx - len, cy - len / 3, img, WHITE);
    bresenham(cx, cy, cx - len / 3, cy - len, img, WHITE);
    bresenham(cx, cy, cx, cy - len, img, WHITE);
    bresenham(cx, cy, cx + len / 3, cy - len, img, WHITE);
    bresenham(cx, cy, cx + len, cy - len / 3, img, WHITE);
}

fn draw_interpolated_triangle(img: &mut Image, off_x: i32, off_y: i32) {
    let v0 = Vec2 { x: off_x as f32, y: off_y as f32 };
    let v1 = Vec2 { x: (off_x + 120) as f32, y: off_y as f32 };
    let v2 = Vec2 { x: (off_x + 60) as f32, y: (off_y + 100) as f32 };
    rasterize_triangle(v0, v1, v2, RED, GREEN, BLUE, img);
    bresenham(v0.x as i32, v0.y as i32, v1.x as i32, v1.y as i32, img, WHITE);
    bresenham(v1.x as i32, v1.y as i32, v2.x as i32, v2.y as i32, img, WHITE);
    bresenham(v2.x as i32, v2.y as i32, v0.x as i32, v0.y as i32, img, WHITE);
}

fn draw_overlapping_triangles(img: &mut Image, off_x: i32, off_y: i32) {
    let s = 50;
    let a0 = Vec2 { x: off_x as f32, y: off_y as f32 };
    let a1 = Vec2 { x: (off_x + 2 * s) as f32, y: off_y as f32 };
    let a2 = Vec2 { x: (off_x + s) as f32, y: (off_y + 2 * s) as f32 };
    let b0 = Vec2 { x: (off_x + s) as f32, y: off_y as f32 };
    let b1 = Vec2 { x: (off_x + 3 * s) as f32, y: off_y as f32 };
    let b2 = Vec2 { x: (off_x + 2 * s) as f32, y: (off_y + 2 * s) as f32 };
    let ca = Color::new(200, 50, 50);
    let cb = Color::new(50, 50, 200);
    rasterize_triangle_flat(a0, a1, a2, ca, img);
    rasterize_triangle_flat(b0, b1, b2, cb, img);
    bresenham(a0.x as i32, a0.y as i32, a1.x as i32, a1.y as i32, img, WHITE);
    bresenham(a1.x as i32, a1.y as i32, a2.x as i32, a2.y as i32, img, WHITE);
    bresenham(a2.x as i32, a2.y as i32, a0.x as i32, a0.y as i32, img, WHITE);
    bresenham(b0.x as i32, b0.y as i32, b1.x as i32, b1.y as i32, img, WHITE);
    bresenham(b1.x as i32, b1.y as i32, b2.x as i32, b2.y as i32, img, WHITE);
    bresenham(b2.x as i32, b2.y as i32, b0.x as i32, b0.y as i32, img, WHITE);
}

fn main() {
    let (w, h) = (256, 400);
    let mut img = Image::new(w, h);

    for y in 180..185 {
        for x in 0..w {
            img.set(x as i32, y as i32, Color::new(40, 40, 40));
        }
    }

    draw_bresenham_demo(&mut img);
    draw_interpolated_triangle(&mut img, 50, 200);
    draw_overlapping_triangles(&mut img, 140, 200);

    match img.write_ppm("output.ppm") {
        Ok(()) => println!("Wrote output.ppm ({}x{})", w, h),
        Err(e) => eprintln!("Failed to write output.ppm: {}", e),
    }
}