# Rasterization II — Z-buffer, Clipping, Culling

> If you can't decide which triangle is in front, you can't render a 3D scene.

**Type:** Learn
**Languages:** Rust, C++
**Prerequisites:** Phase 14 lessons 01–04
**Time:** ~75 minutes

## Learning Objectives

- Implement the Z-buffer algorithm with per-pixel depth testing
- Understand why 1/z is preferable to raw z for depth interpolation
- Diagnose and fix z-fighting artifacts
- Apply backface culling using winding order and view-vector dot products
- Clip triangles against the view frustum using Sutherland-Hodgman
- Explain why clipping must happen in homogeneous coordinates (before the perspective divide)
- Distinguish the culling hierarchy: frustum → backface → occlusion

## The Problem

In Lesson 04 you learned how to rasterize a single triangle. But real scenes have *thousands* of overlapping triangles. Which one is visible at each pixel? Without an answer, you get random flickering — one triangle drawn, then another overwrites it, with no concept of "closer."

This lesson solves the **visibility problem**: given N triangles projecting to the same pixel, which one wins? And before we even rasterize, how do we *avoid* rasterizing triangles the camera can't see?

```
   Camera
     |
     v
  +-------+
  | A / B |   ← Does pixel show triangle A or triangle B?
  |  /B/A |   ← Depends on depth.
  +-------+
```

## The Z-buffer Algorithm

### Core Idea

For every pixel on screen, store the closest depth seen so far. When a new triangle fragment arrives, compare its depth to the stored value. Closer? Overwrite both color and depth. Farther? Discard.

```
  Framebuffer:  color[x][y]    — RGB of closest fragment
  Z-buffer:     zbuf[x][y]     — depth of closest fragment (init to +∞)
```

### Rasterization with Z-buffer

```
for each triangle T = (v0, v1, v2):
    for each pixel (x, y) inside T's bounding box:
        compute barycentric (u, v, w) at (x, y)
        if u, v, w < 0: skip  (outside triangle)

        depth = u*v0.z + v*v1.z + w*v2.z   // interpolate z

        if depth < zbuf[x][y]:
            zbuf[x][y] = depth
            color[x][y] = shade(T, u, v, w)
```

That's it. The Z-buffer is O(pixels × triangles) in the worst case, but early-z and culling make it fast in practice.

### Worked Example

Two triangles overlap at pixel (200, 150):

```
Triangle A: depth at (200,150) = 0.8
Triangle B: depth at (200,150) = 0.3

Z-buffer starts at +∞.
1. Triangle A renders: 0.8 < ∞ → write color_A, zbuf = 0.8
2. Triangle B renders: 0.3 < 0.8 → write color_B, zbuf = 0.3

Pixel (200,150) shows triangle B (closer).
```

## Z-buffer Precision: Why 1/z Beats z

### The Problem with Linear Z

After the perspective projection matrix, depth is *not* linear in screen space. It maps non-linearly:

```
  z_ndc = (f+n)/(f-n) - 2fn/((f-n)*z_eye)

  where n = near plane, f = far plane, z_eye = eye-space depth
```

This means the Z-buffer has *much* better precision near the near plane and terrible precision far away. Most of your precision budget is wasted on the first few meters.

```
  Precision near plane:  ████████████████████  (lots of bits)
  Precision far plane:   ██                     (few bits)

  Near=0.1, Far=1000:
    90% of precision used in first 10% of the depth range
```

### Z-fighting

When two surfaces are at nearly the same depth, the Z-buffer can't distinguish them. Rounding errors cause pixels to randomly pick one surface or the other each frame, producing flickering "z-fighting" artifacts.

```
  Surface A: depth = 10.0000
  Surface B: depth = 10.0001
  Z-buffer resolution at that depth: 0.01

  → Which surface wins? It's a coin flip per frame.
```

**Remedies:**
1. Push the near plane as far out as possible (biggest impact)
2. Pull the far plane as close as possible
3. Use a floating-point depth buffer (`GL_DEPTH_COMPONENT32F`)
4. Use reversed depth: map near → 1.0, far → 0.0, so precision is concentrated far
5. Add a small polygon offset (bias) to co-planar geometry

### Why Interpolate 1/z Instead of z

