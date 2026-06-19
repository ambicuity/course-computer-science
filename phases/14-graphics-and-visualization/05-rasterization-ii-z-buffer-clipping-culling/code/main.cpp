#include <cmath>
#include <algorithm>
#include <fstream>
#include <iostream>
#include <limits>
#include <vector>

struct Vec3 {
    float x, y, z;
    Vec3(float x = 0, float y = 0, float z = 0) : x(x), y(y), z(z) {}
    Vec3 operator+(const Vec3& b) const { return {x + b.x, y + b.y, z + b.z}; }
    Vec3 operator-(const Vec3& b) const { return {x - b.x, y - b.y, z - b.z}; }
    Vec3 operator*(float s) const { return {x * s, y * s, z * s}; }
};

struct Color {
    unsigned char r, g, b;
    Color(unsigned char r = 0, unsigned char g = 0, unsigned char b = 0)
        : r(r), g(g), b(b) {}
};

struct Triangle {
    Vec3 v0, v1, v2;
    Color c;
    bool two_sided;
};

Vec3 cross(const Vec3& a, const Vec3& b) {
    return {a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x};
}

float dot(const Vec3& a, const Vec3& b) {
    return a.x * b.x + a.y * b.y + a.z * b.z;
}

void write_ppm(const char* filename, const std::vector<Color>& fb, int w, int h) {
    std::ofstream f(filename, std::ios::binary);
    f << "P6\n" << w << " " << h << "\n255\n";
    for (int i = h - 1; i >= 0; --i) {
        for (int j = 0; j < w; ++j) {
            const Color& c = fb[i * w + j];
            f.put(c.r).put(c.g).put(c.b);
        }
    }
}

float signed_area_2d(const Vec3& a, const Vec3& b, const Vec3& c) {
    return 0.5f * ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y));
}

void rasterize_triangle(const Triangle& tri,
                         std::vector<Color>& fb,
                         std::vector<float>& zb,
                         int W, int H,
                         bool enable_backface) {
    float sa = signed_area_2d(tri.v0, tri.v1, tri.v2);

    if (enable_backface && !tri.two_sided && sa < 0) return;
    if (std::abs(sa) < 1e-6f) return;

    int min_x = std::max(0, (int)std::floor(std::min({tri.v0.x, tri.v1.x, tri.v2.x})));
    int max_x = std::min(W - 1, (int)std::ceil(std::max({tri.v0.x, tri.v1.x, tri.v2.x})));
    int min_y = std::max(0, (int)std::floor(std::min({tri.v0.y, tri.v1.y, tri.v2.y})));
    int max_y = std::min(H - 1, (int)std::ceil(std::max({tri.v0.y, tri.v1.y, tri.v2.y})));

    float inv_area = 1.0f / sa;

    for (int y = min_y; y <= max_y; ++y) {
        for (int x = min_x; x <= max_x; ++x) {
            float px = x + 0.5f, py = y + 0.5f;

            float u0 = ((tri.v1.x - px) * (tri.v2.y - py) - (tri.v2.x - px) * (tri.v1.y - py));
            float u1 = ((tri.v2.x - px) * (tri.v0.y - py) - (tri.v0.x - px) * (tri.v2.y - py));
            float u2 = ((tri.v0.x - px) * (tri.v1.y - py) - (tri.v1.x - px) * (tri.v0.y - py));

            u0 *= inv_area;
            u1 *= inv_area;
            u2 *= inv_area;

            if (u0 < -0.001f || u1 < -0.001f || u2 < -0.001f) continue;

            float depth = u0 * tri.v0.z + u1 * tri.v1.z + u2 * tri.v2.z;

            int idx = y * W + x;
            if (depth < zb[idx]) {
                zb[idx] = depth;
                fb[idx] = tri.c;
            }
        }
    }
}

void rasterize_triangle_invz(const Triangle& tri,
                               std::vector<Color>& fb,
                               std::vector<float>& zb,
                               int W, int H) {
    float sa = signed_area_2d(tri.v0, tri.v1, tri.v2);
    if (std::abs(sa) < 1e-6f) return;

    int min_x = std::max(0, (int)std::floor(std::min({tri.v0.x, tri.v1.x, tri.v2.x})));
    int max_x = std::min(W - 1, (int)std::ceil(std::max({tri.v0.x, tri.v1.x, tri.v2.x})));
    int min_y = std::max(0, (int)std::floor(std::min({tri.v0.y, tri.v1.y, tri.v2.y})));
    int max_y = std::min(H - 1, (int)std::ceil(std::max({tri.v0.y, tri.v1.y, tri.v2.y})));

    float inv_area = 1.0f / sa;

    for (int y = min_y; y <= max_y; ++y) {
        for (int x = min_x; x <= max_x; ++x) {
            float px = x + 0.5f, py = y + 0.5f;

            float u0 = ((tri.v1.x - px) * (tri.v2.y - py) - (tri.v2.x - px) * (tri.v1.y - py));
            float u1 = ((tri.v2.x - px) * (tri.v0.y - py) - (tri.v0.x - px) * (tri.v2.y - py));
            float u2 = ((tri.v0.x - px) * (tri.v1.y - py) - (tri.v1.x - px) * (tri.v0.y - py));

            u0 *= inv_area;
            u1 *= inv_area;
            u2 *= inv_area;

            if (u0 < -0.001f || u1 < -0.001f || u2 < -0.001f) continue;

            float inv_z0 = 1.0f / tri.v0.z;
            float inv_z1 = 1.0f / tri.v1.z;
            float inv_z2 = 1.0f / tri.v2.z;
            float inv_z_interp = u0 * inv_z0 + u1 * inv_z1 + u2 * inv_z2;
            float depth = 1.0f / inv_z_interp;

            int idx = y * W + x;
            if (depth < zb[idx]) {
                zb[idx] = depth;
                fb[idx] = tri.c;
            }
        }
    }
}

