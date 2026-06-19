# Build a Software Rasterizer (in Rust)

> From Bresenham to a complete triangle rasterizer — one pipeline, zero shortcuts.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 14 lessons 01–08
**Time:** ~120 minutes

## Learning Objectives

- Wire the full vertex processing pipeline: Model → World → View → Clip → NDC → Screen.
- Implement triangle rasterization with barycentric coordinate interpolation.
- Build a Z-buffer with perspective-correct (1/z) depth interpolation.
- Apply Lambert diffuse shading to per-fragment interpolated normals.
- Render three scenes: wireframe cube, flat-shaded cube, and a multi-object Lambert scene.
- Explain why GPUs dominate this workload (data parallelism, cache coherence, fixed-function units) and where software rasterizers still matter.

## The Problem

Lessons 01–08 gave you the pieces: pixels and gamma, transforms and projections, line drawing and barycentric coordinates, Z-buffers and clipping, shading models, and shaders. But the pieces don't *do* anything until you connect them into a pipeline. A vertex sitting in model space doesn't become a colored pixel on screen by magic — it passes through a chain of transformations, gets assembled into primitives, gets rasterized into fragments, and only then is shaded and depth-tested.

This lesson is where you build that chain. The result is a complete software rasterizer — a program that takes a scene description (triangles with positions, normals, and colors) and produces a rendered image. No GPU, no graphics API, just math and pixel writing.

At the end, you'll understand what every step of the pipeline does, *why* it does it, and where the GPU replaces your loops with data-parallel hardware.

## The Concept: The Full Pipeline

### From Vertex to Pixel — The Chain

Every 3D graphics system, from your Rust code to a $2000 GPU, runs the same sequence:

```
  Model space           World space           View space
  ┌──────────┐  Model  ┌──────────┐   View  ┌──────────┐
  │  Vertex   │───────▶│  Vertex   │───────▶│  Vertex   │
  │  (local)  │  mat   │ (global)  │  mat   │ (camera)  │
  └──────────┘         └──────────┘         └──────────┘
                                               │
                                        Projection mat
                                               ▼
  ┌──────────┐  Viewport  ┌──────────┐  Rasterize ┌──────────┐
  │   NDC     │──────────▶│  Screen   │─────────▶│ Fragments │
  │ (-1..1)   │  transform│ (pixels)  │           │ (pixels)  │
  └──────────┘            └──────────┘            └──────────┘
                                                       │
                                                  Z-test & Shade
                                                       ▼
                                                 ┌──────────┐
                                                 │  Pixel    │
                                                 │  (color)  │
                                                 └──────────┘
```

Each stage has a specific job:

1. **Model transform** places the object in the world. A cube centered at its own origin gets translated to (−2.5, 0.5, 0.0) in world space.
2. **View transform** moves the camera to the origin, looking down −Z. The `make_view_matrix` function builds this from an eye position, a look-at target, and an up vector.
3. **Projection transform** maps the view frustum to a cube (NDC). Near things get bigger, far things get smaller — that's perspective. The `make_perspective` function builds this from FOV, aspect ratio, and near/far planes.
4. **Viewport transform** maps NDC (−1..1) to pixel coordinates. Screen Y is flipped (0 at top, height at bottom).
5. **Rasterization** turns a triangle into a set of fragments — one per pixel inside the triangle. Barycentric coordinates tell us *where* in the triangle each fragment falls, so we can interpolate depth, color, and normals.
6. **Z-buffer test** resolves which fragment wins when multiple triangles cover the same pixel. The closer one overwrites the farther one.
7. **Shading** uses the interpolated normal and material color to compute the final pixel color (Lambert diffuse in our case).

### Why 1/z Instead of z?

This is worth emphasizing because it's the most common "gotcha" in a software rasterizer.

After the perspective divide, attributes like texture coordinates, normals, and colors do *not* vary linearly in screen space. They vary linearly in *clip space*, and the perspective divide is a nonlinear mapping. The correct interpolation scheme is:

