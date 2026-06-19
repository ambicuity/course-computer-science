// C++ Low-Latency Idioms — Phase 15, Lesson 15
// Compile: g++ -std=c++17 -O2 -pthread -o main main.cpp
// Run:     ./main

#include <array>
#include <atomic>
#include <cassert>
#include <chrono>
#include <condition_variable>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <iostream>
#include <memory>
#include <mutex>
#include <new>
#include <optional>
#include <span>
#include <sstream>
#include <string>
#include <string_view>
#include <thread>
#include <type_traits>
#include <vector>

// ─── Utilities ──────────────────────────────────────────────────────────────

using Clock = std::chrono::high_resolution_clock;
using us = std::chrono::microseconds;
using ns = std::chrono::nanoseconds;

static constexpr size_t kIterations = 1'000'000;

#define BENCH(label, fn)                                                       \
    do {                                                                       \
        auto _t0 = Clock::now();                                               \
        fn();                                                                  \
        auto _t1 = Clock::now();                                                \
        auto _dur = std::chrono::duration_cast<ns>(_t1 - _t0).count();         \
        double _avg = static_cast<double>(_dur) / kIterations;                 \
        std::cout << label << ": total=" << _dur << "ns  avg=" << _avg         \
                  << "ns/iter\n";                                              \
    } while (0)

// ─── 1. SPSC Ring Buffer (Lock-Free) ───────────────────────────────────────
// Cache-line-padded indices to prevent false sharing between producer/consumer.

template <typename T, size_t N>
class SPSCRingBuffer {
    static_assert((N & (N - 1)) == 0, "N must be a power of 2");

    alignas(64) std::atomic<size_t> write_pos_{0};
    alignas(64) std::atomic<size_t> read_pos_{0};
    alignas(64) std::array<T, N> buffer_{};

    static constexpr size_t kMask = N - 1;

public:
    bool try_push(const T& val) {
        size_t wp = write_pos_.load(std::memory_order_relaxed);
        size_t rp = read_pos_.load(std::memory_order_acquire);
        if (wp - rp >= N) return false;
        buffer_[wp & kMask] = val;
        write_pos_.store(wp + 1, std::memory_order_release);
        return true;
    }

    std::optional<T> try_pop() {
        size_t rp = read_pos_.load(std::memory_order_relaxed);
        size_t wp = write_pos_.load(std::memory_order_acquire);
        if (rp == wp) return std::nullopt;
        T val = buffer_[rp & kMask];
        read_pos_.store(rp + 1, std::memory_order_release);
        return val;
    }

    size_t size() const {
        size_t wp = write_pos_.load(std::memory_order_acquire);
        size_t rp = read_pos_.load(std::memory_order_acquire);
        return wp - rp;
    }
};

// ─── 2. Object Pool ─────────────────────────────────────────────────────────
// Free-list based, no heap allocation on hot path.

template <typename T, size_t N>
class ObjectPool {
    struct Node {
        T data{};
        Node* next{nullptr};
    };

    alignas(64) std::array<Node, N> storage_{};
    alignas(64) Node* free_list_{nullptr};

public:
    ObjectPool() {
        for (size_t i = 0; i < N; ++i) {
            storage_[i].next = free_list_;
            free_list_ = &storage_[i];
        }
    }

    T* acquire() {
        if (!free_list_) return nullptr;
        Node* node = free_list_;
        free_list_ = node->next;
        return &node->data;
    }

    void release(T* ptr) {
        Node* node = reinterpret_cast<Node*>(ptr);
        node->next = free_list_;
        free_list_ = node;
    }

    size_t available() const {
        size_t count = 0;
        for (Node* n = free_list_; n; n = n->next) ++count;
        return count;
    }
};

// ─── 3. Arena Allocator ─────────────────────────────────────────────────────

class ArenaAllocator {
    alignas(64) std::vector<char> slab_;
    size_t offset_{0};

public:
    explicit ArenaAllocator(size_t bytes) : slab_(bytes, 0) {}

    void* allocate(size_t bytes, size_t alignment = alignof(std::max_align_t)) {
        size_t current = reinterpret_cast<size_t>(slab_.data() + offset_);
        size_t aligned = (current + alignment - 1) & ~(alignment - 1);
        size_t padding = aligned - current;
        if (offset_ + padding + bytes > slab_.size()) return nullptr;
        offset_ += padding + bytes;
        return slab_.data() + offset_ - bytes;
    }

