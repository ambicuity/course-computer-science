# Concurrency vs Parallelism — Reference Notes

> Reference companion to `docs/en.md`. Use these notes as a quick lookup during the rest of Phase 13.

---

## Core Definitions

**Concurrency** — Decomposing a program into independently executing tasks that can be composed, paused, and resumed. A design concern: *how you structure the program.*

**Parallelism** — Executing multiple operations simultaneously on different hardware units. An execution concern: *how many operations run at the same instant.*

**Rob Pike's distinction (2012):**
> Concurrency is about *dealing with* lots of things at once. Parallelism is about *doing* lots of things at once.

**Key insight:** Concurrency enables parallelism, but they are orthogonal. A concurrent program can run on a single core (time-slicing). A parallel program needs multiple cores. The same concurrent design can scale from 1 to 128 cores with no structural changes.

---

## Comparison Table

| Aspect | Concurrency | Parallelism |
|--------|-------------|-------------|
| Focus | Program structure | Execution efficiency |
| Question answered | How is work decomposed? | How many ops at once? |
| Hardware needed | Single core | Multiple cores |
| Goal | Responsiveness, clarity, composability | Throughput, speed |
| Measure | Number of tasks managed | Operations / second |
| Primary cost | Context switching | Synchronization, cache contention |
| Debugging challenge | Race conditions, deadlocks | Race conditions + non-deterministic replay |
| Example | Event loop handling 10k connections | SIMD vector addition |
| When it helps | I/O-bound workloads | CPU-bound compute workloads |
| When it hurts | CPU-bound on single core (overhead) | Work items too small (sync dominates) |

---

## Concurrency Models Comparison

### Overview

| Model | Memory | Communication | Execution unit | Example runtime | Best for |
|-------|--------|---------------|----------------|-----------------|----------|
| Processes | Isolated address spaces | IPC (pipes, sockets, shared mem, signals) | OS process | Unix, Linux, Windows | Security isolation, fault containment |
| Threads | Shared address space | Shared memory + locks | OS thread | pthreads, Win32 threads, Java threads | CPU-bound compute, tight-coupling |
| Async/await | Shared (single-threaded loop) | Futures, promises, callbacks | Task (user-level) | JavaScript, Python asyncio, C#, Rust | I/O-bound, high connection counts |
| Coroutines (stackful) | Shared per process | Channels, mutex, select | Goroutine (user-level) | Go, Lua, Erlang | Mixed workloads, composable |
| Actors | Isolated per actor | Mailbox (async messages) | Actor (user-level) | Erlang, Akka, Orleans | Fault-tolerant, distributed |
| CSP | Shared per process | Synchronous channels | Process/goroutine | Go, Clojure core.async | Pipeline, dataflow |

### Detailed View

**Processes**
- Creation cost: high (fork, allocate address space)
- Context switch: heavy (TLB flush, page table switch)
- Isolation: full (one process cannot crash another)
- Scaling limit: thousands (address space overhead)
- Debugging: easier (isolation limits bug propagation)
- Communication: explicit (serialize/deserialize through IPC)
- Use when: fault containment matters more than speed

**Threads**
- Creation cost: medium (kernel sees each thread)
- Context switch: moderate (same address space, different stack)
- Isolation: none (one thread crash takes down process)
- Scaling limit: hundreds (kernel thread overhead)
- Debugging: harder (shared state, race conditions)
- Communication: implicit (shared memory, but need synchronization)
- Use when: fine-grained shared state, CPU-bound compute

**Async/await (stackless coroutines)**
- Creation cost: very low (just a struct on the heap)
- Context switch: extremely cheap (function call, no kernel)
- Isolation: shared (single-threaded, no concurrent mutations without work-stealing)
- Scaling limit: millions (tiny per-task state)
- Debugging: mixed (stack traces are confusing across await points)
- Communication: via futures / promises (.then chaining or await)
- Use when: I/O-bound work, high connection counts