```
  attribute_screen = (u * a0/w0 + v * a1/w1 + w * a2/w2) / (u/w0 + v/w1 + w/w2)
```

where u, v, w are barycentric weights in screen space, w0/w1/w2 are the clip-space W values (reciprocal: 1/w = inv_w), and a0/a1/a2 are the vertex attributes.

This is equivalent to saying: interpolate `a/w` linearly in screen space, then divide by the interpolated `1/w`. The `1/z` term is just the special case where the attribute being interpolated *is* depth.

If you interpolate raw depth linearly in screen space, triangles that should be smooth will show incorrect depth values, especially at wide view angles. The `rasterize_triangle` function in our code uses `inv_w` on every `ScreenVertex` precisely for this reason.

### Barycentric Coordinates Refresher

Given triangle vertices A, B, C and a point P, the barycentric coordinates (u, v, w) are:

```
  u = area(PBC) / area(ABC)
  v = area(APC) / area(ABC)
  w = 1 - u - v
```

P is inside the triangle if and only if u ≥ 0, v ≥ 0, and w ≥ 0. These weights let us interpolate *any* vertex attribute across the triangle's surface.

In code, we compute them via the signed area formula (equivalent to the edge function from Lesson 04):

```
  det = (By - Cy)(Ax - Cx) + (Cx - Bx)(Ay - Cy)
  u   = ((By - Cy)(Px - Cx) + (Cx - Bx)(Py - Cy)) / det
  v   = ((Cy - Ay)(Px - Cx) + (Ax - Cx)(Py - Cy)) / det
  w   = 1 - u - v
```

### The Z-buffer in Action

When two triangles overlap at pixel (x, y), the Z-buffer resolves visibility:

```
  Initialize: depth[x][y] = +∞  for all pixels

  For each triangle:
    For each pixel inside the triangle:
      Compute perspective-correct depth at this pixel
      If depth < depth_buffer[x][y]:
        depth_buffer[x][y] = depth   // closer triangle wins
        color_buffer[x][y] = shade(...)
      Else:
        discard fragment              // this triangle is occluded
```

This is O(pixels × triangles) in the worst case, but backface culling and early-Z rejection make it fast in practice. Our rasterizer doesn't implement early-Z, but the concept is straightforward: if the computed depth is greater than the current buffer value, skip the shading computation entirely.

## Build It

We're building this in three stages, each producing a PPM image. Every stage adds one new concept to the pipeline.

### Step 1: The Wireframe Cube — MVP Transform + Bresenham Lines

**Goal:** Render a wireframe cube with perspective projection. This stage validates your model-view-projection (MVP) pipeline.

The full MVP chain:

```
  clip_pos = projection × view × model × vertex_pos
  ndc_pos = clip_pos.xyz / clip_pos.w     (perspective divide)
  screen_x = (ndc_pos.x + 1) / 2 × width
  screen_y = (1 - ndc_pos.y) / 2 × height  (Y-flip)
```

A cube has 8 vertices and 12 edges. We transform each vertex through the MVP, project to screen coordinates, and draw 12 Bresenham lines.

**What the code does:**

```rust
let proj = make_perspective(fov, aspect, near, far);
let view = make_view_matrix(eye, center, up);
let model = make_model_matrix(rotation_y, Vec3::zero());
let mvp = proj.mul(view).mul(model);
```

We render two cubes at different rotations (0.6 rad and 0.9 rad) in different colors to demonstrate that the pipeline works for any transformation.

**Key insight:** The perspective projection is what makes the far edges of the cube appear smaller than the near edges. Without it, you'd get an orthographic projection where all parallel lines remain parallel regardless of distance — a perfectly valid projection, but not what we see with our eyes.

### Step 2: The Flat-Shaded Cube — Triangle Rasterization + Z-Buffer

**Goal:** Fill the cube's triangles with solid color, using the Z-buffer to resolve which face is in front.

This is where the core rasterization algorithm lives. For each triangle:

