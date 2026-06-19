# Computational Geometry I — Convex Hull, Sweep Line

> Turn a cloud of points into a shape, and find the nearest pair in O(n log n) — the algorithms that underpin maps, games, and collision detection.

**Type:** Learn
**Languages:** Python, C++
**Prerequisites:** Phase 04 lessons 01–21
**Time:** ~75 minutes

## Learning Objectives

- Implement geometric primitives: cross product, orientation test (CCW/collinear/CW)
- Build Graham scan (O(n log n)) and Jarvis march (O(nh)) convex hull algorithms from scratch
- Apply the sweep-line paradigm to closest-pair and line-segment intersection problems
- Recognise where these algorithms appear in GIS, game engines, and collision-detection systems

## The Problem

You have a set of n points on a 2D plane. You need to find the smallest convex polygon enclosing all of them (the **convex hull**), or find which two points are closest, or detect whether line segments intersect. Brute-force on these problems is O(n²) or worse. Computational geometry gives us sub-quadratic algorithms by exploiting the structure of 2D space.

## The Concept

### Geometric Primitives

Everything in 2D computational geometry rests on one primitive: the **cross product** of two vectors.

```
cross(o, a, b) = (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
```

This is the signed area of the triangle (o, a, b). Its sign tells us the **orientation**:

| Result | Meaning |
|--------|---------|
| > 0 | Counter-clockwise (CCW) — point b is left of vector o→a |
| < 0 | Clockwise (CW) — point b is right of vector o→a |
| = 0 | Collinear — all three points on one line |

```
        b              b
       /              /
      / CCW          / CW
     o → a          o → a
```

**Distance** between points: `dist(a, b) = sqrt((a.x - b.x)² + (a.y - b.y)²)`. Squared distance avoids the sqrt when only comparisons matter.

### Convex Hull — Graham Scan

Graham scan finds the convex hull in O(n log n) by sorting points by polar angle around a reference point, then walking the sorted list with a stack, keeping only left turns.

**Algorithm:**

