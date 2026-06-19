# Linear Algebra for Graphics — Transforms, Projections

> Every pixel on your screen is a lie — a carefully constructed illusion born from matrix multiplications and perspective divides. This lesson teaches you the math behind that illusion.

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 14 lessons 01–02
**Time:** ~75 minutes

## Learning Objectives

- Distinguish vectors (direction) from points (position) and explain why homogeneous coordinates unify them.
- Write 2D and 3D transform matrices from memory: translation, rotation, scaling, reflection.
- Explain why transform order matters and predict the difference between TR and RT.
- Derive the perspective projection matrix from first principles.
- Trace a vertex through the full Model → World → View → Clip → NDC → Screen pipeline.
- Identify common gotchas: Gimbal lock, handedness conventions, near-plane clipping.

## The Problem

You want to render a 3D scene on a 2D screen. A teapot sits at (3, 0, -5) in world space. The camera is at the origin looking down -Z. You need to rotate the teapot, move it into the camera's view, and flatten it onto a 2D image plane — all while preserving the illusion of depth.

Without linear algebra, you'd be hard-coding special cases forever. With it, every transform becomes a matrix multiply, every projection a single formula, and the entire graphics pipeline becomes a chain of multiplications you can compose, invert, and reason about.

## Vectors, Points, and Homogeneous Coordinates

### Direction vs. Position

A **vector** represents a direction and magnitude — "3 units right, 2 units up." A **point** represents a specific location — "the pixel at (3, 2)." They behave differently under translation:

```
Vector + Translation = Vector          (directions don't shift)
Point   + Translation = New Point      (locations do shift)
```

### Homogeneous Coordinates

We unify both into 4-component **homogeneous** vectors by adding a `w` component:

```
Point:    (x, y, z, 1)    w = 1 — affected by translation
Vector:   (x, y, z, 0)    w = 0 — NOT affected by translation
```

Why? Look at the translation matrix:

```
    | 1  0  0  tx |     | x |   | x + tx |
T = | 0  1  0  ty |  ×  | y | = | y + ty |
    | 0  0  1  tz |     | z |   | z + tz |
    | 0  0  0  1  |     | 1 |   |   1    |
```

The `w = 1` row picks up `tx, ty, tz`. If `w = 0`, that row becomes zero — translation is ignored. This is not a hack; it's the algebraic reason translation fits into matrix form at all.

### Worked Example

Point P at (2, 3, 1) translated by (5, -1, 0):

```
| 1 0 0  5 |   | 2 |   | 7 |
| 0 1 0 -1 | × | 3 | = | 2 |
| 0 0 1  0 |   | 1 |   | 1 |
| 0 0 0  1 |   | 1 |   | 1 |
```

Result: P' = (7, 2, 1). Correct.

Now translate a *direction* vector d = (2, 3, 1, 0):

```
| 1 0 0  5 |   | 2 |   | 2 |
| 0 1 0 -1 | × | 3 | = | 3 |
| 0 0 1  0 |   | 1 |   | 1 |
| 0 0 0  1 |   | 0 |   | 0 |
```

The translation is ignored — directions don't move. That w component is doing real work.

## 2D Transforms

All 2D transforms extend to 3×3 homogeneous matrices. We show the 2×2 core and the full 3×3 form.

### Scaling

```
2×2:            3×3 (homogeneous):
| Sx  0 |      | Sx  0  0 |
| 0   Sy|      | 0   Sy 0 |
                | 0   0  1 |
```

Negative values flip (reflect). `Sx = -1` reflects across Y-axis.

### Rotation (counter-clockwise, angle θ)

```
2×2:                    3×3 (homogeneous):
| cos θ  -sin θ |      | cos θ  -sin θ  0 |
| sin θ   cos θ |      | sin θ   cos θ  0 |
                         | 0       0      1 |
```

### Reflection

- Across X-axis: `Sx = 1, Sy = -1`
- Across Y-axis: `Sx = -1, Sy = 1`
- Across line y = x: swap x and y → matrix is `|0 1|` / `|1 0|`

### Shear

```
| 1  shx |
| shy  1 |
```

Shear skews the shape. `shx` tilts horizontally; `shy` tilts vertically.

## 3D Transforms

### Translation

```
| 1  0  0  tx |
| 0  1  0  ty |
| 0  0  1  tz |
| 0  0  0  1  |
```

