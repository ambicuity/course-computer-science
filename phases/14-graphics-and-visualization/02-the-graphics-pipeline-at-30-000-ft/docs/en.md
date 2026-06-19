# The Graphics Pipeline at 30,000 ft

> Every pixel on your screen survived a gauntlet of math and hardware.

**Type:** Learn
**Languages:** Markdown
**Prerequisites:** Phase 13
**Time:** ~45 minutes

## Learning Objectives

- Trace a triangle from 3D model space through every pipeline stage to its final pixel color.
- Distinguish fixed-function stages (you configure them) from programmable stages (you write shaders for them).
- Map the conceptual pipeline onto real GPU APIs (Vulkan, Metal, WebGPU) and explain where GPU parallelism fits.
- Reason about what data flows between stages and why the pipeline is structured the way it is.

## The Problem

You open a graphics API tutorial. It says: "Create a vertex buffer. Write a vertex shader. Set up a render pass. Submit command buffer." You follow the steps, a triangle appears, and you have zero idea what just happened inside the machine.

The graphics pipeline is the answer. It is the conceptual framework that every GPU, every API, and every rendering technique builds on. Without it, "vertex shader" is just a name; "depth buffer" is just a setting; "rasterization" is a magic word. With it, each of those terms snaps into place as a specific stage with specific inputs, outputs, and invariants you can reason about.

This lesson gives you the 30,000-foot view: the full pipeline from application code down to pixels in the framebuffer. You won't write GLSL or HLSL here — you'll understand *what* each stage does and *why*, so that when you do write shaders, you can predict their behavior instead of cargo-culting tutorials.

## The Concept

### The Pipeline at a Glance

A GPU graphics pipeline processes geometry and turns it into pixels. The stages form a strict data flow:

```
 Application (CPU)
       │
       ▼
 ┌─────────────┐
 │  Vertex     │  ← Programmable (vertex shader)
 │  Processing │
 └──────┬──────┘
        │
        ▼
 ┌─────────────┐
 │  Primitive  │  ← Fixed-function
 │  Assembly   │
 └──────┬──────┘
        │
        ▼
 ┌─────────────┐
 │  Clipping   │  ← Fixed-function
 └──────┬──────┘
        │
        ▼
 ┌─────────────┐
 │Rasterization│  ← Fixed-function
 └──────┬──────┘
        │
        ▼
 ┌─────────────┐
 │  Fragment   │  ← Programmable (fragment/pixel shader)
 │  Processing │
 └──────┬──────┘
        │
        ▼
 ┌─────────────┐
 │   Output    │  ← Fixed-function (with configurable blending)
 │   Merging   │
 └──────┬──────┘
        │
        ▼
   Framebuffer
   (pixels on screen)
```

Each stage is either **fixed-function** (the GPU does it automatically; you configure parameters) or **programmable** (you write a shader that runs per-vertex or per-fragment). Modern APIs have added optional programmable stages (geometry, tessellation, compute), but those are extensions to this core pipeline.

### Data Flow Between Stages

What flows between stages matters more than the stages themselves:

```
 Vertex Buffer          Uniform Buffer         Texture
 (positions,             (matrices,              (image data)
  normals,               material
  UVs)                   params)
      │                       │                     │
      └───────────┬───────────┘                     │
                  │                                 │
                  ▼                                 │
          ┌──────────────┐                          │
          │ Vertex       │◄─────────────────────────┘
          │ Shader       │   (textures available to
          └──────┬───────┘    all programmable stages)
                 │
                 ▼
         gl_Position +      varying/
         per-vertex          out
         attributes          variables
                 │
                 ▼
          ┌──────────────┐
          │ Primitive     │  Assembles vertices into
          │ Assembly       │  triangles/lines/points
          └──────┬────────┘
                 │
                 ▼
          ┌──────────────┐
          │ Clipping      │  Discards geometry outside
          │               │  clip volume
          └──────┬────────┘
                 │
                 ▼
          ┌──────────────┐
          │ Rasterization │  Generates fragments from
          │               │  triangle coverage
          └──────┬────────┘
                 │
                 ▼
         Fragments:       Interpolated
         screen x,y +     attributes
         depth z          (color, UV, normal)
                 │
                 ▼
          ┌──────────────┐
          │ Fragment      │◄─── Textures, uniforms
          │ Shader        │
          └──────┬────────┘
                 │
                 ▼
          Fragment color   depth value
          (RGBA)           for testing
                 │
                 ▼
          ┌──────────────┐
          │ Output        │  Depth test, stencil test,
          │ Merging       │  blending → framebuffer
          └──────┬────────┘
                 │
                 ▼
           Final pixel
```

