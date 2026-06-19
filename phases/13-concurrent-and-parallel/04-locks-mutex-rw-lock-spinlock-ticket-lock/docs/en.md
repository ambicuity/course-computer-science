# Locks — Mutex, RW Lock, Spinlock, Ticket Lock

> Locks — Mutex, RW Lock, Spinlock, Ticket Lock — the part of CS you can't skip.

**Type:** Build
**Languages:** C, Rust
**Prerequisites:** Phase 13, Lessons 01–03 (Concurrency vs Parallelism; Race Conditions; Memory Models)
**Time:** ~75 minutes

## Learning Objectives

- Understand mutual exclusion: why shared state needs protection and what happens without it.
- Implement four lock types from scratch: spinlock, ticket lock, mutex (spin+yield), and an RW lock pattern.
- Distinguish the four locks on throughput, fairness, blocking behavior, and memory footprint.
- Measure contention overhead and explain why a "fast" lock can be slow under high contention.
- Explain deadlock, priority inversion, and lock poisoning.
- Compare your implementations against production equivalents (pthread_mutex, Linux kernel MCS lock, Rust std::sync).

## The Problem

Two threads increment a shared counter. Without synchronization, the result is wrong — you saw that in Lesson 02. But the fix introduces new questions: *which synchronization primitive should you use?*

Consider a web cache built on a hash table. Multiple worker threads read and occasionally update entries. You need to protect the table's internal structure (buckets, linked lists, resizing flags). The wrong lock choice means:

- **Spinlock:** Workers burn CPU spinning while the cache does a 100 µs disk fetch. Throughput collapses. Power bill rises.
- **Mutex:** Workers block and yield the CPU. Good for long critical sections. But on a hot cache line contested by 64 cores, the mutex's internal spin-waiting in the kernel creates a thundering herd.
- **RW Lock:** Perfect for read-mostly workloads (reads dominate writes 100:1). But if a writer is stuck behind readers that never finish, the writer starves.
- **Ticket lock:** Fair — no thread starves. But "fair" does not mean "fast." The FIFO order can hurt when the thread that should run next is on a remote NUMA node.

The problem is not "which lock is correct?" but "which lock is correct *and* fast for this specific workload?" Getting this wrong is the difference between a cache serving 1M req/s and a cache serving 10K req/s.

This lesson builds all four locks from scratch, benchmarks them, and teaches you to reason about the trade-off space.

## The Concept

### Mutual Exclusion Primitives

A **lock** (or mutex, short for "mutual exclusion") provides the **mutual exclusion property**: at most one thread executes inside a **critical section** protected by the lock. Formally:

> For any lock `L`, if thread `T1` holds `L` and thread `T2` attempts to acquire `L`, then `T2` must wait until `T1` releases `L`.

Beyond correctness, locks have three performance-critical properties:

| Property | Meaning | Why it matters |
|----------|---------|----------------|
| **Fairness** | Every thread that waits eventually gets the lock | Prevents starvation — a thread never making progress |
| **Throughput** | Critical sections/second given N threads | Directly determines application performance |
| **Scalability** | How performance changes as threads increase | A lock that works at 4 threads may collapse at 64 |

### Four Lock Architectures

#### 1. Spinlock — busy-wait

The simplest lock: a thread loops (spins) until the lock is free.

```
Thread A: acquire → while(lock is held) { PAUSE } → critical section → release
Thread B: acquire → while(lock is held) { PAUSE } → critical section → release
```

- **Pros:** No system calls, no context switch, lowest latency (~10-30 ns uncontended).
- **Cons:** Burns CPU while waiting; wastes power; can cause priority inversion.
- **When to use:** Critical sections < 1 µs; thread count ≤ core count; real-time where you can't block.

#### 2. Ticket Lock — fair FIFO spinlock

A ticket lock assigns each waiter a ticket number. Threads proceed in ticket order:

```
Lock state: { ticket: 5, turn: 3 }

Thread C arrives:  my_ticket = fetch_add(ticket, 1) → 5
                   while (turn != 5) { PAUSE }
                   // turn advances through 3, 4, then 5 — C proceeds
```

- **Pros:** Strong FIFO fairness — no thread starves.
- **Cons:** Contention on the `turn` variable (all threads read the same cache line); slower than a plain spinlock under low contention.
- **When to use:** You need fairness guarantees; NUMA systems (avoid starvation on remote nodes).

