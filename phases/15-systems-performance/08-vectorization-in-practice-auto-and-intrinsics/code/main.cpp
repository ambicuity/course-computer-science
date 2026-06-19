#include <immintrin.h>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <numeric>
#include <vector>

#ifdef _MSC_VER
#define ALIGNED_ALLOC(align, size) _aligned_malloc(size, align)
#define ALIGNED_FREE(ptr) _aligned_free(ptr)
#else
#define ALIGNED_ALLOC(align, size) std::aligned_alloc(align, size)
#define ALIGNED_FREE(ptr) free(ptr)
#endif

static constexpr int N = 1024 * 1024;
static constexpr int ITERS = 200;

static float* alloc_aligned(int count, int alignment = 64) {
    size_t bytes = static_cast<size_t>(count) * sizeof(float);
    float* ptr = static_cast<float*>(ALIGNED_ALLOC(alignment, bytes));
    for (int i = 0; i < count; i++) ptr[i] = 0.0f;
    return ptr;
}

static void fill_random(float* data, int n) {
    for (int i = 0; i < n; i++) {
        data[i] = static_cast<float>(rand()) / static_cast<float>(RAND_MAX) - 0.5f;
    }
}

// ─── Scalar implementations ───

float dot_scalar(const float* a, const float* b, int n) {
    float sum = 0.0f;
    for (int i = 0; i < n; i++) {
        sum += a[i] * b[i];
    }
    return sum;
}

float sum_scalar(const float* data, int n) {
    float s = 0.0f;
    for (int i = 0; i < n; i++) {
        s += data[i];
    }
    return s;
}

int filter_scalar(const float* src, float* dst, int n, float threshold) {
    int count = 0;
    for (int i = 0; i < n; i++) {
        if (src[i] > threshold) {
            dst[count++] = src[i];
        }
    }
    return count;
}

// ─── Auto-vectorizable implementations ───
// __restrict__ tells the compiler a and b don't alias, enabling vectorization.

float dot_auto(const float* __restrict__ a,
               const float* __restrict__ b, int n) {
    float sum = 0.0f;
    for (int i = 0; i < n; i++) {
        sum += a[i] * b[i];
    }
    return sum;
}

float sum_auto(const float* __restrict__ data, int n) {
    float s = 0.0f;
    for (int i = 0; i < n; i++) {
        s += data[i];
    }
    return s;
}

// ─── SSE intrinsics (128-bit, 4 × float32) ───

float dot_sse(const float* a, const float* b, int n) {
    __m128 sum_vec = _mm_setzero_ps();
    int i = 0;
    for (; i + 3 < n; i += 4) {
        __m128 va = _mm_loadu_ps(a + i);
        __m128 vb = _mm_loadu_ps(b + i);
        sum_vec = _mm_add_ps(sum_vec, _mm_mul_ps(va, vb));
    }
    float tmp[4];
    _mm_storeu_ps(tmp, sum_vec);
    float sum = tmp[0] + tmp[1] + tmp[2] + tmp[3];
    for (; i < n; i++) sum += a[i] * b[i];
    return sum;
}

float sum_sse(const float* data, int n) {
    __m128 sum_vec = _mm_setzero_ps();
    int i = 0;
    for (; i + 3 < n; i += 4) {
        __m128 v = _mm_loadu_ps(data + i);
        sum_vec = _mm_add_ps(sum_vec, v);
    }
    float tmp[4];
    _mm_storeu_ps(tmp, sum_vec);
    float sum = tmp[0] + tmp[1] + tmp[2] + tmp[3];
    for (; i < n; i++) sum += data[i];
    return sum;
}

// ─── AVX intrinsics (256-bit, 8 × float32) ───

