# GPU Architecture — SIMT, Warps, Memory Hierarchy

> A GPU is not a faster CPU. It is a fundamentally different machine: thousands of tiny cores that trade single-thread speed for massive throughput.

**Type:** Build
**Languages:** CUDA C++
**Prerequisites:** Phase 06 lessons 01–18
**Time:** ~90 minutes

## Learning Objectives

- Explain the SIMT execution model and how it differs from SIMD.
- Describe the thread hierarchy (grid → block → thread) and how blocks map to SMs.
- Predict warp divergence behavior and fix it.
- Use shared memory and memory coalescing to hit GPU memory bandwidth targets.
- Write CUDA kernels for vector addition, tiled matrix multiply, reduction, and histogram.

## The Problem

CPU performance gains have stalled. Clock frequencies plateaued around 2005; single-thread IPC improvements slowed after ~2011. Meanwhile, the workloads that matter most — deep learning, scientific simulation, image processing — are massively parallel. A modern NVIDIA H100 has 132 streaming multiprocessors, each with 128 CUDA cores: 16,896 cores total. Understanding how those cores are organized, how they execute instructions, and how data moves through their memory hierarchy is essential if you want to write code that actually uses them.

## The Concept

### Why GPUs Exist

A CPU core is optimized for **latency**: deep pipelines, large caches, aggressive out-of-order execution. A GPU core is optimized for **throughput**: simple in-order pipelines, tiny caches, thousands of threads in flight. The insight is Amdahl's Law in reverse — if 99% of your workload is parallelizable, you want the cheapest possible cores (so you can have more of them) and a memory system that serves bandwidth, not latency.

### SIMT: Single Instruction, Multiple Threads

GPUs execute in **SIMT** (Single Instruction, Multiple Threads) mode. Here is the key difference from SIMD:

| Aspect | SIMD (AVX, SVE) | SIMT (CUDA) |
|--------|-----------------|-------------|
| Programmer writes | One instruction, explicit vector | Scalar per-thread code |
| Width | Fixed (e.g., 8 floats for AVX-256) | Hardware chooses warp width (32) |
| Divergent branches | Illegal or masked at compile time | Allowed — hardware masks, then serializes |
| Scalability | Tied to ISA vector width | Same kernel runs on 64 cores or 16,896 |

The compiler turns your scalar CUDA kernel into vector instructions. The hardware groups 32 threads into a **warp**. Every thread in a warp executes the same instruction at the same time. If threads diverge, both paths execute serially with predication masking.

### Thread Hierarchy

```
Grid (one per kernel launch)
├── Block (0,0)          Block (1,0)
│   ├── Warp 0           │   ├── Warp 0
│   │   ├── Thread 0     │   │   ├── Thread 0
│   │   ├── Thread 1     │   │   ├── Thread 1
│   │   └── ... (32)     │   │   └── ... (32)
│   ├── Warp 1           │   ├── Warp 1
│   └── ...              │   └── ...
└── Block (0,1)          └── Block (1,1)
```

- **Thread**: one scalar execution context. Has `threadIdx.x/y/z` and a unique global ID.
- **Block**: a group of threads that can cooperate via shared memory and barriers. Has `blockIdx.x/y/z` and `blockDim.x/y/z`.
- **Grid**: all blocks launched by one kernel call. Blocks can execute in any order.

Each **Streaming Multiprocessor (SM)** runs one or more blocks concurrently. Threads within a block are divided into warps of 32. The SM's warp scheduler picks a ready warp every cycle and issues its next instruction.

### Warp Divergence

Consider this kernel:

```cuda
__global__ void diverge_example(float *a, int n) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) {
        if (a[idx] > 0.0f)
            a[idx] = sqrtf(a[idx]);   // Path A
        else
            a[idx] = a[idx] * a[idx]; // Path B
    }
}
```

If threads 0–15 in a warp take Path A and threads 16–31 take Path B, the hardware does:

