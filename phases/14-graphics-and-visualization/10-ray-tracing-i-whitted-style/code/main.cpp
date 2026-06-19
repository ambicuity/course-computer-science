#include <cmath>
#include <fstream>
#include <iostream>
#include <algorithm>
#include <limits>
#include <vector>

struct Vec3 {
    double x, y, z;
    Vec3() : x(0), y(0), z(0) {}
    Vec3(double x, double y, double z) : x(x), y(y), z(z) {}
    Vec3 operator+(const Vec3& v) const { return {x+v.x, y+v.y, z+v.z}; }
    Vec3 operator-(const Vec3& v) const { return {x-v.x, y-v.y, z-v.z}; }
    Vec3 operator*(double s) const { return {x*s, y*s, z*s}; }
    Vec3 operator*(const Vec3& v) const { return {x*v.x, y*v.y, z*v.z}; }
    double dot(const Vec3& v) const { return x*v.x + y*v.y + z*v.z; }
    double length() const { return std::sqrt(x*x + y*y + z*z); }
    Vec3 normalized() const { double l = length(); return {x/l, y/l, z/l}; }
    Vec3 cross(const Vec3& v) const { return {y*v.z - z*v.y, z*v.x - x*v.z, x*v.y - y*v.x}; }
    Vec3 reflect(const Vec3& n) const { return *this - n * (2.0 * this->dot(n)); }
    Vec3 neg() const { return {-x, -y, -z}; }
};

Vec3 operator*(double s, const Vec3& v) { return v * s; }

struct Ray {
    Vec3 origin, direction;
    Vec3 at(double t) const { return origin + direction * t; }
};

struct Material {
    Vec3 color;
    double ambient, diffuse, specular, shininess;
    double reflectivity;
    double transparency, ior;
};

struct HitRecord {
    double t;
    Vec3 point, normal;
    Material material;
};

struct Sphere {
    Vec3 center;
    double radius;
    Material material;
    bool intersect(const Ray& ray, double t_min, double t_max, HitRecord& rec) const {
        Vec3 oc = ray.origin - center;
        double a = ray.direction.dot(ray.direction);
        double half_b = oc.dot(ray.direction);
        double c = oc.dot(oc) - radius * radius;
        double discriminant = half_b * half_b - a * c;
        if (discriminant < 0) return false;
        double sqrtd = std::sqrt(discriminant);
        double root = (-half_b - sqrtd) / a;
        if (root < t_min || root > t_max) {
            root = (-half_b + sqrtd) / a;
            if (root < t_min || root > t_max) return false;
        }
        rec.t = root;
        rec.point = ray.at(root);
        rec.normal = (rec.point - center) * (1.0 / radius);
        rec.material = material;
        if (ray.direction.dot(rec.normal) > 0)
            rec.normal = rec.normal * -1.0;
        return true;
    }
};

struct Plane {
    Vec3 point;
    Vec3 normal;
    Material material;
    bool intersect(const Ray& ray, double t_min, double t_max, HitRecord& rec) const {
        double denom = ray.direction.dot(normal);
        if (std::fabs(denom) < 1e-8) return false;
        double t = (point - ray.origin).dot(normal) / denom;
        if (t < t_min || t > t_max) return false;
        rec.t = t;
        rec.point = ray.at(t);
        rec.normal = normal;
        if (denom > 0)
            rec.normal = rec.normal * -1.0;
        rec.material = material;
        auto p = rec.point;
        int cx = (int)std::floor(p.x);
        int cz = (int)std::floor(p.z);
        if ((cx + cz) % 2 == 0)
            rec.material.color = Vec3(0.9, 0.9, 0.9);
        else
            rec.material.color = Vec3(0.3, 0.3, 0.3);
        rec.material.ambient = 0.05;
        rec.material.diffuse = 0.6;
        rec.material.specular = 0.2;
        rec.material.shininess = 10;
        rec.material.reflectivity = 0.2;
        rec.material.transparency = 0;
        rec.material.ior = 1.0;
        return true;
    }
};

struct Light {
    Vec3 position;
    Vec3 color;
    double intensity;
};

struct Scene {
    std::vector<Sphere> spheres;
    std::vector<Plane> planes;
    std::vector<Light> lights;
    Vec3 background;
    int max_depth;

    bool trace_ray(const Ray& ray, double t_min, double t_max, HitRecord& rec) const {
        HitRecord temp;
        bool hit_anything = false;
        double closest = t_max;
        for (const auto& s : spheres) {
            if (s.intersect(ray, t_min, closest, temp)) {
                hit_anything = true;
                closest = temp.t;
                rec = temp;
            }
        }
        for (const auto& p : planes) {
            if (p.intersect(ray, t_min, closest, temp)) {
                hit_anything = true;
                closest = temp.t;
                rec = temp;
            }
        }
        return hit_anything;
    }

    bool is_shadowed(const Vec3& point, const Vec3& light_dir, double light_dist) const {
        Ray shadow_ray{point, light_dir};
        HitRecord temp;
        return trace_ray(shadow_ray, 0.001, light_dist, temp);
    }

