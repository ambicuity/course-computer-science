//! Animation, Skinning, IK
//! Phase 14 — Computer Graphics & Visualization
//!
//! Implements:
//!   - Skeleton struct (tree of joints with parent indices and bind pose transforms)
//!   - Forward kinematics (compute world transforms from local joint transforms)
//!   - Inverse bind pose computation
//!   - Linear Blend Skinning (LBS): v' = sum(w_i * M_i * v)
//!   - CCD IK solver
//!   - PPM rendering of stick figures in bind pose, animated pose, and IK-solved pose

use std::fmt;
use std::fs::File;
use std::io::Write;

#[derive(Clone, Copy)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vec3 {
    fn new(x: f64, y: f64, z: f64) -> Self {
        Vec3 { x, y, z }
    }

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn cross(self, other: Self) -> Self {
        Vec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    fn normalized(self) -> Self {
        let l = self.length();
        if l < 1e-12 {
            Vec3::new(0.0, 0.0, 0.0)
        } else {
            Vec3::new(self.x / l, self.y / l, self.z / l)
        }
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, s: f64) -> Self {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

#[derive(Clone, Copy)]
struct Mat4 {
    m: [[f64; 4]; 4],
}

impl Mat4 {
    fn identity() -> Self {
        let mut m = [[0.0; 4]; 4];
        for i in 0..4 {
            m[i][i] = 1.0;
        }
        Mat4 { m }
    }

    fn translation(tx: f64, ty: f64, tz: f64) -> Self {
        let mut mat = Mat4::identity();
        mat.m[0][3] = tx;
        mat.m[1][3] = ty;
        mat.m[2][3] = tz;
        mat
    }

    fn rotation_z(deg: f64) -> Self {
        let r = deg.to_radians();
        let (c, s) = (r.cos(), r.sin());
        let mut mat = Mat4::identity();
        mat.m[0][0] = c;
        mat.m[0][1] = -s;
        mat.m[1][0] = s;
        mat.m[1][1] = c;
        mat
    }

    fn rotation_y(deg: f64) -> Self {
        let r = deg.to_radians();
        let (c, s) = (r.cos(), r.sin());
        let mut mat = Mat4::identity();
        mat.m[0][0] = c;
        mat.m[0][2] = s;
        mat.m[2][0] = -s;
        mat.m[2][2] = c;
        mat
    }

    fn rotation_x(deg: f64) -> Self {
        let r = deg.to_radians();
        let (c, s) = (r.cos(), r.sin());
        let mut mat = Mat4::identity();
        mat.m[1][1] = c;
        mat.m[1][2] = -s;
        mat.m[2][1] = s;
        mat.m[2][2] = c;
        mat
    }

    fn inverse(&self) -> Self {
        let m = &self.m;
        let mut inv = [[0.0f64; 4]; 4];
        inv[0][0] = m[1][1]*m[2][2]*m[3][3] - m[1][1]*m[2][3]*m[3][2]
            - m[1][2]*m[2][1]*m[3][3] + m[1][2]*m[2][3]*m[3][1]
            + m[1][3]*m[2][1]*m[3][2] - m[1][3]*m[2][2]*m[3][1];
        inv[0][1] = -m[0][1]*m[2][2]*m[3][3] + m[0][1]*m[2][3]*m[3][2]
            + m[0][2]*m[2][1]*m[3][3] - m[0][2]*m[2][3]*m[3][1]
            - m[0][3]*m[2][1]*m[3][2] + m[0][3]*m[2][2]*m[3][1];
        inv[0][2] = m[0][1]*m[1][2]*m[3][3] - m[0][1]*m[1][3]*m[3][2]
            - m[0][2]*m[1][1]*m[3][3] + m[0][2]*m[1][3]*m[3][1]
            + m[0][3]*m[1][1]*m[3][2] - m[0][3]*m[1][2]*m[3][1];
        inv[0][3] = -m[0][1]*m[1][2]*m[2][3] + m[0][1]*m[1][3]*m[2][2]
            + m[0][2]*m[1][1]*m[2][3] - m[0][2]*m[1][3]*m[2][1]
            - m[0][3]*m[1][1]*m[2][2] + m[0][3]*m[1][2]*m[2][1];

        inv[1][0] = -m[1][0]*m[2][2]*m[3][3] + m[1][0]*m[2][3]*m[3][2]
            + m[1][2]*m[2][0]*m[3][3] - m[1][2]*m[2][3]*m[3][0]
            - m[1][3]*m[2][0]*m[3][2] + m[1][3]*m[2][2]*m[3][0];
        inv[1][1] = m[0][0]*m[2][2]*m[3][3] - m[0][0]*m[2][3]*m[3][2]
            - m[0][2]*m[2][0]*m[3][3] + m[0][2]*m[2][3]*m[3][0]
            + m[0][3]*m[2][0]*m[3][2] - m[0][3]*m[2][2]*m[3][0];
        inv[1][2] = -m[0][0]*m[1][2]*m[3][3] + m[0][0]*m[1][3]*m[3][2]
            + m[0][2]*m[1][0]*m[3][3] - m[0][2]*m[1][3]*m[3][0]
            - m[0][3]*m[1][0]*m[3][2] + m[0][3]*m[1][2]*m[3][0];
        inv[1][3] = m[0][0]*m[1][2]*m[2][3] - m[0][0]*m[1][3]*m[2][2]
            - m[0][2]*m[1][0]*m[2][3] + m[0][2]*m[1][3]*m[2][0]
            + m[0][3]*m[1][0]*m[2][2] - m[0][3]*m[1][2]*m[2][0];

        inv[2][0] = m[1][0]*m[2][1]*m[3][3] - m[1][0]*m[2][3]*m[3][1]
            - m[1][1]*m[2][0]*m[3][3] + m[1][1]*m[2][3]*m[3][0]
            + m[1][3]*m[2][0]*m[3][1] - m[1][3]*m[2][1]*m[3][0];
        inv[2][1] = -m[0][0]*m[2][1]*m[3][3] + m[0][0]*m[2][3]*m[3][1]
            + m[0][1]*m[2][0]*m[3][3] - m[0][1]*m[2][3]*m[3][0]
            - m[0][3]*m[2][0]*m[3][1] + m[0][3]*m[2][1]*m[3][0];
        inv[2][2] = m[0][0]*m[1][1]*m[3][3] - m[0][0]*m[1][3]*m[3][1]
            - m[0][1]*m[1][0]*m[3][3] + m[0][1]*m[1][3]*m[3][0]
            + m[0][3]*m[1][0]*m[3][1] - m[0][3]*m[1][1]*m[3][0];
        inv[2][3] = -m[0][0]*m[1][1]*m[2][3] + m[0][0]*m[1][3]*m[2][1]
            + m[0][1]*m[1][0]*m[2][3] - m[0][1]*m[1][3]*m[2][0]
            - m[0][3]*m[1][0]*m[2][1] + m[0][3]*m[1][1]*m[2][0];

        inv[3][0] = -m[1][0]*m[2][1]*m[3][2] + m[1][0]*m[2][2]*m[3][1]
            + m[1][1]*m[2][0]*m[3][2] - m[1][1]*m[2][2]*m[3][0]
            - m[1][2]*m[2][0]*m[3][1] + m[1][2]*m[2][1]*m[3][0];
        inv[3][1] = m[0][0]*m[2][1]*m[3][2] - m[0][0]*m[2][2]*m[3][1]
            - m[0][1]*m[2][0]*m[3][2] + m[0][1]*m[2][2]*m[3][0]
            + m[0][2]*m[2][0]*m[3][1] - m[0][2]*m[2][1]*m[3][0];
        inv[3][2] = -m[0][0]*m[1][1]*m[3][2] + m[0][0]*m[1][2]*m[3][1]
            + m[0][1]*m[1][0]*m[3][2] - m[0][1]*m[1][2]*m[3][0]
            - m[0][2]*m[1][0]*m[3][1] + m[0][2]*m[1][1]*m[3][0];
        inv[3][3] = m[0][0]*m[1][1]*m[2][2] - m[0][0]*m[1][2]*m[2][1]
            - m[0][1]*m[1][0]*m[2][2] + m[0][1]*m[1][2]*m[2][0]
            + m[0][2]*m[1][0]*m[2][1] - m[0][2]*m[1][1]*m[2][0];

        let det = m[0][0]*inv[0][0] + m[0][1]*inv[1][0]
            + m[0][2]*inv[2][0] + m[0][3]*inv[3][0];

        if det.abs() < 1e-10 {
            return Mat4::identity();
        }

        let inv_det = 1.0 / det;
        let mut result = [[0.0f64; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                result[i][j] = inv[i][j] * inv_det;
            }
        }
        Mat4 { m: result }
    }

    fn transform_point(&self, v: Vec3) -> Vec3 {
        let x = self.m[0][0]*v.x + self.m[0][1]*v.y + self.m[0][2]*v.z + self.m[0][3];
        let y = self.m[1][0]*v.x + self.m[1][1]*v.y + self.m[1][2]*v.z + self.m[1][3];
        let z = self.m[2][0]*v.x + self.m[2][1]*v.y + self.m[2][2]*v.z + self.m[2][3];
        Vec3::new(x, y, z)
    }
}

impl std::ops::Mul for Mat4 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        let mut result = [[0.0f64; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    result[i][j] += self.m[i][k] * rhs.m[k][j];
                }
            }
        }
        Mat4 { m: result }
    }
}

