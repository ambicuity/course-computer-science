// Cache Performance — Locality, Blocking, Prefetch
// Phase 06 — Digital Logic & Computer Architecture
//
// Cache experiments in C++:
//   1) Cache line size detector (stride experiment)
//   2) False sharing demonstration with std::thread
//   3) Prefetch hint benchmark
//
// Compile: g++ -O2 -pthread -o cache_experiments main.cpp

#include <atomic>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <thread>
#include <vector>

// ---------------------------------------------------------------
// 1. Cache line size detector
//    Access every Nth byte in a large buffer. When stride matches
//    cache line size, throughput drops sharply because only one
//    useful byte is consumed per cache line fetch.
// ---------------------------------------------------------------
static long detect_cache_line_size() {
    const long buf_size = 1 << 24; // 16 MB
    char *buf = (char *)malloc(buf_size);
    memset(buf, 1, buf_size);

    const int iterations = 100;
    volatile long long sink = 0;

    long best_stride = 4;
    double worst_time = 0;

    for (long stride = 4; stride <= 256; stride *= 2) {
        auto start = std::chrono::high_resolution_clock::now();
        for (int iter = 0; iter < iterations; iter++) {
            for (long i = 0; i < buf_size; i += stride) {
                sink += buf[i];
            }
        }
        auto end = std::chrono::high_resolution_clock::now();
        double ms = std::chrono::duration<double, std::milli>(end - start).count();

        // Compute bytes per ms (throughput)
        double bytes_per_ms = (buf_size / stride * stride) * iterations / ms;
        printf("  stride %4ld: %7.1f ms  (throughput: %6.1f MB/s)\n",
               stride, ms, bytes_per_ms / 1000.0);

        // The stride just past the cache line size causes the worst
        // throughput per *useful* byte because each access pulls a
        // full line but only uses one byte. But the real signal is
        // the sharp cliff between stride 32→64 or 64→128.
    }

    free(buf);
    return best_stride;
}

// ---------------------------------------------------------------
// 2. False sharing demonstration
//    Two threads increment adjacent counters vs. padded counters.
// ---------------------------------------------------------------

struct BadCounter {
    std::atomic<long> a;
    std::atomic<long> b;
};

struct GoodCounter {
    std::atomic<long> a;
    char pad[56]; // pad to 64 bytes
    std::atomic<long> b;
    char pad2[56];
};

static void false_sharing_demo() {
    const long iters = 50'000'000;

    // --- False sharing (adjacent atomics) ---
    BadCounter bad{};
    auto bad_fn = [&]() {
        auto t0 = std::chrono::high_resolution_clock::now();

        std::thread t1([&]() {
            for (long i = 0; i < iters; i++) bad.a.fetch_add(1, std::memory_order_relaxed);
        });
        std::thread t2([&]() {
            for (long i = 0; i < iters; i++) bad.b.fetch_add(1, std::memory_order_relaxed);
        });
        t1.join();
        t2.join();

        auto t1_end = std::chrono::high_resolution_clock::now();
        double ms = std::chrono::duration<double, std::milli>(t1_end - t0).count();
        printf("  False sharing:   %7.1f ms  (a=%ld, b=%ld)\n", ms,
               bad.a.load(), bad.b.load());
    };

    // --- No false sharing (padded counters) ---
    GoodCounter good{};
    auto good_fn = [&]() {
        auto t0 = std::chrono::high_resolution_clock::now();

        std::thread t1([&]() {
            for (long i = 0; i < iters; i++) good.a.fetch_add(1, std::memory_order_relaxed);
        });
        std::thread t2([&]() {
            for (long i = 0; i < iters; i++) good.b.fetch_add(1, std::memory_order_relaxed);
        });
        t1.join();
        t2.join();

        auto t1_end = std::chrono::high_resolution_clock::now();
        double ms = std::chrono::duration<double, std::milli>(t1_end - t0).count();
        printf("  No false sharing: %7.1f ms  (a=%ld, b=%ld)\n", ms,
               good.a.load(), good.b.load());
    };

    bad_fn();
    good_fn();
}

// ---------------------------------------------------------------
// 3. Prefetch hint benchmark
//    Random-chase through a linked list, with and without
//    software prefetch hints.
// ---------------------------------------------------------------

struct Node {
    int data;
    int next_idx;
};

static void prefetch_benchmark() {
    const long num_nodes = 100'000;
    std::vector<Node> list(num_nodes);

    // Create a random permutation for next pointers
    for (long i = 0; i < num_nodes; i++)
        list[i].next_idx = i;
    for (long i = num_nodes - 1; i > 0; i--) {
        long j = rand() % (i + 1);
        int tmp = list[i].next_idx;
        list[i].next_idx = list[j].next_idx;
        list[j].next_idx = tmp;
    }

    const int traversals = 200;
    volatile int sink = 0;

    // Without prefetch
    {
        auto t0 = std::chrono::high_resolution_clock::now();
        for (int t = 0; t < traversals; t++) {
            int idx = 0;
            for (long i = 0; i < num_nodes; i++) {
                sink += list[idx].data;
                idx = list[idx].next_idx;
            }
        }
        auto t1 = std::chrono::high_resolution_clock::now();
        double ms = std::chrono::duration<double, std::milli>(t1 - t0).count();
        printf("  No prefetch:  %7.1f ms\n", ms);
    }

    // With prefetch (look 4 nodes ahead)
    {
        auto t0 = std::chrono::high_resolution_clock::now();
        for (int t = 0; t < traversals; t++) {
            int idx = 0;
            for (long i = 0; i < num_nodes; i++) {
                int idx4 = list[idx].next_idx;
                for (int d = 0; d < 3 && idx4 != 0; d++)
                    idx4 = list[idx4].next_idx;
                __builtin_prefetch(&list[idx4], 0, 1);
                sink += list[idx].data;
                idx = list[idx].next_idx;
            }
        }
        auto t1 = std::chrono::high_resolution_clock::now();
        double ms = std::chrono::duration<double, std::milli>(t1 - t0).count();
        printf("  With prefetch: %7.1f ms\n", ms);
    }
}

// ---------------------------------------------------------------

int main() {
    printf("=== Cache Line Size Detector ===\n");
    detect_cache_line_size();

    printf("\n=== False Sharing Demo ===\n");
    false_sharing_demo();

    printf("\n=== Prefetch Benchmark ===\n");
    prefetch_benchmark();

    return 0;
}
