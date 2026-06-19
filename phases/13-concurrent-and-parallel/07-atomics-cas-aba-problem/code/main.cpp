// Phase 13, Lesson 07 — Atomics, CAS, ABA Problem (C++)
// Demonstrates: atomic counters, lock-free Treiber stack, the ABA problem
// with a node-recycling allocator, and tagged-pointer ABA prevention.
//
// Compile:  clang++ -std=c++17 -O2 -pthread main.cpp -o atomic_lesson_cpp
// Run:      ./atomic_lesson_cpp

#include <atomic>
#include <chrono>
#include <cassert>
#include <iostream>
#include <thread>
#include <vector>

// ============================================================================
// Utility: stopwatch
// ============================================================================

struct Stopwatch {
    using Clock = std::chrono::high_resolution_clock;
    Clock::time_point start_ = Clock::now();

    double elapsed_ms() const {
        auto end = Clock::now();
        return std::chrono::duration<double, std::milli>(end - start_).count();
    }
};

// ============================================================================
// Step 1: Atomic Counter
// ============================================================================

class AtomicCounter {
    std::atomic<unsigned long long> val_{0};
public:
    void increment() { val_.fetch_add(1, std::memory_order_relaxed); }
    unsigned long long get() const { return val_.load(std::memory_order_relaxed); }
};

class MutexCounter {
    mutable std::mutex mtx_;
    unsigned long long val_{0};
public:
    void increment() {
        std::lock_guard<std::mutex> lock(mtx_);
        ++val_;
    }
    unsigned long long get() const {
        std::lock_guard<std::mutex> lock(mtx_);
        return val_;
    }
};

void bench_counter() {
    constexpr int N = 1'000'000;
    constexpr int THREADS = 8;

    // Atomic counter
    AtomicCounter ac;
    Stopwatch sw;
    std::vector<std::thread> threads;
    for (int i = 0; i < THREADS; ++i)
        threads.emplace_back([&]() { for (int j = 0; j < N; ++j) ac.increment(); });
    for (auto& t : threads) t.join();
    double atomic_ms = sw.elapsed_ms();
    std::cout << "  Atomic counter: " << (N * THREADS) << " in " << atomic_ms
              << " ms (final=" << ac.get() << ")\n";

    // Mutex counter
    MutexCounter mc;
    threads.clear();
    sw = Stopwatch{};
    for (int i = 0; i < THREADS; ++i)
        threads.emplace_back([&]() { for (int j = 0; j < N; ++j) mc.increment(); });
    for (auto& t : threads) t.join();
    double mutex_ms = sw.elapsed_ms();
    std::cout << "  Mutex counter:  " << (N * THREADS) << " in " << mutex_ms
              << " ms (final=" << mc.get() << ")\n";

    std::cout << "  Speedup: " << (mutex_ms / atomic_ms) << "x\n";
}

// ============================================================================
// Step 2: Lock-Free Stack (Treiber Stack) — raw pointers, no ABA protection
// ============================================================================

struct Node {
    int value;
    Node* next;
};

class LockFreeStack {
    std::atomic<Node*> head_{nullptr};
public:
    void push(Node* node) {
        Node* old = head_.load(std::memory_order_acquire);
        do {
            node->next = old;
        } while (!head_.compare_exchange_weak(old, node,
                                               std::memory_order_release,
                                               std::memory_order_relaxed));
    }

    Node* pop() {
        Node* old = head_.load(std::memory_order_acquire);
        while (old) {
            Node* nxt = old->next;
            if (head_.compare_exchange_weak(old, nxt,
                                            std::memory_order_release,
                                            std::memory_order_relaxed)) {
                return old;
            }
        }
        return nullptr;
    }

    // Expose head_ for the ABA demonstration below.
    // In production code this would be private; we expose it here to
    // simulate the thread interleaving that causes the ABA bug.
    std::atomic<Node*>& debug_head() { return head_; }
};

void test_lockfree_stack() {
    LockFreeStack stack;
    constexpr int N = 1000;

    std::vector<std::thread> pushers;
    for (int start = 0; start < N; start += 250) {
        pushers.emplace_back([&, start]() {
            for (int i = start; i < start + 250; ++i) {
                stack.push(new Node{i, nullptr});
            }
        });
    }
    for (auto& t : pushers) t.join();

    std::vector<int> popped;
    while (Node* n = stack.pop()) {
        popped.push_back(n->value);
        delete n;
    }

    std::sort(popped.begin(), popped.end());
    assert(popped.size() == N);
    for (int i = 0; i < N; ++i) assert(popped[i] == i);
    std::cout << "  Lock-free stack: " << popped.size() << " values OK\n";
}

