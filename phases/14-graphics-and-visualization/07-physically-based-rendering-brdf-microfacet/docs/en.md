# Physically Based Rendering — BRDF, Microfacet

> Light doesn't bounce off surfaces the way Phong guessed it did. Microfacet theory tells us why — and how to get it right.

**Type:** Learn
**Languages:** GLSL, Rust
**Prerequisites:** Phase 14 lessons 01–06
**Time:** ~90 minutes

## Learning Objectives

- Derive the rendering equation and explain every term in it.
- Define BRDF formally and state its energy-conservation and reciprocity constraints.
- Explain why microfacet theory models real surfaces better than empirical models like Phong or Blinn-Phong.
- Implement the Cook-Torrance specular BRDF from first principles: GGX NDF, Smith geometry, Schlick Fresnel.
- Distinguish metallic from dielectric materials in a PBR pipeline.
- Render a sphere with PBR materials (gold, plastic, rubber, chrome) using your own CPU evaluator.

## The Problem

You've been rendering spheres with Phong shading. They look okay — bright highlights, dark shadows — but something is *off*. Red plastic doesn't look like red plastic. Gold doesn't look like gold. Slide the "shininess" slider and the highlight changes size, but it never looks *right*.

The core issue: **Phong (and Blinn-Phong) are not grounded in physics.**

They have no energy conservation — crank up the specular exponent and you concentrate the same amount of light into a smaller area, violating conservation. They have no Fresnel effect — view a surface at a grazing angle and it should reflect more light, but Phong doesn't know that. They can't distinguish metal from plastic — both are just "shiny" with an exponent.

PBR fixes all of this. It starts from the rendering equation, builds a BRDF that respects physical laws, and produces images that look *correct* under any lighting condition.

## The Rendering Equation

The rendering equation, formulated by James Kajiya in 1986, is the single equation that governs all light transport:

```
Lo(p, ωo) = Le(p, ωo) + ∫_Ω fr(p, ωi, ωo) · Li(p, ωi) · (n · ωi) dωi
```

Where:

| Symbol | Meaning |
|--------|---------|
| `Lo(p, ωo)` | Outgoing radiance at point p toward direction ωo |
| `Le(p, ωo)` | Emitted radiance (for light sources, else 0) |
| `fr(p, ωi, ωo)` | The BRDF — the heart of this lesson |
| `Li(p, ωi)` | Incoming radiance from direction ωi at point p |
| `n · ωi` | The cosine term — light arriving at an angle is spread over more area |
| `∫_Ω` | Integral over the hemisphere of directions above the surface |

The rendering equation is recursive: `Lo` depends on `Li`, which depends on `Lo` at other points. Path tracing solves this recursively. Rasterization with PBR solves a *direct lighting* approximation — one bounce, a few light sources.

### The Cosine Term

```
        sun (directly above)
          |
          | Li
          |
    -------|-------  <- surface
           n   (normal, pointing up)

    When light arrives at angle θ from normal:
    Same beam of light covers MORE area.
    Energy per unit area ∝ cos(θ) = n · ωi
```

A flashlight pointed straight down illuminates a small circle. Tilt it 45° and the same light spreads over a larger ellipse, so each point receives less light. The `(n · ωi)` term captures this geometric fact.

## The BRDF

**BRDF** = Bidirectional Reflectance Distribution Function. It answers one question:

> Given light arriving from direction ωi, how much is reflected toward direction ωo?

```
       ωi (incoming)        ωo (outgoing)
        \                   /
         \                 /
          \       n       /
           \      |      /
  ---------surface--------->

  fr(ωi, ωo) = dLo(ωo) / (dEi(ωi))

  where dEi(ωi) = Li(ωi) · cosθi dωi
```

The BRDF is a function of *two* directions — it's bidirectional. It takes `ωi` (where light comes from) and `ωo` (where you're looking from) and returns a scalar: the ratio of reflected radiance to incident irradiance. Units: **1/sr** (per steradian).

### Two Constraints on Any Valid BRDF

**1. Energy Conservation:**

```
∫_Ω fr(ωi, ωo) · cosθo dωo  ≤  1   for all ωi
```

A surface can never reflect more energy than it receives. Phong violates this — a narrow specular lobe with high intensity can concentrate more energy than comes in. PBR enforces this by construction.

**2. Helmholtz Reciprocity:**