**Coroutines (stackful)**
- Creation cost: low (small stack, ~4 KB initial in Go)
- Context switch: cheap (save/restore registers, no syscall)
- Isolation: shared (goroutines share address space)
- Scaling limit: millions (Go routinely runs 100k+ goroutines)
- Debugging: easier (stackful = full traceback)
- Communication: channels (or shared memory with synchronization)
- Use when: mixed I/O and CPU, need simple concurrency primitives

**Actors**
- Creation cost: low (lightweight per-actor state)
- Context switch: moderate (mailbox dispatch)
- Isolation: per-actor (no shared state = no locks)
- Scaling limit: millions (Erlang VM handles millions of processes)
- Debugging: good (no shared state, but message ordering can be tricky)
- Communication: async messages (fire and forget or request/reply)
- Use when: distributed systems, fault tolerance, supervision trees

**CSP (Communicating Sequential Processes)**
- Creation cost: depends on implementation (goroutines for Go)
- Context switch: depends on implementation
- Isolation: shared per process (goroutines share memory but communicate via channels)
- Scaling limit: depends on implementation
- Debugging: moderate (channel blocking can cause deadlocks)
- Communication: synchronous channels (sender blocks until receiver is ready)
- Use when: pipeline processing, producer-consumer, dataflow

---

## Amdahl's Law

### Formula

```
S(N) = 1 / ((1 - P) + P/N)
```

Where:
- **P** = fraction of workload that can be parallelized (0.0 to 1.0)
- **N** = number of processors (cores)
- **S(N)** = speedup relative to single-core execution

### Speedup Table

| P | N=2 | N=4 | N=8 | N=16 | N=64 | N→∞ |
|---|-----|-----|-----|------|------|-----|
| 0.50 | 1.33 | 1.60 | 1.78 | 1.88 | 1.97 | 2.0 |
| 0.75 | 1.60 | 2.29 | 2.91 | 3.37 | 3.80 | 4.0 |
| 0.90 | 1.82 | 3.07 | 4.71 | 6.40 | 8.78 | 10.0 |
| 0.95 | 1.90 | 3.48 | 5.82 | 9.14 | 16.20 | 20.0 |
| 0.99 | 1.98 | 3.88 | 7.48 | 13.91 | 38.79 | 100.0 |

### Key Insight

The serial fraction (1-P) is the dominant term. Even with P = 0.99 (1% serial), the maximum speedup is 100x — not 1000x. Optimizing serial bottlenecks gives more benefit than adding cores.

---

## Gustafson's Law

### Formula

```
S(N) = N - (N - 1)(1 - P)
```

Where:
- **P** = parallel fraction of the *scaled* workload
- **N** = number of processors

### Scaled Speedup Table (same P as Amdahl but problem scales)

| P | N=2 | N=4 | N=8 | N=16 | N=64 | N→∞ |
|---|-----|-----|-----|------|------|-----|
| 0.50 | 1.50 | 2.50 | 4.50 | 8.50 | 32.50 | ~N/2 |
| 0.75 | 1.75 | 3.25 | 6.25 | 12.25 | 48.25 | ~3N/4 |
| 0.90 | 1.90 | 3.70 | 7.30 | 14.50 | 57.70 | ~9N/10 |
| 0.95 | 1.95 | 3.85 | 7.65 | 15.25 | 60.85 | ~19N/20 |
| 0.99 | 1.99 | 3.97 | 7.93 | 15.85 | 63.37 | ~99N/100 |

### Key Insight

Gustafson argues that Amdahl's fixed-size assumption is unrealistic. In practice, parallel workloads scale their problem size with the available resources. Weather simulations, ML training, and scientific computing all do this.

**Which to use:**
- **Amdahl:** Fixed-size dataset, latency-critical, strong scaling (same problem, more cores)
- **Gustafson:** Growing dataset, throughput-oriented, weak scaling (bigger problem, more cores)

---

## Three Levels of Concurrency

