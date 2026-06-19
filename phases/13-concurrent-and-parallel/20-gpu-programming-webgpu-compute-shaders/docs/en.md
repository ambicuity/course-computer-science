# GPU Programming — WebGPU / Compute Shaders

> WebGPU is the modern cross-platform GPU API that replaces the legacy WebGL. WGSL (WebGPU Shading Language) compiles to platform-native shader IRs. Compute shaders unlock general-purpose GPU programming in the browser and in native applications via `wgpu` — no CUDA- or vendor-lock-in required.

**Type:** Build
**Languages:** WGSL, TypeScript
**Prerequisites:** Phase 13 lessons 01–19
**Time:** ~75 minutes

## Learning Objectives

- Initialize a WebGPU adapter, device, and queue from JavaScript/TypeScript.
- Write WGSL compute shaders that operate on storage buffers.
- Build a complete compute pipeline: shader module → pipeline layout → bind group → dispatch.
- Implement vector addition and matrix multiplication as compute shaders.
- Use workgroup shared memory (`workgroup` address space) to accelerate matrix multiplication.
- Dispatch compute shaders with appropriate workgroup counts and measure GPU vs CPU performance.
- Compare WebGPU/WGSL with CUDA in terms of abstraction, portability, and mental model.

## The Problem

Lesson 19 introduced CUDA — NVIDIA's proprietary GPU programming model. CUDA is powerful but locks you into NVIDIA hardware, requires a full toolchain (nvcc), and cannot run in the browser. Modern GPU programming needs a **cross-platform**, **cross-vendor** API that works everywhere: Windows (D3D12), macOS (Metal), Linux (Vulkan), and the web browser.

WebGPU solves this. It is the spiritual successor to WebGL, designed by the W3M GPU for the Web Community Group with input from Apple, Google, Mozilla, Microsoft, and Intel. Its shading language, WGSL, compiles to SPIR-V (Vulkan), MSL (Metal), and DXIL (D3D12) — the native IR of each platform.

Concretely: you cannot ship a CUDA kernel to a web browser. You cannot run CUDA on an Apple Silicon Mac without Rosetta. You cannot use CUDA from a Progressive Web App. WebGPU removes all three restrictions.

The core mental model transfers directly from CUDA:
- **Device** ↔ GPU (like `cudaSetDevice`)
- **Queue** ↔ command stream (like `cudaMemcpyAsync` + kernel launches)
- **Compute shader** ↔ CUDA kernel (`__global__` function)
- **Workgroup** ↔ thread block (blockIdx/threadIdx → `@builtin(workgroup_id)` / `@builtin(local_invocation_id)`)
- **Workgroup shared memory** ↔ `__shared__`
- **Storage buffer** ↔ `cudaMalloc` / device pointers

If you completed Lesson 19, you already understand the GPU execution model. WebGPU just exposes it through a different (more verbose) API with explicit resource management.

## The Concept

### WebGPU Architecture

```
┌─────────────────────────────────────────┐
│              Application                 │
├─────────────────────────────────────────┤
│  WebGPU API (JS/TS or native wgpu)      │
├─────────────────┬───────────────────────┤
│  Adapter         │   Device              │
│  (physical GPU)  │   (logical handle)    │
├─────────────────┴───────────────────────┤
│  Queue → CommandEncoder → PassEncoder   │
│         → RenderBundle / ComputePass     │
├─────────────────────────────────────────┤
│  BindGroup + BindGroupLayout            │
│  (resources bound to shader)            │
├─────────────────────────────────────────┤
│  PipelineLayout → ComputePipeline       │
│  (compiled shader module + bindings)    │
└─────────────────────────────────────────┘
```

**Adapter** — Represents a physical GPU (or software fallback). Call `navigator.gpu.requestAdapter()` to get one. Multiple adapters (integrated + discrete GPU) may exist.

**Device** — A logical connection to the adapter. All GPU resources (buffers, textures, pipelines) are created from the device. Call `adapter.requestDevice()`.

**Queue** — The single queue (per device) into which you submit encoded commands. Call `device.queue` to get it.

**CommandEncoder** — Builds a batch of GPU commands (copies, render passes, compute passes). You encode work into it, then `finish()` to produce a `CommandBuffer`.

**ComputePassEncoder** — A section of a command encoder dedicated to compute work. You set the pipeline, bind resources, and dispatch workgroups.

