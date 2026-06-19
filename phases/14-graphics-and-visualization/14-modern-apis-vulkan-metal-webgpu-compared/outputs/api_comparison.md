# API Comparison — Vulkan, Metal, WebGPU

A side-by-side reference of core concepts, with pseudocode snippets for each API.

## Initialization

| Step | Vulkan | Metal | WebGPU |
|------|--------|-------|--------|
| Get GPU | `vkCreateInstance` → `vkEnumeratePhysicalDevices` → pick one | `MTLCreateSystemDefaultDevice()` | `navigator.gpu.requestAdapter()` |
| Create device | `vkCreateDevice(physicalDevice, &createInfo, ...)` → specify queues, features, extensions | Same `MTLDevice` object; no separate logical device | `adapter.requestDevice()` → optional limits/features |
| Get queue | `vkGetDeviceQueue(device, familyIndex, queueIndex, &queue)` | `[device newCommandQueue]` | `device.queue` (auto-created) |

## Pipeline Creation

| Aspect | Vulkan | Metal | WebGPU |
|--------|--------|-------|--------|
| Object type | `VkPipeline` | `MTLRenderPipelineState` | `GPURenderPipeline` |
| Shader format | SPIR-V (compiled from GLSL/HLSL) | MSL (Metal Shading Language) | WGSL (WebGPU Shading Language) |
| State specification | ~15 separate `Vk*CreateInfo` structs in `VkGraphicsPipelineCreateInfo` | `MTLRenderPipelineDescriptor` properties | JavaScript object in `createRenderPipeline()` |
| Pipeline layout | Separate `VkPipelineLayout` object | Implicit from shader function signatures | `'auto'` or explicit `GPUPipelineLayout` |
| Creation call | `vkCreateGraphicsPipelines()` | `[device newRenderPipelineStateWithDescriptor:error:]` | `device.createRenderPipeline({...})` |

### Pipeline Pseudocode — Vulkan
```c
// Must specify: shader stages, vertex input, input assembly,
// viewport, rasterization, multisampling, depth/stencil, color blend,
// pipeline layout, render pass compatibility — ~15 structs
VkGraphicsPipelineCreateInfo info = {};
info.sType = VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO;
info.stageCount = 2;
info.pStages = shaderStages;
info.pVertexInputState = &vertexInputState;
info.pInputAssemblyState = &inputAssemblyState;
info.pViewportState = &viewportState;
info.pRasterizationState = &rasterizationState;
info.pMultisampleState = &multisampleState;
info.pDepthStencilState = &depthStencilState;
info.pColorBlendState = &colorBlendState;
info.layout = pipelineLayout;
info.renderPass = renderPass;
info.subpass = 0;
vkCreateGraphicsPipelines(device, VK_NULL_HANDLE, 1, &info, NULL, &pipeline);
```

### Pipeline Pseudocode — Metal
```objc
MTLRenderPipelineDescriptor *desc = [[MTLRenderPipelineDescriptor alloc] init];
desc.vertexFunction = [library newFunctionWithName:@"vertex_main"];
desc.fragmentFunction = [library newFunctionWithName:@"fragment_main"];
desc.colorAttachments[0].pixelFormat = MTLPixelFormatBGRA8Unorm;
desc.depthAttachmentPixelFormat = MTLPixelFormatDepth32Float;

NSError *error = nil;
id<MTLRenderPipelineState> pipeline =
    [device newRenderPipelineStateWithDescriptor:desc error:&error];
```

### Pipeline Pseudocode — WebGPU
```js
const pipeline = device.createRenderPipeline({
  layout: 'auto',
  vertex: {
    module: shaderModule,
    entryPoint: 'vertex_main',
    buffers: [{
      arrayStride: 12,
      attributes: [{ shaderLocation: 0, format: 'float32x3', offset: 0 }]
    }]
  },
  fragment: {
    module: shaderModule,
    entryPoint: 'fragment_main',
    targets: [{ format: 'bgra8unorm' }]
  },
  primitive: { topology: 'triangle-list' },
  depthStencil: { format: 'depth32float', depthWriteEnabled: true, depthCompare: 'less' }
});
```

## Command Recording & Submission

| Aspect | Vulkan | Metal | WebGPU |
|--------|--------|-------|--------|
| Command buffer | `VkCommandBuffer` | `MTLCommandBuffer` | `GPUCommandEncoder` |
| Render pass encoder | Inline via `vkCmdBeginRenderPass` | `MTLRenderCommandEncoder` | `GPURenderPassEncoder` |
| Begin recording | `vkBeginCommandBuffer()` | `[queue commandBuffer]` | `device.createCommandEncoder()` |
| Begin render pass | `vkCmdBeginRenderPass()` | `[cmdBuf renderCommandEncoderWithDescriptor:]` | `encoder.beginRenderPass({...})` |
| Bind pipeline | `vkCmdBindPipeline()` | `[encoder setRenderPipelineState:]` | `pass.setPipeline()` |
| Bind vertex buffer | `vkCmdBindVertexBuffers()` | `[encoder setVertexBuffer:offset:atIndex:]` | `pass.setVertexBuffer(slot, buffer)` |
| Draw | `vkCmdDraw(vertexCount, instanceCount, firstVertex, firstInstance)` | `[encoder drawPrimitives:type vertexStart:0 vertexCount:3]` | `pass.draw(vertexCount)` |
| End render pass | `vkCmdEndRenderPass()` | `[encoder endEncoding]` | `pass.end()` |
| End recording | `vkEndCommandBuffer()` | (implicit — buffers are ready after commit) | `encoder.finish()` |
| Submit | `vkQueueSubmit()` | `[cmdBuf commit]` | `device.queue.submit([encoder.finish()])` |