#### 3. Mutex (blocking lock)

A mutex blocks the thread if the lock is held. The OS deschedules the waiter and reschedules it later. A typical implementation spins briefly first (optimistic that the lock will be free soon), then falls back to a blocking system call.

```
mutex_lock:
    if cmpxchg(lock, 0, 1) succeeds → got lock (fast path, ~25 ns)
    else → spin N iterations
           if still held → futex_wait(&lock)  ← system call (~1-5 µs)

mutex_unlock:
    lock = 0
    futex_wake(&lock, 1)  ← wake one waiter
```

- **Pros:** No wasted CPU while waiting; good for long or contended critical sections.
- **Cons:** System calls are slow (~1-5 µs); kernel locking overhead.
- **When to use:** Critical sections > 1 µs; contended scenarios; I/O within critical section.

#### 4. RW Lock — multiple readers, exclusive writer

An RW lock distinguishes reads (shared) from writes (exclusive):

| Operations allowed | At most one writer | Any number of readers |
|--------------------|--------------------|-----------------------|
| Read | Not allowed (writer blocks readers) | Allowed simultaneously |
| Write | Not allowed | Not allowed |

```
State: readers = 0, writer = false

read_lock:   if writer is false → readers++
             else → wait

read_unlock: readers--
             if readers == 0 → wake waiting writer

write_lock:  while (readers > 0 || writer) → wait
             writer = true

write_unlock: writer = false → wake all waiting readers + writers
```

- **Pros:** Excellent read throughput — hundreds of concurrent readers.
- **Cons:** Writer starvation if readers never release; slightly slower than mutex for writes; more complex.
- **When to use:** Read-mostly workloads (config caches, routing tables, counters).

### Deadlock

Deadlock is the "deadly embrace": threads waiting for each other's locks in a cycle.

```
Thread A: lock(L1) → lock(L2) → ... → unlock(L2) → unlock(L1)
Thread B: lock(L2) → lock(L1) → ... → unlock(L1) → unlock(L2)

If A holds L1 and B holds L2 simultaneously → DEADLOCK.
```

Four necessary conditions (Coffman, 1971):
1. **Mutual exclusion** — resources cannot be shared
2. **Hold and wait** — a thread holds resources while waiting for others
3. **No preemption** — resources cannot be forcibly taken
4. **Circular wait** — a cycle of threads each waiting for the next

Break any one condition to prevent deadlock. The most common fix: **lock ordering** — always acquire locks in a fixed global order (e.g., address order).

### Priority Inversion

A high-priority thread waits for a low-priority thread that has been preempted by a medium-priority thread. The high-priority thread effectively runs at the priority of the lowest-priority thread.

**Classic case:** Mars Pathfinder (1997). The spacecraft's computer reset repeatedly due to priority inversion on a shared mutex. The fix: **priority inheritance** — temporarily boost the low-priority thread to the highest waiter's priority while it holds the lock.

## Build It

You will implement four lock types and benchmark them. Open `code/main.c` for the C implementations (spinlock, ticket lock, mutex) and `code/main.rs` for the Rust implementations (Mutex with poisoning, RwLock).

### Step 1: Spinlock from Scratch (C)

A spinlock needs one thing: an atomic test-and-set that returns the old value. C11's `atomic_flag` provides this:

```c
typedef struct {
    atomic_flag flag;
} spinlock_t;

void spinlock_lock(spinlock_t *l) {
    while (atomic_flag_test_and_set(&l->flag)) {
        /* spin — on x86, a PAUSE instruction helps */
        PAUSE();
    }
}

void spinlock_unlock(spinlock_t *l) {
    atomic_flag_clear(&l->flag);
}
```

`atomic_flag_test_and_set` atomically sets the flag to `true` and returns the previous value. If the old value was already `true`, the lock is held — loop. Otherwise, we now hold the lock.

The `PAUSE` instruction (x86) or `YIELD` (ARM) hints to the CPU that we are in a spin loop. This:
- Reduces power consumption
- Prevents memory-ordering pipeline stalls
- Improves hyper-thread fairness

**File:** `code/main.c` — functions `spinlock_init`, `spinlock_lock`, `spinlock_unlock`.