### Vertex Processing: Where Coordinates Become Clip Space

The vertex shader's main job is transforming vertex positions through a chain of matrix multiplications:

```
Model Space  ──[Model Matrix]──►  World Space
World Space  ──[View Matrix]───►  View Space
View Space   ──[Proj Matrix]──►  Clip Space
Clip Space   ──[w divide]────►  NDC (Normalized Device Coordinates)
NDC          ──[viewport]───►  Screen Space
```

The combined MVP (Model-View-Projection) matrix collapses all three transforms:

```
gl_Position = projection × view × model × vec4(position, 1.0)
```

Clip space is a 4D homogeneous coordinate (x, y, z, w). The GPU does perspective division (divide by w) automatically after the vertex shader, producing NDC coordinates in [-1, 1]. The viewport transform then maps NDC to pixel coordinates.

A concrete example: a vertex at (100, 50, -200) in model space might end up as clip coordinates (0.3, 0.5, 0.8, 1.0), then NDC (0.3, 0.5, 0.8), then screen position (614, 389) on a 1280×720 viewport.

### Primitive Assembly and Clipping

After the vertex shader, the GPU groups vertices back into their primitive type (triangle, line, point). Triangles that fall partially outside the clip volume get **clipped** — the GPU replaces the original triangle with one or more smaller triangles that fit inside.

```
   Before clipping:          After clipping:
   ┌────────────┐            ┌────────────┐
   │  ╲         │            │  ╲         │
   │    ╲   ◄──┼──outside   │    ╲       │
   │      ╲     │            │      ╲─────│─clipped edge
   │        ●   │            │        ●   │
   └────────────┘            └────────────┘
   clip volume               clip volume
```

Clipping happens in clip space *before* perspective division — this is important because perspective division by w would distort vertices that are behind the camera.

### Rasterization: Triangles Become Fragments

Rasterization determines which pixels (more precisely, which **samples**) a triangle covers. The key concept is the **edge function**:

```
For triangle (v0, v1, v2) and pixel center p:

  edge01 = (p.x - v0.x)(v1.y - v0.y) - (p.y - v0.y)(v1.x - v0.x)
  edge12 = (p.x - v1.x)(v2.y - v1.y) - (p.y - v1.y)(v2.x - v1.x)
  edge20 = (p.x - v2.x)(v0.y - v2.y) - (p.y - v2.y)(v0.x - v2.x)

  Inside if all three edges have the same sign (winding order).
```

Each fragment gets **interpolated attributes** — the vertex shader's per-vertex outputs (color, UV, normal) are barycentrically interpolated across the triangle's surface.

```
  v0 (red)          Barycentric coordinates (λ0, λ1, λ2):
    ╲                λ0 + λ1 + λ2 = 1
     ╲               color = λ0·red + λ1·green + λ2·blue
      ╲              uv     = λ0·uv0 + λ1·uv1 + λ2·uv2
  v1 (green)──v2 (blue)
```

### Fragment Processing: The Programmable Pixel Stage

The fragment shader (called "pixel shader" in DirectX) runs once per-fragment. It receives interpolated attributes and produces:

- **RGBA color** — what color this fragment should be
- **Depth value** — optionally overridden from the interpolated depth
- **Stencil value** — rarely, but possible