## Resource Binding

| Aspect | Vulkan | Metal | WebGPU |
|--------|--------|-------|--------|
| Binding unit | `VkDescriptorSet` | Per-encoder `setBuffer/Texture/SamplerState` | `GPUBindGroup` |
| Layout definition | `VkDescriptorSetLayout` | (implicit in shader) | `GPUBindGroupLayout` |
| Pool allocation | `VkDescriptorPool` → `vkAllocateDescriptorSets` | (no pool — just bind directly) | (no pool — `device.createBindGroup`) |
| Bind at draw time | `vkCmdBindDescriptorSets()` | `[encoder setVertexBuffer:offset:atIndex:]` etc. | `pass.setBindGroup(groupIndex, bindGroup)` |
| Update | `vkUpdateDescriptorSets()` | (bind new resources directly) | Create new `GPUBindGroup` |

### Binding Pseudocode — Vulkan
```c
// 1. Create descriptor set layout
VkDescriptorSetLayoutBinding bindings[] = {
    {0, VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER, 1, VK_SHADER_STAGE_VERTEX, NULL},
    {1, VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER, 1, VK_SHADER_STAGE_FRAGMENT, NULL},
};
VkDescriptorSetLayout layout;
vkCreateDescriptorSetLayout(device, &layoutInfo, NULL, &layout);

// 2. Allocate descriptor set from pool
VkDescriptorSet descSet;
vkAllocateDescriptorSets(device, &allocInfo, &descSet);

// 3. Write resources into descriptor set
VkWriteDescriptorSet writes[] = { /* buffer write, image write */ };
vkUpdateDescriptorSets(device, 2, writes, 0, NULL);

// 4. Bind at draw time
vkCmdBindDescriptorSets(cmdBuf, VK_PIPELINE_BIND_POINT_GRAPHICS,
                        pipelineLayout, 0, 1, &descSet, 0, NULL);
```

### Binding Pseudocode — Metal
```objc
// No descriptor sets — bind resources directly on the encoder
[encoder setVertexBuffer:uniformBuffer offset:0 atIndex:0];
[encoder setFragmentTexture:texture atIndex:0];
[encoder setFragmentSamplerState:sampler atIndex:0];
```

### Binding Pseudocode — WebGPU
```js
// 1. Create bind group layout (or use 'auto' layout from pipeline)
const bindGroupLayout = pipeline.getBindGroupLayout(0);

// 2. Create bind group with resources
const bindGroup = device.createBindGroup({
  layout: bindGroupLayout,
  entries: [
    { binding: 0, resource: { buffer: uniformBuffer } },
    { binding: 1, resource: texture.createView() },
  ]
});

// 3. Bind at draw time
pass.setBindGroup(0, bindGroup);
```

## Synchronization

| What | Vulkan | Metal | WebGPU |
|------|--------|-------|--------|
| CPU waits for GPU | `vkWaitForFences()` on `VkFence` | `[cmdBuf waitUntilCompleted]` or completion handler | `device.queue.onSubmittedWorkDone()` Promise |
| GPU signals GPU | `VkSemaphore` | `MTLEvent` (or `MTLFence` within encoder) | `GPUFence` (limited; most sync is implicit) |
| Memory barrier | `vkCmdPipelineBarrier()` | `[encoder memoryBarrierWithScope:]` | Implicit at render pass boundaries |
| Multiple queues | `VkQueue` objects; semaphores for cross-queue | Single `MTLCommandQueue`; prioritize with `MTLCommandQueue` priority | Single `device.queue`; compute passes can overlap |

## Memory Management

| Aspect | Vulkan | Metal | WebGPU |
|--------|--------|-------|--------|
| Buffer creation | `vkCreateBuffer()` then `vkAllocateMemory()` then `vkBindBufferMemory()` | `[device newBufferWithLength:options:]` — one call | `device.createBuffer({size, usage})` — one call |
| Memory types | Must query `VkPhysicalDeviceMemoryProperties`; choose VRAM vs system RAM vs host-visible | Choose `MTLResourceStorageMode`: `Shared`, `Managed`, `Private`, `Memoryless` | Not exposed; browser chooses |
| Mapping | `vkMapMemory()` / `vkUnmapMemory()` with explicit flush/invalidate | `[buffer contents]` for `Shared`; `didModifyRange:` for `Managed` | `buffer.mapAsync()` + `buffer.getMappedRange()` |
| Sub-allocation | Manual: bind multiple buffers to one `VkDeviceMemory` | Not exposed (Metal manages internally) | Not exposed |
| Deallocation | `vkDestroyBuffer()`, `vkFreeMemory()` | ARC (Automatic Reference Counting) | JavaScript GC + `buffer.destroy()` |

