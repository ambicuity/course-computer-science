// Computational Geometry I — Convex Hull, Sweep Line
// Phase 04 — Algorithms & Complexity Analysis
//
// C++ geometry toolkit: cross product, Graham scan, Jarvis march,
// closest pair sweep line, segment intersection.
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <iostream>
#include <random>
#include <vector>

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

struct Point {
    double x, y;
    bool operator<(const Point& o) const {
        return std::tie(x, y) < std::tie(o.x, o.y);
    }
    bool operator==(const Point& o) const {
        return x == o.x && y == o.y;
    }
};

// ---------------------------------------------------------------------------
// Primitives
// ---------------------------------------------------------------------------

// Cross product of vectors o->a and o->b.
// Positive => CCW, Negative => CW, Zero => collinear.
static double cross(const Point& o, const Point& a, const Point& b) {
    return (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
}

static double dist2(const Point& a, const Point& b) {
    double dx = a.x - b.x, dy = a.y - b.y;
    return dx * dx + dy * dy;
}

static double dist(const Point& a, const Point& b) {
    return std::sqrt(dist2(a, b));
}

// ---------------------------------------------------------------------------
// Convex Hull: Graham Scan  O(n log n)
// ---------------------------------------------------------------------------

std::vector<Point> graham_scan(std::vector<Point> pts) {
    if (pts.size() <= 2) return pts;

    // Step 1: lowest point (y, then x)
    int idx = 0;
    for (int i = 1; i < (int)pts.size(); ++i) {
        if (pts[i].y < pts[idx].y ||
            (pts[i].y == pts[idx].y && pts[i].x < pts[idx].x))
            idx = i;
    }
    std::swap(pts[0], pts[idx]);
    Point start = pts[0];

    // Step 2: sort by polar angle from start
    std::sort(pts.begin() + 1, pts.end(),
              [&start](const Point& a, const Point& b) {
                  double ang_a = std::atan2(a.y - start.y, a.x - start.x);
                  double ang_b = std::atan2(b.y - start.y, b.x - start.x);
                  return ang_a < ang_b;
              });

    // Step 3: build hull with stack
    std::vector<Point> hull;
    for (auto& p : pts) {
        while (hull.size() >= 2 &&
               cross(hull[hull.size() - 2], hull.back(), p) <= 0)
            hull.pop_back();
        hull.push_back(p);
    }
    return hull;
}

// ---------------------------------------------------------------------------
// Convex Hull: Jarvis March (Gift Wrapping)  O(n h)
// ---------------------------------------------------------------------------

std::vector<Point> jarvis_march(const std::vector<Point>& pts) {
    if (pts.size() <= 2) return pts;

    // Leftmost point
    int start_idx = 0;
    for (int i = 1; i < (int)pts.size(); ++i) {
        if (pts[i].x < pts[start_idx].x ||
            (pts[i].x == pts[start_idx].x && pts[i].y < pts[start_idx].y))
            start_idx = i;
    }

    std::vector<Point> hull;
    int current = start_idx;

    while (true) {
        hull.push_back(pts[current]);
        int candidate = (current != 0) ? 0 : 1;

        for (int i = 0; i < (int)pts.size(); ++i) {
            if (i == current) continue;
            double cp = cross(pts[current], pts[candidate], pts[i]);
            if (cp < 0) {
                candidate = i;
            } else if (cp == 0) {
                if (dist2(pts[current], pts[i]) > dist2(pts[current], pts[candidate]))
                    candidate = i;
            }
        }

        current = candidate;
        if (current == start_idx) break;
    }
    return hull;
}

// ---------------------------------------------------------------------------
// Closest Pair (Sweep Line)  O(n log n)
// ---------------------------------------------------------------------------

struct ClosestResult {
    double dist;
    Point a, b;
    bool valid;
};

ClosestResult closest_pair_sweep(std::vector<Point> pts) {
    ClosestResult res{1e18, {}, {}, false};
    if (pts.size() < 2) return res;

    std::sort(pts.begin(), pts.end());
    std::vector<Point> active;

    int j = 0;
    for (auto& p : pts) {
        while (j < (int)pts.size() && pts[j].x < p.x - res.dist)
            ++j;

        // Build strip from active points within vertical distance < best
        active.clear();
        for (int k = j; k < (int)pts.size() && pts[k].x <= p.x + res.dist; ++k) {
            if (pts[k] == p) continue;
            if (std::abs(pts[k].y - p.y) < res.dist)
                active.push_back(pts[k]);
        }
        std::sort(active.begin(), active.end(),
                  [](const Point& a, const Point& b) { return a.y < b.y; });

        for (auto& q : active) {
            double d = dist(p, q);
            if (d < res.dist) {
                res.dist = d;
                res.a = p;
                res.b = q;
                res.valid = true;
            }
        }
    }
    return res;
}

// ---------------------------------------------------------------------------
// Segment Intersection
// ---------------------------------------------------------------------------

static bool on_segment(const Point& p, const Point& q, const Point& r) {
    return (std::min(p.x, q.x) <= r.x && r.x <= std::max(p.x, q.x) &&
            std::min(p.y, q.y) <= r.y && r.y <= std::max(p.y, q.y));
}

bool segments_intersect(const Point& p1, const Point& p2,
                        const Point& p3, const Point& p4) {
    double d1 = cross(p3, p4, p1);
    double d2 = cross(p3, p4, p2);
    double d3 = cross(p1, p2, p3);
    double d4 = cross(p1, p2, p4);

    if (((d1 > 0 && d2 < 0) || (d1 < 0 && d2 > 0)) &&
        ((d3 > 0 && d4 < 0) || (d3 < 0 && d4 > 0)))
        return true;

    if (d1 == 0 && on_segment(p3, p4, p1)) return true;
    if (d2 == 0 && on_segment(p3, p4, p2)) return true;
    if (d3 == 0 && on_segment(p1, p2, p3)) return true;
    if (d4 == 0 && on_segment(p1, p2, p4)) return true;
    return false;
}

// ---------------------------------------------------------------------------
// Hull Area (Shoelace Formula)
// ---------------------------------------------------------------------------

double hull_area(const std::vector<Point>& hull) {
    double area = 0.0;
    int n = (int)hull.size();
    for (int i = 0; i < n; ++i) {
        int j = (i + 1) % n;
        area += hull[i].x * hull[j].y;
        area -= hull[j].x * hull[i].y;
    }
    return std::abs(area) / 2.0;
}

// ---------------------------------------------------------------------------
// Demo
// ---------------------------------------------------------------------------

int main() {
    std::mt19937 rng(42);
    std::uniform_real_distribution<double> dist_rand(0.0, 99.0);

    const int N = 40;
    std::vector<Point> points(N);
    for (auto& p : points) {
        p.x = dist_rand(rng);
        p.y = dist_rand(rng);
    }

    std::cout << "==========================================\n";
    std::cout << "  Computational Geometry I — C++ Toolkit\n";
    std::cout << "==========================================\n\n";

    // --- Graham Scan ---
    auto hull_graham = graham_scan(points);
    std::cout << "--- Graham Scan ---\n";
    std::cout << "Points: " << points.size()
              << "  Hull vertices: " << hull_graham.size() << "\n";

    // --- Jarvis March ---
    auto hull_jarvis = jarvis_march(points);
    std::cout << "\n--- Jarvis March ---\n";
    std::cout << "Points: " << points.size()
              << "  Hull vertices: " << hull_jarvis.size() << "\n";

    // --- Orientation demo ---
    std::cout << "\n--- Orientation Test ---\n";
    Point o{0, 0}, a{1, 0}, b_ccw{1, 1}, b_cw{1, -1}, b_col{2, 0};
    std::cout << "cross (0,0)->(1,0)->(1,1)  = " << cross(o, a, b_ccw) << "  (CCW)\n";
    std::cout << "cross (0,0)->(1,0)->(1,-1) = " << cross(o, a, b_cw) << "  (CW)\n";
    std::cout << "cross (0,0)->(1,0)->(2,0)  = " << cross(o, a, b_col) << "  (collinear)\n";

    // --- Closest Pair ---
    auto cp = closest_pair_sweep(points);
    std::cout << "\n--- Closest Pair (Sweep Line) ---\n";
    if (cp.valid) {
        std::cout << "Distance: " << cp.dist << "\n";
        std::cout << "Points: (" << cp.a.x << "," << cp.a.y
                  << ") and (" << cp.b.x << "," << cp.b.y << ")\n";
    }

    // --- Segment Intersection ---
    std::cout << "\n--- Segment Intersection ---\n";
    std::cout << "(0,0)-(4,4) vs (0,4)-(4,0): "
              << segments_intersect({0, 0}, {4, 4}, {0, 4}, {4, 0}) << "\n";
    std::cout << "(0,0)-(2,2) vs (3,3)-(5,5): "
              << segments_intersect({0, 0}, {2, 2}, {3, 3}, {5, 5}) << "\n";
    std::cout << "(0,0)-(4,4) vs (3,3)-(5,5): "
              << segments_intersect({0, 0}, {4, 4}, {3, 3}, {5, 5}) << "\n";

    // --- Hull Area ---
    double area = hull_area(hull_graham);
    std::cout << "\n--- Hull Area (Shoelace) ---\n";
    std::cout << "Convex hull area: " << area << "\n";

    return 0;
}
