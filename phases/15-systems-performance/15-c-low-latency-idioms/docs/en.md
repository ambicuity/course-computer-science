# C++ Low-Latency Idioms

> Microseconds matter. In high-frequency trading, real-time audio, and game engines, a single cache miss or
> unexpected allocation can blow your latency budget. This lesson teaches the idioms that let C++ programs
> hit deterministic, sub-microsecond response times.

**Type:** Learn
**Languages:** C++17
**Prerequisites:** Phase 15 lessons 01–14 (caches, branches, I/O, profiling)
**Time:** ~90 minutes

## Learning Objectives

- Explain what "low-latency" means quantitatively and why microseconds matter.
- Avoid dynamic allocation at hot-path points using object pools, arena allocators, and custom `new`/`delete`.
- Implement a single-producer single-consumer (SPSC) ring buffer.
- Use branch hints (`[[likely]]`, `[[unlikely]]`, `__builtin_expect`) and understand their effect.
- Align data to cache lines with `alignas(64)` to eliminate false sharing.
- Replace locks with lock-free patterns (atomic SPSC, seqlock).
- Avoid syscalls and blocking I/O via batching, `mmap`, and `pread`.
- Replace virtual dispatch with CRTP for static polymorphism.
- Use `string_view`, `span`, `constexpr`, and pre-allocation to remove hidden costs.
- Reason about memory ordering (`acquire`/`release`/`seq_cst`) for correct lock-free code.

## The Problem

You work on a market-data gateway that processes 10 million messages per second. Your boss says
"latency must stay under 5 μs at the 99th percentile." You profile and find:

| Offender | Typical Cost | Why It Hurts |
|----------|-------------|--------------|
| `new`/`delete` on hot path | 200 ns–50 μs | Heap lock contention, fragmentation |
| `std::mutex` in SPSC queue | 40–100 ns | Kernel arbitration if contested |
| Virtual call + vtable miss | 5–15 ns | Indirection defeats branch predictor |
| False sharing on shared counter | 20–200 ns | Cache-line ping-pong across cores |
| `read()` syscall per message | 500 ns–10 μs | Context switch to kernel |
| `std::string` copy in parse loop | 30–100 ns | Allocates on heap |

Individually these look small. Together they compound: a single `new` can trigger a page fault
that costs 10 μs — blowing your entire budget. The discipline of low-latency programming is about
*eliminating every source of non-determinism*.

## The Concept

### 1. What Low-Latency Means

Low-latency programming is the pursuit of **deterministic response time**. The goal is not just
small *average* latency but small *tail* latency (P99, P99.9). A system that responds in 1 μs 99%
of the time but takes 1 ms 1% of the time is *worse* than one that always responds in 3 μs.

Key insight: **latency distributions have fat tails**. One allocation that hits a page fault
dominates your P99. The discipline is about cutting off the tail, not lowering the mean.

```
Latency Distribution (Before Optimization)
  |████████████▓▓▓▓░░░░░░░░░  ← fat tail from allocations, syscalls
  0μs                      10μs+

Latency Distribution (After Optimization)
  |████████████░░░░           ← tight cluster, no outliers
  0μs        5μs
```

### 2. Allocation Avoidance

Every `new`/`delete` pair is a potential latency spike:

- **Object pools** — pre-allocate `N` objects, hand them out and recycle them. No heap involvement.
- **Arena allocators** — bump-allocate from a contiguous slab; free the whole arena at once.
- **Custom `operator new`/`delete`** — redirect hot-path types to a pool or arena.

**Object Pool Sketch:**
```
[Free List] → obj0 → obj1 → obj2 → nullptr
                ↑ acquire()
obj0 → obj1 → nullptr  (obj2 is now "in use")
                ↓ release(obj2)
[Free List] → obj2 → obj0 → obj1 → nullptr
```

**Arena Allocator Sketch:**
```
Arena: [################                    ]
       ^bump pointer        ^end
       allocate: return bump; bump += size
       reset: bump = start  (free all at once)
```

### 3. Ring Buffers / SPSC Queues

A ring buffer is a fixed-size circular array with a read index and write index. When only one
thread produces and one thread consumes, the indices can be plain `std::atomic` with relaxed or
acquire/release ordering — no locks needed.

```
size_ = 8 (power of 2)

     write_pos
         ↓
[____][DATA][DATA][DATA][____][____][____][____]
         ↑
     read_pos

Available = write_pos - read_pos  (mod 2*size_)
Free      = size_ - available
```

