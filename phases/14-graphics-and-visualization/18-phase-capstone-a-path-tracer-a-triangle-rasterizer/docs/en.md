# Phase Capstone — A Path Tracer + a Triangle Rasterizer

> *"The only way to truly understand rendering is to build it twice."*

**Type:** Build  
**Languages:** Rust  
**Prerequisites:** Phase 14 (Lessons 01–17)  
**Time:** ~180 minutes

---

## Learning Objectives

1. **Unify the rendering equation.** Explain why rasterization and path tracing are both valid strategies for approximating the same physical equation, and why they make opposite tradeoffs between speed and physical accuracy.

2. **Build two complete renderers from shared foundations.** Implement a real-time triangle rasterizer and an offline path tracer that consume the same scene description and camera, demonstrating that the math (transforms, shading, intersection) is renderer-agnostic.

3. **Quantify the difference.** Render the same Cornell-box scene with both pipelines, compare the output images, and articulate what each gets right and wrong (soft shadows, color bleeding, caustics, aliasing, performance).

4. **Connect every prior lesson.** Trace how gamma correction (L01), the pipeline model (L02), transforms and projections (L03), rasterization primitives (L04–05), shading models (L06–08), the software rasterizer (L09), ray tracing (L10–11), and BVH acceleration (L12) all feed into this single dual-renderer artifact.

5. **Ship a reusable framework.** Walk away with code you can extend — add textures, area lights, or GPU dispatch and both renderers benefit because they share the same scene and math layer.

---

## The Problem

You have spent seventeen lessons learning two families of rendering technique. Lessons 01–09 taught you how GPUs turn triangles into pixels: transform them, project them, shade them per-vertex or per-fragment, and resolve visibility with a z-buffer. Lessons 10–12 taught you how ray tracing finds light paths by shooting rays into the scene and recursively following bounces. Lesson 11 showed you that path tracing — shooting many random paths per pixel and averaging — converges to ground-truth physics.

Here is the problem: **neither family alone is sufficient for a well-rounded graphics engineer.**

A rasterizer is fast but physically incomplete. It can produce a shaded image in under a millisecond, but it cannot compute soft shadows, indirect illumination, caustics, or color bleeding without stacking heuristic hacks (shadow maps, ambient occlusion, light probes) that each require separate pipelines and artist tuning.

A path tracer is physically complete but slow. It produces correct global illumination, soft shadows, and caustics for free, but each pixel may require thousands of samples to converge, making it impractical for real-time use.

In practice, production studios and game engines use **both**. Film studios like Pixar render final frames with path tracing (via RenderMan) but use rasterized proxies for interactive viewport preview. Game engines like Unity HDRP and Unreal Engine use rasterization for the real-time frame but layer in ray-traced effects (reflections, shadows, global illumination) via hybrid pipelines. Understanding why both exist, how they share foundations, and when to reach for each is the mark of someone who actually understands rendering.

This capstone asks you to prove that understanding by building **both** renderers, from scratch, against the same scene, producing two images you can compare side by side.

---

## The Concept

### The Rendering Equation — The One True Answer

Every rendering method is an approximation of Kajiya's rendering equation:

$$L_o(p, \omega_o) = L_e(p, \omega_o) + \int_{\Omega} f_r(p, \omega_i, \omega_o) \, L_i(p, \omega_i) \, (\omega_i \cdot n) \, d\omega_i$$

The outgoing radiance at point $p$ in direction $\omega_o$ equals emitted radiance plus the integral over all incoming directions of the BRDF $f_r$ times incoming radiance $L_i$ times the cosine term. That integral has no closed-form solution for non-trivial scenes.

**Rasterization** approximates this by:
1. Evaluating the equation only at directly visible surfaces (primary rays).
2. Using a small fixed number of light source samples (typically one per light).
3. Adding indirect light via pre-baked or screen-space approximations.

**Path tracing** approximates this by:
1. Sampling random directions $\omega_i$ to estimate the integral via Monte Carlo.
2. Recursively tracing secondary rays to find $L_i$.
3. Averaging many independent path samples per pixel to reduce variance.

The tradeoff is stark:

| Property | Rasterizer | Path Tracer |
|---|---|---|
| Time per frame | <1 ms | seconds to hours |
| Global illumination | heuristic | physically correct |
| Soft shadows | needs shadow maps | free |
| Caustics | very hard | free |
| Noise | none (deterministic) | reduces as O(1/√N) |
| Memory usage | per-fragment buffers | per-sample accumulation |
| Interactivity | yes | no (offline) |

### How Prior Lessons Feed This Capstone

Every lesson in Phase 14 contributes a piece:

- **L01 (Pixels, Colors, Gamma):** Correct linear-space shading and gamma-encoding the final output. Without this, both renderers produce washed-out or over-saturated images.
- **L02 (The Graphics Pipeline):** The rasterizer follows the pipeline model exactly: vertex processing → rasterization → fragment shading → framebuffer. The path tracer replaces the pipeline with a per-pixel ray loop.
- **L03 (Linear Algebra):** `Vec3`, `Mat4`, projection matrices, and `look_at` are shared by both renderers. The camera uses the same transform regardless of which renderer draws the scene.
- **L04 (Rasterization I — Lines & Triangles):** Barycentric coordinates for interpolation. The rasterizer uses them for fragment shading; the path tracer uses them for triangle intersection.
- **L05 (Rasterization II — Z-buffer, Clipping, Culling):** The depth buffer solves visibility for the rasterizer. The path tracer solves it implicitly — the closest intersection is the visible surface.
- **L06 (Shading Models — Lambert, Phong):** Lambert's cosine law is the $(\omega_i \cdot n)$ term in the rendering equation. Both renderers compute it.
- **L07 (PBR — BRDF, Microfacet):** The path tracer's BRDF sampling directly implements the microfacet model. The rasterizer uses the simplified Cook-Torrance specular term.
- **L08 (Shaders 101):** The rasterizer's fragment shader is a function that takes interpolated attributes and returns a color. The path tracer's `trace()` function is the same idea, but called recursively.
- **L09 (Software Rasterizer):** This is the direct ancestor of the rasterizer half of this capstone. You are extending it with proper perspective-correct interpolation and Lambert shading.
- **L10 (Ray Tracing I — Whitted):** Primary rays, shadow rays, and reflection rays. The path tracer builds on this foundation but replaces the deterministic bounce selection with stochastic sampling.
- **L11 (Ray Tracing II — Path Tracing):** The mathematical core of this capstone's second renderer: Monte Carlo integration, cosine hemisphere sampling, Russian roulette termination.
- **L12 (Acceleration):** BVH or spatial subdivision reduces ray-scene intersection from $O(n)$ to $O(\log n)$. This capstone uses a simple linear scan for clarity, but the code is structured so that adding a BVH requires changing only the intersection function.

The remaining lessons (L13–L17) cover real-time techniques, compute shaders, modern APIs, animation, and visualization. They represent production refinements that you could layer onto the dual-renderer framework once it works.

---

## Build It

We build two renderers in five steps. Each step produces runnable code. The final step renders the same scene with both and compares.

### Step 1: Shared Math Foundation

Both renderers need the same vector and matrix operations. We define them once:

```rust
#[derive(Clone, Copy)]
struct Vec3 { x: f64, y: f64, z: f64 }

impl Vec3 {
    fn new(x: f64, y: f64, z: f64) -> Self { Self { x, y, z } }
    fn zero() -> Self { Self { x: 0.0, y: 0.0, z: 0.0 } }
    fn dot(self, other: Self) -> f64 { self.x*other.x + self.y*other.y + self.z*other.z }
    fn cross(self, other: Self) -> Self {
        Self::new(self.y*other.z - self.z*other.y,
                  self.z*other.x - self.x*other.z,
                  self.x*other.y - self.y*other.x)
    }
    fn length(self) -> f64 { self.dot(self).sqrt() }
    fn normalized(self) -> Self { let l = self.length(); Self::new(self.x/l, self.y/l, self.z/l) }
}
// Addition, subtraction, scalar multiply, etc. — see full code
```

