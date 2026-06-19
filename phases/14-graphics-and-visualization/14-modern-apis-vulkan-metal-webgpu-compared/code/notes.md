# Notes — Modern APIs: Vulkan, Metal, WebGPU Compared

## Concept Mapping Table

| Concept | Vulkan | Metal | WebGPU |
|---------|--------|-------|--------|
| GPU connection | VkInstance | MTLCreateSystemDefaultDevice() | navigator.gpu.requestAdapter() |
| Logical device | VkDevice | MTLDevice (same object) | adapter.requestDevice() |
| Work queue | VkQueue | MTLCommandQueue | GPUQueue (device.queue) |
| Command buffer | VkCommandBuffer | MTLCommandBuffer | GPUCommandEncoder |
| Render pass encoder | (inline in cmd buf) | MTLRenderCommandEncoder | GPURenderPassEncoder |
| Compute encoder | VkCommandBuffer (dispatch) | MTLComputeCommandEncoder | GPUComputePassEncoder |
| Pipeline (graphics) | VkPipeline | MTLRenderPipelineState | GPURenderPipeline |
| Pipeline (compute) | VkPipeline | MTLComputePipelineState | GPUComputePipeline |
| Pipeline layout | VkPipelineLayout | (implicit via function) | GPUPipelineLayout |
| Descriptor set layout | VkDescriptorSetLayout | (implicit in Metal) | GPUBindGroupLayout |
| Descriptor set | VkDescriptorSet | (setBuffer/Texture calls) | GPUBindGroup |
| Descriptor pool | VkDescriptorPool | (N/A — no pooling) | (N/A — automatic) |
| Vertex buffer | VkBuffer + VkDeviceMemory | MTLBuffer | GPUBuffer |
| Texture | VkImage + VkDeviceMemory | MTLTexture | GPUTexture |
| Framebuffer | VkFramebuffer | (implicit via descriptor) | (implicit via texture view) |
| Render pass | VkRenderPass | MTLRenderPassDescriptor | GPURenderPassDescriptor |
| Fence (GPU→CPU) | VkFence | CmdBuf.completionHandler | device.queue.onSubmittedWorkDone() |
| Semaphore (GPU→GPU) | VkSemaphore | MTLEvent | GPUFence (evolving) |
| Barrier (memory) | vkCmdPipelineBarrier | [encoder memoryBarrier...] | encoder.insertionBarrier? |
| Shader language | SPIR-V | MSL (Metal Shading Language) | WGSL |
| Shader module | VkShaderModule | MTLFunction | GPUShaderModule |

## Triangle Setup Pseudocode — Side by Side

### Vulkan
```
// 1. Create instance + device + queue (~80 lines)
vkCreateInstance(...)
vkEnumeratePhysicalDevices(...)
vkCreateDevice(physicalDevice, ...)
vkGetDeviceQueue(device, ...)

// 2. Create shader modules
VkShaderModule vertModule, fragModule;
vkCreateShaderModule(device, &vertInfo, NULL, &vertModule);
vkCreateShaderModule(device, &fragInfo, NULL, &fragModule);

// 3. Create pipeline layout (descriptor set layouts)
VkPipelineLayout pipelineLayout;
vkCreatePipelineLayout(device, &layoutInfo, NULL, &pipelineLayout);

// 4. Create render pass
VkRenderPass renderPass;
vkCreateRenderPass(device, &rpInfo, NULL, &renderPass);

// 5. Create graphics pipeline (~50 lines of create-info structs)
VkPipeline pipeline;
vkCreateGraphicsPipelines(device, VK_NULL_HANDLE, 1,
    &pipelineInfo, NULL, &pipeline);

// 6. Create framebuffer
VkFramebuffer framebuffer;
vkCreateFramebuffer(device, &fbInfo, NULL, &framebuffer);

// 7. Allocate and record command buffer
VkCommandBuffer cmdBuf;
vkAllocateCommandBuffers(device, &allocInfo, &cmdBuf);
vkBeginCommandBuffer(cmdBuf, &beginInfo);
  vkCmdBeginRenderPass(cmdBuf, &rpBeginInfo, ...);
  vkCmdBindPipeline(cmdBuf, VK_PIPELINE_BIND_POINT_GRAPHICS, pipeline);
  vkCmdBindVertexBuffers(cmdBuf, 0, 1, &vertexBuf, &offset);
  vkCmdDraw(cmdBuf, 3, 1, 0, 0);
  vkCmdEndRenderPass(cmdBuf);
vkEndCommandBuffer(cmdBuf);

// 8. Submit and wait
VkFence fence;
vkCreateFence(device, &fenceInfo, NULL, &fence);
vkQueueSubmit(queue, 1, &submitInfo, fence);
vkWaitForFences(device, 1, &fence, VK_TRUE, UINT64_MAX);
```