impl fmt::Display for Mat4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for row in &self.m {
            writeln!(f, "  [{:8.4}, {:8.4}, {:8.4}, {:8.4}]", row[0], row[1], row[2], row[3])?;
        }
        Ok(())
    }
}

struct Joint {
    name: &'static str,
    parent: Option<usize>,
    local_bind: Mat4,
    bone_length: f64,
}

struct Skeleton {
    joints: Vec<Joint>,
}

impl Skeleton {
    fn forward_kinematics(&self, local_poses: &[Mat4]) -> Vec<Mat4> {
        let mut world = vec![Mat4::identity(); self.joints.len()];
        for (i, joint) in self.joints.iter().enumerate() {
            world[i] = match joint.parent {
                Some(p) => world[p] * local_poses[i],
                None => local_poses[i],
            };
        }
        world
    }

    fn compute_inverse_bind(&self) -> Vec<Mat4> {
        let bind_local: Vec<Mat4> = self.joints.iter().map(|j| j.local_bind).collect();
        let world_bind = self.forward_kinematics(&bind_local);
        world_bind.iter().map(|m| m.inverse()).collect()
    }
}

fn build_humanoid_arm() -> Skeleton {
    let hip      = Joint { name: "hip",      parent: None,      local_bind: Mat4::translation(0.0, 0.0, 0.0),  bone_length: 0.0 };
    let shoulder  = Joint { name: "shoulder",  parent: Some(0),  local_bind: Mat4::translation(0.0, 2.0, 0.0),  bone_length: 2.0 };
    let elbow     = Joint { name: "elbow",     parent: Some(1),  local_bind: Mat4::translation(1.5, 0.0, 0.0),  bone_length: 1.5 };
    let wrist     = Joint { name: "wrist",     parent: Some(2),  local_bind: Mat4::translation(1.2, 0.0, 0.0),  bone_length: 1.2 };
    let fingertip = Joint { name: "fingertip", parent: Some(3),  local_bind: Mat4::translation(0.5, 0.0, 0.0),  bone_length: 0.5 };

    Skeleton {
        joints: vec![hip, shoulder, elbow, wrist, fingertip],
    }
}