1. Transform all three vertices through the MVP pipeline.
2. Compute the screen-space bounding box of the triangle.
3. For each pixel in the bounding box, compute barycentric coordinates.
4. If the fragment is inside the triangle (u, v, w ≥ 0):
   - Interpolate depth using perspective-correct interpolation.
   - If depth < Z-buffer value, write color and update Z-buffer.

**The rasterize_triangle function:**

```rust
fn rasterize_triangle(fb: &mut Framebuffer, tri: &[ScreenVertex; 3], lights: &[Light]) {
    // Compute bounding box
    let min_x = tri[0].sx.min(tri[1].sx).min(tri[2].sx).floor() as i32;
    // ... max_x, min_y, max_y similarly, clamped to framebuffer

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let (u, v, w) = barycentric(px, py, tri[0].sx, tri[0].sy, ...);
            if u < 0.0 || v < 0.0 || w < 0.0 { continue; }  // outside

            // Perspective-correct interpolation of depth
            let z_ndc = (u*z0*inv_w0 + v*z1*inv_w1 + w*z2*inv_w2) / (u*inv_w0 + v*inv_w1 + w*inv_w2);
            let depth = (z_ndc + 1.0) * 0.5;  // map from NDC [-1,1] to [0,1]

            if depth < fb.depth[idx] {
                fb.depth[idx] = depth;
                // Shade the fragment
                let color = lambert(interpolated_normal, lights, base_color);
                fb.color[idx] = color;
            }
        }
    }
}
```

The Z-buffer is initialized to `+∞`, so the first triangle to write at any pixel always succeeds. Subsequent triangles only overwrite if they're closer. This is *order-independent* — draw the triangles in any order, and the result is the same.

**Key insight:** Flat shading means every fragment in a face gets the same color because the normal is constant per face. This makes the cube look clearly faceted — you can see each face as a distinct flat plane. The next step interpolates normals across the surface to create smooth shading.

### Step 3: The Lambert Scene — Multi-Object Rendering with Smooth Shading

**Goal:** Render a scene with a cube, two spheres, and a ground plane, all Lambert-shaded with multiple lights.

This step adds two things:

1. **Multiple scene objects** — We merge all triangle lists into a single flat array and render them all through the same pipeline. The Z-buffer handles occlusion between objects automatically.
2. **Per-fragment shading** — Instead of one color per face, we interpolate the normal across each triangle and shade each pixel independently. This gives spheres their characteristic smooth gradient.

The sphere is constructed by parameterizing the unit sphere with (theta, phi) coordinates:

```
  For each (ring, sector) pair:
    Compute vertices on the sphere surface
    Create two triangles per quad
    Normals are computed from the parameterization (they equal the position for a unit sphere)
```

With 12 rings and 16 sectors, each sphere produces 384 triangles. The Lambert formula for each fragment is:

```
  color_ambient  = 0.05 × base_color                    (fakes indirect light)
  color_diffuse  = base_color × light_color × max(N·L, 0)
  final_color    = color_ambient + Σ color_diffuse       (sum over all lights)
```

**The full render loop:**

```rust
fn render_scene(fb, scene, mvp, lights) {
    for triangle in scene.triangles {
        let screen_verts = transform_each_vertex(triangle, mvp, fb_w, fb_h);
        if any_vertex_behind_camera { continue; }  // simple near-plane rejection
        rasterize_triangle(fb, screen_verts, lights);
    }
}
```

No spatial accel structure — just iterate all triangles. For a scene with ~800 triangles, this runs in milliseconds on a modern CPU. For a million triangles, you'd need a BVH or octree (Lesson 12).

### Step 4 (Understanding): Why Are GPUs So Much Faster?

Our rasterizer processes one pixel at a time, one triangle at a time. The inner loop of `rasterize_triangle` touches every pixel inside each triangle's bounding box, does arithmetic, and writes to memory. On a CPU, this is inherently serial.

GPUs win because the rasterization workload has three forms of parallelism:

1. **Data parallelism** — Processing pixel (x, y) is independent of processing pixel (x+1, y). A GPU has thousands of cores running the *same* instruction on *different* data (SIMD/SIMT). Where our code visits pixels sequentially, a GPU visits hundreds simultaneously.

2. **Cache coherence** — All fragments in a triangle access the Z-buffer in a coherent pattern (sequential memory addresses). A GPU's L2 cache services thousands of fragments before they spill to VRAM. A CPU's cache line (64 bytes) gets evicted by the next triangle before it can be reused.

3. **Fixed-function units** — The Z-buffer comparison, blending, and viewport transform are implemented in dedicated silicon, not as general-purpose instructions. A single GPU clock cycle does what our Rust code takes 10–20 instructions to do.

The practical impact: our software rasterizer renders ~800 triangles at 640×480 in a few seconds. A GPU renders millions of triangles at 4K in 16ms (60 FPS). That's approximately a 10,000× speedup — almost entirely due to parallelism, not clock speed.

### When Are Software Rasterizers Still Useful?

Despite the speed gap, software rasterizers remain critical in several domains:

- **Education and debugging** — You can single-step through the pipeline, inspect any intermediate value, and understand exactly what happens at every stage. Try doing that on a GPU with 10,000 concurrent threads.
- **Offline rendering** — When you need absolute correctness (not real-time speed), a software rasterizer gives you deterministic results with no driver quirks. Render farms for film VFX sometimes use software rasterization for specific passes.
- **Testing and verification** — GPU vendors use software reference rasterizers to validate that their hardware produces bit-identical output. If the GPU result doesn't match the reference, it's a GPU bug.
- **Embedded systems** — Microcontrollers without GPUs still need to draw UIs. A minimal software rasterizer fits in a few KB of code and runs on any CPU.
- **Research** — New rasterization algorithms (stochastic, adaptive resolution, deep buffers) are prototyped in software before being committed to hardware.

Our rasterizer is production-quality in its *algorithm* (perspective-correct interpolation, proper Z-buffer, Lambert shading) but not in its *performance* (no tile-based rendering, no SIMD, no multithreading). The Phase Capstone (Lesson 18) will revisit this code and extend it.

## Use It

The production equivalent of our rasterizer is the **software reference rasterizer** that ships with every major graphics API:

- **OpenGL** has no official reference rasterizer, but Mesa's software driver (`llvmpipe`) implements the full pipeline in CPU code using LLVM for JIT compilation. It's used for testing and on headless servers.
- **Vulkan** provides a conformance test suite that includes a software rasterizer for validation. GPU vendors must prove their hardware matches it bit-for-bit.
- **Direct3D** ships with the Windows Advanced Rasterization Platform (WARP), a production software rasterizer that uses SIMD and multithreading. It's used as a fallback when no GPU is available.

Our code maps directly to these production systems. The key comparison:

| Concept | Our Code | Production (Mesa/LLVMPipe or WARP) |
|---------|----------|-----------------------------------|
| Vertex processing | Manual loop over vertices | LLVM-compiled vertex shader, runs on all CPU cores |
| Triangle setup | Per-triangle barycentric | Fixed-function triangle setup in SIMD |
| Fragment shading | Per-pixel Lambert | Arbitrary GLSL/HLSL compiled to SIMD |
| Z-buffer | `Vec<f32>`, sequential | Tile-based, compressed depth, hierarchical Z |
| Output | PPM file | Swap chain / window surface |

The algorithm is identical. The difference is *how many pixels you process per clock cycle* — and that's purely a hardware question.

## Read the Source

