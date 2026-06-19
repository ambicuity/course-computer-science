# Linear Algebra for Graphics — Cheatsheet

## Homogeneous Coordinates

| Type   | Representation | Translation |
|--------|---------------|-------------|
| Point  | (x, y, z, 1) | Affected    |
| Vector | (x, y, z, 0) | Ignored     |

## 2D Transform Matrices (3×3 homogeneous)

### Translation
```
| 1  0  tx |
| 0  1  ty |
| 0  0  1  |
```

### Scaling
```
| Sx  0   0 |
| 0   Sy  0 |
| 0   0   1 |
```

### Rotation (CCW by θ)
```
| cosθ  -sinθ  0 |
| sinθ   cosθ  0 |
|  0      0    1 |
```

### Reflection
- Across X-axis: `Sx=1, Sy=-1`
- Across Y-axis: `Sx=-1, Sy=1`
- Across y=x: swap rows: `|0 1 0| / |1 0 0| / |0 0 1|`

### Shear
```
| 1   shx  0 |
| shy  1   0 |
| 0    0   1 |
```

## 3D Transform Matrices (4×4)

### Translation
```
| 1  0  0  tx |
| 0  1  0  ty |
| 0  0  1  tz |
| 0  0  0  1  |
```

### Scaling
```
| Sx  0   0   0 |
| 0   Sy  0   0 |
| 0   0   Sz  0 |
| 0   0   0   1 |
```

### Rotation around X
```
| 1    0     0    0 |
| 0   cosθ  -sinθ 0 |
| 0   sinθ   cosθ 0 |
| 0    0     0    1 |
```

### Rotation around Y
```
|  cosθ  0  sinθ  0 |
|   0    1   0    0 |
| -sinθ  0  cosθ  0 |
|   0    0   0    1 |
```

### Rotation around Z
```
| cosθ  -sinθ  0  0 |
| sinθ   cosθ  0  0 |
|  0      0    1  0 |
|  0      0    0  1 |
```

### Rotation around Arbitrary Axis (Rodrigues)

Given unit axis **u** = (ux, uy, uz) and angle θ:

```
R = cosθ·I + (1-cosθ)·(u ⊗ u) + sinθ·[u]×

where [u]× = | 0    -uz   uy  |
              | uz    0   -ux  |
              | -uy   ux    0  |

u ⊗ u = | ux²   ux·uy  ux·uz |
         | uy·ux  uy²   uy·uz |
         | uz·ux uz·uy  uz²  |
```

## Quaternion Quick Reference

### From Axis-Angle
```
q = (cos(θ/2), sin(θ/2)·ux, sin(θ/2)·uy, sin(θ/2)·uz)
```

### To Rotation Matrix
```
| 1-2(y²+z²)   2(xy-wz)   2(xz+wy) |
| 2(xy+wz)   1-2(x²+z²)   2(yz-wx) |
| 2(xz-wy)     2(yz+wx)   1-2(x²+y²)|
```
(where q = (w, x, y, z))

### SLERP
```
slerp(q₁, q₂, t) = sin((1-t)θ)/sin(θ) · q₁ + sin(tθ)/sin(θ) · q₂
where θ = arccos(q₁ · q₂)
```

**Use quaternions when:** animating rotations (no Gimbal lock), interpolating orientations (SLERP).
**Use Euler angles when:** authoring rotations in a UI (intuitive for humans).

## Projection Matrices

### Perspective (symmetric frustum)
```
fovY = vertical field of view in radians
a    = width / height (aspect ratio)
n    = near plane distance (positive)
f    = far plane distance (positive)
cot  = 1 / tan(fovY / 2)

     | cot/a   0      0            0          |
P =  | 0      cot     0            0          |
     | 0       0   -(f+n)/(f-n)  -2fn/(f-n)  |
     | 0       0     -1            0          |
```

**After projection:** divide (x_clip, y_clip, z_clip) by w_clip to get NDC.

### Orthographic
```
Maps box [l,r]×[b,t]×[n,f] → [-1,1]³

     | 2/(r-l)    0        0      -(r+l)/(r-l) |
P =  | 0       2/(t-b)     0      -(t+b)/(t-b) |
     | 0          0    -2/(f-n)   -(f+n)/(f-n) |
     | 0          0        0            1       |
```

**No perspective divide** — w_clip stays 1. Parallel lines stay parallel.

**Use perspective when:** rendering 3D scenes (games, film, VR).
**Use orthographic when:** CAD, 2D games, UI, technical drawings.

## The Full Pipeline

```
Model Space → [Model Matrix] → World Space → [View Matrix] →
View/Camera Space → [Projection Matrix] → Clip Space →
[perspective divide ÷w] → NDC → [viewport transform] → Screen Space
```

### Look-At (View Matrix)
```
forward = normalize(target - eye)
right   = normalize(cross(forward, worldUp))
up      = cross(right, forward)

V = | right.x    right.y    right.z   -dot(right, eye)   |
    | up.x       up.y       up.z      -dot(up, eye)      |
    | -fwd.x    -fwd.y     -fwd.z      dot(fwd, eye)    |
    | 0          0          0           1                  |
```

## Convention Cheatsheet

| System       | Handedness | Y/Z-up | Clip Z range |
|-------------|-----------|--------|-------------|
| OpenGL      | Right     | Y-up   | [-1, 1]     |
| Vulkan      | Right     | Y-up   | [0, 1]      |
| DirectX     | Left      | Y-up   | [0, 1]      |
| Unity       | Left      | Y-up   | [0, 1]      |
| Blender     | Right     | Z-up   | —           |

**Always check conventions** when importing assets or writing cross-platform code.

## Common Gotchas

| Gotcha             | Symptom                              | Fix                                      |
|--------------------|--------------------------------------|------------------------------------------|
| Wrong multiplication order | Object orbits instead of rotating | Remember: column vectors → apply right-to-left |
| Gimbal lock        | Animation snaps at ±90° pitch        | Use quaternions instead of Euler angles  |
| Handedness mismatch| Normals point inward, depth inverted| Flip Z and/or reverse winding order       |
| Near plane too small| Z-fighting (flickering surfaces)    | Push near plane farther from camera      |
| w ≤ 0 at divide    | Vertex behind camera, division by 0  | Clip before perspective divide           |