fn linear_blend_skin(
    vertex: Vec3,
    weights: &[(usize, f64)],
    skinning_matrices: &[Mat4],
) -> Vec3 {
    let mut result = Vec3::new(0.0, 0.0, 0.0);
    for &(joint_idx, w) in weights {
        if w.abs() < 1e-10 {
            continue;
        }
        let transformed = skinning_matrices[joint_idx].transform_point(vertex);
        result = result + transformed * w;
    }
    result
}

fn ccd_ik(
    skeleton: &Skeleton,
    end_effector_idx: usize,
    target: Vec3,
    local_poses: &mut Vec<Mat4>,
    max_iterations: usize,
) -> bool {
    let chain_start = 1usize;
    let n = skeleton.joints.len();

    for _ in 0..max_iterations {
        let world = skeleton.forward_kinematics(local_poses);
        let end_pos = world[end_effector_idx].transform_point(Vec3::new(0.0, 0.0, 0.0));

        if (end_pos - target).length() < 0.01 {
            return true;
        }

        for joint_visiting in (chain_start..end_effector_idx).rev() {
            let world = skeleton.forward_kinematics(local_poses);
            let end_pos = world[end_effector_idx].transform_point(Vec3::new(0.0, 0.0, 0.0));
            let joint_world = world[joint_visiting].transform_point(Vec3::new(0.0, 0.0, 0.0));

            let to_end = (end_pos - joint_world).normalized();
            let to_target = (target - joint_world).normalized();

            let dot = to_end.dot(to_target).clamp(-1.0, 1.0);
            let cross = to_end.cross(to_target);
            let angle = dot.acos();

            if angle.abs() < 1e-8 {
                continue;
            }

            let axis = if cross.length() > 1e-10 {
                cross.normalized()
            } else {
                Vec3::new(0.0, 0.0, 1.0)
            };

            let angle_deg = angle.to_degrees();
            let rotation = if axis.z.abs() > 0.5 {
                Mat4::rotation_z(if axis.z > 0.0 { angle_deg } else { -angle_deg })
            } else if axis.y.abs() > 0.5 {
                Mat4::rotation_y(if axis.y > 0.0 { angle_deg } else { -angle_deg })
            } else {
                Mat4::rotation_x(if axis.x > 0.0 { angle_deg } else { -angle_deg })
            };

            let parent_world_inv = match skeleton.joints[joint_visiting].parent {
                Some(p) => world[p].inverse(),
                None => Mat4::identity(),
            };

            local_poses[joint_visiting] = parent_world_inv * rotation * world[joint_visiting];
        }
    }

    let world = skeleton.forward_kinematics(local_poses);
    let end_pos = world[end_effector_idx].transform_point(Vec3::new(0.0, 0.0, 0.0));
    (end_pos - target).length() < 0.1
}

