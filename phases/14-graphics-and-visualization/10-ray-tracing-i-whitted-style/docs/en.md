# Ray Tracing I — Whitted Style

> Turner Whitted's 1980 paper showed that recursive ray tracing could produce
> reflections, refractions, and shadows — all from a simple recursive formula.

**Type:** Learn
**Languages:** Rust, C++
**Prerequisites:** Phase 14 lessons 01–09
**Time:** ~90 minutes

## Learning Objectives

- Define a ray in parametric form: `P = O + tD` and explain what `t` represents.
- Derive and implement ray-sphere and ray-plane intersection tests.
- Implement shadow rays to determine whether a point is lit or in shadow.
- Implement mirror reflection using the formula `R = D − 2(D·N)N`.
- Implement Snell's law for refraction: `n₁ sin θ₁ = n₂ sin θ₂`.
- Trace the full Whitted recursion: local color + reflected color + refracted color.
- Explain why Whitted-style ray tracing produces sharp reflections and hard shadows.
- Terminate recursion correctly (max depth, miss → background).

## The Problem

Phase 14's capstone requires a path tracer. Before you can distribute rays stochastically
(path tracing), you must understand deterministic recursive ray tracing — the Whitted style.
Without it, you cannot produce reflections off mirrors, refractions through glass, or
shadows cast by point lights. Concretely, if you skip this, your renderer will produce
flat-shaded spheres with no inter-object lighting effects.

## The Concept

### Rays in Parametric Form

A ray has an **origin** `O` and a **direction** `D` (unit vector). Every point on the ray is:

```
P(t) = O + t·D
```

where `t` is a real number:
- `t < 0` : behind the origin (ignore for visibility)
- `t = 0` : at the origin itself
- `t > 0` : in front of the origin along `D`

The "nearest intersection" is the smallest positive `t`.

```
   O ----t₁----> P₁  (first hit)
   O ------t₂------> P₂  (farther hit)
   
   We pick t₁ because it's the nearest positive t.
```

### Ray-Sphere Intersection

A sphere with center `C` and radius `r` satisfies: `|P − C|² = r²`.

Substituting `P = O + tD`:

```
(O + tD − C)·(O + tD − C) = r²
```

Let `L = O − C`. Expand:

```
(D·D)t² + 2(L·D)t + (L·L − r²) = 0
```

This is a quadratic `at² + bt + c = 0` where:
- `a = D·D` (1 if D is normalized)
- `b = 2(L·D)`
- `c = L·L − r²`

**Discriminant** `Δ = b² − 4ac`:
- `Δ < 0` : no intersection
- `Δ = 0` : tangent (one hit)
- `Δ > 0` : two hits; solve for `t`:

```
t = (−b ± √Δ) / 2a
```

Pick the smallest positive `t`.

**Worked example:** Origin at (0,0,0), direction (0,0,1), sphere center (0,0,5), radius 1.

```
L = (0,0,0) − (0,0,5) = (0,0,−5)
a = 1, b = 2(0+0−5) = −10, c = 25 − 1 = 24
Δ = 100 − 96 = 4
t = (10 ± 2) / 2 = 6 or 4
Nearest positive t = 4 → hit point at (0,0,4)
```

### Ray-Plane Intersection

A plane defined by point `Q` on the plane and normal `N`. The plane equation is:
`(P − Q)·N = 0`, meaning `P·N = d` where `d = Q·N`.

Substituting `P = O + tD`:

```
(O + tD)·N = d
O·N + t(D·N) = d
t = (d − O·N) / (D·N)
```

If `D·N = 0` the ray is parallel (no hit). Otherwise, check `t > 0`.

### Shadow Rays

At a hit point, we want to know: can this point "see" the light?

```
   Light
    *  ↓ L = (light_pos − hit_point).normalized()
    |  ↓
    |  ↓
    * hit_point
```

Cast a **shadow ray** from the hit point toward the light. If any object blocks it
(`t` in `[ε, distance_to_light]`), the point is in shadow for that light.

`ε` (epsilon, e.g., 0.001) prevents self-intersection — the hit point sits on the
surface and would intersect itself at `t ≈ 0`.

```
   Light *                        Light *
          |                              |
    sphere |                      (blocked by 2nd sphere)
          |                              |
     hit_point •               hit_point • ← IN SHADOW
```

### Mirror Reflection

Given incident direction `D` and surface normal `N` (pointing outward):

```
R = D − 2(D·N)N
```

The reflected ray starts at the hit point and goes in direction `R`. The color at
this point includes whatever the reflected ray "sees":

```
color = local_color + kr * trace(reflected_ray, depth − 1)
```

where `kr` is the reflection coefficient (0 = matte, 1 = perfect mirror).

```
         \  N  /
          \ | /
     D →   \|/   ← hit point
    --------+--------
           / \
          /   \
         R →   (reflected ray)
```

### Refraction (Snell's Law)

Snell's law: `n₁ sin θ₁ = n₂ sin θ₂`

The transmitted ray direction:

```
η = n₁ / n₂
cos θ_i = −N·D
cos θ_t = √(1 − η²(1 − cos²θ_i))
T = η·D + (η·cos θ_i − cos θ_t)·N
```

If the expression under the square root is negative → **total internal reflection**.
No refracted ray is produced; all energy goes to reflection.

### The Whitted Illumination Model

For each hit point, the final color is:

```
color = local_illumination + kr * trace(reflected_ray, depth−1)
                      + kt * trace(refracted_ray, depth−1)
```