In screen space, 1/z is *linear*. Interpolating z directly through a triangle in screen space is incorrect — z is linear in *clip space*, not screen space. But 1/z *is* linear in screen space:

```
  1/z(x,y) = (1/z0)*u + (1/z1)*v + (1/z2)*w

  This is exact. No approximation.
```

This means:
- We can correctly interpolate depth using the same barycentric weights as color
- We get better precision distribution (more uniform error)
- GPU hardware uses this for perspective-correct interpolation of *all* varyings, not just depth

```
  Attribute interpolation (perspective-correct):
    attr/z = (attr0/z0)*u + (attr1/z1)*v + (attr2/z2)*w
    attr  = (attr/z) * z_interpolated

  Where z_interpolated = 1 / (u/z0 + v/z1 + w/z2)
```

## Backface Culling

### Winding Order

A triangle is **front-facing** if its screen-space vertices appear counter-clockwise (CCW). Clockwise (CW) means it faces away from the camera — a backface. We can skip it entirely.

```
  Front-facing (CCW):       Back-facing (CW):

      v1                        v1
      /\                        /\
     /  \                      /  \
    / →  \                    / ←  \
   v0----v2                  v0----v2
```

### Signed Area Test

The signed area of a screen-space triangle tells us the winding:

```
  signed_area = 0.5 * ((x1-x0)*(y2-y0) - (x2-x0)*(y1-y0))

  signed_area > 0 → CCW → front-facing → KEEP
  signed_area < 0 → CW  → back-facing  → CULL
  signed_area = 0 → degenerate (zero area)
```

This is a cross product of edge vectors in 2D:

```
  edge1 = v1 - v0 = (x1-x0, y1-y0)
  edge2 = v2 - v0 = (x2-x0, y2-y0)
  cross_z = edge1.x * edge2.y - edge2.x * edge1.y
```

### Dot Product with View Vector (World Space)

In world space, you can also test:

```
  face_normal = cross(v1-v0, v2-v0)   // triangle normal
  view_dir    = camera_pos - v0

  if dot(face_normal, view_dir) <= 0:
      CULL (back-facing)
```

### When Backface Culling Fails

1. **Double-sided geometry** — Leaves, cloth, paper: both sides are visible
2. **Mirrors/teleporters** — Reflected geometry inverts winding
3. **Non-manifold meshes** — Inconsistent winding order
4. **Orthographic projection** — The 2D signed-area test works; the 3D dot-product test needs a fixed view direction, not `(camera - vertex)`

## Frustum Clipping

### Why Clip?

Triangles extending beyond the view frustum:
- Produce invalid screen coordinates after perspective divide (w ≤ 0 causes division by zero or flipped geometry)
- Waste rasterization time on off-screen pixels
- Can produce negative-w vertices that confuse the rasterizer

### Near-Plane Clipping Is Critical

The near clip plane is the *most important* clip plane. Geometry behind the camera (w ≤ 0 after projection) would project to inverted screen coordinates — the perspective divide flips them. We *must* clip before projecting.

```
  Before clipping (side view):

       far plane
        __|__
       /     \        ← triangle extends past near plane
      /  near \__
     /  plane    \    ← portion BEHIND camera is invalid
    /     |       \
   camera |

  After near-plane clipping:

       far plane
        __|__
       /     |        ← only the valid portion remains
      /  near |
     /  plane |
    /     |   |
   camera |
```

### Cohen-Sutherland (Line Clipping)

For *lines*, Cohen-Sutherland assigns a 4-bit outcode per endpoint:

```
  bit 0: left of frustum    (x < -w)
  bit 1: right of frustum   (x >  w)
  bit 2: below frustum      (y < -w)
  bit 3: above frustum      (y >  w)

  Trivially accept: outcode_A | outcode_B == 0  (both inside)
  Trivially reject: outcode_A & outcode_B != 0  (both on same side)
  Otherwise: clip against each violated plane
```

### Sutherland-Hodgman (Polygon Clipping)

For *triangles* (polygons), Sutherland-Hodgman clips against each plane in sequence:

```
  input_list = [v0, v1, v2]

  for each clip_plane in [left, right, bottom, top, near, far]:
      output_list = []
      for each edge (S→E) in input_list:
          if S inside plane and E inside plane:
              output_list.append(E)
          elif S inside plane and E outside plane:
              output_list.append(intersection(S, E, plane))
          elif S outside plane and E inside plane:
              output_list.append(intersection(S, E, plane))
              output_list.append(E)
          # else: both outside → discard both

      input_list = output_list

  return input_list  // may be 3-7 vertices (fan into triangles)
```

