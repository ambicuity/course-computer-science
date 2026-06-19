#include <array>
#include <atomic>
#include <chrono>
#include <iostream>
#include <thread>
#include <vector>
#include <cstdint>

static constexpr std::size_t NUM_THREADS = 4;
static constexpr std::uint64_t INCREMENTS = 50'000'000;

struct PackedCounters {
    std::atomic<std::uint64_t> counters[NUM_THREADS];
};

struct alignas(64) PaddedAtomic {
    std::atomic<std::uint64_t> value;
    char padding[64 - sizeof(std::atomic<std::uint64_t>)];
};

struct PaddedCounters {
    PaddedAtomic counters[NUM_THREADS];
};

static void increment_packed(std::size_t thread_id, PackedCounters& pc) {
    for (std::uint64_t i = 0; i < INCREMENTS; ++i) {
        pc.counters[thread_id].store(
            pc.counters[thread_id].load(std::memory_order_relaxed) + 1,
            std::memory_order_relaxed);
    }
}

static void increment_padded(std::size_t thread_id, PaddedCounters& pc) {
    for (std::uint64_t i = 0; i < INCREMENTS; ++i) {
        pc.counters[thread_id].value.store(
            pc.counters[thread_id].value.load(std::memory_order_relaxed) + 1,
            std::memory_order_relaxed);
    }
}

struct ThreadResult {
    std::size_t thread_id;
    double elapsed_ms;
    std::uint64_t final_value;
};

static void run_packed_demo() {
    PackedCounters pc{};
    for (std::size_t i = 0; i < NUM_THREADS; ++i) {
        pc.counters[i].store(0, std::memory_order_relaxed);
    }

    std::vector<std::thread> threads;
    threads.reserve(NUM_THREADS);

    auto start = std::chrono::high_resolution_clock::now();
    for (std::size_t i = 0; i < NUM_THREADS; ++i) {
        threads.emplace_back(increment_packed, i, std::ref(pc));
    }
    for (auto& t : threads) {
        t.join();
    }
    auto end = std::chrono::high_resolution_clock::now();

    double total_ms = std::chrono::duration<double, std::milli>(end - start).count();
    std::cout << "=== Packed (false sharing) ===\n";
    std::cout << "Total time: " << total_ms << " ms\n";
    for (std::size_t i = 0; i < NUM_THREADS; ++i) {
        std::cout << "  Counter " << i << ": "
                  << pc.counters[i].load(std::memory_order_relaxed) << "\n";
    }

    std::cout << "  sizeof(PackedCounters): " << sizeof(PackedCounters) << " bytes\n";
    std::cout << "\n";
}

static void run_padded_demo() {
    PaddedCounters pc{};
    for (std::size_t i = 0; i < NUM_THREADS; ++i) {
        pc.counters[i].value.store(0, std::memory_order_relaxed);
    }

    std::vector<std::thread> threads;
    threads.reserve(NUM_THREADS);

    auto start = std::chrono::high_resolution_clock::now();
    for (std::size_t i = 0; i < NUM_THREADS; ++i) {
        threads.emplace_back(increment_padded, i, std::ref(pc));
    }
    for (auto& t : threads) {
        t.join();
    }
    auto end = std::chrono::high_resolution_clock::now();

    double total_ms = std::chrono::duration<double, std::milli>(end - start).count();
    std::cout << "=== Padded (alignas(64), no false sharing) ===\n";
    std::cout << "Total time: " << total_ms << " ms\n";
    for (std::size_t i = 0; i < NUM_THREADS; ++i) {
        std::cout << "  Counter " << i << ": "
                  << pc.counters[i].value.load(std::memory_order_relaxed) << "\n";
    }

    std::cout << "  sizeof(PaddedCounters): " << sizeof(PaddedCounters) << " bytes\n";
    std::cout << "  sizeof(PaddedAtomic): " << sizeof(PaddedAtomic) << " bytes\n";
    std::cout << "\n";
}

