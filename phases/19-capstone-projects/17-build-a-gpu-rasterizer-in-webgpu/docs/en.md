# Build a GPU Rasterizer in WebGPU

> The GPU pipeline is a fixed-function sequence of programmable stages.

**Type:** Build
**Languages:** WGSL, TypeScript
**Prerequisites:** Phase 19 lessons 01-16
**Time:** ~600 minutes

## Learning Objectives

- Write WGSL vertex and fragment shaders.
- Create a WebGPU render pipeline in TypeScript.
- Upload vertex data and issue draw calls.
- Understand clip space, interpolation, and framebuffer output.

## The Problem

Rasterization is still the dominant rendering technique for real-time graphics. Every game, every UI framework, every 3D application uses rasterization for interactive frame rates. Ray tracing is more physically accurate but too slow for 60fps rendering of complex scenes.

WebGPU is the modern web graphics API, replacing WebGL. It exposes the GPU pipeline as a sequence of programmable stages: vertex shader, rasterizer, fragment shader, output merger. Each stage has a clear contract with the next. The vertex shader transforms positions; the rasterizer converts triangles into fragments; the fragment shader computes colors.

Building a WebGPU rasterizer teaches you the GPU pipeline from the ground up: shader IO contracts, clip space and interpolation, pipeline state objects, and command encoding. These concepts transfer directly to Vulkan, Metal, and DirectX 12.

## The Concept

The GPU rasterization pipeline has four stages:

```
Vertex data (positions, colors)
        │
        ▼
┌───────────────┐
│ 1. Vertex      │  Transform positions to clip space
│  Shader        │  Output: clip-space position + varyings
└───────────────┘
        │
        ▼
┌───────────────┐
│ 2. Rasterizer  │  Convert triangles into fragments
│  (fixed-func)  │  Interpolate varyings across triangle
└───────────────┘
        │
        ▼
┌───────────────┐
│ 3. Fragment    │  Compute output color per fragment
│  Shader        │  Access textures, lighting, etc.
└───────────────┘
        │
        ▼
┌───────────────┐
│ 4. Output      │  Depth test, blending, write to framebuffer
│  Merger        │
└───────────────┘
```

Clip space coordinates: the vertex shader outputs positions in homogeneous clip space [-1, 1] for x, y, z. The rasterizer clips triangles to the view volume, then maps to screen coordinates.

## Build It

### Step 1: WGSL Shaders

```wgsl
// shader.wgsl — Vertex and fragment shaders

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
```

### Step 2: TypeScript Pipeline Setup