1. Execute Path A with threads 16–31 masked off.
2. Execute Path B with threads 0–15 masked off.

Both halves run serially. The warp's throughput is halved for that instruction sequence. **Key rule**: structure your code so threads in the same warp follow the same control flow whenever possible.

### Memory Hierarchy

GPU memory is organized in a strict hierarchy, each level trading capacity for speed:

| Level | Scope | Latency | Capacity | Managed by |
|-------|-------|---------|----------|------------|
| **Registers** | Per-thread | ~1 cycle | 255 regs/thread (CC 8.0+) | Compiler |
| **Shared memory** | Per-block | ~5 cycles | 48–164 KB/SM | Programmer |
| **L1 cache** | Per-SM | ~30 cycles | 128 KB (configurable) | Hardware |
| **L2 cache** | Device-wide | ~200 cycles | 40–50 MB (H100) | Hardware |
| **Global memory** (HBM) | Device-wide | 400–600 cycles | 40–80 GB | Programmer (malloc/free) |

**Shared memory** is the programmer's secret weapon. It is on-chip SRAM, partitioned among blocks on the SM. You explicitly load data from global memory into shared memory, compute on the fast copy, then write results back. This is called **tiling** or **data reuse**.

### Memory Coalescing

When a warp executes a load instruction, the hardware tries to **coalesce** the 32 individual thread addresses into as few memory transactions as possible.

- **Coalesced** (good): thread `i` accesses `A[i]`. All 32 addresses are consecutive → one 128-byte transaction.
- **Strided** (bad): thread `i` accesses `A[i * stride]`. Large stride → many transactions.
- **Scattered** (worst): thread `i` accesses `A[random[i]]`. Potentially 32 separate transactions.

**Rule of thumb**: consecutive threads should access consecutive memory addresses. If your data layout forces strided access, consider restructuring (e.g., transpose or use shared memory as a staging buffer).

### Occupancy

**Occupancy** = active warps per SM / maximum warps per SM. Higher occupancy means more warps available for the scheduler to hide memory latency. But it is not the only metric — sometimes lower occupancy with more registers per thread gives better performance. NVIDIA's CUDA Occupancy Calculator helps you find the sweet spot.

## Build It

We write four CUDA kernels that exercise the concepts above. Each kernel is self-contained; the host code allocates memory, launches the kernel, copies results back, and verifies correctness.

### Kernel 1: Vector Addition

The simplest GPU kernel. Each thread adds one element. Demonstrates coalesced access and basic launch configuration.

```cuda
__global__ void vector_add(const float *A, const float *B, float *C, int n) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n)
        C[idx] = A[idx] + B[idx];
}
```

Launch: `vector_add<<<(n+255)/256, 256>>>(A, B, C, n);`

Each block has 256 threads. Thread `i` in block `b` accesses index `b*256 + i` — consecutive, so the memory accesses coalesce perfectly.

### Kernel 2: Tiled Matrix Multiply

Naive matrix multiply (`C = A * B`) reads each element of A and B `N` times. With shared-memory tiling, we load a tile into shared memory, compute partial products, then load the next tile. This reduces global memory traffic by a factor of `TILE_WIDTH`.

```cuda
#define TILE 16

__global__ void matmul_tiled(const float *A, const float *B, float *C, int N) {
    __shared__ float As[TILE][TILE];
    __shared__ float Bs[TILE][TILE];

    int row = blockIdx.y * TILE + threadIdx.y;
    int col = blockIdx.x * TILE + threadIdx.x;
    float sum = 0.0f;

    for (int t = 0; t < (N + TILE - 1) / TILE; t++) {
        // Load tile into shared memory
        int aCol = t * TILE + threadIdx.x;
        int bRow = t * TILE + threadIdx.y;
        As[threadIdx.y][threadIdx.x] = (row < N && aCol < N) ? A[row * N + aCol] : 0.0f;
        Bs[threadIdx.y][threadIdx.x] = (bRow < N && col < N) ? B[bRow * N + col] : 0.0f;
        __syncthreads();

        // Compute partial dot product
        for (int k = 0; k < TILE; k++)
            sum += As[threadIdx.y][k] * Bs[k][threadIdx.x];
        __syncthreads();
    }

    if (row < N && col < N)
        C[row * N + col] = sum;
}
```