## Shader Languages

| Feature | SPIR-V (Vulkan) | MSL (Metal) | WGSL (WebGPU) |
|---------|-----------------|--------------|----------------|
| Syntax | Binary format; authored via GLSL/HLSL | C++-like | Rust/TypeScript-like |
| Entry point | Declared in `VkPipelineShaderStageCreateInfo` | `[[vertex]]`, `[[fragment]]` attributes | `@vertex fn`, `@fragment fn` |
| Resource binding | Layout qualifiers: `layout(set=0, binding=0)` | Argument buffer indexing: `[[buffer(0)]]` | `@group(0) @binding(0)` |
| Type system | Strong, explicit | C++ types + Metal vector types | Strong, with `vec<f32>`, `mat4x4<f32>` |
| Compilation | GLSL → SPIR-V offline (glslangValidator) | MSL compiled at pipeline creation | WGSL → Tint → SPIR-V/MSL/HLSL at runtime |

### Vertex Shader — Side by Side

**GLSL (for Vulkan):**
```glsl
#version 450
layout(location = 0) in vec3 pos;
layout(location = 0) out vec3 vColor;
void main() {
    gl_Position = vec4(pos, 1.0);
    vColor = pos;
}
```

**MSL (for Metal):**
```objc
#include <metal_stdlib>
using namespace metal;
vertex float4 vs(device const float3* positions [[buffer(0)]],
                 uint vid [[vertex_id]]) {
    return float4(positions[vid], 1.0);
}
```

**WGSL (for WebGPU):**
```wgsl
@vertex
fn vs(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(pos, 1.0);
}
```

## Validation & Error Handling

| API | Mechanism | When Active | Output |
|-----|-----------|-------------|--------|
| Vulkan | Validation layers (`VK_LAYER_KHRONOS_validation`) | Debug builds only (opt-in at instance creation) | Debug messenger callback; extremely detailed |
| Metal | Xcode Metal validation + `MTLCaptureScope` | Toggle in Xcode scheme settings | Xcode console + GPU frame capture |
| WebGPU | Error scopes (`pushErrorScope` / `popErrorScope`) + browser validation | Always on (built into spec compliance) | Promise resolution + `GPUError` objects |

## When to Use Which

| Scenario | Best API | Why |
|----------|----------|-----|
| Cross-platform desktop game (Windows, Linux, Steam Deck) | Vulkan | Only modern API that supports all three natively |
| Apple-first application (macOS, iOS) | Metal | First-class support on Apple platforms; best performance on Apple Silicon |
| Web application (browser graphics) | WebGPU | Only option for GPU compute/graphics in browsers |
| Game engine targeting all platforms | MoltenVK (Vulkan→Metal) or wgpu (WebGPU→native) | Abstraction layer that translates to each native API |
| Research / prototyping | WebGPU (via wgpu in Rust) | Fastest iteration cycle; safe defaults; runs on all platforms |
| High-performance compute with fine-grained sync | Vulkan or Metal | Need explicit barriers, multiple queues, manual memory placement |
| Mobile-first (excluding Apple) | Vulkan (Android) | Vulkan is the modern standard for Android; OpenGL ES is legacy |

## Quick Reference: Same Operation, Three APIs

### Create a Buffer
```
Vulkan:   vkCreateBuffer → vkGetBufferMemoryRequirements → vkAllocateMemory → vkBindBufferMemory
Metal:    [device newBufferWithBytes:length:options:]
WebGPU:   device.createBuffer({size, usage, mappedAtCreation})
```

### Create a Texture
```
Vulkan:   vkCreateImage → vkGetImageMemoryRequirements → vkAllocateMemory → vkBindImageMemory → vkCreateImageView
Metal:    [device newTextureWithDescriptor:]
WebGPU:   device.createTexture({size, format, usage})
```

### Copy Data to Buffer
```
Vulkan:   vkMapMemory → memcpy → vkFlushMappedMemoryRanges (for host-visible, non-coherent)
Metal:    [buffer contents] → memcpy (Shared mode); or [cmdBuffer copyFromBuffer:] (Private mode)
WebGPU:   device.queue.writeBuffer(buffer, offset, data)
```

### Draw a Triangle (after pipeline + buffer setup)
```
Vulkan:   vkCmdBindPipeline → vkCmdBindVertexBuffers → vkCmdBindDescriptorSets → vkCmdDraw
Metal:    [enc setRenderPipelineState:] → [enc setVertexBuffer:] → [enc drawPrimitives:]
WebGPU:  pass.setPipeline → pass.setVertexBuffer → pass.setBindGroup → pass.draw
```