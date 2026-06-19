# Low-Latency C++ Reference Card

Quick-reference for the idioms and patterns from Lesson 15.

## Pattern Summary

| Idiom | Latency Saved | Key Mechanism | When to Use |
|-------|---------------|----------------|-------------|
| Object pool | 200ns–50μs per alloc | Free-list, no heap | Reusable objects on hot path |
| Arena allocator | 200ns–50μs per alloc | Bump pointer, bulk free | Many small allocs with same lifetime |
| SPSC ring buffer | 40–100ns vs mutex | Atomic acquire/release | One producer, one consumer |
| `alignas(64)` | 20–200ns false sharing | Separate cache lines | Hot variables across threads |
| `[[likely]]`/`[[unlikely]]` | 5–15ns per mispredict | Branch layout hint | Skewed branch probabilities |
| CRTP (vs virtual) | 5–15ns per call | Static dispatch + inline | Known set of types at compile time |
| Seqlock | 40–100ns vs mutex | Optimistic read with seq check | Read-heavy, write-rare shared data |
| `mmap`/`pread` | 500ns–10μs per syscall | Avoid kernel context switch | File I/O on hot path |
| Batch syscalls | N× syscall overhead | `readv`/`writev` | Multiple I/O operations |
| `string_view`/`span` | 30–100ns per copy | No allocation | Passing substrings/views |
| `constexpr` | Compile-time cost | Zero runtime | Constants, look-up tables |
| Pre-alloc `.reserve(N)` | Realloc spike | No mid-path growth | Growing containers |

## SPSC Ring Buffer — Quick Code

```cpp
template <typename T, size_t N>  // N must be power of 2
class SPSCRingBuffer {
    static constexpr size_t kMask = N - 1;
    alignas(64) std::atomic<size_t> write_pos_{0};
    alignas(64) std::atomic<size_t> read_pos_{0};
    alignas(64) std::array<T, N> buf_{};

public:
    bool try_push(const T& val) {
        size_t wp = write_pos_.load(std::memory_order_relaxed);
        size_t rp = read_pos_.load(std::memory_order_acquire);
        if (wp - rp >= N) return false;      // full
        buf_[wp & kMask] = val;
        write_pos_.store(wp + 1, std::memory_order_release);
        return true;
    }

    std::optional<T> try_pop() {
        size_t rp = read_pos_.load(std::memory_order_relaxed);
        size_t wp = write_pos_.load(std::memory_order_acquire);
        if (rp == wp) return std::nullopt;    // empty
        T val = buf_[rp & kMask];
        read_pos_.store(rp + 1, std::memory_order_release);
        return val;
    }
};
```

**Key points:**
- `write_pos_` and `read_pos_` on separate cache lines (alignas)
- `acquire` load before reading data, `release` store after writing data
- Bitmask `& kMask` instead of `% N`
- Monotonically increasing indices (wrap is safe with `size_t`)

## Object Pool — Quick Code

```cpp
template <typename T, size_t N>
class ObjectPool {
    struct Node { T data; Node* next; };
    std::array<Node, N> storage_;
    Node* free_list_;
public:
    ObjectPool() {
        for (size_t i = 0; i < N; ++i) {
            storage_[i].next = free_list_;
            free_list_ = &storage_[i];
        }
    }
    T* acquire() {
        if (!free_list_) return nullptr;
        Node* n = free_list_; free_list_ = n->next;
        return &n->data;
    }
    void release(T* p) {
        auto* n = reinterpret_cast<Node*>(p);
        n->next = free_list_; free_list_ = n;
    }
};
```

## Seqlock — Quick Code

```cpp
// Writer: seq++; write_data; seq++;
// Reader: retry until seq is even and unchanged across the read
void write(int a, double b) {
    seq_.store(seq_.load(relaxed) + 1, release);
    data_.a = a; data_.b = b;
    atomic_thread_fence(release);
    seq_.store(seq_.load(relaxed) + 1, release);
}

Data read() {
    uint64_t s1, s2; Data copy;
    do {
        s1 = seq_.load(acquire);
        if (s1 & 1) { yield(); continue; }
        copy = data_;
        atomic_thread_fence(acquire);
        s2 = seq_.load(acquire);
    } while (s1 != s2);
    return copy;
}
```

## CRTP vs Virtual — Quick Code

```cpp
// Virtual (runtime dispatch, vtable indirection)
struct Base { virtual int run(int) = 0; };
struct Derived : Base { int run(int x) override { return x * x; } };

// CRTP (compile-time dispatch, likely inlined)
template <typename D> struct Base { int run(int x) { return static_cast<D*>(this)->run_impl(x); } };
struct Derived : Base<Derived> { int run_impl(int x) { return x * x; } };
```

## Memory Ordering Cheat Sheet

| Ordering | Cost (x86) | Guarantee | Use When |
|----------|-----------|-----------|----------|
| `relaxed` | ~0ns extra | No ordering, only atomicity | Counters, stats |
| `acquire` | ~1ns (lfence) | See all writes before matching release | Consumer loads |
| `release` | ~1ns (sfence) | All prior writes visible before this store | Producer stores |
| `acq_rel` | ~1ns | Both acquire and release | Read-modify-write (fetch_add) |
| `seq_cst` | ~5-10ns (mfence) | Global total order | Rarely needed; when all threads must agree |

On x86, `acquire`/`release` are nearly free (TSO guarantees most ordering). On ARM/RISC-V, they emit real barriers.

## Common Pitfalls

| Pitfall | Symptom | Fix |
|---------|---------|-----|
| False sharing on atomics | P99 latency spikes under contention | `alignas(64)` each independent atomic |
| Using `seq_cst` everywhere | Unnecessary mfence instructions | Use `acquire`/`release` for SPSC |
| Ring buffer with `pos % N` | Variable-latency division | Power-of-2 N, use `pos & (N-1)` |
| Allocating in hot loop | P99 spikes from page faults | Pool or arena pre-allocate |
| Virtual call on hot path | I-cache miss on vtable | CRTP or static dispatch |
| `std::string` copy | Hidden allocation | `std::string_view` or `std::span` |
| Syscall per message | 500ns–10μs per call | Batch with `readv`/`writev` or mmap |
| Forgetting `[[likely]]` on 99/1 branch | Branch mispredict on fast path | Profile, then hint; don't hint 50/50 branches |
| SPSC queue without cache-line padding | Producer and consumer invalidate each other's line | `alignas(64)` on read/write indices |
| Reading seqlock during write (odd seq) | Torn read | Retry until seq is even and stable |

## Benchmark Reference (Typical x86, -O2)

These are representative numbers from `code/main.cpp`. Actual results vary by hardware.

| Idiom | Typical ns/op | Notes |
|-------|--------------|-------|
| Object pool acquire/release | 5–15 | ~10-100x faster than new/delete |
| Arena allocate | 2–5 | Near-zero: just a pointer increment |
| SPSC queue push/pop | 15–50 | Acquire/release, no contention |
| CRTP vs virtual | 1–2 vs 3–8 | CRTP inlines, virtual has indirection |
| `alignas(64)` counters | 1–2 vs 5–20 | Padded avoids false sharing completely |
| `seq_cst` vs `relaxed` | 5–10 vs 0–1 | mfence cost on x86 |
| Seqlock read | 10–30 | Optimistic, no locking |
| `string_view` vs string copy | 2–5 vs 30–100 | No allocation in view path |