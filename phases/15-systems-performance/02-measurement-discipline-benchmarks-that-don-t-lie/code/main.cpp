#include <iostream>
#include <vector>
#include <algorithm>
#include <numeric>
#include <random>
#include <cmath>
#include <chrono>
#include <functional>
#include <iomanip>
#include <sstream>

struct BenchResult {
    std::string name;
    double min_ns;
    double median_ns;
    double mean_ns;
    double p99_ns;
    double max_ns;
    double stddev_ns;
    int iterations;
    int warmup;
};

static void prevent_dce_volatile(volatile int& sink) {
    (void)sink;
}

static void prevent_dce_asm() {
#if defined(_MSC_VER)
    _ReadWriteBarrier();
#else
    asm volatile("" ::: "memory");
#endif
}

static std::vector<double> sorted_samples;

static BenchResult run_benchmark(
    const std::string& name,
    int warmup_iters,
    int measure_iters,
    std::function<int()> fn
) {
    std::vector<double> samples;
    samples.reserve(measure_iters);

    for (int i = 0; i < warmup_iters; i++) {
        volatile int dummy = fn();
        (void)dummy;
    }

    for (int i = 0; i < measure_iters; i++) {
        prevent_dce_asm();
        auto start = std::chrono::high_resolution_clock::now();
        int result = fn();
        prevent_dce_asm();
        volatile int sink = result;
        (void)sink;
        auto end = std::chrono::high_resolution_clock::now();
        double ns = std::chrono::duration<double, std::nano>(end - start).count();
        samples.push_back(ns);
    }

    std::sort(samples.begin(), samples.end());

    int n = (int)samples.size();
    double min_val = samples[0];
    double max_val = samples[n - 1];
    double median_val = (n % 2 == 0)
        ? (samples[n / 2 - 1] + samples[n / 2]) / 2.0
        : samples[n / 2];
    double sum = 0;
    for (double s : samples) sum += s;
    double mean_val = sum / n;
    double p99_idx = 0.99 * (n - 1);
    int p99_lo = (int)std::floor(p99_idx);
    int p99_hi = (int)std::ceil(p99_idx);
    double frac = p99_idx - p99_lo;
    double p99_val = samples[p99_lo] * (1 - frac) + samples[p99_hi] * frac;

    double sq_sum = 0;
    for (double s : samples) {
        double diff = s - mean_val;
        sq_sum += diff * diff;
    }
    double stddev_val = std::sqrt(sq_sum / n);

    return {name, min_val, median_val, mean_val, p99_val, max_val, stddev_val, measure_iters, warmup_iters};
}

static void print_header() {
    std::cout << std::left << std::setw(28) << "Benchmark"
              << std::right << std::setw(10) << "min"
              << std::setw(10) << "median"
              << std::setw(10) << "mean"
              << std::setw(10) << "p99"
              << std::setw(10) << "max"
              << std::setw(10) << "stddev" << "\n";
    std::cout << std::string(88, '-') << "\n";
}

static void print_result(const BenchResult& r) {
    auto fmt = [](double ns) -> std::string {
        std::ostringstream oss;
        if (ns < 1000) oss << std::fixed << std::setprecision(1) << ns << "ns";
        else if (ns < 1e6) oss << std::fixed << std::setprecision(2) << (ns / 1e3) << "us";
        else oss << std::fixed << std::setprecision(2) << (ns / 1e6) << "ms";
        return oss.str();
    };
    std::cout << std::left << std::setw(28) << r.name
              << std::right << std::setw(10) << fmt(r.min_ns)
              << std::setw(10) << fmt(r.median_ns)
              << std::setw(10) << fmt(r.mean_ns)
              << std::setw(10) << fmt(r.p99_ns)
              << std::setw(10) << fmt(r.max_ns)
              << std::setw(10) << fmt(r.stddev_ns) << "\n";
}

static std::vector<int> make_sequential_data(int n) {
    std::vector<int> v(n);
    std::iota(v.begin(), v.end(), 0);
    return v;
}

static std::vector<int> make_random_data(int n, unsigned seed) {
    std::vector<int> v = make_sequential_data(n);
    std::shuffle(v.begin(), v.end(), std::mt19937(seed));
    return v;
}

static int bench_sequential_access(const std::vector<int>& data) {
    long sum = 0;
    for (int val : data) {
        sum += val;
    }
    prevent_dce_asm();
    return static_cast<int>(sum);
}

static int bench_random_access(const std::vector<int>& indices, const std::vector<int>& data) {
    long sum = 0;
    for (int idx : indices) {
        sum += data[idx];
    }
    prevent_dce_asm();
    return static_cast<int>(sum);
}

static int bench_binary_search_sorted(const std::vector<int>& sorted_data, const std::vector<int>& targets) {
    int found = 0;
    for (int t : targets) {
        auto it = std::lower_bound(sorted_data.begin(), sorted_data.end(), t);
        if (it != sorted_data.end() && *it == t) found++;
    }
    prevent_dce_asm();
    return found;
}

static int bench_binary_search_random(const std::vector<int>& random_data, const std::vector<int>& targets) {
    std::vector<int> data_copy = random_data;
    std::sort(data_copy.begin(), data_copy.end());
    int found = 0;
    for (int t : targets) {
        auto it = std::lower_bound(data_copy.begin(), data_copy.end(), t);
        if (it != data_copy.end() && *it == t) found++;
    }
    prevent_dce_asm();
    return found;
}

