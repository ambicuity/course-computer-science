"""
Linear Algebra for Graphics — Transforms, Projections
Phase 14 — Computer Graphics & Visualization

Implements Vec3, Mat4 from scratch, demonstrates TR vs RT,
builds the perspective projection matrix, and renders a
wireframe cube to PPM via the full MVP pipeline.
"""

import math


class Vec3:
    def __init__(self, x=0.0, y=0.0, z=0.0):
        self.x = float(x)
        self.y = float(y)
        self.z = float(z)

    def __add__(self, other):
        return Vec3(self.x + other.x, self.y + other.y, self.z + other.z)

    def __sub__(self, other):
        return Vec3(self.x - other.x, self.y - other.y, self.z - other.z)

    def __mul__(self, scalar):
        return Vec3(self.x * scalar, self.y * scalar, self.z * scalar)

    def __rmul__(self, scalar):
        return self.__mul__(scalar)

    def __neg__(self):
        return Vec3(-self.x, -self.y, -self.z)

    def dot(self, other):
        return self.x * other.x + self.y * other.y + self.z * other.z

    def cross(self, other):
        return Vec3(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )

    def length(self):
        return math.sqrt(self.dot(self))

    def normalized(self):
        ln = self.length()
        if ln < 1e-12:
            return Vec3(0, 0, 0)
        return Vec3(self.x / ln, self.y / ln, self.z / ln)

    def to_vec4(self, w=1.0):
        return Vec4(self.x, self.y, self.z, w)

    def __repr__(self):
        return f"Vec3({self.x:.4f}, {self.y:.4f}, {self.z:.4f})"


class Vec4:
    def __init__(self, x=0.0, y=0.0, z=0.0, w=1.0):
        self.x = float(x)
        self.y = float(y)
        self.z = float(z)
        self.w = float(w)

    def perspective_divide(self):
        if abs(self.w) < 1e-12:
            return Vec3(0, 0, 0)
        return Vec3(self.x / self.w, self.y / self.w, self.z / self.w)

    def __repr__(self):
        return f"Vec4({self.x:.4f}, {self.y:.4f}, {self.z:.4f}, {self.w:.4f})"


