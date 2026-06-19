# Output: Lock-Free Data Structures — Treiber Stack & MS Queue

## Artifact

A self-contained Treiber stack and Michael-Scott queue implementation with ABA-counter protection, plus a mutex-based baseline for performance comparison.

## Files

| File | Description |
|------|-------------|
| `../code/main.rs` | Rust: TreiberStack\<T\>, MSQueue\<T\>, MutexStack\<T\>, MutexQueue\<T\> with concurrent test harness and benchmarks |
| `../code/main.cpp` | C++: TreiberStack\<T\>, MSQueue\<T\>, MutexStack\<T\>, MutexQueue\<T\> with concurrent test harness and benchmarks |

## Usage

### Rust

```bash
cd code
rustc main.rs -o treiber_ms && ./treiber_ms
```

Requires Rust 1.60+ (for `std::sync::atomic::AtomicUsize`).

### C++

```bash
cd code
clang++ -std=c++20 -pthread main.cpp -o treiber_ms && ./treiber_ms
```

Requires C++20 and `-pthread` (or `/MT` on MSVC).

## Benchmark Expectations

Running with 4 threads and 50,000 ops/thread:

| Structure | Lock-free | Mutex-based | Speedup |
|-----------|-----------|-------------|---------|
| Stack     | ~1-2M ops/s | ~150-200k ops/s | ~6-10x |
| Queue     | ~800k-1.2M ops/s | ~120-180k ops/s | ~5-8x |

Results vary by CPU generation, core count, and OS scheduling behavior.

## Key Design Decisions

1. **ABA counter packed into high 16 bits** of the pointer word. Incremented on every successful CAS. Eliminates ABA without external memory reclamation (at the cost of limiting heap to low 48 bits of address space — safe on x86-64 and ARM64).

2. **MS queue dummy node** allocated once in `new()`. Head always points to the dummy; tail points to the last node (or dummy when empty).

3. **Helping** in MS queue: if a thread finds tail points to a node whose `next` is non-null, it advances tail before retrying. This is essential for lock-freedom — a thread that is descheduled between linking and tail-advance does not stall the queue.

4. **Compare-exchange weak** used in C++ (spurious failure is OK in retry loops, and `weak` is faster on ARM). Rust uses `compare_exchange` (strong, since Rust lacks `compare_exchange_weak` in stable).

## Integration

Drop `TreiberStack` or `MSQueue` into any concurrent Rust/C++ project needing a lock-free LIFO/FIFO. Replace the `delete` / `Box::from_raw` deallocation with hazard pointers or epoch-based reclamation for production use where nodes are long-lived or allocation is frequent.