### Rotation around Principal Axes

```
Rx(θ) = | 1     0      0    0 |       Ry(θ) = | cosθ   0  sinθ  0 |       Rz(θ) = | cosθ -sinθ  0  0 |
        | 0   cosθ  -sinθ  0 |               | 0      1   0     0 |               | sinθ  cosθ   0  0 |
        | 0   sinθ   cosθ  0 |               | -sinθ  0  cosθ   0 |               | 0      0     1  0 |
        | 0     0      0    1 |               | 0      0   0     1 |               | 0      0     0  1 |
```

Mnemonic: the axis you rotate around keeps its row and column as identity.

### Rotation around an Arbitrary Axis

Given unit axis `(ux, uy, uz)` and angle θ, use Rodrigues' formula expressed as a matrix:

```
R = cosθ · I + (1 - cosθ) · u⊗u + sinθ · [u]×
```

Where `u⊗u` is the outer product matrix and `[u]×` is the skew-symmetric cross-product matrix:

```
[u]× = | 0    -uz   uy  |
        | uz    0   -ux  |
        | -uy   ux    0  |
```

### Euler Angles vs. Quaternions

**Euler angles** (yaw, pitch, roll) are intuitive but suffer from **Gimbal lock**: when two axes align, you lose a degree of freedom.

```
      Gimbal lock: pitch = 90°
      → yaw and roll produce the SAME rotation around the same axis
      → you've lost one DOF
```

**Quaternions** (q = w + xi + yj + zk) avoid Gimbal lock, interpolate smoothly (SLERP), and compose via quaternion multiplication. Every graphics engine uses quaternions internally for animation.

```
Quaternion from axis-angle:
  q = (cos(θ/2), sin(θ/2)·ux, sin(θ/2)·uy, sin(θ/2)·uz)

Quaternion to rotation matrix: (see outputs/linalg_cheatsheet.md)
```

## Composite Transforms: Why Order Matters

Matrix multiplication is **not commutative**. In practice, we use **column vectors** (OpenGL/Vulkan convention), so transforms are applied right-to-left:

```
v' = M_total · v    where M_total = T · R · S

This means: first Scale, then Rotate, then Translate.
```

### TR vs RT — A Concrete Example

```
Start: point P = (1, 0, 0)
Translate by (5, 0, 0), then Rotate 90° around Z:

  TR:  T(5,0,0) · Rz(90°) · P
     = T(5,0,0) · (0, 1, 0)        [rotate first → still near origin]
     = (5, 1, 0)                    [then translate away]

RT:  Rz(90°) · T(5,0,0) · P
     = Rz(90°) · (6, 0, 0)         [translate first → moved to x=6]
     = (0, 6, 0)                    [then rotate → now far from origin!]
```

Same inputs, wildly different results. **Order matters.** Always.

### Why Column Vectors?

Column vectors (v' = M · v) are the dominant convention in OpenGL, Vulkan, DirectX, and most textbooks. Row-vector convention (v' = v · M) reverses the multiplication order. The math is isomorphic, but you must pick one and stick with it. This lesson uses column vectors throughout.

## The Full MVP Pipeline

A vertex travels through these coordinate systems:

```
Model Space                    (artist-authored coordinates)
   ↓  Model Matrix (M)
World Space                    (where objects live relative to each other)
   ↓  View Matrix (V)
View/Camera Space              (camera at origin, looking down -Z)
   ↓  Projection Matrix (P)
Clip Space                     (after projection, before perspective divide)
   ↓  Perspective Divide (÷ w)
NDC — Normalized Device Coords (homogeneous cube [-1,1]³)
   ↓  Viewport Transform
Screen Space                   (pixel coordinates)
```

### Model Matrix

Places an object in the world. Rotates, scales, translates the model-space coordinates into world-space position and orientation. This is the TRS composite we discussed.

### View Matrix

Transforms the world so the camera is at the origin looking down -Z. Think of it as the inverse of the camera's own model matrix:

```
V = (M_camera)^(-1)
```

If the camera is at eye position `e` looking at target `t`, the classic `lookAt` matrix constructs V from:

```
forward = normalize(t - e)
right   = normalize(cross(forward, up_world))
up      = cross(right, forward)

V = | right.x    right.y    right.z   -dot(right, e)   |
    | up.x       up.y       up.z      -dot(up, e)      |
    | -forward.x -forward.y -forward.z  dot(forward, e) |
    | 0          0          0           1                 |
```

