# Raytracer Reference Card

A quick-reference for ray-object intersection formulas, the Whitted algorithm, and shadow ray logic.

## Ray Definition

```
P(t) = O + t·D    where t > 0 for visible geometry
```

- `O` = ray origin (3D point)
- `D` = ray direction (unit vector)
- `t` = parameter; smallest positive t = nearest intersection

## Ray-Sphere Intersection

Sphere: center `C`, radius `r`. Let `L = O − C`.

```
at² + bt + c = 0
a = D·D
b = 2(L·D)
c = L·L − r²

Δ = b² − 4ac

If Δ < 0  →  no hit
If Δ ≥ 0  →  t = (−b ± √Δ) / 2a

Pick smallest positive t.
Normal at hit: N = (P − C) / r
Flip N if D·N > 0 (ray inside sphere).
```

## Ray-Plane Intersection

Plane: point `Q`, normal `N`. Plane constant `d = Q·N`.

```
t = (d − O·N) / (D·N)

If D·N ≈ 0  →  ray parallel, no hit
If t ≤ 0     →  behind origin, no hit
Otherwise    →  hit at P = O + tD
```

## Shadow Ray Logic

```
For each light L at position P_L:
  direction = (P_L − hit_point).normalized()
  distance  = |P_L − hit_point|
  origin    = hit_point + N · ε     (offset along normal)

  Cast ray(origin, direction) with t ∈ [0, distance]
  If any intersection found → point is in shadow for this light
```

## Mirror Reflection

```
R = D − 2(D·N)N

Reflected ray: origin = hit_point + N·ε, direction = R
Color contribution: kr × trace(reflected_ray, depth−1)
```

## Snell's Law (Refraction)

```
n₁ sin θ₁ = n₂ sin θ₂

η = n₁/n₂
cos θ_i = −N·D
sin²θ_t = η²(1 − cos²θ_i)

If sin²θ_t > 1  →  total internal reflection, no refracted ray

cos θ_t = √(1 − sin²θ_t)
T = η·D + (η·cos θ_i − cos θ_t)·N

Refracted ray: origin = hit_point − N·ε, direction = T.normalized()
```

## Whitted Algorithm Pseudocode

```
function trace(ray, depth):
    if depth ≤ 0:
        return BACKGROUND_COLOR
    
    hit = find_nearest_intersection(ray)
    if no hit:
        return BACKGROUND_COLOR
    
    color = local_illumination(hit)
    
    if hit.material.kr > 0 and depth > 0:
        R = reflect(ray.direction, hit.normal)
        reflected_ray = Ray(hit.point + N·ε, R)
        color += hit.material.kr × trace(reflected_ray, depth−1)
    
    if hit.material.kt > 0 and depth > 0:
        T = refract(ray.direction, hit.normal, hit.material.ior)
        if T is valid (no TIR):
            refracted_ray = Ray(hit.point − N·ε, T)
            color += hit.material.kt × trace(refracted_ray, depth−1)
    
    return color
```

## Local Illumination (Phong)

```
for each light:
    if shadow_ray_blocked:
        continue
    
    L = (light_pos − hit_point).normalized()
    V = (eye − hit_point).normalized()
    R = reflect(−L, N)
    
    diffuse  = kd × max(0, N·L)
    specular = ks × max(0, R·V)^n
    
local_color = ka × object_color + Σ_lights[(diffuse + specular) × light_color × light_intensity]
```

## Key Constants & Tips

| Symbol | Typical Value | Meaning |
|--------|--------------|---------|
| ε      | 0.001        | Shadow/reflection offset to prevent self-intersection |
| depth  | 3–5          | Maximum recursion depth |
| n₁     | 1.0          | Refractive index of air |
| n₂     | 1.5          | Refractive index of glass |
| kr     | 0–1          | Reflection coefficient |
| kt     | 0–1          | Transmission (refraction) coefficient |