This is where texture sampling happens:

```
fragment color = texture(sampler2D, interpolated_uv) × material_color
```

This is also where lighting, normal mapping, procedural texturing, and most of the "looks good" work happens.

### Output Merging: The Final Gate

Even after the fragment shader produces a color, the pixel isn't written to the framebuffer until it passes:

1. **Depth test** — is this fragment closer than what's already in the depth buffer?
2. **Stencil test** — is this region masked?
3. **Blending** — how does the new color combine with what's already there? (alpha blending, additive, etc.)

```
  Fragment          Depth            Existing          Final
   color      ──►  test  ──►  stencil test  ──►  blend  ──►  framebuffer
   (RGBA)           (z cmp)        (mask)           (src+dst)
```

**Double buffering**: The display reads from the front buffer while the GPU writes to the back buffer. On vsync, the buffers swap. This prevents tearing — seeing half of one frame and half of the next.

### Fixed-Function vs. Programmable: A Crucial Distinction

| Stage                | Type            | You...                                       |
|----------------------|-----------------|----------------------------------------------|
| Vertex Processing    | Programmable    | Write a vertex shader                       |
| Tessellation         | Programmable    | Write tessellation control + evaluation shaders |
| Geometry Processing  | Programmable    | Write a geometry shader                      |
| Primitive Assembly   | Fixed-function  | Specify primitive topology (triangle list, etc.) |
| Clipping             | Fixed-function  | GPU clips automatically; you set clip planes |
| Rasterization        | Fixed-function  | GPU determines fragment coverage automatically |
| Fragment Processing  | Programmable    | Write a fragment/pixel shader                |
| Output Merging       | Fixed-function  | Configure depth/stencil/blending state       |

Fixed-function stages are not "simpler" — they are often implemented in dedicated hardware that runs far faster than a programmable equivalent could. The GPU does the work; you configure how it does it.

### Where GPU Parallelism Fits

The pipeline is designed for massive parallelism. Understanding *how* is key:

```
 ┌─────────────────────────────────────────────┐
 │              GPU Chip                       │
 │                                             │
 │  ┌──────┐  ┌──────┐  ┌──────┐              │
 │  │ SM 0 │  │ SM 1 │  │ SM 2 │  ...         │
 │  │(warp)│  │(warp)│  │(warp)│              │
 │  │32 ALU│  │32 ALU│  │32 ALU│              │
 │  └──────┘  └──────┘  └──────┘              │
 │                                             │
 │  Each SM:                                   │
 │  - Runs a warp (32 threads) in lockstep     │
 │  - Same instruction, different data (SIMD)  │
 │  - If threads diverge, both paths execute   │
 │    and mask results                         │
 │                                             │
 │  Pipeline parallelism:                      │
 │  - Vertex shader works on batch N           │
 │  - Rasterizer works on batch N-1            │
 │  - Fragment shader works on batch N-2       │
 └─────────────────────────────────────────────┘
```

Key terms:
- **SIMD** (Single Instruction, Multiple Data): One instruction operates on multiple data elements simultaneously. This is the fundamental execution model.
- **SIMT** (Single Instruction, Multiple Threads): NVIDIA's term. Threads share an instruction stream but have their own registers and can diverge (with a performance cost).
- **Warp/Wavefront**: NVIDIA calls groups of 32 threads "warps." AMD calls groups of 64 threads "wavefronts." Both execute in lockstep on SIMD hardware.

The consequence: vertex shaders run independently per-vertex, fragment shaders run independently per-fragment. This is why branching inside shaders is expensive — if two threads in a warp take different branches, *both* branches execute, and results are masked away for the threads that didn't take that path.

### The Pipeline as a Universal Framework

Even ray tracing and path tracing — which are conceptually very different from rasterization — still have a "pipeline" feel:

