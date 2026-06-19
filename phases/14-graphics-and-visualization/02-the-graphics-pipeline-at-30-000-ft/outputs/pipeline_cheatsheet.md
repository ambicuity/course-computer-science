# Graphics Pipeline Cheatsheet

A one-page reference mapping each pipeline stage to its GPU API equivalent.

## Pipeline Stages → API Mapping

```
 ┌──────────────────────┐  Vulkan                                   Metal                              WebGPU
 │ APPLICATION (CPU)    │  VkCommandBuffer                          MTLCommandBuffer                   GPUCommandBuffer
 │ Submit draw calls     │  vkCmdDraw*                               renderCommandEncoder               renderPassEncoder.draw*
 └──────────┬───────────┘
            ▼
 ┌──────────────────────┐  VkVertexInputState                      MTLVertexAttribute                 GPUVertexBufferLayout
 │ INPUT ASSEMBLY       │  + primitive topology                    + MTLBuffer binding                + primitive topology
 └──────────┬───────────┘
            ▼
 ┌──────────────────────┐  VkPipelineShaderStage                   MTLRenderPipeline                  GPUProgrammableStage
 │ VERTEX SHADER        │  .stage = VERTEX                         .vertexFunction                   .entryPoint on vertex
 │ (Programmable)       │  .module = vertModule                    stage = .vertex                    stage
 └──────────┬───────────┘
            ▼
 ┌──────────────────────┐  VkPipelineInputAssemblyState             MTLRenderPipeline                  GPUPrimitiveState
 │ PRIMITIVE ASSEMBLY   │  .topology = TRIANGLE_LIST               .inputPrimitiveTopology           .topology
 │ (Fixed)              │
 └──────────┬───────────┘
            ▼
 ┌──────────────────────┐  VkPipelineRasterizationState             MTLRenderPipeline                  GPUPrimitiveState
 │ CLIPPING + RASTER    │  .cullMode, .frontFace                   .cullMode, .winding               .cullMode, .frontFace
 │ (Fixed)              │  Perspective divide: automatic           Rasterization: automatic           Rasterization: automatic
 └──────────┬───────────┘
            ▼
 ┌──────────────────────┐  VkPipelineShaderStage                   MTLRenderPipeline                  GPUProgrammableStage
 │ FRAGMENT SHADER      │  .stage = FRAGMENT                      .fragmentFunction                 .entryPoint on fragment
 │ (Programmable)       │  .module = fragModule                    stage = .fragment                  stage
 └──────────┬───────────┘
            ▼
 ┌──────────────────────┐  VkPipelineDepthStencilState            MTLDepthStencilState              GPUDepStencilState
 │ DEPTH / STENCIL      │  .depthCompareOp = LESS                .depthCompareFunction            .depthCompare
 │ (Fixed, Configurable)│  .stencilTestEnable                     .stencilCompareFunction           .stencilFront/stencilBack
 └──────────┬───────────┘
            ▼
 ┌──────────────────────┐  VkPipelineColorBlendState              MTLRenderPipelineColorAttachment  GPUBlendState
 │ COLOR BLENDING       │  .srcColorBlend = SRC_ALPHA            .sourceRGBBlendFactor            .srcFactor
 │ (Fixed, Configurable)│  .dstColorBlend = ONE_MINUS_SRC_ALPHA  .destinationRGBBlendFactor       .dstFactor
 └──────────┬───────────┘
            ▼
 ┌──────────────────────┐  VkSwapchainKHR                          MTLDrawable                        GPUTexture (via canvas)
 │ FRAMEBUFFER          │  Double/triple buffered                  Double buffered                   context.getCurrentTexture()
 └──────────────────────┘
```

## Key Data at Each Stage

| Stage | Input | Output | Key Operation |
|-------|-------|--------|---------------|
| Input Assembly | Vertex + index buffers | Stream of primitives (triangles, lines, points) | Group vertices by topology |
| Vertex Shader | One vertex (pos, attrs) | gl_Position + varyings | MVP transform |
| Clipping | Clip-space primitives | Clipped primitives | Clip planes |
| Rasterization | Screen-space triangles | Fragments with interpolated attrs | Edge testing + barycentric |
| Fragment Shader | Interpolated attrs + uniforms + textures | RGBA color + depth | Lighting, texturing |
| Output Merge | Fragment color + depth | Final framebuffer pixel | Depth/stencil/blending |

## Transform Quick Reference

```
Object Space ──[Model]──► World Space ──[View]──► View Space ──[Projection]──► Clip Space

Clip Space (x,y,z,w) ──[÷w]──► NDC (x/w, y/w, z/w)     where NDC ∈ [-1,1]³ (OpenGL)
                                                            or NDC ∈ [-1,1]×[-1,1]×[0,1] (DirectX)

NDC ──[Viewport]──► Screen Space (pixels)
  sx = (NDCx + 1) / 2 × width + x_offset
  sy = (1 - NDCy) / 2 × height + y_offset     (Y-flipped for screen)
```

## Perspective-Correct Interpolation

```
  For attribute A at vertices with clip-space w values:
    A_interp = (λ₀·A₀/w₀ + λ₁·A₁/w₁ + λ₂·A₂/w₂) / (λ₀/w₀ + λ₁/w₁ + λ₂/w₂)

  Never interpolate attributes linearly in screen space for perspective projections.
```

## Blend Modes Quick Reference

```
  None:       dst = src
  Alpha:      dst = src.a × src + (1-src.a) × dst
  Additive:   dst = src + dst
  Multiply:   dst = src × dst
  Premult:    dst = src + (1-src.a) × dst     (when src already multiplied by alpha)
```

## Fixed-Function vs Programmable Quick Check

```
  Fixed-function (you configure, GPU executes):
    ✓ Input assembly / primitive topology
    ✓ Clipping
    ✓ Rasterization (coverage testing)
    ✓ Depth / stencil testing
    ✓ Blending
    ✓ Viewport / scissor transform

  Programmable (you write shader code):
    ✓ Vertex shader
    ✓ Fragment / pixel shader
    ○ Tessellation shaders (optional stage)
    ○ Geometry shader (optional stage)
    ○ Compute shader (separate pipeline)