#include <iostream>
#include <thread>
#include <vector>
#include <atomic>
#include <mutex>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <algorithm>

class SpinLock {
    std::atomic<bool> flag_{false};
public:
    void lock() {
        while (flag_.exchange(true, std::memory_order_acquire)) {
#if defined(__x86_64__) || defined(_M_X64)
            __builtin_ia32_pause();
#endif
        }
    }
    void unlock() {
        flag_.store(false, std::memory_order_release);
    }
};

class BackoffSpinLock {
    std::atomic<bool> flag_{false};
public:
    void lock() {
        int delay = 1;
        while (flag_.exchange(true, std::memory_order_acquire)) {
            for (int i = 0; i < delay; ++i) {
#if defined(__x86_64__) || defined(_M_X64)
                __builtin_ia32_pause();
#endif
            }
            delay = std::min(delay * 2, 1024);
        }
    }
    void unlock() {
        flag_.store(false, std::memory_order_release);
    }
};

class TicketLock {
    std::atomic<uint32_t> next_{0};
    std::atomic<uint32_t> serving_{0};
public:
    uint32_t lock() {
        auto ticket = next_.fetch_add(1, std::memory_order_acquire);
        while (serving_.load(std::memory_order_acquire) != ticket) {
#if defined(__x86_64__) || defined(_M_X64)
            __builtin_ia32_pause();
#endif
        }
        return ticket;
    }
    void unlock() {
        serving_.fetch_add(1, std::memory_order_release);
    }
};

struct Result {
    double elapsed_ms;
    uint64_t final_value;
    uint64_t expected;
};

template<typename LockType>
Result run_locked_benchmark(int num_threads, uint64_t per_thread, LockType& lock, std::atomic<uint64_t>& counter) {
    counter.store(0, std::memory_order_relaxed);
    auto start = std::chrono::high_resolution_clock::now();
    std::vector<std::thread> threads;
    for (int t = 0; t < num_threads; ++t) {
        threads.emplace_back([&lock, &counter, per_thread]() {
            for (uint64_t i = 0; i < per_thread; ++i) {
                std::lock_guard<LockType> lg(lock);
                counter.fetch_add(1, std::memory_order_relaxed);
            }
        });
    }
    for (auto& th : threads) th.join();
    auto end = std::chrono::high_resolution_clock::now();
    double ms = std::chrono::duration<double, std::milli>(end - start).count();
    return {ms, counter.load(), static_cast<uint64_t>(num_threads) * per_thread};
}

Result run_fetch_add_benchmark(int num_threads, uint64_t per_thread) {
    std::atomic<uint64_t> counter{0};
    auto start = std::chrono::high_resolution_clock::now();
    std::vector<std::thread> threads;
    for (int t = 0; t < num_threads; ++t) {
        threads.emplace_back([&counter, per_thread]() {
            for (uint64_t i = 0; i < per_thread; ++i) {
                counter.fetch_add(1, std::memory_order_relaxed);
            }
        });
    }
    for (auto& th : threads) th.join();
    auto end = std::chrono::high_resolution_clock::now();
    double ms = std::chrono::duration<double, std::milli>(end - start).count();
    return {ms, counter.load(), static_cast<uint64_t>(num_threads) * per_thread};
}

void print_header() {
    std::printf("\n%-15s %-8s %-12s %-12s %-8s\n", "Lock Type", "Threads", "Time (ms)", "Mops/s", "Correct");
    std::printf("%-15s %-8s %-12s %-12s %-8s\n", "--------", "------", "---------", "----------", "-------");
}

void print_result(const char* name, int threads, const Result& r) {
    double mops = r.expected / (r.elapsed_ms / 1000.0) / 1e6;
    const char* ok = (r.final_value == r.expected) ? "YES" : "NO";
    std::printf("%-15s %-8d %-12.2f %-12.2f %-8s\n", name, threads, r.elapsed_ms, mops, ok);
}

int main() {
    constexpr uint64_t INCREMENTS = 10'000'000;
    constexpr int thread_counts[] = {1, 2, 4, 8};

    std::printf("=== Lock Contention Benchmark ===\n");
    std::printf("Incrementing a shared counter %llu times per thread\n", (unsigned long long)INCREMENTS);
    std::printf("Lock types: std::mutex, SpinLock, BackoffSpinLock, TicketLock, fetch_add\n");

    print_header();

    for (int nt : thread_counts) {
        {
            std::mutex mtx;
            std::atomic<uint64_t> counter{0};
            auto r = run_locked_benchmark(nt, INCREMENTS, mtx, counter);
            print_result("std::mutex", nt, r);
        }
        {
            SpinLock spin;
            std::atomic<uint64_t> counter{0};
            auto r = run_locked_benchmark(nt, INCREMENTS, spin, counter);
            print_result("SpinLock", nt, r);
        }
        {
            BackoffSpinLock boff;
            std::atomic<uint64_t> counter{0};
            auto r = run_locked_benchmark(nt, INCREMENTS, boff, counter);
            print_result("BackoffSpin", nt, r);
        }
        {
            TicketLock ticket;
            std::atomic<uint64_t> counter{0};
            auto r = run_locked_benchmark(nt, INCREMENTS, ticket, counter);
            print_result("TicketLock", nt, r);
        }
        {
            auto r = run_fetch_add_benchmark(nt, INCREMENTS);
            print_result("fetch_add", nt, r);
        }
        std::printf("\n");
    }

    std::printf("=== Scaling Analysis ===\n");
    std::printf("Thread counts: ");
    for (int nt : thread_counts) std::printf("%d ", nt);
    std::printf("\n\n");
    std::printf("Expected behaviors:\n");
    std::printf("  1. fetch_add: best scaling — single XADD instruction, minimal contention.\n");
    std::printf("  2. SpinLock: worst scaling — CAS loop bounces cache line N times per acquire.\n");
    std::printf("  3. BackoffSpinLock: better than SpinLock — exponential backoff reduces bouncing.\n");
    std::printf("  4. std::mutex: good scaling — kernel sleep under contention, adaptive spin in glibc.\n");
    std::printf("  5. TicketLock: FIFO fairness — still spins, but no starvation.\n");
    std::printf("  6. Correctness: all approaches produce the correct final count.\n");
    std::printf("\nAs thread count increases:\n");
    std::printf("  - Lock-based throughput degrades (more time waiting, less time working).\n");
    std::printf("  - SpinLock throughput collapses (N cores fighting over one cache line).\n");
    std::printf("  - Mutex throughput plateaus (sleeping threads free up cores for useful work).\n");

    return 0;
}