# Ray Tracing II — Path Tracing, Monte Carlo

> Whitted-style ray tracing gives you perfect reflections and hard shadows. Path tracing gives you reality.

**Type:** Learn
**Languages:** Rust
**Prerequisites:** Phase 14 lessons 01–10 (especially L10: Whitted-style ray tracing)
**Time:** ~90 minutes

## Learning Objectives

- Understand why Whitted-style ray tracing fails at soft shadows, diffuse interreflection, and caustics
- Learn Monte Carlo integration and how it estimates definite integrals
- Derive and implement cosine-weighted hemisphere sampling
- Implement a path tracer with Russian roulette termination
- Explain why variance reduces as O(1/√N) and why path tracing is slow but correct
- Recognize that soft shadows, color bleeding, and caustics emerge naturally from path tracing

## The Problem

Last lesson you built a Whitted-style ray tracer. It traced mirror reflections and refracted rays
recursively, producing shiny balls with sharp shadows. It looked impressive — but it was wrong.

Consider a room with red walls and a white floor lit by an area light. In real life, the floor
near the red wall takes on a reddish tint: light bounces off the red wall and carries that color
to the floor. This is **diffuse interreflection** (also called **color bleeding**).

```
  Whitted-style result          Physical reality

  +-------------------+         +-------------------+
  |                   |         |  red wall         |
  |   hard shadow     |         |  /  soft reddish  |
  |   ___________     |         | /  glow on floor  |
  |  |          |    |         |/___________________|
  |  |  sphere  |    |         | reddish-white floor|
  +--|----------|----+         +-------------------+
     sharp edge                  soft, gradual

```

Whitted's algorithm cannot produce this. It only traces **specular** rays (mirror reflections and
refractions). Diffuse surfaces just stop — they sample the direct light and nothing else. This
means:

- **No soft shadows**: Point lights produce infinitely hard shadow edges.
- **No diffuse interreflection**: A red wall never tints a white floor.
- **No caustics**: Light focused through a glass ball onto a diffuse surface won't appear.

These phenomena account for a huge fraction of what makes real scenes look real. Without them, you
get the "plastic" look of early 3D graphics.

## The Concept

### The Rendering Equation

In 1986, James Kajiya formulated the **rendering equation**, which describes the total light
leaving a point in a scene:

```
  L_o(p, ω_o) = L_e(p, ω_o) + ∫_Ω f(p, ω_i, ω_o) · L_i(p, ω_i) · cos(θ_i) dω_i

  Where:
    L_o      = outgoing radiance at point p in direction ω_o
    L_e      = emitted radiance (light sources)
    f        = BRDF (bidirectional reflectance distribution function)
    L_i      = incoming radiance from direction ω_i
    cos(θ_i) = Lambert's cosine law — dot(n, ω_i)
    Ω        = hemisphere above the surface normal
```

The integral is over the entire hemisphere of incoming directions. The Whitted approach evaluates
this integral at only a few discrete directions (perfectly specular or refracted). Path tracing
evaluates it statistically.

### Monte Carlo Integration

The fundamental tool is **Monte Carlo integration**. Given an integral ∫f(x)dx over some domain,
we approximate it by sampling random points:

```
  ∫f(x) dx ≈ (1/N) Σ f(x_i) / p(x_i)

  Where:
    N     = number of samples
    x_i   = random samples drawn from distribution p
    p(x_i) = probability density of sample x_i
```

**Worked example**: Estimate ∫₀^π sin(x) dx. The true answer is 2.

```
  Sample x uniformly in [0, π]:
    p(x) = 1/π

  For N=4, suppose we draw: x₁=0.8, x₂=1.5, x₃=2.1, x₄=2.8

  Estimate = (1/4) · Σ sin(x_i)/p(x_i)
           = (1/4) · [sin(0.8)/0.318 + sin(1.5)/0.318 + sin(2.1)/0.318 + sin(2.8)/0.318]
           = (1/4) · [2.26 + 3.14 + 2.73 + 1.02]
           = (1/4) · 9.15
           ≈ 2.29

  Error is about 14%. More samples → smaller error.
```

The **variance** of the estimate decreases as O(1/√N). Double the quality → quadruple the samples.
This is both the power and the pain of Monte Carlo.