static void demonstrate_dce() {
    std::cout << "=== Dead Code Elimination Demo ===\n\n";

    constexpr int N = 10000;
    std::vector<int> data(N);
    std::iota(data.begin(), data.end(), 0);

    auto naive = [&]() -> int {
        long sum = 0;
        for (int i = 0; i < N; i++) {
            sum += data[i];
        }
        return static_cast<int>(sum);
    };

    auto protected_fn = [&]() -> int {
        long sum = 0;
        for (int i = 0; i < N; i++) {
            sum += data[i];
        }
        prevent_dce_asm();
        return static_cast<int>(sum);
    };

    const int iters = 100;
    auto r_naive = run_benchmark("naive_loop", 10, iters, naive);
    auto r_protected = run_benchmark("protected_loop", 10, iters, protected_fn);

    print_header();
    print_result(r_naive);
    print_result(r_protected);

    if (r_naive.mean_ns < r_protected.mean_ns * 0.5) {
        std::cout << "\nWARNING: Naive loop appears much faster — likely DCE!\n";
        std::cout << "The compiler probably removed the computation because the result was unused.\n";
    }
    std::cout << "\n";
}

static void demonstrate_warm_cache() {
    std::cout << "=== Cache Warmup Demo ===\n\n";

    constexpr int N = 100000;
    auto data = make_sequential_data(N);
    auto indices = make_random_data(N, 42);

    std::vector<double> first_10;
    std::vector<double> later_10;

    for (int i = 0; i < 20; i++) {
        auto start = std::chrono::high_resolution_clock::now();
        long sum = 0;
        for (int idx : indices) {
            sum += data[idx];
        }
        prevent_dce_asm();
        volatile int sink = static_cast<int>(sum);
        (void)sink;
        auto end = std::chrono::high_resolution_clock::now();
        double ns = std::chrono::duration<double, std::nano>(end - start).count();
        if (i < 10) first_10.push_back(ns);
        else later_10.push_back(ns);
    }

    auto avg = [](const std::vector<double>& v) {
        double s = 0;
        for (double x : v) s += x;
        return s / v.size();
    };

    std::cout << "First 10 iterations avg:  " << std::fixed << std::setprecision(0)
              << avg(first_10) << " ns\n";
    std::cout << "Later 10 iterations avg:  " << avg(later_10) << " ns\n";
    std::cout << "Ratio (cold/warm):       " << std::setprecision(2)
              << avg(first_10) / avg(later_10) << "x\n\n";
    std::cout << "The first iterations are slower because data must be fetched from DRAM.\n";
    std::cout << "After warming the cache, accesses hit L1/L2 instead.\n\n";
}

int main() {
    constexpr int WARMUP = 20;
    constexpr int MEASURE = 200;
    constexpr int DATA_SIZE = 100000;
    constexpr int SMALL_SIZE = 10000;

    std::cout << "Measurement Discipline — Benchmarks That Don't Lie\n";
    std::cout << std::string(60, '=') << "\n\n";

    demonstrate_dce();

    demonstrate_warm_cache();

    std::cout << "=== Main Benchmarks ===\n";
    std::cout << "Data size: " << DATA_SIZE << " elements\n";
    std::cout << "Warmup: " << WARMUP << " iterations, Measure: " << MEASURE << " iterations\n\n";

    auto seq_data = make_sequential_data(DATA_SIZE);
    auto rand_indices = make_random_data(DATA_SIZE, 12345);
    auto sorted_data = make_sequential_data(SMALL_SIZE);
    auto shuffled_data = make_random_data(SMALL_SIZE, 999);
    auto search_targets = make_random_data(SMALL_SIZE, 777);

    std::vector<BenchResult> results;

    results.push_back(run_benchmark("seq_access", WARMUP, MEASURE,
        [&]() { return bench_sequential_access(seq_data); }));

    results.push_back(run_benchmark("random_access", WARMUP, MEASURE,
        [&]() { return bench_random_access(rand_indices, seq_data); }));

    results.push_back(run_benchmark("bin_search_sorted", WARMUP, MEASURE,
        [&]() { return bench_binary_search_sorted(sorted_data, search_targets); }));

    results.push_back(run_benchmark("bin_search_random", WARMUP, MEASURE,
        [&]() { return bench_binary_search_random(shuffled_data, search_targets); }));

    print_header();
    for (const auto& r : results) {
        print_result(r);
    }

    std::cout << "\n=== Analysis ===\n\n";
    double seq_median = results[0].median_ns;
    double rand_median = results[1].median_ns;
    double sorted_median = results[2].median_ns;
    double rand_bs_median = results[3].median_ns;

    std::cout << "Sequential vs Random access:\n";
    std::cout << "  Random is " << std::fixed << std::setprecision(1)
              << rand_median / seq_median << "x slower (cache + TLB effects)\n\n";

    std::cout << "Sorted vs Random binary search:\n";
    std::cout << "  Random layout is " << rand_bs_median / sorted_median
              << "x slower (branch prediction + cache)\n\n";

    double mean_ratio = results[1].mean_ns / results[1].median_ns;
    std::cout << "Random access mean/median ratio: " << std::setprecision(2)
              << mean_ratio << "\n";
    if (mean_ratio > 1.3) {
        std::cout << "  Mean is significantly above median — heavy tail (outliers from OS noise)\n";
        std::cout << "  → Report median, not mean, for typical-case performance\n";
    }

    return 0;
}