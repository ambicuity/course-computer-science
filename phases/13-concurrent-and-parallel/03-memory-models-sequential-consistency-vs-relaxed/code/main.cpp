// Memory Models — Sequential Consistency vs Relaxed
// Phase 13 — Concurrent & Parallel Computing
// Litmus test suite demonstrating SC, TSO, and relaxed ordering
//
// Compile: g++ -O2 -pthread -std=c++20 -o memmodel main.cpp && ./memmodel

#include <atomic>
#include <thread>
#include <iostream>
#include <cassert>
#include <chrono>
#include <cstring>

// =============================================================================
// Part 1: Sequentially Consistent Atomics — Dekker Pattern
// Under seq_cst, (r1=0, r2=0) is impossible because a global total order
// guarantees at least one thread sees the other's write.
// =============================================================================

void part1_dekker_sc() {
    std::cout << "=== Part 1: SC Dekker Pattern ===\n";
    constexpr int ITERATIONS = 50000;

    std::atomic<int> x{0};
    std::atomic<int> y{0};
    int r1{0}, r2{0};

    int outcomes[4] = {0};

    for (int i = 0; i < ITERATIONS; ++i) {
        x.store(0, std::memory_order_seq_cst);
        y.store(0, std::memory_order_seq_cst);
        r1 = r2 = 0;

        std::thread t1([&]() {
            x.store(1, std::memory_order_seq_cst);
            r1 = y.load(std::memory_order_seq_cst);
        });

        std::thread t2([&]() {
            y.store(1, std::memory_order_seq_cst);
            r2 = x.load(std::memory_order_seq_cst);
        });

        t1.join();
        t2.join();

        outcomes[r1 * 2 + r2]++;
    }

    std::cout << "SC Dekker outcomes (" << ITERATIONS << " runs):\n";
    std::cout << "  (0,0): " << outcomes[0] << "  (should be 0)\n";
    std::cout << "  (0,1): " << outcomes[1] << "\n";
    std::cout << "  (1,0): " << outcomes[2] << "\n";
    std::cout << "  (1,1): " << outcomes[3] << "\n";

    if (outcomes[0] == 0) {
        std::cout << "  ✓ SC guarantee holds: (0,0) never observed\n";
    } else {
        std::cout << "  ⚠ Unexpected (0,0) observed — hardware weaker than SC?\n";
    }
}

// =============================================================================
// Part 2: Relaxed Atomics — Surprising Results
// With memory_order_relaxed, the compiler and CPU can reorder stores.
// The reader can observe (a=1, b=0) — b written after a but visible first.
// =============================================================================

void part2_relaxed_reordering() {
    std::cout << "\n=== Part 2: Relaxed Atomics — Surprising Reordering ===\n";
    constexpr int ITERATIONS = 100000;

    std::atomic<int> a{0};
    std::atomic<int> b{0};
    int observed[4] = {0};

    for (int iter = 0; iter < ITERATIONS; ++iter) {
        a.store(0, std::memory_order_relaxed);
        b.store(0, std::memory_order_relaxed);

        std::thread writer([&]() {
            a.store(1, std::memory_order_relaxed);
            b.store(1, std::memory_order_relaxed);
        });

        std::thread reader([&]() {
            int i = b.load(std::memory_order_relaxed);
            int j = a.load(std::memory_order_relaxed);
            observed[i * 2 + j]++;
        });

        writer.join();
        reader.join();
    }

    std::cout << "Relaxed outcomes (" << ITERATIONS << " runs):\n";
    std::cout << "  (a=0,b=0): " << observed[0] << "\n";
    std::cout << "  (a=0,b=1): " << observed[1] << "\n";
    std::cout << "  (a=1,b=0): " << observed[2]
              << "  (reordered! b=1 visible before a=1)\n";
    std::cout << "  (a=1,b=1): " << observed[3] << "\n";

    double pct = 100.0 * observed[2] / ITERATIONS;
    std::cout << "  → " << pct << "% of runs showed reordering\n";
}

// =============================================================================
// Part 3: Message Passing — Acquire/Release vs Relaxed
// Producer stores data, then sets ready flag.
// Consumer spins on ready, then reads data.
//
// With acquire/release: data is always visible when ready is set.
// With relaxed: the assertion can fail — the consumer sees ready=true
// but data is still 0.
// =============================================================================

void part3_message_passing_relaxed() {
    std::cout << "\n=== Part 3a: Message Passing with Relaxed (should fail) ===\n";
    constexpr int ITERATIONS = 50000;

    std::atomic<bool> ready{false};
    int data{0};
    int failures = 0;

    for (int i = 0; i < ITERATIONS && failures < 5; ++i) {
        data = 0;
        ready.store(false, std::memory_order_relaxed);

        std::thread producer([&]() {
            data = 42;
            ready.store(true, std::memory_order_relaxed);
        });

        std::thread consumer([&]() {
            while (!ready.load(std::memory_order_relaxed)) {}
            if (data != 42) {
                failures++;
            }
        });

        producer.join();
        consumer.join();
    }

    if (failures > 0) {
        std::cout << "  ⚠ Failed " << failures << " times: relaxed allows data race\n";
    } else {
        std::cout << "  No failures observed on this hardware (common on x86)\n";
    }
}