    void reset() { offset_ = 0; }
    size_t used() const { return offset_; }
};

// ─── 4. CRTP vs Virtual ─────────────────────────────────────────────────────

namespace virtual_dispatch {
    struct Base {
        virtual ~Base() = default;
        virtual int compute(int x) const = 0;
    };

    struct Derived : Base {
        int compute(int x) const override { return x * x + 1; }
    };
}

namespace crtp_dispatch {
    template <typename D>
    struct Base {
        int compute(int x) const { return static_cast<const D*>(this)->compute_impl(x); }
    };

    struct Derived : Base<Derived> {
        int compute_impl(int x) const { return x * x + 1; }
    };
}

// ─── 5. Cache-Line Aligned vs Naive Struct ───────────────────────────────────

struct NaiveCounters {
    std::atomic<uint64_t> a{0};
    std::atomic<uint64_t> b{0};
};

struct AlignedCounters {
    alignas(64) std::atomic<uint64_t> a{0};
    alignas(64) std::atomic<uint64_t> b{0};
};

// ─── 6. Seqlock ──────────────────────────────────────────────────────────────

struct alignas(64) SeqlockData {
    uint64_t sequence{0};
    int value_a{0};
    double value_b{0.0};
    int value_c{0};
};

class Seqlock {
    alignas(64) std::atomic<uint64_t> seq_{0};
    SeqlockData data_{};

public:
    void write(int a, double b, int c) {
        uint64_t s = seq_.load(std::memory_order_relaxed);
        seq_.store(s + 1, std::memory_order_release);
        data_.value_a = a;
        data_.value_b = b;
        data_.value_c = c;
        std::atomic_thread_fence(std::memory_order_release);
        seq_.store(s + 2, std::memory_order_release);
    }

    SeqlockData read() {
        SeqlockData copy;
        uint64_t s1, s2;
        do {
            s1 = seq_.load(std::memory_order_acquire);
            if (s1 & 1) {
                std::this_thread::yield();
                continue;
            }
            copy.value_a = data_.value_a;
            copy.value_b = data_.value_b;
            copy.value_c = data_.value_c;
            std::atomic_thread_fence(std::memory_order_acquire);
            s2 = seq_.load(std::memory_order_acquire);
        } while (s1 != s2);
        return copy;
    }
};

// ─── 7. string_view / span demonstration ──────────────────────────────────────

static size_t count_words_buf(std::string_view sv) {
    size_t count = 0;
    bool in_word = false;
    for (char c : sv) {
        if (c == ' ' || c == '\t' || c == '\n') {
            in_word = false;
        } else if (!in_word) {
            ++count;
            in_word = true;
        }
    }
    return count;
}

static size_t count_words_str(const std::string& s) {
    return count_words_buf(s);
}

// ─── Benchmarks ──────────────────────────────────────────────────────────────

static void bench_spsc_ring_buffer() {
    SPSCRingBuffer<int, 1024> queue;
    volatile size_t produced = 0, consumed = 0;
    bool done = false;

    std::thread producer([&] {
        for (size_t i = 0; i < kIterations; ++i) {
            while (!queue.try_push(static_cast<int>(i))) {
                std::this_thread::yield();
            }
            ++produced;
        }
        done = true;
    });

    std::thread consumer([&] {
        while (!done || queue.size() > 0) {
            auto val = queue.try_pop();
            if (val) ++consumed;
            else std::this_thread::yield();
        }
    });

    producer.join();
    consumer.join();
    assert(produced == consumed);
    std::cout << "  SPSC ring buffer: produced=" << produced
              << " consumed=" << consumed << "\n";
}

static void bench_object_pool() {
    ObjectPool<int, 4096> pool;
    auto t0 = Clock::now();
    for (size_t i = 0; i < kIterations; ++i) {
        int* p = pool.acquire();
        if (p) {
            *p = static_cast<int>(i);
            pool.release(p);
        }
    }
    auto t1 = Clock::now();
    auto dur = std::chrono::duration_cast<ns>(t1 - t0).count();
    std::cout << "  ObjectPool acquire/release: total=" << dur
              << "ns  avg=" << static_cast<double>(dur) / kIterations << "ns/iter\n";
}

static void bench_arena_allocator() {
    ArenaAllocator arena(1024 * 1024);
    auto t0 = Clock::now();
    for (size_t i = 0; i < kIterations; ++i) {
        arena.reset();
        volatile void* p = arena.allocate(64);
        (void)p;
    }
    auto t1 = Clock::now();
    auto dur = std::chrono::duration_cast<ns>(t1 - t0).count();
    std::cout << "  Arena allocate+reset: total=" << dur
              << "ns  avg=" << static_cast<double>(dur) / kIterations << "ns/iter\n";
}