### Importance Sampling

Uniform sampling wastes effort on directions that contribute little. **Importance sampling**
draws more samples where f(x)/p(x) is large:

```
  Bad:  sample all directions uniformly → lots of wasted samples pointing at empty space
  Good: sample directions toward bright light sources more often

  The key insight: if p(x) is proportional to f(x), then f(x)/p(x) = constant,
  and variance drops to zero. Perfect importance sampling eliminates all noise.
  In practice, we approximate this by sampling proportional to the BRDF or to
  the light distribution.
```

### Cosine-Weighted Hemisphere Sampling

For diffuse (Lambertian) surfaces, the BRDF is constant: f = ρ/π where ρ is the albedo. The
cosine factor in the rendering equation already tells us that directions near the normal
contribute more. So we sample directions with probability proportional to cos(θ):

```
  PDF on hemisphere:  p(ω) = cos(θ)/π

  To generate a cosine-weighted direction on a unit hemisphere
  aligned with the z-axis, use two uniform random numbers r₁, r₂ ∈ [0,1):

    φ = 2π · r₂           (azimuthal angle, uniform around the circle)
    r  = √(1 - r₁)        (radius of disk sample; r₁ maps to cos(θ) = r₁)
    x  = √(r₁) · cos(φ)   = cos(2πr₂) · √r₁
    y  = √(r₁) · sin(φ)   = sin(2πr₂) · √r₁
    z  = √(1 - r₁)

  So the sampled direction is:
    d = (√r₁ · cos(2πr₂),  √r₁ · sin(2πr₂),  √(1-r₁))

  Why this works: the mapping concentrates samples near the pole (z-axis),
  exactly where cos(θ) is largest.  The PDF = cos(θ)/π cancels nicely
  with the cos(θ) in the rendering equation for Lambertian surfaces.
```

### Path Tracing Algorithm

Path tracing is Monte Carlo integration applied to the rendering equation. At each surface hit:

```
  path_trace(ray, depth):
    hit = intersect_scene(ray)
    if no hit:
        return background_color (or black for interior scenes)

    color = hit.material.emission  (for light sources)

    if depth > max_depth:
        return color

    // Choose a random direction on the hemisphere
    direction = cosine_weighted_hemisphere(normal)

    // Compute the BRDF contribution
    cos_theta = dot(normal, direction)
    pdf = cos_theta / π   (for Lambertian)

    // For Lambertian: BRDF = albedo / π
    // throughput *= BRDF * cos_theta / pdf
    // throughput *= (albedo/π) * cos_theta / (cos_theta/π)
    // throughput *= albedo

    // Recurse
    incoming = path_trace(Ray(hit.point, direction), depth + 1)
    color += hit.material.albedo * incoming

    return color
```

The beautiful simplification for pure Lambertian surfaces: cosine-weighted sampling
exactly cancels the BRDF and cosine terms, so each bounce simply multiplies the
throughput by the surface albedo.

### Russian Roulette

Left unchecked, paths bounce forever. We can't set a hard maximum depth without introducing
systematic darkening (paths that would have kept bouncing are simply killed). **Russian roulette**
terminates paths probabilistically:

```
  At each bounce after a minimum depth (e.g., depth 3):
    p_continue = min(max(albedo_r, albedo_g, albedo_b), 1.0)
    // or simply p_continue = 0.5 for uniform termination

    if random() > p_continue:
        return black  (terminate this path)
    else:
        return color / p_continue  (compensate so the estimate stays unbiased)
```

The division by p_continue ensures the *expected value* stays correct. If we terminate 50% of
paths, the surviving paths must carry double the weight.

```
  Without RR:  E[L_o] = E[L_o]                              ✓
  With RR:     E[L_o] = p_continue · E[L_o/p_continue]       ✓
                    = E[L_o]                                  ✓ (unbiased)

  Variance increases slightly (we lose some paths), but we save
  computation on paths that contribute diminishingly small values.
```

### Convergence: Why Path Tracing Is Slow But Correct

