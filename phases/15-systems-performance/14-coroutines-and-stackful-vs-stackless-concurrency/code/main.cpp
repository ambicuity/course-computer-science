#include <coroutine>
#include <iostream>
#include <chrono>
#include <thread>
#include <vector>
#include <numeric>
#include <functional>

template<typename T>
struct Generator {
    struct promise_type {
        T current_value;
        auto yield_value(T value) {
            current_value = value;
            return std::suspend_always{};
        }
        auto get_return_object() {
            return Generator{std::coroutine_handle<promise_type>::from_promise(*this)};
        }
        auto initial_suspend() { return std::suspend_always{}; }
        auto final_suspend() noexcept { return std::suspend_always{}; }
        void return_void() {}
        void unhandled_exception() { throw; }
    };
    std::coroutine_handle<promise_type> handle{nullptr};

    Generator() = default;
    Generator(std::coroutine_handle<promise_type> h) : handle(h) {}
    Generator(const Generator&) = delete;
    Generator& operator=(const Generator&) = delete;
    Generator(Generator&& other) noexcept : handle(other.handle) { other.handle = nullptr; }
    Generator& operator=(Generator&& other) noexcept {
        if (this != &other) {
            if (handle) handle.destroy();
            handle = other.handle;
            other.handle = nullptr;
        }
        return *this;
    }
    ~Generator() { if (handle) handle.destroy(); }

    bool next() {
        if (handle && !handle.done()) {
            handle.resume();
            return !handle.done();
        }
        return false;
    }
    T value() { return handle.promise().current_value; }
};

Generator<int> fibonacci() {
    int a = 0, b = 1;
    while (true) {
        co_yield a;
        int tmp = a;
        a = b;
        b = tmp + b;
    }
}

Generator<int> primes() {
    co_yield 2;
    for (int n = 3; ; n += 2) {
        bool is_prime = true;
        for (int d = 3; d * d <= n; d += 2) {
            if (n % d == 0) { is_prime = false; break; }
        }
        if (is_prime) co_yield n;
    }
}

Generator<int> range(int start, int end) {
    for (int i = start; i < end; ++i) {
        co_yield i;
    }
}

struct Task {
    struct promise_type {
        int result;
        auto get_return_object() {
            return Task{std::coroutine_handle<promise_type>::from_promise(*this)};
        }
        auto initial_suspend() { return std::suspend_never{}; }
        auto final_suspend() noexcept { return std::suspend_always{}; }
        void return_value(int val) { result = val; }
        void unhandled_exception() { throw; }
    };
    std::coroutine_handle<promise_type> handle{nullptr};
    Task() = default;
    Task(std::coroutine_handle<promise_type> h) : handle(h) {}
    Task(const Task&) = delete;
    Task& operator=(const Task&) = delete;
    Task(Task&& other) noexcept : handle(other.handle) { other.handle = nullptr; }
    ~Task() { if (handle) handle.destroy(); }
    int get() { return handle.promise().result; }
};

struct SimulateIO {
    int delay_ms;
    bool await_ready() { return delay_ms == 0; }
    void await_suspend(std::coroutine_handle<> h) {
        std::thread([h, delay_ms = delay_ms]() mutable {
            std::this_thread::sleep_for(std::chrono::milliseconds(delay_ms));
            h.resume();
        }).detach();
    }
    int await_resume() { return delay_ms; }
};

Task async_operation(int id, int delay_ms) {
    int result = co_await SimulateIO{delay_ms};
    co_return id * 100 + result;
}

struct SyncIO {
    int delay_ms;
    int operator()() const {
        std::this_thread::sleep_for(std::chrono::milliseconds(delay_ms));
        return delay_ms;
    }
};

long long benchmark_threads(int count, int delay_ms) {
    std::vector<std::thread> threads;
    std::vector<int> results(count);
    auto start = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < count; ++i) {
        threads.emplace_back([&results, i, delay_ms]() {
            results[i] = SyncIO{delay_ms}();
        });
    }
    for (auto& t : threads) t.join();
    auto end = std::chrono::high_resolution_clock::now();
    return std::chrono::duration_cast<std::chrono::microseconds>(end - start).count();
}

void show_stack_usage() {
    std::cout << "\n=== Stack / Frame Size Comparison ===\n";
    std::cout << "OS thread default stack:     8,388,608 bytes (8 MB, Linux default)\n";
    std::cout << "OS thread minimal stack:       131,072 bytes (128 KB, PTHREAD_STACK_MIN)\n";
    std::cout << "goroutine initial stack:         4,096 bytes (4 KB, Go runtime)\n";
    std::cout << "C++20 coroutine frame:              ~48 bytes (varies by captured locals)\n";
    std::cout << "Rust async state machine:           ~64 bytes (varies by captured locals)\n";
    std::cout << "\nScaling to 100,000 concurrent tasks:\n";
    std::cout << "  OS threads:    100,000 * 8 MB  = ~800 GB  (impossible)\n";
    std::cout << "  goroutines:   100,000 * 4 KB  = ~400 MB  (feasible)\n";
    std::cout << "  C++ coroutines: 100,000 * 48 B = ~4.8 MB  (trivial)\n";
}

int main() {
    std::cout << "=== C++20 Coroutine Demo ===\n\n";

    std::cout << "--- Fibonacci Generator ---\n";
    auto fib = fibonacci();
    std::cout << "First 10 Fibonacci numbers: ";
    for (int i = 0; i < 10; ++i) {
        fib.next();
        std::cout << fib.value() << (i < 9 ? " " : "\n");
    }

    std::cout << "\n--- Prime Generator ---\n";
    auto gen = primes();
    std::cout << "First 10 primes: ";
    for (int i = 0; i < 10; ++i) {
        gen.next();
        std::cout << gen.value() << (i < 9 ? " " : "\n");
    }

    std::cout << "\n--- Range Generator ---\n";
    auto rng = range(1, 11);
    std::cout << "Range(1,11): ";
    while (rng.next()) {
        std::cout << rng.value() << " ";
    }
    std::cout << "\n";

    std::cout << "\n--- Task with co_await (simulated I/O) ---\n";
    std::cout << "Launching async operations...\n";
    auto task1 = async_operation(1, 50);
    auto task2 = async_operation(2, 100);
    auto task3 = async_operation(3, 30);
    std::this_thread::sleep_for(std::chrono::milliseconds(200));
    std::cout << "Task 1 result: " << task1.get() << "\n";
    std::cout << "Task 2 result: " << task2.get() << "\n";
    std::cout << "Task 3 result: " << task3.get() << "\n";

    std::cout << "\n=== Coroutine vs Thread Benchmark ===\n";
    int delay = 1;
    for (int count : {100, 500}) {
        std::cout << "\n" << count << " tasks x " << delay << "ms I/O each:\n";
        auto thread_time = benchmark_threads(count, delay);
        std::cout << "  Threads:           " << thread_time << " us\n";
        std::cout << "  (Coroutine benchmark requires async runtime;\n";
        std::cout << "   typical ratio: coroutine 5-50x faster for I/O-bound)\n";
    }

    show_stack_usage();

    std::cout << "\n=== Key Takeaways ===\n";
    std::cout << "1. Stackless coroutines (C++20, Rust async) use ~100x less memory than goroutines.\n";
    std::cout << "2. Goroutines use ~2000x less memory than OS threads.\n";
    std::cout << "3. Coroutine context switch: ~10 ns (function call).\n";
    std::cout << "4. Thread context switch:    ~1-10 us (kernel syscall).\n";
    std::cout << "5. Use coroutines for I/O-bound, threads for CPU-bound.\n";

    return 0;
}