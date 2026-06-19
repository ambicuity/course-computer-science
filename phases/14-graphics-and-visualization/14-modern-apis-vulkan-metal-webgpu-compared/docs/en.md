# Modern APIs — Vulkan, Metal, WebGPU Compared

> You give up convenience; you gain control.

**Type:** Learn
**Languages:** Markdown
**Prerequisites:** Phase 14 lessons 01–13
**Time:** ~60 minutes

## Learning Objectives

- Explain why OpenGL's implicit state model became a performance bottleneck and how modern APIs fix it.
- Map the same conceptual operation (create a pipeline, record commands, submit work) across Vulkan, Metal, and WebGPU.
- Identify the right API for a given target: cross-platform desktop, Apple ecosystem, or the web.
- Understand the trade-off between verbosity and control: why a Vulkan hello-triangle is ~1000 lines while WebGPU's is ~150.

## The Problem

Imagine you're a GPU driver. An application calls `glEnable(GL_BLEND)`, then `glDrawArrays()`, then changes the blend mode, then draws again. Every single draw call forces you to re-validate the entire pipeline state because the application might have mutated anything at any time. You can't pre-compile shaders because you don't know the full state at compile time. You can't parallelize command recording because state is implicit and global.

This is the OpenGL problem. It's also the DirectX 11 problem. The driver does enormous amounts of work at draw time — validating state, patching shaders, recompiling pipelines — because the API gives the application the illusion that state is free to change at any moment. The driver is the magician's assistant, frantically re-computing what the application actually wants every frame.

Modern APIs flip this contract. Instead of *implicit* state managed by the driver, you get *explicit* control. You describe the pipeline state upfront (blend mode, depth test, shaders, vertex format) as an immutable pipeline object. You record commands into a command buffer that the driver can validate and compile once, then replay cheaply. You manage memory yourself — no more hidden allocation spikes. You can record commands on multiple threads because state is local to each command buffer.

The cost: verbosity. A Vulkan hello-triangle is roughly 1000 lines. A Metal one is ~300. A WebGPU one is ~150. But what you get is predictable performance, multi-threaded command recording, and direct access to the hardware without a "smart" driver guessing your intentions.

## The Concept

### Explicit vs Implicit APIs

```
Implicit (OpenGL/DX11)          Explicit (Vulkan/Metal/WebGPU)
┌──────────────────────┐       ┌──────────────────────┐
│  Application          │       │  Application          │
│  ├─ glEnable(BLEND)   │       │  ├─ CreatePipeline   │
│  ├─ glDrawArrays()    │       │  │  (blend, depth,   │
│  ├─ glDisable(BLEND)  │       │  │   shaders, etc.)  │
│  ├─ glDrawArrays()    │       │  ├─ CreateCmdBuffer  │
│  └─ (implicit state) │       │  ├─ Record commands  │
└──────────┬────────────┘       │  └─ Submit(queue)    │
           │                    └──────────┬───────────┘
           ▼                               │
┌──────────────────────┐       ┌───────────▼────────────┐
│  Driver               │       │  Driver                │
│  Validate state       │       │  Validate once         │
│  Patch shaders        │       │  Compile pipeline      │
│  Re-validate state    │       │  Replay recorded cmds  │
│  Patch shaders again  │       │  (no per-draw cost)    │
└──────────────────────┘       └────────────────────────┘
```

The driver in the implicit model re-validates on every draw. The explicit model validates once when you create the pipeline, then replays your command buffers with near-zero CPU overhead.

### Shared Mental Model: The Rendering Pipeline

Despite different names, all three modern APIs share the same conceptual pipeline:

