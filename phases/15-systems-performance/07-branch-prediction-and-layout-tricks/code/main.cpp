#include <algorithm>
#include <chrono>
#include <cstdint>
#include <iostream>
#include <numeric>
#include <random>
#include <string>
#include <vector>

using Clock = std::chrono::high_resolution_clock;

static long long bench_label = 0;

#define BENCH(label, code)                                                     \
    do {                                                                       \
        auto _t1 = Clock::now();                                              \
        code;                                                                  \
        auto _t2 = Clock::now();                                              \
        auto _ns = std::chrono::duration_cast<std::chrono::nanoseconds>(       \
                       _t2 - _t1)                                             \
                       .count();                                               \
        std::cout << label << ": " << _ns / 1'000'000 << " ms"               \
                  << " (sum=" << bench_label << ")\n";                        \
    } while (0)

static std::vector<int> make_data(int n, bool sorted) {
    std::mt19937 rng(42);
    std::uniform_int_distribution<int> dist(0, 255);
    std::vector<int> data(n);
    for (auto& x : data) x = dist(rng);
    if (sorted) std::sort(data.begin(), data.end());
    return data;
}

// --- Benchmark 1: Sorted vs Unsorted with branchy if/else ---

static long long branchy_sum(const std::vector<int>& data) {
    long long sum = 0;
    for (int v : data) {
        if (v > 128) sum += v;
    }
    return sum;
}

static long long branchless_sum(const std::vector<int>& data) {
    long long sum = 0;
    for (int v : data) {
        sum += (v > 128) * v;
    }
    return sum;
}

static long long branchless_cmov_sum(const std::vector<int>& data) {
    long long sum = 0;
    for (int v : data) {
        long long add = v;
        long long zero = 0;
        sum += (v > 128) ? add : zero;
    }
    return sum;
}

static void bench_sorted_vs_unsorted(int n) {
    std::cout << "\n=== Benchmark 1: Sorted vs Unsorted (branchy if/else) ===\n";

    auto unsorted = make_data(n, false);
    auto sorted = make_data(n, true);

    bench_label = branchy_sum(unsorted);
    BENCH("Unsorted branchy", bench_label = branchy_sum(unsorted));

    bench_label = branchy_sum(sorted);
    BENCH("Sorted   branchy", bench_label = branchy_sum(sorted));
}

// --- Benchmark 2: Branchy vs Branchless ---

static void bench_branchy_vs_branchless(int n) {
    std::cout << "\n=== Benchmark 2: Branchy vs Branchless (unsorted data) ===\n";

    auto data = make_data(n, false);

    bench_label = branchy_sum(data);
    BENCH("Branchy if/else ", bench_label = branchy_sum(data));

    bench_label = branchless_sum(data);
    BENCH("Branchless (v>128)*v", bench_label = branchless_sum(data));

    bench_label = branchless_cmov_sum(data);
    BENCH("Branchless ternary ", bench_label = branchless_cmov_sum(data));
}

// --- Benchmark 3: likely/unlikely hints ---

static long long likely_sum(const std::vector<int>& data) {
    long long sum = 0;
    for (int v : data) {
        if (v > 128) [[likely]]
            sum += v;
    }
    return sum;
}

static long long unlikely_sum(const std::vector<int>& data) {
    long long sum = 0;
    for (int v : data) {
        if (v > 128) [[unlikely]]
            sum += v;
    }
    return sum;
}

static long long builtin_likely_sum(const std::vector<int>& data) {
    long long sum = 0;
    for (int v : data) {
        if (__builtin_expect(v > 128, 1))
            sum += v;
    }
    return sum;
}

static void bench_hints(int n) {
    std::cout << "\n=== Benchmark 3: likely/unlikely hints ===\n";

    {
        auto data = make_data(n, false);
        std::cout << "  (unsorted data, ~50% taken)\n";
        bench_label = branchy_sum(data);
        BENCH("No hint           ", bench_label = branchy_sum(data));
        bench_label = likely_sum(data);
        BENCH("[[likely]]        ", bench_label = likely_sum(data));
        bench_label = unlikely_sum(data);
        BENCH("[[unlikely]]      ", bench_label = unlikely_sum(data));
        bench_label = builtin_likely_sum(data);
        BENCH("__builtin_expect  ", bench_label = builtin_likely_sum(data));
    }

    {
        auto sorted = make_data(n, true);
        std::cout << "  (sorted data, predictable after transition)\n";
        bench_label = branchy_sum(sorted);
        BENCH("No hint (sorted)  ", bench_label = branchy_sum(sorted));
        bench_label = likely_sum(sorted);
        BENCH("[[likely]] (sort) ", bench_label = likely_sum(sorted));
    }
}