static void bench_heap_new_delete() {
    auto t0 = Clock::now();
    for (size_t i = 0; i < kIterations; ++i) {
        int* p = new int(static_cast<int>(i));
        delete p;
    }
    auto t1 = Clock::now();
    auto dur = std::chrono::duration_cast<ns>(t1 - t0).count();
    std::cout << "  new/delete: total=" << dur
              << "ns  avg=" << static_cast<double>(dur) / kIterations << "ns/iter\n";
}

static void bench_crtp_vs_virtual() {
    constexpr size_t iters = 10'000'000;
    volatile int sink = 0;

    auto t0 = Clock::now();
    virtual_dispatch::Derived vd;
    virtual_dispatch::Base* vbp = &vd;
    for (size_t i = 0; i < iters; ++i) {
        sink = vbp->compute(static_cast<int>(i));
    }
    auto t1 = Clock::now();
    auto vdur = std::chrono::duration_cast<ns>(t1 - t0).count();

    auto t2 = Clock::now();
    crtp_dispatch::Derived cd;
    for (size_t i = 0; i < iters; ++i) {
        sink = cd.compute(static_cast<int>(i));
    }
    auto t3 = Clock::now();
    auto cdur = std::chrono::duration_cast<ns>(t3 - t2).count();

    std::cout << "  Virtual dispatch: total=" << vdur << "ns  avg="
              << static_cast<double>(vdur) / iters << "ns/call\n";
    std::cout << "  CRTP dispatch:    total=" << cdur << "ns  avg="
              << static_cast<double>(cdur) / iters << "ns/call\n";
    std::cout << "  Speedup: " << static_cast<double>(vdur) / cdur << "x\n";
    (void)sink;
}

static void bench_cache_line_alignment() {
    constexpr size_t iters = 10'000'000;

    auto bench_two_threads = [](auto& counters, const char* label) {
        auto t0 = Clock::now();
        std::thread t1([&] {
            for (size_t i = 0; i < iters; ++i)
                counters.a.fetch_add(1, std::memory_order_relaxed);
        });
        std::thread t2([&] {
            for (size_t i = 0; i < iters; ++i)
                counters.b.fetch_add(1, std::memory_order_relaxed);
        });
        t1.join();
        t2.join();
        auto t1_dur = std::chrono::duration_cast<ns>(Clock::now() - t0).count();
        std::cout << "  " << label << ": total=" << t1_dur << "ns  avg="
                  << static_cast<double>(t1_dur) / (iters * 2) << "ns/op\n";
    };

    NaiveCounters naive;
    AlignedCounters aligned;
    bench_two_threads(naive, "NaiveCounters (false sharing)");
    bench_two_threads(aligned, "AlignedCounters (cache-line padded)");
}

static void bench_memory_ordering() {
    constexpr size_t iters = 10'000'000;
    alignas(64) std::atomic<int> flag{0};
    alignas(64) int data{0};

    auto bench_order = [&](std::memory_order order, const char* name) {
        auto t0 = Clock::now();
        for (size_t i = 0; i < iters; ++i) {
            data = static_cast<int>(i);
            flag.store(1, order);
            flag.load(order);
            flag.store(0, order);
        }
        auto dur = std::chrono::duration_cast<ns>(Clock::now() - t0).count();
        std::cout << "  " << name << ": total=" << dur << "ns  avg="
                  << static_cast<double>(dur) / iters << "ns/roundtrip\n";
    };

    bench_order(std::memory_order_relaxed, "relaxed");
    bench_order(std::memory_order_acquire, "acquire");
    bench_order(std::memory_order_release, "release");
    bench_order(std::memory_order_acq_rel, "acq_rel");
    bench_order(std::memory_order_seq_cst, "seq_cst");
    (void)data;
}