fn extract_joint_positions(world_transforms: &[Mat4]) -> Vec<Vec3> {
    world_transforms
        .iter()
        .map(|m| m.transform_point(Vec3::new(0.0, 0.0, 0.0)))
        .collect()
}

fn lerp_color(c1: (u8, u8, u8), c2: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let r = (c1.0 as f64 * (1.0 - t) + c2.0 as f64 * t) as u8;
    let g = (c1.1 as f64 * (1.0 - t) + c2.1 as f64 * t) as u8;
    let b = (c1.2 as f64 * (1.0 - t) + c2.2 as f64 * t) as u8;
    (r, g, b)
}

fn draw_line(pixels: &mut [(u8, u8, u8)], w: usize, h: usize, x0: i32, y0: i32, x1: i32, y1: i32, color: (u8, u8, u8)) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    let mut cx = x0;
    let mut cy = y0;
    loop {
        if cx >= 0 && cx < w as i32 && cy >= 0 && cy < h as i32 {
            pixels[(cy as usize) * w + (cx as usize)] = color;
        }
        if cx == x1 && cy == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            cx += sx;
        }
        if e2 < dx {
            err += dx;
            cy += sy;
        }
    }
}

fn draw_dot(pixels: &mut [(u8, u8, u8)], w: usize, h: usize, x: i32, y: i32, radius: i32, color: (u8, u8, u8)) {
    for ddx in -radius..=radius {
        for ddy in -radius..=radius {
            if ddx*ddx + ddy*ddy <= radius*radius {
                let nx = x + ddx;
                let ny = y + ddy;
                if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                    pixels[(ny as usize) * w + (nx as usize)] = color;
                }
            }
        }
    }
}

fn project(positions: &[Vec3], w: usize, h: usize, scale: f64, offset_x: f64, offset_y: f64) -> Vec<(i32, i32)> {
    positions
        .iter()
        .map(|p| {
            let sx = (p.x * scale + offset_x) as i32;
            let sy = (h as f64 - (p.y * scale + offset_y)) as i32;
            (sx, sy)
        })
        .collect()
}