### Step 2: Ticket Lock from Scratch (C)

A ticket lock adds fairness. Every locker gets a ticket number; they wait until their ticket is `turn`.

```c
typedef struct {
    atomic_uint ticket;   // next ticket to hand out
    atomic_uint turn;     // whose turn it is now
} ticketlock_t;

void ticketlock_lock(ticketlock_t *l) {
    unsigned my_ticket = atomic_fetch_add(&l->ticket, 1);
    while (atomic_load(&l->turn) != my_ticket) {
        PAUSE();
    }
}

void ticketlock_unlock(ticketlock_t *l) {
    atomic_fetch_add(&l->turn, 1);
}
```

`atomic_fetch_add` returns the old value atomically — so every thread gets a unique, monotonically increasing ticket. The `turn` variable advances one-by-one as threads release the lock.

**Fairness guarantee:** If thread A got ticket 5 and thread B got ticket 6, A always goes before B. No thread can skip ahead.

**Trade-off:** All waiting threads read the same `turn` variable. On a multi-socket (NUMA) machine, this means cache-line bouncing across sockets — the ticket lock does not scale past ~8-16 threads on multi-socket systems.

**File:** `code/main.c` — functions `ticketlock_init`, `ticketlock_lock`, `ticketlock_unlock`.

### Step 3: Mutex from Scratch (C)

A mutex combines spinning with yielding. For short critical sections, the lock is released before the scheduler runs — spinning wins. For long sections, yielding frees the CPU for other work.

```c
typedef struct {
    atomic_int locked;   // 0 = free, 1 = held
} mutex_t;

void mutex_lock(mutex_t *m) {
    unsigned spins = 0;
    for (;;) {
        int expected = 0;
        if (atomic_compare_exchange_weak(&m->locked, &expected, 1))
            return;     // acquired!
        if (++spins > 100) {
            spins = 0;
            sched_yield();   // give up the rest of our timeslice
        } else {
            PAUSE();
        }
    }
}

void mutex_unlock(mutex_t *m) {
    atomic_store(&m->locked, 0);
}
```

The fast path (contended CAS on `locked` being 0) completes in ~25-40 ns — no system call. Only after 100 failed spins do we yield. On Linux, a proper mutex would use the `futex` syscall to block the thread entirely, removing it from the run queue.

**Why not always spin?** Spinning consumes CPU and doesn't help if the lock holder is descheduled. Yielding allows the holder to run and release the lock sooner.

**Why not always yield?** Yielding is a system call. Doing it on every acquire attempt would be 100x slower than the CAS fast path.

**File:** `code/main.c` — functions `mutex_init`, `mutex_lock`, `mutex_unlock`.

### Step 4: Mutex with Poisoning (Rust)

Rust's `std::sync::Mutex` wraps a value and provides lock() -> MutexGuard. The guard dereferences to the inner value. If a thread panics while holding the guard, the mutex becomes **poisoned**:

```rust
use std::sync::{Arc, Mutex};

let counter = Arc::new(Mutex::new(0));
let c = Arc::clone(&counter);
let handle = thread::spawn(move || {
    let mut guard = c.lock().unwrap();
    *guard += 1;
    panic!("simulated failure");  // mutex becomes poisoned
});

match counter.lock() {
    Ok(guard) => println!("Value: {}", *guard),
    Err(poisoned) => {
        // Can still recover via into_inner():
        let val = poisoned.into_inner();
        println!("Recovered value: {}", val);
    }
}
```

Poisoning is a **safety feature**: when a thread crashes inside a critical section, the data it was modifying may be in an inconsistent state. The poison prevents other threads from seeing corrupted data. You can recover if you know the data is still valid.

**File:** `code/main.rs` — function `demo_mutex_poisoning`.

### Step 5: RW Lock (Rust)

`std::sync::RwLock` allows multiple concurrent readers or one exclusive writer:

```rust
use std::sync::RwLock;

let cache = RwLock::new(HashMap::new());

// Multiple readers can proceed simultaneously:
let readers: Vec<_> = (0..4).map(|i| {
    thread::spawn(move || {
        let map = cache.read().unwrap();
        println!("Reader {} sees {} entries", i, map.len());
    })
}).collect();

// Writer blocks all readers:
thread::spawn(move || {
    let mut map = cache.write().unwrap();
    map.insert("key", "value");
});
```

