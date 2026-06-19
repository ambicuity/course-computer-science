# Rasterization I — Lines and Triangles

> How a continuous mathematical object becomes a discrete grid of pixels.

**Type:** Learn
**Languages:** Rust, C++
**Prerequisites:** Phase 14 lessons 01–03
**Time:** ~75 minutes

## Learning Objectives

- Derive Bresenham's line algorithm from first principles and implement it using only integer arithmetic.
- Explain the edge function and use it to test whether a point lies inside a triangle.
- Compute barycentric coordinates as area ratios and use them for attribute interpolation.
- Apply the top-left fill rule to resolve tie-breaking on shared edges.
- Reason about why triangles — not quads or n-gons — are the universal rasterization primitive.

## The Problem

You have a continuous 2D coordinate — say, the endpoint of a line at (3.7, 5.2) — but your screen is a grid of integer pixel positions. *Which* pixel do you light up? And if you pick (4, 5), what about the pixel at (3, 5) — is it also "on the line"?

Without a deterministic answer, every GPU and software renderer would make different choices. Gaps appear between adjacent triangles. Lines wander. The image flickers.

Rasterization is the set of rules that converts continuous geometry into discrete fragments. Rules that must be:

1. **Deterministic** — same input, same output, always.
2. **Conservative** — no gaps between adjacent primitives.
3. **Efficient** — computed per-pixel, per-triangle, billions of times per frame.

This lesson covers the two most fundamental rasterization algorithms: line drawing (Bresenham) and triangle filling (edge-equation / barycentric).

## The Concept: Line Drawing

### The Naive Approach

Draw a line from (x0, y0) to (x1, y1). The simplest idea:

```
for x from x0 to x1:
    y = y0 + (y1 - y0) * (x - x0) / (x1 - x0)
    plot(x, round(y))
```

This works, but every step requires a floating-point multiply and divide. On early hardware — and on today's GPUs processing millions of edges per frame — that's wasteful. Worse, floating-point round-off can cause the line to drift by a pixel.

### Bresenham's Algorithm: Integer-Only Line Drawing

Jack Bresenham (1962) observed that you can decide which pixel to pick using *only additions and sign checks*. Here is the derivation for lines where `0 ≤ dy ≤ dx` (shallow lines going right and slightly up).

**Setup:** We walk `x` one pixel at a time. At each step, we choose between two pixels:

```
     (x+1, y)      — the pixel straight right
     (x+1, y+1)    — the pixel diagonally up-right
```

We pick whichever pixel is closer to the *true* line `y = mx + b`.

**The decision parameter.** Define the error term:

```
d = 2·dy·x - 2·dx·y + 2·dx·y0 - 2·dy·x0 + dx
```

`d` is always an integer if all inputs are integers. At each step:

- If `d > 0`: the line is above the midpoint, pick `(x+1, y+1)` and update `d = d + 2·(dy - dx)`
- If `d ≤ 0`: the line is at or below the midpoint, pick `(x+1, y)` and update `d = d + 2·dy`

That's it. Two integer additions per pixel. No multiply, no divide, no float.

**Worked example.** Line from (1, 1) to (8, 5). Here `dx = 7`, `dy = 4`.

```
Initial d = 2·4 - 2·7 + 7 = 1

x=1, y=1, d=1  → d>0 → pick (2,2), d = 1 + 2·(4-7) = -5
x=2, y=2, d=-5 → d≤0 → pick (3,2), d = -5 + 2·4 = 3
x=3, y=2, d=3  → d>0 → pick (4,3), d = 3 + 2·(4-7) = -3
x=4, y=3, d=-3 → d≤0 → pick (5,3), d = -3 + 2·4 = 5
x=5, y=3, d=5  → d>0 → pick (6,4), d = 5 + 2·(4-7) = -1
x=6, y=4, d=-1 → d≤0 → pick (7,4), d = -1 + 2·4 = 7
x=7, y=4, d=7  → d>0 → pick (8,5), done
```

The plotted pixels: (1,1) (2,2) (3,2) (4,3) (5,3) (6,4) (7,4) (8,5).

```
y
5 |              ●
4 |          ●   ●
3 |      ●   ●
2 |  ●   ●
1 |  ●
  +----------------→ x
    1 2 3 4 5 6 7 8
```

### Steep Lines

When `dy > dx`, the line is steep. Walking `x` one pixel at a time would skip `y` values. Solution: swap roles — walk `y` and decide `x`. More generally:

```
if abs(dx) >= abs(dy):  step along x
else:                    step along y
```

For each octant, the increments to `d` change sign accordingly, but the structure is the same.

### Endpoint Convention

Lines drawn from A to B and from B to A should produce the same set of pixels. That means the algorithm must handle both `(x0 < x1)` and `(x0 > x1)` — swap if needed so the primary axis always increments.

## The Concept: Triangle Rasterization

### Why Triangles?

Before we discuss *how* to rasterize, why triangles specifically?

