#include <iostream>
#include <vector>
#include <chrono>
#include <cmath>
#include <algorithm>
#include <iomanip>
#include <cstring>

#ifdef __linux__
#include <perf_event.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <asm/unistd.h>
#include <sys/ioctl.h>
#endif

static constexpr int TILE_SIZE = 32;

static void mat_mul_naive(const double* A, const double* B, double* C, int N) {
    for (int i = 0; i < N; i++)
        for (int j = 0; j < N; j++) {
            double sum = 0.0;
            for (int k = 0; k < N; k++)
                sum += A[i * N + k] * B[k * N + j];
            C[i * N + j] = sum;
        }
}

static void mat_mul_tiled(const double* A, const double* B, double* C, int N) {
    std::memset(C, 0, sizeof(double) * N * N);
    for (int i = 0; i < N; i += TILE_SIZE)
        for (int j = 0; j < N; j += TILE_SIZE)
            for (int k = 0; k < N; k += TILE_SIZE)
                for (int ii = i; ii < std::min(i + TILE_SIZE, N); ii++)
                    for (int jj = j; jj < std::min(j + TILE_SIZE, N); jj++) {
                        double sum = C[ii * N + jj];
                        for (int kk = k; kk < std::min(k + TILE_SIZE, N); kk++)
                            sum += A[ii * N + kk] * B[kk * N + jj];
                        C[ii * N + jj] = sum;
                    }
}

static void mat_mul_recursive(const double* A, const double* B, double* C,
                              int N, int ldA, int ldB, int ldC, int orig_N) {
    if (N <= 64) {
        for (int i = 0; i < N; i++)
            for (int j = 0; j < N; j++) {
                double sum = C[i * ldC + j];
                for (int k = 0; k < N; k++)
                    sum += A[i * ldA + k] * B[k * ldB + j];
                C[i * ldC + j] = sum;
            }
        return;
    }
    int h = N / 2;
    mat_mul_recursive(A, B, C, h, ldA, ldB, ldC, orig_N);
    mat_mul_recursive(A + h, B + h * ldB, C, h, ldA, ldB, ldC, orig_N);
    mat_mul_recursive(A, B + h, C + h, h, ldA, ldB, ldC, orig_N);
    mat_mul_recursive(A + h, B + h * ldB + h, C + h, h, ldA, ldB, ldC, orig_N);
    mat_mul_recursive(A + h * ldA, B, C + h * ldC, h, ldA, ldB, ldC, orig_N);
    mat_mul_recursive(A + h * ldA + h, B + h * ldB, C + h * ldC, h, ldA, ldB, ldC, orig_N);
    mat_mul_recursive(A + h * ldA, B + h, C + h * ldC + h, h, ldA, ldB, ldC, orig_N);
    mat_mul_recursive(A + h * ldA + h, B + h * ldB + h, C + h * ldC + h, h, ldA, ldB, ldC, orig_N);
}

static void mat_mul_cache_oblivious(const double* A, const double* B, double* C, int N) {
    std::memset(C, 0, sizeof(double) * N * N);
    mat_mul_recursive(A, B, C, N, N, N, N, N);
}

static double checksum(const double* M, int N) {
    double s = 0.0;
    for (int i = 0; i < N * N; i++)
        s += M[i];
    return s;
}

static void init_matrix(double* M, int N) {
    for (int i = 0; i < N * N; i++)
        M[i] = static_cast<double>(i % 7 - 3) / 7.0;
}

#ifdef __linux__
static long long read_perf_event(int fd) {
    long long count = 0;
    read(fd, &count, sizeof(count));
    return count;
}

struct PerfCounter {
    int fd;
    PerfCounter() : fd(-1) {}
    bool setup(uint64_t type, uint64_t config) {
        struct perf_event_attr pe;
        memset(&pe, 0, sizeof(pe));
        pe.type = type;
        pe.size = sizeof(pe);
        pe.config = config;
        pe.disabled = 1;
        pe.exclude_kernel = 1;
        pe.exclude_hv = 1;
        fd = syscall(__NR_perf_event_open, &pe, 0, -1, -1, 0);
        return fd != -1;
    }
    void enable() { if (fd != -1) ioctl(fd, PERF_EVENT_IOC_ENABLE, 0); }
    void disable() { if (fd != -1) ioctl(fd, PERF_EVENT_IOC_DISABLE, 0); }
    long long read_count() { return fd != -1 ? read_perf_event(fd) : -1; }
    ~PerfCounter() { if (fd != -1) close(fd); }
};
#endif

struct BenchResult {
    double elapsed_ms;
    double checksum;
};

template<typename Fn>
static BenchResult benchmark(Fn fn, const double* A, const double* B, double* C, int N, int repeats = 3) {
    BenchResult best{1e9, 0.0};
    for (int r = 0; r < repeats; r++) {
        auto t0 = std::chrono::high_resolution_clock::now();
        fn(A, B, C, N);
        auto t1 = std::chrono::high_resolution_clock::now();
        double ms = std::chrono::duration<double, std::milli>(t1 - t0).count();
        double cs = checksum(C, N);
        if (ms < best.elapsed_ms) {
            best.elapsed_ms = ms;
            best.checksum = cs;
        }
    }
    return best;
}

