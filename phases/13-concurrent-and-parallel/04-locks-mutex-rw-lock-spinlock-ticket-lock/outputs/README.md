# Lesson 13.04 — Outputs

## Artifact: Lock Implementation & Benchmark Suite

This directory contains the compiled binaries and measurement data produced by the lesson code.

### Files

| File | Source | Description |
|------|--------|-------------|
| `locks_bench` | `code/main.c` | C benchmark binary (compile with `clang -std=c11 -pthread -O2 -o locks_bench ../code/main.c`) |
| `lock_demo` | `code/main.rs` | Rust demo binary (compile with `rustc ../code/main.rs -o lock_demo`) |
| `perf_output.txt` | `perf stat ./locks_bench` | Perf measurements on Linux showing cycles, cache misses, context switches per lock type |

### How to Generate

```bash
# C benchmark
clang -std=c11 -pthread -O2 -o locks_bench ../code/main.c
./locks_bench

# On Linux, profile with perf:
perf stat ./locks_bench 2> perf_output.txt

# Rust demos
rustc ../code/main.rs -o lock_demo
./lock_demo
```

### Expected Output

The C benchmark prints a table with wall-clock times at 1, 2, and 4 threads for each lock type. The Rust demos show Mutex correctness, lock poisoning recovery, RwLock with concurrent readers, and a throughput comparison.

### Reuse

Use the lock implementations (spinlock, ticket lock, mutex) as reference for Phase 13 lessons 07–09 (lock-free data structures, atomics). The benchmark harness can be adapted to measure contention in your own concurrent code.