This can turn one triangle into up to 7 vertices (fan into multiple triangles).

### Clipping in Homogeneous Coordinates

**Critical insight:** Clip *before* the perspective divide. In clip space, each plane is defined by a simple inequality:

```
  Left:   x + w ≥ 0      Right:  w - x ≥ 0
  Bottom: y + w ≥ 0      Top:    w - y ≥ 0
  Near:   z + w ≥ 0      Far:    w - z ≥ 0

  (for OpenGL-style NDC where clip space maps to [-1,1])
```

Why not clip in NDC after the divide? Because vertices with w ≤ 0 produce *inverted* coordinates. A point behind the camera gets mapped to a point on the opposite side of NDC — but only after the divide. The divide destroys the information about which side of the camera the point was on.

Clipping in homogeneous coordinates:
- Preserves the w component for perspective-correct interpolation
- Correctly discards geometry behind the camera
- Allows exact intersection computation via parametric lines
- Avoids the singularity at w = 0

### Triangle Clipping: Generating New Vertices

When a triangle straddles a clip plane, we split it:

```
  Before clipping (triangle crosses near plane):

    v0 (behind camera)
     \
      \   near plane
       \  |
        \ |
    v1---v2 (in front of camera)

  After clipping (two new vertices a, b on near plane):

    a----v2
    |   /
    |  /
    b-/
    |
    v1

    Two triangles: (v1, a, b) and (v1, b, v2)
```

The intersection point along edge (S→E) at clip plane:

```
  t = (dot_value + S.w) / (S.w - E.w)
  // where dot_value depends on the plane:
  // near plane: S.z + S.w
  intersection = S + t * (E - S)
```

## The Culling Hierarchy

Not all invisible geometry is eliminated the same way. The GPU pipeline applies culling in stages, from cheapest to most expensive:

```
  ┌─────────────────────────────────────────────────┐
  │  1. View Frustum Culling  (CPU / object level)  │
  │     - Discard objects outside the frustum        │
  │     - Bounding sphere / AABB test               │
  │     - Very cheap, eliminates entire meshes      │
  ├─────────────────────────────────────────────────┤
  │  2. Backface Culling  (GPU / triangle level)     │
  │     - Discard triangles facing away from camera  │
  │     - Signed area test (2D) or dot product (3D)  │
  │     - Eliminates ~50% of triangles in closed meshes │
  ├─────────────────────────────────────────────────┤
  │  3. Occlusion Culling  (GPU / pixel level)      │
  │     - Z-buffer: only the closest fragment wins  │
  │     - HiZ: hierarchical depth buffer for early rejection │
  │     - Most expensive, but eliminates remaining hidden surfaces │
  └─────────────────────────────────────────────────┘
```

Each stage feeds into the next. Frustum culling removes whole objects. Backface culling removes half the remaining triangles. Occlusion culling handles the rest, pixel by pixel.

**Order matters:** doing culling out of order (e.g., testing every pixel of a backface against the Z-buffer) wastes work that the cheaper test would have eliminated.

## Scanline Optimization

### Bounding Box

Don't iterate over the entire screen. Compute the axis-aligned bounding box of each triangle and iterate only within it:

```
  min_x = floor(min(v0.x, v1.x, v2.x))
  max_x = ceil(max(v0.x, v1.x, v2.x))
  min_y = floor(min(v0.y, v1.y, v2.y))
  max_y = ceil(max(v0.x, v1.x, v2.y))  // whoops, v2.y

  for y in min_y..=max_y:
      for x in min_x..=max_x:
          ... (barycentric test + Z-buffer)
```

### Edge Walking (Scanline Triangle Fill)

Instead of testing every pixel in the bounding box, walk along triangle edges and fill horizontal spans:

```
  For each scanline y:
      Find x_left  = intersection with left edge
      Find x_right = intersection with right edge
      Fill pixels from x_left to x_right

  This avoids the barycentric test entirely for interior pixels.
  Only edge pixels need a bounds check.
```

Edge walking is how production rasterizers work, but barycentric + bounding-box is simpler to implement correctly.