**BindGroup** — The WebGPU analog of a descriptor set / resource table. It binds buffers, textures, and samplers to specific binding points in the shader. A `BindGroupLayout` defines the template; a `BindGroup` fills in the actual resources.

### WGSL Basics

WGSL (WebGPU Shading Language) is a typed, imperative shading language. Key properties:

- **Address spaces:** `function` (local), `private` (per-invocation static), `workgroup` (shared within a workgroup), `storage` (buffer visible to all dispatches), `uniform` (read-only constants).
- **Built-in values:** `@builtin(global_invocation_id)` — the global (x, y, z) ID of this invocation, equivalent to CUDA's `blockIdx * blockDim + threadIdx`.
- **Binding syntax:** `@group(0) @binding(0) var<storage, read_write> buffer: array<f32>;`
- **Workgroup size:** `@workgroup_size(64, 1, 1)` on the entry point function.

```
@group(0) @binding(0) var<storage, read>   a: array<f32>;
@group(0) @binding(1) var<storage, read>   b: array<f32>;
@group(0) @binding(2) var<storage, read_write> c: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3u) {
    let idx = id.x;
    if (idx >= arrayLength(&a)) { return; }
    c[idx] = a[idx] + b[idx];
}
```

Each invocation gets a unique `global_invocation_id`. The total number of invocations is `workgroup_size * dispatch_count`. In the example: `256 * dispatchCountX`.

### Workgroups and Occupancy

A **workgroup** is a collection of invocations that execute together and share `workgroup` memory. WebGPU guarantees that all invocations within a workgroup execute concurrently (though not necessarily in lockstep). Synchronization within a workgroup uses `workgroupBarrier()`.

**Occupancy** — On modern GPUs, the scheduler keeps many workgroups in flight to hide memory latency. The optimal workgroup size depends on the GPU architecture, but 64–256 invocations per workgroup is a common sweet spot (balances register pressure, shared memory usage, and scheduler flexibility).

### Matrix Multiplication with Shared Memory

Matrix multiply `C = A × B` where all are N×N. The naive approach reads A[i][k] and B[k][j] for every (i, j) — O(N³) memory reads. With workgroup shared memory, a workgroup loads a tile of A (M×K) and a tile of B (K×N) into fast on-chip memory, then the N×M invocations in that workgroup each compute one element of C using the shared tile. This reduces global memory traffic from O(N³) to O(N³ / tile_size).

```
┌──────────┐   ┌──────────┐   ┌──────────┐
│ A tile   │   │ B tile   │   │ C tile   │
│ (M×K)    │ × │ (K×N)    │ = │ (M×N)    │
└──────────┘   └──────────┘   └──────────┘
   workgroup      workgroup      all invocations
   loads tile     loads tile     accumulate partial
   into shared    into shared    products in registers
```

Each invocation accumulates one element of C across the K dimension. A `workgroupBarrier()` ensures all loads complete before computation starts.

## Build It

> **Note:** This lesson builds a browser-based WebGPU demo. Open `index.html` in a browser that supports WebGPU (Chrome 113+, Edge 113+, or recent Firefox Nightly). The code in `code/main.ts` and `code/main.wgsl` is standalone. All files ship in this directory.

### Step 1: Set Up the WebGPU Adapter, Device, and Queue

The entry point is `navigator.gpu`. If it does not exist, the browser does not support WebGPU.

**`main.ts` — initialization:**

```typescript
async function initWebGPU(): Promise<{
  adapter: GPUAdapter;
  device: GPUDevice;
  queue: GPUQueue;
}> {
  if (!navigator.gpu) {
    throw new Error("WebGPU not supported in this browser");
  }

  const adapter: GPUAdapter = await navigator.gpu.requestAdapter();
  if (!adapter) {
    throw new Error("No GPU adapter found");
  }

  const device: GPUDevice = await adapter.requestDevice();
  const queue: GPUQueue = device.queue;

  console.log(`Adapter: ${adapter.name}`);
  console.log(`Features: ${[...adapter.features].join(", ")}`);

  return { adapter, device, queue };
}
```

The adapter query is analogous to `cudaGetDeviceProperties`. The device creation is analogous to `cudaSetDevice` + `cudaStreamCreate`.

### Step 2: Compute Shader — Vector Addition

A WGSL compute shader that adds two float32 arrays element-by-element.

**`main.wgsl` — vector addition entry:**