```
  Standard deviation ~ σ/√N

  To halve the noise, you need 4× the samples.
  To reduce noise by a factor of 10, you need 100× the samples.

  At N=1     per pixel:  very noisy (like TV static)
  At N=16    per pixel:  soft shadows emerge, color bleeding visible
  At N=256   per pixel:  smooth, photorealistic (for simple scenes)
  At N=4096  per pixel:  reference quality

  A 400×300 image at 32 samples/pixel = 3,840,000 paths
  Each path: ray-scene intersection + recursion = ~5-10 bounces average
  Total: ~20-40 million ray-scene tests for a small scene
```

This is why production renderers use **denoising**: AI-based filters (NVIDIA OptiX Denoiser,
Intel OIDN) take a noisy 32-sample render and produce results that look like a 1000-sample render.
They exploit the spatial coherence of noise — neighboring pixels have correlated errors that can
be separated from the signal.

### Soft Shadows, Color Bleeding, Caustics — All For Free

Here's the key insight: path tracing doesn't need special cases for these effects. They emerge
naturally from the algorithm:

```
  ┌─────────────────────────────────────────────────────────────┐
  │  Effect            │ Whitted needs?     │ Path tracer?     │
  ├─────────────────────┼────────────────────┼──────────────────┤
  │  Soft shadows       │ Area light hack     │ Automatic        │
  │  Color bleeding     │ Can't do it         │ Automatic        │
  │  Caustics           │ Photon map hack     │ Automatic*       │
  │  Specular reflect.  │ Recursive ray       │ Automatic        │
  │  Glossy surfaces    │ Can't do it         │ Automatic        │
  │  Volumetric scat.   │ Very hard           │ Automatic        │
  └─────────────────────┴────────────────────┴──────────────────┘

  * Caustics work in principle but converge very slowly without
    specialized sampling (e.g., bidirectional path tracing).
```

When a path hits a diffuse surface, it bounces in a random direction. That bounce might
hit the red wall, carry red light back, and tinge the floor. No special code needed — just
the physics.

## Build It

### Step 1: Minimal Path Tracer (Direct Lighting Only)

First, trace a single ray per pixel to a light source. This gives you hard shadows (like
Whitted) but sets up the infrastructure.

```rust
fn trace_ray(ray: &Ray, scene: &Scene, depth: u32) -> Color {
    if depth > MAX_DEPTH { return Color::black(); }
    if let Some(hit) = scene.intersect(ray) {
        let to_light = (scene.light_pos - hit.point).normalize();
        let shadow = scene.intersect(&Ray::new(hit.point + hit.normal * 0.001, to_light));
        if shadow.is_some() { return Color::black(); }
        let cos_theta = hit.normal.dot(&to_light).max(0.0);
        return hit.material.albedo * cos_theta;
    }
    Color::black()
}
```

This is just Whitted with a point light — hard shadows, no interreflection.

### Step 2: Full Path Tracer

Replace the single shadow ray with a random bounce. Average many paths per pixel:

```rust
fn path_trace(ray: &Ray, scene: &Scene, rng: &mut Rng, depth: u32) -> Color {
    if depth > MAX_DEPTH { return Color::black(); }
    if let Some(hit) = scene.intersect(ray) {
        // Emission from light sources
        let mut color = hit.material.emission;

        // Russian roulette after depth 3
        if depth > 3 {
            let p_continue = hit.material.albedo.max_component().min(1.0);
            if rng.gen_float() > p_continue { return color; }
            // Compensate: divide by p_continue (done in caller via throughput)
        }

        // Random bounce (cosine-weighted hemisphere)
        let direction = cosine_weighted_hemisphere(&hit.normal, rng);
        let incoming = path_trace(&Ray::new(hit.point + hit.normal * 0.001, direction),
                                    scene, rng, depth + 1);
        color += hit.material.albedo * incoming;
        return color;
    }
    background_color()
}
```

For each pixel, average N paths:

```
  pixel_color = (1/N) * Σ path_trace(camera_ray_i, scene, rng, 0)
```

This single algorithm produces soft shadows, color bleeding, and mirror reflections (when you
add specular materials that choose the mirror direction instead of a random direction).

### Step 3: PPM Output

Write the final image as a PPM file — dead simple format, no libraries needed:

```
  P3
  width height
  255
  r g b  r g b  ...
  r g b  r g b  ...
```

