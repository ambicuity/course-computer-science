# GPU Programming — CUDA Basics

> Write data-parallel kernels that run thousands of threads on a GPU.
> Understand the memory hierarchy and thread model so your kernels
> saturate the hardware instead of leaving 90 % of its throughput idle.

**Type:** Build
**Languages:** CUDA C++
**Prerequisites:** Phase 13 lessons 01–17, especially lesson 16 "Parallel
Patterns" (map, reduce, scan) and lesson 18 "SIMD Programming"
**Time:** ~90 minutes

---

## Learning Objectives

- Explain the SIMT (Single Instruction, Multiple Thread) execution model
  and contrast it with SIMD on CPUs.
- Describe the CUDA thread hierarchy: grid → block → thread, and map
  kernel launch dimensions to the problem size.
- Identify the GPU memory hierarchy: global, shared, local, constant, and
  texture memory; explain when to use each.
- Write a vector addition kernel with correct grid/block sizing.
- Write a naive matrix multiplication kernel and an optimised version
  using shared-memory tiling.
- Implement a parallel sum reduction kernel with warp-level primitives.
- Handle CUDA errors with `cudaGetLastError` and `cudaDeviceSynchronize`.
- Profile a kernel with Nsight Systems / `nvprof` and identify the
  dominant bottleneck (memory-bound vs compute-bound).

---

## The Problem

You have a parallelisable workload — say, adding two arrays of 10 million
floats — running on a modern CPU.  A single core might process 8 floats
per cycle with AVX-512, giving roughly 100 GFLOPS peak.  A mid-range GPU
(GTX 3060) has 3584 CUDA cores running at ~1.8 GHz and delivers over
12 TFLOPS — **100× more throughput**.

But you cannot simply write a for loop and get this speed.  The GPU is a
**throughput-optimised** device: it hides latency by running many threads
concurrently, not by making individual threads fast.  If you launch too
few threads, use the wrong memory space, or access memory in a
non-coalesced pattern, your kernel will run 10–100× slower than the
hardware's capability.

This lesson gives you the mental model and the concrete patterns to avoid
those pitfalls.

---

## The Concept

### GPU Architecture — SIMT

A GPU contains an array of **Streaming Multiprocessors (SMs)**.  Each SM
has its own set of CUDA cores, registers, shared memory, and warp
schedulers.  Key numbers for a typical Ampere/Ada-generation GPU:

| Component | Typical count | Role |
|-----------|--------------|------|
| SMs        | 28–128       | Each SM executes warps independently |
| CUDA cores per SM | 64–128 | Integer/FP arithmetic units |
| Warp size  | 32 threads   | Hardware scheduling unit |
| Max threads per SM | 1024–2048 | Hides memory latency via interleaving |
| Register file per SM | 64K–256K × 32-bit | Zero-cycle access if operand hits register |
| Shared memory per SM | 48–228 KB | Program-managed L1 cache |
| L2 cache   | 2–96 MB      | Shared across all SMs |

**SIMT (Single Instruction, Multiple Thread):** All 32 threads in a warp
execute the *same instruction* on different data.  If threads in a warp
take different code paths (divergence), all paths are serialised and
disabled threads are masked out — the warp executes each path in turn.

Compare with CPU SIMD:
- SIMD: 1 instruction → N data elements, explicit vector width (8 × f32).
- SIMT: 1 instruction → 32 threads, each thread has its own program
  counter and registers.  Threads are not obliged to be lockstep, but the
  hardware is most efficient when they are.

### Thread Hierarchy — Grid, Block, Thread

```
grid (1D / 2D / 3D)
  └── block (0..1023 threads per block)
        └── thread (0..blockDim-1)
```

- **Grid**: all threads launched by `<<<grid, block>>>`.  Blocks within a
  grid can execute in any order (enables scalability across SMs).
- **Block**: a group of threads that cooperate via shared memory and
  `__syncthreads()`.  All threads in a block run on the *same* SM.
- **Thread**: the smallest execution unit.  Identified by
  `threadIdx.x`, `threadIdx.y`, `threadIdx.z` (within its block) and
  `blockIdx.x`, etc. (within the grid).

#### Kernel Launch Syntax

