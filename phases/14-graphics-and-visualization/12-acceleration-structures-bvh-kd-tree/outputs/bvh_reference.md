# BVH & kd-Tree Reference Card

## AABB — Axis-Aligned Bounding Box

```
AABB { min: Vec3, max: Vec3 }
```

### Slab Method Intersection

For each axis, compute the ray's entry/exit t values:

```
t_min[axis] = (aabb.min[axis] - ray.origin[axis]) / ray.direction[axis]
t_max[axis] = (aabb.max[axis] - ray.origin[axis]) / ray.direction[axis]
```

If `ray.direction[axis] < 0`, swap `t_min` and `t_max` for that axis.

```
t_enter = max(t_min[x], t_min[y], t_min[z])
t_exit  = min(t_max[x], t_max[y], t_max[z])
```

**Hit if:** `t_enter < t_exit` AND `t_exit > 0`

If `ray.direction[axis] ≈ 0` (parallel): check if `origin[axis]` is within the slab. If not, no intersection.

---

## BVH — Bounding Volume Hierarchy

### Construction (Middle Split / Centroid Median)

```
function build_bvh(primitives, depth):
    bounds = union of all primitive AABBs
    if len(primitives) <= LEAF_SIZE:
        return Leaf { bounds, primitives }

    axis = depth % 3
    sort primitives by centroid along axis
    mid = len(primitives) / 2

    left  = build_bvh(primitives[0:mid], depth+1)
    right = build_bvh(primitives[mid:],  depth+1)
    return Inner { bounds, left, right }
```

### SAH — Surface Area Heuristic

```
C(split) = C_trav
         + (SA_left  / SA_parent) * N_left  * C_isect
         + (SA_right / SA_parent) * N_right * C_isect
```

- `C_trav` ≈ 1.0 — cost of traversing one BVH node
- `C_isect` ≈ 1.0 — cost of one primitive intersection test
- `SA` — surface area of the bounding box
- Choose the split position that minimizes `C(split)`
- If `C(split) >= N * C_isect`, make a leaf instead of splitting

**Surface Area of AABB:**

```
SA = 2 * ((dx*dy) + (dy*dz) + (dz*dx))
where dx = max.x - min.x, dy = max.y - min.y, dz = max.z - min.z
```

### BVH Traversal (Iterative with Stack)

```
function intersect_bvh(ray, root):
    closest_t = infinity
    hit = none
    stack = [root]
    while stack not empty:
        node = stack.pop()
        if ray does not intersect node.aabb (0.001, closest_t):
            continue
        if node is Leaf:
            for each primitive in node.primitives:
                t = primitive.intersect(ray)
                if t and t < closest_t:
                    closest_t = t
                    hit = HitRecord(t, ...)
        else:  # Inner node
            stack.push(node.left)
            stack.push(node.right)
    return hit
```

---

## kd-Tree

### Structure

Each node stores: `axis` (x/y/z), `split_position`, `left_child`, `right_child`

Leaves store: list of primitives whose AABBs overlap the cell

### Construction (SAH-based)

```
function build_kdtree(bounds, primitives, depth):
    if len(primitives) <= LEAF_SIZE:
        return Leaf { primitives }

    axis = depth % 3
    best_pos, best_cost = find_SAH_split(axis, bounds, primitives)

    if best_cost >= len(primitives) * C_isect:
        return Leaf { primitives }

    left_prims  = [p for p in primitives if p.centroid[axis] <= best_pos]
    right_prims = [p for p in primitives if p.centroid[axis] >  best_pos]
    # Primitives straddling the split go into BOTH

    left_bounds  = clip bounds to left of best_pos
    right_bounds = clip bounds to right of best_pos

    return Inner {
        axis, split: best_pos,
        left:  build_kdtree(left_bounds,  left_prims,  depth+1),
        right: build_kdtree(right_bounds, right_prims, depth+1)
    }
```

### kd-Tree Traversal (Havran-style)

```
function intersect_kdtree(ray, node, t_min, t_max):
    if node is Leaf:
        return closest intersection with node.primitives in [t_min, t_max]

    t_split = (node.split - ray.origin[node.axis]) / ray.direction[node.axis]
    (first, second) = if ray.direction[node.axis] > 0
                         then (node.left, node.right)
                         else (node.right, node.left)

    if t_split >= t_max:    # ray ends before split plane
        return intersect_kdtree(ray, first,  t_min, t_max)
    elif t_split <= t_min:  # ray starts after split plane
        return intersect_kdtree(ray, second, t_min, t_max)
    else:                    # ray crosses split plane
        hit = intersect_kdtree(ray, first,  t_min, t_split)
        if hit and hit.t <= t_split:
            return hit
        return intersect_kdtree(ray, second, t_split, t_max)
```

---

## BVH vs. kd-Tree Comparison

| Property | BVH | kd-Tree |
|----------|-----|---------|
| **Partitioning** | Object (each primitive in one node) | Spatial (space split into cells) |
| **Sibling overlap** | Possible (AABBs may overlap) | None (cells partition space) |
| **Primitive duplication** | None (each in one leaf) | Possible (straddling split planes) |
| **Construction time** | O(n log n) | O(n log² n) with SAH |
| **Refitting (dynamic)** | Easy — update AABBs bottom-up | Hard — typically full rebuild |
| **Memory** | Lower (no duplication) | Higher (duplicated primitives) |
| **Traversal quality** | Good (some redundancy from overlap) | Better (spatial partitioning, no overlap) |
| **GPU friendliness** | Good (stack-based, used in OptiX/DXR) | Poorer (requires stack + backtracking) |
| **When to use** | Dynamic scenes, GPU rendering | Static scenes, CPU rendering |

---

## When Acceleration Matters

| Primitives | Brute-force tests | BVH ~tests | Speedup |
|------------|-------------------|------------|---------|
| 10 | 10 | ~4 | ~2× |
| 100 | 100 | ~7 | ~14× |
| 1,000 | 1,000 | ~10 | ~100× |
| 1,000,000 | 1,000,000 | ~20 | ~50,000× |

**Rule of thumb:** Acceleration structures become worthwhile at ~100 primitives. Below that, brute force is often faster due to BVH traversal overhead.