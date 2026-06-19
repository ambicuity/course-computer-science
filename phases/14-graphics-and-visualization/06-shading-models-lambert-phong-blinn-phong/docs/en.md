# Shading Models — Lambert, Phong, Blinn-Phong

> How we turn geometry into light — the three equations behind every shaded pixel on screen.

**Type:** Learn
**Languages:** GLSL, Rust
**Prerequisites:** Phase 14 lessons 01–05 (vectors, dot products, ray-sphere intersection)
**Time:** ~60 minutes

## Learning Objectives

- Derive Lambert's cosine law from first principles and explain why it produces flat-looking results.
- Implement the Phong reflection model (ambient + diffuse + specular) and identify its computational bottleneck.
- Explain the Blinn-Phong half-vector optimization and why it is more correct at grazing angles.
- Distinguish flat, Gouraud, and Phong shading (per-face, per-vertex, per-pixel).
- Reason about energy conservation: why a Lambert surface reflects at most 1/π of incoming irradiance.
- Understand that all three models are approximations of the full rendering equation.

## The Problem

You have a sphere. You have a light. You know where the camera is. Which color goes in which pixel?

Without a shading model, your renderer is flat — every surface gets one color, no sense of form or depth. The entire difference between a "looks like a gray circle" and "looks like a shiny ball" comes down to three equations and how you apply them.

## The Concept

### 1. Lambert's Cosine Law (Diffuse Reflection)

A perfect diffuse surface scatters incoming light equally in all directions. How much light arrives at a point depends on the angle of incidence.

```
        Light
          \  |  /
           \ | /
         N  \|/  L
          \  |  /
           \ |/
    --------+--------  Surface
            P
```

If **N** is the surface normal (unit vector pointing outward) and **L** is the direction *toward* the light, the irradiance at P is proportional to:

```
    I_diffuse = C_diffuse * max(N · L, 0)
```

The `max(..., 0)` clamps negative values — when the light is behind the surface, it contributes nothing (no "negative light").

**Why it looks flat:** Lambert only depends on surface orientation relative to the light. A sphere lit from the right has a smooth gradient but no highlights. There's no sense of the viewer's position — wherever you stand, the diffuse color at point P is the same.

**Worked example:** Consider a point on a sphere where N = (0, 1, 0) and the light direction L = (0.707, 0.707, 0) (45° above horizontal).

```
    N · L = 0*0.707 + 1*0.707 + 0*0 = 0.707
    C_diffuse = (0.8, 0.2, 0.2)   (red-ish)
    I_diffuse = (0.8, 0.2, 0.2) * 0.707 = (0.566, 0.141, 0.141)
```

At a point where L is directly overhead (N = L), N · L = 1.0, so the surface is at full brightness. At 60° off, N · L = 0.5 — half as bright. That's all there is to Lambert.

### 2. The Phong Reflection Model

Bui Tuong Phong (1975) proposed a three-term model:

```
    I = I_ambient + I_diffuse + I_specular
```

Each term captures a different physical phenomenon:

```
    I_ambient  = k_a * C_ambient                     (fakes indirect light)
    I_diffuse  = k_d * C_diffuse * max(N · L, 0)     (Lambert term)
    I_specular = k_s * C_specular * max(R · V, n)    (specular highlight)
```

```
         L       R
          \     /
           \   /
            \ /
    ---------+---------  Surface
         N |  V
            | /
            |/
            Eye
```

- **k_a, k_d, k_s** — weights controlling how much ambient, diffuse, and specular contribute.
- **R** = reflect(-L, N) — the reflection of the light direction about the normal.
- **V** — direction from the surface point toward the viewer.
- **n** (shininess exponent) — controls specular size. n=1 gives a broad, dull highlight; n=128 gives a tight, sharp highlight.

**Worked example (specular only):** Same point, N = (0, 1, 0), L = (0.707, 0.707, 0).

```
    R = reflect(-L, N) = 2*(N·(-L))*N - (-L) = 2*(-0.707)*(0,1,0) - (-0.707, -0.707, 0)
      = (0, -1.414, 0) + (0.707, 0.707, 0) = (0.707, -0.707, 0)

    Wait — let's redo this correctly:
    L = (0.707, 0.707, 0)   (toward light)
    I = -L = (-0.707, -0.707, 0)  (incident direction)
    R = I - 2*(I·N)*N = (-0.707, -0.707, 0) - 2*(-0.707)*(0,1,0)
      = (-0.707, -0.707, 0) + (0, 1.414, 0) = (-0.707, 0.707, 0)

    V = (0, 0, 1)  (looking along Z)
    R · V = (-0.707)*0 + 0.707*0 + 0*1 = 0

    With V = (-0.5, 0.866, 0):
    R · V = (-0.707)*(-0.5) + 0.707*0.866 + 0*0
          = 0.354 + 0.612 = 0.966
    (R · V)^32 ≈ 0.33   (tight highlight)
    (R · V)^8  ≈ 0.74   (broader highlight)
```