class Mat4:
    def __init__(self, data=None):
        if data is None:
            self.m = [[1 if i == j else 0 for j in range(4)] for i in range(4)]
        else:
            self.m = [row[:] for row in data]

    @staticmethod
    def identity():
        return Mat4()

    @staticmethod
    def translation(tx, ty, tz):
        m = Mat4()
        m.m[0][3] = tx
        m.m[1][3] = ty
        m.m[2][3] = tz
        return m

    @staticmethod
    def scaling(sx, sy, sz):
        m = Mat4([[0] * 4 for _ in range(4)])
        m.m[0][0] = sx
        m.m[1][1] = sy
        m.m[2][2] = sz
        m.m[3][3] = 1
        return m

    @staticmethod
    def rotation_x(deg):
        r = math.radians(deg)
        c, s = math.cos(r), math.sin(r)
        m = Mat4()
        m.m[1][1] = c;  m.m[1][2] = -s
        m.m[2][1] = s;  m.m[2][2] = c
        return m

    @staticmethod
    def rotation_y(deg):
        r = math.radians(deg)
        c, s = math.cos(r), math.sin(r)
        m = Mat4()
        m.m[0][0] = c;   m.m[0][2] = s
        m.m[2][0] = -s;  m.m[2][2] = c
        return m

    @staticmethod
    def rotation_z(deg):
        r = math.radians(deg)
        c, s = math.cos(r), math.sin(r)
        m = Mat4()
        m.m[0][0] = c;  m.m[0][1] = -s
        m.m[1][0] = s;  m.m[1][1] = c
        return m

    @staticmethod
    def perspective(fov_deg, aspect, near, far):
        fov_rad = math.radians(fov_deg)
        f = 1.0 / math.tan(fov_rad / 2.0)
        m = Mat4([[0] * 4 for _ in range(4)])
        m.m[0][0] = f / aspect
        m.m[1][1] = f
        m.m[2][2] = -(far + near) / (far - near)
        m.m[2][3] = -(2 * far * near) / (far - near)
        m.m[3][2] = -1
        m.m[3][3] = 0
        return m

    @staticmethod
    def look_at(eye, target, up):
        forward = (target - eye).normalized()
        right = forward.cross(up).normalized()
        true_up = right.cross(forward)
        m = Mat4()
        m.m[0][0] = right.x;   m.m[0][1] = right.y;   m.m[0][2] = right.z;   m.m[0][3] = -right.dot(eye)
        m.m[1][0] = true_up.x; m.m[1][1] = true_up.y;  m.m[1][2] = true_up.z;  m.m[1][3] = -true_up.dot(eye)
        m.m[2][0] = -forward.x; m.m[2][1] = -forward.y; m.m[2][2] = -forward.z; m.m[2][3] = forward.dot(eye)
        m.m[3][0] = 0; m.m[3][1] = 0; m.m[3][2] = 0; m.m[3][3] = 1
        return m

    def __mul__(self, other):
        if isinstance(other, Mat4):
            result = Mat4([[0] * 4 for _ in range(4)])
            for i in range(4):
                for j in range(4):
                    for k in range(4):
                        result.m[i][j] += self.m[i][k] * other.m[k][j]
            return result
        elif isinstance(other, Vec4):
            x = sum(self.m[0][k] * [other.x, other.y, other.z, other.w][k] for k in range(4))
            y = sum(self.m[1][k] * [other.x, other.y, other.z, other.w][k] for k in range(4))
            z = sum(self.m[2][k] * [other.x, other.y, other.z, other.w][k] for k in range(4))
            w = sum(self.m[3][k] * [other.x, other.y, other.z, other.w][k] for k in range(4))
            return Vec4(x, y, z, w)
        elif isinstance(other, Vec3):
            v4 = other.to_vec4(1.0)
            return (self * v4).perspective_divide()
        return NotImplemented


CUBE_VERTICES = [
    Vec3(-1, -1, -1), Vec3(1, -1, -1), Vec3(1, 1, -1), Vec3(-1, 1, -1),
    Vec3(-1, -1,  1), Vec3(1, -1,  1), Vec3(1, 1,  1), Vec3(-1, 1,  1),
]

CUBE_EDGES = [
    (0, 1), (1, 2), (2, 3), (3, 0),
    (4, 5), (5, 6), (6, 7), (7, 4),
    (0, 4), (1, 5), (2, 6), (3, 7),
]


def transform_vertex(v, model, view, proj, width, height):
    clip = proj * view * model * v.to_vec4(1.0)
    if clip.w < 0.001:
        return None
    ndc = clip.perspective_divide()
    sx = (ndc.x + 1.0) * 0.5 * width
    sy = (1.0 - ndc.y) * 0.5 * height
    return (sx, sy)