```
 Ray Tracing "Pipeline":
 ┌──────────────┐
 │ Ray Generation│  (like the Application stage)
 └──────┬───────┘
        ▼
 ┌──────────────┐
 │ Acceleration  │  (like primitive assembly —
 │ Traversal     │   finding what geometry to test)
 └──────┬───────┘
        ▼
 ┌──────────────┐
 │ Intersection  │  (like rasterization —
 │ Testing       │   determining hits)
 └──────┬───────┘
        ▼
 ┌──────────────┐
 │ Shading       │  (like fragment processing)
 └──────┬───────┘
        ▼
 Final radiance
```

Modern APIs (Vulkan, DX12) expose ray-tracing pipeline stages explicitly. The framework scales.

## Build It

Since this is a Markdown (Learn) lesson, we won't write runnable code. Instead, we'll trace a single triangle through every pipeline stage with concrete numbers.

### Step 1: Define a Triangle in Model Space

```
Triangle vertices (model space):

  v0 = ( 0.0,  1.0,  0.0)   ← top
  v1 = (-1.0, -1.0,  0.0)   ← bottom-left
  v2 = ( 1.0, -1.0,  0.0)   ← bottom-right

Vertex colors (for interpolation):
  v0 → red   (1, 0, 0)
  v1 → green (0, 1, 0)
  v2 → blue  (0, 0, 1)
```

### Step 2: Transform to Clip Space (Vertex Shader)

```
Model matrix:  identity (object is at origin, no rotation/scale)
View matrix:   lookAt from (0, 0, 3) toward origin
Proj matrix:   perspective with 90° FOV, 1.0 aspect, near=0.1, far=10

For v0 = (0, 1, 0, 1) in model space:

  world_pos  = model × v0 = (0, 1, 0)        [identity model]
  view_pos   = view × world_pos                [transform to camera space]
  clip_pos   = proj × view_pos                 [project to clip space]

  Assuming clip_pos = (0.0, 0.7, -2.9, -3.0)
```

The vertex shader outputs `gl_Position = clip_pos` plus any varying attributes we want interpolated.

### Step 3: Primitive Assembly

The GPU groups the three processed vertices back into a triangle:
```
  Assembled primitive: triangle(v0_clip, v1_clip, v2_clip)
```

### Step 4: Clipping

All three vertices are inside the clip volume (|x/w| ≤ 1, |y/w| ≤ 1, |z/w| ≤ 1), so no clipping occurs. If one vertex were outside, the GPU would generate a new smaller triangle or quad (split into two triangles).

### Step 5: Perspective Division and Viewport Transform

```
NDC = clip_pos.xyz / clip_pos.w

For v0: NDC = (0.0, 0.7, -2.9) / (-3.0) = (0.0, -0.233, 0.967)

Viewport (640×480):
  screen_x = (NDC.x + 1) / 2 × 640 = 320
  screen_y = (1 - NDC.y) / 2 × 480 = 296   [Y flipped for screen]
```

### Step 6: Rasterization

The GPU walks the triangle's bounding box, testing each pixel center:

```
     300  310  320  330  340
 280  ·    ·    ·    ·    ·
 290  ·    ·    ●    ·    ·     ← v0 somewhere near here
 300  ·    ●●●  ●●   ·    ·
 310  ·   ●●●●  ●●●  ●    ·     ← covered pixels form the triangle
 320  ·  ●●●●● ●●●●  ●    ·
 330  ·   ●●●● ●●●   ·    ·
 340  ·    ●●● ●●    ·    ·     ← v1 to the left, v2 to the right
```

Each covered pixel becomes a **fragment** with barycentrically interpolated attributes.

### Step 7: Fragment Shader

For a fragment at pixel (320, 300) near the center of the triangle:

```
Barycentric coords: λ0 ≈ 0.5, λ1 ≈ 0.25, λ2 ≈ 0.25

Interpolated color:
  r = 0.5×1 + 0.25×0 + 0.25×0 = 0.5
  g = 0.5×0 + 0.25×1 + 0.25×0 = 0.25
  b = 0.5×0 + 0.25×0 + 0.25×1 = 0.25

Fragment shader output: RGBA(0.5, 0.25, 0.25, 1.0)
```

