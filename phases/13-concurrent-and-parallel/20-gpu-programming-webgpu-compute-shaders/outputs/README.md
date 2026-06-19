# Output: WebGPU Compute Pipeline Scaffold

## Artifact

**`compute-pipeline.ts`** — A reusable class that wraps WebGPU initialization, buffer management, and compute pipeline creation. Use this scaffold in any project that needs GPU compute (image processing, physics, particle systems, ML inference, audio processing).

## Usage

```typescript
import { ComputePipeline } from "./compute-pipeline";

async function main() {
  const gpu = await ComputePipeline.init();

  // Create input/output buffers
  const input = new Float32Array([1, 2, 3, 4]);
  const bufIn = gpu.createStorageBuffer(input);
  const bufOut = gpu.createOutputBuffer(4);
  const bindings = [bufIn, bufOut];

  // Run a compute shader
  const result: Float32Array = await gpu.dispatch(
    wgslCode,
    entryPoint,     // e.g. "main"
    bindings,
    workgroupCountX, // e.g. 1
  );

  console.log(result);
}
```

## What It Abstracts

| Concern | Without scaffold | With scaffold |
|---------|-----------------|---------------|
| Adapter/device init | ~15 lines | 1 call |
| Pipeline compilation | ~10 lines | handled |
| Bind group creation | ~8 lines per dispatch | auto-derived |
| Buffer readback | ~10 lines + async | 1 call |
| Error handling | manual | built-in |

## Dependencies

- `@webgpu/types` — TypeScript type definitions for the WebGPU API.
- A browser with WebGPU support (Chrome 113+, Edge 113+, Firefox Nightly).

## Integration Points

- **From `code/main.ts`:** Copy the inner functions (`createStorageBuffer`, `createOutputBuffer`, `readBuffer`, `buildPipelines`) into the class.
- **From `code/main.wgsl`:** Reference the `vec_add` and `matmul_tiled` shaders as examples for writing your own compute kernels.

## When to Use This

- Any lesson or project in Phase 13+ that needs GPGPU without vendor lock-in.
- As a drop-in replacement for CUDA kernels in cross-platform applications.
- For in-browser compute (data viz, real-time simulation, client-side ML).
