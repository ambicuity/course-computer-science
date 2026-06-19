# Real-Time Techniques — Deferred, Tiled, Cluster

> How modern engines render hundreds of lights in real time by decoupling geometry from lighting.

**Type:** Learn
**Languages:** GLSL, Rust
**Prerequisites:** Phase 14 lessons 01–12
**Time:** ~75 minutes

## Learning Objectives

- Explain why forward rendering scales as O(objects × lights) and why that breaks down.
- Describe the deferred rendering pipeline: geometry pass → G-buffer → lighting pass.
- List the contents of a typical G-buffer and explain what each channel is used for.
- Compare tiled and clustered forward rendering and explain how they extend tiling into depth.
- Analyze the tradeoffs: when deferred wins, when forward wins, when hybrid approaches are best.
- Implement a deferred renderer (geometry pass + lighting pass) in GLSL and simulate it in Rust.

## The Problem

Imagine a scene with 10 objects and 1,000 point lights. In a classic forward renderer,
every fragment of every object must test against every light. The cost is:

```
fragments × lights = O(objects × lights)
```

At 10 objects × 1,000 lights, that's 10,000 shading operations — and most fragments
won't even be affected by most lights. A pixel in the corner of the screen doesn't need
to sample a light on the opposite side of the scene.

This is the **forward rendering bottleneck**: you pay for every light on every surface,
even when the light contributes nothing. The question becomes:

> How can we make the cost proportional to the lights that *actually affect* each pixel,
> rather than all lights in the scene?

Three answers have emerged over the last two decades: **deferred**, **tiled**, and
**clustered** rendering. Each attacks the O(objects × lights) problem from a different angle.

## The Concept: Forward Rendering and Its Cost

In forward rendering, the GPU draws each object and shades it against all relevant
lights in a single pass (or multipass with additive blending). The pseudocode is:

```
for each object in scene:
    for each light affecting object:
        shade(object, light)  // accumulate contribution
```

This is simple and works well for few lights. But the inner loop grows with the number
of lights. In a multipass approach (one pass per light with additive blending), you issue
one draw call per light per object:

```
for each light:
    for each object affected by light:
        draw(object, light)  // additive blend
```

Either way, the cost scales as O(objects × lights). At hundreds of lights, this kills
performance.

```
  Forward Rendering Pipeline
  ┌─────────────────────────────────┐
  │  For each object:               │
  │    ┌─────────────────────────┐  │
  │    │  For each light:        │  │
  │    │    compute shading      │  │
  │    │    accumulate color     │  │
  │    └─────────────────────────┘  │
  │  Output final pixel color       │
  └─────────────────────────────────┘

  Cost = objects × lights × shading_cost
```

### Worked Example: 100 lights, 1M fragments

Suppose a 1920×1080 scene (~2M fragments, ~1M visible after depth test) with 100 lights
and 10 objects:

- **Forward (single pass):** Each of 1M fragments evaluates 100 lights = 100M light evaluations.
- **Forward (multipass):** 100 draw calls, each touching ~1M fragments = 100M evaluations.
- **If only 5 lights affect the average pixel:** Ideally only 5M evaluations needed.
  The gap is 20× — that's the opportunity.

## Deferred Rendering: Decouple Geometry from Lighting

Deferred rendering splits the pipeline into two passes:

1. **Geometry pass:** Render all objects, writing surface properties to multiple render
   targets (the **G-buffer**). No lighting is computed.
2. **Lighting pass:** For each pixel, read the G-buffer and compute lighting from all
   relevant lights.

