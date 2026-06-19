// Build a GPU Rasterizer in WebGPU — TypeScript Pipeline Setup
// Run: npx vite (or similar bundler with TypeScript support)
// Requires: HTML page with <canvas> element, WebGPU-capable browser
//
// Architecture:
//   Vertex data → Vertex Shader → Rasterizer → Fragment Shader → Framebuffer
//
// Implements full WebGPU pipeline setup: adapter/device init, render pipeline
// creation with vertex buffer layout, uniform buffer for transforms, indexed
// drawing, and render pass execution.

// =============================================================================
// Step 1: WebGPU Initialization
// =============================================================================

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

// =============================================================================
// Step 2: Pipeline Creation
// =============================================================================

function createTrianglePipeline(device: GPUDevice, shaderModule: GPUShaderModule): GPURenderPipeline {
    return device.createRenderPipeline({
        layout: "auto",
        vertex: {
            module: shaderModule,
            entryPoint: "vs_main",
            buffers: [{
                arrayStride: 24, // 6 floats * 4 bytes (position + color)
                attributes: [
                    { shaderLocation: 0, offset: 0, format: "float32x3" as GPUVertexFormat },
                    { shaderLocation: 1, offset: 12, format: "float32x3" as GPUVertexFormat },
                ],
            }],
        },
        fragment: {
            module: shaderModule,
            entryPoint: "fs_main",
            targets: [{ format: navigator.gpu.getPreferredCanvasFormat() as GPUTextureFormat }],
        },
        primitive: { topology: "triangle-list" },
    });
}

// =============================================================================
// Step 3: Buffer Creation
// =============================================================================

function createVertexBuffer(device: GPUDevice): GPUBuffer {
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

function createUniformBuffer(device: GPUDevice): GPUBuffer {
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

function createIndexedBuffers(device: GPUDevice): { vertexBuffer: GPUBuffer; indexBuffer: GPUBuffer } {
    const vertices = new Float32Array([
        -0.5, -0.5, 0.0,    1.0, 0.0, 0.0,
         0.5, -0.5, 0.0,    0.0, 1.0, 0.0,
         0.5,  0.5, 0.0,    0.0, 0.0, 1.0,
        -0.5,  0.5, 0.0,    1.0, 1.0, 0.0,
    ]);

    const indices = new Uint16Array([0, 1, 2, 0, 2, 3]);

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

// =============================================================================
// Step 4: Render Functions
// =============================================================================

function render(
    device: GPUDevice, context: GPUCanvasContext,
    pipeline: GPURenderPipeline, vertexBuffer: GPUBuffer
) {
    const commandEncoder = device.createCommandEncoder();
    const textureView = context.getCurrentTexture().createView();

    const renderPass = commandEncoder.beginRenderPass({
        colorAttachments: [{
            view: textureView,
            clearValue: { r: 0.1, g: 0.1, b: 0.1, a: 1.0 },
            loadOp: "clear" as GPULoadOp,
            storeOp: "store" as GPUStoreOp,
        }],
    });

    renderPass.setPipeline(pipeline);
    renderPass.setVertexBuffer(0, vertexBuffer);
    renderPass.draw(3);
    renderPass.end();

    device.queue.submit([commandEncoder.finish()]);
}

function renderIndexed(
    device: GPUDevice, context: GPUCanvasContext,
    pipeline: GPURenderPipeline,
    vertexBuffer: GPUBuffer, indexBuffer: GPUBuffer
) {
    const commandEncoder = device.createCommandEncoder();
    const textureView = context.getCurrentTexture().createView();

    const renderPass = commandEncoder.beginRenderPass({
        colorAttachments: [{
            view: textureView,
            clearValue: { r: 0.1, g: 0.1, b: 0.1, a: 1.0 },
            loadOp: "clear" as GPULoadOp,
            storeOp: "store" as GPUStoreOp,
        }],
    });

    renderPass.setPipeline(pipeline);
    renderPass.setVertexBuffer(0, vertexBuffer);
    renderPass.setIndexBuffer(indexBuffer, "uint16");
    renderPass.drawIndexed(6);
    renderPass.end();

    device.queue.submit([commandEncoder.finish()]);
}

// =============================================================================
// Step 5: Main Entry Point
// =============================================================================

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