```cuda
// Launch a 1D grid of 1D blocks
kernel<<<numBlocks, threadsPerBlock>>>(args);

// 2D launch
dim3 grid(16, 16);
dim3 block(16, 16);
kernel<<<grid, block>>>(args);
```

The total number of threads is `gridDim.x × blockDim.x`.  Choose the
block size to maximise SM occupancy (typically 128–256 threads/block).

### Memory Hierarchy

| Memory    | Scope       | Latency | Size      | Cached? |
|-----------|-------------|---------|-----------|---------|
| Register  | Thread      | 0       | 255 per thread | — |
| Local     | Thread      | ~cycle + spill to global | 512 KB per thread | L1/L2 |
| Shared    | Block       | ~30 cyc  | 48–228 KB per SM | Program-managed |
| Global    | All threads + host | ~400–800 cyc | 4–24 GB | L2 only |
| Constant  | All         | ~1 cyc (hit), ~400 cyc (miss) | 64 KB | Dedicated |
| Texture   | All         | Varies   | Up to device memory | Optimised for 2D locality |

**Global memory** is the main data store.  It is *not* coherent between
host and device until `cudaDeviceSynchronize()` or an explicit copy.

**Shared memory** is on-chip SRAM, 30–50× faster than global memory.
Threads in a block use it as a collaborative scratchpad.  Banks are
organised in 32-bit words; avoid bank conflicts by having adjacent
threads access adjacent words.

**Coalesced access**: When 32 threads in a warp issue a global-memory
load, the hardware combines requests that fall on the same cache-line
segment (128 bytes).  Maximal coalescing happens when thread `i` in a
warp accesses element `i` of a contiguous array:

```cuda
float val = data[threadIdx.x + blockIdx.x * blockDim.x];  // coalesced
```

Strided access where thread `i` reads `data[i * 2]` halves effective
bandwidth.  Random access destroys coalescing entirely.

---

## Build It

Your task: implement four CUDA kernels that cover the essential patterns
for data-parallel GPU programming.

### Setup

All code lives in `code/main.cu`.  Compile with:

```bash
nvcc -O3 -o gpu main.cu
```

If you do not have an NVIDIA GPU, the code is still syntactically valid
CUDA C++ and can be read as a reference implementation.  To run on a
free cloud GPU: use Google Colab with a Tesla T4 runtime, or
<https://github.com/NVIDIA/nvidia-docker>.

Profile with:

```bash
# Nsight Systems (modern, recommended)
nsys nvprof ./gpu

# Legacy profiler
nvprof ./gpu
```

---

### Step 1: Vector Addition — The "Hello World" of CUDA

Implementation at `code/main.cu:vecAdd`.

The kernel is straightforward:

```cuda
__global__ void vecAdd(const float *A, const float *B, float *C, int N) {
    int i = threadIdx.x + blockIdx.x * blockDim.x;
    if (i < N) C[i] = A[i] + B[i];
}
```

**Grid/block sizing:**

```cuda
int threadsPerBlock = 256;
int blocksPerGrid = (N + threadsPerBlock - 1) / threadsPerBlock;
vecAdd<<<blocksPerGrid, threadsPerBlock>>>(d_A, d_B, d_C, N);
```

Each of the `blocksPerGrid × threadsPerBlock` launched threads computes
one output element.  The `if (i < N)` guard handles cases where `N` is
not a multiple of the block size.

**Host-side steps:**
1. Allocate host memory (`malloc` or `cudaMallocHost` for pinned memory).
2. Allocate device memory (`cudaMalloc`).
3. Copy input data from host → device (`cudaMemcpy`, `cudaMemcpyHostToDevice`).
4. Launch kernel.
5. Copy result from device → host (`cudaMemcpy`, `cudaMemcpyDeviceToHost`).
6. Free device and host allocations (`cudaFree`, `free`).

---

### Step 2: Matrix Multiplication — Naive and Tiled

Implementation at `code/main.cu:matMulNaive` and `code/main.cu:matMulTiled`.

**Naive version** (`matMulNaive`): Each thread computes one element of
the output matrix `C = A × B`.  Thread `(row, col)` computes the dot
product of `A[row, :]` and `B[:, col]`:

```cuda
__global__ void matMulNaive(const float *A, const float *B, float *C,
                            int M, int N, int K) {
    int row = blockIdx.y * blockDim.y + threadIdx.y;
    int col = blockIdx.x * blockDim.x + threadIdx.x;
    if (row < M && col < N) {
        float sum = 0.0f;
        for (int k = 0; k < K; k++)
            sum += A[row * K + k] * B[k * N + col];
        C[row * N + col] = sum;
    }
}
```

This is correct but **memory-bound**: each output element requires `2K`
global loads.  For a 1024×1024 matrix with K = 1024, each thread does
2048 global reads.  Global memory bandwidth is the bottleneck.

**Tiled version** (`matMulTiled`): Use shared memory to cache tiles of
`A` and `B`.  Threads in a block cooperatively load a tile from global
→ shared, compute partial dot-products from the tile, then load the next
tile.  The inner loop now reads shared memory (30× faster) instead of
global.

```cuda
#define TILE_SIZE 16
__global__ void matMulTiled(const float *A, const float *B, float *C,
                            int M, int N, int K) {
    __shared__ float As[TILE_SIZE][TILE_SIZE];
    __shared__ float Bs[TILE_SIZE][TILE_SIZE];

    int row = blockIdx.y * TILE_SIZE + threadIdx.y;
    int col = blockIdx.x * TILE_SIZE + threadIdx.x;
    float sum = 0.0f;

    for (int t = 0; t < (K + TILE_SIZE - 1) / TILE_SIZE; t++) {
        if (row < M && t * TILE_SIZE + threadIdx.x < K)
            As[threadIdx.y][threadIdx.x] = A[row * K + t * TILE_SIZE + threadIdx.x];
        else
            As[threadIdx.y][threadIdx.x] = 0.0f;

        if (col < N && t * TILE_SIZE + threadIdx.y < K)
            Bs[threadIdx.y][threadIdx.x] = B[(t * TILE_SIZE + threadIdx.y) * N + col];
        else
            Bs[threadIdx.y][threadIdx.x] = 0.0f;

        __syncthreads();

        for (int k = 0; k < TILE_SIZE; k++)
            sum += As[threadIdx.y][k] * Bs[k][threadIdx.x];

        __syncthreads();
    }

    if (row < M && col < N)
        C[row * N + col] = sum;
}
```

Key details:
- `__shared__` declares a per-block shared-memory array.
- `__syncthreads()` is a **block-wide barrier**: all threads in the block
  must reach it before any thread proceeds.  Needed between tile load
  and tile compute, and between compute and next tile load.
- Threads outside the matrix bounds write zero into the tile (padding).
- The tile size 16 means a block of 16×16 = 256 threads, which gives
  high occupancy on all modern GPUs.

**Speedup**: For 1024×1024 f32 matrices the tiled version typically
runs 4–8× faster than the naive version, depending on GPU memory
bandwidth and L2 cache size.

---

### Step 3: Parallel Reduction — Sum of an Array

Implementation at `code/main.cu:reduce` and `code/main.cu:reduceWarp`.

**Sum reduction**: given an array of `N` elements, compute the sum using
a tree-like approach.  Each thread starts with one element; after
`log₂(N)` steps, one thread holds the total.

```cuda
__global__ void reduce(const float *in, float *out, int N) {
    extern __shared__ float sdata[];
    int i = threadIdx.x + blockIdx.x * blockDim.x;
    sdata[threadIdx.x] = (i < N) ? in[i] : 0.0f;
    __syncthreads();

    for (int s = blockDim.x / 2; s > 0; s >>= 1) {
        if (threadIdx.x < s)
            sdata[threadIdx.x] += sdata[threadIdx.x + s];
        __syncthreads();
    }

    if (threadIdx.x == 0)
        out[blockIdx.x] = sdata[0];
}
```

**Warp-level primitives** (`reduceWarp`): Starting with Kepler (SM 3.0),
CUDA provides `__shfl_down_sync` for warp-level communication without
shared memory.  Within a single warp (32 threads), we can reduce in
just 5 steps:

```cuda
__device__ float warpReduce(float val) {
    unsigned mask = 0xffffffff;
    val += __shfl_down_sync(mask, val, 16);
    val += __shfl_down_sync(mask, val, 8);
    val += __shfl_down_sync(mask, val, 4);
    val += __shfl_down_sync(mask, val, 2);
    val += __shfl_down_sync(mask, val, 1);
    return val;  // lane 0 holds the warp sum
}
```

Combine warp reduction with shared memory:

```cuda
__global__ void reduceWarp(const float *in, float *out, int N) {
    extern __shared__ float sdata[];
    int i = threadIdx.x + blockIdx.x * blockDim.x;
    float val = (i < N) ? in[i] : 0.0f;

    // Each warp reduces to a single value
    val = warpReduce(val);

    // Warp leader writes to shared
    if (threadIdx.x % 32 == 0)
        sdata[threadIdx.x / 32] = val;
    __syncthreads();

    // Final reduction of warp leaders
    if (threadIdx.x < (blockDim.x + 31) / 32) {
        val = sdata[threadIdx.x];
        val = warpReduce(val);
        if (threadIdx.x == 0)
            out[blockIdx.x] = val;
    }
}
```

The warp-level version reduces shared-memory pressure and `__syncthreads`
calls, giving 1.5–2× speedup over the naive tree reduction for large
arrays.

---

### Step 4: Error Handling

Every CUDA API call and kernel launch can fail.  Always check errors:

```cuda
#define CUDA_CHECK(call)                                                    \
    do {                                                                    \
        cudaError_t err = call;                                             \
        if (err != cudaSuccess) {                                           \
            fprintf(stderr, "CUDA error at %s:%d: %s\n",                    \
                    __FILE__, __LINE__, cudaGetErrorString(err));            \
            exit(EXIT_FAILURE);                                             \
        }                                                                   \
    } while (0)
```

Use this macro for:
- `cudaMalloc`, `cudaMemcpy`, `cudaFree`
- `cudaDeviceSynchronize()` after kernel launch
- Other API calls

Kernel launches return immediately; the actual error (e.g., invalid
memory access) is reported asynchronously.  Always call
`cudaDeviceSynchronize()` and check with `cudaGetLastError()`:

```cuda
kernel<<<grid, block>>>(args);
CUDA_CHECK(cudaDeviceSynchronize());
CUDA_CHECK(cudaGetLastError());  // last asynchronous error
```

### Profiling

Profile your kernels to see if they are memory-bound or compute-bound:

```bash
# Nsight Systems timeline
nsys nvprof ./gpu

# Kernel-level metrics
nvprof --metrics gld_efficiency,gld_throughput,sm_efficiency ./gpu
```

Key metrics:
- **`gld_efficiency`**: fraction of global load requests that are
  coalesced.  Below 100 % indicates strided or random access.
- **`gld_throughput`**: bytes per second delivered from global memory.
  Compare to the device's peak bandwidth (e.g., ~450 GB/s for RTX 3060).
- **`sm_efficiency`**: fraction of cycles at least one warp is active.
  Low values suggest occupancy problems or synchronisation overhead.

For the tiled matrix multiply, expect `gld_efficiency` near 100 %
because tiles are loaded with coalesced access.

---

## Use It

Real GPU programs rarely stop at a single kernel.  The patterns in this
lesson compose into larger pipelines:

- **Vector add** is the template for any element-wise operation
  (sigmoid, relu, normalisation, colour-space conversion).
- **Tiled matrix multiply** is the core of cuBLAS, cuDNN, and all deep
  learning frameworks.  Real implementations add autotuning, register
  tiling, and Tensor Core instructions.
- **Reduction** appears in loss functions, attention softmax, batch
  normalisation, and any "sum over axis" operation.
- **Warp-level primitives** are used in graph traversal, sorting
  networks, and histograms.

### Read the Source

- **cuBLAS**: `cublasSgemm` — the production matrix multiply.  Compare
  its source (or documentation) with your tiled version.
- **CUB** (CUDA Unbound): `cub::BlockReduce`, `cub::WarpReduce` —
  production-quality reduction primitives.
- **Thrust**: `thrust::reduce`, `thrust::transform` — STL-like CUDA
  algorithms.