```
  Deferred Rendering Pipeline
  ┌──────────────────────┐     ┌─────────────────────┐
  │  GEOMETRY PASS       │     │  LIGHTING PASS       │
  │                      │     │                       │
  │  vertex shader       │     │  full-screen quad     │
  │      │               │     │      │                │
  │  fragment shader     │     │  read G-buffer texels  │
  │      │               │     │      │                │
  │  write G-buffer:     │     │  for each light:      │
  │    RT0: albedo+spec  │────▶│    compute contrib    │
  │    RT1: normal+rough │     │    accumulate color    │
  │    RT2: position     │     │      │                │
  │    RT3: depth        │     │  output final color    │
  └──────────────────────┘     └─────────────────────┘

  Cost = objects × geometry_cost + pixels × lights × shading_cost
       = O(objects + pixels × lights)
```

The key insight: geometry and lighting are **decoupled**. Adding more objects only costs
the geometry pass. Adding more lights only costs the lighting pass. The total is
**O(objects + lights)** for a fixed screen resolution, because every pixel evaluates
every light but we've eliminated the object×light coupling.

### The G-Buffer

The G-buffer (geometry buffer) stores everything the lighting pass needs to reconstruct
surface shading at each pixel:

```
  ┌──────────────────────────────────────────────────┐
  │                  G-Buffer Layout                  │
  ├────────────────────┬─────────────────────────────┤
  │ Render Target       │ Contents                     │
  ├────────────────────┼─────────────────────────────┤
  │ RT0 (RGBA8)         │ R: albedo.r  G: albedo.g    │
  │                     │ B: albedo.b  A: specular     │
  ├────────────────────┼─────────────────────────────┤
  │ RT1 (RGBA8)         │ R: normal.x  G: normal.y    │
  │                     │ B: normal.z  A: roughness    │
  ├────────────────────┼─────────────────────────────┤
  │ RT2 (RGBA16F)       │ R: world_pos.x              │
  │                     │ G: world_pos.y               │
  │                     │ B: world_pos.z  A: unused    │
  ├────────────────────┼─────────────────────────────┤
  │ GL_DEPTH24_STENCIL8 │ Depth (reconstruct position) │
  └────────────────────┴─────────────────────────────┘
```

In practice, positions are often reconstructed from depth rather than stored explicitly,
saving one render target. A compact G-buffer might use only 2–3 render targets.

### Advantages of Deferred Rendering

1. **Decoupled complexity:** Adding objects is cheap (geometry pass only). Adding lights
   is proportional to screen pixels, not objects.
2. **Many lights:** Hundreds or thousands of lights are feasible because each pixel only
   reads its G-buffer data and loops over lights that affect it.
3. **Consistent shading:** Every surface gets the same lighting evaluation — no special
   cases for different object types.
4. **Post-processing friendly:** The G-buffer gives you world-space normals, depth, and
   material properties "for free" — useful for SSAO, SSR, DOF, etc.

### Disadvantages of Deferred Rendering

1. **No MSAA:** G-buffer MRTs can't be multisampled easily. Anti-aliasing must use
   post-process (FXAA, SMAA, TAA). The G-buffer stores per-pixel surface data; MSAA
   requires multiple samples per pixel, multiplying bandwidth.
2. **Bandwidth-heavy:** Writing and reading 3–4 full-screen render targets is a lot of
   memory traffic. On bandwidth-limited GPUs, this dominates.
3. **No transparency:** Only the nearest surface is stored in the G-buffer. Transparent
   objects must be rendered separately in a forward pass after the lighting pass.
4. **Multiple render targets:** Requires MRT support (core since OpenGL 3.0 / D3D10).
   Encoding material variety into a fixed G-buffer layout constrains shader flexibility.
5. **No hardware depth-based optimizations:** In forward rendering, early-Z can skip
   shading for occluded fragments. In the deferred lighting pass, every pixel is shaded
   regardless (though stencil culling helps).

### Worked Example: Deferred vs Forward with 100 lights

| Metric             | Forward            | Deferred              |
|--------------------|--------------------|-----------------------|
| Draw calls         | 10 objects × 100 lights = 1000 | 10 objects + 1 lighting full-screen pass |
| Fragment ops       | 1M × 100 = 100M    | 1M (geom) + 1M × 100 (light) = 101M |
| But with culling   | 1M × ~5 avg = 5M  | 1M (geom) + 1M × ~5 (culled) = 6M |
| Memory bandwidth   | low                | high (G-buffer read/write) |
| MSAA               | yes                | no (need post-process) |