float dot_avx(const float* a, const float* b, int n) {
    __m256 sum_vec = _mm256_setzero_ps();
    int i = 0;
    for (; i + 7 < n; i += 8) {
        __m256 va = _mm256_loadu_ps(a + i);
        __m256 vb = _mm256_loadu_ps(b + i);
        sum_vec = _mm256_add_ps(sum_vec, _mm256_mul_ps(va, vb));
    }
    float tmp[8];
    _mm256_storeu_ps(tmp, sum_vec);
    float sum = 0.0f;
    for (int j = 0; j < 8; j++) sum += tmp[j];
    for (; i < n; i++) sum += a[i] * b[i];
    return sum;
}

float sum_avx(const float* data, int n) {
    __m256 sum_vec = _mm256_setzero_ps();
    int i = 0;
    for (; i + 7 < n; i += 8) {
        __m256 v = _mm256_loadu_ps(data + i);
        sum_vec = _mm256_add_ps(sum_vec, v);
    }
    float tmp[8];
    _mm256_storeu_ps(tmp, sum_vec);
    float sum = 0.0f;
    for (int j = 0; j < 8; j++) sum += tmp[j];
    for (; i < n; i++) sum += data[i];
    return sum;
}

// ─── AVX2 filter: copy src[i] > threshold to dst, return count ───
// Uses AVX2 compare + masked store (compact).

int filter_avx2(const float* src, float* dst, int n, float threshold) {
    __m256 thresh = _mm256_set1_ps(threshold);
    int count = 0;
    int i = 0;
    for (; i + 7 < n; i += 8) {
        __m256 v = _mm256_loadu_ps(src + i);
        __m256 cmp = _mm256_cmp_ps(v, thresh, _CMP_GT_OS);
        unsigned mask = _mm256_movemask_ps(cmp);
        while (mask) {
            unsigned bit = __builtin_ctz(mask);
            dst[count++] = src[i + bit];
            mask &= mask - 1;
        }
    }
    for (; i < n; i++) {
        if (src[i] > threshold) dst[count++] = src[i];
    }
    return count;
}

// ─── AVX-512 dot product (if supported at runtime) ───

#ifdef __AVX512F__
float dot_avx512(const float* a, const float* b, int n) {
    __m512 sum_vec = _mm512_setzero_ps();
    int i = 0;
    for (; i + 15 < n; i += 16) {
        __m512 va = _mm512_loadu_ps(a + i);
        __m512 vb = _mm512_loadu_ps(b + i);
        sum_vec = _mm512_fmadd_ps(va, vb, sum_vec);
    }
    float sum = _mm512_reduce_add_ps(sum_vec);
    for (; i < n; i++) sum += a[i] * b[i];
    return sum;
}
#endif

// ─── Alignment benchmark: aligned vs unaligned load ───

float sum_aligned_load(const float* data, int n) {
    __m256 sum_vec = _mm256_setzero_ps();
    int i = 0;
    for (; i + 7 < n; i += 8) {
        __m256 v = _mm256_load_ps(data + i);
        sum_vec = _mm256_add_ps(sum_vec, v);
    }
    float tmp[8];
    _mm256_storeu_ps(tmp, sum_vec);
    float sum = 0.0f;
    for (int j = 0; j < 8; j++) sum += tmp[j];
    for (; i < n; i++) sum += data[i];
    return sum;
}

// ─── Timing helper ───

template <typename F>
static double bench(const char* label, F func, int iters = ITERS) {
    auto start = std::chrono::high_resolution_clock::now();
    volatile float sink = 0.0f;
    for (int i = 0; i < iters; i++) {
        sink = func();
    }
    (void)sink;
    auto end = std::chrono::high_resolution_clock::now();
    double us = std::chrono::duration<double, std::micro>(end - start).count() / iters;
    std::cout << label << ": " << us << " us/iter" << std::endl;
    return us;
}