1. **Always planar.** Three points define a plane. A quad can be non-planar (a twisted quad), causing ambiguous rasterization.
2. **Always convex.** Every point on the segment between any two points inside the triangle is also inside. No holes, no self-intersections.
3. **Decomposable.** Any polygon can be triangulated. Any polygon mesh can be converted to triangles. GPUs only need one primitive.
4. **Well-defined interior.** For a triangle, "inside" is unambiguous. For a self-intersecting polygon, it's not.

### The Edge Function

Given a triangle with vertices `V0`, `V1`, `V2` in counter-clockwise order, define the **edge function** for each edge:

```
E01(P) = (V1.x - V0.x)·(P.y - V0.y) - (V1.y - V0.y)·(P.x - V0.x)
E12(P) = (V2.x - V1.x)·(P.y - V1.y) - (V2.y - V1.y)·(P.x - V1.x)
E20(P) = (V0.x - V2.x)·(P.y - V2.y) - (V0.y - V2.y)·(P.x - V2.x)
```

This is the 2D cross product (V_a → V_b) × (V_a → P). The sign tells you which side of the edge P is on:

- `E > 0` → P is on the left side (inside for CCW triangles)
- `E < 0` → P is on the right side (outside for CCW triangles)
- `E = 0` → P is exactly on the edge

A pixel center P is inside the triangle iff all three edge functions are ≥ 0 (for CCW winding) or all ≤ 0 (for CW winding).

```
          V2
         / \
        /   \
       /  ●  \     ← ● is inside (all E > 0)
      /       \
    V0--------V1

           V2
          / \
         /   \
        /  ×  \     ← × is outside (E12 < 0)
       /       \
     V0--------V1
```

### Barycentric Coordinates

The edge function values are *not* just inside/outside tests — they are proportional to the area of sub-triangles, which gives us barycentric coordinates.

Given the three edge function values `E01(P)`, `E12(P)`, `E20(P)`, and the total edge function `E_total = E01(P_any) + E12(P_any) + E20(P_any)` (which is constant for any P inside), the barycentric coordinates are:

```
w0 = E12(P) / E_total    (weight of V0)
w1 = E20(P) / E_total    (weight of V1)
w2 = E01(P) / E_total    (weight of V2)
```

These satisfy `w0 + w1 + w2 = 1` and interpolate any vertex attribute A:

```
A(P) = w0·A0 + w1·A1 + w2·A2
```

**Why area ratios?** `E12(P)` measures the signed area of the triangle (P, V1, V2). The full triangle area is `E_total`. The ratio of the sub-triangle to the full triangle gives the barycentric weight of the opposite vertex.

```
       V2
      /|\
     / | \
    / w0|  \
   /    |   \
  V0----P----V1

  w0 = area(P,V1,V2) / area(V0,V1,V2)
  w1 = area(P,V0,V2) / area(V0,V1,V2)
  w2 = area(P,V0,V1) / area(V0,V1,V2)
```

This is what makes barycentric interpolation *perspective-correct* when combined with the w-divide (covered in a later lesson).

### The Top-Left Rule

When a pixel center lies exactly on a shared edge of two triangles, which triangle "owns" it? Without a rule, either both triangles claim it (double-rendering) or neither does (a gap).

The **top-left rule** resolves this:

- A pixel on a **horizontal** edge (top edge) is owned by the triangle *above* it.
- A pixel on a **non-horizontal** edge that is a **left** edge is owned by the triangle on the left side.

More precisely, an edge is classified as "top" if it is horizontal and the triangle interior is below it. An edge is "left" if it goes upward from left to right, or is vertical with the interior to the right.

In practice, this is implemented as: a pixel is inside the triangle iff all edge functions are > 0, *or* the edge function is 0 on a top or left edge.

```
Triangle A    |  Triangle B
  /\          |    /
 /  \    ←---|→ /
/____\   shared edge
              |
  Top-left rule: pixels exactly on the
  shared edge belong to the triangle whose
  edge is classified as "top" or "left".
```

### Scanline vs. Edge-Equation

Two approaches to triangle rasterization:

**Scanline rasterization:** Walk y row by row. For each row, compute the left and right x from the edge slopes. Fill all pixels in between. Fast, but requires sorting edges and handling special cases (flat tops, flat bottoms).

**Edge-equation rasterization:** For each pixel in the bounding box of the triangle, evaluate the three edge functions. If all non-negative, the pixel is inside. Simple, parallelizable (GPU-friendly), but evaluates pixels that are clearly outside near the corners of the bounding box.

Modern GPUs use the edge-equation approach (or a hybrid with bounding-box pruning) because it parallelizes trivially across pixel quad groups.

## Build It

### Step 1: Minimal Bresenham

The minimal version draws a line for the first octant (0 ≤ slope ≤ 1):

```cpp
void bresenham(int x0, int y0, int x1, int y1, Image& img) {
    int dx = x1 - x0;
    int dy = y1 - y0;
    int d = 2 * dy - dx;
    int y = y0;
    for (int x = x0; x <= x1; x++) {
        img.set(x, y, white);
        if (d > 0) { y++; d -= 2 * dx; }
        d += 2 * dy;
    }
}
```

