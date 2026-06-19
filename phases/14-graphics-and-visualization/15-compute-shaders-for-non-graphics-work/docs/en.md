# Compute Shaders for Non-Graphics Work

> GPUs aren't just for pixels anymore. Compute shaders let you harness thousands of parallel
> threads for any data-parallel problem — sorting, scanning, reducing, simulating — without ever
> touching a rasterizer.

**Type:** Learn
**Languages:** WGSL, CUDA C++
**Prerequisites:** Phase 14 lessons 01–14
**Time:** ~60 minutes

## Learning Objectives

- Explain what a compute shader is and why it exists outside the graphics pipeline.
- Describe the thread hierarchy: grid → workgroup → thread, and the role of local vs global IDs.
- Use shared/workgroup memory and barriers to coordinate threads within a workgroup.
- Implement parallel reduction (sum, max, min) with O(log n) depth.
- Implement parallel prefix sum (Blelloch scan).
- Compare CUDA and WebGPU compute shader models and translate between them.
- Identify when GPU compute beats CPU approaches and when it doesn't.

## The Problem

You have an array of 16 million floating-point numbers and you need the sum. On a CPU that's a
simple loop — 16 million iterations, one after another. On a GPU you can do better. Much better.
But you can't use a vertex shader or a fragment shader for this — there's no geometry, no pixels.
What you need is a **compute shader**: a program that runs on the GPU but isn't part of any
rendering pipeline.

This lesson sits in **Phase 14 — Computer Graphics & Visualization**. Compute shaders are the
Swiss Army knife of the GPU. Path tracers use them for BVH construction. Rasterizers use them
for culling and occlusion. Particle systems, physics solvers, AI inference — all compute shaders.
Without them, you're limited to the fixed graphics pipeline and you'll hit a wall the moment you
need to do general-purpose parallel computation on GPU data.

## The Concept

### What Is a Compute Shader?

A compute shader is a GPU program that:
- Has no vertex input, no fragment output, no fixed-function stages
- Runs across a user-defined grid of threads
- Reads and writes arbitrary buffers (storage buffers, images)
- Can share data between threads in the same workgroup via fast on-chip memory

Think of it as: "give me a pile of data, and I'll run the same function on every element — but
smarter than just one-thread-per-element, because threads can cooperate."

### The Thread Hierarchy

```
┌─────────────────────────────────────────────────────┐
│                     GRID (dispatch)                  │
│  Total threads = workgroup_count × workgroup_size   │
│                                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────┐ │
│  │ Workgroup 0  │  │ Workgroup 1  │  │   ...     │ │
│  │ ┌──┬──┬──┐  │  │ ┌──┬──┬──┐  │  │           │ │
│  │ │T0│T1│T2│  │  │ │T0│T1│T2│  │  │           │ │
│  │ ├──┼──┼──┤  │  │ ├──┼──┼──┤  │  │           │ │
│  │ │T3│T4│T5│  │  │ │T3│T4│T5│  │  │           │ │
│  │ └──┴──┴──┘  │  │ └──┴──┴──┘  │  │           │ │
│  │ shared mem  │  │ shared mem   │  │           │ │
│  └──────────────┘  └──────────────┘  └───────────┘ │
└─────────────────────────────────────────────────────┘

Each thread has:
  - Local ID  (within its workgroup): threadIdx.x / @local_invocation_id
  - Global ID (in the whole grid):     blockIdx.x*blockDim.x+threadIdx.x
                                      / @global_id
```

**Key insight:** Threads within the same workgroup share memory and can synchronize. Threads in
different workgroups **cannot** communicate during a single dispatch. This is fundamental.

### Memory Hierarchy

```
┌─────────────────────────────────────┐
│           REGISTERS (per thread)    │  Fastest, most private
│           1 cycle access            │
├─────────────────────────────────────┤
│     SHARED / WORKGROUP MEMORY       │  Fast on-chip, shared
│     ~5 cycle access                 │  within workgroup only
├─────────────────────────────────────┤
│     GLOBAL / DEVICE MEMORY          │  Slow off-chip, accessible
│     ~400-800 cycle access           │  by all threads, persists
└─────────────────────────────────────┘
```

| Memory Level   | CUDA term          | WGSL term          | Scope        | Speed    |
|----------------|--------------------|--------------------|--------------|----------|
| Registers      | Local variable     | Function variable  | Single thread| ~1 cycle |
| Shared         | `__shared__`       | `var<workgroup>`   | Workgroup    | ~5 cycles|
| Global          | Device pointer     | Storage buffer     | All threads  | ~500 cyc |
| Constant       | `__constant__`     | `var<uniform>`     | All threads  | ~5 cycles|

### Barriers: Why and When

Threads in a workgroup run in parallel but not in lockstep. If thread 0 writes to shared memory
and thread 3 reads it, thread 3 might read stale data unless you insert a barrier:

```
Thread 0:  shared[0] = 5;  ──●──  read shared[1]  →  correct!
Thread 3:  shared[3] = 7;  ──●──  read shared[0]  →  correct!
                                  ▲
                          barrier() here

Without barrier():
Thread 0:  shared[0] = 5;  ──────  read shared[1]  →  might be 0!
Thread 3:  shared[3] = 7;  ───  read shared[0]  →  might be 0! ✗
```

- **CUDA:** `__syncthreads()` — barrier for all threads in a block
- **WGSL:** `workgroupBarrier()` — barrier for all threads in a workgroup

## The Reduction Pattern

Reducing an array to a single value (sum, max, min) is the "hello world" of compute shaders.

### Naive Approach (O(n) depth)

One thread iterates the whole array — no parallelism at all. Depth: O(n).

### Tree Reduction (O(log n) depth)

```
Input:  [3, 1, 4, 1, 5, 9, 2, 6]

Step 1 (stride=4):  [3+5, 1+9, 4+2, 1+6, 5, 9, 2, 6]
                  = [8,   10,  6,   7,   5, 9, 2, 6]

Step 2 (stride=2):  [8+6,  10+7,  6,    7,   5, 9, 2, 6]
                  = [14,   17,    6,    7,   5, 9, 2, 6]

Step 3 (stride=1):  [14+17, 17,    6,    7,   5, 9, 2, 6]
                  = [31,    17,    6,    7,   5, 9, 2, 6]

Result: shared[0] = 31  (sum of all 8 elements)
```

At each step, half the threads are active. After log₂(n) steps, thread 0 holds the answer.
Total work: n/2 + n/4 + ... + 1 = n-1 additions (same as sequential), but depth is only log₂(n).

### Worked Example: Reduction with 8 elements

| Step | Stride | Thread 0 | Thread 1 | Thread 2 | Thread 3 |
|------|--------|----------|----------|----------|----------|
| init | —      | s[0]=3   | s[1]=1   | s[2]=4   | s[3]=1   |
| 1    | 4      | s[0]+=s[4]=5→8 | s[1]+=s[5]=9→10 | s[2]+=s[6]=2→6 | s[3]+=s[7]=6→7 |
| 2    | 2      | s[0]+=s[2]=6→14 | s[1]+=s[3]=7→17 | — | — |
| 3    | 1      | s[0]+=s[1]=17→31 | — | — | — |

Thread 0 reads `shared[0]` = 31 = 3+1+4+1+5+9+2+6. Correct!

## The Prefix Sum (Scan) Pattern

Prefix sum computes the running totals: given [x₀, x₁, x₂, ...], produce
[x₀, x₀+x₁, x₀+x₁+x₂, ...]. This is the backbone of stream compaction, sorting, and
many other parallel algorithms.

### Blelloch (Up-Sweep / Down-Sweep) Scan

Two phases, each O(log n) depth:

```
Input:     [2, 1, 3, 4, 5, 6, 7, 8]

UP-SWEEP (reduction phase — build partial sums):
  stride=1:  [2, 3, 3, 7, 5,11, 7,15]   (pairs summed)
  stride=2:  [2, 3, 3,10, 5,11, 7,22]   (quads summed)
  stride=4:  [2, 3, 3,10, 5,11, 7,31]   (halves summed)

  After up-sweep: last element = total sum = 31

DOWN-SWEEP (distribution phase — spread partial sums):
  Set last element to 0:
  stride=4:  [2, 3, 3,10, 5,11,31, 0]   (swap & distribute)
  stride=2:  [2, 3,10, 0,21,10, 0, 0]   (swap & distribute)
  stride=1:  [ 0, 2, 3, 5,10,15,21, 0]   (swap & distribute)

  Exclusive scan: [0, 2, 3, 6, 10, 15, 21, 28]
```

The Blelloch scan produces an **exclusive** prefix sum in O(log n) steps using only
O(n) total work.

## CUDA vs WebGPU Compute: Translation Guide

| Concept              | CUDA                            | WebGPU / WGSL                     |
|---------------------|----------------------------------|------------------------------------|
| Kernel declaration   | `__global__ void kernel(...)`   | `@compute fn kernel(...)`         |
| Dispatch size        | `<<<grid, block>>>`             | `@workgroup_size(x, y, z)`        |
| Block/workgroup ID   | `blockIdx.x`                    | `@workgroup_id`                   |
| Thread ID (local)    | `threadIdx.x`                   | `@local_invocation_id`            |
| Thread ID (global)   | `blockIdx.x*blockDim.x+threadIdx.x` | `@global_id`                 |
| Shared memory        | `__shared__ float s[N]`         | `var<workgroup> s: array<f32, N>` |
| Barrier              | `__syncthreads()`               | `workgroupBarrier()`              |
| Buffer binding       | Pointer arg                      | `@binding(N)` storage buffer      |
| Atomic ops           | `atomicAdd()`, `atomicMax()`   | `atomicAdd()`, `atomicMax()`      |
| Dispatch call        | `kernel<<<...>>>(args)`         | `pass.setPipeline(...); pass.dispatchWorkgroups(...)` |