// ============================================================================
// Step 3: The ABA Problem
// ============================================================================
// We use a single-slot recycling allocator so that address reuse is
// deterministic — the ABA bug always manifests.
//
// Sequence:
//   1. Thread T1: reads head = node A, computes new_head = A->next = B.
//   2. Thread T2: pops A, pops B, recycles A's address.
//   3. Thread T2: allocates C at A's recycled address, pushes C.
//   4. Thread T1: CAS(&head, A, B) — SUCCEEDS because head == C == A's address!
//   5. head now points to B — but B was freed! Data corruption.

Node* recycled = nullptr;

void recycle_node(Node* n) {
    recycled = n;
}

Node* alloc_node(int value) {
    if (recycled) {
        Node* p = recycled;
        recycled = nullptr;
        p->value = value;
        p->next = nullptr;
        return p;
    }
    return new Node{value, nullptr};
}

void demonstrate_aba() {
    std::cout << "\n=== ABA Problem Demonstration ===\n\n";

    // Step 0: allocate A and B, build stack: A -> B -> null
    Node* A = alloc_node(1);
    Node* B = alloc_node(2);
    A->next = B;
    B->next = nullptr;

    std::atomic<Node*> head{A};

    std::cout << "Initial stack: head -> A(" << A << ", val=1)"
              << " -> B(" << B << ", val=2) -> null\n\n";

    // Step 1: Thread T1 reads head = A, plans CAS(&head, A, A->next=B)
    Node* observed = head.load(std::memory_order_acquire);
    Node* new_head = observed->next;  // B
    std::cout << "[T1] load(head) = " << observed
              << "  (will CAS(&head, " << observed << ", " << new_head << "))\n";

    // Step 2: Thread T2 pops A, pops B, recycles A
    head.store(B, std::memory_order_release);          // pop A
    std::cout << "[T2] pop(A): head -> B\n";

    head.store(nullptr, std::memory_order_release);    // pop B
    std::cout << "[T2] pop(B): head -> null\n";

    recycle_node(A);   // A's address is now available for reuse
    std::cout << "[T2] recycle(A) — address " << A << " is freed\n";

    // Step 3: Thread T2 allocates C at A's recycled address, pushes C
    Node* C = alloc_node(3);  // same address as A!
    head.store(C, std::memory_order_release);
    std::cout << "[T2] alloc(C) at " << C << " (reuses A's address, val=3)\n";
    std::cout << "[T2] push(C): head -> C\n\n";

    // Step 4: Thread T1 resumes — CAS(&head, observed=A, new_head=B)
    // head is C, which has the same address as A.
    // (observed) == (C) == address of A.
    bool cas_result = head.compare_exchange_strong(observed, new_head);
    std::cout << "[T1] CAS(&head, " << observed << ", " << new_head << ")"
              << " = " << std::boolalpha << cas_result << "\n";

    if (cas_result) {
        Node* current_head = head.load();
        std::cout << "[T1] head now = " << current_head
                  << " which is FREED MEMORY (B was deleted)!\n";
        std::cout << "[T1] ABA BUG CONFIRMED — data structure corrupted.\n";
    } else {
        std::cout << "[T1] CAS failed — no bug this run.\n";
    }

    // Cleanup (in a real program we would leak C here — for the demo, delete it).
    delete C;
    delete B;

    std::cout << "\n=== End of ABA Demo ===\n";
}

// ============================================================================
// Step 4: Tagged Pointer (ABA Solution)
// ============================================================================
// Embed a version counter in the lowest 3 bits of the pointer.
// Pointers are at least 8-byte aligned → bottom 3 bits are always 0.
// Every CAS increments the tag. Even if the address cycles, the tag
// won't match → CAS fails → safe retry.

constexpr uintptr_t TAG_MASK = 0x7;  // 3 bits
constexpr uintptr_t PTR_MASK = ~TAG_MASK;

inline uintptr_t pack(Node* ptr, uintptr_t tag) {
    return (reinterpret_cast<uintptr_t>(ptr) & PTR_MASK) | (tag & TAG_MASK);
}

inline std::pair<Node*, uintptr_t> unpack(uintptr_t val) {
    return {reinterpret_cast<Node*>(val & PTR_MASK), val & TAG_MASK};
}

class TaggedStack {
    std::atomic<uintptr_t> head_{0};  // packed: address | tag
public:
    void push(Node* node) {
        uintptr_t old = head_.load(std::memory_order_acquire);
        do {
            auto [ptr, tag] = unpack(old);
            node->next = ptr;
            uintptr_t new_tag = (tag + 1) & TAG_MASK;
            uintptr_t desired = pack(node, new_tag);
            if (head_.compare_exchange_weak(old, desired,
                                            std::memory_order_release,
                                            std::memory_order_relaxed))
                return;
        } while (true);
    }

