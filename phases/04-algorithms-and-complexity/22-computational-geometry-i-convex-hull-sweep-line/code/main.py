"""
Computational Geometry I — Convex Hull, Sweep Line
Phase 04 — Algorithms & Complexity Analysis

Implementations:
  - cross product / orientation test
  - Graham scan  O(n log n)
  - Jarvis march  O(n h)
  - Closest pair (sweep line)  O(n log n)
  - Segment intersection detection
"""

import math
from typing import List, Tuple, Optional

Point = Tuple[float, float]
Segment = Tuple[Point, Point]


# ---------------------------------------------------------------------------
# Primitives
# ---------------------------------------------------------------------------

def cross(o: Point, a: Point, b: Point) -> float:
    """Cross product of vectors o->a and o->b.
    Positive => CCW, Negative => CW, Zero => collinear.
    """
    return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])


def dist2(a: Point, b: Point) -> float:
    """Squared Euclidean distance."""
    return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2


def dist(a: Point, b: Point) -> float:
    """Euclidean distance."""
    return math.sqrt(dist2(a, b))


# ---------------------------------------------------------------------------
# ASCII visualisation
# ---------------------------------------------------------------------------

def ascii_plot(points: List[Point], hull: Optional[List[Point]] = None,
               width: int = 60, height: int = 20) -> str:
    """Render points and optional hull in an ASCII grid."""
    if not points:
        return "(no points)"

    xs = [p[0] for p in points]
    ys = [p[1] for p in points]
    min_x, max_x = min(xs), max(xs)
    min_y, max_y = min(ys), max(ys)

    # Guard against degenerate range
    if max_x == min_x:
        max_x = min_x + 1
    if max_y == min_y:
        max_y = min_y + 1

    grid = [[' ' for _ in range(width)] for _ in range(height)]

    def to_grid(px, py):
        gx = int((px - min_x) / (max_x - min_x) * (width - 1))
        gy = int((1 - (py - min_y) / (max_y - min_y)) * (height - 1))
        gx = max(0, min(width - 1, gx))
        gy = max(0, min(height - 1, gy))
        return gx, gy

    # Draw hull edges as '.'
    if hull and len(hull) >= 2:
        for i in range(len(hull)):
            x0, y0 = to_grid(hull[i][0], hull[i][1])
            x1, y1 = to_grid(hull[(i + 1) % len(hull)][0], hull[(i + 1) % len(hull)][1])
            steps = max(abs(x1 - x0), abs(y1 - y0), 1)
            for s in range(steps + 1):
                cx = x0 + int(round((x1 - x0) * s / steps))
                cy = y0 + int(round((y1 - y0) * s / steps))
                if grid[cy][cx] == ' ':
                    grid[cy][cx] = '.'

    # Draw hull vertices as '#'
    if hull:
        for hx, hy in hull:
            gx, gy = to_grid(hx, hy)
            grid[gy][gx] = '#'

    # Draw interior points as '*'
    for px, py in points:
        gx, gy = to_grid(px, py)
        if grid[gy][gx] == ' ':
            grid[gy][gx] = '*'

    border = '+' + '-' * width + '+'
    rows = [border]
    for row in grid:
        rows.append('|' + ''.join(row) + '|')
    rows.append(border)
    return '\n'.join(rows)


# ---------------------------------------------------------------------------
# Convex Hull: Graham Scan
# ---------------------------------------------------------------------------

def graham_scan(points: List[Point]) -> List[Point]:
    """Convex hull via Graham scan. Returns hull vertices in CCW order.

    Complexity: O(n log n) — dominated by the sort.
    """
    if len(points) <= 2:
        return list(points)

    # Step 1: find lowest point (lowest y, then leftmost x)
    start = min(points, key=lambda p: (p[1], p[0]))

    # Step 2: sort by polar angle from start
    def polar_key(p):
        return math.atan2(p[1] - start[1], p[0] - start[0])

    sorted_pts = sorted(points, key=polar_key)

    # Step 3: build hull — pop on right turns (cross <= 0)
    hull: List[Point] = []
    for p in sorted_pts:
        while len(hull) >= 2 and cross(hull[-2], hull[-1], p) <= 0:
            hull.pop()
        hull.append(p)

    return hull


# ---------------------------------------------------------------------------
# Convex Hull: Jarvis March (Gift Wrapping)
# ---------------------------------------------------------------------------

def jarvis_march(points: List[Point]) -> List[Point]:
    """Convex hull via gift wrapping. Returns hull vertices in CCW order.

    Complexity: O(n h) where h = hull size.
    """
    if len(points) <= 2:
        return list(points)

    start = min(points, key=lambda p: (p[0], p[1]))
    hull: List[Point] = []
    current = start

    while True:
        hull.append(current)
        # Pick a candidate that is not current
        candidate = points[0] if points[0] != current else points[1]

        for p in points:
            if p == current:
                continue
            cp = cross(current, candidate, p)
            if cp < 0:
                candidate = p
            elif cp == 0:
                # Collinear — take the farther one
                if dist2(current, p) > dist2(current, candidate):
                    candidate = p

        current = candidate
        if current == start:
            break

    return hull


# ---------------------------------------------------------------------------
# Closest Pair (Sweep Line)
# ---------------------------------------------------------------------------

