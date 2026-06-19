# Notes — The Graphics Pipeline at 30,000 ft

## Pipeline Overview

```
 ┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
 │   Input      │───►│  Vertex      │───►│  Rasterizer  │───►│  Fragment    │───►│  Output      │
 │  Assembly    │    │  Processing  │    │              │    │  Processing  │    │  Merging     │
 └──────────────┘    └──────────────┘    └──────────────┘    └──────────────┘    └──────────────┘
   CPU feeds           Vertex shader       Fixed-function      Fragment shader     Depth/stencil/
   vertex data         (programmable)       triangle walk       (programmable)      blending
```

## Stage-by-Stage Data Flow

### 1. Application / Input Assembly (CPU → GPU)

- CPU submits vertex data via vertex buffers and index buffers.
- Input assembly reads vertices and groups them into primitives (points, lines, triangles) based on the topology you specify.
- Input: raw vertex arrays on the CPU side.
- Output: a stream of vertices tagged with their primitive topology, ready for the vertex shader.

### 2. Vertex Processing (Programmable: Vertex Shader)

- Runs once per vertex.
- Input: one vertex with its attributes (position, normal, UV, color).
- Output: `gl_Position` (clip-space coordinates) + any varying/out attributes.
- The canonical transform chain:

```
  object_pos ──[M]──► world_pos ──[V]──► view_pos ──[P]──► clip_pos

  M = Model matrix  (object → world)
  V = View matrix   (world → camera)
  P = Projection    (camera → clip space)

  MVP = P × V × M
  gl_Position = MVP × vec4(position, 1.0)
```

- Clip space is homogeneous: (x, y, z, w). A vertex is inside the view frustum if:
  -w ≤ x ≤ w, -w ≤ y ≤ w, 0 ≤ z ≤ w (OpenGL convention, z may differ in other APIs)

### 3. Primitive Assembly

- Groups post-transform vertices into primitives (triangles, lines, points).
- Handles primitive restart, adjacency, and strip winding order.
- Can optionally emit geometry to geometry shader (if present).
- Input: processed vertex stream.
- Output: assembled primitives (triangles with 3 vertices each).

### 4. Clipping

- Removes geometry outside the view frustum.
- Triangles straddling the clip boundary are clipped: new vertices are inserted on the clip plane.
- Original vertex attributes at clip points are linearly interpolated.
- Input: assembled primitives in clip space.
- Output: clipped primitives (all vertices inside or on the clip volume).

```
  Before:                After:
     ╱│                     ╱│
    ╱ │                   ╱  │  ← new vertex on clip plane
   ╱  │                  ╱   │     with interpolated attributes
  ●   │                ●─────│
  │   │                │     │
  │   │                │     │
  ●   ●                ●─────●
     ╱                     ╱
   Outside              Inside only
```

### 5. Rasterization

- Determines which fragments (sample positions) each primitive covers.
- Uses edge functions to test containment:

```
  For triangle (v0, v1, v2) and point p:
    E01(p) = (p.x - v0.x)(v1.y - v0.y) - (p.y - v0.y)(v1.x - v0.x)
    E12(p) = (p.x - v1.x)(v2.y - v1.y) - (p.y - v1.y)(v2.x - v1.x)
    E20(p) = (p.x - v2.x)(v0.y - v2.y) - (p.y - v2.y)(v0.x - v2.x)

    Inside if all three have the same sign (consistent winding).

  Barycentric coordinates:
    λ0 = E12(p) / E12(v0)   weight of v0
    λ1 = E20(p) / E20(v1)   weight of v1
    λ2 = E01(p) / E01(v2)   weight of v2

    λ0 + λ1 + λ2 = 1

  Interpolated attribute:
    attr(p) = λ0·attr(v0) + λ1·attr(v1) + λ2·attr(v2)
```

- Perspective-correct interpolation divides by w at each vertex, interpolates, then multiplies back.
- Input: clipped primitives in screen space.
- Output: a stream of fragments, each with screen (x, y), depth z, and interpolated attributes.

### 6. Fragment Processing (Programmable: Fragment Shader)

- Runs once per fragment.
- Input: interpolated attributes + uniforms + textures.
- Output: RGBA color + (optional) depth override.
- This is where lighting, texturing, and shading decisions are made:

```
  // Simplified Phong-like fragment shader pseudocode
  vec3 normal  = normalize(interpolated_normal);
  vec3 lightDir = normalize(light_position - fragment_position);
  float diff   = max(dot(normal, lightDir), 0.0);
  vec3 color   = diff * light_color * texture(diffuseMap, interpolated_uv).rgb;
  output_color = vec4(color, 1.0);
```

### 7. Output Merging (Fixed-Function, Configurable)

- Depth test: compare fragment.z against depth buffer (LESS, LEQUAL, GREATER, etc.).
- Stencil test: mask off regions of the framebuffer.
- Blending: combine fragment color with framebuffer color using blend operations.

```
  Common blend modes:
    No blending:     dst = src
    Alpha blending:  dst = src.a × src + (1 - src.a) × dst
    Additive:        dst = src + dst
    Multiply:        dst = src × dst
```

