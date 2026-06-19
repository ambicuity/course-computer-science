// Rasterization I — Lines and Triangles
// Phase 14 — Computer Graphics & Visualization
//
// Renders a PPM image (output.ppm) demonstrating:
//   1. Bresenham line drawing (several lines of varying slope)
//   2. A filled triangle with barycentric color interpolation
//   3. Two overlapping triangles showing rasterization rules

#include <cmath>
#include <cstdint>
#include <algorithm>
#include <fstream>
#include <iostream>
#include <array>
#include <vector>

struct Vec2 {
    float x, y;
};

struct Color {
    uint8_t r, g, b;
    Color operator+(const Color& o) const { return {uint8_t(r+o.r), uint8_t(g+o.g), uint8_t(b+o.b)}; }
    Color operator*(float s) const { return {uint8_t(r*s), uint8_t(g*s), uint8_t(b*s)}; }
};

const Color BLACK = {0, 0, 0};
const Color WHITE = {255, 255, 255};
const Color RED   = {255, 0, 0};
const Color GREEN = {0, 255, 0};
const Color BLUE  = {0, 0, 255};

class Image {
    int w, h;
    std::vector<Color> pixels;
public:
    Image(int w, int h) : w(w), h(h), pixels(w * h, BLACK) {}
    int width() const { return w; }
    int height() const { return h; }
    void set(int x, int y, Color c) {
        if (x >= 0 && x < w && y >= 0 && y < h)
            pixels[y * w + x] = c;
    }
    Color get(int x, int y) const {
        if (x >= 0 && x < w && y >= 0 && y < h)
            return pixels[y * w + x];
        return BLACK;
    }
    bool write_ppm(const char* filename) const {
        std::ofstream f(filename, std::ios::binary);
        if (!f) return false;
        f << "P6\n" << w << " " << h << "\n255\n";
        f.write(reinterpret_cast<const char*>(pixels.data()), pixels.size() * 3);
        return f.good();
    }
};