The shininess exponent n controls how quickly the highlight falls off.

### 3. The Blinn-Phong Improvement

Jim Blinn (1977) observed that computing R is expensive and that R · V has artifacts at grazing angles. He proposed replacing it with the **half-vector** approach:

```
    H = normalize(L + V)        (the "halfway" vector between light and view)

    I_specular = k_s * C_specular * max(N · H, n)
```

```
         L       H       V
          \      |      /
           \     |     /
            \    |    /
    ---------+---------  Surface
                N
```

**Why it's better:**

1. **Cheaper:** No `reflect()` call. Just normalize(L + V). For a directional light with fixed view, H is constant per pixel for Blinn vs per-light-per-pixel for Phong's R.

2. **More correct at grazing angles:** When the viewer is nearly parallel to the surface, Phong's R · V produces an elongated highlight that doesn't match real materials. N · H produces a rounder highlight that better matches experimental data.

3. **The shininess exponent maps differently:** A Blinn-Phong exponent n_blinn ≈ 4 * n_phong gives roughly the same highlight size. So Blinn n=80 ≈ Phong n=20.

**Worked example:** Same scenario: N = (0, 1, 0), L = (0.707, 0.707, 0), V = (-0.5, 0.866, 0).

```
    L + V = (0.707 + (-0.5), 0.707 + 0.866, 0 + 0) = (0.207, 1.573, 0)
    |L + V| = sqrt(0.043 + 2.474) = sqrt(2.517) = 1.586
    H = (0.130, 0.993, 0)

    N · H = 0*0.130 + 1*0.993 + 0*0 = 0.993
    (N · H)^32 ≈ 0.80   (vs Phong's 0.33 — brighter because H is closer to N)
```

To match Phong's n=32, Blinn-Phong would use n≈128.

### 4. Flat vs Gouraud vs Phong Shading

These are *interpolation strategies*, not reflection models. Confusingly, "Phong shading" and "Phong reflection" are different things.

```
    Flat shading:          Gouraud shading:        Phong shading:
    (per-face)             (per-vertex)             (per-pixel)

       _____                 _____                    _____
      /     \               /  1  \                  / 1 2 3 \
     / flat  \             / 1   2 \                / 4 5 6 7 \
    |  color   |          |___3___| |              | 8 9  A  B |
     \       /             \  4  /                  \  C  D  /
      \_____/               \___/                     \_____/
                             Each vertex             Each pixel
                             gets its own             computes its
                             color, then              own color from
                             interpolated             interpolated
                                                      normals
```

- **Flat shading:** One normal per face. One color per face. Fast but faceted.
- **Gouraud shading:** Compute color at each vertex, interpolate color across the face. Misses specular highlights in the middle of polygons.
- **Phong shading:** Interpolate the *normal* across the face, compute color per-pixel. Best quality, most computation.

### 5. Energy Conservation

A perfectly white Lambert surface (albedo = 1.0) must not reflect more energy than it receives. The total energy reflected over a hemisphere is:

```
    ∫∫  (N · L) / π  dΩ = 1.0   (when albedo = 1)
```

The 1/π factor is the normalization constant. In practice, this means:

```
    I_diffuse = C_diffuse / π * max(N · L, 0) * E_light
```

Most game engines and legacy shaders omit the 1/π for artistic control — they treat the diffuse color as already "authoring-friendly." But physically-based rendering (PBR) requires this normalization.

**Key insight:** If your diffuse term has no 1/π, you're not energy-conserving. A surface with k_d = 1.0 under direct light will reflect more energy than it receives.

### 6. The Rendering Equation (Brief)

All three models are approximations of Kajiya's rendering equation (1986):

```
    L_o(p, ω_o) = L_e(p, ω_o) + ∫_Ω f_r(p, ω_i, ω_o) * L_i(p, ω_i) * (N · ω_i) dω_i
```

- Lambert is an approximation where `f_r` is constant (flat BRDF).
- Phong/Blinn-Phong approximate the specular lobe with a cosine power.
- Neither handles global illumination, interreflection, or caustics.

These models are local — they only consider direct light. The rendering equation is the full global solution.

## Build It

### Step 1: GLSL Shader (Minimal — Lambert Only)