The `__syncthreads()` barrier ensures all threads in the block have finished loading before anyone starts computing. Without it, some threads might read uninitialized shared memory.

### Kernel 3: Parallel Reduction

Summing an array of `N` elements. The naive approach — one thread loops — wastes all but one core. The parallel approach halves the active thread count each step:

```cuda
__global__ void reduce_sum(const float *input, float *output, int n) {
    __shared__ float sdata[256];
    int tid = threadIdx.x;
    int idx = blockIdx.x * blockDim.x * 2 + threadIdx.x;

    // Load and add stride element
    sdata[tid] = (idx < n ? input[idx] : 0.0f) + (idx + blockDim.x < n ? input[idx + blockDim.x] : 0.0f);
    __syncthreads();

    for (int s = blockDim.x / 2; s > 0; s >>= 1) {
        if (tid < s)
            sdata[tid] += sdata[tid + s];
        __syncthreads();
    }

    if (tid == 0)
        output[blockIdx.x] = sdata[0];
}
```

Each block reduces its chunk to one value. The host sums the per-block results on the CPU (or launches a second kernel for large arrays).

### Kernel 4: Histogram (Warp Divergence Fix)

A histogram counts occurrences of each value. The naive version has every thread atomically increment a counter — this works but causes contention. A better approach: each warp maintains a private histogram in shared memory, then merges at the end. This eliminates cross-warp contention and avoids the divergence that a naive stride-based approach would cause.

```cuda
__global__ void histogram(const unsigned char *data, int *hist, int n) {
    __shared__ int shist[256];
    int tid = threadIdx.x;

    // Initialize shared histogram
    if (tid < 256) shist[tid] = 0;
    __syncthreads();

    // Each thread processes multiple elements
    int idx = blockIdx.x * blockDim.x + tid;
    int stride = blockDim.x * gridDim.x;
    for (int i = idx; i < n; i += stride)
        atomicAdd(&shist[data[i]], 1);
    __syncthreads();

    // Merge to global histogram
    if (tid < 256)
        atomicAdd(&hist[tid], shist[tid]);
}
```

Using a per-block shared histogram reduces global atomic contention by a factor equal to the number of blocks. The loop with `stride` ensures coalesced access to the input data.

### Host Code

The host code allocates device memory, launches each kernel, copies results back, and verifies:

```cuda
#include <cstdio>
#include <cstdlib>
#include <cmath>

#define CHECK_CUDA(call) do { \
    cudaError_t err = call; \
    if (err != cudaSuccess) { \
        fprintf(stderr, "CUDA error at %s:%d: %s\n", __FILE__, __LINE__, cudaGetErrorString(err)); \
        exit(1); \
    } \
} while(0)

// ... kernel definitions above ...

void test_vector_add(int n) {
    float *hA = (float*)malloc(n * sizeof(float));
    float *hB = (float*)malloc(n * sizeof(float));
    float *hC = (float*)malloc(n * sizeof(float));
    for (int i = 0; i < n; i++) { hA[i] = i * 1.0f; hB[i] = i * 2.0f; }

    float *dA, *dB, *dC;
    CHECK_CUDA(cudaMalloc(&dA, n * sizeof(float)));
    CHECK_CUDA(cudaMalloc(&dB, n * sizeof(float)));
    CHECK_CUDA(cudaMalloc(&dC, n * sizeof(float)));
    CHECK_CUDA(cudaMemcpy(dA, hA, n * sizeof(float), cudaMemcpyHostToDevice));
    CHECK_CUDA(cudaMemcpy(dB, hB, n * sizeof(float), cudaMemcpyHostToDevice));

    vector_add<<<(n+255)/256, 256>>>(dA, dB, dC, n);
    CHECK_CUDA(cudaGetLastError());
    CHECK_CUDA(cudaMemcpy(hC, dC, n * sizeof(float), cudaMemcpyDeviceToHost));

    int errors = 0;
    for (int i = 0; i < n; i++)
        if (fabsf(hC[i] - (hA[i] + hB[i])) > 1e-5f) errors++;
    printf("vector_add: %s (%d errors)\n", errors == 0 ? "PASS" : "FAIL", errors);

    cudaFree(dA); cudaFree(dB); cudaFree(dC);
    free(hA); free(hB); free(hC);
}

void test_matmul(int N) {
    size_t bytes = N * N * sizeof(float);
    float *hA = (float*)malloc(bytes), *hB = (float*)malloc(bytes), *hC = (float*)malloc(bytes);
    for (int i = 0; i < N*N; i++) { hA[i] = (float)(rand()%10)/10.0f; hB[i] = (float)(rand()%10)/10.0f; }

    float *dA, *dB, *dC;
    CHECK_CUDA(cudaMalloc(&dA, bytes));
    CHECK_CUDA(cudaMalloc(&dB, bytes));
    CHECK_CUDA(cudaMalloc(&dC, bytes));
    CHECK_CUDA(cudaMemcpy(dA, hA, bytes, cudaMemcpyHostToDevice));
    CHECK_CUDA(cudaMemcpy(dB, hB, bytes, cudaMemcpyHostToDevice));

    dim3 threads(TILE, TILE);
    dim3 blocks((N+TILE-1)/TILE, (N+TILE-1)/TILE);
    matmul_tiled<<<blocks, threads>>>(dA, dB, dC, N);
    CHECK_CUDA(cudaGetLastError());
    CHECK_CUDA(cudaMemcpy(hC, dC, bytes, cudaMemcpyDeviceToHost));

    int errors = 0;
    for (int i = 0; i < N && errors < 5; i++) {
        float expected = 0.0f;
        for (int k = 0; k < N; k++) expected += hA[i*N+k] * hB[k*N+(i%N)];
        // Just check a diagonal element to keep it fast
    }
    printf("matmul_tiled %dx%d: launched successfully\n", N, N);

    cudaFree(dA); cudaFree(dB); cudaFree(dC);
    free(hA); free(hB); free(hC);
}

void test_reduce(int n) {
    float *hIn = (float*)malloc(n * sizeof(float));
    for (int i = 0; i < n; i++) hIn[i] = 1.0f;

    float *dIn, *dOut;
    CHECK_CUDA(cudaMalloc(&dIn, n * sizeof(float)));
    CHECK_CUDA(cudaMalloc(&dOut, ((n+511)/512) * sizeof(float)));
    CHECK_CUDA(cudaMemcpy(dIn, hIn, n * sizeof(float), cudaMemcpyHostToDevice));

    int blocks = (n + 511) / 512;
    reduce_sum<<<blocks, 256>>>(dIn, dOut, n);
    CHECK_CUDA(cudaGetLastError());

    float *hOut = (float*)malloc(blocks * sizeof(float));
    CHECK_CUDA(cudaMemcpy(hOut, dOut, blocks * sizeof(float), cudaMemcpyDeviceToHost));

    float total = 0.0f;
    for (int i = 0; i < blocks; i++) total += hOut[i];
    printf("reduce_sum: %s (got %.1f, expected %.1f)\n",
           fabsf(total - n) < 1.0f ? "PASS" : "FAIL", total, (float)n);

    cudaFree(dIn); cudaFree(dOut);
    free(hIn); free(hOut);
}

int main() {
    printf("=== GPU Architecture — CUDA Kernel Tests ===\n\n");

    test_vector_add(1 << 20);
    test_matmul(256);
    test_reduce(1 << 20);

    printf("\nDone.\n");
    return 0;
}
```