fn render_ppm(filename: &str, title: &str, scenes: &[(&[Vec3], (u8, u8, u8))], width: usize, height: usize) {
    let mut pixels = vec![(15u8, 15u8, 25u8); width * height];

    for (positions, color) in scenes {
        let proj = project(positions, width, height, 80.0, width as f64 / 2.0, 0.0);
        for i in 1..positions.len() {
            let parent_idx = i;
            draw_line(&mut pixels, width, height, proj[parent_idx - 1].0, proj[parent_idx - 1].1, proj[parent_idx].0, proj[parent_idx].1, *color);
        }
        for &p in &proj {
            draw_dot(&mut pixels, width, height, p.0, p.1, 4, *color);
        }
    }

    let mut file = File::create(filename).expect("Failed to create PPM");
    write!(file, "P3\n{} {}\n255\n", width, height).unwrap();
    for row in 0..height {
        for col in 0..width {
            let (r, g, b) = pixels[row * width + col];
            write!(file, "{} {} {} ", r, g, b).unwrap();
        }
        writeln!(file).unwrap();
    }
    println!("  Wrote {} ({})", filename, title);
}

fn demo_skeleton_and_fk() {
    println!("=== Forward Kinematics Demo ===");
    let skeleton = build_humanoid_arm();
    let bind_local: Vec<Mat4> = skeleton.joints.iter().map(|j| j.local_bind).collect();
    let world_bind = skeleton.forward_kinematics(&bind_local);
    let bind_positions = extract_joint_positions(&world_bind);

    println!("Bind pose joint positions (T-pose):");
    for (i, pos) in bind_positions.iter().enumerate() {
        println!("  {}: ({:.3}, {:.3}, {:.3})", skeleton.joints[i].name, pos.x, pos.y, pos.z);
    }
    println!();

    let mut animated_poses = bind_local.clone();
    animated_poses[1] = Mat4::translation(0.0, 2.0, 0.0);
    animated_poses[2] = Mat4::translation(1.5, 0.0, 0.0) * Mat4::rotation_z(-45.0);
    animated_poses[3] = Mat4::translation(0.0, 0.0, 0.0) * Mat4::rotation_z(-30.0);

    let world_animated = skeleton.forward_kinematics(&animated_poses);
    let anim_positions = extract_joint_positions(&world_animated);
    println!("Animated pose (elbow bent -45°, wrist bent -30°):");
    for (i, pos) in anim_positions.iter().enumerate() {
        println!("  {}: ({:.3}, {:.3}, {:.3})", skeleton.joints[i].name, pos.x, pos.y, pos.z);
    }
    println!();
}

fn demo_skinning() {
    println!("=== Linear Blend Skinning Demo ===");
    let skeleton = build_humanoid_arm();
    let inv_bind = skeleton.compute_inverse_bind();

    let mut local_poses: Vec<Mat4> = skeleton.joints.iter().map(|j| j.local_bind).collect();
    local_poses[2] = Mat4::translation(1.5, 0.0, 0.0) * Mat4::rotation_z(-45.0);

    let world_poses = skeleton.forward_kinematics(&local_poses);

    let skinning_matrices: Vec<Mat4> = world_poses
        .iter()
        .zip(inv_bind.iter())
        .map(|(wp, ib)| wp * *ib)
        .collect();

    println!("Skinning matrices computed (pose × inverse_bind):");
    for (i, m) in skinning_matrices.iter().enumerate() {
        println!("  Joint {} skinning matrix:", skeleton.joints[i].name);
        print!("{}", m);
    }
    println!();

    let vertex = Vec3::new(2.8, 2.0, 0.0);
    let weights: [(usize, f64); 2] = [(2, 0.7), (1, 0.3)];
    let deformed = linear_blend_skin(vertex, &weights, &skinning_matrices);
    println!("LBS vertex deformation:");
    println!("  Original vertex: ({:.3}, {:.3}, {:.3})", vertex.x, vertex.y, vertex.z);
    println!("  Weights: joint(elbow)=0.7, joint(shoulder)=0.3");
    println!("  Deformed vertex: ({:.3}, {:.3}, {:.3})", deformed.x, deformed.y, deformed.z);
    println!();

    let vertex2 = Vec3::new(2.8, 2.0, 0.0);
    let weights_bind: [(usize, f64); 1] = [(2, 1.0)];
    let deformed_bind = linear_blend_skin(vertex2, &weights_bind, &{
        let bind_skin: Vec<Mat4> = skeleton.forward_kinematics(&skeleton.joints.iter().map(|j| j.local_bind).collect())
            .iter().zip(inv_bind.iter()).map(|(wp, ib)| wp * *ib).collect();
        bind_skin
    });
    println!("  Verification: 100% bind pose skinning preserves vertex:");
    println!("  v=({:.3}, {:.3}, {:.3}) -> v'=({:.3}, {:.3}, {:.3})",
        vertex2.x, vertex2.y, vertex2.z,
        deformed_bind.x, deformed_bind.y, deformed_bind.z);
    println!("  (Should be identical — identity skinning matrices in bind pose)");
    println!();
}

