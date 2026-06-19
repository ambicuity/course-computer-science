// GPU Architecture — SIMT, Warps, Memory Hierarchy
// CUDA kernel collection: vector add, tiled matmul, reduction, histogram
// Compile: nvcc -o gpu_kernels main.cu
// Requires: CUDA-capable GPU, CUDA toolkit

#include <cstdio>
#include <cstdlib>
#include <cmath>
#include <cstring>

#define CHECK_CUDA(call) do { \
    cudaError_t err = call; \
    if (err != cudaSuccess) { \
        fprintf(stderr, "CUDA error at %s:%d: %s\n", __FILE__, __LINE__, cudaGetErrorString(err)); \
        exit(1); \
    } \
} while(0)

// ============================================================
// Kernel 1: Vector Addition
// Each thread adds one element. Demonstrates coalesced access.
// ============================================================

__global__ void vector_add(const float *A, const float *B, float *C, int n) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n)
        C[idx] = A[idx] + B[idx];
}

// ============================================================
// Kernel 2: Tiled Matrix Multiply (C = A * B)
// Uses shared memory to reduce global memory traffic.
// Each tile reduces global loads by a factor of TILE.
// ============================================================

#define TILE 16

__global__ void matmul_tiled(const float *A, const float *B, float *C, int N) {
    __shared__ float As[TILE][TILE];
    __shared__ float Bs[TILE][TILE];

    int row = blockIdx.y * TILE + threadIdx.y;
    int col = blockIdx.x * TILE + threadIdx.x;
    float sum = 0.0f;

    for (int t = 0; t < (N + TILE - 1) / TILE; t++) {
        int aCol = t * TILE + threadIdx.x;
        int bRow = t * TILE + threadIdx.y;

        As[threadIdx.y][threadIdx.x] = (row < N && aCol < N) ? A[row * N + aCol] : 0.0f;
        Bs[threadIdx.y][threadIdx.x] = (bRow < N && col < N) ? B[bRow * N + col] : 0.0f;
        __syncthreads();

        for (int k = 0; k < TILE; k++)
            sum += As[threadIdx.y][k] * Bs[k][threadIdx.x];
        __syncthreads();
    }

    if (row < N && col < N)
        C[row * N + col] = sum;
}

// ============================================================
// Kernel 3: Parallel Sum Reduction
// Each block reduces its chunk to one value using shared memory.
// Demonstrates tree reduction and warp divergence avoidance.
// ============================================================

__global__ void reduce_sum(const float *input, float *output, int n) {
    __shared__ float sdata[256];
    int tid = threadIdx.x;
    int idx = blockIdx.x * blockDim.x * 2 + threadIdx.x;

    // Load two elements per thread to increase arithmetic intensity
    sdata[tid] = 0.0f;
    if (idx < n) sdata[tid] += input[idx];
    if (idx + blockDim.x < n) sdata[tid] += input[idx + blockDim.x];
    __syncthreads();

    // Tree reduction — each step halves active threads
    for (int s = blockDim.x / 2; s > 0; s >>= 1) {
        if (tid < s)
            sdata[tid] += sdata[tid + s];
        __syncthreads();
    }

    if (tid == 0)
        output[blockIdx.x] = sdata[0];
}

// ============================================================
// Kernel 4: Histogram
// Counts occurrences of each byte value (0–255).
// Uses per-block shared histogram to reduce global atomic contention.
// ============================================================

__global__ void histogram(const unsigned char *data, int *hist, int n) {
    __shared__ int shist[256];
    int tid = threadIdx.x;

    // Initialize shared histogram
    if (tid < 256) shist[tid] = 0;
    __syncthreads();

    // Coalesced access: each thread processes elements at stride = blockDim * gridDim
    int idx = blockIdx.x * blockDim.x + tid;
    int stride = blockDim.x * gridDim.x;
    for (int i = idx; i < n; i += stride)
        atomicAdd(&shist[data[i]], 1);
    __syncthreads();

    // Merge per-block histogram into global
    if (tid < 256)
        atomicAdd(&hist[tid], shist[tid]);
}

// ============================================================
// Host test harness
// ============================================================

void test_vector_add() {
    const int n = 1 << 20; // 1M elements
    size_t bytes = n * sizeof(float);

    float *hA = (float*)malloc(bytes);
    float *hB = (float*)malloc(bytes);
    float *hC = (float*)malloc(bytes);

    for (int i = 0; i < n; i++) {
        hA[i] = i * 1.0f;
        hB[i] = i * 2.0f;
    }

    float *dA, *dB, *dC;
    CHECK_CUDA(cudaMalloc(&dA, bytes));
    CHECK_CUDA(cudaMalloc(&dB, bytes));
    CHECK_CUDA(cudaMalloc(&dC, bytes));
    CHECK_CUDA(cudaMemcpy(dA, hA, bytes, cudaMemcpyHostToDevice));
    CHECK_CUDA(cudaMemcpy(dB, hB, bytes, cudaMemcpyHostToDevice));

    int threads = 256;
    int blocks = (n + threads - 1) / threads;
    vector_add<<<blocks, threads>>>(dA, dB, dC, n);
    CHECK_CUDA(cudaGetLastError());
    CHECK_CUDA(cudaDeviceSynchronize());
    CHECK_CUDA(cudaMemcpy(hC, dC, bytes, cudaMemcpyDeviceToHost));

    int errors = 0;
    for (int i = 0; i < n; i++) {
        float expected = hA[i] + hB[i];
        if (fabsf(hC[i] - expected) > 1e-5f) errors++;
    }
    printf("vector_add (n=%d): %s (%d errors)\n", n, errors == 0 ? "PASS" : "FAIL", errors);

    cudaFree(dA); cudaFree(dB); cudaFree(dC);
    free(hA); free(hB); free(hC);
}

