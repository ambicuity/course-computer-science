// Lock-Free Data Structures — Treiber Stack, MS Queue
// Phase 13 — Concurrent & Parallel Computing
//
// Implements:
//   - Treiber stack (lock-free LIFO) with ABA-counter tag
//   - Michael-Scott queue (lock-free FIFO) with dummy node
//   - Mutex-based equivalents for performance comparison
//   - Multi-threaded benchmark harness

#include <atomic>
#include <chrono>
#include <cstdint>
#include <cassert>
#include <iomanip>
#include <iostream>
#include <mutex>
#include <optional>
#include <queue>
#include <thread>
#include <vector>

// ==========================================================================
//  CONSTANTS
// ==========================================================================

constexpr int NUM_THREADS = 4;
constexpr int OPS_PER_THREAD = 50'000;

// ==========================================================================
//  UTILITY: packed pointer + ABA tag
// ==========================================================================
// Reserve the high 16 bits for a monotonic ABA counter.
// The low 48 bits hold the pointer (x86-64 only uses 48 bits of VA space).

constexpr uintptr_t TAG_SHIFT = 48;
constexpr uintptr_t PTR_MASK  = (uintptr_t(1) << TAG_SHIFT) - 1;

template <typename T>
inline uintptr_t pack_ptr_tag(T* ptr, uintptr_t tag) {
    return reinterpret_cast<uintptr_t>(ptr) | (tag << TAG_SHIFT);
}

template <typename T>
inline T* unpack_ptr(uintptr_t packed) {
    return reinterpret_cast<T*>(packed & PTR_MASK);
}

inline uintptr_t unpack_tag(uintptr_t packed) {
    return packed >> TAG_SHIFT;
}

// ==========================================================================
//  TREIBER STACK  (lock-free, with ABA counter)
// ==========================================================================

template <typename T>
struct TreiberNode {
    T     data;
    uintptr_t next;  // packed (ptr + tag)
};

template <typename T>
class TreiberStack {
    std::atomic<uintptr_t> head_; // packed (ptr + tag)

public:
    TreiberStack() : head_(0) {}

    void push(const T& data) {
        auto* node = new TreiberNode<T>{data, 0};
        uintptr_t head = head_.load(std::memory_order_acquire);
        do {
            node->next = head;
        } while (!head_.compare_exchange_weak(
            head,
            pack_ptr_tag(node, unpack_tag(head) + 1),
            std::memory_order_release,
            std::memory_order_relaxed));
    }

    std::optional<T> pop() {
        uintptr_t head;
        TreiberNode<T>* head_ptr;
        do {
            head = head_.load(std::memory_order_acquire);
            head_ptr = unpack_ptr<TreiberNode<T>>(head);
            if (head_ptr == nullptr) {
                return std::nullopt;
            }
            uintptr_t next = head_ptr->next;
            if (head_.compare_exchange_weak(
                    head,
                    pack_ptr_tag(unpack_ptr<TreiberNode<T>>(next),
                                 unpack_tag(head) + 1),
                    std::memory_order_release,
                    std::memory_order_relaxed)) {
                break;
            }
        } while (true);
        T val = head_ptr->data;
        delete head_ptr;
        return val;
    }

    bool empty() const {
        return unpack_ptr<TreiberNode<T>>(head_.load(std::memory_order_relaxed)) == nullptr;
    }
};

// ==========================================================================
//  MICHAEL-SCOTT QUEUE  (lock-free, with dummy node)
// ==========================================================================

template <typename T>
struct MSNode {
    std::optional<T> data;
    std::atomic<uintptr_t> next; // packed (ptr + tag)
};

template <typename T>
class MSQueue {
    std::atomic<uintptr_t> head_; // always points to dummy
    std::atomic<uintptr_t> tail_; // points to last node (or dummy when empty)

public:
    MSQueue() {
        auto* dummy = new MSNode<T>{std::nullopt, 0};
        uintptr_t addr = pack_ptr_tag(dummy, 0);
        head_.store(addr, std::memory_order_relaxed);
        tail_.store(addr, std::memory_order_relaxed);
    }

    ~MSQueue() {
        // Drain all nodes
        while (dequeue()) {}
        delete unpack_ptr<MSNode<T>>(head_.load(std::memory_order_relaxed));
    }

    void enqueue(const T& data) {
        auto* node = new MSNode<T>{data, 0};
        uintptr_t node_packed = pack_ptr_tag(node, 0);
        while (true) {
            uintptr_t tail = tail_.load(std::memory_order_acquire);
            auto* tail_ptr = unpack_ptr<MSNode<T>>(tail);
            uintptr_t next = tail_ptr->next.load(std::memory_order_acquire);
            if (tail != tail_.load(std::memory_order_relaxed)) continue;
            if (next != 0) {
                // Tail is lagging; help advance it
                tail_.compare_exchange_weak(tail, next,
                    std::memory_order_release, std::memory_order_relaxed);
                continue;
            }
            // Try to link new node at tail->next
            uintptr_t expected_next = 0;
            if (tail_ptr->next.compare_exchange_weak(
                    expected_next, node_packed,
                    std::memory_order_release, std::memory_order_relaxed)) {
                // Advance tail (best-effort)
                tail_.compare_exchange_weak(tail, node_packed,
                    std::memory_order_release, std::memory_order_relaxed);
                break;
            }
        }
    }