// --- Benchmark 4: Struct layout - hot/cold split ---

struct alignas(64) EntityNaive {
    int x, y, z;
    float health;
    char name[64];
    char description[256];
    int inventory[20];

    void update_hot(int dx, int dy, int dz) {
        x += dx;
        y += dy;
        z += dz;
        health -= 0.01f;
    }
};

struct alignas(64) EntityHot {
    int x, y, z;
    float health;
    int cold_id;
};

struct EntityCold {
    int entity_id;
    char name[64];
    char description[256];
    int inventory[20];
};

static void bench_struct_layout(int n) {
    std::cout << "\n=== Benchmark 4: Struct Layout - Hot/Cold Split ===\n";
    std::cout << "  sizeof(EntityNaive) = " << sizeof(EntityNaive) << "\n";
    std::cout << "  sizeof(EntityHot)   = " << sizeof(EntityHot) << "\n";
    std::cout << "  sizeof(EntityCold)  = " << sizeof(EntityCold) << "\n";

    std::mt19937 rng(42);
    std::uniform_int_distribution<int> pos_dist(-10, 10);

    // Naive layout
    {
        std::vector<EntityNaive> entities(n);
        for (auto& e : entities) {
            e.x = pos_dist(rng);
            e.y = pos_dist(rng);
            e.z = pos_dist(rng);
            e.health = 100.0f;
        }

        auto t1 = Clock::now();
        for (auto& e : entities) {
            e.update_hot(1, -1, 0);
        }
        auto t2 = Clock::now();
        auto ms = std::chrono::duration_cast<std::chrono::nanoseconds>(t2 - t1).count() / 1'000'000;
        std::cout << "Naive layout:  " << ms << " ms\n";
    }

    // Hot/cold split
    {
        std::vector<EntityHot> hot(n);
        std::vector<EntityCold> cold(n);
        for (int i = 0; i < n; i++) {
            hot[i].x = pos_dist(rng);
            hot[i].y = pos_dist(rng);
            hot[i].z = pos_dist(rng);
            hot[i].health = 100.0f;
            hot[i].cold_id = i;
            cold[i].entity_id = i;
        }

        auto t1 = Clock::now();
        for (auto& e : hot) {
            e.x += 1;
            e.y -= 1;
            e.health -= 0.01f;
        }
        auto t2 = Clock::now();
        auto ms = std::chrono::duration_cast<std::chrono::nanoseconds>(t2 - t1).count() / 1'000'000;
        std::cout << "Hot/cold split: " << ms << " ms\n";
    }
}

// --- Benchmark 5: Partition + branchless vs branchy ---

static long long partition_then_process(std::vector<int>& data) {
    auto mid = std::partition(data.begin(), data.end(),
                              [](int x) { return x <= 128; });
    long long sum_low = 0;
    for (auto it = data.begin(); it != mid; ++it)
        sum_low += *it;
    long long sum_high = 0;
    for (auto it = mid; it != data.end(); ++it)
        sum_high += *it;
    return sum_low + sum_high;
}

static void bench_partition(int n) {
    std::cout << "\n=== Benchmark 5: Partition then process (no branch in loop) ===\n";

    {
        auto data = make_data(n, false);
        bench_label = branchy_sum(data);
        BENCH("Branchy (unsorted)", bench_label = branchy_sum(data));
    }

    {
        auto data = make_data(n, false);
        bench_label = partition_then_process(data);
        BENCH("Partition + walk ", bench_label = partition_then_process(data));
    }

    {
        auto data = make_data(n, false);
        bench_label = branchless_sum(data);
        BENCH("Branchless (v>128)*v", bench_label = branchless_sum(data));
    }
}

int main() {
    const int N = 100'000'000;

    std::cout << "Branch Prediction & Layout Tricks Benchmark\n";
    std::cout << "=============================================\n";
    std::cout << "Data size: " << N << " integers in [0, 255]\n\n";

    bench_sorted_vs_unsorted(N);
    bench_branchy_vs_branchless(N);
    bench_hints(N);
    bench_struct_layout(5'000'000);
    bench_partition(N);

    std::cout << "\n=== Key Takeaways ===\n";
    std::cout << "1. Sorted data = predictable branches = 3-4x faster.\n";
    std::cout << "2. Branchless (cmov/arithmetic) wins on unpredictable data.\n";
    std::cout << "3. likely/unlikely hints help when one path is >90% taken.\n";
    std::cout << "4. Hot/cold struct split reduces cache pressure in hot loops.\n";
    std::cout << "5. Partitioning data eliminates branches entirely.\n";
    return 0;
}