```wgsl
@group(0) @binding(0) var<storage, read>     a: array<f32>;
@group(0) @binding(1) var<storage, read>     b: array<f32>;
@group(0) @binding(2) var<storage, read_write> c: array<f32>;

@compute @workgroup_size(256)
fn vec_add(@builtin(global_invocation_id) id: vec3u) {
    let idx = id.x;
    let n = arrayLength(&a);
    if (idx >= n) { return; }
    c[idx] = a[idx] + b[idx];
}
```

**`main.ts` — dispatching vector addition:**

```typescript
async function runVectorAdd(
  device: GPUDevice,
  queue: GPUQueue,
  n: number,
): Promise<Float32Array> {
  // 1. Create input buffers
  const aData = new Float32Array(n);
  const bData = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    aData[i] = i;
    bData[i] = 2 * i;
  }

  const bufA = createBuffer(device, aData, GPUBufferUsage.STORAGE);
  const bufB = createBuffer(device, bData, GPUBufferUsage.STORAGE);
  const bufC = device.createBuffer({
    size: n * Float32Array.BYTES_PER_ELEMENT,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
  });

  // 2. Create bind group
  const bindGroup = device.createBindGroup({
    layout: pipelineVecAdd.getBindGroupLayout(0),
    entries: [
      { binding: 0, resource: { buffer: bufA } },
      { binding: 1, resource: { buffer: bufB } },
      { binding: 2, resource: { buffer: bufC } },
    ],
  });

  // 3. Encode and dispatch
  const encoder = device.createCommandEncoder();
  const pass = encoder.beginComputePass();
  pass.setPipeline(pipelineVecAdd);
  pass.setBindGroup(0, bindGroup);
  pass.dispatchWorkgroups(Math.ceil(n / 256));
  pass.end();
  queue.submit([encoder.finish()]);

  // 4. Read back result
  return readBuffer(device, queue, bufC, n);
}
```

Total invocations = `ceil(n / 256) * 256`. Workgroups are dispatched in (X, Y, Z) dimensions; for a 1D vector we only use X.

### Step 3: WGSL Matrix Multiply with Workgroup Shared Memory

The tile-based approach. A workgroup of size 16×16 loads a 16×16 tile of A and a 16×16 tile of B into `workgroup` memory, then all 256 invocations compute partial products.

**`main.wgsl` — matrix multiply entry:**

```wgsl
const TILE_SIZE = 16u;

var<workgroup> tileA: array<array<f32, TILE_SIZE>, TILE_SIZE>;
var<workgroup> tileB: array<array<f32, TILE_SIZE>, TILE_SIZE>;

@group(0) @binding(0) var<storage, read>     A: array<f32>;
@group(0) @binding(1) var<storage, read>     B: array<f32>;
@group(0) @binding(2) var<storage, read_write> C: array<f32>;

@compute @workgroup_size(TILE_SIZE, TILE_SIZE, 1)
fn matmul(@builtin(global_invocation_id) gid: vec3u,
          @builtin(local_invocation_id) lid: vec3u,
          @builtin(workgroup_id) wgid: vec3u) {
    let N = u32(sqrt(f32(arrayLength(&C)))); // assume square
    let col = gid.x;
    let row = gid.y;
    if (col >= N || row >= N) { return; }

    var sum = 0.0;
    let numTiles = (N + TILE_SIZE - 1u) / TILE_SIZE;

    for (var t = 0u; t < numTiles; t++) {
        let tcol = t * TILE_SIZE + lid.x;
        let trow = t * TILE_SIZE + lid.y;
        if (tcol < N && trow < N) {
            tileA[lid.y][lid.x] = A[trow * N + (t * TILE_SIZE + lid.x)];
            tileB[lid.y][lid.x] = B[(t * TILE_SIZE + lid.y) * N + col];
        } else {
            tileA[lid.y][lid.x] = 0.0;
            tileB[lid.y][lid.x] = 0.0;
        }
        workgroupBarrier();

        for (var k = 0u; k < TILE_SIZE; k++) {
            sum += tileA[lid.y][k] * tileB[k][lid.x];
        }
        workgroupBarrier();
    }

    C[row * N + col] = sum;
}
```

**Key points:**
- `workgroupBarrier()` synchronizes all invocations in the workgroup — analogous to `__syncthreads()` in CUDA.
- Shared memory `tileA` / `tileB` are in `workgroup` address space: fast on-chip SRAM.
- Each invocation handles the (row, col) element of C, accumulating across tiles of the K dimension.

### Step 4: CPU vs GPU Performance Comparison

