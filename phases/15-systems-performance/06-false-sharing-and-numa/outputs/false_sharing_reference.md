# False Sharing & NUMA — Quick Reference

## Detection Commands

### perf (cache misses)

```bash
# Count L1 data cache load misses and overall cache misses
perf stat -e L1-dcache-load-misses,cache-misses ./your_program

# Record cache-miss events for annotation
perf record -e cache-misses ./your_program
perf report

# Hardware-level event (may need priv access)
perf stat -e cache-misses,instructions ./your_program
```

### Valgrind cachegrind (simulated)

```bash
valgrind --tool=cachegrind ./your_program
cg_annotate cachegrind.out.<pid>
```

### NUMA topology

```bash
numactl --hardware          # Show node topology
numastat -m                 # Per-node memory allocation stats
cat /sys/devices/system/node/node0/cpulist   # CPUs on Node 0
```

---

## Fix Patterns

### C++ — Padding with alignas

```cpp
// BAD: counters share cache lines
struct BadCounters {
    std::atomic<uint64_t> c0, c1, c2, c3;  // packed into 1-2 cache lines
};

// GOOD: each counter owns its cache line
struct alignas(64) PaddedAtomic {
    std::atomic<uint64_t> value;
    // implicit padding to 64 bytes
};

struct GoodCounters {
    PaddedAtomic counters[4];  // 4 separate cache lines
};
```

### Rust — Padding with #[repr(C, align(64))]

```rust
// BAD: counters share cache lines
struct BadCounters {
    c0: AtomicU64,
    c1: AtomicU64,
    c2: AtomicU64,
    c3: AtomicU64,
}

// GOOD: each counter owns its cache line
#[repr(C, align(64))]
struct CachePadded<T> {
    value: T,
    _pad: [u8; 64 - std::mem::size_of::<AtomicU64>()],
}

struct GoodCounters {
    counters: [CachePadded<AtomicU64>; 4],
}
```

### Rust — crossbeam CachePadded (production)

```rust
use crossbeam_utils::CachePadded;

struct GoodCounters {
    counters: [CachePadded<AtomicU64>; 4],
}
```

### C++ — Thread-local accumulation (avoid atomics entirely)

```cpp
uint64_t local_counter = 0;
for (int i = 0; i < N; ++i) {
    local_counter++;
}
global.store(local_counter, std::memory_order_relaxed);
```

---

## NUMA Commands

### Inspect topology

```bash
numactl --hardware        # Nodes, CPUs per node, memory per node
numastat -m               # Per-node memory allocation
lscpu | grep NUMA          # Quick NUMA summary
```

### Bind a process to a node

```bash
# Run on Node 0 with local memory only
numactl --cpunodebind=0 --membind=0 ./your_program

# Run on specific CPU cores
taskset -c 0,2,4,6 ./your_program

# Prefer local memory, allow remote if needed
numactl --cpunodebind=1 --preferred=1 ./your_program
```

### Migrate pages

```bash
# Move process's pages to Node 1
numactl --cpunodebind=1 --membind=1 --pid <pid>
# Or use move_pages for fine-grained control
```

---

## Thread Pinning (CPU Affinity)

### C++

```cpp
#include <pthread.h>
#include <sched.h>

void pin_thread(int core_id) {
    cpu_set_t cpuset;
    CPU_ZERO(&cpuset);
    CPU_SET(core_id, &cpuset);
    int rc = pthread_setaffinity_np(pthread_self(), sizeof(cpu_set_t), &cpuset);
    if (rc != 0) {
        // handle error
    }
}
```

### Rust

```rust
// Using core_affinity crate
use core_affinity;

let core_ids = core_affinity::get_core_ids().unwrap();
core_affinity::set_for_current(core_ids[0]);
```

### Command line

```bash
taskset -c 0,2,4,6 ./your_program
numactl --cpunodebind=0 --membind=0 ./your_program
```

---

## When to Worry (And When Not To)

| Scenario | Worry? | Why |
|----------|--------|-----|
| Multiple threads writing adjacent atomics | Yes | Each write invalidates the shared line |
| Per-thread counters in a struct | Yes | Classic false sharing pattern |
| Lock-free ring buffer head/tail on same line | Yes | Producer and consumer thrash the line |
| Read-heavy, rare writes | No | Reads don't invalidate |
| Single-threaded code | No | No other core contests the line |
| Data already 64+ bytes apart | No | Already on different lines |
| Occasional writes among mostly reads | Maybe | Cost is proportional to write frequency |

---

## Key Numbers

| Metric | Value |
|--------|-------|
| x86-64 cache line size | 64 bytes |
| L1 hit latency | ~1 ns (4 cycles) |
| L3 hit latency | ~10 ns (40 cycles) |
| Local DRAM latency | ~60–80 ns |
| Remote NUMA DRAM latency | ~120–150 ns |
| False-sharing penalty | ~40–100 ns per invalidation |