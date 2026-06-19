/// <reference types="@webgpu/types" />

type GPUResources = {
  adapter: GPUAdapter;
  device: GPUDevice;
  queue: GPUQueue;
};

type PipelineCache = {
  vecAdd: GPUComputePipeline;
  matMul: GPUComputePipeline;
};

let resources: GPUResources | null = null;
let pipelines: PipelineCache | null = null;

const WGSL_VEC_ADD = `
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
`;

const WGSL_MATMUL = `
const TILE_SIZE = 16u;

var<workgroup> tileA: array<array<f32, TILE_SIZE>, TILE_SIZE>;
var<workgroup> tileB: array<array<f32, TILE_SIZE>, TILE_SIZE>;

@group(0) @binding(0) var<storage, read>     A: array<f32>;
@group(0) @binding(1) var<storage, read>     B: array<f32>;
@group(0) @binding(2) var<storage, read_write> C: array<f32>;

@compute @workgroup_size(TILE_SIZE, TILE_SIZE, 1)
fn matmul_tiled(@builtin(global_invocation_id) gid: vec3u,
                @builtin(local_invocation_id) lid: vec3u) {
    let N = u32(sqrt(f32(arrayLength(&C))));
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
`;

export async function initWebGPU(): Promise<GPUResources> {
  if (!navigator.gpu) {
    throw new Error("WebGPU is not supported in this browser");
  }

  const adapter = await navigator.gpu.requestAdapter();
  if (!adapter) {
    throw new Error("No GPU adapter found");
  }

  const device = await adapter.requestDevice();
  const queue = device.queue;

  device.addEventListener("uncapturederror", (event) => {
    console.error("Uncaptured GPU error:", (event as GPUUncapturedErrorEvent).error);
  });

  console.log(`WebGPU adapter: ${adapter.name}`);
  console.log(`Features: ${[...adapter.features].join(", ")}`);

  resources = { adapter, device, queue };
  return resources;
}

function createShaderModule(device: GPUDevice, code: string): GPUShaderModule {
  return device.createShaderModule({ code });
}

function buildPipelines(device: GPUDevice): PipelineCache {
  const vecAddModule = createShaderModule(device, WGSL_VEC_ADD);
  const vecAddLayout = device.createPipelineLayout({
    bindGroupLayouts: [
      device.createBindGroupLayout({
        entries: [
          { binding: 0, visibility: GPUShaderStage.COMPUTE, buffer: { type: "read-only-storage" } },
          { binding: 1, visibility: GPUShaderStage.COMPUTE, buffer: { type: "read-only-storage" } },
          { binding: 2, visibility: GPUShaderStage.COMPUTE, buffer: { type: "storage" } },
        ],
      }),
    ],
  });
  const vecAdd = device.createComputePipeline({
    layout: vecAddLayout,
    compute: { module: vecAddModule, entryPoint: "vec_add" },
  });

  const matMulModule = createShaderModule(device, WGSL_MATMUL);
  const matMulLayout = device.createPipelineLayout({
    bindGroupLayouts: [
      device.createBindGroupLayout({
        entries: [
          { binding: 0, visibility: GPUShaderStage.COMPUTE, buffer: { type: "read-only-storage" } },
          { binding: 1, visibility: GPUShaderStage.COMPUTE, buffer: { type: "read-only-storage" } },
          { binding: 2, visibility: GPUShaderStage.COMPUTE, buffer: { type: "storage" } },
        ],
      }),
    ],
  });
  const matMul = device.createComputePipeline({
    layout: matMulLayout,
    compute: { module: matMulModule, entryPoint: "matmul_tiled" },
  });

  pipelines = { vecAdd, matMul };
  return pipelines;
}

function ensureResources(): GPUResources {
  if (!resources) throw new Error("initWebGPU() must be called first");
  return resources;
}

function ensurePipelines(): PipelineCache {
  if (!pipelines) {
    return buildPipelines(ensureResources().device);
  }
  return pipelines;
}

function createStorageBuffer(device: GPUDevice, data: Float32Array): GPUBuffer {
  const buffer = device.createBuffer({
    size: data.byteLength,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
  });
  device.queue.writeBuffer(buffer, 0, data);
  return buffer;
}

function createOutputBuffer(device: GPUDevice, n: number): GPUBuffer {
  return device.createBuffer({
    size: n * Float32Array.BYTES_PER_ELEMENT,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
  });
}

function createReadbackBuffer(device: GPUDevice, size: number): GPUBuffer {
  return device.createBuffer({
    size,
    usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
  });
}

async function readBuffer(
  device: GPUDevice,
  queue: GPUQueue,
  src: GPUBuffer,
  floatCount: number,
): Promise<Float32Array> {
  const byteSize = floatCount * Float32Array.BYTES_PER_ELEMENT;
  const dst = createReadbackBuffer(device, byteSize);

  const encoder = device.createCommandEncoder();
  encoder.copyBufferToBuffer(src, 0, dst, 0, byteSize);
  queue.submit([encoder.finish()]);

  await dst.mapAsync(GPUMapMode.READ);
  const data = new Float32Array(dst.getMappedRange().slice(0));
  dst.unmap();
  dst.destroy();
  return data;
}