void part3_message_passing_acq_rel() {
    std::cout << "\n=== Part 3b: Message Passing with Acquire/Release (always correct) ===\n";
    constexpr int ITERATIONS = 50000;

    std::atomic<bool> ready{false};
    int data{0};
    int failures = 0;

    for (int i = 0; i < ITERATIONS; ++i) {
        data = 0;
        ready.store(false, std::memory_order_release);

        std::thread producer([&]() {
            data = 42;
            ready.store(true, std::memory_order_release);
        });

        std::thread consumer([&]() {
            while (!ready.load(std::memory_order_acquire)) {}
            if (data != 42) {
                failures++;
            }
        });

        producer.join();
        consumer.join();
    }

    if (failures == 0) {
        std::cout << "  ✓ Acquire/release guarantees: data always visible (0 failures)\n";
    } else {
        std::cout << "  ⚠ Unexpected failures: " << failures << "\n";
    }
}

// =============================================================================
// Part 4: Memory Fences
// Using std::atomic_thread_fence instead of ordering on atomic operations.
// A release fence + relaxed store has the same effect as a release store.
// =============================================================================

void part4_fences() {
    std::cout << "\n=== Part 4: Memory Fences ===\n";
    constexpr int ITERATIONS = 50000;

    std::atomic<bool> ready{false};
    int data{0};
    int failures = 0;

    for (int i = 0; i < ITERATIONS; ++i) {
        data = 0;
        ready.store(false, std::memory_order_relaxed);

        std::thread producer([&]() {
            data = 42;
            std::atomic_thread_fence(std::memory_order_release);
            ready.store(true, std::memory_order_relaxed);
        });

        std::thread consumer([&]() {
            while (!ready.load(std::memory_order_relaxed)) {}
            std::atomic_thread_fence(std::memory_order_acquire);
            if (data != 42) {
                failures++;
            }
        });

        producer.join();
        consumer.join();
    }

    if (failures == 0) {
        std::cout << "  ✓ Release fence + acquire fence: always correct (0 failures)\n";
    } else {
        std::cout << "  ⚠ Failures with fences: " << failures << "\n";
    }
}

// =============================================================================
// Part 5: IRIW Litmus Test (Independent Reads of Independent Writes)
// Two threads each write to their own variable.
// Two reader threads observe both variables.
// Under SC, all readers should agree on the order of the two writes.
// On relaxed hardware (ARM/Power), readers can disagree.
// =============================================================================

void part5_iriw() {
    std::cout << "\n=== Part 5: IRIW (Independent Reads of Independent Writes) ===\n";
    constexpr int ITERATIONS = 50000;

    std::atomic<int> x{0};
    std::atomic<int> y{0};
    int disagreement_count = 0;

    for (int i = 0; i < ITERATIONS; ++i) {
        x.store(0, std::memory_order_seq_cst);
        y.store(0, std::memory_order_seq_cst);
        int r1 = 0, r2 = 0, r3 = 0, r4 = 0;

        std::thread t1([&]() {
            x.store(1, std::memory_order_seq_cst);
        });

        std::thread t2([&]() {
            y.store(1, std::memory_order_seq_cst);
        });

        std::thread t3([&]() {
            r1 = x.load(std::memory_order_seq_cst);
            r2 = y.load(std::memory_order_seq_cst);
        });

        std::thread t4([&]() {
            r3 = y.load(std::memory_order_seq_cst);
            r4 = x.load(std::memory_order_seq_cst);
        });

        t1.join();
        t2.join();
        t3.join();
        t4.join();

        // Reader 1 saw (x=1, y=0) and reader 2 saw (y=1, x=0) → disagreement
        if (r1 == 1 && r2 == 0 && r3 == 1 && r4 == 0) {
            disagreement_count++;
        }
    }

    if (disagreement_count > 0) {
        std::cout << "  Disagreements: " << disagreement_count
                  << " (hardware weaker than SC)\n";
    } else {
        std::cout << "  ✓ IRIW: no disagreements, hardware is SC or TSO\n";
    }
}

// =============================================================================
// Main — Run all litmus tests
// =============================================================================

int main() {
    std::cout << "Memory Model Litmus Test Suite\n";
    std::cout << "==============================\n";
    std::cout << "Hardware: ";

#ifdef __x86_64__
    std::cout << "x86-64 (TSO)\n\n";
#elif defined(__aarch64__)
    std::cout << "ARM64 (Relaxed)\n\n";
#else
    std::cout << "Unknown\n\n";
#endif

    part1_dekker_sc();
    part2_relaxed_reordering();
    part3_message_passing_relaxed();
    part3_message_passing_acq_rel();
    part4_fences();
    part5_iriw();

    std::cout << "\nDone. See docs/en.md for explanation of each result.\n";
    return 0;
}