    std::optional<T> dequeue() {
        while (true) {
            uintptr_t head = head_.load(std::memory_order_acquire);
            auto* head_ptr = unpack_ptr<MSNode<T>>(head);
            uintptr_t tail = tail_.load(std::memory_order_acquire);
            uintptr_t next = head_ptr->next.load(std::memory_order_acquire);
            if (head != head_.load(std::memory_order_relaxed)) continue;
            if (head == tail) {
                if (next == 0) return std::nullopt; // empty
                // Tail is lagging; advance it
                tail_.compare_exchange_weak(tail, next,
                    std::memory_order_release, std::memory_order_relaxed);
                continue;
            }
            auto* first_node = unpack_ptr<MSNode<T>>(next);
            T val = *(first_node->data);
            if (head_.compare_exchange_weak(
                    head, next,
                    std::memory_order_release, std::memory_order_relaxed)) {
                delete head_ptr; // old dummy
                return val;
            }
        }
    }

    bool empty() const {
        uintptr_t head = head_.load(std::memory_order_acquire);
        auto* head_ptr = unpack_ptr<MSNode<T>>(head);
        return head_ptr->next.load(std::memory_order_relaxed) == 0;
    }
};

// ==========================================================================
//  MUTEX-BASED STACK  (for comparison)
// ==========================================================================

template <typename T>
class MutexStack {
    std::mutex mtx_;
    std::vector<T> data_;

public:
    void push(const T& val) {
        std::lock_guard<std::mutex> lock(mtx_);
        data_.push_back(val);
    }

    std::optional<T> pop() {
        std::lock_guard<std::mutex> lock(mtx_);
        if (data_.empty()) return std::nullopt;
        T val = data_.back();
        data_.pop_back();
        return val;
    }
};

// ==========================================================================
//  MUTEX-BASED QUEUE  (for comparison)
// ==========================================================================

template <typename T>
class MutexQueue {
    std::mutex mtx_;
    std::queue<T> data_;

public:
    void enqueue(const T& val) {
        std::lock_guard<std::mutex> lock(mtx_);
        data_.push(val);
    }

    std::optional<T> dequeue() {
        std::lock_guard<std::mutex> lock(mtx_);
        if (data_.empty()) return std::nullopt;
        T val = data_.front();
        data_.pop();
        return val;
    }
};

// ==========================================================================
//  HELPERS FOR CONCURRENT TESTING
// ==========================================================================

void test_treiber_stack() {
    TreiberStack<int> stack;
    std::vector<std::thread> threads;
    for (int t = 0; t < NUM_THREADS; ++t) {
        threads.emplace_back([&stack] {
            for (int i = 0; i < OPS_PER_THREAD; ++i) stack.push(i);
            for (int i = 0; i < OPS_PER_THREAD; ++i) {
                while (!stack.pop()) {} // spin
            }
        });
    }
    for (auto& th : threads) th.join();
    assert(stack.empty());
    std::cout << "  Treiber stack: passed\n";
}

void test_ms_queue() {
    MSQueue<int> queue;
    std::vector<std::thread> threads;
    for (int t = 0; t < NUM_THREADS; ++t) {
        threads.emplace_back([&queue] {
            for (int i = 0; i < OPS_PER_THREAD; ++i) queue.enqueue(i);
            for (int i = 0; i < OPS_PER_THREAD; ++i) {
                while (!queue.dequeue()) {} // spin
            }
        });
    }
    for (auto& th : threads) th.join();
    assert(queue.empty());
    std::cout << "  MS queue: passed\n";
}

// ==========================================================================
//  BENCHMARKS
// ==========================================================================

