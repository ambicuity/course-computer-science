# Lesson 14: Synchronization in the Kernel — Spinlocks, RCU

## The Problem

The kernel is a concurrent environment: multiple CPUs execute kernel code simultaneously, interrupts can fire at any moment, and preemptive scheduling can switch threads mid-operation. Without synchronization, shared data structures become corrupted. But synchronization primitives themselves have costs — choosing the wrong one can destroy performance or introduce subtle deadlocks.

## Sources of Kernel Concurrency

1. **SMP (Symmetric Multiprocessing):** Multiple CPUs share memory. Two CPUs updating the same variable concurrently causes data races.
2. **Interrupts:** An interrupt handler can preempt the currently running kernel code on the same CPU. If the interrupted code held a lock, and the handler tries to acquire the same lock → deadlock.
3. **Preemption:** The scheduler can switch threads at any point. If thread A reads a pointer, gets preempted, and thread B frees the object → use-after-free.

## Spinlock

A spinlock is the simplest lock: the waiting thread **busy-waits** (spins) in a tight loop until the lock is released. Implementation uses an atomic test-and-set operation.

```c
typedef struct { int locked; } spinlock_t;

void spin_lock(spinlock_t *lock) {
    while (__atomic_test_and_set(&lock->locked, __ATOMIC_ACQUIRE))
        ; // spin
}

void spin_unlock(spinlock_t *lock) {
    __atomic_clear(&lock->locked, __ATOMIC_RELEASE);
}
```

**When to use:** Very short critical sections (nanoseconds to low microseconds). The lock hold time must be less than the cost of a context switch. Used extensively in the kernel for protecting per-CPU data, interrupt handlers, and scheduling queues.

**Drawbacks:**
- Wastes CPU cycles while spinning.
- **Priority inversion:** a high-priority thread spins waiting for a low-priority thread that cannot run.
- **Interrupt deadlock:** if a CPU holds a spinlock and an interrupt on the same CPU tries to acquire it → livelock. Solution: `spin_lock_irqsave()` disables interrupts while holding the lock.

## Mutex (Sleeping Lock)

A mutex (mutual exclusion lock) puts the waiting thread to **sleep** instead of spinning. When the lock is released, one waiter is woken up.

```c
typedef struct {
    int locked;
    // In a real kernel: wait queue of sleeping threads
} mutex_t;

void mutex_lock(mutex_t *m) {
    // Fast path: try to acquire without sleeping
    if (__atomic_exchange_n(&m->locked, 1, __ATOMIC_ACQUIRE) == 0)
        return;
    // Slow path: add to wait queue, sleep (futex / context switch)
    while (__atomic_exchange_n(&m->locked, 1, __ATOMIC_ACQUIRE) != 0) {
        // put thread to sleep via futex_wait / schedule()
        sleep_on_wait_queue(m);
    }
}

void mutex_unlock(mutex_t *m) {
    __atomic_store_n(&m->locked, 0, __ATOMIC_RELEASE);
    // Wake one waiter
    wake_one(m);
}
```

**When to use:** Longer critical sections (microseconds to milliseconds) or when the lock holder might sleep. The sleeping thread yields the CPU to other work.

**Trade-off vs spinlock:** Lower CPU waste, but higher latency (context switch cost on lock/unlock).

## RCU (Read-Copy-Update)

RCU is a synchronization mechanism optimized for read-heavy workloads. **Readers never block, never acquire locks, and never write shared memory.** Writers create a new version of the data and atomically swap the pointer.

### Core Idea

1. Writers allocate a new copy of the data structure, modify it, then atomically publish the new pointer.
2. Readers dereference the pointer and read the data. They see a consistent snapshot.
3. After all pre-existing readers finish (a **grace period**), the old version is freed.

```c
// Reader side (no locks!)
rcu_read_lock();        // begin critical section (disables preemption or similar)
ptr = rcu_dereference(global_ptr);  // safe pointer read
// use ptr...
rcu_read_unlock();      // end critical section

// Writer side
new = kmalloc(...);
*new = *old;
new->field = new_value;
rcu_assign_pointer(global_ptr, new);  // atomic pointer publish
synchronize_rcu();     // wait for all readers to finish
kfree(old);             // safe to free old version
```

