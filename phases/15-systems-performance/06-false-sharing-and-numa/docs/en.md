# False Sharing and NUMA

> When two threads pound different variables on the same cache line, the line ping-pongs between cores — and your program crawls. This lesson teaches you to spot it, fix it, and think about the machine beyond a single socket.

**Type:** Learn
**Languages:** C++, Rust
**Prerequisites:** Phase 15 lessons 01–05
**Time:** ~75 minutes

## Learning Objectives

- Explain why false sharing degrades multithreaded performance, even though no data is actually shared.
- Detect false sharing using `perf` cache-miss counters.
- Fix false sharing by padding variables to cache line boundaries (`alignas(64)` / `#[repr(C, align(64))]`).
- Distinguish true sharing (intentional) from false sharing (accidental co-location on a cache line).
- Describe NUMA topology: multiple sockets, local vs. remote memory, and the latency penalty of cross-node access.
- Use Linux NUMA tools (`numactl`, `numastat`, `move_pages`) and thread pinning (`sched_setaffinity`) to place data close to the threads that use it.

## The Problem

You wrote a multithreaded counter. Four threads each increment their own `atomic<uint64_t>` a billion times. The variables sit next to each other in a `struct Counters { atomic<uint64_t> c0, c1, c2, c3; }`. You expect linear scaling — four threads should finish in roughly a quarter of the single-thread time.

Instead, the four-thread version is *slower* than one thread. Not a little slower. Five to ten times slower.

The problem is not the atomics. The problem is not the algorithm. The problem is that your four counters live on the same 64-byte cache line. Every time Thread 0 increments `c0`, it invalidates the cache line on cores 1–3 — even though they only care about `c1`/`c2`/`c3`. The line bounces across the interconnect like a hot potato. This is **false sharing**: no logical sharing of data, but physical sharing of the cache line that carries them.

This lesson builds the mental model, the diagnostics, the fix, and then scales out to NUMA — because once you fix false sharing within a socket, the next bottleneck is crossing socket boundaries.

## The Concept

### Cache Lines: The Unit of Coherence

Modern CPUs do not manage individual bytes in their caches. They manage **cache lines** — typically 64 bytes on x86 and ARM. When core A writes to byte 0 of a line, it **invalidates** that entire 64-byte line on every other core that holds it. If core B then reads byte 4 of that same line — even though core A never touched byte 4 — core B suffers a cache miss and must fetch the line from core A (or from shared L3 / main memory).

This is the coherence protocol doing its job. The protocol cannot know that bytes 0–7 and 8–15 belong to different logical variables. It only sees: "someone wrote to this line, so other copies are stale."

### True Sharing vs. False Sharing

| Aspect | True Sharing | False Sharing |
|--------|-------------|---------------|
| Threads access | The **same** variable | **Different** variables |
| Intent | Deliberate — they share data | Accidental — same cache line, different data |
| Performance cost | Unavoidable serialization (or use lock-free algorithms) | Unnecessary serialization; entirely avoidable by layout |
| Fix | Rethink the algorithm | Pad or rearrange data layout |

True sharing is when two threads genuinely read and write the same memory location — say, a shared atomic counter. The cache-line bouncing is the *point*: only one thread should own the line at a time for correctness.

False sharing is when two threads touch *independent* variables that happen to be close enough in memory to share a cache line. The bouncing is *not* the point — it is a performance bug.

### Worked Example: The Counter Array

Consider this layout:

```
Offset:  0x00  0x08  0x10  0x18  0x20  0x28  0x30  0x38
Thread:  T0    T1    T2    T3    (unused bytes to 0x3F)
Variable: c0    c1    c2    c3
         <---------- same 64-byte cache line ---------->
```

Each `atomic<uint64_t>` is 8 bytes. Four counters = 32 bytes. They all fit in one 64-byte cache line. On a 4-core machine:

1. T0 reads the line into its L1. T0 writes c0 → invalidates line on T1, T2, T3.
2. T1 wants to write c1 → cache miss. Fetches the line. T0's copy invalidated.
3. T2 wants to write c2 → cache miss. Fetches the line. T1's copy invalidated.
4. T3 wants to write c3 → cache miss. Fetches the line. T2's copy invalidated.
5. Repeat a billion times.

Each increment triggers a cache-line ownership transfer. On Intel, that is ~40–100 ns per transfer via the L3 / QPI interconnect. A billion increments × ~70 ns = ~70 seconds. A single thread doing the same work at ~1 ns per increment finishes in ~1 second.

**The fix:** Pad each counter to its own cache line:

```
Offset:  0x00       0x40       0x80       0xC0
Thread:  T0         T1         T2         T3
Variable: c0+pad     c1+pad     c2+pad     c3+pad
         <-line 0-> <-line 1-> <-line 2-> <-line 3->
```

Now each thread's writes only invalidate its own line. No cross-thread invalidation. Four threads finish in ~0.25 seconds — true linear scaling.

### How to Detect False Sharing

**perf** is the primary tool:

```bash
# Count L1-dcache-load-misses and cache-references
perf stat -e L1-dcache-load-misses,cache-misses ./your_program

# A spike in L1-dcache-load-misses that grows with thread count is the signature.
# Record and annotate:
perf record -e cache-misses ./your_program
perf report
```

Other clues:

- **Scalability inversion**: Adding threads makes the program *slower*.
- **High `cache-misses` specifically on writers**: perf shows the hot lines are all in your counter struct.
- **cachegrind** (Valgrind) simulates cache behavior and can pinpoint which struct members thrash.

### How to Fix False Sharing

| Technique | C++ | Rust | Notes |
|-----------|-----|------|-------|
| Padding | `alignas(64) atomic<uint64_t> c0;` | `#[repr(C, align(64))] struct PaddedAtomic(AtomicU64);` | Simplest, most common. |
| Compiler padding attribute | `__attribute__((aligned(64)))` | N/A (use `#[repr(align(64))]`) | GCC/Clang extension. |
| Array of structs | One padded struct per counter instead of array of counters | Same idea | Natural with structs. |
| Thread-local accumulation | Each thread accumulates in a local variable, writes back once at the end | Same | Avoids atomics entirely. |
| `__builtin_prefetch` | Prefetch the line before you need it | Not a fix for false sharing — only hides latency | Doesn't eliminate invalidation. |

The lesson code demonstrates the `alignas` / `#[repr(C, align(64))]` approach.

### When False Sharing Matters — and When It Doesn't

**Matters:**

- High-frequency writes by multiple threads to adjacent data (counters, per-thread histograms, hash-table bins).
- Lock-free data structures where threads write to adjacent slots (ring buffers, work-stealing deques).
- Any `struct` with per-thread fields packed together.

**Doesn't matter:**

- Read-heavy workloads. reads don't invalidate.
- Single-threaded code. No other core disputes the line.
- Data already separated by ≥64 bytes. No sharing, no problem.
- Low-frequency writes. The invalidation cost is amortized over useful work.

### NUMA Architecture

Modern servers have multiple CPU **sockets** (also called **nodes**). Each socket has its own memory controller and a pool of **local** DRAM. A thread on Socket 0 can access memory on Socket 1, but that **remote** access traverses an interconnect (Intel UPI, AMD Infinity Fabric) and costs 1.5–2× the latency of local access.