### Step 8: Output Merging

```
1. Depth test:  fragment.z = 0.967 vs depth_buffer[320,300] = ∞
   → Pass (closer than nothing)

2. Stencil test: disabled → pass

3. Blend: src_alpha × src + (1 - src_alpha) × dst
   → 1.0 × (0.5, 0.25, 0.25) + 0.0 × (old_color)
   → (0.5, 0.25, 0.25)

4. Write to framebuffer at (320, 300)
```

One triangle, one pixel, one complete pipeline traversal.

## Use It

### How Real APIs Expose the Pipeline

**WebGL / OpenGL** — The oldest and most forgiving API. Vertex array objects, shader programs, draw calls. The driver does a lot implicitly.

```javascript
// WebGL: the pipeline is mostly implicit
gl.bindVertexArray(vao);
gl.useProgram(shaderProgram);
gl.drawArrays(gl.TRIANGLES, 0, vertexCount);
// Pipeline: vertex shader → primitive assembly → rasterization
//           → fragment shader → depth/blending → framebuffer
```

**Vulkan** — The most explicit API. You describe *everything*: pipeline stages, descriptor layouts, render passes, subpasses, synchronization.

```cpp
// Vulkan: the pipeline is fully explicit
VkGraphicsPipelineCreateInfo pipelineInfo = {};
pipelineInfo.stageCount = 2;
pipelineInfo.pStages = shaderStages;         // vertex + fragment
pipelineInfo.pVertexInputState = &vertexInput;  // vertex format
pipelineInfo.pInputAssemblyState = &inputAssembly; // triangle list
pipelineInfo.pViewportState = &viewportState;   // viewport + scissor
pipelineInfo.pRasterizationState = &rasterState; // cull mode, fill mode
pipelineInfo.pMultisampleState = &msaaState;    // sample count
pipelineInfo.pDepthStencilState = &depthState;  // depth/stencil config
pipelineInfo.pColorBlendState = &blendState;    // blending config
// ... plus render pass, subpass, layout, etc.
```

**Metal** — Apple's middle ground: explicit but less verbose than Vulkan.

```swift
// Metal: pipeline state is a compiled object
let pipelineState = try device.makeRenderPipelineState(
    descriptor: MTLRenderPipelineDescriptor().then {
        $0.vertexFunction = vertexShader
        $0.fragmentFunction = fragmentShader
        $0.colorAttachments[0].pixelFormat = .bgra8Unorm
        $0.depthAttachmentPixelFormat = .depth32Float
    }
)
```

**WebGPU** — The web standard, inspired by Vulkan and Metal.

```javascript
// WebGPU: explicit pipeline, similar to Vulkan but JS-friendly
const pipeline = device.createRenderPipeline({
    layout: 'auto',
    vertex: {
        module: shaderModule,
        entryPoint: 'vs_main',
        buffers: [vertexBufferLayout]
    },
    fragment: {
        module: shaderModule,
        entryPoint: 'fs_main',
        targets: [{ format: navigator.gpu.getPreferredCanvasFormat() }]
    },
    primitive: { topology: 'triangle-list' },
    depthStencil: { depthWriteEnabled: true, format: 'depth24plus' }
});
```

The key insight: all four APIs implement the same conceptual pipeline. The differences are in *how explicitly* you must describe each stage. WebGL hides stages; Vulkan makes you specify everything.

## Read the Source

