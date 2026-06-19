# Lock Contention Patterns and Cures — Quick Reference

## Choosing the Right Synchronization Primitive

| Primitive | Read Overhead | Write Overhead | Fairness | Best When |
|-----------|--------------|----------------|----------|-----------|
| Mutex | 1 atomic CAS + potential futex_wait | 1 atomic + potential futex_wake | Kernel FIFO | General purpose, unknown hold time |
| SpinLock | 1 atomic CAS loop | 1 atomic store | None | Hold time < context switch cost (~1–5 µs) |
| Backoff SpinLock | CAS with exponential pause | 1 atomic store | None | Short hold time, moderate contention |
| Ticket Lock | fetch_add + spin | fetch_add on serving | FIFO | Need fairness guarantee, short hold time |
| RW Lock | Atomic increment | Exclusive lock | Writer starvation risk | Read-heavy, can tolerate reader cache bounce |
| Seqlock | 2 reads of seq (no atomic) | Seq increment + write lock | Writer priority (exclusive) | Read-mostly, small data, no reader side effects |
| RCU | Zero (plain pointer read) | Copy + atomic swap + grace period | Writer defers | Read-heavy, frequent reads, rare writes |
| Per-CPU | Zero (local access) | Zero (local access) | N/A | Per-thread statistics, counters, allocators |
| Atomic (fetch_add) | N/A | 1 instruction (XADD) | Hardware fairness | Simple counters, accumulators |

## Contention Scaling Rules of Thumb

- **Amdahl's law for locks**: If fraction `f` of work is in a critical section, max speedup = `1/f`.
- **Spinlock under contention**: Throughput degrades as ~1/N (N = thread count). Each failed CAS bounces the cache line.
- **Mutex under contention**: Throughput plateaus at ~1/α (α = critical section fraction). Sleeping threads free cores.
- **fetch_add**: Near-linear scaling for simple counters. Hardware handles cache coherence.

## Atomic Operations Cheat Sheet

| Operation | x86 Instruction | ARM Equivalent | Can Fail? |
|-----------|----------------|----------------|-----------|
| CAS | `CMPXCHG` | LDXR + STXR (LL/SC) | Yes (CAS can fail spuriously on ARM) |
| fetch_add | `XADD` | LDADD | No (always succeeds) |
| swap | `XCHG` | SWP | No (always succeeds) |
| load (acquire) | `MOV` | LDAR | No |
| store (release) | `MOV` | STLR | No |

## ABA Problem — Quick Diagnosis

If you use CAS on a pointer or value that can be freed and reallocated:
1. Thread reads value A.
2. Another thread changes A → B → A.
3. Original CAS succeeds but state is stale.

**Fix**: Use versioned CAS (double-width on x86-64), hazard pointers, or epoch-based reclamation.

## Priority Inversion — Quick Fix

```
Low-priority holds lock → High-priority waits → Medium-priority runs instead
```

**Fix**: Use `PTHREAD_PRIO_INHERIT` on the mutex, or use `mutexattr_setprotocol(PRIO_INHERIT)`.

## Futex — When It Helps

- **0 contention**: No syscall. Pure userspace CAS. ~25 ns.
- **Heavy contention**: Sleep/wake via kernel. ~1–5 µs per transition.
- **Hybrid**: glibc's `PTHREAD_MUTEX_ADAPTIVE_NP` spins a few iterations, then calls `futex_wait`.

## Code Patterns

### Exponential Backoff Spinlock (C++)
```cpp
void lock(atomic<bool>& flag) {
    int delay = 1;
    while (flag.exchange(true, memory_order_acquire)) {
        for (int i = 0; i < delay; ++i) _mm_pause();
        delay = min(delay * 2, 1024);
    }
}
```

### Seqlock Read (C++)
```cpp
T read(seqlock_t& sl, T* data) {
    unsigned seq;
    T copy;
    do {
        seq = sl.seq.load(memory_order_acquire);
        copy = *data; // plain read
    } while (seq != sl.seq.load(memory_order_acquire) || (seq & 1));
    return copy;
}
```

### Per-CPU Counter (Linux Kernel)
```c
DEFINE_PER_CPU(long, my_counter);

void inc_my_counter(void) {
    this_cpu_inc(my_counter); // no lock, no atomic
}

long total_my_counter(void) {
    long sum = 0;
    for_each_possible_cpu(cpu)
        sum += per_cpu(my_counter, cpu);
    return sum;
}
```