Why power of 2? So `pos & (size-1)` replaces `pos % size`, and the compiler emits a mask instead
of a division.

### 4. Branch Hints

Modern CPUs predict branches, but a mispredict costs ~15 ns (pipeline flush). `[[likely]]` and
`[[unlikely]]` (C++20) or `__builtin_expect` (GCC/Clang) tell the compiler which path to lay out
in the "fast" (fall-through) position in the instruction cache.

```cpp
if (order_is_valid) [[likely]] {
    process_order(order);    // hot path — laid out first
} else {
    log_error(order);        // cold path — moved out of line
}
```

Trade-off: hints only help when the compiler can reorder the machine layout. If the branch is
50/50, hints *hurt*.

### 5. Cache-Line Padding

A 64-byte cache line is the smallest unit the CPU fetches. If two hot variables used by different
cores sit on the same line, every write forces the other core to invalidate — **false sharing**.

```
Bad:   | counter_A (8B) | counter_B (8B) | ... padding ... |  ← same line!
Good:  | counter_A (8B) | ... 56B padding ... | counter_B (8B) | ... 56B padding ... |
       alignas(64) int counter_A;
       alignas(64) int counter_B;
```

### 6. Avoiding Locks

- **Lock-free SPSC queue** — `atomic<size_t>` with `memory_order_acquire`/`release` for read/write positions.
- **Seqlock** — a sequence counter that writers increment before/after mutation; readers spin-wait
  until the counter is even (no writer active), then read, then verify the counter didn't change.

```
Writer:  seq++; write_data; seq++;
Reader:  do { s1 = seq; read_data; s2 = seq; } while (s1 != s2 || s1 & 1);
```

### 7. Avoiding Syscalls and I/O

Each syscall costs 500 ns–10 μs (context switch, cache pollution). Strategies:

| Technique | Mechanism | Savings |
|-----------|-----------|---------|
| Batch `writev`/`readv` | One syscall for N operations | N× fewer syscalls |
| `mmap` | Map file into address space | No `read()` syscalls at all |
| `pread` | Read at offset without `lseek` | Avoids 2 syscalls for seek+read |
| io_uring | Kernel-side queue, batched completion | Near-zero syscall overhead |

### 8. Avoiding Dynamic Dispatch

Virtual calls require a vtable indirection (memory load) and defeat devirtualization / inlining. CRTP
(Curiously Recurring Template Pattern) resolves the call at compile time:

```cpp
// Virtual (runtime dispatch, potential cache miss on vtable)
struct Base { virtual void run() = 0; };
struct Derived : Base { void run() override { /* ... */ } };
Base* b = new Derived; b->run();  // indirect call

// CRTP (compile-time dispatch, inlined)
template<typename Derived>
struct Base { void run() { static_cast<Derived*>(this)->run_impl(); } };
struct Derived : Base<Derived> { void run_impl() { /* ... */ } };
Derived d; d.run();  // direct call, likely inlined
```

### 9. Pre-allocation, string_view, span, constexpr

- **Pre-allocate** vectors to `.reserve(N)` so no `realloc` happens mid-hot-path.
- **`std::string_view`** / **`std::span`** — non-owning, no allocation. Pass views instead of copies.
- **`constexpr`** — compute at compile time. `constexpr size_t kBufSize = 1 << 20;` costs zero at runtime.

### 10. Memory Ordering

C++ atomics default to `memory_order_seq_cst` (total order, most expensive). For SPSC:

- **Producer**: `store(pos, memory_order_release)` — all prior writes are visible before the store.
- **Consumer**: `load(pos, memory_order_acquire)` — all writes before the matching release are visible.
- **Seq-cst** is only needed when *all threads must agree on a total order* (rare in practice).

Use the weakest ordering that is correct. `relaxed` for counters that don't synchronize data.
`acquire`/`release` for producer-consumer. `seq_cst` only when you need a global lock step.

## Build It

### Step 1: Minimal SPSC Ring Buffer

A barebones queue that compiles and works for a single-threaded test. No atomics yet.

```cpp
template<typename T, size_t N>
class SPSCRingBuffer {
    std::array<T, N> buf_{};
    size_t head_ = 0, tail_ = 0;
public:
    bool push(const T& val) {
        if (size() == N) return false;
        buf_[head_ % N] = val;
        ++head_;
        return true;
    }
    std::optional<T> pop() {
        if (head_ == tail_) return std::nullopt;
        T val = buf_[tail_ % N];
        ++tail_;
        return val;
    }
    size_t size() const { return head_ - tail_; }
};
```