```
┌─────────────────────────────────────────────────────────────┐
│                    Modern GPU API Flow                       │
│                                                              │
│  1. Create Pipeline State                                    │
│     (shaders + blend + depth + vertex format — immutable)    │
│                                                              │
│  2. Record Commands                                          │
│     Begin Command Buffer/Encoder                             │
│     ├─ Begin Render Pass (specify attachments)               │
│     ├─ Bind Pipeline                                        │
│     ├─ Bind Resources (textures, buffers, uniforms)          │
│     ├─ Draw / DrawIndexed                                   │
│     └─ End Render Pass                                       │
│     End Command Buffer/Encoder                               │
│                                                              │
│  3. Submit to Queue                                          │
│     (GPU executes recorded commands)                         │
│                                                              │
│  4. Synchronize                                              │
│     (fence / event to know when GPU is done)                 │
└─────────────────────────────────────────────────────────────┘
```

### Core Concepts Across All Three APIs

| Concept | Vulkan | Metal | WebGPU |
|---------|--------|-------|--------|
| Connection to GPU | VkInstance | MTLDevice | GPUAdapter |
| Logical device | VkDevice | MTLDevice (same) | GPUDevice |
| Work queue | VkQueue | MTLCommandQueue | GPUQueue |
| Command recording | VkCommandBuffer | MTLCommandBuffer | GPUCommandEncoder |
| Pipeline state | VkPipeline | MTLRenderPipelineState | GPURenderPipeline |
| Render pass | VkRenderPass | MTLRenderPassDescriptor | GPURenderPassDescriptor |
| Resource binding | VkDescriptorSet | MTLBuffer + setVertexBytes | GPUBindGroup |
| Memory | VkDeviceMemory (manual) | MTLBuffer (managed) | GPUBuffer (managed) |
| Synchronization | VkFence | MTLEvent / completion handler | GPUFence |
| Shader language | SPIR-V | MSL (Metal Shading Language) | WGSL |

### What a Render Pass Is

A **render pass** defines the load/store behavior of attachments (color, depth). It answers: do we clear this attachment? Load the previous contents? Store the result? This matters because GPUs tile their rendering — a render pass tells the driver exactly when data needs to flow between tile memory and main memory, enabling bandwidth savings.

```
┌─────────────────────────────────────────────┐
│ Render Pass                                  │
│                                              │
│  Attachment 0 (color):  LoadOp=Clear (black) │
│  Attachment 1 (depth):  LoadOp=Clear (1.0)   │
│                                              │
│  ┌──────────────────────────────────────┐    │
│  │ Subpass: draws happen here           │    │
│  │  - input: previous pass outputs     │    │
│  │  - output: color + depth            │    │
│  └──────────────────────────────────────┘    │
│                                              │
│  StoreOp = Store (save color to texture)     │
│  StoreOp = DontCare (discard depth)          │
└─────────────────────────────────────────────┘
```

### What a Descriptor Set / Bind Group Is

Shaders need access to resources: uniform buffers (transform matrices), combined image-samplers (textures), storage buffers (compute data). In OpenGL, you bind these with `glUniform*` and `glBindTexture` — stateful, global, error-prone. Modern APIs group these bindings into **descriptor sets** (Vulkan) or **bind groups** (WebGPU) — immutable bundles of resource bindings that you create once and bind as a unit. Metal uses a simpler model: set bytes/buffers/textures on a command encoder directly.

```
OpenGL (per-draw, global state):      Modern APIs (pre-baked bundles):
┌──────────────────────┐             ┌──────────────────────────┐
│ glUniformMatrix4fv() │             │ BindGroup 0:             │
│ glUniform1i()        │             │   binding 0: UBO         │
│ glBindTexture()      │             │   binding 1: sampler     │
│ glActiveTexture()    │             │   binding 2: texture     │
│ ... (repeat per draw)│             │                          │
└──────────────────────┘             │ cmdBuf.setBindGroup(0)   │
                                     │ cmdBuf.setBindGroup(1)   │
                                     └──────────────────────────┘
```

## Build It

Since this is a Learn lesson with no executable code, we walk through the triangle setup process in all three APIs with side-by-side comparison.

### Step 1: Initialization

Every API needs to connect to a GPU. The verbosity differs dramatically.