### Clip Space and the Perspective Divide

The projection matrix maps the view frustum into clip space. After multiplication, vertices have a `w` component that is NOT 1 anymore — it holds the eye-space depth:

```
After P · V · M · v = (x_clip, y_clip, z_clip, w_clip)

The perspective divide:  (x_ndc, y_ndc, z_ndc) = (x_clip/w_clip, y_clip/w_clip, z_clip/w_clip)
```

This is where "things farther away appear smaller" actually happens mathematically. Dividing by `w` (which is proportional to `z`) shrinks distant objects.

## Perspective Projection Matrix

### Derivation from First Principles

We want to project a 3D point (x, y, z) onto a near plane at distance `n` from the camera.

```
         y (eye space)
         |
         |   * (x, y, z)
         |  /|
         | / |
    -----|/--+-------- near plane (z = -n)
         |/   |
    -----+----+-------- image plane
         |    |
         +----|--------→ x
              y' = projected y
```

By similar triangles: `y' / n = y / (-z)` → `y' = -n · y / z`

More generally: the projected coordinates are `x' = -n · x / z` and `y' = -n · y / z`.

We need `1/z` to appear. But matrix multiply can only produce **linear** combinations. The trick: we encode `z` in the `w` component so the **perspective divide** does the `1/z` for us:

```
| x' |     | x |         x_clip = n·x         x_ndc = x_clip/w_clip = n·x / (-z)
| y' |  ∝  | y |   →     y_clip = n·y    →    y_ndc = y_clip/w_clip = n·y / (-z)
| ?  |     | z |         z_clip = ...           z_ndc = ...
| w' |     | 1 |         w_clip = -z            (nonlinear depth, handled separately)
```

The full OpenGL-style perspective matrix (symmetric frustum, vertical FOV = fovY, aspect = a, near = n, far = f):

```
        | f_cot/a   0     0                    0                  |
P    =  | 0       f_cot   0                    0                  |
        | 0         0   -(f+n)/(f-n)   -2·f·n/(f-n)             |
        | 0         0     -1                   0                  |

where f_cot = 1/tan(fovY/2)
```

Key observations:
- The bottom row `(0, 0, -1, 0)` sets `w_clip = -z_eye` — this is what drives the perspective divide.
- The top two rows scale by `f_cot` (focal length) and correct for aspect ratio.
- The third row remaps depth from [n, f] to [-1, 1] in NDC.

### Orthographic Projection — Special Case

Set `w_clip = 1` (no perspective divide) and linearly map the box [left, right] × [bottom, top] × [near, far] to NDC:

```
        | 2/(r-l)    0          0        -(r+l)/(r-l)  |
Porth = | 0       2/(t-b)      0        -(t+b)/(t-b)  |
        | 0          0     -2/(f-n)   -(f+n)/(f-n)   |
        | 0          0          0              1        |
```

No `1/z` foreshortening. Lines that are parallel in 3D stay parallel on screen. Used in CAD, 2D games, and UI rendering.

## View Frustum and Clipping

The view frustum is the truncated pyramid the camera can see:

```
        far plane
      ___________
     /           \
    /             \      ← visible volume
   /               \
  /_________________\
      near plane
      (screen)
```

Vertices outside the frustum are clipped. After the perspective divide, NDC coordinates outside [-1, 1] in any axis are outside the visible region. In practice, clipping is done in **clip space** (before the divide) to avoid division by zero.

The six clip planes correspond to:
```
-w ≤ x ≤ w
-w ≤ y ≤ w
-w ≤ z ≤ w     (for perspective)
```

## Coordinate System Conventions

### Right-Hand vs. Left-Hand

```
Right-hand (OpenGL):           Left-hand (DirectX):
  Y                                Y
  |                                |
  |  Z points                      |  Z points
  | / toward viewer                | \ into screen
  |/                               |/
  +------→ X                       +------→ X
```

- **Right-hand**: +Z toward the viewer (OpenGL, most math/physics contexts)
- **Left-hand**: +Z into the screen (DirectX, Unity, some game engines)

### Y-up vs Z-up

- **Y-up** (OpenGL, DirectX, most 3D software): Y is vertical, ground is XZ plane
- **Z-up** (Blender, many CAD packages): Z is vertical, ground is XY plane