static void run_per_thread_timing_demo() {
    std::cout << "=== Per-thread timing (padded) ===\n";

    PaddedCounters pc{};
    for (std::size_t i = 0; i < NUM_THREADS; ++i) {
        pc.counters[i].value.store(0, std::memory_order_relaxed);
    }

    std::atomic<bool> go{false};
    std::vector<std::thread> threads;
    threads.reserve(NUM_THREADS);
    std::array<double, NUM_THREADS> per_thread_ms{};

    for (std::size_t i = 0; i < NUM_THREADS; ++i) {
        threads.emplace_back([&pc, &go, &per_thread_ms, i]() {
            while (!go.load(std::memory_order_acquire)) {
                std::this_thread::yield();
            }
            auto t0 = std::chrono::high_resolution_clock::now();
            for (std::uint64_t n = 0; n < INCREMENTS; ++n) {
                pc.counters[i].value.store(
                    pc.counters[i].value.load(std::memory_order_relaxed) + 1,
                    std::memory_order_relaxed);
            }
            auto t1 = std::chrono::high_resolution_clock::now();
            per_thread_ms[i] = std::chrono::duration<double, std::milli>(t1 - t0).count();
        });
    }

    go.store(true, std::memory_order_release);

    for (auto& t : threads) {
        t.join();
    }

    for (std::size_t i = 0; i < NUM_THREADS; ++i) {
        std::cout << "  Thread " << i << ": "
                  << per_thread_ms[i] << " ms, counter = "
                  << pc.counters[i].value.load(std::memory_order_relaxed) << "\n";
    }
    std::cout << "\n";
}

static void print_cache_line_info() {
    std::cout << "=== Cache line diagnostics ===\n";
    std::cout << "  sizeof(std::atomic<uint64_t>): " << sizeof(std::atomic<std::uint64_t>) << " bytes\n";
    std::cout << "  sizeof(PackedCounters):      " << sizeof(PackedCounters) << " bytes\n";
    std::cout << "  sizeof(PaddedAtomic):         " << sizeof(PaddedAtomic) << " bytes\n";
    std::cout << "  sizeof(PaddedCounters):       " << sizeof(PaddedCounters) << " bytes\n";
    std::cout << "  alignof(PaddedAtomic):        " << alignof(PaddedAtomic) << "\n";

    PackedCounters packed{};
    std::cout << "\n  Packed counter addresses:\n";
    for (std::size_t i = 0; i < NUM_THREADS; ++i) {
        std::cout << "    &counters[" << i << "] = " << static_cast<void*>(&packed.counters[i])
                  << "  (offset " << (reinterpret_cast<char*>(&packed.counters[i]) - reinterpret_cast<char*>(&packed))
                  << ")\n";
    }

    PaddedCounters padded{};
    std::cout << "\n  Padded counter addresses:\n";
    for (std::size_t i = 0; i < NUM_THREADS; ++i) {
        std::cout << "    &counters[" << i << "] = " << static_cast<void*>(&padded.counters[i].value)
                  << "  (offset " << (reinterpret_cast<char*>(&padded.counters[i].value) - reinterpret_cast<char*>(&padded))
                  << ")\n";
    }
    std::cout << "\n";
}

static void print_numa_hints() {
    std::cout << "=== NUMA detection hints ===\n";
    std::cout << "  Run: numactl --hardware\n";
    std::cout << "  Run: numastat -m\n";
    std::cout << "  Run: perf stat -e L1-dcache-load-misses,cache-misses <program>\n";
    std::cout << "\n";
}

int main() {
    std::cout << "False Sharing and NUMA Demo\n";
    std::cout << "============================\n";
    std::cout << "Threads: " << NUM_THREADS << "\n";
    std::cout << "Increments per thread: " << INCREMENTS << "\n\n";

    print_cache_line_info();
    run_packed_demo();
    run_padded_demo();
    run_per_thread_timing_demo();
    print_numa_hints();

    return 0;
}