```typescript
// main.ts — WebGPU pipeline setup and rendering

async function initWebGPU(): Promise<{ device: GPUDevice; context: GPUCanvasContext }> {
    const adapter = await navigator.gpu.requestAdapter();
    if (!adapter) throw new Error("No GPU adapter found");

    const device = await adapter.requestDevice();
    const canvas = document.querySelector("canvas") as HTMLCanvasElement;
    const context = canvas.getContext("webgpu") as GPUCanvasContext;

    context.configure({
        device,
        format: navigator.gpu.getPreferredCanvasFormat(),
        alphaMode: "premultiplied",
    });

    return { device, context };
}

function createTrianglePipeline(device: GPUDevice, shaderModule: GPUShaderModule): GPURenderPipeline {
    return device.createRenderPipeline({
        layout: "auto",
        vertex: {
            module: shaderModule,
            entryPoint: "vs_main",
            buffers: [{
                arrayStride: 24, // 6 floats * 4 bytes (position + color)
                attributes: [
                    { shaderLocation: 0, offset: 0, format: "float32x3" },  // position
                    { shaderLocation: 1, offset: 12, format: "float32x3" }, // color
                ],
            }],
        },
        fragment: {
            module: shaderModule,
            entryPoint: "fs_main",
            targets: [{ format: navigator.gpu.getPreferredCanvasFormat() }],
        },
        primitive: {
            topology: "triangle-list",
        },
    });
}

function createVertexBuffer(device: GPUDevice): GPUBuffer {
    // Triangle vertices: position (x, y, z) + color (r, g, b)
    const vertices = new Float32Array([
        // Position          Color
         0.0,  0.5, 0.0,    1.0, 0.0, 0.0,  // Top: red
        -0.5, -0.5, 0.0,    0.0, 1.0, 0.0,  // Bottom-left: green
         0.5, -0.5, 0.0,    0.0, 0.0, 1.0,  // Bottom-right: blue
    ]);

    const buffer = device.createBuffer({
        size: vertices.byteLength,
        usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
    });

    device.queue.writeBuffer(buffer, 0, vertices);
    return buffer;
}

function render(
    device: GPUDevice,
    context: GPUCanvasContext,
    pipeline: GPURenderPipeline,
    vertexBuffer: GPUBuffer
) {
    const commandEncoder = device.createCommandEncoder();
    const textureView = context.getCurrentTexture().createView();

    const renderPass = commandEncoder.beginRenderPass({
        colorAttachments: [{
            view: textureView,
            clearValue: { r: 0.1, g: 0.1, b: 0.1, a: 1.0 },
            loadOp: "clear",
            storeOp: "store",
        }],
    });

    renderPass.setPipeline(pipeline);
    renderPass.setVertexBuffer(0, vertexBuffer);
    renderPass.draw(3); // 3 vertices = 1 triangle
    renderPass.end();

    device.queue.submit([commandEncoder.finish()]);
}

async function main() {
    const { device, context } = await initWebGPU();

    const shaderCode = await fetch("shader.wgsl").then(r => r.text());
    const shaderModule = device.createShaderModule({ code: shaderCode });

    const pipeline = createTrianglePipeline(device, shaderModule);
    const vertexBuffer = createVertexBuffer(device);

    render(device, context, pipeline, vertexBuffer);
    console.log("Triangle rendered!");
}

main();
```

### Step 3: Uniform Buffer for Transforms