```
┌──────── Socket 0 (Node 0) ────────┐    ┌──────── Socket 1 (Node 1) ────────┐
│  Core 0  Core 1  Core 2  Core 3    │    │  Core 4  Core 5  Core 6  Core 7    │
│  ┌─────────────────────────────┐  │    │  ┌─────────────────────────────┐  │
│  │       L3 Cache (shared)     │  │    │  │       L3 Cache (shared)     │  │
│  └─────────────────────────────┘  │    │  └─────────────────────────────┘  │
│  ┌─────────────────────────────┐  │    │  ┌─────────────────────────────┐  │
│  │    Local DRAM (Node 0)      │◄─┼────┼─►│    Local DRAM (Node 1)      │  │
│  └─────────────────────────────┘  │    │  └─────────────────────────────┘  │
└───────────────────────────────────┘    └───────────────────────────────────┘
          │                                       │
          └──────── UPI / Infinity Fabric ────────┘
              Remote access: ~120-150ns
              Local access:   ~60-80ns
```

**Key NUMA facts:**

- Memory allocated on Node 0 is *local* to threads on Node 0 and *remote* to threads on Node 1.
- The Linux kernel's default page allocation is "first touch": the node that first writes a page owns it. This means if Thread 0 (on Node 0) initializes a large array, that array lives on Node 0 — even if Thread 5 (on Node 1) does all the later reads.
- The kernel's automatic NUMA balancing (`numad`) tries to migrate pages toward the threads that use them, but it is conservative and may not kick in fast enough for latency-sensitive work.

### NUMA-Aware Programming

The principle: **minimize remote memory access**. Three tactics:

1. **Allocate on the right node.** Use `numa_alloc_onnode()` (C) or `numactl --membind=<node>` to place memory where the consuming threads live.
2. **Schedule threads near their data.** Pin threads to cores on the same node as their data using `sched_setaffinity()` or `numactl --cpunodebind=<node>`.
3. **Structure data per-node.** Instead of one big hash table shared across nodes, give each node its own partition. Threads only go remote when they need data from another partition — and you can make that the cold path.

### Linux NUMA Tools

| Tool | Purpose | Example |
|------|---------|---------|
| `numactl --hardware` | Show node topology | `numactl --hardware` |
| `numastat` | Per-node memory allocation stats | `numastat -m` |
| `numactl --membind=1 --cpunodebind=1 ./prog` | Bind program to Node 1 | Run with local memory |
| `move_pages` | Migrate pages between nodes | `move_pages -p <pid> <nr_pages> <pages> <nodes>` |
| `perf stat -e cache-misses,numa_miss` | Detect remote accesses | `perf stat -a -e cache-misses ./prog` |

### Thread Pinning (CPU Affinity)

Thread pinning prevents the OS from migrating a thread between cores (or sockets), which hurts cache warmth and NUMA locality.

C++:
```cpp
#include <pthread.h>
void pin_thread(int core_id) {
    cpu_set_t cpuset;
    CPU_ZERO(&cpuset);
    CPU_SET(core_id, &cpuset);
    pthread_setaffinity_np(pthread_self(), sizeof(cpu_set_t), &cpuset);
}
```

Rust:
```rust
use core_affinity;
let core_ids = core_affinity::get_core_ids().unwrap();
core_affinity::set_for_current(core_ids[0]);
```

Command line:
```bash
taskset -c 0,2,4,6 ./prog   # Pin to even cores
numactl --cpunodebind=0 --membind=0 ./prog  # Pin to Node 0
```

## Build It

### Step 1: Minimal Version — Demo the Bug

The minimal version demonstrates false sharing with `std::atomic<uint64_t>` in C++ and `AtomicU64` in Rust. Four threads each increment their own counter a large number of times. Because the counters are packed on the same cache line, each write invalidates the line on the other cores.

### Step 2: Realistic Version — Fix with Padding

The realistic version pads each counter to a 64-byte cache line using `alignas(64)` (C++) or `#[repr(C, align(64))]` (Rust). The same workload runs, but now each thread's counter lives on its own line — no invalidation crossfire. Print timing for both versions so the difference is visible.

## Use It

**Production patterns:**