**Vulkan** (~200 lines for init alone):
```
VkInstance instance;
VkInstanceCreateInfo createInfo = {};
createInfo.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
createInfo.enabledExtensionCount = ...;
createInfo.ppEnabledExtensionNames = ...;
vkCreateInstance(&createInfo, NULL, &instance);

VkPhysicalDevice physicalDevice;
// enumerate VkPhysicalDevices, pick one
vkEnumeratePhysicalDevices(instance, &deviceCount, &physicalDevices);

VkDevice device;
VkDeviceCreateInfo deviceCreateInfo = {};
// specify queues, extensions, features...
vkCreateDevice(physicalDevice, &deviceCreateInfo, NULL, &device);

VkQueue queue;
vkGetDeviceQueue(device, queueFamilyIndex, 0, &queue);
```

**Metal** (~10 lines):
```objc
id<MTLDevice> device = MTLCreateSystemDefaultDevice();
id<MTLCommandQueue> commandQueue = [device newCommandQueue];
```

**WebGPU** (~15 lines):
```js
const adapter = await navigator.gpu.requestAdapter();
const device = await adapter.requestDevice();
const queue = device.queue;
```

The pattern is the same: get a connection to the GPU, create a logical device, get a queue. Vulkan makes you specify every detail; Metal and WebGPU choose sensible defaults.

### Step 2: Creating the Pipeline

This is where the explicit API philosophy shows most clearly. You declare *all* state upfront.

**Vulkan**:
```
VkPipeline pipeline;
VkGraphicsPipelineCreateInfo pipelineInfo = {};
pipelineInfo.sType = VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO;
pipelineInfo.stageCount = 2;                     // vertex + fragment
pipelineInfo.pStages = shaderStages;             // VkPipelineShaderStageCreateInfo
pipelineInfo.pVertexInputState = &vertexInput;   // vertex format
pipelineInfo.pInputAssemblyState = &inputAssembly; // triangle list
pipelineInfo.pViewportState = &viewportState;    // viewport + scissor
pipelineInfo.pRasterizationState = &rasterizer;  // fill mode, cull mode
pipelineInfo.pMultisampleState = &multisampling; // sample count
pipelineInfo.pDepthStencilState = &depthStencil; // depth test
pipelineInfo.pColorBlendState = &colorBlend;     // blend state
pipelineInfo.layout = pipelineLayout;            // descriptor layout
pipelineInfo.renderPass = renderPass;            // compatible render pass
// Total: ~15 separate create-info structs
vkCreateGraphicsPipelines(device, VK_NULL_HANDLE, 1,
                          &pipelineInfo, NULL, &pipeline);
```

**Metal**:
```objc
MTLRenderPipelineDescriptor *desc = [[MTLRenderPipelineDescriptor alloc] init];
desc.vertexFunction = vertexFunction;
desc.fragmentFunction = fragmentFunction;
desc.colorAttachments[0].pixelFormat = MTLPixelFormatBGRA8Unorm;

id<MTLRenderPipelineState> pipeline =
    [device newRenderPipelineStateWithDescriptor:desc error:nil];
```

**WebGPU**:
```js
const pipeline = device.createRenderPipeline({
  vertex: {
    module: shaderModule,
    entryPoint: 'vs_main',
    buffers: [{ arrayStride: 12, attributes: [...] }]
  },
  fragment: {
    module: shaderModule,
    entryPoint: 'fs_main',
    targets: [{ format: 'bgra8unorm' }]
  },
  primitive: { topology: 'triangle-list' },
  layout: 'auto'
});
```

### Step 3: Recording and Submitting Commands

The command buffer is a serialized list of GPU commands recorded on the CPU, then submitted to a queue for execution on the GPU.

**Vulkan**:
```
VkCommandBuffer cmdBuf;
vkAllocateCommandBuffers(device, &cmdBufAllocInfo, &cmdBuf);
vkBeginCommandBuffer(cmdBuf, &beginInfo);

VkRenderPassBeginInfo rpInfo = {};
// specify render pass, framebuffer, clear values...

vkCmdBeginRenderPass(cmdBuf, &rpInfo, VK_SUBPASS_CONTENTS_INLINE);
vkCmdBindPipeline(cmdBuf, VK_PIPELINE_BIND_POINT_GRAPHICS, pipeline);
vkCmdBindVertexBuffers(cmdBuf, 0, 1, &vertexBuffer, &offset);
vkCmdDraw(cmdBuf, 3, 1, 0, 0);
vkCmdEndRenderPass(cmdBuf);

vkEndCommandBuffer(cmdBuf);

VkSubmitInfo submitInfo = {};
submitInfo.commandBufferCount = 1;
submitInfo.pCommandBuffers = &cmdBuf;
vkQueueSubmit(queue, 1, &submitInfo, fence);
```

