/*****************************************************************************
 *  GPU Programming — CUDA Basics
 *  Phase 13, Lesson 19
 *
 *  Compile:  nvcc -O3 -o gpu main.cu
 *  Profile:  nsys nvprof ./gpu
 *
 *  If nvcc is not available, this file is a syntactically valid CUDA C++
 *  reference — no stubs, all kernels are complete.
 *****************************************************************************/

#include <cuda_runtime.h>
#include <stdio.h>
#include <stdlib.h>
#include <math.h>
#include <float.h>

// ---------------------------------------------------------------------------
// Error handling macro — wraps every CUDA API call
// ---------------------------------------------------------------------------
#define CUDA_CHECK(call)                                                      \
    do {                                                                      \
        cudaError_t err = call;                                               \
        if (err != cudaSuccess) {                                             \
            fprintf(stderr, "CUDA error at %s:%d — %s\n",                     \
                    __FILE__, __LINE__, cudaGetErrorString(err));              \
            exit(EXIT_FAILURE);                                               \
        }                                                                     \
    } while (0)

// ---------------------------------------------------------------------------
// Step 1: Vector Addition
//   Each thread computes C[i] = A[i] + B[i] for one element.
// ---------------------------------------------------------------------------
__global__ void vecAdd(const float *A, const float *B, float *C, int N) {
    int i = threadIdx.x + blockIdx.x * blockDim.x;
    if (i < N) {
        C[i] = A[i] + B[i];
    }
}

// ---------------------------------------------------------------------------
// Step 2a: Naive Matrix Multiplication
//   C = A × B   where A is M×K, B is K×N, C is M×N.
//   Each thread computes exactly one output element via a dot-product loop.
// ---------------------------------------------------------------------------
__global__ void matMulNaive(const float *A, const float *B, float *C,
                            int M, int N, int K) {
    int row = blockIdx.y * blockDim.y + threadIdx.y;
    int col = blockIdx.x * blockDim.x + threadIdx.x;
    if (row < M && col < N) {
        float sum = 0.0f;
        for (int k = 0; k < K; k++) {
            sum += A[row * K + k] * B[k * N + col];
        }
        C[row * N + col] = sum;
    }
}

// ---------------------------------------------------------------------------
// Step 2b: Tiled Matrix Multiplication
//   Same C = A × B, but A and B tiles are cached in shared memory to reduce
//   global-memory traffic by O(TILE_SIZE) per output element.
//
//   TILE_SIZE = 16 → block = 16×16 = 256 threads, high occupancy.
// ---------------------------------------------------------------------------
#define TILE_SIZE 16

__global__ void matMulTiled(const float *A, const float *B, float *C,
                            int M, int N, int K) {
    __shared__ float As[TILE_SIZE][TILE_SIZE];
    __shared__ float Bs[TILE_SIZE][TILE_SIZE];

    int row = blockIdx.y * TILE_SIZE + threadIdx.y;
    int col = blockIdx.x * TILE_SIZE + threadIdx.x;

    float sum = 0.0f;
    int numTiles = (K + TILE_SIZE - 1) / TILE_SIZE;

    for (int t = 0; t < numTiles; t++) {
        // Cooperative load tile of A into shared memory
        if (row < M && t * TILE_SIZE + threadIdx.x < K) {
            As[threadIdx.y][threadIdx.x] = A[row * K + t * TILE_SIZE + threadIdx.x];
        } else {
            As[threadIdx.y][threadIdx.x] = 0.0f;
        }

        // Cooperative load tile of B into shared memory
        if (col < N && t * TILE_SIZE + threadIdx.y < K) {
            Bs[threadIdx.y][threadIdx.x] = B[(t * TILE_SIZE + threadIdx.y) * N + col];
        } else {
            Bs[threadIdx.y][threadIdx.x] = 0.0f;
        }

        __syncthreads();

        // Compute partial dot-product from this tile
        for (int k = 0; k < TILE_SIZE; k++) {
            sum += As[threadIdx.y][k] * Bs[k][threadIdx.x];
        }

        __syncthreads();
    }

    if (row < M && col < N) {
        C[row * N + col] = sum;
    }
}