int main() {
    std::cout << "=== Vectorization Benchmark ===" << std::endl;
    std::cout << "Array size: " << N << " floats (" << (N * sizeof(float) / 1024) << " KB)" << std::endl;
    std::cout << std::endl;

    float* a = alloc_aligned(N);
    float* b = alloc_aligned(N);
    float* dst = alloc_aligned(N);

    fill_random(a, N);
    fill_random(b, N);

    // ─── Dot Product Benchmarks ───
    std::cout << "--- Dot Product ---" << std::endl;

    float ref = dot_scalar(a, b, N);

    bench("scalar   ", [&]() { return dot_scalar(a, b, N); });
    bench("auto-vec ", [&]() { return dot_auto(a, b, N); });
    bench("SSE      ", [&]() { return dot_sse(a, b, N); });
    bench("AVX      ", [&]() { return dot_avx(a, b, N); });
#ifdef __AVX512F__
    bench("AVX-512  ", [&]() { return dot_avx512(a, b, N); });
#else
    std::cout << "AVX-512  : (not compiled, use -mavx512f)" << std::endl;
#endif

    // ─── Sum Benchmarks ───
    std::cout << std::endl << "--- Sum ---" << std::endl;

    bench("scalar   ", [&]() { return sum_scalar(a, N); });
    bench("auto-vec ", [&]() { return sum_auto(a, N); });
    bench("SSE      ", [&]() { return sum_sse(a, N); });
    bench("AVX      ", [&]() { return sum_avx(a, N); });
    bench("aligned  ", [&]() { return sum_aligned_load(a, N); });

    // ─── Filter Benchmark ───
    std::cout << std::endl << "--- Filter (> 0.0) ---" << std::endl;
    float threshold = 0.0f;

    int cnt_s = 0;
    auto t1 = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < ITERS; i++) {
        cnt_s = filter_scalar(a, dst, N, threshold);
    }
    auto t2 = std::chrono::high_resolution_clock::now();
    double filter_scalar_us = std::chrono::duration<double, std::micro>(t2 - t1).count() / ITERS;
    std::cout << "scalar   : " << filter_scalar_us << " us/iter (" << cnt_s << " kept)" << std::endl;

    int cnt_v = 0;
    auto t3 = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < ITERS; i++) {
        cnt_v = filter_avx2(a, dst, N, threshold);
    }
    auto t4 = std::chrono::high_resolution_clock::now();
    double filter_avx2_us = std::chrono::duration<double, std::micro>(t4 - t3).count() / ITERS;
    std::cout << "AVX2     : " << filter_avx2_us << " us/iter (" << cnt_v << " kept)" << std::endl;

    // ─── Verification ───
    std::cout << std::endl << "--- Verification ---" << std::endl;
    float d_auto = dot_auto(a, b, N);
    float d_sse  = dot_sse(a, b, N);
    float d_avx  = dot_avx(a, b, N);

    bool dot_ok = true;
    if (std::fabs(d_auto - ref) > 1e-3f) { std::cout << "FAIL: auto != scalar" << std::endl; dot_ok = false; }
    if (std::fabs(d_sse - ref) > 1e-3f)  { std::cout << "FAIL: SSE != scalar" << std::endl; dot_ok = false; }
    if (std::fabs(d_avx - ref) > 1e-3f)  { std::cout << "FAIL: AVX != scalar" << std::endl; dot_ok = false; }
    if (dot_ok) std::cout << "All dot product results match (within 1e-3)." << std::endl;

    float s_auto = sum_auto(a, N);
    float s_avx  = sum_avx(a, N);
    if (std::fabs(s_auto - sum_scalar(a, N)) < 1e-2f && std::fabs(s_avx - sum_scalar(a, N)) < 1e-2f) {
        std::cout << "All sum results match (within 1e-2)." << std::endl;
    } else {
        std::cout << "FAIL: sum results differ." << std::endl;
    }

    if (cnt_s == cnt_v) {
        std::cout << "Filter counts match: " << cnt_s << " elements." << std::endl;
    } else {
        std::cout << "FAIL: filter counts differ: scalar=" << cnt_s << " avx2=" << cnt_v << std::endl;
    }

    ALIGNED_FREE(a);
    ALIGNED_FREE(b);
    ALIGNED_FREE(dst);

    return 0;
}