1. Pick the point with the lowest y-coordinate (lowest x as tiebreaker) as `p0`. This is guaranteed to be on the hull.
2. Sort all other points by the polar angle they make with `p0`.
3. Walk the sorted list with a stack. For each point:
   - While the stack has ≥ 2 points and `cross(stack[-2], stack[-1], point) <= 0`, pop the top (it makes a right turn or is collinear — it's inside the hull).
   - Push the point.
4. The stack holds the hull in CCW order.

The stack invariant: at every step, the points in the stack form a convex chain with only left turns.

```
Sort by angle from p0:

         p4
        /
    p2 /   p5
     \/   /
  p0 ----p3
    |  p1
    |/

Walk: push p0, p1, p2.
  See p3: cross(p1, p2, p3) <= 0? pop p2. push p3.
  Continue...
```

**Complexity:** The sort is O(n log n). Each point is pushed once and popped at most once, so the scan is O(n). Total: **O(n log n)**.

### Convex Hull — Jarvis March (Gift Wrapping)

Jarvis march wraps the hull one point at a time: start at the leftmost point, find the next point with the smallest polar angle (the most CCW point), repeat.

**Algorithm:**

1. Start at the leftmost point `p0`.
2. Set `current = p0` and `candidate = next point`.
3. For every other point `q`: if `cross(current, candidate, q) < 0`, set `candidate = q` (q is more CCW).
4. `candidate` is the next hull point. If it's `p0`, we're done. Otherwise, repeat from step 2.

**Complexity:** O(nh) where h is the number of hull points. Each of the h hull vertices requires scanning all n points.

### When Each Wins

| Algorithm | Time | Best when |
|-----------|------|-----------|
| Graham scan | O(n log n) | n is large, need optimal worst-case |
| Jarvis march | O(nh) | h is small relative to n (few hull points) |

If h = O(log n), Jarvis march is O(n log n) — same as Graham scan but simpler. If h = O(n), Jarvis degrades to O(n²).

### Sweep Line — Closest Pair

The sweep-line paradigm moves a vertical line left-to-right across the plane, maintaining an **active set** of points near the sweep line. The closest pair algorithm:

**Algorithm:**

1. Sort points by x-coordinate.
2. Maintain the active set: points within distance d of the sweep line, where d is the current best pair distance.
3. When considering a new point:
   - Remove active-set points more than d behind the sweep line.
   - For the new point, only check distances to the nearest 6 points in the active set sorted by y-coordinate (a geometric argument shows checking 6 is sufficient).
   - Update d if a closer pair is found.

```
  Active set (sorted by y):
  ┌─────────────┐
  │ .  .        │  strip of width 2d
  │    . .      │  ← new point checked
  │  .   .      │    against ≤ 6 neighbors
  │      .      │    in y-sorted order
  └─────────────┘
      sweep line →
```

**Complexity:** Sort by x is O(n log n). Each point is inserted once, removed once, and checked against ≤ 6 neighbors → O(n log n) total (the log n comes from maintaining y-sorted order in a balanced BST).

### Sweep Line — Line Segment Intersection

The Bentley-Ottmann algorithm detects all k intersections among n segments in O((n + k) log n):

1. **Events** = segment endpoints + intersection points, sorted by x-coordinate.
2. **Active set** = segments currently crossing the sweep line, ordered by y at the current x.
3. Process each event: add/remove segments from active set, check only adjacent segments for new intersections (since only neighbors can cross next).

For a simpler version that just detects *whether any two segments intersect*, sort endpoints left-to-right and maintain the active set with a balanced BST keyed by y-coordinate at the current x.

## Build It

### Step 1: Geometric Primitives

```python
def cross(o, a, b):
    """Cross product of vectors o→a and o→b. Positive = CCW."""
    return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

def dist2(a, b):
    """Squared distance between two points."""
    return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2
```

### Step 2: Graham Scan

```python
import math

def graham_scan(points):
    if len(points) <= 2:
        return list(points)

    # Step 1: find lowest point
    start = min(points, key=lambda p: (p[1], p[0]))

    # Step 2: sort by polar angle with start
    def angle_key(p):
        return math.atan2(p[1] - start[1], p[0] - start[0])

    sorted_pts = sorted(points, key=angle_key)

    # Step 3: build hull with stack
    hull = []
    for p in sorted_pts:
        while len(hull) >= 2 and cross(hull[-2], hull[-1], p) <= 0:
            hull.pop()
        hull.append(p)

    return hull
```

### Step 3: Jarvis March

```python
def jarvis_march(points):
    if len(points) <= 2:
        return list(points)

    # Start at leftmost point
    start = min(points, key=lambda p: (p[0], p[1]))
    hull = []
    current = start

    while True:
        hull.append(current)
        candidate = points[0] if points[0] != current else points[1]

        for p in points:
            if p == current:
                continue
            cp = cross(current, candidate, p)
            if cp < 0 or (cp == 0 and dist2(current, p) > dist2(current, candidate)):
                candidate = p

        current = candidate
        if current == start:
            break

    return hull
```

### Step 4: Closest Pair (Sweep Line)

```python
def closest_pair_sweep(points):
    pts = sorted(points)
    best = float('inf')
    best_pair = None
    active = []

    j = 0
    for i, p in enumerate(pts):
        # Remove points too far behind
        while pts[j][0] < p[0] - best:
            active.remove(pts[j])
            j += 1

        # Check neighbors in y-sorted active set (strip of width 2*best)
        strip = [q for q in active if abs(q[1] - p[1]) < best]
        strip.sort(key=lambda q: q[1])

        for q in strip:
            d = dist2(p, q) ** 0.5
            if d < best:
                best = d
                best_pair = (p, q)

        active.append(p)

    return best, best_pair
```

### Step 5: Segment Intersection Detection

```python
from sortedcontainers import SortedList

def segments_intersect(s1, s2):
    """Check if two segments (p1,p2) and (p3,p4) intersect."""
    def ccw(a, b, c):
        return cross(a, b, c)

    p1, p2 = s1
    p3, p4 = s2

    d1 = ccw(p3, p4, p1)
    d2 = ccw(p3, p4, p2)
    d3 = ccw(p1, p2, p3)
    d4 = ccw(p1, p2, p4)

    if ((d1 > 0 and d2 < 0) or (d1 < 0 and d2 > 0)) and \
       ((d3 > 0 and d4 < 0) or (d3 < 0 and d4 > 0)):
        return True

    # Collinear cases
    if d1 == 0 and _on_segment(p3, p4, p1): return True
    if d2 == 0 and _on_segment(p3, p4, p2): return True
    if d3 == 0 and _on_segment(p1, p2, p3): return True
    if d4 == 0 and _on_segment(p1, p2, p4): return True

    return False

def _on_segment(p, q, r):
    """Is point r on segment p-q (assuming collinear)?"""
    return min(p[0], q[0]) <= r[0] <= max(p[0], q[0]) and \
           min(p[1], q[1]) <= r[1] <= max(p[1], q[1])
```

## Use It

- **GIS / mapping:** Google Maps, OpenStreetMap compute convex hulls of waypoints for bounding boxes, route simplification, and coverage regions.
- **Game engines:** Unity and Unreal use convex hull decomposition for collision detection — convex hulls allow fast GJK/EPA algorithms.
- **Robotics:** Motion planning (Minkowski sums) requires convex hull computation to compute configuration space obstacles.
- **3D scanning:** Convex hulls of point clouds approximate object surfaces; CGAL and PCL provide industrial implementations.

Production implementations (CGAL, scipy.spatial.ConvexHull) handle degeneracies (collinear points, duplicate points, numerical robustness) that our toy code skips. The core algorithm, however, is identical.

### Read the Source

- [scipy.spatial.ConvexHull](https://github.com/scipy/scipy/blob/main/scipy/spatial/_qhull.pyx) — wraps Qhull, a production O(n log n) hull algorithm in C. Note the numerical robustness handling.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A geometry toolkit** — `graham_scan`, `closest_pair_sweep`, and orientation primitives you can drop into any 2D problem.

## Exercises

1. **Easy** — Compute the area of a convex hull given its CCW-ordered vertices. (Hint: the shoelace formula — sum over edges of `x_i * y_{i+1} - x_{i+1} * y_i`, divide by 2.)
2. **Medium** — Implement **rotating calipers** on the convex hull to find the diameter (maximum pairwise distance) in O(h).
3. **Hard** — Find the **minimum area bounding rectangle** of a convex hull using rotating calipers. The rectangle has one side flush with a hull edge — iterate over all h edges, use calipers to find the opposite side and the perpendicular extremes.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Convex hull | "The smallest polygon containing all points" | The intersection of all convex sets containing the point set; boundary is a sequence of left turns |
| Cross product | "Which way do we turn" | Signed area of triangle (o,a,b); sign encodes CW/CCW/collinear orientation |
| CCW | "Left turn" | Counter-clockwise orientation — points wind around a center counterclockwise |
| Sweep line | "Scan left to right" | A vertical line moving across the plane; the active set tracks geometry crossing it |
| Active set | "Points near the sweep line" | The data structure maintained during sweep; only points within some distance are relevant |
| Gift wrapping | "Wrap the points in string" | Jarvis march — find the next hull vertex by searching for the most-CCW point |
| Rotating calipers | "Calipers around the hull" | Technique that walks antipodal pairs of hull edges in O(h) to compute diameter, width, bounding box |

## Further Reading

- [Computational Geometry: Algorithms and Applications](https://link.springer.com/book/10.1007/978-3-540-77974-2) — de Berg et al., chapters 1–3 (Convex Hulls) and 11 (Voronoi/Delaunay as bonus)
- [Competitive Programming by Steven Halim](https://cpbook.net/) — chapter 7 covers geometry in contest settings
- [CGAL Documentation](https://doc.cgal.org/latest/Manual/packages.html) — production C++ geometry library
