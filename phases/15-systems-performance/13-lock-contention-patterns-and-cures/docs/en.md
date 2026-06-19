# Lock Contention Patterns and Cures

> Lock contention is the silent killer of multicore performance — learn to recognize it, measure it, and cure it.

**Type:** Learn
**Languages:** Rust, C++
**Prerequisites:** Phase 15 lessons 01–12
**Time:** ~75 minutes

## Learning Objectives

- Understand mutex, spinlock, seqlock, and RW lock internals and when each is appropriate.
- Explain atomic operations (CAS, fetch_add, LL/SC) and the ABA problem.
- Contrast lock-free and lock-based approaches with their trade-offs.
- Diagnose contention scaling: why throughput degrades as thread count grows.
- Apply backoff strategies (exponential backoff, queue-based fairness).
- Understand RCU, per-CPU data, adaptive spinning, priority inversion, and futex.

## The Problem

You wrote a multithreaded program, threw a `mutex` around shared state, and it runs *slower* on 8 cores than on 1. This is not a bug in the scheduler — it is lock contention. When threads fight over a lock, they spend more time waiting than working. The more threads, the worse it gets. This lesson teaches you to recognize the pattern, measure it, and cure it with the right synchronization primitive or architecture.

## The Concept

### Mutex Internals

A mutex (mutual exclusion lock) guarantees that only one thread holds the lock at a time. On Linux, `pthread_mutex_t` is built on top of **futex** (fast userspace mutex):

1. **Fast path**: Try an atomic `compare_exchange` on the futex word. If it transitions from 0 → 1, the lock is acquired — no kernel involvement.
2. **Slow path**: If the CAS fails, call `futex(FUTEX_WAIT)` to sleep in the kernel. The kernel queues the thread.
3. **Wake path**: On unlock, if waiters exist, call `futex(FUTEX_WAKE)` to wake one waiter.

A mutex that can be acquired on the fast path costs ~25 ns. One that goes through the kernel costs ~1–5 µs. The ratio is 100:1.

```
Thread A (owner)          Thread B (contender)
  lock()                    lock()
  CAS 0→1 ✓                CAS 0→1 ✗
  [critical section]        futex_wait() → kernel sleep
  unlock()
  CAS 1→0
  futex_wake()           →  kernel wakes Thread B
                           CAS 0→1 ✓
                           [critical section]
```

### Spinlock Internals

A spinlock is the simplest lock: a tight loop doing `while (!try_lock()) {}`. The atomic operation is typically `test_and_set` or `compare_exchange` on a boolean flag.

- **Pro**: No kernel call. If the hold time is < the cost of a context switch (~1–5 µs), spinning avoids the overhead.
- **Con**: Burns CPU cycles while waiting. Under heavy contention, spinning wastes cores that could do useful work.
- **Cache-line bouncing**: Every failed CAS invalidates the cache line. With N spinners, the line bounces N × acquire_attempts times.

```
Spinlock acquire (pseudocode):
  while (atomic_exchange(&flag, 1) == 1) {
      // spin — optionally with pause instruction
  }
```

On x86, the `PAUSE` instruction (used in `_mm_pause()`) hints to the CPU that this is a spin-wait loop, reducing power consumption and improving hyper-threading throughput.

### Seqlock Internals

A **sequence lock** (seqlock) allows lock-free reads and exclusive writes:

- A writer increments a sequence counter (even → odd) before and after writing.
- A reader reads the counter before and after reading data. If the counter changed or is odd, the reader retries.
- Writers still use a spinlock or mutex among themselves.

```
Write path:
  spin_lock(&write_lock)
  seq++                // odd → readers will retry
  [write data]
  seq++                // even → data is consistent again
  spin_unlock(&write_lock)

Read path:
  do {
      s = read(seq)
      [read data]
  } while (s != read(seq) || s & 1)
```

Seqlocks are used in the Linux kernel for `clocksource` and `jiffies` — data that is read constantly but written rarely.

### RW Lock Internals

A **read-write lock** allows concurrent readers OR a single exclusive writer:

- Multiple readers can hold the lock simultaneously.
- A writer must wait until all readers have released.
- Writer starvation is a real risk if readers continuously acquire the lock.

Linux `pthread_rwlock_t` uses an internal atomic counter: positive values = reader count, −1 = writer-held. Writers set a "writer waiting" flag to block new readers (write-preferring variant).

**Problem**: RW locks suffer from **cache-line contention** on the counter itself. Every reader does an atomic increment/decrement, bouncing the cache line.

### Atomic Operations

#### Compare-And-Swap (CAS)

```
bool cas(T *addr, T expected, T desired) {
    if (*addr == expected) {
        *addr = desired;
        return true;
    }
    return false;
}
```

CAS is the universal primitive for lock-free programming. On x86, it maps to `CMPXCHG`. On ARM, it is built from **LL/SC** (Load-Link / Store-Conditional):

- LDXR loads a value and tags the address.
- STXR stores only if no other write to that address occurred since LDXR.
- LL/SC avoids the ABA problem at the hardware level but can experience **spurious failures** (store fails even when no conflict occurred).

#### Fetch-and-Add