fn demo_ccd_ik() {
    println!("=== CCD IK Demo ===");
    let skeleton = build_humanoid_arm();
    let end_effector_idx = 4;
    let target = Vec3::new(1.5, 3.0, 0.0);
    println!("Target position: ({:.3}, {:.3}, {:.3})", target.x, target.y, target.z);

    let mut local_poses: Vec<Mat4> = skeleton.joints.iter().map(|j| j.local_bind).collect();
    let converged = ccd_ik(&skeleton, end_effector_idx, target, &mut local_poses, 50);

    let world = skeleton.forward_kinematics(&local_poses);
    let positions = extract_joint_positions(&world);
    println!("IK result (converged={}):", converged);
    for (i, pos) in positions.iter().enumerate() {
        println!("  {}: ({:.3}, {:.3}, {:.3})", skeleton.joints[i].name, pos.x, pos.y, pos.z);
    }
    let end_pos = positions[end_effector_idx];
    let dist = (end_pos - target).length();
    println!("  End effector distance to target: {:.4}", dist);
    println!();
}

fn render_all_poses() {
    let width = 600usize;
    let height = 400usize;

    let skeleton = build_humanoid_arm();

    let bind_local: Vec<Mat4> = skeleton.joints.iter().map(|j| j.local_bind).collect();
    let world_bind = skeleton.forward_kinematics(&bind_local);
    let bind_positions = extract_joint_positions(&world_bind);

    let mut anim_local: Vec<Mat4> = skeleton.joints.iter().map(|j| j.local_bind).collect();
    anim_local[2] = Mat4::translation(1.5, 0.0, 0.0) * Mat4::rotation_z(-60.0);
    anim_local[3] = Mat4::translation(0.0, 0.0, 0.0) * Mat4::rotation_z(-40.0);
    let world_anim = skeleton.forward_kinematics(&anim_local);
    let anim_positions = extract_joint_positions(&world_anim);

    let target = Vec3::new(1.5, 3.0, 0.0);
    let mut ik_local: Vec<Mat4> = skeleton.joints.iter().map(|j| j.local_bind).collect();
    ccd_ik(&skeleton, 4, target, &mut ik_local, 50);
    let world_ik = skeleton.forward_kinematics(&ik_local);
    let ik_positions = extract_joint_positions(&world_ik);

    let target_pos = vec![
        Vec3::new(0.0, 0.0, 0.0), target,
    ];

    let mut pixels = vec![(15u8, 15u8, 25u8); width * height];

    for (col_start, pos_ref, color, label) in [
        (50usize,    &bind_positions, (100, 200, 255), "Bind Pose"),
        (230usize, &anim_positions, (255, 200, 50),  "Animated Pose"),
        (410usize, &ik_positions,    (50, 255, 150),  "IK Solved Pose"),
    ] {
        let proj: Vec<(i32, i32)> = pos_ref.iter().map(|p| {
            let sx = (p.x * 60.0 + col_start as f64) as i32;
            let sy = (height as f64 - p.y * 60.0 - 40.0) as i32;
            (sx, sy)
        }).collect();

        for i in 1..proj.len() {
            draw_line(&mut pixels, width, height, proj[i - 1].0, proj[i - 1].1, proj[i].0, proj[i].1, color);
        }
        for &p in &proj {
            draw_dot(&mut pixels, width, height, p.0, p.1, 4, color);
        }
    }

    let tx = (target.x * 60.0 + 410.0) as i32;
    let ty = (height as f64 - target.y * 60.0 - 40.0) as i32;
    for ddx in -6..=6 {
        for ddy in -6..=6 {
            if (ddx*ddx + ddy*ddy) <= 36 {
                let nx = tx + ddx;
                let ny = ty + ddy;
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    pixels[(ny as usize) * width + (nx as usize)] = (255, 80, 80);
                }
            }
        }
    }
    for ddx in -10..=10 {
        let nx = tx + ddx;
        let ny = ty;
        if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
            pixels[(ny as usize) * width + (nx as usize)] = (255, 80, 80);
        }
        let ny2 = ty + ddx;
        if tx >= 0 && tx < width as i32 && ny2 >= 0 && ny2 < height as i32 {
            pixels[(ny2 as usize) * width + (tx as usize)] = (255, 80, 80);
        }
    }

    let mut file = File::create("animation_skinning_ik.ppm").expect("Failed to create PPM");
    write!(file, "P3\n{} {}\n255\n", width, height).unwrap();
    for row in 0..height {
        for col in 0..width {
            let (r, g, b) = pixels[row * width + col];
            write!(file, "{} {} {} ", r, g, b).unwrap();
        }
        writeln!(file).unwrap();
    }

    println!("=== PPM Output ===");
    println!("  Wrote animation_skinning_ik.ppm (3 skeleton poses + IK target)");
    println!("  Blue = Bind pose, Yellow = Animated pose, Green = IK solved, Red = IK target");
    println!();
}