| Level | Granularity | Programmer control | Typical hardware | Example |
|-------|-------------|-------------------|------------------|---------|
| **Task-level** | Coarse (function/method) | High — explicit task decomposition | Multi-core CPU, cluster | Web server handling requests, MapReduce |
| **Data-level (SIMD)** | Fine (element) | Medium — vectorization hints | GPU, vector CPU units (AVX, NEON) | Image convolution, matrix multiply |
| **Instruction-level (ILP)** | Very fine (instruction) | Low — compiler + CPU handle it | CPU pipeline, superscalar | Any sequential code (pipelining, branch prediction) |

Most programmers work at the task level. Data-level parallelism requires specialized hardware knowledge (CUDA, AVX intrinsics). Instruction-level parallelism is automatic — you get it from modern CPUs without effort.

---

## Real-World System Classification

| System | Concurrent? | Parallel? | Mechanism |
|--------|-------------|-----------|-----------|
| Nginx (single worker) | Yes | No | Event loop + non-blocking I/O |
| Nginx (multi-worker) | Yes | Yes | One event loop per core |
| Node.js (default) | Yes | No | Single-threaded event loop + libuv thread pool for some I/O |
| Node.js (worker_threads) | Yes | Yes | Event loop + worker pool for CPU |
| Go (GOMAXPROCS=1) | Yes | No | Goroutines on 1 thread |
| Go (GOMAXPROCS=N) | Yes | Yes | Goroutines on N threads |
| Redis (single-threaded) | Yes | No | Event loop multiplexing clients |
| PostgreSQL | Yes | Yes (SELECT) | Process per connection + parallel query workers |
| TensorFlow training | Yes | Yes | Graph partitioned across devices + data parallelism |
| Python (single-threaded) | No (unless async) | No | GIL serializes execution |
| Python (multiprocessing) | Yes | Yes | Separate processes, each with own GIL |
| Java (single thread) | No | No | Sequential execution |
| Java (thread pool) | Yes | Yes (on multi-core) | Threads spawned and scheduled by OS |
| Erlang/OTP | Yes | Yes (typically) | Actors (processes) scheduled by BEAM VM |

---

## When NOT to Use Parallelism

### 1. Amdahl's Diminishing Returns
- Serial fraction of 5% caps speedup at 20x
- Going from 16 → 64 cores at P=0.95: 9.14x → 16.2x (only 1.8x more for 4x the cores)
- The first few cores give most of the benefit

### 2. Synchronization Overhead Dominates
- A mutex lock/unlock costs ~25 ns
- A cache-coherent atomic CAS on x86 costs ~15 ns
- If each work item takes 100 ns, lock overhead is 25% of the budget
- **Rule:** work / sync ratio should be > 10,000:1 in cycles

### 3. False Sharing
- Two cores write to different variables on the same 64-byte cache line
- The cache line "bounces" between cores, invalidating L1 each time
- Solution: pad data structures so that per-thread fields land on separate cache lines

```
Bad (false sharing):
  struct { int a; int b; }  // a and b on same cache line
  Thread 0 writes a, Thread 1 writes b → cache line bounces

Good (padding):
  struct { int a; char pad[60]; int b; char pad2[60]; }
```

### 4. I/O-Bound Workloads
- If the bottleneck is disk or network, adding cores doesn't help
- The CPU is idle waiting for I/O — adding more idle CPUs changes nothing
- Solution: concurrency (overlap I/O), not parallelism

### 5. Context-Switch Thrash
- More threads than cores → time-slicing
- Each context switch costs 1–10 μs (OS thread) or ~50 ns (goroutine)
- At 10,000 OS threads on 4 cores: each thread gets 0.04% of a core
- You spend more time switching than computing
- Solution: use thread pools or lightweight user-level tasks

### 6. Memory Bandwidth Saturation
- Modern DDR5: ~50 GB/s per channel (dual channel = ~100 GB/s)
- 64 cores each reading 2 GB/s = 128 GB/s demand
- Memory bus becomes the bottleneck
- Solution: optimize data locality, reduce memory footprint per thread