**`main.ts` — benchmark harness:**

```typescript
function cpuMatMul(A: Float32Array, B: Float32Array, N: number): Float32Array {
  const C = new Float32Array(N * N);
  for (let i = 0; i < N; i++) {
    for (let j = 0; j < N; j++) {
      let sum = 0;
      for (let k = 0; k < N; k++) {
        sum += A[i * N + k] * B[k * N + j];
      }
      C[i * N + j] = sum;
    }
  }
  return C;
}

async function benchmark(device: GPUDevice, queue: GPUQueue, N: number) {
  // CPU
  const cpuStart = performance.now();
  const C_cpu = cpuMatMul(A, B, N);
  const cpuTime = performance.now() - cpuStart;

  // GPU
  const gpuStart = performance.now();
  const C_gpu = await gpuMatMul(device, queue, A, B, N);
  const gpuTime = performance.now() - gpuStart;

  // Verify correctness
  let maxDiff = 0;
  for (let i = 0; i < N * N; i++) {
    maxDiff = Math.max(maxDiff, Math.abs(C_cpu[i] - C_gpu[i]));
  }

  console.log(`N=${N}: CPU=${cpuTime.toFixed(2)}ms GPU=${gpuTime.toFixed(2)}ms ` +
              `speedup=${(cpuTime / gpuTime).toFixed(2)}x maxErr=${maxDiff.toExponential(2)}`);
}
```

At N=512, expect the GPU to be 5–20x faster depending on hardware. At small N (N=32), the GPU dispatch overhead dominates and the CPU wins.

## Use It

WebGPU is the direct cross-platform replacement for CUDA when portability matters. Here is how they compare:

| Aspect | CUDA | WebGPU / WGSL |
|--------|------|---------------|
| **Vendor lock-in** | NVIDIA only | Any GPU (D3D12, Metal, Vulkan) |
| **Driver requirement** | NVIDIA driver + CUDA toolkit | Native: WebGPU implementation. Browser: included in Chrome/Edge |
| **Language** | CUDA C++ (extended C++) | WGSL (new language, GLSL-like) |
| **Memory model** | Unified virtual addressing | Explicit buffer binding (bind groups) |
| **Shared memory** | `__shared__` | `var<workgroup>` |
| **Synchronization** | `__syncthreads()` | `workgroupBarrier()` |
| **Kernel launch** | `kernel<<<grid, block>>>` | `pass.dispatchWorkgroups(x, y, z)` |
| **Error handling** | `cudaGetLastError` | GPU error scopes, uncaptured error callbacks |
| **Compilation model** | Offline (nvcc) + PTX JIT | Runtime compilation (WGSL → native IR) |
| **Ecosystem** | cuBLAS, cuDNN, Thrust, etc. | Growing (direct-wgsl, WebGPU compute libraries) |

**What the production version does that yours doesn't:**
- **Pipeline caching:** Production WebGPU implementations (Dawn, wgpu) cache compiled shaders to disk, avoiding recompilation.
- **Resource binding at scale:** Large apps use multiple bind groups and dynamic offsets; the scaffolding abstracts this.
- **Command reuse:** `RenderBundle`/`ComputeBundle` allows pre-recording work that replays each frame.
- **Buffer pooling:** Production apps reuse buffers across frames rather than allocating per-dispatch.
- **Adapter selection:** Sophisticated heuristics choose between integrated and discrete GPUs.

**When WebGPU won't beat CUDA:**
- Peak FP32/FP16 throughput — CUDA can exploit NVIDIA-specific tensor cores and warp-level primitives.
- Access to vendor-specific features (NVIDIA's async copy engines, AMD's Infinity Fabric).
- Mature libraries (cuBLAS GEMM is hand-tuned for each GPU generation; WebGPU WSL implementations do not have this level of tuning).

## Read the Source