## When Compute Shaders Win (and When They Don't)

**Compute shaders beat CPUs when:**
- Data is already on the GPU (no transfer cost)
- The problem has massive data parallelism (millions of elements)
- Each element does the same operation (SIMD-friendly)
- Memory access patterns are coalesced or can use shared memory

**CPUs still win when:**
- Data must be transferred from CPU → GPU → CPU (transfer latency)
- The problem has complex branching or is inherently serial
- Working set fits in CPU cache and data parallelism is low
- Debugging and development speed matter more than throughput

**Rule of thumb:** If n < 10,000 or the kernel does less than ~100 FLOPs per element,
the GPU overhead probably isn't worth it.

## Build It

### Step 1: Minimal Parallel Reduction (CUDA)

```cuda
__global__ void reduce_sum(float* data, float* result, int n) {
    __shared__ float s[256];
    int tid = threadIdx.x;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    s[tid] = (i < n) ? data[i] : 0.0f;
    __syncthreads();
    for (int stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (tid < stride) s[tid] += s[tid + stride];
        __syncthreads();
    }
    if (tid == 0) atomicAdd(result, s[0]);
}
```

### Step 2: Full Implementation with Prefix Sum

The complete code in `code/main.cu` includes both reduction and Blelloch prefix sum,
with host code that allocates, fills, dispatches, and verifies.

## Use It

### Production: Thrust and CUB

NVIDIA's **CUB** (CUDA Unbound) library provides `cub::DeviceReduce::Sum()` and
`cub::DeviceScan::ExclusiveSum()` — production-grade implementations that handle
arbitrary input sizes, multiple CTAs, and warp-level primitives. The code in
`code/main.cu` is the educational core of what CUB does internally.

### Production: WebGPU-native

WebGPU implementations (Chrome, Dawn, wgpu) compile WGSL compute shaders to native
GPU compute pipelines. The WGSL code in `code/main.wgsl` mirrors what production
WebGPU apps use for post-processing, particle simulation, and GPU-driven rendering.

## Read the Source

- **CUB reduction kernel:** `cub/device/device_reduce.cuh` in the NVIDIA CUB library
- **WebGPU spec compute pipeline:** https://www.w3.org/TR/webgpu/#compute-pipeline
- **CUDA programming guide — shared memory:** Chapter on shared memory and synchronization

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. It is:

- **`compute_patterns.md`** — A reference card with reduction, prefix sum, and bitonic sort
  patterns in both CUDA and WGSL pseudocode. Keep it open when implementing GPU algorithms.

## Exercises

1. **Easy** — Modify the reduction kernel to compute the maximum instead of the sum. Change
   `s[tid] += s[tid + stride]` to `fmaxf(s[tid], s[tid + stride])`.
2. **Medium** — Implement a multi-block reduction that handles arrays larger than one
   workgroup. Each block writes its partial sum to a global array, then a second kernel
   reduces that array.
3. **Hard** — Implement bitonic merge sort on the GPU. Given 2^k elements, use
   O(k²) compare-and-swap stages, each parallelized across all elements. Compare
   performance against CPU `std::sort` for n ≥ 1M.

## Key Terms

| Term               | What people say        | What it actually means                                      |
|--------------------|------------------------|-------------------------------------------------------------|
| Compute shader     | "GPU program"          | A kernel that runs on GPU outside the graphics pipeline      |
| Workgroup          | "Thread block"         | A group of threads that share memory and can synchronize    |
| Dispatch           | "Launch"               | Specifying how many workgroups and threads to execute       |
| Shared memory      | "Fast memory"          | On-chip memory shared by threads in a workgroup (~5 cycles) |
| Barrier            | "Sync point"           | A fence ensuring all threads reach this point before any proceed |
| Reduction          | "Fold"                 | Combining all elements to one value (sum, max, min)        |
| Prefix sum         | "Scan"                 | Computing all running totals of an input sequence           |
| Blelloch scan      | "Work-efficient scan"  | Parallel prefix sum with O(n) work and O(log n) depth       |
| Global ID          | "Thread index"         | Unique index of a thread across the entire dispatch grid    |
| Local ID           | "Lane index"           | Index of a thread within its workgroup                      |

## Further Reading

- **CUDA C++ Programming Guide** — Chapter on shared memory and synchronization primitives
- **GPU Gems 2, Chapter 31** — "Scan Primitives for GPU Computing" (Blelloch et al.)
- **WebGPU Specification** — https://www.w3.org/TR/webgpu/#compute-pipeline
- **CUB Library** — https://nvlabs.github.io/cub/ — production CUDA primitives
- **"Parallel Prefix Sum (Scan) with CUDA"** — Harris et al., NVIDIA whitepaper