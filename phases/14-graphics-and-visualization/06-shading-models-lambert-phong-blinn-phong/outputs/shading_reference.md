# Shading Models Reference Card

## Lambert (Diffuse Only)

```
I_diffuse = (C_d / π) × max(N · L, 0) × E_light
```

- Omit 1/π for non-PBR (artistic) workflows.
- Viewer-independent: same result from any camera position.
- Looks flat/matte — no highlights.

## Phong Reflection Model (Ambient + Diffuse + Specular)

```
I = k_a × C_a  +  k_d × (C_d / π) × max(N · L, 0) × E  +  k_s × C_s × max(R · V, 0)^n
R = reflect(-L, N)  =  2(N · L)N - L
```

- Three terms: ambient (fakes indirect light), diffuse (Lambert), specular (R · V).
- R is the reflected light direction; V is toward the viewer.
- n controls highlight sharpness (1 = broad, 128+ = pin-sharp).

## Blinn-Phong (Half-Vector Optimization)

```
I = k_a × C_a  +  k_d × (C_d / π) × max(N · L, 0) × E  +  k_s × C_s × max(N · H, 0)^n
H = normalize(L + V)
```

- Same structure as Phong, but replaces R · V with N · H.
- **Cheaper:** No reflect() — just normalize(L + V).
- **Better at grazing angles:** No elongated highlight artifacts.
- **Exponent mapping:** Blinn n ≈ 4 × Phong n for same highlight size.

## Comparison Table

| Property              | Lambert         | Phong              | Blinn-Phong        |
|-----------------------|-----------------|--------------------|--------------------|
| Diffuse term          | N · L           | N · L              | N · L              |
| Specular term         | —               | (R · V)^n          | (N · H)^n          |
| Computing spec        | —               | reflect() + dot    | normalize() + dot  |
| Grazing accuracy      | —               | Elongated highlights | Correct roundness |
| Energy-conserving     | Needs 1/π       | Needs 1/π          | Needs 1/π          |
| Viewer-dependent      | No              | Yes                | Yes                |
| Common shininess range| —               | 2–256              | 8–1024             |
| GPU cost (per pixel)  | 1 dot product   | 1 reflect + 2 dots | 1 normalize + 2 dots |
| Fixed-function default| —               | —                  | OpenGL/D3D default |

## Shading Interpolation Methods

| Method   | Computed at    | Interpolated | Quality  | Cost    |
|----------|---------------|--------------|----------|---------|
| Flat     | Per-face       | Nothing       | Faceted  | Lowest  |
| Gouraud  | Per-vertex     | Color         | Misses highlights | Medium |
| Phong    | Per-pixel      | Normal        | Smooth   | Highest |

## Common Parameter Ranges

| Parameter       | Typical Range    | Notes                                  |
|-----------------|------------------|----------------------------------------|
| k_d (diffuse)   | 0.0 – 1.0       | Surface base color intensity           |
| k_s (specular)  | 0.0 – 1.0       | 0 = matte, 1 = mirror-like highlight   |
| shininess (Phong)| 2 – 256         | Low = plastic/rubber, High = metal/glass |
| shininess (Blinn)| 8 – 1024        | ~4× Phong for same visual size          |
| ambient (k_a)   | 0.0 – 0.3       | Fakes indirect fill light               |

## Energy Conservation Rule

```
k_d + k_s ≤ 1.0   (energy in ≤ energy out)
```

Without 1/π normalization: a surface can reflect more than it receives.
PBR enforces this; legacy shaders often don't (for artistic control).

## The Big Picture

```
Lambert ──────────> Phong ──────────> Blinn-Phong ──────────> PBR (GGX)
  (diffuse only)     (+specular R·V)   (+specular N·H)        (microfacet BRDF)
     │                    │                   │                      │
     └── approximates ────┘── approximates ────┘── approximates ────┘── The Rendering Equation
```

Each model adds physical accuracy on the path toward the full rendering equation.