- [WebGPU W3C Specification](https://www.w3.org/TR/webgpu/) — The authoritative spec. Read sections on compute pipelines (9.4), bind groups (8.6), and command encoding (9.3).
- [WGSL W3C Specification](https://www.w3.org/TR/WGSL/) — The shading language spec. Address spaces (section 4), built-in functions (section 16), and memory model (section 11).
- [Dawn](https://dawn.googlesource.com/dawn) — Google's C++ WebGPU implementation (used by Chrome). The `dawn_native` backend maps WebGPU to D3D12/Vulkan/Metal.
- [wgpu](https://github.com/gfx-rs/wgpu) — Mozilla's Rust WebGPU implementation (used by Firefox). It powers `wgpu-native` for native applications. Look at `wgpu-core/src/compute.rs` for compute pipeline compilation.
- [WebGPU Samples (Google)](https://webgpu.github.io/webgpu-samples/) — Official sample gallery. The "Compute Boids" and "Matrix Multiplication" samples are closest to this lesson.

## Ship It

The reusable artifact from this lesson lives in `outputs/`. It is:

- **A self-contained WebGPU compute shader scaffold** (`compute-pipeline.ts`) — a reusable class that wraps adapter/device initialization, buffer creation, bind group management, and readback. Use it in later projects that need GPU compute (image processing, physics simulation, particle systems) without rewriting the WebGPU boilerplate.

Every time you need to move data to the GPU, run a kernel, and read results back, start from `outputs/compute-pipeline.ts`.

## Exercises

1. **Easy** — Change the vector addition shader to compute `c[i] = a[i] * b[i] + a[i]` (fused multiply-add). Re-run the benchmark. Does the GPU dispatch logic change? Why or why not?

2. **Medium** — Extend the matrix multiplication shader from N×N to M×K×N (rectangular matrices). The `N = sqrt(arrayLength(&C))` assumption breaks. Modify both the WGSL shader and the TypeScript dispatch to accept arbitrary dimensions. Validate correctness against the CPU version.

3. **Hard** — Implement a 1D convolution compute shader (box blur on an audio buffer). Input: `float32[n]`, kernel size K. Each output element is the average of K neighbors. Use workgroup shared memory to reduce global memory reads: each workgroup loads a tile of the input, applies the kernel, and writes the tile's output. Compare performance with and without shared memory at varying kernel sizes (K=3, 7, 15, 31). At what K does shared memory stop helping (register pressure wins)?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| WebGPU | "The new WebGL" | A cross-platform GPU API providing direct access to D3D12, Metal, and Vulkan from the browser and native apps |
| WGSL | "WebGPU Shading Language" | A typed GPU shading language that compiles to SPIR-V, MSL, or DXIL depending on the backend |
| Adapter | "The GPU" | A handle to a physical GPU device (or software fallback), obtained via `navigator.gpu.requestAdapter()` |
| Device | "The logical GPU connection" | An allocated connection to the adapter — all GPU resources and pipelines are created from it |
| Queue | "The command stream" | The submission channel for encoded GPU commands; `device.queue.submit([commandBuffer])` |
| Bind Group | "The resource table" | A set of GPU resources (buffers, textures, samplers) bound to specific binding points in the shader |
| Compute Pipeline | "The compiled shader" | A state object combining the compiled compute shader, pipeline layout, and device limits |
| Workgroup | "The thread block" | A group of invocations that execute together, share `workgroup` memory, and synchronize via barriers |
| Compute Shader | "The GPU kernel" | A shader stage that runs on the compute unit, processing arbitrary data via storage buffers — no vertex/fragment pipeline needed |
| Storage Buffer | "The GPU memory" | A buffer visible to compute shaders with `read`, `write`, or `read_write` access, analogous to `cudaMalloc` |
| Dispatch | "Launch the kernel" | The WebGPU command `pass.dispatchWorkgroups(x, y, z)` that launches a grid of workgroups |
| workgroupBarrier | "Thread synchronization" | A WGSL built-in that synchronizes all invocations within a workgroup at a memory fence point — equivalent to `__syncthreads()` in CUDA |

## Further Reading

- W3C, "WebGPU Specification" — The canonical API reference. Read the compute pipeline and command encoder sections for the low-level details of dispatch.
- W3C, "WGSL Specification" — The language reference. Section 3 (address spaces) and Section 11 (memory model) are essential reading.
- Google, "WebGPU Samples" — Official sample gallery with runnable code for compute, render, and hybrid pipelines.
- Dzmitry Malyshau, "wgpu: The cross-platform graphics and compute library" (Rust) — The wgpu project README explains the design philosophy: safe, portable, and fast GPU access without vendor lock-in.
- "WebGPU Fundamentals" (webgpufundamentals.org) — A tutorial series that starts from the same basics as this lesson and extends to rendering, textures, and multi-pass compute.
- NVIDIA, "CUDA C++ Programming Guide" — Compare the programming model from Lesson 19 with WebGPU. The mental model (grids of thread blocks → grids of workgroups) maps directly.
