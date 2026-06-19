# Rasterization Reference Card

Quick-reference for Bresenham's line algorithm, the edge function, barycentric coordinates, and rasterization tie-breaking rules.

---

## Bresenham's Line Algorithm (Integer-Only)

### Pseudocode (All Octants)

```
bresenham(x0, y0, x1, y1):
    steep = |y1 - y0| > |x1 - x0|
    if steep:  swap(x0, y0); swap(x1, y1)
    if x0 > x1: swap(x0, x1); swap(y0, y1)

    dx = x1 - x0
    dy = |y1 - y0|
    err = dx / 2
    ystep = (y0 < y1) ? 1 : -1
    y = y0

    for x = x0 to x1:
        plot(steep ? (y, x) : (x, y))
        err = err - dy
        if err < 0:
            y = y + ystep
            err = err + dx
```

### Decision Parameter Derivation

For a line `y = mx + b` with `m = dy/dx`, at step `x` we choose between:
- `(x+1, y)` (straight right)
- `(x+1, y+1)` (diagonal up-right)

The true line at `x+1` is at height `y_exact`. The midpoint between candidates is `y + 0.5`.

The decision parameter `d` = 2·(distance from midpoint to true line)·dx, simplified so that:

| Condition | Action | Update |
|-----------|--------|--------|
| `d > 0` (line above midpoint) | Pick `(x+1, y+1)` | `d = d + 2·(dy - dx)` |
| `d ≤ 0` (line at/below midpoint) | Pick `(x+1, y)` | `d = d + 2·dy` |

Initial value: `d = 2·dy - dx`.

---

## Edge Function

For edge from vertex A to vertex B, evaluated at point P:

```
E_AB(P) = (B.x - A.x)·(P.y - A.y) - (B.y - A.y)·(P.x - A.x)
```

This is the 2D cross product `(B - A) × (P - A)`.

### Inside/Outside Test

For triangle (V0, V1, V2) in **counter-clockwise** winding:

```
E01(P) = edge_function(V0, V1, P)
E12(P) = edge_function(V1, V2, P)
E20(P) = edge_function(V2, V0, P)

P is inside iff:  E01 ≥ 0  AND  E12 ≥ 0  AND  E20 ≥ 0
```

For **clockwise** winding: P is inside iff all three are ≤ 0.

---

## Barycentric Coordinates

Given the three edge function values and the total (signed) area:

```
Area   = E01(P_any) + E12(P_any) + E20(P_any)   (constant for all P)

w0 = E12(P) / Area    (weight of V0 — opposite to edge V1→V2)
w1 = E20(P) / Area    (weight of V1 — opposite to edge V2→V0)
w2 = E01(P) / Area    (weight of V2 — opposite to edge V0→V1)
```

Properties:
- `w0 + w1 + w2 = 1`
- `w0, w1, w2 ≥ 0` for interior points
- At vertex Vi, `wi = 1` and the other weights are 0

### Interpolation

Any per-vertex attribute A interpolates linearly:

```
A(P) = w0·A0 + w1·A1 + w2·A2
```

Works for: color, depth, texture coordinates, normals, etc.

### Area Ratio Interpretation

```
w0 = Area(V1, V2, P) / Area(V0, V1, V2)
w1 = Area(V2, V0, P) / Area(V0, V1, V2)
w2 = Area(V0, V1, P) / Area(V0, V1, V2)
```

Each weight is the ratio of the sub-triangle opposite the corresponding vertex to the full triangle.

---

## Top-Left Fill Rule

When a pixel center lies exactly on a shared edge, this rule ensures exactly one triangle claims it.

### Edge Classification

An edge is a **top edge** if:
- It is horizontal (both endpoints have the same y)
- The interior of the triangle is below it (the third vertex has a larger y)

An edge is a **left edge** if:
- It is not horizontal AND going from bottom to top (y increasing)
- OR it is vertical (same x for both endpoints) and the interior is to the right

### Rule

A pixel on edge E is owned by the triangle for which E is either a top edge or a left edge. A pixel on an edge that is neither top nor left for a triangle is not owned by that triangle.

This guarantees: no gaps, no double-writes, deterministic results.

---

## Scanline vs. Edge-Equation Comparison

| Property | Scanline | Edge-Equation |
|----------|----------|---------------|
| Approach | Walk y, compute x spans per row | Evaluate 3 edge functions per pixel in bounding box |
| Parallelism | Serial per scanline | Trivially parallel (each pixel independent) |
| Wasted work | Minimal (only spans inside triangle) | Evaluates pixels in bounding-box corners that are outside |
| GPU suitability | Poor | Excellent (SIMD across pixel quads) |
| Edge cases | Flat tops, flat bottoms need special handling | Uniform — just sign checks |
| Typical use | Software renderers | GPU hardware |