```typescript
// Add a uniform buffer for model-view-projection matrix
function createUniformBuffer(device: GPUDevice): GPUBuffer {
    // Simple identity matrix (4x4)
    const mvp = new Float32Array([
        1, 0, 0, 0,
        0, 1, 0, 0,
        0, 0, 1, 0,
        0, 0, 0, 1,
    ]);

    const buffer = device.createBuffer({
        size: mvp.byteLength,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });

    device.queue.writeBuffer(buffer, 0, mvp);
    return buffer;
}

// WGSL shader with uniform
const shaderWithUniform = `
struct Uniforms {
    mvp: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.mvp * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
`;
```

### Step 4: Indexed Drawing

```typescript
function createIndexedBuffers(device: GPUDevice): { vertexBuffer: GPUBuffer; indexBuffer: GPUBuffer } {
    // Two triangles forming a square
    const vertices = new Float32Array([
        // Position          Color
        -0.5, -0.5, 0.0,    1.0, 0.0, 0.0,  // 0: bottom-left red
         0.5, -0.5, 0.0,    0.0, 1.0, 0.0,  // 1: bottom-right green
         0.5,  0.5, 0.0,    0.0, 0.0, 1.0,  // 2: top-right blue
        -0.5,  0.5, 0.0,    1.0, 1.0, 0.0,  // 3: top-left yellow
    ]);

    const indices = new Uint16Array([
        0, 1, 2,  // First triangle
        0, 2, 3,  // Second triangle
    ]);

    const vertexBuffer = device.createBuffer({
        size: vertices.byteLength,
        usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
    });
    device.queue.writeBuffer(vertexBuffer, 0, vertices);

    const indexBuffer = device.createBuffer({
        size: indices.byteLength,
        usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST,
    });
    device.queue.writeBuffer(indexBuffer, 0, indices);

    return { vertexBuffer, indexBuffer };
}

// Draw with indices
function renderIndexed(
    device: GPUDevice,
    context: GPUCanvasContext,
    pipeline: GPURenderPipeline,
    vertexBuffer: GPUBuffer,
    indexBuffer: GPUBuffer
) {
    const commandEncoder = device.createCommandEncoder();
    const textureView = context.getCurrentTexture().createView();

    const renderPass = commandEncoder.beginRenderPass({
        colorAttachments: [{
            view: textureView,
            clearValue: { r: 0.1, g: 0.1, b: 0.1, a: 1.0 },
            loadOp: "clear",
            storeOp: "store",
        }],
    });

    renderPass.setPipeline(pipeline);
    renderPass.setVertexBuffer(0, vertexBuffer);
    renderPass.setIndexBuffer(indexBuffer, "uint16");
    renderPass.drawIndexed(6); // 6 indices = 2 triangles
    renderPass.end();

    device.queue.submit([commandEncoder.finish()]);
}
```

## Use It

Modern engines layer material systems and frame graphs on top of the same primitive model:

- **wgpu (Rust)**: the Rust implementation of WebGPU. Used by Firefox (via wgpu-native), Bevy engine, and many Rust graphics projects. The API is nearly identical to the web WebGPU API.
- **Dawn (C++)**: Google's WebGPU implementation used in Chrome. The native API mirrors the web API with C++ types.
- **Three.js**: the most popular web 3D library. Under the hood, it uses WebGL (and increasingly WebGPU) with the same pipeline model: geometry -> vertex shader -> rasterization -> fragment shader.
- **Unity/Unreal**: professional game engines. Their rendering pipelines are built on the same concepts: pipeline state objects, command buffers, shader stages.

The key production lesson: **pipeline state objects are expensive to create, cheap to use**. Creating a pipeline compiles shaders and configures fixed-function state. Switching pipelines has overhead. Production engines minimize pipeline switches by sorting draw calls by pipeline state.

## Read the Source

- [WebGPU specification](https://www.w3.org/TR/webgpu/) — The W3C specification for WebGPU.
- [wgpu examples](https://github.com/gfx-rs/wgpu/tree/trunk/examples) — Rust WebGPU examples including triangle, texture, and compute shaders.
- [WebGPU samples](https://webgpu.github.io/webgpu-samples/) — Official WebGPU samples with TypeScript and WGSL.

## Ship It

- `code/shader.wgsl`: vertex and fragment shaders for colored triangle rendering.
- `code/main.ts`: WebGPU pipeline setup, vertex buffer creation, and draw call.
- `outputs/README.md`: screenshot of rendered triangle and note on shader inputs and pipeline layout.

## Exercises

1. **Easy** — Add indexed drawing for multiple triangles. Create a vertex buffer with 6 vertices and an index buffer with 12 indices (4 triangles). Draw a colored square made of two triangles.
2. **Medium** — Add uniform buffers for transforms. Create a uniform buffer with a 4x4 model-view-projection matrix. Pass it to the vertex shader and multiply positions by it. Demonstrate rotation by updating the matrix each frame.
3. **Hard** — Add a depth buffer and simple camera. Create a depth texture, enable depth testing in the pipeline, and render multiple triangles at different depths. Implement a simple perspective projection matrix.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Clip Space | "GPU coordinates" | The coordinate system output by the vertex shader. Positions in clip space [-1, 1] are visible; positions outside are clipped. The perspective divide (x/w, y/w, z/w) converts to normalized device coordinates. |
| Rasterization | "triangle fill" | The fixed-function stage that converts geometric triangles into fragments (pixel candidates). For each pixel covered by a triangle, the rasterizer interpolates vertex attributes (color, UV, normal). |
| Fragment | "pixel candidate" | A per-pixel data structure produced by the rasterizer. The fragment shader computes the final color. Fragments can be discarded (alpha test) or blended with the framebuffer. |
| Pipeline | "GPU state bundle" | A complete rendering configuration: vertex shader, fragment shader, blend state, depth state, vertex layout. Creating a pipeline compiles shaders; switching pipelines changes the rendering configuration. |
| Command Encoder | "draw recorder" | An object that records GPU commands (draw calls, compute dispatches, copies) into a command buffer. The command buffer is submitted to the GPU queue for execution. |

## Further Reading

- [WebGPU specification](https://www.w3.org/TR/webgpu/) — The W3C specification.
- [Learn WebGPU](https://eliemichel.github.io/LearnWebGPU/) — Step-by-step WebGPU tutorial.
- [wgpu](https://github.com/gfx-rs/wgpu) — Rust WebGPU implementation.