### Step 2: Realistic Version (Thread-Safe, Cache-Aligned)

See `code/main.cpp` for the full implementation with:
- `std::atomic` with acquire/release ordering
- Cache-line padding to prevent false sharing
- Power-of-2 sizing with bitmask instead of modulo
- Object pool with free-list
- CRTP vs virtual benchmark
- Memory ordering comparison

## Use It

### Production Equivalents

| Idiom | Production Library |
|-------|--------------------|
| SPSC queue | `boost::lockfree::spsc_queue`, `folly::ProducerConsumerQueue` |
| Object pool | `boost::object_pool`, `jemalloc` tcache |
| Arena allocator | `folly::Arena`, `bumpalo` (Rust) |
| Seqlock | Linux kernel `seqlock_t`, `folly::SharedMutex` (read-biased) |
| CRTP | Various in LLVM, Eigen expression templates |

**Key difference** from your hand-built version: `folly::ProducerConsumerQueue` uses a compile-time
fixed size template parameter and places the read/write cursors on separate cache lines. Your
implementation should do the same once you add `alignas(64)`.

### What production does that minimal code doesn't

- Poison padding bytes after destruction for ASAN hygiene
- Handle ABA problem for indices that wrap around (use `size_t` and rely on monotonic wrap)
- Provide `try_push` / `try_pop` returning immediately (never blocking)
- Destruct objects in-place when the queue is destroyed

## Read the Source

- **`folly/ProducerConsumerQueue.h`** — Facebook's single-producer single-consumer queue. Note
  the `alignas(hardware_destructive_interference_size)` on `readIdx_` and `writeIdx_`.
- **Linux kernel `include/linux/seqlock.h`** — The canonical seqlock implementation. Notice how
  readers retry on odd sequence numbers.

## Ship It

The reusable reference card lives in `outputs/lowlatency_reference.md`. It contains:

- Quick-reference tables for every idiom in this lesson
- Benchmark numbers from the `code/main.cpp` measurements
- Common pitfalls and their fixes

## Exercises

1. **Easy** — Modify the SPSC ring buffer to use a bitmask instead of modulo. Verify the
   benchmark shows improvement.
2. **Medium** — Implement a seqlock that protects a 64-byte market-data snapshot struct.
   Test with 1 writer thread and 4 reader threads. Measure P99 read latency.
3. **Hard** — Build a lock-free multi-producer single-consumer (MPSC) queue. Reason about
   why `memory_order_acq_rel` is needed for the producer side but `acquire` suffices for the
   consumer. Prove (or argue convincingly) that no ABA problem arises.

## Key Terms

| Term | What people say | What it actually means |
|------|-----------------|------------------------|
| Low-latency | "Fast" | Deterministic worst-case response time (P99), not average speed |
| Lock-free | "No locks" | At least one thread always makes progress; no mutex blocking |
| SPSC | "Thread-safe queue" | Exactly one producer thread and one consumer thread (no contention) |
| Cache-line padding | "Alignment" | Placing volatile data on separate 64B lines so cores don't invalidate each other |
| Acquire/release | "Memory barrier" | Acquire = see all writes before the matching release; release = make all prior writes visible |
| Seqlock | "Sequence lock" | Optimistic read: note sequence, read data, verify sequence unchanged. Retry on conflict. |
| CRTP | "Template pattern" | Compile-time polymorphism that replaces virtual calls with static dispatch + inlining |
| False sharing | "Slow counter" | Two independent variables on the same cache line, causing unnecessary invalidation traffic |
| Arena allocator | "Bump allocator" | Allocate by incrementing a pointer; free by resetting the pointer. O(1) alloc, O(1) bulk free. |
| `[[likely]]` | "Branch prediction hint" | Informs the compiler which branch is more probable so it lays out the hot path first |

## Further Reading

- **Martin Thompson, "Mechanical Sympathy" blog** — The canonical series on low-latency Java/C++.
- **`folly/ProducerConsumerQueue.h`** — Facebook's battle-tested SPSC queue.
- **Herb Sutter, "atomic Weapons" talk (CppCon 2012)** — Deep dive on memory ordering.
- **David Dryjowicz, "Low-Latency C++ for Finance" video** — HFT-specific patterns.
- **Linux kernel `seqlock.h` source** — Production seqlock with Read-Copy Update integration.
- **ISO C++ `P0476R5`** — `[[likely]]`/`[[unlikely]]` proposal rationale.
- **Ulrich Drepper, "What Every Programmer Should Know About Memory"** — Cache-line fundamentals.