float edge_function(const Vec2& a, const Vec2& b, const Vec2& p) {
    return (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
}

void bresenham(int x0, int y0, int x1, int y1, Image& img, Color c) {
    bool steep = std::abs(y1 - y0) > std::abs(x1 - x0);
    if (steep) { std::swap(x0, y0); std::swap(x1, y1); }
    if (x0 > x1) { std::swap(x0, x1); std::swap(y0, y1); }
    int dx = x1 - x0;
    int dy = std::abs(y1 - y0);
    int err = dx / 2;
    int ystep = (y0 < y1) ? 1 : -1;
    int y = y0;
    for (int x = x0; x <= x1; x++) {
        if (steep) img.set(y, x, c);
        else       img.set(x, y, c);
        err -= dy;
        if (err < 0) { y += ystep; err += dx; }
    }
}

void rasterize_triangle(Vec2 v0, Vec2 v1, Vec2 v2,
                         Color c0, Color c1, Color c2,
                         Image& img) {
    int minX = std::max(0, (int)std::floor(std::min({v0.x, v1.x, v2.x})));
    int maxX = std::min(img.width() - 1, (int)std::ceil(std::max({v0.x, v1.x, v2.x})));
    int minY = std::max(0, (int)std::floor(std::min({v0.y, v1.y, v2.y})));
    int maxY = std::min(img.height() - 1, (int)std::ceil(std::max({v0.y, v1.y, v2.y})));

    float area = edge_function(v0, v1, v2);
    if (std::abs(area) < 1e-6f) return;

    for (int y = minY; y <= maxY; y++) {
        for (int x = minX; x <= maxX; x++) {
            Vec2 p = {float(x) + 0.5f, float(y) + 0.5f};
            float w0 = edge_function(p, v1, v2);
            float w1 = edge_function(p, v2, v0);
            float w2 = edge_function(p, v0, v1);
            bool inside = (area > 0) ? (w0 >= 0 && w1 >= 0 && w2 >= 0)
                                      : (w0 <= 0 && w1 <= 0 && w2 <= 0);
            if (inside) {
                float inv = 1.0f / area;
                w0 *= inv; w1 *= inv; w2 *= inv;
                uint8_t r = uint8_t(std::min(255.f, w0 * c0.r + w1 * c1.r + w2 * c2.r));
                uint8_t g = uint8_t(std::min(255.f, w0 * c0.g + w1 * c1.g + w2 * c2.g));
                uint8_t b = uint8_t(std::min(255.f, w0 * c0.b + w1 * c1.b + w2 * c2.b));
                img.set(x, y, {r, g, b});
            }
        }
    }
}

void rasterize_triangle_flat(Vec2 v0, Vec2 v1, Vec2 v2, Color c, Image& img) {
    rasterize_triangle(v0, v1, v2, c, c, c, img);
}

void draw_bresenham_demo(Image& img) {
    int cx = img.width() / 2;
    int cy = img.height() / 2;
    int len = 60;
    bresenham(cx, cy, cx + len, cy, img, WHITE);
    bresenham(cx, cy, cx + len, cy + len/3, img, WHITE);
    bresenham(cx, cy, cx + len/3, cy + len, img, WHITE);
    bresenham(cx, cy, cx, cy + len, img, WHITE);
    bresenham(cx, cy, cx - len/3, cy + len, img, WHITE);
    bresenham(cx, cy, cx - len, cy + len/3, img, WHITE);
    bresenham(cx, cy, cx - len, cy, img, WHITE);
    bresenham(cx, cy, cx - len, cy - len/3, img, WHITE);
    bresenham(cx, cy, cx - len/3, cy - len, img, WHITE);
    bresenham(cx, cy, cx, cy - len, img, WHITE);
    bresenham(cx, cy, cx + len/3, cy - len, img, WHITE);
    bresenham(cx, cy, cx + len, cy - len/3, img, WHITE);
}

void draw_interpolated_triangle(Image& img, int offX, int offY) {
    Vec2 v0 = {float(offX), float(offY)};
    Vec2 v1 = {float(offX + 120), float(offY)};
    Vec2 v2 = {float(offX + 60), float(offY + 100)};
    Color c0 = RED, c1 = GREEN, c2 = BLUE;
    rasterize_triangle(v0, v1, v2, c0, c1, c2, img);
    bresenham(int(v0.x), int(v0.y), int(v1.x), int(v1.y), img, WHITE);
    bresenham(int(v1.x), int(v1.y), int(v2.x), int(v2.y), img, WHITE);
    bresenham(int(v2.x), int(v2.y), int(v0.x), int(v0.y), img, WHITE);
}

void draw_overlapping_triangles(Image& img, int offX, int offY) {
    int s = 50;
    Vec2 a0 = {float(offX), float(offY)};
    Vec2 a1 = {float(offX + 2*s), float(offY)};
    Vec2 a2 = {float(offX + s), float(offY + 2*s)};
    Vec2 b0 = {float(offX + s), float(offY)};
    Vec2 b1 = {float(offX + 3*s), float(offY)};
    Vec2 b2 = {float(offX + 2*s), float(offY + 2*s)};
    Color ca = {200, 50, 50};
    Color cb = {50, 50, 200};
    rasterize_triangle_flat(a0, a1, a2, ca, img);
    rasterize_triangle_flat(b0, b1, b2, cb, img);
    bresenham(int(a0.x), int(a0.y), int(a1.x), int(a1.y), img, WHITE);
    bresenham(int(a1.x), int(a1.y), int(a2.x), int(a2.y), img, WHITE);
    bresenham(int(a2.x), int(a2.y), int(a0.x), int(a0.y), img, WHITE);
    bresenham(int(b0.x), int(b0.y), int(b1.x), int(b1.y), img, WHITE);
    bresenham(int(b1.x), int(b1.y), int(b2.x), int(b2.y), img, WHITE);
    bresenham(int(b2.x), int(b2.y), int(b0.x), int(b0.y), img, WHITE);
}

int main() {
    const int W = 256, H = 400;
    Image img(W, H);

    for (int y = 180; y < 185; ++y)
        for (int x = 0; x < W; ++x)
            img.set(x, y, {40, 40, 40});

    draw_bresenham_demo(img);
    draw_interpolated_triangle(img, 50, 200);
    draw_overlapping_triangles(img, 140, 200);

    if (img.write_ppm("output.ppm")) {
        std::cout << "Wrote output.ppm (" << W << "x" << H << ")\n";
        return 0;
    } else {
        std::cerr << "Failed to write output.ppm\n";
        return 1;
    }
}