    Node* pop() {
        uintptr_t old = head_.load(std::memory_order_acquire);
        while (true) {
            auto [ptr, tag] = unpack(old);
            if (!ptr) return nullptr;
            Node* next = ptr->next;
            uintptr_t new_tag = (tag + 1) & TAG_MASK;
            uintptr_t desired = pack(next, new_tag);
            if (head_.compare_exchange_weak(old, desired,
                                            std::memory_order_release,
                                            std::memory_order_relaxed))
                return ptr;
        }
    }

    std::atomic<uintptr_t>& debug_head() { return head_; }
    uintptr_t debug_head_val() const { return head_.load(); }
};

void test_tagged_stack() {
    // Reproduce the ABA scenario but with tagged pointer protection.
    // The CAS should FAIL and retry, preventing corruption.

    std::cout << "\n=== Tagged Pointer — ABA Prevention ===\n\n";

    Node* A = alloc_node(1);
    Node* B = alloc_node(2);
    A->next = B;

    TaggedStack stack;
    stack.push(B);
    stack.push(A);
    // Stack: A -> B -> null, head = pack(A, tag=0)

    uintptr_t head_val_before = stack.debug_head_val();
    auto [ptr_before, tag_before] = unpack(head_val_before);
    std::cout << "Initial: head = pack(" << ptr_before << ", tag=" << tag_before << ")\n";

    // Simulate the interleaving
    // Step 1: read head
    uintptr_t observed = head_val_before;
    auto [observed_ptr, observed_tag] = unpack(observed);
    Node* observed_next = observed_ptr->next;  // B
    std::cout << "[T1] load(head) = pack(" << observed_ptr
              << ", tag=" << observed_tag << ")\n";

    // Step 2: pop A, pop B, recycle A
    stack.pop();                    // pops A
    stack.pop();                    // pops B (stack empty now)

    // head is now pack(nullptr, tag=2) — tag incremented twice
    uintptr_t after_pop = stack.debug_head_val();
    std::cout << "[T2] after 2 pops: head = " << after_pop << "\n";

    // Recycled A will be reused
    Node* C = alloc_node(3);        // same address as A
    recycle_node(nullptr);          // reset recycler

    // Try to push C with the tagged stack (which increments tag)
    stack.push(C);
    uintptr_t after_push = stack.debug_head_val();
    auto [push_ptr, push_tag] = unpack(after_push);
    std::cout << "[T2] push(C at " << push_ptr << "): head = pack("
              << push_ptr << ", tag=" << push_tag << ")\n";

    // Step 3: T1 tries CAS with the old tag
    // In a real tagged stack, T1 would use the full head value (including tag)
    // But here we manually test: CAS(&head_, observed, pack(B, tag+1))
    // observed has tag=0, but head now has tag=3
    // They DON'T match → CAS fails → no corruption
    uintptr_t new_val = pack(observed_next, (observed_tag + 1) & TAG_MASK);
    bool cas_result = stack.debug_head().compare_exchange_strong(observed, new_val);

    std::cout << "[T1] CAS(&head, " << observed << ", " << new_val << ")"
              << " = " << std::boolalpha << cas_result << "\n";

    if (!cas_result) {
        auto [final_ptr, final_tag] = unpack(observed);
        std::cout << "[T1] CAS correctly FAILED — observed updated to pack("
                  << final_ptr << ", tag=" << final_tag << ")\n";
        std::cout << "Tagged pointer prevented ABA! ✓\n";
    } else {
        std::cout << "[T1] CAS succeeded (unexpected — tag collision)\n";
    }

    // Drain remaining nodes
    delete B;
    while (stack.pop()) {}
}

// ============================================================================
// Main
// ============================================================================

int main() {
    std::cout << "=== Phase 13.07: Atomics, CAS, ABA Problem (C++) ===\n\n";

    std::cout << "--- Step 1: Atomic Counter (FAA vs Mutex) ---\n";
    std::cout << "  8 threads x 1,000,000 increments each\n";
    bench_counter();

    std::cout << "\n--- Step 2: Lock-Free Stack ---\n";
    test_lockfree_stack();

    std::cout << "\n--- Step 3: The ABA Problem ---\n";
    demonstrate_aba();

    std::cout << "\n--- Step 4: Tagged Pointer Solution ---\n";
    test_tagged_stack();

    std::cout << "\n=== All steps completed. ===\n";
    return 0;
}
