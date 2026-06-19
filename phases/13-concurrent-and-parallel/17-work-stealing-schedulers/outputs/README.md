# Reusable Artifact: Work-Stealing Deque & Thread Pool

This directory contains a reusable implementation of:

## Chase-Lev Work-Stealing Deque

A lock-free double-ended queue that supports:
- **Owner push/pop** from bottom (LIFO) — `WorkDeque::push`, `WorkDeque::pop`
- **Concurrent steal** from top (FIFO) — `WorkDeque::steal`

No mutexes, no blocking. Uses only `AtomicIsize`, `AtomicPtr`, and CAS.
Suitable as a building block for custom task schedulers, fork-join runtimes,
or async executors.

### Key Properties

- **Lock-free**: at least one thread makes progress on every operation.
- **Linearizable**: push is visible to subsequent pop/steal.
- **Growable**: circular buffer doubles when full (old buffers leaked for
  correctness).
- **Memory ordering**: documented per-operation in the source.

## Work-Stealing Thread Pool

A minimal `WorkStealingPool` that distributes tasks round-robin across
workers and uses random-victim work-stealing for load balancing. Includes:

- **Random victim selection** (XorShift PRNG per thread, no contention).
- **Termination detection** via atomic counter.
- **Correctness stress test** with concurrent owner + thieves.

## Usage

Copy `WorkDeque<T>` and `WorkStealingPool` into your project. Dependencies:
Rust standard library only (no crates required).

```rust
let pool = WorkStealingPool::new(num_workers, tasks);
pool.wait();
```

## Benchmark Reference

Performance characteristics on a compute-bound workload
(e.g., 64 × fib(42) on 4 cores):

| Approach | Expected relative time |
|----------|----------------------|
| thread-per-task | baseline (slowest) |
| mutex-based pool | ~2–5× faster than baseline |
| work-stealing pool | ~3–8× faster than baseline |

Numbers vary by hardware and workload. The key insight is monotonic:
work-stealing improves with core count; mutex-based pools degrade.