- **WebGPU specification** — [w3.org/TR/webgpu](https://www.w3.org/TR/webgpu/) — The pipeline state object definition in Section 9 maps directly to the stages described in this lesson.
- **Vulkan specification** — [khronos.org/vulkan](https://www.khronos.org/registry/vulkan/) — `VkGraphicsPipelineCreateInfo` is the pipeline-as-data-structure. Each sub-struct corresponds to a pipeline stage.
- **Mesa 3D** — [mesa3d.org](https://www.mesa3d.org/) — Open-source OpenGL/Vulkan driver. The `src/gallium/` directory contains software implementations of every pipeline stage. Start with `src/gallium/auxiliary/util/u_draw.c` to see how draw calls feed the pipeline.
- **Filament** — [google.github.io/filament](https://google.github.io/filament/Filament.html) — Google's rendering engine documentation. The "Rendering Architecture" chapter has the best pipeline diagram in the industry.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/pipeline_cheatsheet.md` — a one-page reference card mapping each pipeline stage to its GPU API equivalent in Vulkan, Metal, and WebGPU.

## Exercises

1. **Easy** — Draw the pipeline diagram from memory (no peeking). Label each stage as fixed-function or programmable.

2. **Medium** — A triangle's three vertices have clip-space coordinates where one vertex is outside the clip volume (w < 0 or |x/w| > 1). Describe step-by-step what happens during clipping: how many new triangles are generated, and what happens to the varying attributes.

3. **Hard** — Modern ray-tracing pipelines (Vulkan ray queries, DXR) add acceleration-structure traversal and hit-shading stages. Sketch how these stages fit into (or alongside) the traditional graphics pipeline. What data flows between ray-generation, traversal, and hit-shading? How does this parallel the vertex → rasterization → fragment data flow?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Pipeline | "The GPU pipeline" | The ordered sequence of stages that transforms 3D geometry into 2D pixels. Each stage has defined inputs and outputs. |
| Vertex shader | "The vertex program" | A GPU program that runs once per vertex. Its primary job is computing clip-space position; it also passes per-vertex data to fragment shaders via varying/out variables. |
| Fragment shader | "The pixel shader" | A GPU program that runs once per fragment (potential pixel). It computes the final color for that fragment, including texture lookups and lighting. |
| Rasterization | "Triangulating the pixels" | The process of determining which screen-space fragments a primitive covers and interpolating vertex attributes across them. |
| Fragment vs. pixel | Used interchangeably | A fragment is a candidate pixel — it may be discarded by depth/stencil tests. A pixel is a fragment that survived all tests and was written to the framebuffer. |
| Fixed-function | "Hardware stage" | A pipeline stage implemented in dedicated silicon. You configure it (set blend mode, cull mode, etc.) but you don't write code for it. |
| Clip space | "After the vertex shader" | The 4D homogeneous coordinate system (x, y, z, w) that the vertex shader outputs. The GPU uses w for perspective division and x, y, z for clipping. |
| Warp/Wavefront | "A group of threads" | A group of 32 (NVIDIA) or 64 (AMD) threads that execute in lockstep on SIMD hardware. Divergent branches within a warp are serialized. |
| Double buffering | "Vsync, swap chain" | Using two framebuffers so the display reads one while the GPU writes the other, preventing visible tearing. |
| Barycentric coordinates | "Triangle weights" | The three weights (λ0, λ1, λ2) that express a point inside a triangle as a weighted combination of its three vertices. Used for attribute interpolation. |

## Further Reading

- **"A Trip Down the Graphics Pipeline"** — Jim Blinn's classic column series. Start with "The Triangle's Triangle" for rasterization intuition.
- **"The Graphics Pipeline"** chapter in *Foundations of Game Engine Development, Volume 2* by Eric Lengyel — rigorous but readable, with the math behind each transform.
- **GPU Architecture** — [lgdc.io/gpu-architecture](https://www.lgcfs.com/blog/gpu-architecture-101) — How SIMD hardware actually dispatches warps and why branch divergence hurts.
- **WebGPU specification** — [w3.org/TR/webgpu](https://www.w3.org/TR/webgpu/) — The most readable modern GPU specification. Section 9 (Pipeline) maps directly to this lesson.
- **Filament Material Guide** — [google.github.io/filament/Filament.html](https://google.github.io/filament/Filant.html) — See how a production engine organizes the pipeline for physically-based rendering.