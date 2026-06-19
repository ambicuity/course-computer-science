# Animation, Skinning, IK

> How characters move: from keyframes through bone trees to inverse kinematics.

**Type:** Learn
**Languages:** Rust
**Prerequisites:** Phase 14 lessons 01–15
**Time:** ~60 minutes

## Learning Objectives

- Explain keyframe interpolation and why spline interpolation beats linear.
- Describe skeletal animation: joints as a tree, bind pose, animation pose.
- Derive the skinning matrix: `M_i = joint_pose_i × inverse_bind_i`.
- Implement Linear Blend Skinning (LBS) and understand its artifacts (volume loss, candy-wrapper).
- Implement CCD IK to solve for end-effector targets.
- Compare FABRIK vs CCD and understand animation blending with normalized time.

## The Problem

You have a mesh — a character model — and you want it to bend its arm, run, or reach for a cup on a shelf. Moving every vertex by hand is impossible for 10,000+ vertices. You need:

1. **Keyframe animation** — define poses at key moments, interpolate between them.
2. **Skeletal animation** — attach vertices to a skeleton of joints; move the joints, and vertices follow.
3. **Skinning** — the math that transforms vertices from bind pose to animated pose.
4. **Inverse Kinematics** — given a desired end-effector position, automatically compute the joint angles.

Without these, you cannot animate characters, creatures, or any articulated figure. This lesson builds all four from scratch.

## The Concept

### Keyframe Animation

A **keyframe** stores a pose (position + rotation) at a specific time. Between keyframes, we interpolate:

```
Linear:    v(t) = (1-t)·v₀ + t·v₁         — angular, causes slowdown/speedup
Spline:    v(t) = Hermite/Bezier/...         — smooth, continuous tangent
```

For rotation, **never linearly interpolate matrices** — use quaternions and slerp (spherical linear interpolation):

```
slerp(q₀, q₁, t) = (sin((1-t)θ) / sinθ)·q₀ + (sin(tθ) / sinθ)·q₁
```

**Normalized time** `u = (t - t₀) / (t₁ - t₀)` maps any keyframe interval to [0, 1].

```
  Keyframe 0       Keyframe 1       Keyframe 2
     ●─────────────────●─────────────────●
     t₀               t₁               t₂
         |←─ u ─→|
         u = (t-t₀)/(t₁-t₀)  ∈ [0,1]
```

### Skeletal Animation: The Joint Tree

A skeleton is a tree of joints. Each joint stores a local transform relative to its parent:

```
         root (hip)
        /          \
     left_hip    right_hip
       |            |
     left_knee   right_knee
       |            |
     left_ankle  right_ankle
```

**Bind pose** = the pose the mesh was rigged in (T-pose or A-pose).
**Animation pose** = the pose at some frame of animation.

**Forward kinematics** computes world transforms by walking the tree:

```
world(joint_i) = local(joint_i) × world(parent(i))
```

### Joint Transforms: The Skinning Formula

For each joint `i`, we precompute the **inverse bind pose** matrix: the matrix that takes a vertex from model space to joint-local space in the bind pose.

The **skinning matrix** for joint `i` at animation time is:

```
M_i = world_pose_i × inverse_bind_i
```

This matrix takes a vertex from bind-pose model space → animated model space in one step.

Why `inverse_bind`? Because we need to **undo** the bind pose before applying the new pose:

```
M_i × v = (new_world_i) × (inverse_bind_i × v)
         = (new_world_i) × (v expressed in joint_i local space)
```

Without the inverse bind, you'd be transforming vertices that are already in model space with a matrix that expects local-space input — the result would be garbage.

### Linear Blend Skinning (LBS)

Each vertex can be influenced by multiple joints, with weights `w_i` that sum to 1:

```
v' = Σ(w_i × M_i × v)
      i
```

Example: an elbow vertex might be 70% upper arm, 30% forearm:

```
v' = 0.7 × M_upper_arm × v + 0.3 × M_forearm × v
```

### LBS Problems

LBS has a well-known artifact: **volume loss**. When two joint transforms diverge significantly (e.g., twisting 90°), the blended result collapses:

```
   Bind pose:          LBS at 90° twist:
    ┌───┐                 ╎
    │   │    ──→          ╎  (volume collapsed
    └───┘                 ╎   to a thin line)
```

This is the **candy-wrapper effect** — the mesh pinches to zero volume at extreme twists. It happens because blending rotation matrices linearly does not preserve the rigid-body constraint.

**Dual quaternion skinning** fixes this by blending in the dual quaternion space, preserving rigid transforms. It's the industry standard improvement, but LBS remains common because of its simplicity and GPU support.

### Inverse Kinematics (IK)

IK is the inverse problem: given a target position for the end effector, find the joint angles that reach it.

```
  Target ●
         ╲
          ╲  ← find these angles
     ●────●
   joint1  joint2 (end effector should reach target)
```

### CCD IK (Cyclic Coordinate Descent)

CCD iterates from the end effector back toward the root. For each joint:

1. Compute the vector from the joint to the end effector.
2. Compute the vector from the joint to the target.
3. Rotate the joint to align these vectors.
4. Repeat until convergence or max iterations.

```
Iteration 1: Rotate joint2 toward target
  ●────● ──→ ●────●  (end closer to target)
              ╲
               ● target

Iteration 2: Rotate joint1 toward target
  ●────●  ──→  ●
                 ╲
                  ● (end reaches target!)
```

CCD is simple, efficient, and widely used in games. It can get stuck in local minima but usually converges quickly for reasonable targets.

### FABRIK (Forward And Backward Reaching IK)

FABRIK works differently — it moves joints directly rather than computing rotations:

**Forward pass:** Start from the end effector, move it to the target, then adjust each parent joint to maintain bone lengths.

**Backward pass:** Start from the root, move it back to its fixed position, then adjust each child joint to maintain bone lengths.

```
Forward:  root→ ··· →end  ──→  root→ ··· →target position
Backward: root← ··· ←end  ──→  anchor→ ··· →corrected positions
```

FABRIK converges faster than CCD and produces more natural poses, but is harder to implement with rotational constraints.

### Animation Blending (Crossfading)

To transition between two animations (e.g., idle → walk), we **blend** (crossfade):

```
v'(t) = (1 - α) × v_anim_A(t) + α × v_anim_B(t)
```

where `α` ramps from 0 to 1 over the blend duration. Both animations must be evaluated at their own **normalized time** to stay in sync.

### Animation Compression

Keyframes are expensive to store. Compression strategies:
- **Key reduction**: Remove keyframes where interpolation error stays below a threshold.
- **Quaternion quantization**: Store quaternion components as 16-bit integers (or smaller) instead of 32-bit floats. Since `q` and `-q` represent the same rotation, ensure consistent sign.

## Build It

### Step 1: Minimal Version — Skeleton and Forward Kinematics

We represent a skeleton as a flat array of joints, each with a parent index and a local transform. Forward kinematics walks the tree to compute world transforms.

```rust
struct Joint {
    parent: Option<usize>,
    local_bind: Mat4,  // local transform in bind pose
}

fn forward_kinematics(joints: &[Joint]) -> Vec<Mat4> {
    let mut world = vec![Mat4::identity(); joints.len()];
    for (i, j) in joints.iter().enumerate() {
        world[i] = match j.parent {
            Some(p) => world[p] * j.local_bind,
            None    => j.local_bind,
        };
    }
    world
}
```

### Step 2: Full Version — Skinning, LBS, CCD IK, and PPM Rendering

The full implementation includes:

- **Skeleton** struct with joints, parent indices, and bind pose transforms.
- **Inverse bind pose** computation (inverse of world bind pose).
- **Linear Blend Skinning**: `v' = Σ w_i × (pose_i × inv_bind_i) × v`.
- **CCD IK solver**: iteratively rotate joints to reach a target.
- **PPM rendering**: draw stick figures showing bind pose, animated pose, and IK-solved pose.

See `code/main.rs` for the complete implementation.

## Use It

### Production Skinning

In production engines (Unity, Unreal, Godot):
- Skinning runs on the GPU via vertex shaders.
- Uniform buffer passes `M_i` (skinning matrices) for all joints.
- Weights are stored per-vertex (typically up to 4 weights per vertex — the "8 weights" limit is legacy).
- Dual quaternion skinning is available as an option but LBS remains the default.

### Production IK

- **Unity**: `IK` solvers in Animator, `Cinemachine` for procedural aiming.
- **Unreal**: `FABRIK` node in Animation Blueprint, CCD node available.
- **Godot**: `Skeleton3D` with `PhysicalBone3D` for physics-driven IK.

The key difference: production IK solvers add joint limits, iterative refinement, and multi-end-effector support. Our CCD solver is the simplest version; production adds constraints.

## Read the Source

- **glTF SDK** (`Joint.rs`): The glTF SDK's joint/skin loading code shows how skinning data flows from file to GPU.
- **Ozz-animation** (`ozz-animation/animation/runtime`): An open-source C++ animation library with production-quality skeleton sampling, blending, and IK. Look at `local_to_model_job.cc` for forward kinematics.

## Ship It

The reusable artifact for this lesson is `outputs/animation_reference.md` — a quick-reference card covering the skinning matrix formula, LBS formula, CCD IK pseudocode, FABRIK steps, and animation blending equation.

## Exercises

1. **Easy** — Modify the skeleton to use a different arm configuration (e.g., add a finger chain) and verify forward kinematics still produces correct world transforms.
2. **Medium** — Implement FABRIK alongside CCD and compare their convergence on the same target. Which reaches faster? Which handles unreachable targets better?
3. **Hard** — Implement dual quaternion skinning and compare visual output against LBS. Show the candy-wrapper artifact disappears with DQS at extreme twist angles.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Bind pose | "rest pose" | The default T/A-pose the mesh was rigged in; used to compute inverse bind matrices |
| Inverse bind | "offset matrix" | The matrix that un-transforms a vertex from model space back into a joint's local space |
| Skinning matrix | "bone matrix" | `M_i = world_pose_i × inverse_bind_i`; transforms a vertex from bind pose to animated pose |
| LBS | "blend skinning" | Linear Blend Skinning: `v' = Σ w_i M_i v`; simple but causes volume loss |
| Candy-wrapper | "collapse artifact" | LBS artifact where mesh collapses to zero volume under extreme twist |
| Dual quaternion skinning | "DQS" | Blends rotations in dual quaternion space; preserves volume but can cause bulging |
| Forward kinematics | "FK" | Computing world positions from joint angles (tree traversal) |
| Inverse kinematics | "IK" | Computing joint angles from desired end-effector position |
| CCD IK | "cyclic coordinate descent" | Iterative IK: rotate each joint to point end-effector toward target |
| FABRIK | "forward-backward IK" | IK by directly moving joint positions, then enforcing bone-length constraints |
| Crossfade | "animation blend" | Smoothly transitioning between two animations by interpolating their poses |

## Further Reading

- **"Real-Time Rendering"** (Akenine-Möller et al.), Chapter 4 — Skinning and morphing in detail.
- **"Foundations of Game Engine Development, Volume 2"** (Gregory), Chapter 11 — Animation blending, compression, and IK.
- **Ozz-animation**: Open-source C++ animation runtime: https://github.com/guillaumeblanc/ozz-animation
- **"Animating Rotation with Quaternion Curves"** (Shoemake, SIGGRAPH 1985) — The original paper on quaternion interpolation for animation.