**Metal**:
```objc
id<MTLCommandBuffer> cmdBuf = [commandQueue commandBuffer];
id<MTLRenderCommandEncoder> encoder =
    [cmdBuf renderCommandEncoderWithDescriptor:renderPassDesc];

[encoder setRenderPipelineState:pipeline];
[encoder setVertexBuffer:vertexBuffer offset:0 atIndex:0];
[encoder drawPrimitives:MTLPrimitiveTypeTriangle
            vertexStart:0 vertexCount:3];
[encoder endEncoding];

[cmdBuf commit];
```

**WebGPU**:
```js
const encoder = device.createCommandEncoder();
const pass = encoder.beginRenderPass({
  colorAttachments: [{
    view: context.getCurrentTexture().createView(),
    loadOp: 'clear',
    storeOp: 'store',
    clearValue: [0, 0, 0, 1]
  }]
});

pass.setPipeline(pipeline);
pass.setVertexBuffer(0, vertexBuffer);
pass.draw(3);
pass.end();

device.queue.submit([encoder.finish()]);
```

### Step 4: Synchronization — Knowing When the GPU Is Done

```
┌──────────────────────────────────────────────────────────────┐
│  CPU Timeline                    GPU Timeline                 │
│  ─────────────                   ─────────────                │
│       │                               │                       │
│  Record cmds ──► Submit to queue ──►  │                       │
│       │                          Execute cmds                 │
│       │                               │                       │
│  Wait on fence ──► ◄────────── Signal fence                  │
│       │                               │                       │
│  Read back results               Results ready               │
└──────────────────────────────────────────────────────────────┘
```

**Vulkan**: `vkQueueSubmit` takes a `VkFence`. CPU calls `vkWaitForFences()` to block until GPU signals.
**Metal**: `[cmdBuf addCompletedHandler:^(id<MTLCommandBuffer> buf) { ... }]` — callback when done. Or `MTLEvent` for fine-grained GPU-to-GPU sync.
**WebGPU**: `device.queue.onSubmittedWorkDone()` returns a Promise. `GPUFence` for GPU-to-GPU synchronization (still evolving in the spec).

### The Verbosity Spectrum

```
Shorter / Higher-level                    Longer / Lower-level

    WebGPU ──────── Metal ──────── DX12 ──────── Vulkan
     150 lines        300 lines      ~500 lines     ~1000 lines

     - Managed        - Partial        - Partial       - Manual
       memory            memory           memory          memory
     - Validation      - Metal           - Debug          - Layers
       built in          validation       layer            (optional)
     - Safe by         - ARC for         - COM            - Raw C
       design             objects                          handle API
```

## Use It

Which real engines use which API, and why?

| Engine | Primary API | Cross-platform Strategy |
|--------|------------|------------------------|
| Unreal Engine 5 | DX12 / Vulkan | Uses both; Metal via MoltenVK on macOS |
| Unity | DX11/DX12 / Vulkan / Metal | Targets all; Metal is first-class on Apple |
| Bevy | wgpu (WebGPU) | Targets WebGPU natively; wgpu translates to Vulkan/Metal/DX12 |
| wgpu (library) | WebGPU | A Rust implementation of the WebGPU API that translates to native backends |
| Godot 4 | Vulkan | Uses Vulkan cluster renderer; Metal via MoltenVK |

**wgpu** is the most interesting case: it exposes the WebGPU API to Rust applications, then translates to Vulkan, Metal, or DX12 on the backend. This means application code writes against one API (WebGPU) but runs on all native platforms with near-native performance. It's the same strategy MoltenVK uses for Vulkan-on-Metal, but starting from WebGPU instead.