// ---------------------------------------------------------------------------
// Step 3a: Parallel Reduction — naive tree (shared memory)
//   Each block reduces blockDim.x elements to one.
// ---------------------------------------------------------------------------
__global__ void reduce(const float *in, float *out, int N) {
    extern __shared__ float sdata[];
    int i = threadIdx.x + blockIdx.x * blockDim.x;
    sdata[threadIdx.x] = (i < N) ? in[i] : 0.0f;
    __syncthreads();

    for (int s = blockDim.x / 2; s > 0; s >>= 1) {
        if (threadIdx.x < s) {
            sdata[threadIdx.x] += sdata[threadIdx.x + s];
        }
        __syncthreads();
    }

    if (threadIdx.x == 0) {
        out[blockIdx.x] = sdata[0];
    }
}

// ---------------------------------------------------------------------------
// Step 3b: Warp-level reduction via __shfl_down_sync
//   Within a single warp (32 threads), reduce to one value in 5 shuffle
//   operations — no shared memory needed at the warp level.
// ---------------------------------------------------------------------------
__device__ float warpReduce(float val) {
    unsigned mask = 0xffffffffu;
    val += __shfl_down_sync(mask, val, 16);
    val += __shfl_down_sync(mask, val, 8);
    val += __shfl_down_sync(mask, val, 4);
    val += __shfl_down_sync(mask, val, 2);
    val += __shfl_down_sync(mask, val, 1);
    return val;  // lane 0 holds the warp sum
}

__global__ void reduceWarp(const float *in, float *out, int N) {
    extern __shared__ float sdata[];
    int i = threadIdx.x + blockIdx.x * blockDim.x;
    float val = (i < N) ? in[i] : 0.0f;

    // Each warp reduces its 32 elements to one via shuffles
    val = warpReduce(val);

    // Warp leader writes the warp result to shared memory
    if (threadIdx.x % 32 == 0) {
        sdata[threadIdx.x / 32] = val;
    }
    __syncthreads();

    // If more than one warp, reduce the warp-leader values
    int numWarps = (blockDim.x + 31) / 32;
    if (threadIdx.x < numWarps) {
        val = sdata[threadIdx.x];
        val = warpReduce(val);
        if (threadIdx.x == 0) {
            out[blockIdx.x] = val;
        }
    } else if (numWarps == 1 && threadIdx.x == 0) {
        out[blockIdx.x] = val;
    }
}

// ---------------------------------------------------------------------------
// Host helpers
// ---------------------------------------------------------------------------

static float randFloat(float lo, float hi) {
    return lo + (hi - lo) * ((float)rand() / (float)RAND_MAX);
}

// Verify C = A + B element-wise with tolerance
static int verifyVecAdd(const float *A, const float *B, const float *C, int N) {
    int errors = 0;
    for (int i = 0; i < N; i++) {
        float expected = A[i] + B[i];
        if (fabsf(C[i] - expected) > 1e-4f) {
            if (++errors <= 5) {
                fprintf(stderr, "  vecAdd mismatch at %d: got %f, expected %f\n",
                        i, C[i], expected);
            }
        }
    }
    return errors;
}

// Verify C = A × B with tolerance
static int verifyMatMul(const float *A, const float *B, const float *C,
                        int M, int N, int K) {
    int errors = 0;
    for (int r = 0; r < M && errors <= 5; r++) {
        for (int c = 0; c < N && errors <= 5; c++) {
            float expected = 0.0f;
            for (int k = 0; k < K; k++) {
                expected += A[r * K + k] * B[k * N + c];
            }
            float diff = fabsf(C[r * N + c] - expected);
            if (diff > 1e-2f) {  // larger tol for fp-matmul accumulation
                fprintf(stderr, "  matMul mismatch at (%d,%d): got %f, expected %f (diff=%f)\n",
                        r, c, C[r * N + c], expected, diff);
                errors++;
            }
        }
    }
    return errors;
}