See `code/main.rs` for the complete implementation that renders a scene with:
- A diffuse red sphere and a diffuse blue sphere on a white plane
- Soft shadows from an overhead area light (approximated by a bright emitting sphere)
- Color bleeding on the white floor
- A mirror sphere showing reflections

## Use It

Production path tracers don't differ in fundamentals — they differ in engineering:

| Feature | Our tracer | Production (pbrt, Mitsuba, Cycles) |
|---------|-----------|--------------------------------------|
| Acceleration structure | None (brute force) | BVH, Kd-tree, Embree |
| Sampling | Cosine-weighted | Multiple importance sampling (MIS) |
| Light sampling | None | Direct illumination (next event estimation) |
| Parallelism | None | SIMD, multi-thread, GPU |
| Denoising | None | OptiX, OIDN |
| Materials | Lambertian only | Microfacet, subsurface, volumetric |
| Spectral | RGB | Full spectral + wavelengths |

The core algorithm is identical. The engineering is what makes it fast.

**Next event estimation** (NEE) is the biggest practical improvement: instead of waiting for a
random bounce to hit a light, explicitly sample the light source at every bounce. This is the
standard technique in pbrt — see `pbrt/src/integrators/path.cpp`.

## Read the Source

- **pbrt-v4**: `src/pbrt/integrators/path.cpp` — The path integrator. Look at `PathIntegrator::Li()` to see
  how production code handles MIS, Russian roulette, and direct vs. indirect lighting.
- **Mitsuba 3**: `src/render/integrators/path.cpp` — Similar path tracer with different architecture.
  Compare how both handle throughput accumulation and Russian roulette.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`pathtracer_reference.md`** — A quick-reference card with path tracing pseudocode, sampling
  formulas, Russian roulette explanation, and variance reduction tips. Keep it handy for the
  capstone project.

## Exercises

1. **Easy** — Modify the scene to use different colored spheres. Observe how color bleeding
   changes. Render with 4 samples then 64 samples and compare the noise.
2. **Medium** — Add **next event estimation**: at each diffuse bounce, also shoot a shadow ray
   toward the light source. This combines direct and indirect illumination and dramatically
   reduces noise. This is how all production path tracers work.
3. **Hard** — Implement **bidirectional path tracing** (BDPT): trace paths from both the camera
   and the light, then connect sub-paths. This handles caustics much better than unidirectional
   path tracing. Reference: Veach's 1997 thesis.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Monte Carlo integration | "Random sampling to estimate integrals" | Estimate ∫f(x)dx ≈ (1/N)Σf(xᵢ)/p(xᵢ) where xᵢ∼p |
| Importance sampling | "Sample where it matters" | Choose p(x) proportional to f(x) to reduce variance |
| Path tracing | "Shoot random rays" | Monte Carlo integration of the rendering equation via random walks |
| Russian roulette | "Kill paths randomly" | Terminate paths with probability 1-p, divide survivors by p to stay unbiased |
| Cosine-weighted hemisphere | "Sample near the normal" | Generate directions with PDF = cos(θ)/π, concentrating samples toward the normal |
| Throughput | "Path weight" | The accumulated product of BRDF·cos(θ)/PDF along a path — how much light the path carries |
| Next event estimation | "Sample lights directly" | At each bounce, explicitly connect to light sources instead of waiting for random hits |
| Color bleeding | "Red wall tints the floor" | Diffuse interreflection — light bouncing off colored surfaces carries that color |

## Further Reading

- **Kajiya, 1986** — "The Rendering Equation," SIGGRAPH. The original paper that defined the problem.
- **Veach, 1997** — "Robust Monte Carlo Methods for Light Transport Simulation," PhD thesis. The definitive
  reference on Multiple Importance Sampling and bidirectional path tracing.
- **Pharr, Jakob & Humphreys, 2023** — *Physically Based Rendering* (4th ed.). Chapter 13 covers path
  tracing in detail. Free online at pbr-book.org.
- **NVIDIA OptiX Denoiser** — Production AI denoiser that takes noisy path-traced images and produces
  clean results. Uses temporal and spatial coherence.
- **Intel Open Image Denoise (OIDN)** — Open-source denoiser for ray tracing. Integrates with
  Blender Cycles, Mitsuba, and pbrt.