def render_cube_ppm(filename, angle_deg, width=400, height=400):
    model = Mat4.rotation_y(angle_deg) * Mat4.rotation_x(15)
    eye = Vec3(0, 2, 6)
    target = Vec3(0, 0, 0)
    up = Vec3(0, 1, 0)
    view = Mat4.look_at(eye, target, up)
    proj = Mat4.perspective(60, width / height, 0.1, 100.0)
    screen_pts = []
    for v in CUBE_VERTICES:
        pt = transform_vertex(v, model, view, proj, width, height)
        screen_pts.append(pt)
    pixels = [[(15, 15, 25)] * width for _ in range(height)]
    for i, j in CUBE_EDGES:
        a, b = screen_pts[i], screen_pts[j]
        if a is None or b is None:
            continue
        x0, y0 = int(a[0]), int(a[1])
        x1, y1 = int(b[0]), int(b[1])
        dx = abs(x1 - x0)
        dy = abs(y1 - y0)
        sx = 1 if x0 < x1 else -1
        sy = 1 if y0 < y1 else -1
        err = dx - dy
        while True:
            if 0 <= x0 < width and 0 <= y0 < height:
                pixels[y0][x0] = (0, 255, 160)
            if x0 == x1 and y0 == y1:
                break
            e2 = 2 * err
            if e2 > -dy:
                err -= dy
                x0 += sx
            if e2 < dx:
                err += dx
                y0 += sy
    for idx, v in enumerate(CUBE_VERTICES):
        pt = screen_pts[idx]
        if pt is None:
            continue
        cx, cy = int(pt[0]), int(pt[1])
        for dx in range(-2, 3):
            for dy in range(-2, 3):
                nx, ny = cx + dx, cy + dy
                if 0 <= nx < width and 0 <= ny < height:
                    pixels[ny][nx] = (255, 220, 50)
    with open(filename, "w") as f:
        f.write("P3\n")
        f.write(f"{width} {height}\n")
        f.write("255\n")
        for row in pixels:
            for r, g, b in row:
                f.write(f"{r} {g} {b} ")
            f.write("\n")


def demo_tr_vs_rt():
    print("=== TR vs RT Demo ===")
    v = Vec3(1, 0, 0)
    T = Mat4.translation(5, 0, 0)
    R = Mat4.rotation_z(90)
    result_tr = T * R * v
    result_rt = R * T * v
    print(f"Point v = {v}")
    print(f"T(v) then R: T*R*v = {result_tr}")
    print(f"R(v) then T: R*T*v = {result_rt}")
    print(f"They differ! TR gives (5,1,0) vs RT gives (0,6,0)")
    print()


def demo_perspective_divide():
    print("=== Perspective Divide Demo ===")
    near = 1.0
    clip_near = Vec4(0, 0.5, -1.0, 1.0)
    clip_far = Vec4(0, 0.5, -10.0, 10.0)
    print(f"Near point (z={clip_near.z:.1f}, w={clip_near.w:.1f}): "
          f"ndc_y = {clip_near.y / clip_near.w:.4f}")
    print(f"Far point  (z={clip_far.z:.1f}, w={clip_far.w:.1f}): "
          f"ndc_y = {clip_far.y / clip_far.w:.4f}")
    print("Far point has smaller ndc_y — appears closer to center = 'smaller'")
    print()


def demo_projection_matrix():
    print("=== Perspective Projection Matrix ===")
    P = Mat4.perspective(60, 1.0, 0.1, 100.0)
    print("Perspective matrix (fov=60, aspect=1, near=0.1, far=100):")
    for row in P.m:
        print("  [" + ", ".join(f"{v:8.4f}" for v in row) + "]")
    v_eye = Vec4(0, 0, -5, 1)
    v_clip = P * v_eye
    print(f"\nEye-space point: {v_eye}")
    print(f"Clip-space: {v_clip}")
    ndc = v_clip.perspective_divide()
    print(f"After perspective divide: {ndc}")
    print()


def main():
    print("Lesson 14.03: Linear Algebra for Graphics — Transforms, Projections")
    print("=" * 65)
    print()
    demo_tr_vs_rt()
    demo_perspective_divide()
    demo_projection_matrix()
    print("=== Rendering Wireframe Cube ===")
    render_cube_ppm("cube_wireframe.ppm", angle_deg=30)
    print("Wrote cube_wireframe.ppm (rotate by changing angle_deg)")
    render_cube_ppm("cube_wireframe_0deg.ppm", angle_deg=0)
    print("Wrote cube_wireframe_0deg.ppm")
    render_cube_ppm("cube_wireframe_60deg.ppm", angle_deg=60)
    print("Wrote cube_wireframe_60deg.ppm")
    print()
    print("Key takeaway: TR and RT give different results — order matters!")
    print("The perspective divide (÷w) is what makes far things small.")


if __name__ == "__main__":
    main()