# Dual-Renderer Graphics Framework

This artifact is a self-contained Rust program that implements **two complete rendering pipelines** against a shared scene and math foundation:

- **Triangle Rasterizer** — MVP transform, barycentric interpolation, z-buffer, Lambert shading with shadow rays. Produces a deterministic, noise-free image in milliseconds. No global illumination.

- **Monte Carlo Path Tracer** — Ray-scene intersection, cosine hemisphere sampling, Russian roulette termination, direct + indirect lighting. Produces physically correct global illumination (soft shadows, color bleeding, indirect light) with noise that decreases as sample count increases.

Both renderers consume the same `Scene` (spheres, triangles, materials, lights) and `Camera` (eye, target, up, FOV), demonstrating that the mathematical description of "what to render" is renderer-agnostic.

## Build and Run

```bash
rustc -O main.rs -o dual_renderer
./dual_renderer
```

Produces two PPM images: `rasterizer.ppm` and `pathtracer.ppm`.

## Output Format

PPM (Portable Pixmap) — viewable in any image viewer or convertible via ImageMagick:

```bash
convert rasterizer.ppm rasterizer.png
convert pathtracer.ppm pathtracer.png
```

## Architecture

```
Shared Layer
├── Vec3, Vec4, Mat4, Ray          — Linear algebra primitives
├── Material, Sphere, Triangle     — Scene description
├── Light, Scene, Camera           — View and illumination
└── Framebuffer, save_ppm          — Output

Rasterizer
├── rasterize()                    — Per-triangle MVP transform, barycentric test, z-buffer, Lambert shading
└── Shadow rays for direct lighting

Path Tracer
├── trace()                        — Recursive Monte Carlo integration
├── path_trace()                   — Per-pixel multi-sample driver
└── cosine_hemisphere() + local_to_world() — Importance sampling
```

## Extending the Framework

| Extension | What to change | Who benefits |
|-----------|---------------|-------------|
| BVH acceleration | `Scene::intersect()` | Both renderers |
| Texture mapping | `Material` struct | Both renderers |
| Blinn-Phong / PBR shading | Fragment shader in `rasterize()`, BRDF sampling in `trace()` | Both renderers |
| Importance sampling | Sampling strategy in `trace()` | Path tracer |
| GPU dispatch | Per-pixel loop in `path_trace()`, per-triangle loop in `rasterize()` | Both renderers |
| Anti-aliasing | Jittered sampling, MSAA | Both renderers |

## Connection to Subsequent Phases

- **Phase 15 (Systems Performance):** The path tracer is an ideal benchmark for parallelization. Each pixel's path is independent — trivially parallelizable across CPU cores (via Rayon) or GPU Compute (via wgpu). Memory layout optimization (SoA vs AoS), SIMD for Vec3 operations, and cache-friendly BVH traversal are all applicable.

- **Phase 16 (Software Architecture):** The dual-renderer is a case study in abstraction boundaries. The shared `Scene`/`Camera`/`Math` layer versus the renderer-specific pipelines demonstrates the Strategy pattern, interface segregation, and the cost/benefit of adding abstraction layers. When should the `Material` struct grow a `texture_handle` field? When should `Scene::intersect()` become a trait object? These are architecture decisions.