The total operations are similar without culling, but deferred makes *per-pixel light
culling* natural: in the lighting pass, you only evaluate lights that overlap the
current pixel's position. Further optimizations (tiled, clustered) reduce this further.

## Tiled Rendering: Divide and Conquer in 2D

Tiled rendering subdivides the screen into small tiles (typically 16×16 pixels) and
assigns lights to each tile. Only lights that overlap a tile are evaluated for pixels
in that tile.

```
  Screen divided into 16×16 tiles:

  ┌────┬────┬────┬────┐
  │ T0 │ T1 │ T2 │ T3 │   Each tile has a light list:
  │2L  │3L  │1L  │0L  │   T0 has 2 lights, T1 has 3,
  ├────┼────┼────┼────┤   T2 has 1, T3 has 0.
  │ T4 │ T5 │ T6 │ T7 │
  │4L  │5L  │3L  │1L  │   Avg lights per tile ≈ 5
  ├────┼────┼────┼────┤   vs. total lights = 100
  │ T8 │ T9 │T10 │T11 │
  │3L  │4L  │2L  │1L  │   Cost reduction: 100 → ~5
  ├────┼────┼────┼────┤
  │T12 │T13 │T14 │T15 │
  │2L  │1L  │1L  │0L  │
  └────┴────┴────┴────┘
```

**How light assignment works:**

1. For each tile, compute a min/max depth from the depth buffer (or G-buffer).
2. For each point light, compute a screen-space bounding sphere.
3. If the light's sphere overlaps the tile's depth range, add it to the tile's light list.

```
  Light Culling (per tile):
  ┌─────────────────────────────────┐
  │  for each tile:                 │
  │    compute tile min/max depth   │
  │    for each point light:        │
  │      project light sphere      │
  │      if overlaps tile AABB and │
  │         depth range:            │
  │        add to tile light list   │
  └─────────────────────────────────┘
```

This is done as a **compute shader** pass (or on the CPU with OpenCL/DirectCompute)
before the lighting pass. Then the lighting pass reads the per-tile light list and
only evaluates those lights.

### Tiled Forward (Forward+)

Tiled rendering doesn't require a G-buffer. **Forward+ (tiled forward)** uses the same
tile-based light culling but stays in a forward rendering pipeline:

1. **Cull pass:** Dispatch compute shader to assign lights to tiles.
2. **Render pass:** For each object, in the fragment shader, look up which tile the
   fragment falls in, get that tile's light list, shade against only those lights.

This gives you the light-culling benefit of deferred without the G-buffer bandwidth cost
or the MSAA limitation.

### Tiled Rendering Limitation

The tile approach has a flaw: a tile's depth range might span the entire scene
(e.g., a tile containing both a near wall and a far wall). Lights between those surfaces
are included in the tile's list even though they affect neither surface.

```
  Side view of a tile with bad depth range:

  Near wall ───────────────────── near depth
         │                    │
         │  Light A (useless) │   ← light is between surfaces,
         │                    │      illuminates nothing visible
  Far wall  ───────────────────── far depth

  The tile's light list includes Light A, but no visible surface
  is actually lit by it. This is wasted work.
```

This is where **clustered rendering** comes in.

## Clustered Rendering: Extend Tiling into Depth

Clustered rendering extends the 2D tile grid into a 3D grid of **frustum clusters**.
Instead of a 2D depth range per tile, each cluster covers a small depth slice.