static void bench_seqlock() {
    Seqlock sl;
    constexpr size_t iters = 1'000'000;
    volatile int sink = 0;
    bool stop = false;

    std::thread writer([&] {
        for (size_t i = 0; i < iters; ++i) {
            sl.write(static_cast<int>(i), static_cast<double>(i) * 0.5, static_cast<int>(i * 3));
        }
        stop = true;
    });

    auto t0 = Clock::now();
    size_t reads = 0;
    std::thread reader([&] {
        while (!stop) {
            auto d = sl.read();
            sink = d.value_a;
            ++reads;
        }
        auto d = sl.read();
        sink = d.value_a;
        ++reads;
    });

    writer.join();
    reader.join();
    auto dur = std::chrono::duration_cast<ns>(Clock::now() - t0).count();
    std::cout << "  Seqlock: " << reads << " reads in " << dur << "ns\n";
    std::cout << "  Avg read latency: "
              << static_cast<double>(dur) / reads << "ns\n";
    (void)sink;
}

static void bench_string_view_vs_string() {
    const char* text = "the quick brown fox jumps over the lazy dog";
    constexpr size_t iters = 1'000'000;

    auto t0 = Clock::now();
    size_t sv_count = 0;
    for (size_t i = 0; i < iters; ++i) {
        sv_count += count_words_buf(std::string_view(text));
    }
    auto sv_dur = std::chrono::duration_cast<ns>(Clock::now() - t0).count();

    auto t1 = Clock::now();
    size_t str_count = 0;
    for (size_t i = 0; i < iters; ++i) {
        str_count += count_words_str(std::string(text));
    }
    auto str_dur = std::chrono::duration_cast<ns>(Clock::now() - t1).count();

    std::cout << "  string_view: total=" << sv_dur << "ns  avg="
              << static_cast<double>(sv_dur) / iters << "ns/call\n";
    std::cout << "  std::string copy: total=" << str_dur << "ns  avg="
              << static_cast<double>(str_dur) / iters << "ns/call\n";
    std::cout << "  string_view speedup: " << static_cast<double>(str_dur) / sv_dur << "x\n";
    assert(sv_count == str_count);
}

static void bench_branch_hints() {
    constexpr size_t iters = 10'000'000;
    constexpr int threshold = 10;
    volatile long sink = 0;

    auto bench_unhinted = [&] {
        auto t0 = Clock::now();
        for (size_t i = 0; i < iters; ++i) {
            int val = static_cast<int>(i % 20);
            if (val < threshold) {
                sink += val;
            } else {
                sink -= 1;
            }
        }
        return std::chrono::duration_cast<ns>(Clock::now() - t0).count();
    };

    auto bench_likely = [&] {
        auto t0 = Clock::now();
        for (size_t i = 0; i < iters; ++i) {
            int val = static_cast<int>(i % 20);
            if (val < threshold) [[likely]] {
                sink += val;
            } else {
                sink -= 1;
            }
        }
        return std::chrono::duration_cast<ns>(Clock::now() - t0).count();
    };

    auto unhinted = bench_unhinted();
    auto hinted = bench_likely();
    std::cout << "  Unhinted branch: " << unhinted << "ns\n";
    std::cout << "  [[likely]] hint:  " << hinted << "ns\n";
    std::cout << "  Difference: likely " << (unhinted > hinted ? "faster" : "no measurable gain") << "\n";
    (void)sink;
}

// ─── Main ────────────────────────────────────────────────────────────────────

int main() {
    std::cout << "=== C++ Low-Latency Idioms Benchmark Suite ===\n\n";

    std::cout << "--- 1. SPSC Ring Buffer (lock-free) ---\n";
    bench_spsc_ring_buffer();
    std::cout << "\n";

    std::cout << "--- 2. Object Pool vs new/delete ---\n";
    bench_object_pool();
    bench_heap_new_delete();
    std::cout << "\n";

    std::cout << "--- 3. Arena Allocator ---\n";
    bench_arena_allocator();
    std::cout << "\n";

    std::cout << "--- 4. CRTP vs Virtual Dispatch ---\n";
    bench_crtp_vs_virtual();
    std::cout << "\n";

    std::cout << "--- 5. Cache-Line Alignment (False Sharing) ---\n";
    bench_cache_line_alignment();
    std::cout << "\n";

    std::cout << "--- 6. Memory Ordering Comparison ---\n";
    bench_memory_ordering();
    std::cout << "\n";

    std::cout << "--- 7. Seqlock Read Latency ---\n";
    bench_seqlock();
    std::cout << "\n";

    std::cout << "--- 8. string_view vs std::string Copy ---\n";
    bench_string_view_vs_string();
    std::cout << "\n";

    std::cout << "--- 9. Branch Hints ([[likely]]/[[unlikely]]) ---\n";
    bench_branch_hints();
    std::cout << "\n";

    std::cout << "=== All benchmarks complete ===\n";
    return 0;
}