We also define `Mat4` with `look_at()` and `perspective()` — used by the rasterizer for the MVP transform and by the path tracer to construct primary rays from the camera.

**Key insight:** The camera does not care which renderer draws the scene. A `Camera` struct holds `eye`, `target`, `up`, `fov`, and `aspect`. Both renderers consume it.

### Step 2: Scene Definition

Both renderers share the same scene structure:

```rust
struct Material {
    diffuse: Vec3,   // base color (albedo)
    emissive: Vec3,  // emitted light (for area lights)
}

struct Sphere { center: Vec3, radius: f64, material: usize }
struct Triangle { v0: Vec3, v1: Vec3, v2: Vec3, material: usize }
struct Light { position: Vec3, color: Vec3, intensity: f64 }
struct Scene {
    spheres: Vec<Sphere>,
    triangles: Vec<Triangle>,
    lights: Vec<Light>,
    materials: Vec<Material>,
}
```

The rasterizer iterates over triangles, transforms vertices to clip space, scans their screen-space footprint, and shades visible fragments. The path tracer iterates over spheres and triangles for each ray, finds the closest intersection, and recursively bounces.

Both produce `Framebuffer { width, height, color: Vec<Vec3> }` — an array of linear-space radiance values.

### Step 3: The Triangle Rasterizer

The rasterizer pipeline for each triangle:

1. **Vertex Transform:** Multiply each vertex by the Model-View-Projection matrix to get clip-space coordinates, then perspective-divide to get NDC, then map to screen coordinates.
2. **Bounding Box & Scissor:** Compute the screen-space axis-aligned bounding box of the triangle. Clip to the framebuffer.
3. **Barycentric Test:** For each pixel center in the bounding box, compute barycentric coordinates. If all three weights are non-negative, the pixel is inside the triangle.
4. **Depth Test:** Use the barycentric weights to interpolate depth. If the interpolated depth is closer than the z-buffer value, this fragment wins.
5. **Shading:** Interpolate the surface normal using barycentric weights (or compute it from the face normal). Compute Lambert shading: `color = material.diffuse * max(0, dot(normal, light_dir)) * light.intensity`. Sum contributions from all lights.
6. **Write:** Store the final color in the framebuffer.

```rust
fn rasterize(fb: &mut Framebuffer, scene: &Scene, camera: &Camera) {
    let view = Mat4::look_at(camera.eye, camera.target, camera.up);
    let proj = Mat4::perspective(camera.fov, camera.aspect, 0.1, 100.0);
    let mvp = proj * view;

    for tri in &scene.triangles {
        let v0 = mvp * Vec4::from_point(tri.v0);
        let v1 = mvp * Vec4::from_point(tri.v1);
        let v2 = mvp * Vec4::from_point(tri.v2);

        // Perspective divide
        let sv0 = ScreenVertex::from_clip(v0, fb.width, fb.height);
        let sv1 = ScreenVertex::from_clip(v1, fb.width, fb.height);
        let sv2 = ScreenVertex::from_clip(v2, fb.width, fb.height);

        // Compute bounding box, iterate pixels, test barycentric, shade...
    }
}
```

The rasterizer is deterministic: same input, same output, no noise. It produces an image in microseconds per triangle. But it cannot produce soft shadows, indirect illumination, or caustics — only direct lighting with hard shadow edges (via shadow rays or shadow maps, which we simplify here by not including).

### Step 4: The Path Tracer

The path tracer replaces the per-triangle scan with a per-pixel ray march:

1. **Primary Ray:** For each pixel, construct a ray from the camera eye through the pixel center on the near plane.
2. **Intersection:** Find the closest intersection with any sphere or triangle in the scene.
3. **Direct Lighting:** At the hit point, shoot shadow rays to each light. If unoccluded, accumulate direct illumination using the BRDF.
4. **Indirect Lighting:** Sample a random direction on the cosine hemisphere above the surface normal. Recursively trace that ray and accumulate indirect illumination weighted by the BRDF and the cosine term.
5. **Russian Roulette:** With probability $p$, terminate the path. Otherwise, scale throughput by $1/(1-p)$. This converts infinite-length paths into finite expected cost.
6. **Accumulation:** Average multiple samples per pixel to reduce variance.