fn demo_blend() {
    println!("=== Animation Blending Demo ===");
    let skeleton = build_humanoid_arm();

    let mut pose_a: Vec<Mat4> = skeleton.joints.iter().map(|j| j.local_bind).collect();
    pose_a[2] = Mat4::translation(1.5, 0.0, 0.0) * Mat4::rotation_z(-20.0);
    pose_a[3] = Mat4::rotation_z(-10.0);

    let mut pose_b: Vec<Mat4> = skeleton.joints.iter().map(|j| j.local_bind).collect();
    pose_b[2] = Mat4::translation(1.5, 0.0, 0.0) * Mat4::rotation_z(-80.0);
    pose_b[3] = Mat4::rotation_z(-60.0);

    println!("Blending pose A (wave -20°) and pose B (reach -80°):");
    for &alpha in &[0.0, 0.25, 0.5, 0.75, 1.0] {
        println!("  alpha = {:.2}:", alpha);
        let blend_positions = extract_joint_positions(&skeleton.forward_kinematics(&pose_a));
        let pos_b = extract_joint_positions(&skeleton.forward_kinematics(&pose_b));
        let blended: Vec<Vec3> = blend_positions.iter().zip(pos_b.iter())
            .map(|(a, b)| Vec3::new(
                a.x * (1.0 - alpha) + b.x * alpha,
                a.y * (1.0 - alpha) + b.y * alpha,
                a.z * (1.0 - alpha) + b.z * alpha,
            ))
            .collect();
        for (i, pos) in blended.iter().enumerate() {
            println!("    {}: ({:.3}, {:.3}, {:.3})", skeleton.joints[i].name, pos.x, pos.y, pos.z);
        }
    }
    println!();
}

fn main() {
    println!("Lesson 14.16: Animation, Skinning, IK");
    println!("========================================\n");
    demo_skeleton_and_fk();
    demo_skinning();
    demo_ccd_ik();
    demo_blend();
    render_all_poses();
    println!("Key takeaway:");
    println!("  - Skinning matrix = joint_world_pose × inverse_bind_pose");
    println!("  - LBS: v' = sum(w_i × M_i × v) — simple but causes volume loss");
    println!("  - CCD IK: iteratively rotate joints to point end-effector at target");
    println!("  - Animation blending: interpolate poses with normalized time alpha");
}