void bench_treiber_vs_mutex() {
    int n = NUM_THREADS;
    int ops = OPS_PER_THREAD;

    // Lock-free
    TreiberStack<int> lf_stack;
    auto start = std::chrono::steady_clock::now();
    {
        std::vector<std::thread> threads;
        for (int t = 0; t < n; ++t) {
            threads.emplace_back([&lf_stack, ops] {
                for (int i = 0; i < ops; ++i) lf_stack.push(i);
                for (int i = 0; i < ops; ++i) {
                    while (!lf_stack.pop()) {}
                }
            });
        }
        for (auto& th : threads) th.join();
    }
    auto lf_us = std::chrono::duration_cast<std::chrono::microseconds>(
        std::chrono::steady_clock::now() - start).count();

    // Mutex-based (must spin like lock-free version for fair comparison)
    MutexStack<int> mx_stack;
    start = std::chrono::steady_clock::now();
    {
        std::vector<std::thread> threads;
        for (int t = 0; t < n; ++t) {
            threads.emplace_back([&mx_stack, ops] {
                for (int i = 0; i < ops; ++i) mx_stack.push(i);
                for (int i = 0; i < ops; ++i) {
                    while (!mx_stack.pop()) {}
                }
            });
        }
        for (auto& th : threads) th.join();
    }
    auto mx_us = std::chrono::duration_cast<std::chrono::microseconds>(
        std::chrono::steady_clock::now() - start).count();

    double lf_ops = double(n) * ops * 2 / (lf_us / 1e6);
    double mx_ops = double(n) * ops * 2 / (mx_us / 1e6);

    std::cout << "  Treiber stack: "
              << std::setw(10) << std::llround(lf_ops) << " ops/s  |  Mutex stack: "
              << std::setw(10) << std::llround(mx_ops) << " ops/s  |  Speedup: "
              << (lf_ops / mx_ops) << "x\n";
}

void bench_ms_vs_mutex() {
    int n = NUM_THREADS;
    int ops = OPS_PER_THREAD;

    // Lock-free
    MSQueue<int> lf_queue;
    auto start = std::chrono::steady_clock::now();
    {
        std::vector<std::thread> threads;
        for (int t = 0; t < n; ++t) {
            threads.emplace_back([&lf_queue, ops] {
                for (int i = 0; i < ops; ++i) lf_queue.enqueue(i);
                for (int i = 0; i < ops; ++i) {
                    while (!lf_queue.dequeue()) {}
                }
            });
        }
        for (auto& th : threads) th.join();
    }
    auto lf_us = std::chrono::duration_cast<std::chrono::microseconds>(
        std::chrono::steady_clock::now() - start).count();

    // Mutex-based (must spin like lock-free version for fair comparison)
    MutexQueue<int> mx_queue;
    start = std::chrono::steady_clock::now();
    {
        std::vector<std::thread> threads;
        for (int t = 0; t < n; ++t) {
            threads.emplace_back([&mx_queue, ops] {
                for (int i = 0; i < ops; ++i) mx_queue.enqueue(i);
                for (int i = 0; i < ops; ++i) {
                    while (!mx_queue.dequeue()) {}
                }
            });
        }
        for (auto& th : threads) th.join();
    }
    auto mx_us = std::chrono::duration_cast<std::chrono::microseconds>(
        std::chrono::steady_clock::now() - start).count();

    double lf_ops = double(n) * ops * 2 / (lf_us / 1e6);
    double mx_ops = double(n) * ops * 2 / (mx_us / 1e6);

    std::cout << "  MS queue:      "
              << std::setw(10) << std::llround(lf_ops) << " ops/s  |  Mutex queue: "
              << std::setw(10) << std::llround(mx_ops) << " ops/s  |  Speedup: "
              << (lf_ops / mx_ops) << "x\n";
}

// ==========================================================================
//  MAIN
// ==========================================================================

int main() {
    std::cout << "═══ Lock-Free Data Structures — Treiber Stack & MS Queue ═══\n\n";

    std::cout << "─── Correctness tests ───\n";
    test_treiber_stack();
    test_ms_queue();
    std::cout << "\n";

    std::cout << "─── Performance benchmarks ("
              << NUM_THREADS << " threads, " << OPS_PER_THREAD << " ops/thread) ───\n";
    bench_treiber_vs_mutex();
    bench_ms_vs_mutex();
    std::cout << "\n";

    std::cout << "─── Demonstration of lock-free push/pop patterns ───\n\n";
    // Treiber stack sequential demo (LIFO)
    TreiberStack<int> s;
    s.push(10);
    s.push(20);
    s.push(30);
    auto v30 = s.pop(); assert(v30 && *v30 == 30);
    auto v20 = s.pop(); assert(v20 && *v20 == 20);
    auto v10 = s.pop(); assert(v10 && *v10 == 10);
    auto vn  = s.pop(); assert(!vn);
    std::cout << "  Treiber stack sequential demo: OK (30, 20, 10, empty)\n";

    // MS queue sequential demo
    MSQueue<std::string> q;
    q.enqueue("a");
    q.enqueue("b");
    q.enqueue("c");
    auto a = q.dequeue(); assert(a && *a == "a");
    auto b = q.dequeue(); assert(b && *b == "b");
    auto c = q.dequeue(); assert(c && *c == "c");
    auto n = q.dequeue(); assert(!n);
    std::cout << "  MS queue sequential demo: OK (a, b, c, empty)\n\n";

    std::cout << "═══ All checks passed ═══\n";
    return 0;
}