```
T fetch_add(T *addr, T delta) {
    return atomic_add(addr, delta) - delta;  // returns old value
}
```

On x86, `XADD` does this in a single instruction. More efficient than CAS loops because it cannot fail (no retry needed).

#### ABA Problem

A classic CAS pitfall:

```
Thread 1: reads *addr = A
          [preempted]
Thread 2: *addr = B, then *addr = A   // value changed and changed back
Thread 1: CAS(addr, A, C) succeeds — but state has changed!
```

**Cures**:
- Use **double-width CAS** (128-bit on x86-64) to include a version counter alongside the value.
- Hazard pointers — readers register that they are accessing a pointer, preventing reclamation.
- Epoch-based reclamation — defer freeing until all threads have passed a safe epoch.

### Lock-Free vs Lock-Based

| Aspect | Lock-Based | Lock-Free |
|--------|-----------|-----------|
| Progress guarantee | At least one thread makes progress | All threads make progress (wait-free) or at least one (lock-free) |
| Complexity | Lower | Significantly higher |
- Composability | Easy: acquire lock, do multiple operations | Hard: each operation must be independently safe |
| Fault tolerance | Deadlock if a lock-holder crashes | No single point of failure |
| Performance | Under high contention, threads sleep (good) | Under high contention, CAS retries burn CPU (bad) |

Lock-free is not always faster. Under moderate-to-high contention, a mutex that puts threads to sleep outperforms a CAS loop that keeps retrying.

### Contention Scaling

Amdahl's law for locks: if a critical section takes fraction *f* of total work, maximum speedup = 1/f.

With N threads contending on one lock:

```
Throughput ≈ N / (1 + α(N-1))
```

where α is the fraction of time spent in the critical section. As N grows, throughput approaches 1/α — a hard ceiling.

**Example**: If 5% of work is in a critical section (α = 0.05), max speedup ≈ 20× regardless of how many cores you add.

### Backoff Strategies

#### Exponential Backoff

After a failed CAS, wait `2^attempt × base_delay` before retrying:

```
for attempt in 0.. {
    if cas(&lock, 0, 1) { return; }
    sleep(2^attempt * base_delay)
}
```

Reduces cache-line bouncing because contending threads stagger their retries.

#### Queue-Based Fairness (Ticket Lock)

```
struct TicketLock {
    atomic_uint next_ticket = 0;
    atomic_uint now_serving = 0;
};

acquire:
    my_ticket = fetch_add(&next_ticket, 1)
    while (now_serving != my_ticket) { pause(); }

release:
    now_serving++
```

Guarantees FIFO ordering. No starvation. But threads still spin, wasting cycles on the `now_serving` cache line.

### Read-Copy-Update (RCU)

RCU is the gold standard for read-mostly data:

1. Readers access data with **no lock, no atomic** — just a pointer dereference.
2. Writers create a copy, modify the copy, then atomically swap the pointer.
3. Old data is freed only after a **grace period** — all pre-existing readers have finished.

```
Reader:
  rcu_read_lock()      // disables preemption or increments per-CPU counter
  v = *shared_ptr      // plain load — no atomic on the fast path
  rcu_read_unlock()

Writer:
  new = copy(old)
  modify(new)
  old = atomic_swap(&shared_ptr, new)
  synchronize_rcu()    // wait for all pre-existing readers
  free(old)
```

In the Linux kernel, RCU is used for module unloading, `task_struct` lookups, and routing tables. Read-side overhead is essentially zero (a compiler barrier + preempt disable in non-preemptible kernels).

### Per-CPU Data

Eliminate contention entirely: give each CPU its own copy of the data.

```c
DEFINE_PER_CPU(int, counter);

void inc_counter(void) {
    this_cpu_write(counter, this_cpu_read(counter) + 1);
}

int total_counter(void) {
    int sum = 0;
    for_each_possible_cpu(cpu)
        sum += per_cpu(counter, cpu);
    return sum;
}
```

No lock, no atomic on the fast path. Only the aggregate operation crosses CPUs. Used extensively in the Linux kernel for statistics, memory allocators (`percpu` allocator), and interrupt counters.

### Mutex Variants: Adaptive Spinning

Linux `pthread_mutex` with `PTHREAD_MUTEX_ADAPTIVE_NP` (glibc extension):

- First, spin for a limited number of iterations (typically proportional to `NR_CPUS`).
- If the lock is not acquired, fall back to `futex_wait`.
- Best of both worlds: fast acquisition for short critical sections, no CPU waste for long ones.

### Priority Inversion

```
Low-priority thread L holds lock → Medium-priority thread M preempts L → High-priority thread H waits for lock
```

H is blocked indefinitely by M, even though H should run. This caused the **Mars Pathfinder** restart in 1997.

**Cure: Priority Inheritance Protocol (PI)**

When H waits for a lock held by L, L's priority is temporarily boosted to H's priority. L runs, releases the lock, and returns to low priority. Linux `PTHREAD_PRIO_INHERIT` implements this.

The trade-off: PI mutexes have higher overhead (kernel tracks inheritance chains) and can cause **chain blocking** if the inheritance chain is long.