### Metal
```
// 1. Create device + queue
id<MTLDevice> device = MTLCreateSystemDefaultDevice();
id<MTLCommandQueue> queue = [device newCommandQueue];

// 2. Load shaders from library
id<MTLLibrary> library = [device newDefaultLibrary];
id<MTLFunction> vertFn = [library newFunctionWithName:@"vs"];
id<MTLFunction> fragFn = [library newFunctionWithName:@"fs"];

// 3. Create pipeline
MTLRenderPipelineDescriptor *desc = [[MTLRenderPipelineDescriptor alloc] init];
desc.vertexFunction = vertFn;
desc.fragmentFunction = fragFn;
desc.colorAttachments[0].pixelFormat = MTLPixelFormatBGRA8Unorm;
id<MTLRenderPipelineState> pipeline =
    [device newRenderPipelineStateWithDescriptor:desc error:nil];

// 4. Create vertex buffer
id<MTLBuffer> vertexBuf = [device newBufferWithBytes:vertices
    length:sizeof(vertices) options:MTLResourceStorageModeShared];

// 5. Record and submit
id<MTLCommandBuffer> cmdBuf = [queue commandBuffer];
MTLRenderPassDescriptor *rpDesc = [[MTLRenderPassDescriptor alloc] init];
rpDesc.colorAttachments[0].texture = drawable.texture;
rpDesc.colorAttachments[0].loadAction = MTLLoadActionClear;
rpDesc.colorAttachments[0].storeAction = MTLStoreActionStore;

id<MTLRenderCommandEncoder> enc =
    [cmdBuf renderCommandEncoderWithDescriptor:rpDesc];
[enc setRenderPipelineState:pipeline];
[enc setVertexBuffer:vertexBuf offset:0 atIndex:0];
[enc drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:3];
[enc endEncoding];
[cmdBuf commit];
```

### WebGPU
```
// 1. Get adapter + device
const adapter = await navigator.gpu.requestAdapter();
const device = await adapter.requestDevice();

// 2. Create shader module
const shaderModule = device.createShaderModule({ code: wgslCode });

// 3. Create pipeline
const pipeline = device.createRenderPipeline({
  vertex:   { module: shaderModule, entryPoint: 'vs' },
  fragment: { module: shaderModule, entryPoint: 'fs',
              targets: [{ format: 'bgra8unorm' }] },
  primitive: { topology: 'triangle-list' },
  layout: 'auto'
});

// 4. Create vertex buffer
const vertexBuf = device.createBuffer({
  size: vertices.byteLength,
  usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
});
device.queue.writeBuffer(vertexBuf, 0, vertices);

// 5. Record and submit
const encoder = device.createCommandEncoder();
const pass = encoder.beginRenderPass({
  colorAttachments: [{
    view: context.getCurrentTexture().createView(),
    loadOp: 'clear', storeOp: 'store',
    clearValue: [0, 0, 0, 1]
  }]
});
pass.setPipeline(pipeline);
pass.setVertexBuffer(0, vertexBuf);
pass.draw(3);
pass.end();

device.queue.submit([encoder.finish()]);
```

## Synchronization Model Comparison

| Sync Type | Vulkan | Metal | WebGPU |
|-----------|--------|-------|--------|
| GPU → CPU | VkFence + vkWaitForFences() | CommandBuffer.completionHandler | device.queue.onSubmittedWorkDone() |
| GPU → GPU | VkSemaphore | MTLEvent (MTLFence for within-encoder) | GPUFence (limited; implicit barriers) |
| Memory barrier | vkCmdPipelineBarrier() | [encoder memoryBarrierWithScope:] | (implicit at pass boundaries) |
| Host visibility | vkFlushMappedMemoryRanges | (Managed buffers auto-flush; Shared buffers always visible) | (All buffers are host-visible via mapAsync) |

Key insight: Vulkan makes every synchronization explicit. Metal and WebGPU insert implicit barriers at render pass boundaries. If you need finer control (compute → fragment data dependency), Vulkan requires an explicit pipeline barrier; Metal requires explicit memory barrier calls; WebGPU handles it implicitly in most cases.

## Memory Management Philosophy

```
Vulkan (Manual)                    Metal (Managed)              WebGPU (Automatic)
─────────────────────────────────  ──────────────────────────  ─────────────────────────────
vkCreateBuffer()                   [device newBufferWith...]    device.createBuffer()
vkGetBufferMemoryRequirements()     (MTLBuffer owns its         (GPUBuffer is allocation-
vkAllocateMemory()                   memory; you choose          opaque; you specify size
vkBindBufferMemory()                 storage mode:               and usage flags)
                                      Shared, Managed,
                                      Private, Memoryless)
vkMapMemory()                      [buffer contents]           buffer.mapAsync()
vkFlushMappedMemoryRanges()          (Managed: auto-flush)

Total control.           Partial control.            No control (by design).
You manage alignment,    Metal chooses heap;         Browser manages everything.
fragmentation, and      you choose cache mode.      Buffer stays alive while
coherence.                                           referenced.
```

Vulkan's model is necessary for maximum performance on heterogeneous memory architectures (discrete GPU VRAM vs system RAM). Metal gives you enough control to optimize (Private for GPU-only data, Shared for CPU-GPU data) without exposing raw heaps. WebGPU abstracts it all away — you say what you want and the browser handles placement.

## Validation Comparison

- **Vulkan**: Install validation layers at instance creation. They check every call, report misaligned offsets, missing barriers, and invalid state. Zero cost in release (layers not loaded). Extremely thorough but can be overwhelming.
- **Metal**: Xcode's GPU debugger + Metal validation toggle in scheme settings. Catches API misuse, resource conflicts. Less granular than Vulkan layers but well-integrated into the development workflow.
- **WebGPU**: Error scopes (`pushErrorScope` / `popErrorScope`) + browser console. The spec mandates validation — invalid operations generate errors rather than undefined behavior. The safest API by design.