    Vec3 shade(const HitRecord& rec, const Ray& ray, int depth) const {
        Vec3 result{0, 0, 0};

        // Ambient
        result = result + rec.material.color * rec.material.ambient;

        // Diffuse + Specular with shadow test
        for (const auto& light : lights) {
            Vec3 to_light = light.position - rec.point;
            double dist = to_light.length();
            Vec3 L = to_light * (1.0 / dist);

            Vec3 shadow_origin = rec.point + rec.normal * 0.001;
            if (is_shadowed(shadow_origin, L, dist))
                continue;

            // Diffuse
            double diff = std::max(0.0, rec.normal.dot(L));
            result = result + rec.material.color * light.color * (rec.material.diffuse * diff * light.intensity);

            // Specular
            Vec3 R = L.neg().reflect(rec.normal);
            Vec3 V = (ray.origin - rec.point).normalized();
            double spec = std::pow(std::max(0.0, R.dot(V)), rec.material.shininess);
            result = result + light.color * (rec.material.specular * spec * light.intensity);
        }

        // Reflection
        if (rec.material.reflectivity > 0 && depth > 0) {
            Vec3 reflect_dir = ray.direction.reflect(rec.normal);
            Ray reflect_ray{rec.point + rec.normal * 0.001, reflect_dir};
            Vec3 reflect_color = compute_color(reflect_ray, depth - 1);
            result = result + reflect_color * rec.material.reflectivity;
        }

        // Refraction
        if (rec.material.transparency > 0 && depth > 0) {
            Vec3 refraction_dir;
            double kr = 1.0;
            Vec3 outward_normal = rec.normal;
            double ni_over_nt = rec.material.ior;
            bool entering = ray.direction.dot(rec.normal) < 0;

            if (entering) {
                outward_normal = rec.normal;
                ni_over_nt = 1.0 / rec.material.ior;
            } else {
                outward_normal = rec.normal * -1.0;
                ni_over_nt = rec.material.ior;
            }

            double cos_i = -ray.direction.dot(outward_normal);
            if (cos_i < 0) cos_i = -cos_i;
            double sin2_t = ni_over_nt * ni_over_nt * (1.0 - cos_i * cos_i);

            if (sin2_t <= 1.0) {
                double cos_t = std::sqrt(1.0 - sin2_t);
                refraction_dir = ray.direction * ni_over_nt
                    + outward_normal * (ni_over_nt * cos_i - cos_t);
                Ray refract_ray{rec.point - outward_normal * 0.001, refraction_dir.normalized()};
                Vec3 refract_color = compute_color(refract_ray, depth - 1);
                kr = 0.0;
                result = result + refract_color * rec.material.transparency;
            }
        }

        return result;
    }

    Vec3 compute_color(const Ray& ray, int depth) const {
        if (depth <= 0) return background;
        HitRecord rec;
        if (trace_ray(ray, 0.001, 1e9, rec))
            return shade(rec, ray, depth);
        return background;
    }
};

int clamp01(double v) { return std::min(255, std::max(0, (int)(v * 255))); }

int main() {
    const int W = 800, H = 600;

    Material red_mat{  {0.8, 0.2, 0.2}, 0.1, 0.6, 0.3, 50, 0.3, 0.0, 1.0 };
    Material blue_mat{ {0.2, 0.2, 0.8}, 0.1, 0.6, 0.3, 50, 0.3, 0.0, 1.0 };
    Material mirror_mat{{0.9, 0.9, 0.9}, 0.05, 0.2, 0.8, 200, 0.8, 0.0, 1.0 };
    Material glass_mat{ {1.0, 1.0, 1.0}, 0.05, 0.1, 0.3, 50, 0.1, 0.8, 1.5 };

    Scene scene;
    scene.background = {0.2, 0.3, 0.5};
    scene.max_depth = 5;
    scene.spheres.push_back({{0, 1, -4}, 1.0, red_mat});
    scene.spheres.push_back({{-2.5, 0.7, -3}, 0.7, mirror_mat});
    scene.spheres.push_back({{2.5, 1, -5}, 1.0, glass_mat});
    scene.spheres.push_back({{1.2, 0.5, -2}, 0.5, blue_mat});
    scene.planes.push_back({{0, 0, 0}, {0, 1, 0}, Material{}});
    scene.lights.push_back({{-5, 8, -2}, {1, 1, 1}, 1.0});
    scene.lights.push_back({{5, 6, 1}, {0.8, 0.8, 1.0}, 0.6});

    Vec3 cam_pos{0, 2, 2};
    Vec3 cam_target{0, 1, -3};
    Vec3 cam_up{0, 1, 0};
    Vec3 forward = (cam_target - cam_pos).normalized();
    Vec3 right = forward.cross(cam_up).normalized();
    Vec3 up = right.cross(forward);
    double fov = 60.0;
    double aspect = (double)W / H;
    double half_h = std::tan(fov * 0.5 * M_PI / 180.0);
    double half_w = half_h * aspect;

    std::ofstream out("output.ppm");
    out << "P3\n" << W << " " << H << "\n255\n";
    for (int j = 0; j < H; j++) {
        for (int i = 0; i < W; i++) {
            double u = (2.0 * (i + 0.5) / W - 1.0) * half_w;
            double v = (1.0 - 2.0 * (j + 0.5) / H) * half_h;
            Vec3 dir = (forward + right * u + up * v).normalized();
            Ray ray{cam_pos, dir};
            Vec3 col = scene.compute_color(ray, scene.max_depth);
            out << clamp01(col.x) << " " << clamp01(col.y) << " " << clamp01(col.z) << "\n";
        }
    }
    out.close();
    std::cerr << "Rendered " << W << "x" << H << " to output.ppm\n";
    return 0;
}