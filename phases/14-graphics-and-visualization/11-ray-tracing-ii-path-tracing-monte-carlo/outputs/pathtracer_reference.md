# Path Tracer Reference Card

Quick reference for path tracing, Monte Carlo integration, and related sampling techniques.

---

## Core Equation

**The Rendering Equation** (Kajiya, 1986):

```
L_o(p, ω_o) = L_e(p, ω_o) + ∫_Ω f(p, ω_i, ω_o) · L_i(p, ω_i) · cos(θ_i) dω_i
```

- `L_o` = outgoing radiance at point `p` in direction `ω_o`
- `L_e` = emitted radiance (nonzero only for light sources)
- `f` = BRDF (e.g., Lambertian: `ρ/π`)
- `L_i` = incoming radiance from direction `ω_i`
- `cos(θ_i)` = `dot(n, ω_i)` (Lambert's cosine law)
- `Ω` = hemisphere above surface normal

---

## Monte Carlo Integration

Estimate an integral by random sampling:

```
∫f(x) dx ≈ (1/N) Σᵢ f(xᵢ) / p(xᵢ)
```

- `N` = number of samples
- `xᵢ` = random samples drawn from distribution `p`
- `p(xᵢ)` = probability density at sample `xᵢ`

**Variance**: `O(1/√N)` — quadruple samples to halve noise.

---

## Importance Sampling

Choose `p(x)` to be proportional to `f(x)` when possible:

- If `p(x) ∝ f(x)`, then `f(x)/p(x) = constant` → variance drops to zero
- In practice: sample proportional to BRDF (cosine-weighted for Lambertian), or proportional to light source intensity
- **Multiple Importance Sampling (MIS)**: combine BRDF sampling and light sampling, weighting by power heuristic

---

## Cosine-Weighted Hemisphere Sampling

Generate a random direction on the hemisphere with PDF = `cos(θ)/π`:

```
Given r₁, r₂ ∈ [0, 1) uniform random:

  φ = 2π · r₂
  local_direction = (√r₁ · cos(φ),  √r₁ · sin(φ),  √(1 − r₁))

  // cos(θ) = √(1 − r₁) = z-component
  // sin(θ) = √r₁
```

To transform from local to world coordinates, build an orthonormal basis (ONB):

```
  w = normal
  u = normalize(cross(w, (0,1,0)))   // or (1,0,0) if normal ≈ (0,1,0)
  v = cross(w, u)

  world_direction = u · local.x + v · local.y + w · local.z
```

**Why this works**: The mapping concentrates samples near the normal (pole) where `cos(θ)` is large.

**Throughput simplification** for Lambertian surfaces:

```
throughput_new = throughput_old × (BRDF · cos(θ)) / PDF
              = throughput_old × (ρ/π · cos(θ)) / (cos(θ)/π)
              = throughput_old × ρ
```

Each bounce just multiplies throughput by the surface albedo. No extra terms.

---

## Path Tracing Algorithm (Pseudocode)

```
function path_trace(ray, scene, depth):
    hit = scene.intersect(ray)
    if no hit:
        return background_color

    color = hit.material.emission

    if depth > MAX_DEPTH:
        return color

    // ----- Material branch -----
    if hit.material is mirror:
        reflected = ray.direction.reflect(hit.normal)
        return color + hit.material.albedo *
               path_trace(Ray(hit.point, reflected), scene, depth+1)

    // ----- Diffuse material -----
    // Russian roulette (after minimum depth)
    if depth >= RR_MIN_DEPTH:
        p_continue = min(max(albedo_r, albedo_g, albedo_b), 1.0)
        if random() > p_continue:
            return color
    else:
        p_continue = 1.0

    // Random bounce via cosine-weighted sampling
    direction = cosine_weighted_hemisphere(hit.normal)
    incoming = path_trace(Ray(hit.point, direction), scene, depth+1)

    // Accumulate (albedo simplification for Lambertian)
    color += hit.material.albedo * incoming / p_continue
    return color
```

**Per pixel**: average `N` independent paths:

```
pixel_color = (1/N) × Σ path_trace(camera_ray_i, scene, 0)
```

---

## Russian Roulette

Terminate paths probabilistically after `RR_MIN_DEPTH` bounces:

| Depth | Action |
|-------|--------|
| `< RR_MIN_DEPTH` | Always continue (`p_continue = 1`) |
| `≥ RR_MIN_DEPTH` | Continue with `p_continue = min(max(albedo), 1.0)`, terminate with `1 − p_continue` |

**Unbiased**: `E[result] = p_continue × E[contrib/p_continue] + (1−p_continue) × 0 = E[contrib]`

**Why use it**: Long paths contribute diminishingly small light but cost the same as short paths. Russian roulette focuses computation where it matters.

---

## Variance Reduction Tips

| Technique | How it helps | When to use |
|-----------|-------------|------------|
| **More samples** | Direct O(1/√N) reduction | Always, but expensive |
| **Cosine-weighted sampling** | Eliminates BRDF×cos(θ) variance | Lambertian surfaces |
| **Next Event Estimation (NEE)** | Explicitly samples light sources | All diffuse scenes |
| **Multiple Importance Sampling** | Combines BRDF + light sampling | Complex scenes |
| **Stratified sampling** | Ensures samples cover the domain uniformly | Jittered pixel samples |
| **Russian roulette** | Saves time on dim paths | Depth > RR_MIN_DEPTH |
| **Denoising (post-process)** | Exploits spatial coherence of noise | Final output (OptiX, OIDN) |

---

## Key Formulas at a Glance

```
Monte Carlo:              L̂ = (1/N) Σ f(xᵢ)/p(xᵢ)
Lambertian BRDF:          f = ρ/π
Cosine-hemisphere PDF:    p(ω) = cos(θ)/π
Direction sampling:       d = (√r₁·cos(2πr₂), √r₁·sin(2πr₂), √(1−r₁))
Throughput (Lambertian):  Tᵢ₊₁ = Tᵢ · ρ
Russian roulette:         survive with prob p, divide by p; E[·] unchanged
Convergence:              σ ∝ 1/√N
```

---

## Common Pitfalls

1. **Forgetting to offset ray origin** — Self-intersection. Offset by `EPSILON` along normal.
2. **Normal pointing wrong way** — Check `dot(ray.dir, normal) < 0`; flip if needed.
3. **Not compensating for Russian roulette** — Must divide by `p_continue` to stay unbiased.
4. **Using `rand()` without proper seeding** — Correlated sequences cause patterns. Use per-pixel seeds.
5. **Gamma correction** — Linear values must be gamma-corrected (pow 1/2.2) for display.
6. **HDR scene emission** — Use Reinhard tone mapping or filmic curves before gamma.

---

## Production Extensions

| Feature | What it does |
|---------|-------------|
| **BVH / Kd-tree** | Accelerate ray-scene intersection from O(n) to O(log n) |
| **Next Event Estimation (NEE)** | Sample lights directly at each bounce, massively reduces noise |
| **Multiple Importance Sampling** | Weight BRDF and light samples by power heuristic |
| **Bidirectional Path Tracing** | Trace from camera and lights, connect sub-paths |
| **Metropolis Light Transport** | Mutate existing good paths, excellent for caustics |
| **Spectral rendering** | Trace full wavelengths instead of RGB for dispersion |
| **Volumetric path tracing** | Handle participating media (fog, clouds, smoke) |
| **AI Denoising** | Post-process noisy render to clean result (OptiX, OIDN) |