void test_matmul() {
    const int N = 256;
    size_t bytes = N * N * sizeof(float);

    float *hA = (float*)malloc(bytes);
    float *hB = (float*)malloc(bytes);
    float *hC = (float*)malloc(bytes);
    float *hRef = (float*)malloc(bytes);

    srand(42);
    for (int i = 0; i < N * N; i++) {
        hA[i] = (float)(rand() % 10) / 10.0f;
        hB[i] = (float)(rand() % 10) / 10.0f;
    }

    // CPU reference
    for (int i = 0; i < N; i++)
        for (int j = 0; j < N; j++) {
            float s = 0.0f;
            for (int k = 0; k < N; k++) s += hA[i * N + k] * hB[k * N + j];
            hRef[i * N + j] = s;
        }

    float *dA, *dB, *dC;
    CHECK_CUDA(cudaMalloc(&dA, bytes));
    CHECK_CUDA(cudaMalloc(&dB, bytes));
    CHECK_CUDA(cudaMalloc(&dC, bytes));
    CHECK_CUDA(cudaMemcpy(dA, hA, bytes, cudaMemcpyHostToDevice));
    CHECK_CUDA(cudaMemcpy(dB, hB, bytes, cudaMemcpyHostToDevice));

    dim3 threads(TILE, TILE);
    dim3 blocks((N + TILE - 1) / TILE, (N + TILE - 1) / TILE);
    matmul_tiled<<<blocks, threads>>>(dA, dB, dC, N);
    CHECK_CUDA(cudaGetLastError());
    CHECK_CUDA(cudaDeviceSynchronize());
    CHECK_CUDA(cudaMemcpy(hC, dC, bytes, cudaMemcpyDeviceToHost));

    int errors = 0;
    for (int i = 0; i < N * N; i++) {
        if (fabsf(hC[i] - hRef[i]) > 1e-3f) errors++;
    }
    printf("matmul_tiled %dx%d: %s (%d errors)\n", N, N, errors == 0 ? "PASS" : "FAIL", errors);

    cudaFree(dA); cudaFree(dB); cudaFree(dC);
    free(hA); free(hB); free(hC); free(hRef);
}

void test_reduce() {
    const int n = 1 << 20;
    size_t bytes = n * sizeof(float);

    float *hIn = (float*)malloc(bytes);
    for (int i = 0; i < n; i++) hIn[i] = 1.0f;

    float *dIn, *dOut;
    CHECK_CUDA(cudaMalloc(&dIn, bytes));

    int threads = 256;
    int blocks = (n + threads * 2 - 1) / (threads * 2);
    CHECK_CUDA(cudaMalloc(&dOut, blocks * sizeof(float)));
    CHECK_CUDA(cudaMemcpy(dIn, hIn, bytes, cudaMemcpyHostToDevice));

    reduce_sum<<<blocks, threads>>>(dIn, dOut, n);
    CHECK_CUDA(cudaGetLastError());
    CHECK_CUDA(cudaDeviceSynchronize());

    float *hOut = (float*)malloc(blocks * sizeof(float));
    CHECK_CUDA(cudaMemcpy(hOut, dOut, blocks * sizeof(float), cudaMemcpyDeviceToHost));

    float total = 0.0f;
    for (int i = 0; i < blocks; i++) total += hOut[i];
    printf("reduce_sum (n=%d): %s (got %.1f, expected %.1f)\n",
           n, fabsf(total - n) < 1.0f ? "PASS" : "FAIL", total, (float)n);

    cudaFree(dIn); cudaFree(dOut);
    free(hIn); free(hOut);
}

void test_histogram() {
    const int n = 1 << 20;
    size_t bytes = n * sizeof(unsigned char);

    unsigned char *hData = (unsigned char*)malloc(bytes);
    int hHist[256] = {0};

    srand(42);
    for (int i = 0; i < n; i++) {
        hData[i] = (unsigned char)(rand() % 256);
        hHist[hData[i]]++;
    }

    unsigned char *dData;
    int *dHist;
    CHECK_CUDA(cudaMalloc(&dData, bytes));
    CHECK_CUDA(cudaMalloc(&dHist, 256 * sizeof(int)));
    CHECK_CUDA(cudaMemcpy(dData, hData, bytes, cudaMemcpyHostToDevice));
    CHECK_CUDA(cudaMemset(dHist, 0, 256 * sizeof(int)));

    int threads = 256;
    int blocks = 64; // enough blocks to cover the data
    histogram<<<blocks, threads>>>(dData, dHist, n);
    CHECK_CUDA(cudaGetLastError());
    CHECK_CUDA(cudaDeviceSynchronize());

    int gHist[256];
    CHECK_CUDA(cudaMemcpy(gHist, dHist, 256 * sizeof(int), cudaMemcpyDeviceToHost));

    int errors = 0;
    for (int i = 0; i < 256; i++) {
        if (gHist[i] != hHist[i]) errors++;
    }
    printf("histogram (n=%d): %s (%d bins wrong)\n", n, errors == 0 ? "PASS" : "FAIL", errors);

    cudaFree(dData); cudaFree(dHist);
    free(hData);
}

int main() {
    printf("=== GPU Architecture — CUDA Kernel Tests ===\n\n");

    test_vector_add();
    test_matmul();
    test_reduce();
    test_histogram();

    printf("\nDone.\n");
    return 0;
}