### Futex (Fast Userspace Mutex)

A futex is a Linux system call (`futex(2)`) that combines:

- **A userspace atomic word** for the fast path (no syscall when uncontended).
- **A kernel wait queue** for the slow path (thread sleeps, wakes on unlock).

```
futex(int *uaddr, int op, int val, ...)
  FUTEX_WAIT:  if *uaddr == val, sleep; else return immediately.
  FUTEX_WAKE:  wake up to val waiters on uaddr.
  FUTEX_CMP_REQUEUE:  wake some, move rest to another queue (for PI).
```

Futex enables pthread_mutex, pthread_cond, barriers, and more — all in userspace when uncontended, kernel-assisted when contended. The `op` argument has grown to include `FUTEX_WAIT_BITSET`, `FUTEX_WAKE_BITSET`, `FUTEX_REQUEUE`, and priority-inheritance variants.

## Build It

### Step 1: Minimal — Mutex vs Spinlock vs Atomic Counter

We benchmark three approaches to incrementing a shared counter under contention:

1. **Mutex**: `pthread_mutex_lock` / `unlock`
2. **Spinlock**: Atomic `compare_exchange` in a loop
3. **Atomic**: `fetch_add` (lock-free single instruction on x86)

### Step 2: Realistic — Contention Scaling

We run each approach with 1, 2, 4, 8 threads and measure how throughput scales (or doesn't). We also add:

- **Exponential backoff spinlock**: failed CAS → `pause` × 2^attempt
- **Ticket lock**: FIFO fairness guarantee

## Use It

### Production Systems

- **Linux kernel**: `mutex`, `spinlock`, `seqlock_t`, `rw_semaphore`, `RCU`, `percpu` — all in `include/linux/`. The kernel has ~15 different locking primitives tuned for different workloads.
- **glibc `pthread_mutex`**: Adaptive spinning (`PTHREAD_MUTEX_ADAPTIVE_NP`), priority inheritance (`PTHREAD_PRIO_INHERIT`), error-checking, and robust variants.
- **Rust `std::sync::Mutex`**: Wraps OS futex on Linux (since 1.62), uses a fair queue internally.
- **Facebook `folly/MicroSpinLock.h`**: 1-byte spinlock with exponential backoff. Used when critical section is < 100 ns.
- **Google `abseil/base/internal/spinlock.h`**: Adaptive spin with `Sched_yield` fallback before kernel sleep.

## Read the Source

- `linux/kernel/locking/mutex.c` — adaptive spinning mutex with optimistic fast path and hand-off protocol.
- `linux/kernel/locking/spinlock.c` — architecture-specific spinlock with `ARCH_SPINLOCK_SIZE`.
- `linux/include/linux/rcupdate.h` — RCU read-side API (single instruction on x86 TSO).
- `glibc/nptl/pthread_mutex_lock.c` — the `LLL_MUTEX_TRYLOCK` fast path + `futex_wait` slow path.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`lock_contention_reference.md`** — a quick-reference card for choosing the right synchronization primitive.

## Exercises

1. **Easy** — Modify the C++ benchmark to add a "no-contention" baseline (each thread increments a thread-local counter, then sums at the end). Compare against the contended versions.
2. **Medium** — Implement a ticket lock in Rust and add it to the benchmark suite. Measure fairness by recording how many times each thread acquires the lock.
3. **Hard** — Implement a simple seqlock in C++ for a multi-field struct. Demonstrate that readers never see torn writes. Benchmark against an RW lock for read-heavy (99:1) workloads.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Mutex | "a lock" | A kernel-backed mutual exclusion primitive that sleeps the calling thread on contention |
| Spinlock | "spinning lock" | A lock that burns CPU in a CAS loop instead of sleeping — appropriate only for very short critical sections |
| Seqlock | "sequence lock" | A lock that allows lock-free reads by checking a sequence counter for consistency |
| CAS | "compare-and-swap" | An atomic operation that writes a new value only if the current value matches an expected value |
| ABA problem | "the ABA thing" | A CAS may succeed even though the value changed and changed back, leading to incorrect behavior |
| Futex | "fast mutex" | A Linux syscall combining a userspace atomic word with a kernel wait queue for uncontended fast paths |
| RCU | "read-copy-update" | A synchronization mechanism offering zero-overhead reads by deferring reclamation to a grace period |
| Priority inversion | "priority thing" | A high-priority thread blocked by a low-priority thread holding a lock, while medium-priority threads run |
| Per-CPU data | "percpu" | Data replicated per CPU core, eliminating all synchronization on the fast path |
| Backoff | "exponential wait" | Strategy of increasing delay between retries to reduce contention on a shared cache line |

## Further Reading

- *Is Parallel Programming Hard, and, If So, What Can You Do About It?* — Paul E. McKenney (free, covers RCU in depth)
- *The Art of Multiprocessor Programming* — Herlihy & Shavit (CAS, lock-free, ABA)
- *Understanding the Linux Kernel* — Bovet & Cesati (futex, kernel locking)
- Ulrich Drepper, "Futexes Are Tricky" (paper, 2011)
- Linux kernel source: `kernel/locking/` directory