```glsl
// Minimal Lambert-only fragment
uniform vec3 u_light_dir;
uniform vec3 u_diffuse_color;

void main() {
    float NdotL = max(dot(v_normal, u_light_dir), 0.0);
    gl_FragColor = vec4(u_diffuse_color * NdotL, 1.0);
}
```

### Step 2: GLSL Shader (Full — All Three Models)

See `code/main.glsl` for the complete implementation with `#define` toggles.

### Step 3: Rust CPU Renderer (Minimal — Single Sphere)

See `code/main.rs` for the Rust implementation that renders three spheres side by side comparing Lambert, Phong, and Blinn-Phong.

## Use It

### Production Equivalents

In OpenGL/WebGL, Blinn-Phong is the default in fixed-function pipelines. In Vulkan/DirectX, you write it yourself.

**Three.js** uses MeshPhongMaterial (Blinn-Phong internally) and MeshStandardMaterial (PBR with GGX microfacet).

**Unity's** Standard shader uses Cook-Torrance microfacet BRDF — a more physically correct model that reduces to Blinn-Phong for smooth surfaces.

**Filament** (Google's PBR library) provides an excellent reference:
- `filament/shaders/src/shading_lit.fs` — Shows how Blinn-Phong is replaced by a full microfacet model in production.
- The Filament Materials Guide (https://google.github.io/filament/Filament.html) is the best free reference for physically-based shading.

### What Production Does That Your Code Doesn't

1. **Importance sampling:** Production renderers sample the BRDF stochastically; you evaluate it analytically.
2. **Tone mapping:** Your linear values go straight to pixels; production maps HDR → LDR.
3. **Fresnel:** Real materials reflect more at grazing angles (Schlick's approximation); your code ignores this.
4. **Multiple lights:** Production sums contributions from N lights; you handle one.
5. **Shadowing:** Production attenuates light blocked by geometry; you assume full visibility.

## Read the Source

- **Filament** (`filament/shaders/src/shading_lit.fs`): Shows how the Blinn-Phong specular term generalizes to a full microfacet BRDF. Look at `specular()` to see how D, F, G terms replace the single n exponent.

- **Mesa** (`mesa/src/compiler/glsl/`): The software OpenGL implementation. Shows how vertex attributes and varyings are compiled down.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`shading_reference.md`** — A one-page reference card with all three shading equations, a comparison table, and common parameter ranges.

## Exercises

1. **Easy** — Modify the GLSL shader to animate the light position over time using `u_time`. Watch how the specular highlight moves.

2. **Medium** — Add a second light source (a dim fill light from the opposite side). Show that diffuse adds linearly while only the brightest specular highlight dominates.

3. **Hard** — Implement a Cook-Torrance microfacet BRDF with the GGX distribution function. Compare the specular lobe shape against Blinn-Phong. When does GGX's longer tail matter visually?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Lambert | "flat shading" | Diffuse reflection model where brightness = N · L. Flat-looking because it ignores the viewer. |
| Phong | "the shiny one" | A three-term model (ambient + diffuse + specular). The specular uses R · V raised to a power. |
| Blinn-Phong | "better Phong" | Replaces R · V with N · H (half-vector). Cheaper, better at grazing angles, used everywhere. |
| Shininess exponent | "specular power" | Controls highlight size. Higher = tighter. Phong n and Blinn-Phong n differ by ~4x. |
| Half-vector H | "the H vector" | normalize(L + V). Points halfway between light and view. Replaces the expensive reflect() call. |
| Flat shading | "per-face color" | One normal per triangle. Fast, faceted look. |
| Gouraud shading | "per-vertex color" | Color computed at vertices, interpolated. Can miss specular highlights. |
| Phong shading | "per-pixel color" | Normals interpolated per-pixel. Best quality, most computation. |
| Energy conservation | "don't add energy" | Surfaces must not reflect more light than they receive. Requires the 1/π normalization. |
| Rendering equation | "the real deal" | The integral equation describing all light transport. Every shading model approximates it. |

## Further Reading

- **Phong's original thesis:** Bui Tuong Phong, "Illumination for Computer Generated Pictures" (1975). The 11-page paper that started it all.
- **Blinn's paper:** James Blinn, "Models of Light Reflection for Computer Synthesized Pictures" (1977). The half-vector innovation.
- **Filament Materials Guide:** https://google.github.io/filament/Filament.html — Best modern reference for PBR and the relationship between Phong and microfacet models.
- **Physically Based Rendering** (Pharr, Jakob, Humphreys) — Chapter 5 covers reflection models in depth.
- **Real-Time Rendering** (Akenine-Möller et al.) — Chapter 7 has the definitive comparison of all shading models.