- Double buffering: front buffer (display reads) + back buffer (GPU writes). Swap on vsync.
- Input: fragment shader output (color + depth).
- Output: final pixel in the framebuffer.

## Key Equations

### Perspective Projection Matrix (OpenGL)

```
         │ 2n/(r-l)    0      (r+l)/(r-l)        0        │
         │                                                  │
         │    0     2n/(t-b)  (t+b)/(t-b)        0        │
  P  =   │                                                  │
         │    0        0      -(f+n)/(f-n)  -2fn/(f-n)      │
         │                                                  │
         │    0        0          -1              0        │

  n = near, f = far, l/r/t/b = frustum bounds at near plane
```

### Viewport Transform

```
  screen_x = (NDC_x + 1) / 2 × viewport_width  + viewport_x
  screen_y = (1 - NDC_y) / 2 × viewport_height + viewport_y   [Y flipped]
```

### Perspective-Correct Interpolation

```
  For attribute A at vertices v0, v1, v2 with clip-space w0, w1, w2:

  bary = (λ0, λ1, λ2)  — screen-space barycentric
  w_interp = λ0/w0 + λ1/w1 + λ2/w2
  A_interp = (λ0·A0/w0 + λ1·A1/w1 + λ2·A2/w2) / w_interp
```

## Fixed-Function vs Programmable Summary

```
  Stage                   Type            Config Example
  ─────────────────────   ────            ──────────────
  Input Assembly          Fixed           primitiveTopology: TRIANGLE_LIST
  Vertex Shader          Programmable    write a .vert / .vs shader
  Tessellation Shaders   Programmable    (optional) .tesc / .tese
  Geometry Shader         Programmable    (optional) .geom / .gs
  Primitive Assembly      Fixed           frontFace: CCW, cullMode: BACK
  Clipping                Fixed           clip planes, viewport bounds
  Rasterization           Fixed           rasterizerDiscardEnable: false
  Fragment Shader         Programmable    write a .frag / .ps shader
  Sample Shading          Fixed           multisampleCount: 1
  Depth/Stencil Test      Fixed           compare: LESS, write: true
  Color Blending          Fixed           srcFactor: SRC_ALPHA, dstFactor: ONE_MINUS_SRC_ALPHA
  Framebuffer             Fixed           double-buffered, vsync swapchain
```

## GPU Parallelism Model

```
  ┌─────────────────────────────────────────────┐
  │ Streaming Multiprocessor (SM / CU)          │
  │                                              │
  │  ┌─────────┐  ┌─────────┐  ┌─────────┐      │
  │  │ Warp 0  │  │ Warp 1  │  │ Warp 2  │ ...  │
  │  │ 32 lanes│  │ 32 lanes│  │ 32 lanes│      │
  │  └─────────┘  └─────────┘  └─────────┘      │
  │                                              │
  │  All lanes in a warp execute the same       │
  │  instruction. Divergent branches are         │
  │  serialized (both paths run, masked).        │
  └─────────────────────────────────────────────┘

  Pipeline parallelism:
    Frame N:  Vertices → VertShader
    Frame N-1:            Raster → FragShader
    Frame N-2:                      Depth/Blend → FB

  Data parallelism:
    Thousands of vertices processed simultaneously across many SMs.
    Thousands of fragments processed simultaneously across many SMs.
```

## Conceptual Pipeline → API Mapping

| Conceptual Stage      | Vulkan                                  | Metal                                  | WebGPU                                |
|-----------------------|-----------------------------------------|----------------------------------------|---------------------------------------|
| Vertex Input          | VkVertexInputState                      | MTLVertexAttribute + MTLBuffer         | GPUVertexBufferLayout                 |
| Vertex Shader         | VkPipelineShaderStageCreateInfo          | MTLRenderPipelineState.vertexFunction  | GPUProgrammableStage (vertex)        |
| Primitive Assembly    | VkPipelineInputAssemblyStateCreateInfo    | MTLRenderPipelineDescriptor (topology) | GPUPrimitiveState.topology           |
| Rasterization         | VkPipelineRasterizationStateCreateInfo   | MTLRenderPipelineDescriptor (raster)  | GPUPrimitiveState.stripIndexFormat    |
| Fragment Shader       | VkPipelineShaderStageCreateInfo          | MTLRenderPipelineState.fragmentFunction| GPUProgrammableStage (fragment)      |
| Depth/Stencil         | VkPipelineDepthStencilStateCreateInfo    | MTLDepthStencilState                   | GPUDepthStencilState                  |
| Color Blend           | VkPipelineColorBlendStateCreateInfo      | MTLRenderPipelineColorAttachmentDescriptor| GPUBlendState                     |
| Render Pass           | VkRenderPassCreateInfo                   | MTLRenderPassDescriptor                | GPURenderPassDescriptor              |
| Command Submission     | VkSubmitInfo                            | MTLCommandBuffer.commit                | GPUQueue.submit                      |