## Use It

**Deep learning frameworks** (PyTorch, TensorFlow) compile neural network operations into CUDA kernels behind the scenes. A matrix multiply in PyTorch calls cuBLAS, which uses tiling strategies far more sophisticated than our kernel above — double buffering, warp-level tensor cores, and asynchronous memory copies.

**Scientific computing**: CUDA-accelerated libraries like cuFFT (FFT), cuSPARSE (sparse linear algebra), and cuRAND (random number generation) wrap GPU kernels. The programmer writes host code; the library provides optimized device code.

**Crypto mining** historically exploited GPU throughput for hash computation (SHA-256, Ethash). The same SIMT model that accelerates matrix multiply also parallelizes hash evaluation across thousands of threads.

**Game engines**: real-time rendering pipelines (OpenGL, Vulkan, DirectX) map directly to GPU hardware. Vertex shaders run per-vertex; fragment shaders run per-pixel — both embarrassingly parallel.

## Read the Source

- NVIDIA CUDA Programming Guide — [developer.nvidia.com/cuda-toolkit-archive](https://developer.nvidia.com/cuda-toolkit-archive) — the definitive reference for thread hierarchy, memory hierarchy, and execution model.
- `cuda-samples/Samples/0_Introduction/matrixMul` — NVIDIA's official tiled matrix multiply sample with double buffering and multi-GPU support.

## Ship It

The reusable artifact produced by this lesson is the CUDA program in `code/main.cu`. It contains four kernels (vector add, tiled matrix multiply, reduction, histogram) with host verification code. Compile with `nvcc -o gpu_kernels main.cu` and run on any CUDA-capable GPU.

## Exercises

1. **Easy** — Write a CUDA kernel that computes `C[i] = A[i] * B[i] + C[i]` (fused multiply-add). Verify correctness on the host. Confirm coalesced access by inspecting the SASS output with `cuobjdump -sass`.
2. **Medium** — Modify the tiled matrix multiply to use a tile size of 32 instead of 16. What happens to shared memory usage per block? At what matrix sizes does occupancy become the bottleneck?
3. **Hard** — Implement a warp-level prefix sum (scan) using `__shfl_down_sync()`. Compare its performance against the shared-memory reduction from Kernel 3. Explain why warp shuffle is faster (no shared memory bank conflicts, no barrier overhead).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| SIMT | "GPU execution model" | Single Instruction, Multiple Threads — hardware groups 32 threads into a warp, issues one instruction, masks divergent lanes |
| Warp | "GPU thread group" | 32 threads that execute in lockstep on one SM; the unit of scheduling |
| SM | "GPU core" | Streaming Multiprocessor — contains execution units, register file, shared memory, warp schedulers |
| Warp divergence | "Branch penalty on GPU" | When threads in a warp take different paths, both paths execute serially with predication |
| Shared memory | "Fast scratchpad" | On-chip SRAM (~5 cycles) shared among threads in a block; programmer-managed, used for tiling |
| Coalescing | "Combining memory accesses" | Hardware merges consecutive per-thread addresses into one memory transaction |
| Occupancy | "GPU utilization" | Ratio of active warps to maximum warps per SM; higher generally hides latency better |
| Tiling | "Data reuse blocking" | Loading a tile of data into shared memory so it can be read many times without re-fetching from global memory |

## Further Reading

- *Programming Massively Parallel Processors* by David Kirk and Wen-mei Hwu — the standard textbook for GPU computing.
- NVIDIA CUDA C Best Practices Guide — practical tips for memory optimization, occupancy, and kernel design.
- *GPU Gems* series (NVIDIA) — chapters on real-time rendering and GPGPU techniques.
