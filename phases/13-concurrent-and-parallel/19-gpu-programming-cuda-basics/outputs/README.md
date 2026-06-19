# GPU Programming — CUDA Basics — Outputs

## Artifact

The reusable artifact is a set of CUDA kernels covering the four
essential data-parallel patterns:

- **Vector addition** (`vecAdd`) — template for any element-wise GPU
  operation (activation functions, normalisation, colour transforms).
- **Matrix multiplication** (`matMulNaive`, `matMulTiled`) — naive and
  shared-memory-tiled versions.  The tiled version reduces global-memory
  traffic by reusing data within 16×16 tiles.
- **Parallel reduction** (`reduce`, `reduceWarp`) — naive tree reduction
  in shared memory and an optimised version using `__shfl_down_sync`
  warp-level primitives.
- **Error handling** — `CUDA_CHECK` macro for all CUDA API calls.

These primitives are self-contained and reusable in later phases
(especially Phase 14 deep learning and the Phase 13 capstone:
work-stealing scheduler for heterogeneous workloads).

## Benchmarks

Compile and run:

```bash
cd code
nvcc -O3 -o gpu main.cu
./gpu
```

Profile with:

```bash
nsys nvprof ./gpu       # Nsight Systems timeline
nvprof --metrics gld_efficiency,gld_throughput,sm_efficiency ./gpu
```

### Expected Output (representative — Tesla T4 or RTX 3060)

```
=== GPU Programming — CUDA Basics ===

--- Step 1: Vector Addition ---
  N = 16777216, grid=65536, block=256
  Time: 18.20 ms  (176.3 GB/s)
  Errors: 0

--- Step 2: Matrix Multiplication ---
  Matrix 512x512 (K=512)
  Naive:  7.80 ms  (34.4 GFLOPS)  errors=0
  Tiled:  1.95 ms  (137.6 GFLOPS)  errors=0
  Speedup: 4.0x

--- Step 3: Parallel Reduction ---
  N = 4194304, blocks=16384, threads/block=256
  CPU sum (ref): 2097012.375000
  Naive tree:    2097012.375000  time=1.050 ms  diff=0.00e+00
  Warp shuffle:  2097012.375000  time=0.720 ms  diff=0.00e+00
  Speedup (warp vs tree): 1.5x

--- Step 4: Error Handling Demo ---
  Attempting invalid access to trigger CUDA_CHECK...
  cudaMalloc(..., -1) correctly failed: out of memory
  Error handling: PASSED
```

**Important:** Actual numbers depend on GPU model, clock speed, memory
bandwidth, and whether data fits in L2 cache.  The vector-add benchmark
operates on 64 MB of data per array (192 MB total), which exceeds the
L2 cache on most consumer GPUs, so it is memory-bandwidth-bound.

## Files

| File       | Purpose                                                       |
|------------|---------------------------------------------------------------|
| `gpu`      | Compiled binary (after `nvcc -O3 -o gpu main.cu`)            |

## Key Metrics

| Pattern          | Metric                     | What to look for                         |
|------------------|----------------------------|------------------------------------------|
| Vector add       | `gld_throughput`           | Should be near device peak bandwidth      |
| MatMul naive     | `gld_efficiency`           | ~100 % (coalesced) but still memory-bound |
| MatMul tiled     | `gld_throughput` + speedup | 4–8× over naive on 1024²+ matrices       |
| Reduction (tree) | `sm_efficiency`            | Drops as threads stall at `__syncthreads` |
| Reduction (warp) | Speedup over tree          | 1.3–2× due to fewer barriers + less smem |