```
fr(ωi, ωo) = fr(ωo, ωi)
```

If you swap the light and the eye, the result is the same. Light paths are reversible. This is built into the mathematics of microfacet theory (all three Cook-Torrance terms are symmetric or nearly so in i/o).

**3. Non-negativity:**

```
fr(ωi, ωo) ≥ 0
```

Negative reflectance doesn't exist in nature.

## Microfacet Theory

The key insight: **a "smooth" surface isn't smooth.** At microscopic scale, every surface is a landscape of tiny facets, each a perfect mirror.

```
  Macro view: smooth surface    Micro view: microfacets
  
  ________________________      /\/\\/\//\/\/\\/\/\\/\//\/
                                ^ tiny mirrors, each has
                                  its own normal mh

  The BRDF emerges from STATISTICS of these microfacets.
  Different distributions of normals → different appearance.
```

Each microfacet reflects light according to the law of perfect mirror reflection. But we don't track individual facets — that's computationally infeasible. Instead, we use a **statistical distribution** of microfacet normals, and the BRDF is the aggregate result.

### Three Things That Can Go Wrong

Not every microfacet contributes to the reflection you see:

```
  1. MASKING    2. SHADOWING    3. INTERREFLECTION

  eye         eye             eye
   \           \               \
    \           \    ______    /
     \           \  /      \  /
  ____\_____  ____\/shadow_\/____
  facet hits   facet in      light bounces
  back of      shadow from   between facets
  next facet   neighbor      (usually ignored)
```

- **Masking:** Outgoing direction is blocked by another facet.
- **Shadowing:** Incoming direction is blocked by another facet.
- **Interreflection:** Light bounces between facets before escaping. Usually ignored in the standard model for performance.

The **geometry function** G(ωi, ωo) models masking and shadowing. It returns a value in [0, 1] — the fraction of microfacets that are both lit and visible.

## The Cook-Torrance Specular BRDF

The dominant PBR specular model, proposed by Robert Cook and Kenneth Torrance in 1982:

```
        D(mh) · G(ωi, ωo) · F(ωo, mh)
fr = ─────────────────────────────────────
            4 · (n · ωi) · (n · ωo)
```

Three multiplicative terms, each with a physical meaning:

| Term | Name | Models |
|------|------|--------|
| D(mh) | Normal Distribution Function | What fraction of microfacets have normal mh? |
| G(ωi, ωo) | Geometry Function | What fraction are both lit and visible? |
| F(ωo, mh) | Fresnel Function | How much does each facet reflect (vs. absorb)? |

The denominator `4·(n·V)·(n·L)` is a normalization factor that ensures energy conservation. It converts from microfacet-local to macro-surface-local coordinates.

### GGX / Trowbridge-Reitz NDF

The **GGX** (Trowbridge-Reitz) Normal Distribution Function describes how microfacet normals are distributed around the macrosurface normal:

```
              α²
D_GGX(mh) = ───────────────────────
             π · ((n·mh)² · (α² - 1) + 1)²
```

Where:
- `mh` is the half-vector: `mh = normalize(ωi + ωo)` (the direction a perfect mirror would need to reflect ωi toward ωo)
- `α = roughness²` is the squared roughness parameter, 0→1
- `n·mh` is the cosine of the angle between macrosurface normal and microfacet normal

**Key properties:**

```
  roughness α = 0.0:  D peaks sharply → perfect mirror
  roughness α = 0.3:  D is broad → visible highlight but spread
  roughness α = 1.0:  D is uniform → fully diffuse (Lambertian limit)

  GGX has a LONGER TAIL than Beckmann (the other common NDF).
  This means: more energy at grazing highlights → realistic "halo"
  around bright specular spots that Beckmann can't produce.
```

**Worked example:** Compute D_GGX for n·mh = 0.707, roughness = 0.5:

```
  α = 0.25,  α² = 0.0625
  (n·mh)² = 0.5
  denom = (0.5 · (0.0625 - 1) + 1)² = (0.5 · -0.9375 + 1)² = (0.53125)² = 0.28223
  D = 0.0625 / (π · 0.28223) ≈ 0.0704
```

### Smith Geometry Function (GGX)

The Smith model factors masking and shadowing independently:

```
G_Smith(ωi, ωo) = G1(ωi) · G1(ωo)
```

Each term uses the **Schlick-GGX** approximation:

```
              n·v
G1_Schlick(v) = ──────────────────
                (n·v)(1 - k) + k
```

Where `k` depends on the use case:
- **Direct lighting:** k = (roughness + 1)² / 8
- **IBL/environment lighting:** k = roughness² / 2

For the full geometry term, we use the combined form to avoid artifacts:

```
G_Smith(ωi, ωo) = ──────────────────────────────────────
                   G1(ωi) · G1(ωo)              (n·ωi)(n·ωo)
                   ────────────────────────────────────────────  ×  ──────────────
                   G1(ωi) + G1(ωo) - G1(ωi)·G1(ωo)   (n·ωi)(n·ωo)
```

More practically, using Schlick-GGX with the visibility term (V = G / (4·N·V·N·L)):

```
V_SmithGGXCorrelated(n, v, l, α) =

  a  = α²
  ggx_v = n·v · sqrt(a + (n·l)² · (1 - a))
  ggx_l = n·l · sqrt(a + (n·v)² · (1 - a))

  V = 0.5 / (ggx_v + ggx_l)
```

This correlated form prevents the "black highlight" artifact that the separable Smith form can produce.

### Fresnel: Schlick's Approximation

Real Fresnel describes how reflectance varies with viewing angle. Schlick (1994) provides an excellent polynomial approximation:

```
F_Schlick(cosθ, F0) = F0 + (1 - F0) · (1 - cosθ)⁵
```

Where:
- `cosθ = max(n·ωo, 0)` or `n·h` — the angle between view and half-vector
- `F0` = reflectance at normal incidence (0° grazing angle)

