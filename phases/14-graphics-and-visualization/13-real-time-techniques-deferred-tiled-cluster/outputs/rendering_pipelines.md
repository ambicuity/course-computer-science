# Real-Time Rendering Pipelines — Reference Card

## Pipeline Comparison

| Feature                | Forward          | Deferred              | Forward+ (Tiled)       | Clustered               |
|------------------------|------------------|-----------------------|------------------------|-------------------------|
| **Light scalability**  | O(obj × light)  | O(obj + pix × light) | O(obj + pix × tile_lt) | O(obj + pix × clust_lt) |
| **MSAA**               | Yes              | No (post-process AA) | Yes                    | Yes                     |
| **Transparency**       | Native           | Separate forward pass | Native                 | Native                  |
| **G-buffer required**  | No               | Yes (3-4 MRTs)       | Optional (depth only)  | Optional (depth only)   |
| **Memory bandwidth**   | Low              | High                  | Medium                 | Medium                  |
| **Per-pixel light cost**| All scene lights| All scene lights     | Per-tile light list    | Per-cluster light list  |
| **Depth handling**     | Per-object       | Per-pixel (G-buffer)  | Min/max per tile        | Explicit depth slices   |
| **Draw calls**         | obj × lights    | obj + full-screen pass| obj + 1 cull pass      | obj + 1 cull pass       |

## When to Use Each

### Forward Rendering
- **Best for:** Mobile, simple scenes, < 8 lights, need MSAA
- **Choose when:** You need transparency, VR (low latency), or simple scenes
- **Avoid when:** Many dynamic lights (> ~10), or you need G-buffer data for post-processing

### Deferred Rendering
- **Best for:** Desktop/console, many lights (50+), need G-buffer for post-fx
- **Choose when:** SSAO, SSR, DOF are important; scene is mostly opaque
- **Avoid when:** Lots of transparency, targeting bandwidth-limited hardware, need MSAA

### Forward+ (Tiled Forward)
- **Best for:** Many lights + transparency + MSAA requirements
- **Choose when:** You want light culling without G-buffer overhead, need MSAA
- **Avoid when:** Severe depth complexity causes per-tile min/max ranges to be loose

### Clustered Rendering
- **Best for:** Scene with high depth complexity, many lights, need tight culling
- **Choose when:** Tiled approach wastes too many lights per tile due to depth spread
- **Avoid when:** Very low light count (overhead of cluster construction isn't worth it)

## Performance Characteristics

### Cost Models

```
Forward:     cost ≈ visible_fragments × num_lights × shading_cost
Deferred:    cost ≈ visible_fragments × geometry_cost + 
                    screen_pixels × num_lights × shading_cost
Forward+:    cost ≈ visible_fragments × geometry_cost +
                    tiles × culling_cost + 
                    screen_pixels × avg_tile_lights × shading_cost
Clustered:   cost ≈ visible_fragments × geometry_cost + 
                    clusters × culling_cost + 
                    screen_pixels × avg_cluster_lights × shading_cost
```

### Memory Usage

```
Forward:     ~1 render target (color), minimal bandwidth
Deferred:    ~3-4 full-screen MRTs (RGBA8 × 2-3, RGBA16F × 1) + depth
             At 1920×1080: ~30-50 MB per frame
Forward+:    ~1 depth target + per-tile light index lists (~1-5 MB)
Clustered:   ~1 depth target + per-cluster light index lists (~2-10 MB)
```

## G-Buffer Layouts

### Common Deferred G-Buffer (4 MRTs)

```
RT0 (RGBA8):  R=albedo.r  G=albedo.g  B=albedo.b  A=specular
RT1 (RGBA8):  R=normal.x  G=normal.y  B=normal.z  A=roughness
RT2 (RGBA16F): R=world_pos.x  G=world_pos.y  B=world_pos.z  A=AO
Depth (24/32-bit float)
```

### Compact G-Buffer (2 MRTs + depth)

```
RT0 (RGBA8):  R=albedo.r  G=albedo.g  B=albedo.b  A=packed_material
RT1 (RG16F):  R=enc(normal).x  G=enc(normal).y  (reconstruct z)
Depth (float): reconstruct world position from depth + inverse proj
```

## Real-World Engine Usage

| Engine        | Default Technique       | Notes                                   |
|---------------|-------------------------|-----------------------------------------|
| Unreal 4/5    | Clustered deferred      | Can switch to forward for mobile VR     |
| Unity HDRP    | Clustered forward/deff | Configurable per project                |
| Godot 4       | Clustered forward / def| Forward on mobile, deferred on desktop   |
| three.js      | Forward                  | Custom deferred renderer available       |
| Frostbite     | Clustered deferred      | Battlefield, FIFA, Star Wars            |
| id Tech 7    | Hybrid (forward + tile)| Doom Eternal                            |

## Key Tradeoffs Summary

```
                    Forward    Deferred    Forward+    Clustered
                    ───────    ────────    ────────    ─────────
Simplicity         ★★★★★     ★★★☆☆      ★★☆☆☆      ★★☆☆☆
Many lights        ★☆☆☆☆    ★★★★☆      ★★★★★      ★★★★★
MSAA               ★★★★★     ★☆☆☆☆      ★★★★★      ★★★★★
Transparency       ★★★★★     ★★☆☆☆      ★★★★★      ★★★★★
Post-processing    ★★☆☆☆    ★★★★★      ★★★☆☆      ★★★☆☆
Memory efficient   ★★★★★     ★★☆☆☆      ★★★★☆      ★★★☆☆
```

## Quick Decision Flowchart

```
Start → How many lights?
         │
         ├─ < 8 lights → Use FORWARD (simple, fast)
         │
         ├─ 8-50 lights → Need MSAA or transparency?
         │               ├─ Yes → Use FORWARD+ (tiled forward)
         │               └─ No  → Use DEFERRED (G-buffer post-fx)
         │
         └─ 50+ lights → High depth complexity?
                         ├─ Yes → Use CLUSTERED (tight culling)
                         └─ No  → Use FORWARD+ or DEFERRED
```