```rust
fn trace(ray: &Ray, scene: &Scene, depth: u32, rng: &mut impl Rng) -> Vec3 {
    if depth == 0 { return Vec3::zero(); }

    let hit = scene.intersect(ray);
    let hit = match hit { Some(h) => h, None => return Vec3::zero() };

    let mat = &scene.materials[hit.material];
    let emitted = mat.emissive;

    // Russian roulette
    let rr_prob = 0.8;
    if rng.gen::<f64>() > rr_prob { return emitted; }

    // Sample cosine hemisphere
    let (sample_dir, pdf) = cosine_hemisphere(rng);
    let cos_theta = sample_dir.dot(hit.normal).abs();
    let brdf = mat.diffuse * (1.0 / std::f64::consts::PI);
    let incoming = trace(&Ray::new(hit.point, sample_dir), scene, depth - 1, rng);

    emitted + brdf * incoming * cos_theta / (pdf * (1.0 - rr_prob))
}
```

The path tracer is stochastic: different runs produce slightly different images due to random sampling. It converges to the physical truth as `samples_per_pixel` increases. It produces soft shadows, color bleeding, and caustics "for free" — they are emergent properties of simulating light transport, not explicit features you must code.

### Step 5: Render and Compare

We render a Cornell Box scene — five walls (floor, ceiling, back, left-red, right-green), two small spheres, and one area light on the ceiling. The Cornell Box is the standard test case for global illumination because:

- The red and green walls produce obvious color bleeding onto the white surfaces.
- The area light creates soft shadows that the rasterizer cannot reproduce.
- Any light bouncing off the red wall and onto the green wall (and vice versa) demonstrates multi-bounce indirect illumination.

```rust
fn main() {
    let scene = build_cornell_box();
    let camera = Camera::new(
        Vec3::new(0.0, 0.0, 2.0),   // eye
        Vec3::new(0.0, 0.0, -1.0),   // target
        Vec3::new(0.0, 1.0, 0.0),    // up
        60.0, 400.0 / 300.0          // fov, aspect
    );

    let mut fb_rast = Framebuffer::new(400, 300);
    rasterize(&mut fb_rast, &scene, &camera);
    save_ppm(&fb_rast, "rasterizer.ppm");

    let fb_path = path_trace(&scene, &camera, 32);
    save_ppm(&fb_path, "pathtracer.ppm");

    println!("Rasterizer: 400x300 in fast deterministic time");
    println!("Path tracer: 400x300 x 32 samples, physically correct GI");
    println!("Compare: color bleeding, soft shadows, indirect light");
}
```

**What to look for when comparing the two images:**

1. **Direct illumination:** Both renderers should agree on directly lit surfaces — they are both computing Lambert's cosine law for direct light.
2. **Shadows:** The rasterizer produces hard-edged shadows (or no shadows if we skip shadow mapping). The path tracer produces soft penumbrae around shadow boundaries.
3. **Color bleeding:** Only the path tracer will show the red and green walls "staining" nearby white surfaces with their color. The rasterizer cannot do this without extra passes.
4. **Noise:** The rasterizer output is noise-free. The path tracer output has noise that reduces with more samples.
5. **Brightness:** The path tracer's indirect illumination adds light that the rasterizer misses entirely, making the overall image brighter and more realistic.

---

## Use It

### Production Rendering Systems