def closest_pair_sweep(points: List[Point]) -> Tuple[float, Optional[Tuple[Point, Point]]]:
    """Find closest pair of points using sweep line.

    Complexity: O(n log n) — sort + active-set maintenance.

    Returns (distance, (point_a, point_b)).
    """
    if len(points) < 2:
        return float('inf'), None

    pts = sorted(points, key=lambda p: p[0])  # sort by x
    best = float('inf')
    best_pair: Optional[Tuple[Point, Point]] = None
    active: List[Point] = []

    j = 0
    for p in pts:
        # Evict points whose x is more than best behind the sweep line
        while j < len(pts) and pts[j][0] < p[0] - best:
            if pts[j] in active:
                active.remove(pts[j])
            j += 1

        # Build strip: active points within vertical distance < best
        strip = [q for q in active if abs(q[1] - p[1]) < best]
        strip.sort(key=lambda q: q[1])

        for q in strip:
            d = dist(p, q)
            if d < best:
                best = d
                best_pair = (p, q)

        active.append(p)

    return best, best_pair


# ---------------------------------------------------------------------------
# Segment Intersection
# ---------------------------------------------------------------------------

def _on_segment(p: Point, q: Point, r: Point) -> bool:
    """Is r on segment p-q (assuming collinear)?"""
    return (min(p[0], q[0]) <= r[0] <= max(p[0], q[0]) and
            min(p[1], q[1]) <= r[1] <= max(p[1], q[1]))


def segments_intersect(s1: Segment, s2: Segment) -> bool:
    """Check whether two line segments (p1,p2) and (p3,p4) intersect."""
    p1, p2 = s1
    p3, p4 = s2

    d1 = cross(p3, p4, p1)
    d2 = cross(p3, p4, p2)
    d3 = cross(p1, p2, p3)
    d4 = cross(p1, p2, p4)

    if ((d1 > 0 and d2 < 0) or (d1 < 0 and d2 > 0)) and \
       ((d3 > 0 and d4 < 0) or (d3 < 0 and d4 > 0)):
        return True

    # Collinear endpoint cases
    if d1 == 0 and _on_segment(p3, p4, p1):
        return True
    if d2 == 0 and _on_segment(p3, p4, p2):
        return True
    if d3 == 0 and _on_segment(p1, p2, p3):
        return True
    if d4 == 0 and _on_segment(p1, p2, p4):
        return True

    return False


# ---------------------------------------------------------------------------
# Demo
# ---------------------------------------------------------------------------

def main() -> None:
    import random
    random.seed(42)

    points = [(random.randint(0, 99), random.randint(0, 99)) for _ in range(40)]

    print("=" * 62)
    print("  Computational Geometry I — Convex Hull, Sweep Line")
    print("=" * 62)

    # --- Graham Scan ---
    hull_graham = graham_scan(points)
    print(f"\n--- Graham Scan ---")
    print(f"Points: {len(points)},  Hull vertices: {len(hull_graham)}")
    print(ascii_plot(points, hull_graham))

    # --- Jarvis March ---
    hull_jarvis = jarvis_march(points)
    print(f"\n--- Jarvis March ---")
    print(f"Points: {len(points)},  Hull vertices: {len(hull_jarvis)}")

    # Verify both hulls produce the same set of vertices
    set_graham = set(hull_graham)
    set_jarvis = set(hull_jarvis)
    print(f"Hull match: {set_graham == set_jarvis}")

    # --- Orientation demo ---
    print("\n--- Orientation Test ---")
    o, a, b = (0, 0), (1, 0), (1, 1)
    print(f"cross{o}->{a}->{b} = {cross(o, a, b):.1f}  (CCW)")
    o, a, b = (0, 0), (1, 0), (1, -1)
    print(f"cross{o}->{a}->{b} = {cross(o, a, b):.1f}  (CW)")
    o, a, b = (0, 0), (1, 0), (2, 0)
    print(f"cross{o}->{a}->{b} = {cross(o, a, b):.1f}  (collinear)")

    # --- Closest Pair ---
    d, pair = closest_pair_sweep(points)
    print(f"\n--- Closest Pair (Sweep Line) ---")
    print(f"Distance: {d:.4f}")
    if pair:
        print(f"Points: {pair[0]} and {pair[1]}")

    # --- Segment Intersection ---
    print("\n--- Segment Intersection ---")
    s1 = ((0, 0), (4, 4))
    s2 = ((0, 4), (4, 0))
    s3 = ((0, 0), (2, 2))
    s4 = ((3, 3), (5, 5))
    print(f"Segments (0,0)-(4,4) and (0,4)-(4,0): {segments_intersect(s1, s2)}")
    print(f"Segments (0,0)-(2,2) and (3,3)-(5,5): {segments_intersect(s3, s4)}")
    print(f"Segments (0,0)-(4,4) and (3,3)-(5,5): {segments_intersect(s1, s4)}")

    # --- Hull area (shoelace formula) ---
    def hull_area(hull: List[Point]) -> float:
        n = len(hull)
        area = 0.0
        for i in range(n):
            j = (i + 1) % n
            area += hull[i][0] * hull[j][1]
            area -= hull[j][0] * hull[i][1]
        return abs(area) / 2.0

    area = hull_area(hull_graham)
    print(f"\n--- Hull Area (Shoelace) ---")
    print(f"Convex hull area: {area:.2f}")


if __name__ == "__main__":
    main()