- **Mesa `llvmpipe`** — [`src/gallium/drivers/llvmpipe/lp_rast_tri.c`](https://gitlab.freedesktop.org/mesa/mesa/-/blob/main/src/gallium/drivers/llvmpipe/lp_rast_tri.c) — The triangle rasterizer. Notice how it processes 4×4 pixel tiles in SIMD. Compare the `lp_rast_tri_3` function to our `rasterize_triangle` — same algorithm, vectorized.
- **WARP** — Microsoft's WARP rasterizer is closed-source, but the [original GDC 2009 presentation](https://queue.acm.org/detail.cfm?id=1557559) describes its tile-based architecture and SIMD strategy.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A complete software rasterizer module** — the `main.rs` file is a self-contained Rust module that you can compile with `rustc main.rs` (no Cargo needed). It exports the core rasterization pipeline (`Framebuffer`, `rasterize_triangle`, `transform_vertex`, `lambert`, `save_ppm`) and produces three PPM images demonstrating wireframe, flat-shaded, and Lambert-shaded rendering.

This rasterizer reappears in the Phase Capstone (Lesson 18) where it will be extended with texturing and combined with a path tracer.

## Exercises

1. **Easy** — Modify the wireframe demo to draw a pyramid (5 vertices, 8 edges) instead of a cube. Change the camera position and observe how perspective changes.

2. **Medium** — Add **Gouraud shading** (per-vertex, interpolated to fragments) as an alternative to per-fragment Lambert. In Gouraud shading, you compute the color at each vertex and let barycentric interpolation blend between them. Compare the result: where does Gouraud miss specular highlights that per-fragment shading catches?

3. **Hard** — Implement **backface culling** before rasterization: compute the signed area of the projected triangle (in screen space). If the area is negative (clockwise winding), skip the triangle entirely. Measure the rendering speedup on the Lambert scene. Then add **near-plane clipping**: if a vertex has `clip.w < near`, clip the triangle against the near plane using Sutherland-Hodgman (Lesson 05). This prevents the catastrophic visual artifacts that occur when vertices pass behind the camera.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| MVP matrix | "The transform" | The product of Model × View × Projection matrices that takes a vertex from object space to clip space |
| Perspective divide | "The w divide" | Dividing clip-space (x, y, z) by w to get normalized device coordinates — this is what makes near things big and far things small |
| Barycentric coordinates | "The triangle weights" | Three values (u, v, w) that express any point inside a triangle as a weighted combination of the three vertices, summing to 1 |
| Perspective-correct interpolation | "Correct interpolation" | Interpolating attributes by dividing by w, linearly interpolating in screen space, then dividing by interpolated 1/w — prevents texture swim and depth errors |
| Z-buffer | "Depth buffer" | A per-pixel array of depth values, initialized to ∞, where only closer fragments overwrite — the standard visibility algorithm since 1974 |
| Fragment | "Almost a pixel" | A candidate color+depth for a pixel produced by rasterizing a triangle. Only becomes a pixel after the Z-buffer test and blending. |
| Software rasterizer | "CPU renderer" | A rasterizer that runs entirely on the CPU, used for debugging, testing, education, and offline rendering |

## Further Reading

- **Tiny Renderer** (Dmitry Sokolov) — [github.com/ssloy/tinyrenderer](https://github.com/ssloy/tinyrenderer) — A 500-line C++ software rasterizer that covers the same pipeline. Excellent companion to this lesson.
- **Mesa llvmpipe** — [mesa3d.org](https://www.mesa3d.org/) — The production software rasterizer for OpenGL. Study the tile-based rendering path.
- **Rasterization in One Weekend** (Fabian Giesen) — [fgiesen.wordpress.com](https://fgiesen.wordpress.com/2013/02/10/optimizing-the-basic-rasterizer/) — A deep dive into optimizing the inner loop of triangle rasterization. The series covers half-space tests, tile-based rendering, and SIMD.
- **A Trip Through the Graphics Pipeline** (Fabian Giesen) — [fgiesen.wordpress.com](https://fgiesen.wordpress.com/2011/07/09/a-trip-through-the-graphics-pipeline-2011-index/) — A 13-part series covering the entire GPU pipeline from application to pixel. Recommended for understanding the hardware that replaces your software loops.
- **WARP: High-Performance Software Rasterization** (Microsoft, GDC 2009) — The design document for Direct3D's software fallback. Covers tile-based rendering, SIMD fragment shading, and multithreading.