# Software Rasterizer Module

A complete, self-contained software rasterizer in Rust. No external dependencies — compiles with `rustc main.rs`.

## What It Does

This rasterizer implements the full graphics pipeline:

1. **Vertex processing** — Model → World → View → Projection → Perspective Divide → Viewport
2. **Primitive assembly** — Triangles with per-vertex positions, normals, and colors
3. **Rasterization** — Barycentric coordinate interpolation with perspective-correct (1/z) depth
4. **Fragment shading** — Lambert diffuse lighting with multiple directional lights and ambient
5. **Z-buffer** — Per-pixel depth testing with `f32` precision
6. **Output** — PPM images (P6 binary format)

## How to Use

```bash
# Compile and run
rustc main.rs -o rasterizer && ./rasterizer

# This produces three images:
#   wireframe_cube.ppm   — Rotating wireframe cube (perspective)
#   flat_cube.ppm        — Flat-shaded cube with Z-buffer
#   lambert_scene.ppm    — Lambert-shaded multi-object scene (cube + 2 spheres + ground)
```

View PPM files with any image viewer (ImageMagick, macOS Preview, GIMP, etc.) or convert:
```bash
convert lambert_scene.ppm lambert_scene.png
```

## Core API

```rust
// Framebuffer
let mut fb = Framebuffer::new(640, 480);
fb.clear([20, 20, 40]);                     // background color

// Camera setup
let proj = make_perspective(fov_rad, aspect, near, far);
let view = make_view_matrix(eye, center, up);
let mvp = proj.mul(view);                   // or with model: proj.mul(view).mul(model)

// Scene construction
let triangles = cube_triangles(offset, scale, color);
let triangles = sphere_triangles(center, radius, color, rings, sectors);
let scene = Scene { triangles: all_tris };

// Rendering
let lights = [Light { dir: Vec3::new(0.5, 1.0, 0.3), color: Vec3::new(1.0, 1.0, 1.0) }];
render_scene(&mut fb, &scene, &mvp, &lights);

// Output
save_ppm(&fb, "output.ppm");
```

## Key Algorithms

| Algorithm | Implementation | File Location |
|-----------|---------------|---------------|
| Bresenham line drawing | `draw_line()` | Step 1: wireframe |
| Barycentric coordinates | `barycentric()` | Step 2: triangle fill |
| Perspective-correct interpolation | 1/z weighting in `rasterize_triangle()` | Step 2: Z-buffer |
| Lambert diffuse shading | `lambert()` | Step 3: shading |
| Z-buffer depth test | `Framebuffer.depth` per-pixel | Step 2–3: visibility |

## Phase Capstone (Lesson 18)

This rasterizer reappears in the Phase Capstone, where it will be extended with:
- Texture mapping (UV coordinates + texture sampling)
- Specular highlights (Blinn-Phong)
- Combination with the path tracer for hybrid rendering

The module is designed to be importable — all core types (`Vec3`, `Mat4`, `Vertex`, `Triangle`, `Framebuffer`) and functions are public-ready and can be pasted into the capstone project.

## Limitations

- No near-plane clipping (vertices behind the camera cause artifacts)
- No backface culling (all triangles are rasterized regardless of winding)
- No multithreading or SIMD (purely sequential per-pixel processing)
- No texture mapping (vertex colors only)
- PPM output only (no PNG/JPEG without conversion)

These limitations are intentional — the rasterizer is designed for education and debugging, not production performance. The Capstone will address some of them.