#include <cstdio>
#include <cstdlib>
#include <cmath>

#define BLOCK_SIZE 256

__global__ void reduce_sum(float* data, float* result, int n) {
    __shared__ float sdata[BLOCK_SIZE];
    int tid = threadIdx.x;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    sdata[tid] = (i < n) ? data[i] : 0.0f;
    __syncthreads();
    for (int stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (tid < stride) {
            sdata[tid] += sdata[tid + stride];
        }
        __syncthreads();
    }
    if (tid == 0) {
        atomicAdd(result, sdata[0]);
    }
}

__global__ void reduce_max(float* data, float* result, int n) {
    __shared__ float sdata[BLOCK_SIZE];
    int tid = threadIdx.x;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    sdata[tid] = (i < n) ? data[i] : -INFINITY;
    __syncthreads();
    for (int stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (tid < stride) {
            sdata[tid] = fmaxf(sdata[tid], sdata[tid + stride]);
        }
        __syncthreads();
    }
    if (tid == 0) {
        atomicAdd(result, sdata[0]);
    }
}

__global__ void blelloch_scan(float* input, float* output, int n) {
    __shared__ float temp[BLOCK_SIZE * 2];
    int tid = threadIdx.x;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    int offset = 1;
    int ai = tid;
    int bi = tid + BLOCK_SIZE / 2;
    if (ai < n) {
        temp[ai] = input[ai];
    } else {
        temp[ai] = 0.0f;
    }
    if (bi < n) {
        temp[bi] = input[bi];
    } else {
        temp[bi] = 0.0f;
    }
    __syncthreads();
    for (int d = BLOCK_SIZE >> 1; d > 0; d >>= 1) {
        if (tid < d) {
            int a = offset * (2 * tid + 1) - 1;
            int b = offset * (2 * tid + 2) - 1;
            temp[b] += temp[a];
        }
        offset *= 2;
        __syncthreads();
    }
    if (tid == 0) {
        temp[BLOCK_SIZE - 1] = 0.0f;
    }
    __syncthreads();
    for (int d = 1; d < BLOCK_SIZE; d *= 2) {
        offset >>= 1;
        if (tid < d) {
            int a = offset * (2 * tid + 1) - 1;
            int b = offset * (2 * tid + 2) - 1;
            float t = temp[a];
            temp[a] = temp[b];
            temp[b] += t;
        }
        __syncthreads();
    }
    if (ai < n) {
        output[ai] = temp[ai];
    }
    if (bi < n) {
        output[bi] = temp[bi];
    }
}

void verify_reduce_sum(float* data, int n) {
    float expected = 0.0f;
    for (int i = 0; i < n; i++) {
        expected += data[i];
    }
    printf("  CPU expected sum: %.1f\n", expected);
}

void verify_reduce_max(float* data, int n) {
    float expected = data[0];
    for (int i = 1; i < n; i++) {
        if (data[i] > expected) expected = data[i];
    }
    printf("  CPU expected max: %.1f\n", expected);
}

void verify_prefix_sum(float* data, int n) {
    printf("  CPU prefix sum: [");
    float running = 0.0f;
    for (int i = 0; i < n && i < 10; i++) {
        printf("%.0f", running);
        running += data[i];
        if (i < n - 1 && i < 9) printf(", ");
    }
    if (n > 10) printf(" ...");
    printf("]\n");
}

int main() {
    int n = 1024;
    size_t bytes = n * sizeof(float);
    float* h_data = (float*)malloc(bytes);
    for (int i = 0; i < n; i++) {
        h_data[i] = (float)(i + 1);
    }
    float* d_data = nullptr;
    float* d_result = nullptr;
    float* d_scan_out = nullptr;
    cudaMalloc(&d_data, bytes);
    cudaMalloc(&d_result, sizeof(float));
    cudaMalloc(&d_scan_out, bytes);
    cudaMemcpy(d_data, h_data, bytes, cudaMemcpyHostToDevice);
    printf("=== Parallel Reduction (Sum) ===\n");
    cudaMemset(d_result, 0, sizeof(float));
    int num_blocks = (n + BLOCK_SIZE - 1) / BLOCK_SIZE;
    reduce_sum<<<num_blocks, BLOCK_SIZE>>>(d_data, d_result, n);
    cudaDeviceSynchronize();
    float gpu_sum = 0.0f;
    cudaMemcpy(&gpu_sum, d_result, sizeof(float), cudaMemcpyDeviceToHost);
    printf("  GPU sum: %.1f\n", gpu_sum);
    verify_reduce_sum(h_data, n);
    printf("\n=== Parallel Reduction (Max) ===\n");
    float h_neg_inf = -INFINITY;
    cudaMemcpy(d_result, &h_neg_inf, sizeof(float), cudaMemcpyHostToDevice);
    reduce_max<<<num_blocks, BLOCK_SIZE>>>(d_data, d_result, n);
    cudaDeviceSynchronize();
    float gpu_max = 0.0f;
    cudaMemcpy(&gpu_max, d_result, sizeof(float), cudaMemcpyDeviceToHost);
    printf("  GPU max: %.1f\n", gpu_max);
    verify_reduce_max(h_data, n);
    printf("\n=== Blelloch Prefix Sum (Exclusive Scan) ===\n");
    blelloch_scan<<<1, BLOCK_SIZE / 2>>>(d_data, d_scan_out, n);
    cudaDeviceSynchronize();
    float* h_scan = (float*)malloc(bytes);
    cudaMemcpy(h_scan, d_scan_out, bytes, cudaMemcpyDeviceToHost);
    printf("  GPU prefix sum: [");
    for (int i = 0; i < 10 && i < n; i++) {
        printf("%.0f", h_scan[i]);
        if (i < 9 && i < n - 1) printf(", ");
    }
    printf(" ...]\n");
    verify_prefix_sum(h_data, n);
    bool scan_correct = true;
    float running = 0.0f;
    for (int i = 0; i < n; i++) {
        if (fabsf(h_scan[i] - running) > 0.01f) {
            scan_correct = false;
            printf("  MISMATCH at i=%d: GPU=%.1f CPU=%.1f\n", i, h_scan[i], running);
            break;
        }
        running += h_data[i];
    }
    if (scan_correct) printf("  Scan result: CORRECT\n");
    free(h_data);
    free(h_scan);
    cudaFree(d_data);
    cudaFree(d_result);
    cudaFree(d_scan_out);
    printf("\nAll computations complete.\n");
    return 0;
}