**Local illumination** uses the Phong model:

```
local = ambient + Σ_lights [ shadow_test(l) * (diffuse + specular) ]
```

Where:
- `ambient = ka * object_color`
- `diffuse = kd * object_color * max(0, N·L)`
- `specular = ks * max(0, R·V)^n`
- `shadow_test(l)` is 0 if a shadow ray to light `l` is blocked, 1 otherwise

### Recursive Termination

The recursion terminates when:
1. **Max depth reached** (e.g., depth = 0) → return background color
2. **Ray misses all objects** → return background color
3. **Hit a non-reflective, non-refractive surface** → only local color

Typical max depths: 3–5. Each reflection/refraction spawns one more ray, so
5 levels = up to 2⁵ = 32 rays per pixel (one reflected + one refracted per hit).

### Why "Whitted Style" Produces Sharp Effects

Whitted-style tracing casts **exactly one** reflected ray and **exactly one**
refracted ray per hit. No stochastic sampling, no area lights, no soft shadows.
This produces:
- **Hard shadows** (point light → binary in/out of shadow)
- **Sharp reflections** (mirror direction → no blur)
- **Sharp refractions** (Snell's law → no caustic blur)

Path tracing (a later lesson) replaces the single reflected ray with many
stochastically-sampled rays, producing soft shadows and glossy reflections.

### Ray Generation from Camera

For each pixel `(i, j)` in the output image:

```
1. Map pixel to normalized device coordinates
2. Compute ray direction from camera through that pixel
3. Trace the ray

   Camera at eye position E, looking at target T.
   FOV defines the image plane size.
   
   +---+---+---+
   | . | . | . |  ← each pixel = one ray (no AA yet)
   +---+---+---+
   | . | . | . |
   +---+---+---+
         |
         | ray direction
         |
         E (eye)
```

## Build It

### Step 1: Minimal Version — Single Sphere, No Shadows

Define `Ray`, `Vec3`, `Sphere`, and trace a ray to produce a 200×200 PPM image
of a sphere shaded with diffuse + ambient only.

### Step 2: Full Whitted Tracer

Add:
- Shadow rays (hard shadows)
- Mirror reflection (recursive)
- A ground plane
- Multiple spheres
- Multiple lights
- `trace()` recursion with depth counter
- PPM output at 800×600

The code in `main.cpp` and `main.rs` implements Step 2.

## Use It

Production renderers that implement Whitted-style tracing (or its descendants):

- **pbrt** (Physically Based Rendering Toolkit) — Chapter 1 traces Whitted-style rays
  as the starting point before adding Monte Carlo sampling.
  See `src/integrators/whitted.cpp` in the pbrt-v4 source.

- **Ray Tracing in One Weekend** (Shirley, 2020) — builds a path tracer incrementally,
  starting from the same ray-sphere intersection we derived above.

Key differences between our minimal tracer and production:
- Production uses axis-aligned bounding boxes (AABBs) for acceleration; we test every object.
- Production uses proper tone mapping (Reinhard, ACES); we clamp to [0,255].
- Production uses stratified or Monte Carlo sampling for anti-aliasing; we sample once per pixel.
- Production handles texturing, area lights, and media; we keep it simple.

## Read the Source

- pbrt-v4: `src/integrators/whitted.cpp` — Compare the `Li()` method against our
  `trace()` function. Note how pbrt separates the integrator from the geometry.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **raytracer_reference.md** — A quick-reference card with ray-object intersection
  formulas, Whitted algorithm pseudocode, and shadow ray logic. Keep this handy for
  the path tracer capstone.

## Exercises

1. **Easy** — Modify the tracer to render a scene with three spheres of different
   colors and verify that shadows appear on the ground plane.

2. **Medium** — Add a glass sphere (kr = 0.1, kt = 0.9, ior = 1.5) and observe
   refraction through it. Verify total internal reflection when looking at steep angles.

3. **Hard** — Replace the single shadow ray with 4 jittered shadow rays aimed at
   a small area light. Observe how the shadow boundary softens (this bridges toward
   path tracing).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Ray | "a line in 3D" | A half-line: origin + t·direction for t > 0 |
| Discriminant | "the thing under the square root" | b²−4ac; tells you if the ray misses, grazes, or pierces the sphere |
| Shadow ray | "ray to light" | A test ray from hit point to light; if blocked, surface is in shadow |
| Reflection coefficient kr | "how mirror-y" | Fraction of incoming light reflected (0=matte, 1=perfect mirror) |
| Whitted-style | "classic ray tracing" | Recursive, deterministic: one reflected + one refracted ray per hit |
| Total internal reflection | "light bouncing inside glass" | When Snell's law yields imaginary cos(θ_t); all light reflects |
| PPM | "that image format" | Portable Pixmap — simplest image format: header + RGB bytes, trivial to write |

## Further Reading

- Whitted, T. (1980). "An Improved Illumination Model for Shaded Display."
  *Communications of the ACM*, 23(6), 343–349. — The original paper.
- Shirley, P. (2020). *Ray Tracing in One Weekend*. — Builds a path tracer from scratch.
- Pharr, M., Jakob, W., & Humphreys, G. (2016). *Physically Based Rendering* (3rd ed.). — Chapter 1 traces Whitted-style rays.
- Glassner, A. (1989). *An Introduction to Ray Tracing*. — Classic reference for intersection mathematics.