Vec3 sutherland_hodgman_clip(Vec3 s, Vec3 e, float plane_z, bool clip_near) {
    float d_s = clip_near ? (plane_z - s.z) : (s.z - plane_z);
    float d_e = clip_near ? (plane_z - e.z) : (e.z - plane_z);
    float t = d_s / (d_s - d_e);
    return s + (e - s) * t;
}

std::vector<Triangle> clip_triangle_near(const Triangle& tri, float near_z) {
    bool v0_in = tri.v0.z >= near_z;
    bool v1_in = tri.v1.z >= near_z;
    bool v2_in = tri.v2.z >= near_z;
    int inside = v0_in + v1_in + v2_in;

    if (inside == 3) return {tri};
    if (inside == 0) return {};

    std::vector<Triangle> result;

    if (inside == 1) {
        Vec3 vin = v0_in ? tri.v0 : (v1_in ? tri.v1 : tri.v2);
        Vec3 out1 = v0_in ? (v1_in ? tri.v2 : tri.v1) : tri.v0;
        Vec3 out2 = (v0_in && !v1_in) ? tri.v1 : ((v1_in && !v2_in) ? tri.v2 : tri.v0);
        if (!v0_in) { out1 = tri.v1; out2 = tri.v2; }
        else if (!v1_in) { out1 = tri.v0; out2 = tri.v2; }
        else { out1 = tri.v0; out2 = tri.v1; }
        vin = v0_in ? tri.v0 : (v1_in ? tri.v1 : tri.v2);

        Vec3 a = sutherland_hodgman_clip(vin, out1, near_z, true);
        Vec3 b = sutherland_hodgman_clip(vin, out2, near_z, true);
        result.push_back({vin, a, b, tri.c, tri.two_sided});
    } else {
        Vec3 out = !v0_in ? tri.v0 : (!v1_in ? tri.v1 : tri.v2);
        Vec3 in1, in2;
        if (!v0_in) { in1 = tri.v1; in2 = tri.v2; }
        else if (!v1_in) { in1 = tri.v0; in2 = tri.v2; }
        else { in1 = tri.v0; in2 = tri.v1; }

        Vec3 a = sutherland_hodgman_clip(in1, out, near_z, true);
        Vec3 b = sutherland_hodgman_clip(in2, out, near_z, true);
        result.push_back({in1, in2, a, tri.c, tri.two_sided});
        result.push_back({in2, b, a, tri.c, tri.two_sided});
    }

    return result;
}

void render_scene(const std::vector<Triangle>& tris, const char* filename,
                  int W, int H, bool backface, bool use_invz) {
    std::vector<Color> fb(W * H, Color(30, 30, 30));
    std::vector<float> zb(W * H, std::numeric_limits<float>::max());

    for (const auto& tri : tris) {
        auto clipped = clip_triangle_near(tri, 0.1f);
        for (auto& ct : clipped) {
            if (use_invz) {
                rasterize_triangle_invz(ct, fb, zb, W, H);
            } else {
                rasterize_triangle(ct, fb, zb, W, H, backface);
            }
        }
    }
    write_ppm(filename, fb, W, H);
    std::cout << "Wrote " << filename << "\n";
}

int main() {
    const int W = 400, H = 400;

    // Scene 1: Two overlapping triangles, correct depth ordering
    std::vector<Triangle> scene1 = {
        {{100, 50, 0.5f}, {350, 50, 0.4f}, {200, 300, 0.6f}, {220, 50, 50}, false},
        {{150, 100, 0.3f}, {350, 200, 0.35f}, {100, 350, 0.45f}, {50, 180, 50}, false},
    };
    render_scene(scene1, "zbuffer_overlapping.ppm", W, H, true, false);

    // Scene 2: Z-fighting demo — two nearly co-planar triangles fight for pixels
    std::vector<Triangle> scene2 = {
        {{80, 80, 0.500f}, {320, 80, 0.500f}, {200, 320, 0.500f}, {200, 60, 60}, false},
        {{100, 120, 0.5001f}, {300, 120, 0.5001f}, {200, 300, 0.5001f}, {60, 60, 200}, false},
    };
    render_scene(scene2, "zbuffer_zfighting.ppm", W, H, true, false);

    // Scene 3: Backface culling — front and back facing triangles
    std::vector<Triangle> scene3 = {
        {{50, 50, 0.5f}, {200, 50, 0.5f}, {125, 200, 0.5f}, {200, 50, 50}, false},
        {{250, 50, 0.4f}, {250, 200, 0.4f}, {400, 200, 0.4f}, {50, 200, 50}, false},
        {{250, 50, 0.3f}, {400, 200, 0.3f}, {250, 200, 0.3f}, {50, 50, 200}, true},
    };
    render_scene(scene3, "zbuffer_backface.ppm", W, H, true, false);

    // Scene 4: 1/z interpolation demo
    std::vector<Triangle> scene4 = {
        {{100, 50, 5.0f}, {350, 50, 50.0f}, {200, 300, 5.0f}, {220, 50, 50}, false},
        {{150, 150, 3.0f}, {350, 200, 30.0f}, {100, 350, 10.0f}, {50, 180, 50}, false},
    };
    render_scene(scene4, "zbuffer_invz.ppm", W, H, false, true);

    std::cout << "Z-buffer rasterizer complete. Check .ppm output files.\n";
    return 0;
}