# Animation, Skinning, IK — Reference Card

## Skinning Matrix Formula

For joint `i` with world animation pose `W_i` and inverse bind pose `B_i^{-1}`:

```
M_i = W_i × B_i^{-1}
```

- `B_i^{-1}` transforms a vertex from model space → joint-local space (bind pose)
- `W_i` transforms from joint-local space → model space (animated pose)
- Together, `M_i` takes a bind-pose vertex directly to its animated position

## Linear Blend Skinning (LBS)

```
v' = Σ(w_i × M_i × v)
     i

where:  Σ w_i = 1  (weights sum to 1)
        M_i = W_i × B_i^{-1}  (skinning matrix for joint i)
```

**Artifacts:** Volume loss under large rotations (candy-wrapper effect). Dual quaternion skinning (DQS) fixes this.

## Dual Quaternion Skinning (Brief)

```
v' = normalize(Σ w_i × dq_i) × v
```

Blends in dual quaternion space → preserves rigid transforms → no volume loss. Can cause bulging at joints.

## CCD IK Pseudocode

```
function CCD_IK(joints, end_effector, target, max_iter):
    for iter in 0..max_iter:
        end_pos = FK(end_effector)           # forward kinematics
        if |end_pos - target| < threshold:
            return CONVERGED
        for joint in reverse(end_effector → root):
            end_pos = FK(end_effector)
            joint_pos = FK(joint).position
            to_end = normalize(end_pos - joint_pos)
            to_target = normalize(target - joint_pos)
            axis = cross(to_end, to_target)
            angle = acos(dot(to_end, to_target))
            rotate joint by (axis, angle)    # in local space
    return NOT_CONVERGED
```

**Properties:** Simple, fast, GPU-friendly. Can get stuck in local minima. No rotational constraints by default.

## FABRIK Steps

```
Forward pass (end → root):
    1. Move end effector to target
    2. For each parent joint back to root:
       - Direction = normalized(child_position - this_position)
       - this_position = child_position - direction × bone_length

Backward pass (root → end):
    1. Move root back to its fixed anchor
    2. For each child joint to end effector:
       - Direction = normalized(child_position - this_position)
       - child_position = this_position + direction × bone_length

Repeat until convergence or max iterations.
```

**Properties:** Faster convergence, more natural poses. Hard to add rotational constraints.

## Animation Blending (Crossfading)

```
v_blend(t) = (1 - α) × v_anim_A(u_A(t)) + α × v_anim_B(u_B(t))

where:
    α = blend factor, ramps 0→1 over transition duration
    u_A(t) = (t - t_start_A) / duration_A   (normalized time for anim A)
    u_B(t) = (t - t_start_B) / duration_B   (normalized time for anim B)
```

Each animation is evaluated at its own normalized time independently.

## Animation Compression

| Technique | Method | Typical Savings |
|-----------|--------|-----------------|
| Key reduction | Remove keyframes where interpolation error < threshold | 30-70% |
| Quaternion quantization | Store quat components as 16-bit ints (not 32-bit floats) | 50% |
| Curve fitting | Replace dense keyframes with fitted splines | 40-60% |

**Important:** Before quantizing quaternions, canonicalize sign (ensure `w ≥ 0`) to prevent interpolation artifacts from the q/-q double cover.