int main() {
    std::cout << "=== Cache-Aware Algorithm Design: Matrix Multiply Benchmarks ===\n\n";

    const int N = 512;
    std::cout << "Matrix size: " << N << "x" << N << " (" << N * N * sizeof(double) / 1024
              << " KB per matrix)\n";
    std::cout << "L1 cache:     32 KB   (~4 cycles)\n";
    std::cout << "L2 cache:     256 KB  (~12 cycles)\n";
    std::cout << "L3 cache:     varies  (~40 cycles)\n";
    std::cout << "DRAM:         varies  (~200+ cycles)\n\n";

    std::vector<double> A(N * N), B(N * N), C(N * N);
    init_matrix(A.data(), N);
    init_matrix(B.data(), N);

    std::cout << std::fixed << std::setprecision(1);

    std::cout << "--- Running naive matrix multiply ---\n";
    auto res_naive = benchmark(mat_mul_naive, A.data(), B.data(), C.data(), N);
    double naive_cs = res_naive.checksum;
    std::cout << "  Time: " << res_naive.elapsed_ms << " ms  checksum: " << naive_cs << "\n\n";

    std::cout << "--- Running tiled matrix multiply (TILE=" << TILE_SIZE << ") ---\n";
    auto res_tiled = benchmark(mat_mul_tiled, A.data(), B.data(), C.data(), N);
    double tiled_cs = res_tiled.checksum;
    std::cout << "  Time: " << res_tiled.elapsed_ms << " ms  checksum: " << tiled_cs << "\n";
    std::cout << "  Speedup vs naive: " << res_naive.elapsed_ms / res_tiled.elapsed_ms << "x\n\n";

    std::cout << "--- Running cache-oblivious matrix multiply ---\n";
    auto res_oblivious = benchmark(mat_mul_cache_oblivious, A.data(), B.data(), C.data(), N);
    double oblivious_cs = res_oblivious.checksum;
    std::cout << "  Time: " << res_oblivious.elapsed_ms << " ms  checksum: " << oblivious_cs << "\n";
    std::cout << "  Speedup vs naive: " << res_naive.elapsed_ms / res_oblivious.elapsed_ms << "x\n\n";

    bool checksums_match = (std::abs(naive_cs - tiled_cs) < 1.0) &&
                           (std::abs(naive_cs - oblivious_cs) < 1.0);
    std::cout << "Checksum validation: " << (checksums_match ? "PASS" : "FAIL") << "\n\n";

    std::cout << "=== Results Summary ===\n";
    std::cout << "  Naive:          " << res_naive.elapsed_ms << " ms  (1.0x baseline)\n";
    std::cout << "  Tiled:          " << res_tiled.elapsed_ms << " ms  ("
              << res_naive.elapsed_ms / res_tiled.elapsed_ms << "x vs naive)\n";
    std::cout << "  Cache-oblivious:" << res_oblivious.elapsed_ms << " ms  ("
              << res_naive.elapsed_ms / res_oblivious.elapsed_ms << "x vs naive)\n\n";

#ifdef __linux__
    {
        std::cout << "--- Cache miss measurement (Linux perf) ---\n";
        PerfCounter l1_misses, ll_misses;
        bool has_perf = true;
        if (!l1_misses.setup(PERF_TYPE_HW_CACHE, PERF_COUNT_HW_CACHE_L1D << 0 |
                             (PERF_COUNT_HW_CACHE_OP_READ << 8) |
                             (PERF_COUNT_HW_CACHE_RESULT_MISS << 16))) {
            has_perf = false;
        }
        if (!ll_misses.setup(PERF_TYPE_HW_CACHE, PERF_COUNT_HW_CACHE_LL << 0 |
                            (PERF_COUNT_HW_CACHE_OP_READ << 8) |
                            (PERF_COUNT_HW_CACHE_RESULT_MISS << 16))) {
            has_perf = false;
        }

        if (has_perf) {
            auto measure = [&](const char* name, auto fn) {
                l1_misses.enable();
                ll_misses.enable();
                fn(A.data(), B.data(), C.data(), N);
                l1_misses.disable();
                ll_misses.disable();
                std::cout << "  " << name << ": L1 misses=" << l1_misses.read_count()
                          << "  LL misses=" << ll_misses.read_count() << "\n";
            };
            measure("Naive  ", mat_mul_naive);
            measure("Tiled  ", mat_mul_tiled);
            measure("Oblivious", mat_mul_cache_oblivious);
        } else {
            std::cout << "  perf_event_open not available (run on Linux with perf access)\n";
        }
        std::cout << "\n";
    }
#else
    std::cout << "Note: Cache miss counters available on Linux only.\n";
    std::cout << "On Linux, run: perf stat -e L1-dcache-load-misses,LLC-load-misses ./main\n\n";
#endif

    std::cout << "=== Key Takeaways ===\n";
    std::cout << "1. Naive matrix multiply is correct but cache-hostile.\n";
    std::cout << "2. Tiled (blocked) multiply keeps working set in L1: 5-30x speedup.\n";
    std::cout << "3. Cache-oblivious recursion auto-tiles without hardcoded sizes.\n";
    std::cout << "4. Same O(n^3) arithmetic, dramatically different wall-clock time.\n";
    std::cout << "5. Cache misses dominate; algorithmic complexity is only half the story.\n";

    return 0;
}