---

## Memory Models and Consistency

A **memory model** defines what values a read can return in a concurrent program.

### Sequential Consistency (SC)
- All operations appear to execute in some global total order
- Each thread's operations appear in program order
- **Cost:** prohibits all reordering (compiler + hardware)
- **Used by:** Java `volatile` (sort of), Rust `SeqCst` ordering

### Total Store Order (TSO)
- Writes are buffered (store buffer)
- A thread sees its own writes immediately, but other threads see them later
- **Used by:** x86/x64
- **Effect:** reads can pass writes to different addresses

### Relaxed / Weak Memory Models
- Almost no ordering guarantees
- Compiler and CPU can reorder freely
- **Used by:** ARM, RISC-V, C++20 `memory_order_relaxed`
- **Effect:** need explicit memory barriers/fences

### Common Reorderings

```
Initial: x = 0, y = 0

Thread 0:          Thread 1:
x = 1              y = 1
r1 = y             r2 = x

Under SC: (r1, r2) ∈ {(0,0), (0,1), (1,0), (1,1)} — all possible
Under TSO: (r1=0, r2=0) possible — Thread 0's x=1 is in store buffer
Under relaxed: (r1=0, r2=0) more likely — writes can be delayed arbitrarily
```

---

## Performance Numbers to Internalize

| Operation | Latency | Notes |
|-----------|---------|-------|
| L1 cache hit | ~0.5 ns | 3 cycles @ 3 GHz |
| L2 cache hit | ~7 ns | ~20 cycles |
| L3 cache hit | ~15 ns | ~45 cycles |
| Main memory (DRAM) | ~100 ns | ~300 cycles |
| Mutex lock/unlock | ~25 ns | Fast path (no contention) |
| Atomic CAS (x86) | ~15 ns | Locked instruction |
| Thread context switch | ~1–10 μs | OS thread |
| Goroutine switch | ~50 ns | User-level, Go runtime |
| Cache line transfer (another core) | ~40 ns | Coherence protocol |
| System call | ~200 ns–1 μs | Mode switch |
| 1 GbE round trip (same DC) | ~500 μs | Limited by speed of light |
| SSD random read | ~100 μs | 4 KB |
| DRAM bandwidth | ~20–50 GB/s | Per channel |

---

## Quick Reference: Is It Concurrent? Is It Parallel?

```
Does the program structure independent tasks?
├── Yes → It's CONCURRENT
│   Does it run on multiple cores?
│   ├── Yes → CONCURRENT + PARALLEL
│   └── No  → CONCURRENT only (time-sliced)
└── No  → Is it doing one thing at a time?
    ├── Yes → NEITHER (sequential)
    └── No  → PARALLEL only (e.g., SIMD vector op)
```

---

## Glossary of Key Acronyms

| Acronym | Stands for | Meaning |
|---------|-----------|---------|
| CSP | Communicating Sequential Processes | Model where processes communicate via synchronous channels |
| SIMD | Single Instruction, Multiple Data | Same operation applied to multiple data elements in one instruction |
| SIMT | Single Instruction, Multiple Threads | NVIDIA GPU variant of SIMD |
| ILP | Instruction-Level Parallelism | CPU executes multiple instructions simultaneously (pipelining, superscalar) |
| DLP | Data-Level Parallelism | Same as SIMD — parallelism across data elements |
| TLP | Task-Level Parallelism | Parallelism across independent tasks |
| IPC | Inter-Process Communication | Mechanisms for data exchange between processes |
| CAS | Compare-And-Swap | Atomic primitive used in lock-free programming |
| ABA | (ABA Problem) | CAS can fail if a value changes from A→B→A — cannot detect modification |
| S/N | Strong/Weak Scaling | Strong = fixed problem, more cores. Weak = bigger problem, more cores. |