## Build It

### Step 1: Minimal Z-buffer Rasterizer (C++)

See `code/main.cpp` for a complete implementation that:
- Rasterizes two overlapping triangles with correct depth ordering
- Demonstrates z-fighting with nearly co-planar triangles
- Outputs PPM images showing correct occlusion

### Step 2: Rust Implementation with Backface Culling

See `code/main.rs` for:
- Same Z-buffer rasterizer in idiomatic Rust
- Backface culling via signed-area test
- PPM output

## Use It

In production graphics APIs:

- **OpenGL/Vulkan/DirectX:** Z-buffer is `GL_DEPTH_COMPONENT` / `VK_FORMAT_D32_SFLOAT`. You create it as a texture attachment alongside the color framebuffer.
- **WebGPU:** `depthStencilAttachment` with `depthCompare: 'less'` and `depthWriteEnabled: true`.
- **Software rendering:** Mesa's `swrast` uses the exact algorithm described here — bounding box, barycentric coordinates, Z-buffer comparison.

The key insight in production code: GPUs pipeline everything. While triangle N is being rasterized, triangle N+1 is being set up, and triangle N+2 is being fetched from memory. The Z-buffer lives in fast on-chip memory (tile-based rendering) so that depth comparisons don't require main memory bandwidth.

## Read the Source

- **Mesa `swrast`:** `src/gallium/drivers/swrast/s_tris.c` — software triangle rasterization with Z-buffer
- **tinyrenderer:** `github.com/ssloy/tinyrenderer` — the canonical "write a renderer from scratch" tutorial, lesson 2 covers Z-buffer

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. It is:

- **`depth_cheatsheet.md`** — A one-page reference card with Z-buffer algorithm, precision formulas, clipping pipeline, and culling decision flowchart.

## Exercises

1. **Easy** — Modify the C++ code to use `1/z` interpolation instead of raw `z`. Verify that z-fighting near the far plane is reduced.

2. **Medium** — Implement Sutherland-Hodgman polygon clipping in homogeneous coordinates. Test by placing a triangle that straddles the near plane and verify no inverted geometry.

3. **Hard** — Implement hierarchical Z-buffer (HiZ): maintain a mip pyramid of depth buffers. Before rasterizing a triangle, test its bounding box against the coarsest depth mip level. If the entire bbox is behind the stored depth, skip it entirely. Measure the performance improvement on a scene with heavy occlusion.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Z-buffer | "depth buffer" | A per-pixel array storing the closest depth so far; fragments pass only if their depth is less |
| Z-fighting | "flickering" | Two surfaces at nearly the same depth alternating which one wins, due to insufficient depth precision |
| 1/z interpolation | "perspective-correct depth" | Interpolating the reciprocal of depth (which *is* linear in screen space) rather than raw z |
| Backface culling | "hull culling" | Discarding triangles whose normal faces away from the camera, eliminating ~50% of geometry in closed meshes |
| Sutherland-Hodgman | "polygon clipper" | An algorithm that clips a polygon against each plane of a convex volume in sequence, producing a new polygon |
| Clip space | "homogeneous coordinates before divide" | The coordinate space after projection but before the perspective divide; where clipping must happen |
| Frustum culling | "view culling" | Discarding entire objects whose bounding volumes don't intersect the view frustum |
| Signed area | "cross product z-component" | The 2D cross product of triangle edges; positive = CCW (front-facing), negative = CW (back-facing) |

## Further Reading

- **Real-Time Rendering, 4th ed.** — Tomas Akenine-Möller et al., Chapter 2 (The Graphics Pipeline) and Chapter 23 (Intersection Test Methods)
- **tinyrenderer** — Dmitry Sokolov, `github.com/ssloy/tinyrenderer` — Lessons 2-4 walk through Z-buffer, projection, and clipping from scratch
- **Scratchapixel** — `scratchapixel.com` — "Rendering Pipeline" and "Rasterization" articles
- **OpenGL Wiki: Depth Buffer Precision** — `khronos.org/opengl/wiki/Depth_Buffer_Precision` — Detailed explanation of z-fighting and reversed-Z
- **Sutherland-Hodgman original paper** — Sutherland, I.E., Hodgman, G.W. (1974). "Reentrant Polygon Clipping." Communications of the ACM, 17(1), 32-42.