### Step 2: General Bresenham

The realistic version handles all 8 octants by considering the signs of dx and dy, and stepping along whichever axis has the larger absolute magnitude:

```cpp
void bresenham_general(int x0, int y0, int x1, int y1, Image& img) {
    bool steep = abs(y1 - y0) > abs(x1 - x0);
    if (steep) { swap(x0,y0); swap(x1,y1); }
    if (x0 > x1) { swap(x0,x1); swap(y0,y1); }
    int dx = x1 - x0, dy = abs(y1 - y0);
    int err = dx / 2;
    int ystep = (y0 < y1) ? 1 : -1;
    int y = y0;
    for (int x = x0; x <= x1; x++) {
        img.set(steep ? y : x, steep ? x : y, white);
        err -= dy;
        if (err < 0) { y += ystep; err += dx; }
    }
}
```

### Step 3: Triangle Rasterization with Barycentric Interpolation

```cpp
void rasterize_triangle(Vec2 v0, Vec2 v1, Vec2 v2, Image& img) {
    int minX = min({v0.x, v1.x, v2.x});
    int maxX = max({v0.x, v1.x, v2.x});
    int minY = min({v0.y, v1.y, v2.y});
    int maxY = max({v0.y, v1.y, v2.y});
    float total = edge(v0, v1, v2);
    for (int y = minY; y <= maxY; y++) {
        for (int x = minX; x <= maxX; x++) {
            Vec2 p = {float(x), float(y)};
            float w0 = edge(p, v1, v2);
            float w1 = edge(p, v2, v0);
            float w2 = edge(p, v0, v1);
            if (w0 >= 0 && w1 >= 0 && w2 >= 0) {
                Color c = w0/total*c0 + w1/total*c1 + w2/total*c2;
                img.set(x, y, c);
            }
        }
    }
}
```

## Use It

In production graphics:

- **GPU hardware** implements Bresenham-like line drawing in fixed-function units, but triangle rasterization uses edge equations evaluated in parallel across 2×2 pixel quads.
- **OpenGL/Vulkan/DirectX** specify the top-left fill rule precisely. Read the Vulkan spec (Section 27.8 "Rasterization") for the exact tie-breaking rules.
- **Software renderers** (like `tinyrenderer` by Dmitry Sokolov) implement these same algorithms in plain C++ — exactly the code you've just written.

The difference between your code and a GPU? A GPU evaluates all three edge functions for 8 pixels simultaneously, using SIMD lanes. The math is the same.

## Read the Source

- `tinyrenderer` by Dmitry Sokolov — `tgaimage.cpp` and the line/triangle lessons at https://github.com/ssloy/tinyrenderer
- Mesa3D source — `src/gallium/auxiliary/util/u_rect.c` for software rasterization patterns

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`rasterizer_reference.md`** — A quick-reference card with Bresenham's pseudocode, edge function formula, barycentric derivation, and tie-breaking rules.

## Exercises

1. **Easy** — Implement Bresenham's algorithm from memory and draw a line on a blank PPM image.
2. **Medium** — Modify the triangle rasterizer to use the top-left fill rule for pixels exactly on edges. Test with two triangles sharing a diagonal edge and verify no gaps or double-writes.
3. **Hard** — Implement a scanline triangle rasterizer that computes span endpoints per row using edge slopes. Benchmark it against the edge-equation version for triangles of various sizes. Which is faster for thin, tall triangles? For wide, flat ones?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|----------------------|
| Bresenham's algorithm | "The line drawing algorithm" | An integer-only incremental algorithm that uses an error accumulator to decide which pixel to light up next, avoiding all floating-point arithmetic |
| Decision parameter | "The d variable in Bresenham" | Twice the signed vertical distance from the midpoint between two candidate pixels to the true line; its sign chooses the next pixel |
| Edge function | "The 2D cross product" | A signed area computation `E(P) = (B-A)×(P-A)` whose sign determines which side of edge AB a point P falls on |
| Barycentric coordinates | "The triangle interpolation weights" | Three values (w0, w1, w2) that sum to 1, expressing point P as a weighted combination of the three vertices; each weight is a sub-triangle area ratio |
| Top-left rule | "Tie-breaking rule" | When a pixel center lies exactly on a shared edge, it belongs to the triangle for which that edge is classified as top or left, ensuring no gaps and no double-writes |
| Rasterization | "Making pixels from geometry" | The process of determining which discrete pixel locations are covered by a continuous geometric primitive, governed by deterministic fill rules |

## Further Reading

- Bresenham, J.E. "Algorithm for computer control of a digital plotter." *IBM Systems Journal* 4(1), 1965.
- Akenine-Möller, T., Haines, E., & Hoffman, N. *Real-Time Rendering*, 4th ed., Chapter 23 "Rasterization."
- Pineda, J. "A Parallel Algorithm for Polygon Rasterization." *SIGGRAPH '88.* (Introduced the edge-function approach used in GPUs.)
- Vulkan Specification, Section 27.8: "Rasterization" — the definitive reference for fill rules.