**Always check the convention** when importing/exporting between tools. Getting this wrong inverts your normals or flips your depth buffer.

## Build It

### Step 1: Minimal Version

Implement `Vec3` and `Mat4` by hand. Prove TR ≠ RT with concrete numbers.

```python
# See code/main.py for the full implementation
v = Vec3(1, 0, 0)
T = Mat4.translation(5, 0, 0)
R = Mat4.rotation_z(90)

result_TR = T * R * v   # Scale→Rotate→Translate order
result_RT = R * T * v   # Translate→Rotate order
# TR gives (5, 1, 0), RT gives (0, 6, 0) — different!
```

### Step 2: Realistic Version

Build the perspective projection matrix from scratch. Render a wireframe cube to PPM through the full pipeline: Model → View → Projection → Perspective Divide → Viewport → Pixels.

The code in `code/main.py` and `code/main.rs` implements:
1. `Vec3` and `Mat4` classes with operator overloading
2. Transform composition (`TR` vs `RT` demonstration)
3. Perspective projection matrix construction
4. A full render pipeline that produces a rotating wireframe cube PPM image

## Use It

In production graphics code (OpenGL/Vulkan/DirectX), you never write matrix classes by hand — you use GLM (C++), `numpy` (Python), or `nalgebra` (Rust). But every graphics programmer needs to understand what these libraries do, because:

- Debugging projection artifacts requires knowing which matrix entry is wrong.
- Choosing FOV, near/far planes, and handedness requires understanding the matrix.
- Performance tuning sometimes requires pre-composing matrices (`MVP = P·V·M` once, not per vertex).
- Custom projections (oblique, shadow map, etc.) require writing your own matrices.

**GLM** (used with OpenGL): `glm::perspective(fov, aspect, near, far)` produces exactly the matrix we derived above.
**nalgebra** (Rust): `Perspective3::new(fov, aspect, near, far)` does the same.

## Read the Source

- [ Mesa3D `u_viewport_transform`](https://gitlab.freedesktop.org/mesa/mesa) — see how a real OpenGL implementation converts NDC to window coordinates.
- [ glm perspective matrix](https://github.com/g-truc/glm) — compare `glm::perspective` against our derivation.

## Ship It

The reusable artifact for this lesson lives in `outputs/linalg_cheatsheet.md` — a single-page reference card of every key matrix formula, convention notes, and "when to use what" guidance. Print it, pin it next to your monitor.

## Exercises

1. **Easy** — Implement `Vec3` and `Mat4` from scratch without looking at the lesson code. Verify that `T · R · v` and `R · T · v` give different results for a non-origin point.
2. **Medium** — Modify the perspective projection to create an **oblique** projection (near plane not perpendicular to view direction) and render the same cube. Compare with the perspective result.
3. **Hard** — Implement quaternion SLERP and animate a smooth rotation between two orientations. Compare the result with Euler angle interpolation (you'll see the Gimbal lock artifact when the middle axis approaches ±90°).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Homogeneous coordinates | "Using w" | A 4-component representation that lets translation be a matrix multiply (w=1 for points, w=0 for vectors) |
| Perspective divide | "Dividing by w" | The nonlinear `x/w, y/w` step that makes distant objects smaller — the core of 3D foreshortening |
| MVP matrix | "The projection matrix" | The composed Model×View×Projection matrix; it's three matrices, not one |
| Gimbal lock | "Euler angle problem" | When two rotation axes align, you lose a DOF — quaternions fix this |
| NDC | "Clip space" | Normalized Device Coordinates: after perspective divide, everything is in [-1, 1]³ |
| View frustum | "What the camera sees" | The truncated pyramid defined by near/far planes and FOV |

## Further Reading

- **Fundamentals of Computer Graphics** (Marschner & Shirley) — Chapters 5–7 for transforms and projection derivations
- **3D Math Primer for Graphics and Game Development** (Dunn & Parberry) — Practical guide to the math with code
- **OpenGL spec, section 2.13** — The definitive reference for the fixed-function coordinate transforms
- **Essence of Linear Algebra** (3Blue1Brown, YouTube) — Visual intuition for linear algebra fundamentals
- **Real-Time Rendering** (Akenine-Möller et al.) — Chapter 4 covers transforms with production-level detail