At normal incidence (cosθ = 1): `F = F0` (the material's inherent reflectance)
At grazing angles (cosθ → 0): `F → 1` (everything becomes a mirror at grazing angles)

This is the edge-darkening-you-see-in-a-window effect turned upside down: surfaces reflect MORE at grazing angles.

**F0 values for common materials:**

| Material | F0 (linear RGB) | Notes |
|----------|-----------------|-------|
| Water | (0.02, 0.02, 0.02) | Very low |
| Plastic | (0.03–0.05, 0.03–0.05, 0.03–0.05) | Dielectric, achromatic |
| Glass | (0.04, 0.04, 0.04) | Low |
| Gold | (1.00, 0.76, 0.34) | Metallic — tinted! |
| Copper | (0.95, 0.63, 0.54) | Metallic — tinted |
| Aluminum | (0.91, 0.92, 0.92) | Metallic — nearly achromatic |
| Iron | (0.56, 0.57, 0.58) | Metallic — slightly tinted |
| Chrome | (0.55, 0.55, 0.55) | Metallic — achromatic |

## The Complete PBR BRDF: Metallic-Roughness Workflow

The Disney/Brickford metallic-roughness model combines diffuse and specular:

```
fr = (1 - metallic) · f_lambert + f_cook_torrance

where:
  f_lambert = albedo / π

  f_cook_torrance = D(h) · G(ωi, ωo) · F(ωo, h)
                    ─────────────────────────────────
                          4 · (n·ωi) · (n·ωo)
```

The `(1 - metallic)` term is the key to understanding PBR's metallic/dielectric split:

**Dielectric (metallic = 0):**
- Has diffuse reflection (light enters the material, scatters, exits — that's color)
- Has white/achromatic specular with F0 ≈ 0.04
- Example: red plastic → red diffuse, white specular highlight

**Metal (metallic = 1):**
- Has NO diffuse (light is absorbed by free electrons, doesn't scatter inside)
- Has COLORED specular with F0 = the metal's albedo
- Example: gold → no diffuse term, gold-colored specular reflection

```
  Dielectric (metallic=0)         Metal (metallic=1)

  F0 = lerp(0.04, albedo, 0)     F0 = lerp(0.04, albedo, 1)
     = 0.04                           = albedo (gold: 1.0, 0.76, 0.34)

  diffuse = albedo                diffuse = 0
  specular = F0..1 (achromatic)   specular = albedo..1 (tinted)
```

### Roughness

Roughness (0→1) controls the NDF width:

```
  roughness = 0.0: perfect mirror, D→∞ at n=mh, 0 elsewhere
  roughness = 0.5: visible specular highlight, moderate spread
  roughness = 1.0: maximum spread, highlight disappears into lambert
```

The square of roughness (`α = roughness²`) is used in the NDF because the relationship is non-linear. A roughness of 0.5 *looks* about halfway between mirror and matte, but the α = 0.25 value in the NDF produces a well-defined lobe.

### Worked Example: Computing Cook-Torrance for Gold

Let's evaluate the specular BRDF for gold at a specific point:

```
Given:
  Material: Gold (metallic=1, roughness=0.3, albedo=(1.0, 0.76, 0.34))
  n = (0, 0, 1), v = (0.3, 0, 0.954), normalized
  Light from directly above: l = (0, 0, 1)

Step 1: Half-vector
  h = normalize(v + l) = normalize((0.3, 0, 1.954)) ≈ (0.151, 0, 0.9885)

Step 2: Dot products
  NdotL = n·l = 1.0
  NdotV = n·v = 0.954
  NdotH = n·h = 0.9885
  VdotH = v·h ≈ 0.3·0.151 + 0.954·0.9885 ≈ 0.989

Step 3: GGX NDF (α = 0.09)
  α² = 0.0081
  denom = π · (0.9885² · (0.0081 - 1) + 1)² ≈ π · (0.9772 · -0.9919 + 1)²
       ≈ π · (0.0308)² ≈ 0.00298
  D = 0.0081 / 0.00298 ≈ 2.72

Step 4: Fresnel (F0 for gold = albedo = (1.0, 0.76, 0.34))
  F_R = 1.0 + (1.0 - 1.0) · (1 - 0.989)⁵ ≈ 1.0    (metal → F→1 at any angle)
  F_G = 0.76 + (0.24) · (0.011)⁵ ≈ 0.76
  F_B = 0.34 + (0.66) · (0.011)⁵ ≈ 0.34

Step 5: Geometry (Schlick-GGX, direct light, k = (0.3+1)²/8 ≈ 0.211)
  G1(L) = 1.0 / (1.0·(1-0.211) + 0.211) ≈ 1.0
  G1(V) = 0.954 / (0.954·0.789 + 0.211) ≈ 0.954/0.965 ≈ 0.989
  G = G1(L)·G1(V) ≈ 0.989

Step 6: Cook-Torrance specular
  fspec = D·G·F / (4·NdotV·NdotL)
        = 2.72 · 0.989 · (1.0, 0.76, 0.34) / (4 · 0.954 · 1.0)
        ≈ (0.707, 0.537, 0.240)

Step 7: Final BRDF (metallic=1 → no diffuse)
  f = (1-1)·albedo/π + fspec = (0.707, 0.537, 0.240)
```

The gold specular clearly shows: at near-normal incidence with near-direct lighting, gold reflects with its characteristic warm tint.

## Build It

### Step 1: Minimal PBR — GLSL Fragment Shader

The GLSL shader (see `code/main.glsl`) implements a complete PBR pipeline:

1. Compute half-vector H = normalize(V + L)
2. Evaluate GGX NDF at N·H
3. Evaluate Schlick-GGX geometry term
4. Evaluate Schlick Fresnel
5. Combine into Cook-Torrance specular
6. Mix with Lambertian diffuse weighted by (1 - metallic)
7. Apply tone mapping (Reinhard) and gamma correction

### Step 2: Minimal PBR — Rust CPU Evaluator

The Rust code (see `code/main.rs`) implements the same pipeline on the CPU:

1. Define Vec3 struct with all needed operations
2. Implement `ggx_ndf`, `smith_ggx`, `schlick_fresnel` as standalone functions
3. Implement `cook_torrance_brdf` that combines all three
4. Render a sphere to a PPM file with multiple material presets
5. The sphere shows gold, chrome, plastic, and rubber side by side

## Use It

In production, PBR is implemented in every major engine:

- **Unreal Engine** uses the Disney BRDF with GGX and Smith-Schlick (Epic's 2013 SIGGRAPH presentation essentially popularized PBR in games)
- **Unity** uses the same metallic-roughness workflow with `UNITY_BRDF_PBS`
- **Blender's Cycles** uses a GGX-based Principled BSDF for production rendering
- **filament** (Google's open-source PBR library) provides a reference implementation in `libs/filabridge/src/Material.cpp` and `shaders/src/brdf.fs`

Compare our implementation against filament's:

| Aspect | Our version | Filament |
|--------|-------------|----------|
| NDF | GGX | GGX + LTC for area lights |
| Geometry | Smith-Schlick correlated | Smith-GGX correlated (same) |
| Fresnel | Schlick | Schlick (same) |
| Diffuse | Lambert | Lambert + Disney diffuse (sheen) |
| Distribution | N/A | Multiscattering compensation |
| Visibility | V = G/(4·N·V·N·L) | Same + energy compensation |

The biggest difference: our version ignores **multiscattering** (light bouncing between microfacets multiple times). For rough metals, this means our BRDF loses energy. Filament and UE5 add a correction term to preserve total energy at high roughness.

## Read the Source

- **Google Filament BRDF:** `shaders/src/brdf.fs` in the filament repository — the reference PBR implementation. Also see the accompanying Material Guide PDF.
- **Epic Games PBR Note:** "Real Shading in Unreal Engine 4" (SIGGRAPH 2013) — the paper that defined the metallic-roughness workflow for games.
- **Disney BRDF:** Brent Burley's 2012 SIGGRAPH paper "Physically Based Shading at Disney" — the origin of the principled BRDF.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. It is:

- **A PBR reference card** (`pbr_reference.md`) with all equations (GGX NDF, Smith G, Schlick Fresnel, Cook-Torrance), parameter tables for common materials, and a quick-lookup for F0 values. Keep it next to your keyboard when writing shaders.

## Exercises

1. **Easy** — Modify the Rust code to add a new material (copper). Use F0 = (0.95, 0.63, 0.54), roughness = 0.25. Observe how the warm tint appears in the specular highlight.
2. **Medium** — Add a second light source to the GLSL shader. The rendering equation integrates over *all* incoming radiance, so multiple lights just means summing the BRDF evaluation for each light direction.
3. **Hard** — Implement multiscattering compensation. When microfacets bounce light between each other before it escapes, the simple G term underestimates energy. Research the Kulla-Conty energy compensation term and add it to the Rust evaluator.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| BRDF | "The shading model" | A 4D function fr(ωi, ωo) that maps incoming+outgoing direction to reflectance ratio. Not a "model" — it's the *definition* of surface reflectance. |
| Microfacet | "Bumpy surface thing" | A statistical model of surface roughness: the surface is composed of tiny perfect mirrors, and the BRDF is the integral of their combined behavior. |
| NDF | "The highlight shape" | Normal Distribution Function — what fraction of microfacets point in a given direction. Controls specular highlight width and shape. |
| Roughness | "How shiny it is" | A parameter (0→1) that controls the statistical spread of microfacet normals. 0 = perfect mirror, 1 = maximum spread. Squared before use in the NDF. |
| Metallic | "Metal vs not metal" | A binary-ish parameter that controls whether the surface has diffuse reflection (0 = yes, dielectric) or not (1 = metal, specular only, with colored F0). |
| F0 | "Base reflectivity" | Fresnel reflectance at normal incidence (0°). For dielectrics ≈ 0.04, for metals = the base color. Determines how bright the specular is head-on. |
| Energy Conservation | "Can't make more light than comes in" | ∫ fr · cosθ dω ≤ 1. The BRDF can never reflect more energy than arrives. Phong violates this; Cook-Torrance with proper NDF and G satisfies it. |

## Further Reading

- Kajiya, James T. "The Rendering Equation." SIGGRAPH 1986. — *The foundational paper.*
- Cook, Robert L. and Torrance, Kenneth E. "A Reflectance Model for Computer Graphics." SIGGRAPH 1981. — *The Cook-Torrance model.*
- Walter, Bruce et al. "Microfacet Models for Refraction through Rough Surfaces." EGSR 2007. — *GGX NDF introduced.*
- Schlick, Christophe. "An Inexpensive BRDF Model for Physically-Based Rendering." Computer Graphics Forum 1994. — *Schlick Fresnel and Schlick-GGX.*
- Burley, Brent. "Physically Based Shading at Disney." SIGGRAPH 2012. — *The Disney principled BRDF that started the metallic-roughness workflow.*
- Karis, Brian. "Real Shading in Unreal Engine 4." SIGGRAPH 2013. — *Epic's adaptation of Disney's BRDF for real-time rendering.*
- Heitz, Eric. "Understanding the Masking-Shadowing Function in Microfacet-Based BRDFs." JCGT 2014. — *Definitive reference on Smith G and its variants.*