export async function gpuVectorAdd(n: number): Promise<Float32Array> {
  const { device, queue } = ensureResources();
  const { vecAdd } = ensurePipelines();

  const a = new Float32Array(n);
  const b = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    a[i] = i;
    b[i] = 2 * i;
  }

  const bufA = createStorageBuffer(device, a);
  const bufB = createStorageBuffer(device, b);
  const bufC = createOutputBuffer(device, n);

  const bindGroup = device.createBindGroup({
    layout: vecAdd.getBindGroupLayout(0),
    entries: [
      { binding: 0, resource: { buffer: bufA } },
      { binding: 1, resource: { buffer: bufB } },
      { binding: 2, resource: { buffer: bufC } },
    ],
  });

  const encoder = device.createCommandEncoder();
  const pass = encoder.beginComputePass();
  pass.setPipeline(vecAdd);
  pass.setBindGroup(0, bindGroup);
  pass.dispatchWorkgroups(Math.ceil(n / 256));
  pass.end();
  queue.submit([encoder.finish()]);

  const result = await readBuffer(device, queue, bufC, n);

  bufA.destroy();
  bufB.destroy();
  bufC.destroy();

  return result;
}

export async function gpuMatMul(
  A: Float32Array,
  B: Float32Array,
  N: number,
): Promise<Float32Array> {
  const { device, queue } = ensureResources();
  const { matMul } = ensurePipelines();

  const bufA = createStorageBuffer(device, A);
  const bufB = createStorageBuffer(device, B);
  const bufC = createOutputBuffer(device, N * N);

  const bindGroup = device.createBindGroup({
    layout: matMul.getBindGroupLayout(0),
    entries: [
      { binding: 0, resource: { buffer: bufA } },
      { binding: 1, resource: { buffer: bufB } },
      { binding: 2, resource: { buffer: bufC } },
    ],
  });

  const encoder = device.createCommandEncoder();
  const pass = encoder.beginComputePass();
  pass.setPipeline(matMul);
  pass.setBindGroup(0, bindGroup);
  pass.dispatchWorkgroups(Math.ceil(N / 16), Math.ceil(N / 16));
  pass.end();
  queue.submit([encoder.finish()]);

  const result = await readBuffer(device, queue, bufC, N * N);

  bufA.destroy();
  bufB.destroy();
  bufC.destroy();

  return result;
}

export function cpuMatMul(A: Float32Array, B: Float32Array, N: number): Float32Array {
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

function makeRandomMatrix(N: number): Float32Array {
  const data = new Float32Array(N * N);
  for (let i = 0; i < data.length; i++) {
    data[i] = Math.random() * 10;
  }
  return data;
}

function verifyCorrectness(cpu: Float32Array, gpu: Float32Array): number {
  let maxDiff = 0;
  for (let i = 0; i < cpu.length; i++) {
    const diff = Math.abs(cpu[i] - gpu[i]);
    if (diff > maxDiff) maxDiff = diff;
  }
  return maxDiff;
}

export async function benchmarkVectorAdd(): Promise<void> {
  const sizes = [1_000, 100_000, 10_000_000];

  for (const n of sizes) {
    const start = performance.now();
    const result = await gpuVectorAdd(n);
    const gpuTime = performance.now() - start;

    let ok = true;
    for (let i = 0; i < n; i++) {
      if (Math.abs(result[i] - i - 2 * i) > 1e-6) {
        ok = false;
        break;
      }
    }

    console.log(
      `[vecAdd] n=${n.toLocaleString()} GPU=${gpuTime.toFixed(2)}ms ` +
      `first[0]=${result[0]} last=${result[n - 1]} ${ok ? "PASS" : "FAIL"}`,
    );
  }
}

export async function benchmarkMatMul(): Promise<void> {
  const sizes = [64, 128, 256, 512];

  for (const N of sizes) {
    const A = makeRandomMatrix(N);
    const B = makeRandomMatrix(N);

    const cpuStart = performance.now();
    const C_cpu = cpuMatMul(A, B, N);
    const cpuTime = performance.now() - cpuStart;

    const gpuStart = performance.now();
    const C_gpu = await gpuMatMul(A, B, N);
    const gpuTime = performance.now() - gpuStart;

    const err = verifyCorrectness(C_cpu, C_gpu);

    console.log(
      `[matMul] N=${N} CPU=${cpuTime.toFixed(2)}ms ` +
      `GPU=${gpuTime.toFixed(2)}ms ` +
      `speedup=${cpuTime > 0 ? (cpuTime / gpuTime).toFixed(2) : "N/A"}x ` +
      `maxErr=${err.toExponential(2)} ${err < 1e-2 ? "PASS" : "FAIL"}`,
    );
  }
}

export async function runFullDemo(): Promise<void> {
  console.log("=== WebGPU Compute Shaders Demo ===");
  console.log("Initializing WebGPU...");

  try {
    await initWebGPU();
    buildPipelines(resources!.device);

    console.log("\n--- Vector Addition ---");
    await benchmarkVectorAdd();

    console.log("\n--- Matrix Multiplication (tiled, shared memory) ---");
    await benchmarkMatMul();

    console.log("\nDemo complete.");
  } catch (err) {
    console.error("Demo failed:", err);
  }
}

if (typeof window !== "undefined") {
  (window as any).runWebGPUDemo = runFullDemo;
}