---

## Ship It

The reusable artifact from this lesson lives in `outputs/`:

- **`outputs/README.md`** — benchmark results and usage guide.
- **Compiled binary** — `gpu` (after `nvcc -O3 -o gpu main.cu`).

The `vecAdd`, `matMulNaive`, `matMulTiled`, `reduce`, and `reduceWarp`
kernels are self-contained and reusable in later phases (especially
Phase 14 deep learning and the Phase 13 capstone work-stealing
scheduler, where GPU tasks could be dispatched as grid launches).

---

## Exercises

1. **Easy** — Change `vecAdd` to compute `C[i] = sin(A[i]) + cos(B[i])`.
   Profile the kernel with `nvprof`; compare `sm_efficiency` to the
   original addition-only kernel.  Is it compute-bound now?

2. **Medium** — Extend `matMulTiled` to support arbitrary tile sizes
   (8, 16, 32).  Time each tile size for 1024×1024 matrices.  Which
   gives the best performance on your GPU, and why?

3. **Hard** — Implement a **3D convolution** kernel (1 channel, 3×3×3
   filter) using shared memory tiling.  The input is a W×H×D volume.
   Each thread computes one output voxel using a tile of the input
   volume loaded into shared memory.  Handle boundary conditions.

4. **Challenge** — Write a kernel that performs a **prefix sum (scan)**
   using the Blelloch algorithm with shared memory.  Compare your
   performance against `thrust::inclusive_scan`.

---

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| CUDA | Compute Unified Device Architecture | NVIDIA's parallel computing platform and programming model. |
| SIMT | Single Instruction, Multiple Thread | 32 threads in a warp share the same instruction but have private registers; divergence serialises paths. |
| Warp | A group of 32 threads scheduled together | The hardware unit of execution on an SM; all threads in a warp issue the same instruction. |
| SM | Streaming Multiprocessor | A GPU compute unit with its own CUDA cores, shared memory, registers, and warp schedulers. |
| Grid | All blocks launched by one `<<<...>>>` | A set of thread blocks that can execute in any order across SMs. |
| Block | A group of threads that cooperate via shared memory | All threads in a block run on the same SM; limited to 1024 threads. |
| Thread | A single execution context on the GPU | Has private registers, local memory, and a unique `threadIdx` / `blockIdx`. |
| Kernel | A `__global__` function that runs on the GPU | Called from host with `<<<grid, block>>>`; executes on device. |
| Shared memory | On-chip SRAM visible to all threads in a block | Program-managed cache, ~30-cycle latency, 48–228 KB per SM. |
| Global memory | Off-chip DRAM visible to all threads + host | High capacity (4–24 GB), high latency (~400–800 cycles). |
| Coalesced access | Adjacent threads access adjacent memory addresses | Necessary for full memory bandwidth; stride-1 access within a warp. |
| Reduction | Combining many values into one via an associative op | Tree-based parallel sum; warp-level shuffle for last 5 steps. |
| Tiling | Partitioning data into blocks that fit in shared memory | Reduces global-memory traffic by reusing data within a tile. |
| `__syncthreads` | Block-wide synchronisation barrier | All threads in a block must reach this point before proceeding. |
| Divergence | Threads in a warp take different branches | All taken paths are serialised; disabled threads are masked. |
| Occupancy | Active warps ÷ max warps per SM | Higher occupancy hides latency; balance with register/shared-memory usage. |
| `cudaGetLastError` | Retrieve the last asynchronous error | Essential after kernel launch to catch execution errors. |

---

## Further Reading

- NVIDIA CUDA C++ Programming Guide: https://docs.nvidia.com/cuda/cuda-c-programming-guide/
- NVIDIA CUDA C++ Best Practices Guide: https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/
- **CUB** (CUDA Unbound): https://nvlabs.github.io/cub/
- **Thrust**: https://thrust.github.io/
- Nsight Systems documentation: https://docs.nvidia.com/nsight-systems/
- Kirk & Hwu, *Programming Massively Parallel Processors* — the
  definitive textbook on GPU programming, now in its 4th edition.
- **GPU Gems** series: https://developer.nvidia.com/gpu-gems
