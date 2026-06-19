# Depth & Culling Cheatsheet

## Z-buffer Algorithm

```
Initialize: zbuf[x][y] = +∞ for all pixels
For each triangle (v0, v1, v2):
    For each pixel (x,y) in bounding box:
        Compute barycentric (w0, w1, w2) at (x,y)
        If any wi < 0: skip (outside triangle)
        depth = w0*v0.z + w1*v1.z + w2*v2.z
        If depth < zbuf[x][y]:
            zbuf[x][y] = depth
            color[x][y] = shade(triangle, w0, w1, w2)
```

## Precision: 1/z vs z

```
Raw z in screen space:    z(x,y) = w0*v0.z + w1*v1.z + w2*v2.z    ← WRONG
1/z in screen space:  1/z(x,y) = w0/v0.z + w1/v1.z + w2/v2.z    ← CORRECT
z(x,y) = 1 / (1/z(x,y))

Precision distribution (linear z buffer):
  Near plane: ████████████████████ (most precision)
  Far plane:  ██                     (least precision)

Remedies for z-fighting:
  1. Push near plane out (biggest impact)
  2. Pull far plane in
  3. Use floating-point depth buffer (D32F)
  4. Use reversed depth: near→1.0, far→0.0
  5. Polygon offset for co-planar surfaces
```

## Backface Culling

```
Signed Area Test (screen space):
  area = 0.5 * ((v1.x-v0.x)*(v2.y-v0.y) - (v2.x-v0.x)*(v1.y-v0.y))
  area > 0  →  CCW  →  front-facing  →  KEEP
  area < 0  →  CW   →  back-facing   →  CULL
  area = 0  →  degenerate triangle

3D View-Vector Test (world space):
  normal = cross(v1-v0, v2-v0)
  view   = camera_pos - v0
  dot(normal, view) > 0  →  KEEP
  dot(normal, view) ≤ 0  →  CULL

When backface culling fails:
  - Double-sided geometry (leaves, cloth, paper)
  - Mirrors / reprojected geometry
  - Inconsistent winding order in mesh
  - Orthographic: use signed-area, not view vector
```

## Clipping Pipeline

```
View Frustum (6 planes in homogeneous clip space):
  Left:   x + w ≥ 0      Right:  w - x ≥ 0
  Bottom: y + w ≥ 0      Top:    w - y ≥ 0
  Near:   z + w ≥ 0      Far:    w - z ≥ 0

Sutherland-Hodgman (per plane):
  For each edge (S→E) in polygon:
    S inside, E inside    → output E
    S inside, E outside   → output intersection(S,E,plane)
    S outside, E inside   → output intersection(S,E,plane), then E
    S outside, E outside   → output nothing

  Clip against all 6 planes in sequence.
  May produce 3–7 vertices → fan into triangles.
```

## Triangle Clipping at Near Plane

```
Intersection parameter:
  t = (near_z - S.z) / (E.z - S.z)    [for near plane at z = near_z]
  clipped_vertex = S + t * (E - S)

One vertex inside  → 1 triangle  (inside vertex + 2 intersection points)
Two vertices inside → 2 triangles (quad from 2 inside + 2 intersection points)
All inside         → 0 clipping needed
All outside        →  triangle discarded
```

## Culling Hierarchy (Cheapest → Most Expensive)

```
┌─────────────────────────────────────────────────────┐
│  1. FRUSTUM CULLING    (object-level, CPU)           │
│     Test bounding sphere/AABB vs 6 frustum planes   │
│     Cost: O(N) objects, ~6 dot products each        │
│     Eliminates: everything outside the view          │
├─────────────────────────────────────────────────────┤
│  2. BACKFACE CULLING   (triangle-level, GPU)         │
│     Test signed area of screen-space triangle       │
│     Cost: ~3 multiplies per triangle                │
│     Eliminates: ~50% of triangles in closed meshes  │
├─────────────────────────────────────────────────────┤
│  3. OCCLUSION CULLING  (pixel-level, GPU)            │
│     Z-buffer depth test per fragment                │
│     Cost: 1 comparison per fragment per triangle     │
│     Eliminates: all surfaces hidden behind closer    │
│     geometry — the final arbiter of visibility      │
└─────────────────────────────────────────────────────┘
```

## Scanline Optimization

```
Bounding box: iterate only over [min_x..max_x] × [min_y..max_y]
  min_x = floor(min(v0.x, v1.x, v2.x))
  max_x = ceil(max(v0.x, v1.x, v2.x))
  (same for y)

Edge walking (production rasterizers):
  For each scanline y:
    Find x_left  = intersection with left edge
    Find x_right = intersection with right edge
    Fill pixels from x_left to x_right
  Avoids barycentric test for interior pixels.
```