### Grace Period

The kernel tracks which CPUs are in an RCU read-side critical section. After a writer publishes a new pointer, it waits (or schedules deferred work) until every CPU has passed through at least one quiescent state (context switch, idle loop, or user mode). At that point, no reader can still hold a reference to the old data.

### Use Cases

Linux uses RCU heavily for:
- **Routing tables** (netfilter, networking): readers look up routes at line speed.
- **Module reference counting and `dentry` / path resolution.**
- **PID and fd tables** in process management.

## Seqlock (Sequence Lock)

A seqlock favors writers. A sequence counter is incremented before and after a write. Readers check the counter: if it is odd (write in progress) or changed (write happened during read), the reader retries.

```c
typedef struct { unsigned seq; spinlock_t writer_lock; } seqlock_t;

// Writer
void write_seqlock(seqlock_t *sl) {
    spin_lock(&sl->writer_lock);
    sl->seq++;  // odd = write in progress
    __atomic_thread_fence(__ATOMIC_RELEASE);
}
void write_sequnlock(seqlock_t *sl) {
    __atomic_thread_fence(__ATOMIC_RELEASE);
    sl->seq++;  // even = write complete
    spin_unlock(&sl->writer_lock);
}

// Reader (no locks — may retry)
unsigned read_seqbegin(seqlock_t *sl) {
    unsigned s;
    do {
        s = __atomic_load_n(&sl->seq, __ATOMIC_ACQUIRE);
        if (s & 1) continue; // write in progress, retry
        __atomic_thread_fence(__ATOMIC_ACQUIRE);
    } while (s & 1);
    return s;
}
int read_seqretry(seqlock_t *sl, unsigned start) {
    __atomic_thread_fence(__ATOMIC_ACQUIRE);
    return sl->seq != start;  // writer modified during read?
}
```

Used in Linux for the `jiffies` time counter and `ktime_get()` — values readers need frequently and writers update rarely.

## Memory Barriers

Modern CPUs and compilers reorder memory operations for performance. Memory barriers enforce ordering:

| Barrier | Effect |
|---------|--------|
| `smp_mb()` | Full memory barrier — no loads or stores reorder across it |
| `smp_rmb()` | Read barrier — no loads reorder across it |
| `smp_wmb()` | Write barrier — no stores reorder across it |
| `smp_store_release()` | All prior stores visible before this store |
| `smp_load_acquire()` | All subsequent loads see values written before this load |

Without barriers, a CPU might see a pointer updated before the object it points to is initialized — a classic initialization race. Linux uses `smp_store_release()` / `smp_load_acquire()` to safely publish objects lock-free.

## Build It: All Lock Types from Scratch

The code implements spinlock, mutex, RCU, and seqlock in userspace using pthreads and GCC atomics, then benchmarks them.

## Use It

The Linux kernel uses all of these primitives. Choosing correctly matters:
- Spinlock: interrupt handlers, per-CPU data, sub-microsecond critical sections.
- Mutex: anything that might sleep, longer operations.
- RCU: read-mostly data structures (routing tables, caches).
- Seqlock: frequently read, rarely written counters.

## Ship It

A synchronization library implementing these primitives demonstrates the trade-offs: spinlock latency vs mutex CPU cost, RCU reader throughput vs writer overhead, seqlock writer priority.

## Exercises

**Level 1 — Spinlock vs Mutex Benchmark:**
Write a program with N threads incrementing a shared counter. Protect it first with a spinlock, then with a mutex. Measure throughput (ops/sec) for N = 1, 2, 4, 8. At what thread count does the mutex begin to outperform the spinlock?

**Level 2 — RCU Reader-Writer Demo:**
Implement an RCU-protected linked list. One writer thread adds/removes nodes. Multiple reader threads traverse the list. Verify that readers never see torn data. Measure reader throughput vs a mutex-protected list.

**Level 3 — Seqlock for a Concurrent Counter:**
Implement a seqlock-protected struct holding {timestamp, value}. Writer updates both fields atomically via the seqlock. Readers read both fields and verify consistency. Add a timing harness showing retry rate under contention.