**Mitsuba 3** (https://mitsuba-renderer.org) — A research-oriented physically based renderer that supports both rasterization and path tracing backends. Mitsuba's architecture closely mirrors what we built: a scene description is consumed by multiple rendering algorithms. Its plugin system lets you swap between a rasterizer, a path tracer, a bidirectional path tracer, and more, without changing the scene.

**pbrt** (https://pbrt.org) — The reference implementation from *Physically Based Rendering: From Theory to Implementation* by Pharr, Jakob, and Humphreys. pbrt is the gold standard for path tracer implementations. Its `Integrator` class hierarchy maps directly to our `rasterize()` and `path_trace()` functions — each is an "integrator" that estimates the rendering equation differently.

**Google Filament** (https://github.com/google/filament) — A real-time PBR rasterizer for Android and the web. Filament implements the rasterization side of our capstone at production quality: perspective-correct interpolation, z-buffer, PBR shading with IBL (image-based lighting for indirect illumination). It uses pre-baked environment maps as a fast approximation of what path tracing computes exactly.

**Unity HDRP / Unreal Engine** — Modern game engines use hybrid rendering: rasterize the primary visibility, then overlay ray-traced reflections, shadows, and global illumination via DXR or Vulkan ray tracing. The two renderers we built are the two halves of this hybrid approach.

### What Production Does That We Don't

| Feature | Our Implementation | Production |
|---|---|---|
| Acceleration | Linear scan $O(n)$ | BVH $O(\log n)$ |
| Textures | Solid colors | UV-mapped texture sampling |
| Anti-aliasing | None (rasterizer) / statistical (path tracer) | MSAA, FXAA, TAA |
| Denoising | None | OptiX denoiser, OIDN |
| Importance sampling | Cosine only | BRDF-aware, environment map |
| Multiple importance sampling | No | Yes (combines BSDF and light sampling) |
| Spectral rendering | RGB | Full spectrum for dispersion |

---

## Read the Source

- **pbrt** (https://github.com/mmp/pbrt-v4) — Read `src/pbrt/integrators/path.cpp` for the path tracing integrator and `src/pbrt/cpu/integrators.cpp` for the other integrators (Whitted, direct lighting, AO). Notice how pbrt's `Integrator` interface makes it trivial to swap rendering algorithms.

- **Filament** (https://github.com/google/filament) — Read `libs/filament/src/Renderer.cpp` and `libs/filament/src/Scene.cpp` for how a production rasterizer handles the full pipeline. Compare the draw-call batching and sorting logic to our simple per-triangle loop.

- **raytracing.github.io** — The free online book "Ray Tracing in One Weekend" (https://raytracing.github.io) by Peter Shirley is an excellent companion to our path tracer implementation. Our code follows the same structure but adds triangle intersection and a side-by-side rasterizer.

---

## Ship It

The reusable artifact lives in `outputs/`. It is **a dual-renderer graphics framework** — a single Rust file that implements:

1. **Shared math:** `Vec3`, `Vec4`, `Mat4`, `Ray` — reusable across any rendering project.
2. **Shared scene:** `Material`, `Sphere`, `Triangle`, `Light`, `Scene` — renderer-agnostic geometry and materials.
3. **Rasterizer:** `Framebuffer`, `rasterize()` — a complete software triangle rasterizer with MVP transform, barycentric interpolation, z-buffer, and Lambert shading.
4. **Path tracer:** `trace()`, `path_trace()` — a Monte Carlo path tracer with cosine hemisphere sampling, Russian roulette, and multi-bounce global illumination.
5. **Output:** `save_ppm()` — writes standard PPM images that any image viewer can open.

This framework is deliberately structured for extension:

- Adding a BVH requires changing only `Scene::intersect()` — both renderers benefit.
- Adding textures requires extending `Material` — both renderers benefit.
- Adding importance sampling requires changing `trace()`'s sampling strategy.
- Porting to GPU (via compute shaders) requires replacing the per-pixel loop in `path_trace()` and the per-triangle loop in `rasterize()` — the scene and math layers stay the same.

This artifact connects forward to **Phase 15 (Systems Performance)** — the path tracer is an ideal candidate for parallelization, SIMD optimization, and cache-friendly memory layout. It also connects to **Phase 16 (Software Architecture)** — the shared scene/math layer vs. renderer-specific pipeline is a case study in abstraction boundaries and interface design.

---

## Exercises

### Easy

1. **Reproduce the implementation.** Delete the code, keep only the lesson document, and re-implement both renderers from scratch. Verify your output matches the reference by comparing PPM files.

2. **Switch the scene.** Replace the Cornell Box with a different configuration — add more spheres, change wall colors, or add an emissive floor. Observe how both renderers handle the change.

### Medium

3. **Add shadow rays to the rasterizer.** Extend `rasterize()` to shoot a shadow ray from each fragment toward each light. If any geometry occludes the ray, multiply the light contribution by zero. Compare the hard shadow result with the path tracer's soft shadows.

4. **Add a BVH.** Build a simple bounding volume hierarchy for the scene's triangles. Replace the linear scan in `Scene::intersect()` with BVH traversal. Measure the speedup for a scene with 1000+ triangles.

5. **Implement Blinn-Phong shading in the rasterizer.** Extend the fragment shader to include a specular highlight using the Blinn-Phong halfway vector. Compare the specular highlights with the path tracer's natural specular behavior.

### Hard

6. **Bidirectional path tracing.** Implement light-path tracing — start paths from both the camera and the lights, then connect sub-paths. This significantly reduces variance in scenes with small light sources or specular-diffuse-specular paths (caustics).

7. **Spectral rendering.** Replace the RGB `Vec3` color model with a sampled- spectrum representation (e.g., 30 wavelengths from 380nm to 780nm). Render a prism scene that demonstrates wavelength-dependent refraction. Convert the spectral output to XYZ and then to sRGB for display.

8. **GPU path tracer.** Port the path tracer to run on the GPU using WebGPU compute shaders (via `wgpu` in Rust). Compare the speedup on the same scene. This is an excellent bridge to Phase 15.

---

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Rendering equation | "the light equation" | An integral equation describing the equilibrium of light at every surface point; all renderers approximate it |
| Rasterization | "turning triangles to pixels" | Projecting geometry onto screen, scanning fragments, and shading only directly visible surfaces |
| Path tracing | "ray tracing with Monte Carlo" | Estimating the rendering equation by sampling random light paths and averaging many samples |
| Monte Carlo integration | "random sampling to solve integrals" | Estimating an integral by averaging random samples; variance decreases as O(1/√N) |
| Barycentric coordinates | "weights inside a triangle" | Three numbers (λ₀, λ₁, λ₂) that express any point inside a triangle as a weighted combination of vertices |
| Z-buffer | "depth buffer" | A per-pixel array storing the closest depth seen so far; used by rasterizers to resolve visibility |
| Russian roulette | "stochastic path termination" | Randomly terminating paths with some probability, rescaling surviving paths to keep the estimator unbiased |
| Cosine hemisphere sampling | "sample proportional to cos θ" | Generating random directions on the hemisphere weighted by the cosine of the angle with the normal, matching the Lambertian BRDF |
| BRDF | "how shiny is it" | Bidirectional Reflectance Distribution Function — the ratio of reflected radiance to incident irradiance as a function of incoming and outgoing directions |
| Cornell Box | "the GI test scene" | A standardized scene with colored walls, a light on the ceiling, and sometimes blocks; used to validate global illumination algorithms |
| Global illumination (GI) | "bounced light" | Light that has bounced off one or more surfaces before reaching the camera; includes color bleeding, soft shadows, and caustics |
| MVP matrix | "the transform" | Model-View-Projection — the matrix chain that transforms vertices from object space to clip space in a rasterizer |

---

## Further Reading

- **Pharr, Jakob, Humphreys.** *Physically Based Rendering: From Theory to Implementation* (4th ed, 2023). The definitive reference for path tracing. Free online at https://pbrt.org.
- **Shirley.** *Ray Tracing in One Weekend* series. Free at https://raytracing.github.io. Start here for your first path tracer.
- **Akenine-Möller, Haines, Hoffman.** *Real-Time Rendering* (4th ed, 2018). The definitive reference for rasterization and GPU pipelines.
- **Kajiya.** "The Rendering Equation" (SIGGRAPH 1986). The original paper that formalized light transport as an integral equation.
- **Veach.** "Robust Monte Carlo Methods for Light Transport Simulation" (PhD thesis, 1997). Introduced multiple importance sampling and bidirectional path tracing.
- **Ritschel et al.** "The State of the Art in HDR and Image-Based Lighting" (Eurographics 2023). Modern survey of how rasterizers approximate global illumination.
- **Peters.** "Mitsuba 3: A Retargetable Forward and Inverse Renderer" (SIGGRAPH Asia 2023). The latest in research-grade physically based rendering systems.