- **Linux kernel `percpu` variables**: The kernel allocates each per-CPU variable on its own cache line. See `DEFINE_PER_CPU` in `include/linux/percpu-defs.h`. This is the kernel's built-in false-sharing defense.
- **tcmalloc `TCMalloc_ThreadCache`**: Each thread gets its own heap cache, padded to avoid false sharing with other threads' caches. See `src/tcmalloc/thread_cache.h`.
- **Java `@Contended` annotation**: The JVM pads annotated fields to avoid false sharing. Used extensively in `java.util.concurrent` classes like `LongAdder` (which uses striped cells, each padded to a cache line).
- **Rust `crossbeam-utils::CachePadded`**: The `crossbeam` crate provides `CachePadded<T>` that wraps any `T` in a padded struct aligned to 64 bytes. Production Rust code uses this instead of manual `repr(align)`.

**Compare your hand-built version against `CachePadded`:**

```rust
use crossbeam_utils::CachePadded;
let counters: Vec<CachePadded<AtomicU64>> = ...;
```

`CachePadded` does exactly what your `#[repr(C, align(64))]` struct does, but it also handles `Drop` semantics and works for any inner type — not just atomics.

## Read the Source

- **Linux `percpu`**: `include/linux/percpu-defs.h` — look at `DEFINE_PER_CPU` and how it aligns each per-CPU variable to `CACHELINE_SIZE`.
- **Java `LongAdder`**: `java.util.concurrent.atomic.LongAdder` —Striped64 uses `@Contended` to pad each cell to avoid false sharing. Reading the JDK source is worth it for the pattern alone.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`false_sharing_reference.md`** — A quick-reference card with: detection commands (`perf`, `numastat`), fix patterns in C++ and Rust, NUMA commands, and thread-pinning snippets. Keep it open when profiling.

## Exercises

1. **Easy** — Run the provided code. Observe the timing difference between the packed and padded versions. Confirm that `perf stat -e L1-dcache-load-misses` shows more misses in the packed version.
2. **Medium** — Modify the code to use thread-local accumulation instead of atomic increments. Each thread accumulates in a plain `uint64_t` local variable and writes back only at the end. Compare timing against both the padded and unpadded atomics.
3. **Hard** — Build a minimal work-stealing deque. Use `CachePadded` (or `alignas(64)`) to pad the deque's head and tail indices. Measure throughput with and without padding under contention from 4+ stealing threads. Write a short analysis of when padding is worth the memory overhead and when it isn't.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| False sharing | "Cache line bouncing" | Unintended cache-line invalidation caused by independent variables co-located on the same 64-byte line |
| True sharing | "Shared data" | Two threads intentionally accessing the same variable; cache-line bouncing is expected |
| Cache line | "Cache block" | The 64-byte unit of transfer and coherence in the CPU cache hierarchy |
| NUMA node | "Socket" or "NUMA domain" | A CPU socket with its own local memory; remote access costs ~1.5–2× local |
| Thread pinning | "CPU affinity" or "pinning" | Binding a thread to a specific core or set of cores to prevent migration |
| `alignas(64)` | "Cache line padding" | C++ specifier that aligns a variable to a 64-byte boundary, ensuring it owns its own cache line |
| `numactl` | "NUMA control" | Linux tool to bind processes to specific NUMA nodes for memory and CPU |
| First-touch policy | "First writer owns the page" | Linux default: memory is allocated on the NUMA node of the first thread that writes to it |

## Further Reading

- Drepper, Ulrich. *What Every Programmer Should Know About Memory* (2007) — Sections 3–4 on cache coherency and NUMA.
- Fog, Agner. *Optimizing software in C++* — Section on false sharing and data alignment.
- Linux kernel source: `include/linux/percpu-defs.h` — The canonical `DEFINE_PER_CPU` pattern.
- `man numactl`, `man move_pages` — Linux NUMA tooling documentation.
- The `crossbeam` crate documentation: https://docs.rs/crossbeam-utils — `CachePadded<T>` API.
- Jeffrey, Mark. *A Primer on Synchronization and False Sharing* — Oracle technical note on `@Contended`.