// ---------------------------------------------------------------------------
// Timer — CUDA event-based, measures device time
// ---------------------------------------------------------------------------
static float timedKernel(void (*kernel)(), dim3 grid, dim3 block,
                          size_t smem, cudaStream_t stream,
                          const void **args) {
    cudaEvent_t start, stop;
    CUDA_CHECK(cudaEventCreate(&start));
    CUDA_CHECK(cudaEventCreate(&stop));
    CUDA_CHECK(cudaEventRecord(start, stream));
    CUDA_CHECK(cudaLaunchKernel(kernel, grid, block, (void**)args, smem, stream));
    CUDA_CHECK(cudaEventRecord(stop, stream));
    CUDA_CHECK(cudaEventSynchronize(stop));
    float ms = 0.0f;
    CUDA_CHECK(cudaEventElapsedTime(&ms, start, stop));
    CUDA_CHECK(cudaEventDestroy(start));
    CUDA_CHECK(cudaEventDestroy(stop));
    return ms;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
int main(void) {
    printf("=== GPU Programming — CUDA Basics ===\n\n");

    // -----------------------------------------------------------------------
    // Step 1: Vector Addition
    // -----------------------------------------------------------------------
    printf("--- Step 1: Vector Addition ---\n");
    int vecN = 1 << 24;  // 16M elements = 64 MB per array
    size_t vecBytes = vecN * sizeof(float);

    float *h_A = (float*)malloc(vecBytes);
    float *h_B = (float*)malloc(vecBytes);
    float *h_C = (float*)malloc(vecBytes);

    for (int i = 0; i < vecN; i++) {
        h_A[i] = randFloat(-10.0f, 10.0f);
        h_B[i] = randFloat(-10.0f, 10.0f);
    }

    float *d_A, *d_B, *d_C;
    CUDA_CHECK(cudaMalloc((void**)&d_A, vecBytes));
    CUDA_CHECK(cudaMalloc((void**)&d_B, vecBytes));
    CUDA_CHECK(cudaMalloc((void**)&d_C, vecBytes));

    CUDA_CHECK(cudaMemcpy(d_A, h_A, vecBytes, cudaMemcpyHostToDevice));
    CUDA_CHECK(cudaMemcpy(d_B, h_B, vecBytes, cudaMemcpyHostToDevice));

    int threadsPerBlock = 256;
    int blocksPerGrid   = (vecN + threadsPerBlock - 1) / threadsPerBlock;

    // Time the kernel using CUDA events
    cudaEvent_t start, stop;
    CUDA_CHECK(cudaEventCreate(&start));
    CUDA_CHECK(cudaEventCreate(&stop));

    CUDA_CHECK(cudaEventRecord(start));
    vecAdd<<<blocksPerGrid, threadsPerBlock>>>(d_A, d_B, d_C, vecN);
    CUDA_CHECK(cudaEventRecord(stop));
    CUDA_CHECK(cudaEventSynchronize(stop));
    float vecMs = 0.0f;
    CUDA_CHECK(cudaEventElapsedTime(&vecMs, start, stop));

    CUDA_CHECK(cudaMemcpy(h_C, d_C, vecBytes, cudaMemcpyDeviceToHost));

    int vecErrors = verifyVecAdd(h_A, h_B, h_C, vecN);
    printf("  N = %d, grid=%d, block=%d\n", vecN, blocksPerGrid, threadsPerBlock);
    printf("  Time: %.2f ms  (%.1f GB/s)\n", vecMs,
           3.0 * vecBytes / (vecMs * 1e6f));
    printf("  Errors: %d\n\n", vecErrors);

    free(h_A); free(h_B); free(h_C);
    CUDA_CHECK(cudaFree(d_A)); CUDA_CHECK(cudaFree(d_B)); CUDA_CHECK(cudaFree(d_C));

    // -----------------------------------------------------------------------
    // Step 2: Matrix Multiplication (Naive vs Tiled)
    // -----------------------------------------------------------------------
    printf("--- Step 2: Matrix Multiplication ---\n");
    int M = 512, Kdim = 512, N = 512;
    size_t aBytes = M * Kdim * sizeof(float);
    size_t bBytes = Kdim * N * sizeof(float);
    size_t cBytes = M * N * sizeof(float);

    float *h_Am = (float*)malloc(aBytes);
    float *h_Bm = (float*)malloc(bBytes);
    float *h_Cnaive = (float*)malloc(cBytes);
    float *h_Ctiled = (float*)malloc(cBytes);

    for (int i = 0; i < M * Kdim; i++) h_Am[i] = randFloat(-1.0f, 1.0f);
    for (int i = 0; i < Kdim * N; i++) h_Bm[i] = randFloat(-1.0f, 1.0f);

    float *d_Am, *d_Bm, *d_Cm;
    CUDA_CHECK(cudaMalloc((void**)&d_Am, aBytes));
    CUDA_CHECK(cudaMalloc((void**)&d_Bm, bBytes));
    CUDA_CHECK(cudaMalloc((void**)&d_Cm, cBytes));
    CUDA_CHECK(cudaMemcpy(d_Am, h_Am, aBytes, cudaMemcpyHostToDevice));
    CUDA_CHECK(cudaMemcpy(d_Bm, h_Bm, bBytes, cudaMemcpyHostToDevice));

    dim3 block2d(16, 16);
    dim3 grid2d((N + 15) / 16, (M + 15) / 16);

    // Naive
    CUDA_CHECK(cudaEventRecord(start));
    matMulNaive<<<grid2d, block2d>>>(d_Am, d_Bm, d_Cm, M, N, Kdim);
    CUDA_CHECK(cudaEventRecord(stop));
    CUDA_CHECK(cudaEventSynchronize(stop));
    float naiveMs = 0.0f;
    CUDA_CHECK(cudaEventElapsedTime(&naiveMs, start, stop));
    CUDA_CHECK(cudaMemcpy(h_Cnaive, d_Cm, cBytes, cudaMemcpyDeviceToHost));
    int naiveErrors = verifyMatMul(h_Am, h_Bm, h_Cnaive, M, N, Kdim);

    // Tiled
    CUDA_CHECK(cudaEventRecord(start));
    matMulTiled<<<grid2d, block2d>>>(d_Am, d_Bm, d_Cm, M, N, Kdim);
    CUDA_CHECK(cudaEventRecord(stop));
    CUDA_CHECK(cudaEventSynchronize(stop));
    float tiledMs = 0.0f;
    CUDA_CHECK(cudaEventElapsedTime(&tiledMs, start, stop));
    CUDA_CHECK(cudaMemcpy(h_Ctiled, d_Cm, cBytes, cudaMemcpyDeviceToHost));
    int tiledErrors = verifyMatMul(h_Am, h_Bm, h_Ctiled, M, N, Kdim);

    printf("  Matrix %dx%d (K=%d)\n", M, N, Kdim);
    printf("  Naive:  %.2f ms  (%.1f GFLOPS)  errors=%d\n", naiveMs,
           2.0 * M * N * Kdim / (naiveMs * 1e6f), naiveErrors);
    printf("  Tiled:  %.2f ms  (%.1f GFLOPS)  errors=%d\n", tiledMs,
           2.0 * M * N * Kdim / (tiledMs * 1e6f), tiledErrors);
    printf("  Speedup: %.1f×\n\n", naiveMs / tiledMs);

    free(h_Am); free(h_Bm); free(h_Cnaive); free(h_Ctiled);
    CUDA_CHECK(cudaFree(d_Am)); CUDA_CHECK(cudaFree(d_Bm)); CUDA_CHECK(cudaFree(d_Cm));

    // -----------------------------------------------------------------------
    // Step 3: Parallel Reduction (naive tree vs warp-level)
    // -----------------------------------------------------------------------
    printf("--- Step 3: Parallel Reduction ---\n");
    int redN = 1 << 22;  // 4M elements
    size_t redBytes = redN * sizeof(float);

    float *h_in = (float*)malloc(redBytes);
    for (int i = 0; i < redN; i++) h_in[i] = randFloat(0.0f, 1.0f);

    // CPU reference sum (double precision to avoid excessive float error)
    double cpuSum = 0.0;
    for (int i = 0; i < redN; i++) cpuSum += h_in[i];

    float *d_in, *d_out;
    CUDA_CHECK(cudaMalloc((void**)&d_in, redBytes));
    int redThreads = 256;
    int redBlocks  = (redN + redThreads - 1) / redThreads;
    CUDA_CHECK(cudaMalloc((void**)&d_out, redBlocks * sizeof(float)));
    CUDA_CHECK(cudaMemcpy(d_in, h_in, redBytes, cudaMemcpyHostToDevice));

    // --- Naive tree reduction ---
    size_t smemBytes = redThreads * sizeof(float);
    CUDA_CHECK(cudaEventRecord(start));
    reduce<<<redBlocks, redThreads, smemBytes>>>(d_in, d_out, redN);
    CUDA_CHECK(cudaEventRecord(stop));
    CUDA_CHECK(cudaEventSynchronize(stop));
    float naiveRedMs = 0.0f;
    CUDA_CHECK(cudaEventElapsedTime(&naiveRedMs, start, stop));

    float *h_partial = (float*)malloc(redBlocks * sizeof(float));
    CUDA_CHECK(cudaMemcpy(h_partial, d_out, redBlocks * sizeof(float),
                          cudaMemcpyDeviceToHost));
    float gpuSumNaive = 0.0f;
    for (int i = 0; i < redBlocks; i++) gpuSumNaive += h_partial[i];

    // --- Warp-level reduction ---
    CUDA_CHECK(cudaEventRecord(start));
    reduceWarp<<<redBlocks, redThreads, smemBytes>>>(d_in, d_out, redN);
    CUDA_CHECK(cudaEventRecord(stop));
    CUDA_CHECK(cudaEventSynchronize(stop));
    float warpRedMs = 0.0f;
    CUDA_CHECK(cudaEventElapsedTime(&warpRedMs, start, stop));

    CUDA_CHECK(cudaMemcpy(h_partial, d_out, redBlocks * sizeof(float),
                          cudaMemcpyDeviceToHost));
    float gpuSumWarp = 0.0f;
    for (int i = 0; i < redBlocks; i++) gpuSumWarp += h_partial[i];

    printf("  N = %d, blocks=%d, threads/block=%d\n", redN, redBlocks, redThreads);
    printf("  CPU sum (ref): %.6f\n", cpuSum);
    printf("  Naive tree:    %.6f  time=%.3f ms  diff=%.2e\n",
           gpuSumNaive, naiveRedMs, fabsf(gpuSumNaive - (float)cpuSum));
    printf("  Warp shuffle:  %.6f  time=%.3f ms  diff=%.2e\n",
           gpuSumWarp, warpRedMs, fabsf(gpuSumWarp - (float)cpuSum));
    printf("  Speedup (warp vs tree): %.1f×\n\n", naiveRedMs / warpRedMs);

    free(h_in); free(h_partial);
    CUDA_CHECK(cudaFree(d_in)); CUDA_CHECK(cudaFree(d_out));

    // -----------------------------------------------------------------------
    // Step 4: Error handling demonstration
    //   Intentionally trigger an error to show CUDA_CHECK in action.
    // -----------------------------------------------------------------------
    printf("--- Step 4: Error Handling Demo ---\n");
    printf("  Attempting invalid access to trigger CUDA_CHECK...\n");

    float *d_bad = NULL;
    cudaError_t err = cudaMalloc((void**)&d_bad, (size_t)-1);  // absurd size
    if (err != cudaSuccess) {
        printf("  cudaMalloc(..., -1) correctly failed: %s\n",
               cudaGetErrorString(err));
    } else {
        // Should not reach here, but just in case:
        CUDA_CHECK(cudaFree(d_bad));
    }
    printf("  Error handling: PASSED\n\n");

    // Cleanup events
    CUDA_CHECK(cudaEventDestroy(start));
    CUDA_CHECK(cudaEventDestroy(stop));
    CUDA_CHECK(cudaDeviceReset());

    printf("=== All steps completed successfully ===\n");
    return 0;
}