```
  2D Tiled:                   3D Clustered:

  ┌────┬────┐                ┌────┬────┐
  │ T0 │ T1 │                │C00 │C10 │  ← near slice
  ├────┼────┤    ───────►    ├────┼────┤
  │ T2 │ T3 │                │C01 │C11 │  ← mid slice
  └────┴────┘                ├────┼────┤
                              │C02 │C12 │  ← far slice
                              └────┴────┘

  Each cluster = tile × depth slice
  Light lists: one per cluster, not per tile
```

The depth slices are typically distributed exponentially (more slices near the camera,
fewer far away), matching how depth precision works.

**Light assignment** becomes more precise:

```
  for each cluster (tile_x, tile_y, depth_slice):
    compute cluster AABB in view space
    for each light:
      if light sphere overlaps cluster AABB:
        add to cluster's light list
```

A light that was in the "2D tile" but between two surfaces will end up in a depth slice
that contains no visible geometry, so no fragment will ever look up that light list.

### Clustered vs Tiled: Summary

| Aspect              | Tiled               | Clustered                  |
|---------------------|---------------------|----------------------------|
| Subdivision         | 2D (screen tiles)   | 3D (tiles × depth slices) |
| Depth handling      | Min/max per tile    | Explicit depth slices      |
| Light lists         | Per tile             | Per cluster                |
| Wasted work         | Lights in empty depth | Minimal                    |
| Memory for lists    | Modest              | Higher (more clusters)     |
| Compute culling     | Moderate            | Higher (more clusters)     |

## Deferred vs Forward+: When to Use What

```
  ┌──────────────────────────────────────────────────────────────────┐
  │                    Technique Selection Guide                      │
  ├────────────┬─────────────────────────────────────────────────────┤
  │ Technique  │ Choose When...                                       │
  ├────────────┼─────────────────────────────────────────────────────┤
  │ Forward    │ < 8 lights, need MSAA, mobile, simple scenes         │
  │ Deferred   │ > 50 lights, no transparency, desktop/console       │
  │ Forward+   │ Many lights + need MSAA/transparency                │
  │ Clustered  │ Many lights + depth complexity + bandwidth concern   │
  └────────────┴─────────────────────────────────────────────────────┘
```

- **Forward** wins on mobile, low-light scenes, or when you need MSAA and transparency.
- **Deferred** wins when you have many lights and want easy access to G-buffer data for
  post-processing. Most AAA console/PC games use some form of deferred.
- **Forward+ (tiled forward)** wins when you need both many lights *and* MSAA or
  transparency. Used by some Unity and custom engines.
- **Clustered** wins when depth complexity causes tile inefficiency. Used by
  Unreal Engine 4+ (clustered deferred), EA's Frostbite, and many modern engines.

### Real-World Usage

| Engine        | Technique                         | Notes                                  |
|---------------|-----------------------------------|----------------------------------------|
| Unreal Engine | Clustered deferred (default)     | Can switch to forward for mobile       |
| Unity HDRP    | Clustered forward / deferred      | Configurable per project               |
| three.js      | Forward (default)                 | WebGL limitations; custom deferred exists |
| Godot 4       | Clustered forward (mobile) / deferred (desktop) | Switchable              |
| Frostbite     | Clustered deferred                | Battlefield series, FIFA               |

## Build It

### Step 1: GLSL — Deferred Rendering Shaders

The `code/main.glsl` file contains two shader programs:

1. **Geometry pass:** Vertex + fragment shader that writes position, normal, albedo,
   and depth to a G-buffer (multiple render targets).
2. **Lighting pass:** Full-screen quad fragment shader that reads G-buffer textures and
   computes Blinn-Phong lighting per pixel.

### Step 2: Rust — CPU Simulation

The `code/main.rs` file simulates deferred rendering on the CPU:

1. Render geometry into G-buffer arrays (position, normal, albedo, depth).
2. Lighting pass reads G-buffer and computes shading.
3. Compare against a naive forward approach to demonstrate the cost difference.
4. Outputs PPM images for both forward and deferred results.

## Use It

In production engines, these techniques are deeply integrated:

- **Unreal Engine:** Look at `Engine/Source/Runtime/Renderer/Private/ClusteredDeferredShading.cpp`
  for the clustered deferred implementation. The light grid is built in a compute shader
  and consumed in the deferred lighting shader.
- **Unity HDRP:** The `com.unity.render-pipelines.high-definition` package contains
  `LightLoop.cs` and `LightDensityCluster.cs` for clustered light assignment.
- **three.js:** The `WebGLDeferredRenderer` extension demonstrates a pure WebGL
  deferred pipeline using MRT with `gl.FRAMEBUFFER`.

Comparing our hand-built version to production:

| Our implementation            | Production engines                           |
|-------------------------------|---------------------------------------------|
| Fixed G-buffer layout         | Configurable G-buffer with material IDs     |
| O(n) light loop per pixel     | Tiled/clustered light lists, compute culling|
| No transparency               | Separate forward pass for transparents      |
| No shadows                    | Shadow maps, ray-traced shadows             |
| No SSAO/SSR                   | Full post-process stack                     |
| Single Blinn-Phong model      | PBR (GGX, IBL, transmission)                |

## Read the Source

- **Unreal Engine Clustered Deferred:** `Engine/Source/Runtime/Renderer/Private/ClusteredDeferredShading.cpp`
  — The core light-grid build and deferred shading pass.
- **Frostbite Architecture:** "Deferred Rendering for Current Engines" (SIGGRAPH 2013)
  — The presentation that popularized clustered deferred in production.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`rendering_pipelines.md`** — A reference card comparing forward, deferred, tiled,
  and clustered rendering, with pros/cons, performance characteristics, and when to
  use each technique.

## Exercises

1. **Easy** — Modify the GLSL G-buffer to store only depth instead of position, and
   reconstruct world position from depth + inverse projection matrix in the lighting pass.
2. **Medium** — Implement tiled light culling in a compute shader: divide the screen into
   16×16 tiles, compute min/max depth, and assign lights to tiles. Output the per-tile
   light count as a debug visualization.
3. **Hard** — Extend the tiled culling to clustered: add exponential depth slices and
   assign lights to 3D clusters. Compare the light list sizes (total entries) between
   tiled and clustered for a scene with high depth complexity.

## Key Terms

| Term                | What people say           | What it actually means                                  |
|---------------------|---------------------------|---------------------------------------------------------|
| G-buffer            | "the G-buffer"           | A set of render targets storing surface properties per pixel |
| Deferred rendering  | "deferred shading"       | Split pipeline: geometry pass → G-buffer → lighting pass |
| Forward+            | "tiled forward"           | Forward rendering with per-tile light lists from compute culling |
| Clustered rendering | "3D tiling"              | Extend 2D tiles into depth slices for tighter light lists |
| Light culling       | "frustum culling lights" | Determining which lights overlap which screen regions   |
| MRT                 | "multiple render targets"| Writing to multiple textures in a single fragment shader pass |
| Tiled rendering     | "tile-based"             | Subdivide screen into 2D tiles, assign lights per tile  |
| Depth slice         | "z-slice"                | A range of depth values defining one layer in a cluster grid |

## Further Reading

- **"Deferred Shading"** (Hargreaves & Harris, GDC 2004) — The original presentation
  introducing deferred shading to the game industry.
- **"Tiled Deferred Shading"** (O'Donnell & Chajdas, SIGGRAPH 2012) — The tiled approach
  and its benefits over naive deferred.
- **"Clustered Deferred Shading"** (Olsson et al., HPG 2012) — The paper that introduced
  clustered shading with depth slicing.
- **"Forward+: A Next-Generation Rendering Pipeline"** (Takahashi & Harada, SIGGRAPH 2013) —
  Tiled forward rendering that preserves MSAA and transparency.
- **"Real-Time Rendering" (4th ed.)**, Akenine-Möller et al., Chapter 20 — The
  definitive reference for real-time rendering pipelines, including deferred methods.