The `read()` method returns `RwLockReadGuard`, which dereferences to `&T`. The `write()` method returns `RwLockWriteGuard`, which dereferences to `&mut T`. Multiple readers can hold the guard simultaneously; a writer must wait for all readers and other writers.

**Trade-off:** RwLock adds overhead (~2x) compared to Mutex for exclusive access. Its advantage shows only when read concurrency is high.

**File:** `code/main.rs` — function `demo_rwlock`.

### Step 6: Benchmark All Four (C + Rust)

The C benchmark (`bench_all` in `main.c`):
- 1, 2, 4 threads
- Each thread does 1M lock-acquire-increment-unlock cycles
- Measures wall time for spinlock, ticket lock, mutex (spin+yield), and a "no lock" race version
- Shows the counter value to verify correctness (the race version is wrong)

The Rust benchmark (`main.rs`):
- Compares `Mutex`, `RwLock (read)`, `RwLock (write)`, hand-rolled Spinlock
- Shows lock poisoning recovery

```bash
# Compile and run C:
clang -std=c11 -pthread -O2 -o locks_bench code/main.c && ./locks_bench

# Compile and run Rust:
rustc code/main.rs -o rwlock_demo && ./rwlock_demo
```

## Use It

### pthread_mutex — The POSIX Standard

On every POSIX system, `pthread_mutex_t` is the production mutex. It uses a hybrid approach:
- **Fast path:** CAS on a 32-bit word in user space
- **Slow path:** `futex` (fast userspace mutex) system call to block

```c
pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
pthread_mutex_lock(&lock);
// critical section
pthread_mutex_unlock(&lock);
```

`futex` is Linux's key trick: the kernel only gets involved when there's contention. In the uncontended case, no system call. glibc's implementation (NPTL) is a textbook "futex-based mutex" — spin briefly, then futex_wait.

**Error checking vs recursive vs normal:** `pthread_mutexattr_settype` lets you configure:
- `PTHREAD_MUTEX_NORMAL` — no error checking; deadlock on double-lock (undefined behavior)
- `PTHREAD_MUTEX_ERRORCHECK` — returns EDEADLK on double-lock
- `PTHREAD_MUTEX_RECURSIVE` — same thread can lock multiple times (counted)

### MCS Lock — The Linux Kernel's Scalable Lock

The Linux kernel moved from ticket locks to **MCS locks** (Mellor-Crummey/Scott) for its most contended paths. An MCS lock maintains a per-waiter queue with one cache-line-sized node per thread:

- Each waiter spins on **its own local flag** — no cache-line bouncing.
- The lock handoff advances via a single store to the next waiter's node.

Result: MCS locks scale to hundreds of cores. Ticket locks collapse on large NUMA systems because all waiters read the same `turn` variable (cache-line bouncing across sockets). You can see the MCS implementation in `kernel/locking/mcs_spinlock.h`.

**Our vs production:**
- Our spinlock: 1 word, no queue → can starve, high caching overhead
- Ticket lock: 2 words, FIFO → fair, but cache-line bouncing
- MCS lock: N words (one per thread), NUMA-aware → scales to hundreds of cores

### Rust std::sync::Mutex

Rust's `Mutex` is implemented differently per platform:
- **Linux:** `futex`-based, similar to glibc
- **macOS:** `os_unfair_lock` (a priority-inversion-avoiding spinlock)
- **Windows:** `SRWLOCK` (slim reader-writer lock)

The lock guard uses RAII (Resource Acquisition Is Initialization): the lock is released when the guard goes out of scope, even if the scope is exited via `return` or `?`. This makes it **impossible to forget to unlock**.

```rust
{
    let guard = my_mutex.lock().unwrap();
    // ... critical section ...
} // guard dropped here → automatically unlocks
```

## Read the Source