### Validation and Debugging

**Vulkan** uses *validation layers* — optional installable callback chains that check every API call for correctness. You enable them in debug builds; they disappear in release. This is why Vulkan can be both the most verbose and the most debuggable API.

**Metal** uses the `MTLCaptureScope` and Xcode GPU debugger. Metal validation can be toggled in Xcode's scheme settings. It checks state consistency and resource usage.

**WebGPU** uses *error scopes* — structured try/catch for GPU operations:

```js
device.pushErrorScope('validation');
// ... GPU operations ...
const error = await device.popErrorScope();
if (error) console.error(error.message);
```

WebGPU also validates by default at the specification level — the browser enforces correct usage before commands ever reach the GPU.

## Read the Source

- [Vulkan Specification](https://registry.khronos.org/vulkan/specs/1.3/html/) — The definitive reference. Search for `VkGraphicsPipelineCreateInfo` to see just how much state a single pipeline object captures.
- [Metal Programming Guide](https://developer.apple.com/metal/Metal-Shading-Language-Specification.pdf) — Apple's reference. Compare MTLRenderPipelineDescriptor against VkGraphicsPipelineCreateInfo to see how Metal defaults reduce verbosity.
- [WebGPU Specification](https://www.w3.org/TR/webgpu/) — The W3C spec. Read Section 3 (Core Concepts) for the design rationale.
- [wgpu source](https://github.com/gfx-rs/wgpu) — See how each WebGPU concept maps to Vulkan/Metal/DX12 in `wgpu-hal/src/`.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`api_comparison.md`** — A detailed side-by-side reference card of Vulkan, Metal, and WebGPU concepts with pseudocode snippets for each.

## Exercises

1. **Easy** — Write a comparison table of how each API handles vertex buffer creation. What's manual in Vulkan that's automatic in WebGPU?
2. **Medium** — Pick one operation (creating a texture, setting up a render pass, or binding shader resources) and write the pseudocode for all three APIs side by side. Note where each API makes a different trade-off.
3. **Hard** — Design a thin abstraction layer that maps all three APIs to a common interface. What concepts map cleanly? What concepts resist unification? (Hint: memory allocation and synchronization are the hardest.)

## Key Terms

| Term | What people say | What it actually means |
|------|-----------------|------------------------|
| Command buffer | "A list of GPU commands" | A recorded, immutable sequence of GPU operations that can be submitted to a queue and replayed with near-zero CPU overhead |
| Render pass | "A draw-call group" | A structured scope that declares load/store behavior for attachments, enabling tile-based rendering optimizations |
| Pipeline state object | "The shader setup" | An immutable object capturing *all* GPU state (shaders, blend, depth, rasterizer, vertex format) — created once, bound cheaply |
| Descriptor set / Bind group | "A texture binding" | A pre-validated, immutable bundle of resource bindings (buffers, textures, samplers) that you bind as a unit rather than one at a time |
| Validation layer | "A debug mode" | An installable callback chain that intercepts API calls to check for misuse — optional at runtime, zero cost in release builds |
| Explicit API | "Hard to use" | An API that makes you state your intent upfront so the driver can validate and compile once, rather than re-validating on every draw call |

## Further Reading

- [GPU Zen 2](https://www.kickstarter.com/projects/802824703/gpu-zen-2-advanced-rendering-techniques) — Chapter on modern API practical usage
- [Vulkan Tutorial](https://vulkan-tutorial.com/) — Walk through a full Vulkan hello-triangle (~1000 lines)
- [Metal Best Practices Guide](https://developer.apple.com/library/archive/documentation/3DDrawing/Conceptual/MTLBestPracticesGuide/) — Apple's guide to getting the most from Metal
- [WebGPU Fundamentals](https://webgpufundamentals.org/) — Practical WebGPU tutorials
- [Life of a Triangle](https://developer.nvidia.com/content/life-triangle-nvidias-logical-pipeline) — NVIDIA's deep dive on what the driver does between your API call and pixels on screen