# Lesson 13.22 — Outputs

## Artifact: Work-Stealing Scheduler + Lock-Free Queue

This directory contains the compiled binary and benchmark output produced by the lesson code.

### Files

| File | Source | Description |
|------|--------|-------------|
| `work-stealing-scheduler` | `code/` (Rust, cargo) | Rust binary with Chase-Lev deque, MS queue, work-stealing pool, and benchmark suite (compile with `cargo build --release` from `code/`) |
| `benchmark_output.txt` | Rust benchmark output | Timing and statistics for Fibonacci, parallel map, and tree traversal benchmarks across three scheduler variants (work-stealing, thread-per-task, mutex pool) |

### How to Generate

```bash
# Rust (stdlib only — no external dependencies)
cd ../code
cargo build --release
./target/release/work-stealing-scheduler
```

### Expected Output

The binary prints four sections:

1. **Verification** — sequential and concurrent correctness tests for the Chase-Lev deque, MS queue, and work-stealing pool.
2. **Fibonacci Benchmark** — 512 × fib(25) tasks comparing sequential, work-stealing pool, thread-per-task, and mutex pool throughput.
3. **Parallel Map Benchmark** — 1,000,000-element map in 4 chunks, sequential vs. work-stealing pool.
4. **Tree Traversal Benchmark** — 16 trees (depth 18) summed in parallel vs. sequentially.
5. **Work-Stealing Statistics** — per-benchmark counts of tasks executed locally, stolen, and from submission queue, plus steal success rate.

Example excerpt:
```
─── Benchmark: Fibonacci (fib(25), 512 tasks, 4 workers) ───
  Sequential              0.412s (1243/s)
  Work-Stealing Pool      0.142s (3606/s)
  Thread-per-Task         0.891s (575/s)
  Mutex Pool              0.523s (979/s)
```

### Reuse

Use the work-stealing pool as a drop-in `Box<dyn FnOnce() + Send>` task scheduler for concurrent Rust programs that need dynamic load balancing. The Chase-Lev deque can be extracted as a standalone lock-free deque for producer–consumer workloads. The MS queue is suitable as a lock-free channel for single-consumer multi-producer scenarios.