- **glibc NPTL `pthread_mutex_lock.c`:** `/usr/glibc/nptl/pthread_mutex_lock.c` — the production futex-based mutex. Look at the fast path (CAS), the spin-retry loop, and the fallback to `futex_wait`.
- **Linux kernel ticket lock:** `arch/x86/include/asm/spinlock.h` — the classic ticket lock that was used in the kernel for decades before being replaced by qspinlock.
- **Linux kernel MCS lock:** `kernel/locking/mcs_spinlock.h` — the NUMA-scalable replacement with per-waiter nodes. Shows how kernel developers avoid the cache-line bouncing problem.
- **Linux kernel qspinlock:** `kernel/locking/qspinlock.c` — the current x86 spinlock combining MCS with a "queued" approach; hybrid of ticket lock's simplicity and MCS's scalability.
- **Rust std::sync::Mutex source:** `library/std/src/sys/sync/mutex/` — platform-specific implementations. The `futex.rs` module is particularly instructive.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained lock implementation and benchmark suite** — spinlock, ticket lock, and mutex implementations in C with a timing harness, plus Rust Mutex/RwLock/Spinlock demos. Reuse the spinlock as a building block for lock-free data structures in Phase 13, Lessons 07–09. Reuse the benchmark harness to measure contention in your own concurrent code.

## Exercises

1. **Easy** — Run the C benchmark with 1, 2, 4, and 8 threads. Record the wall times. Which lock scales best? Which collapses fastest? Why?

2. **Medium** — Modify the C spinlock to add an exponential backoff delay (e.g., double a spin limit on each failed attempt, up to some max). Benchmark against the simple spinlock. Does backoff improve or hurt throughput? Under what conditions?

3. **Hard** — Implement a Rust RwLock from scratch using `AtomicUsize` (use the high bit for the writer flag and lower bits for the reader count). Compare its performance against `std::sync::RwLock`. Where is yours slower? Where is it faster? (Hint: your version probably lacks fairness — how would you add it?)

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Mutex | "A lock" | Mutual exclusion primitive. Only one thread holds it at a time. If the lock is taken, the caller blocks (descheduled). |
| Spinlock | "A lock that burns CPU" | Mutual exclusion via busy-waiting. The thread loops until the lock is free. No blocking. |
| Ticket lock | "A fair spinlock" | Spinlock with FIFO ordering. Each waiter gets a ticket; waiters proceed in ticket order. |
| RW Lock | "Many readers, one writer" | Allows multiple concurrent readers OR one exclusive writer. Optimizes for read-mostly workloads. |
| MCS lock | "Scalable lock" | Lock with per-waiter queue nodes; each waiter spins on its own cache line. NUMA-scalable. |
| Contention | "Many threads want the same lock" | The fraction of time threads spend waiting for a lock instead of doing useful work. |
| Fairness | "Everyone gets a turn" | Every waiting thread eventually acquires the lock. Ticket locks are fair; spinlocks are not. |
| Deadlock | "Two locks, two threads, circular wait" | Each thread holds a lock the other needs. Neither can proceed. Four necessary conditions. |
| Priority inversion | "High waits for low" | A high-priority thread is blocked by a low-priority thread holding a shared lock. The fix: priority inheritance. |
| Futex | "Fast userspace mutex" | Linux syscall for blocking threads on a user-space address. Enables hybrid mutexes: fast path in userspace, slow path in kernel. |

## Further Reading

1. **Mellor-Crummey and Scott, "Algorithms for Scalable Synchronization on Shared-Memory Multiprocessors" (1991)** — The paper that introduced MCS locks. Explains why ticket locks fail on NUMA and shows the per-processor queue approach. ACM Transactions on Computer Systems, Vol. 9, No. 1.

2. **Linux kernel documentation on locking:** `Documentation/locking/` — The kernel's locking rules and patterns. Includes `spinlock.txt`, `mutex-design.txt`, and `rt-mutex.txt` (the real-time priority inheritance mutex).

3. **Herlihy & Shavit, "The Art of Multiprocessor Programming," 2nd ed., Chapter 7 (Locks)** — Covers spinlocks, ticket locks, and MCS locks with linearizability proofs. The CLH lock (a variant of MCS) is also presented.

4. **Butenhof, "Programming with POSIX Threads" (1997)** — The standard reference for pthreads. Chapters on mutex attributes, error checking, recursive locks, and deadlock prevention.

5. **Rust `std::sync` documentation:** https://doc.rust-lang.org/std/sync/ — The standard library's synchronization primitives with code examples. The `Mutex`, `RwLock`, and `Barrier` pages include poisoning behavior and performance notes.
