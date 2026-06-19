# Condition Variables and Monitors

> Condition variables let threads sleep until a condition becomes true — no spinning, no polling, no wasted CPU.

**Type:** Build
**Languages:** C, Rust
**Prerequisites:** Phase 13 lessons 01–04 (Threads, Mutexes, Deadlock, Semaphores)
**Time:** ~75 minutes

## Learning Objectives

- Explain why a mutex alone cannot express "wait until condition X is true."
- Implement a producer-consumer queue with `pthread_cond_t` in C.
- Implement a monitor pattern with `std::sync::Condvar` in Rust.
- Distinguish Mesa semantics (signal keeps lock, waker re-checks) from Hoare semantics (signal transfers lock immediately).
- Handle spurious wakeups correctly in both languages.
- Compare condition variables with the Java `Object.wait()`/`notify()` model.

## The Problem

A mutex guarantees **mutual exclusion** — only one thread touches the critical section at a time. But many concurrent problems need more than exclusion: a thread needs to **wait** until some condition is true.

Consider a bounded buffer: a producer puts items in, a consumer takes them out. If the buffer is empty, the consumer must wait until a producer adds something. With just a mutex, the consumer must **spin** — lock, check, unlock, repeat:

```c
while (buffer_is_empty) {
    pthread_mutex_unlock(&lock);
    sched_yield();          // please, CPU, let someone else run
    pthread_mutex_lock(&lock);
}
```

This is **busy-waiting**. It wastes CPU cycles, increases latency, and doesn't scale. What we need is a primitive that lets a thread **sleep atomically** — release the lock and go to sleep as one operation, so no wakeup is missed between the check and the sleep.

That primitive is the **condition variable**.

## The Concept

A condition variable (CV) is a synchronization primitive that enables threads to wait until a particular condition holds. It is **always used with a mutex**.

### Core API

| Operation | What it does |
|-----------|-------------|
| `wait(lock)` | Atomically: release `lock`, put thread to sleep. When woken: re-acquire `lock` before returning. |
| `signal()` | Wake one thread waiting on this CV (if any). |
| `broadcast()` | Wake all threads waiting on this CV. |

### The Wait-Entry Contract

The canonical usage pattern is always a `while` loop — never an `if`:

```c
pthread_mutex_lock(&lock);
while (!condition) {
    pthread_cond_wait(&cv, &lock);
}
// condition is now true — proceed
pthread_mutex_unlock(&lock);
```

The `while` loop is not paranoia — it is required by correctness:

1. **Spurious wakeups:** The OS may wake a thread even though nobody signaled. POSIX permits this, and it happens in practice on some platforms.
2. **Mesa semantics:** Most systems (pthreads, C++, Rust std, Java) use Mesa semantics: `signal()` wakes the waiter, but the **signaler keeps the lock**. The waiter must re-acquire the lock before running, and by then the condition may be false again (another thread grabbed it first).

### Mesa vs Hoare Semantics

| | Mesa | Hoare |
|---|---|---|
| **Signal behavior** | Signaler keeps lock. Waker is marked "ready" but doesn't run yet. | Signaler transfers lock to waker immediately. Waker runs next. |
| **After wait returns** | Condition may be false — must re-check in a loop. | Condition is guaranteed true — `if` suffices. |
| **Overhead** | Lower (fewer context switches). | Higher (immediate handoff forces scheduling). |
| **Used by** | pthreads, C++, Rust, Java, Windows | Most textbooks (theoretically cleaner) |

All modern systems use Mesa semantics. The `while` loop is the price of admission.

### The Monitor Pattern

A **monitor** is a higher-level concurrency abstraction: a mutex + one or more condition variables that together guard shared state. The monitor's invariant is:
- Shared state is only accessed while holding the mutex.
- Threads that find the state unsatisfying wait on a CV.
- Threads that change the state signal the CV to wake waiters.

```
┌─────────────────────────┐
│      Monitor            │
│  ┌───────────────────┐  │
│  │   Shared State    │  │
│  │   (protected by   │  │
│  │    mutex)         │  │
│  └───────────────────┘  │
│  ┌───────────────────┐  │
│  │  Condition Var 1  │  │
│  │  "has data"       │  │
│  ├───────────────────┤  │
│  │  Condition Var 2  │  │
│  │  "has space"      │  │
│  └───────────────────┘  │
└─────────────────────────┘
```

Languages with native monitor support (Java, C#) bake the mutex into the `synchronized` keyword. C and Rust expose the primitives directly.

## Build It

We implement four programs that build on each other:

1. **Broken Busy-Wait** (C) — Demonstrate why spinning wastes CPU.
2. **Producer-Consumer with CV** (C) — Fix it with `pthread_cond_t`.
3. **Monitor Pattern** (Rust) — Encapsulate Mutex + Condvar in a safe API.
4. **Multiple Conditions** (Rust) — Reader/writer with separate CVs for "has data" and "has space."

### Step 1: Broken Busy-Wait (C)

File: `code/main.c` (compile with `gcc -o busywait main.c -lpthread`)

A producer sets a flag; the consumer spins until the flag changes. Run it and watch CPU usage with `top` or Activity Monitor — one core will be pegged at 100%.

```c
// Core loop of the consumer — do not write code like this:
volatile int flag = 0;

void* consumer(void* arg) {
    while (flag == 0) {
        // spin, spin, spin — 100% CPU doing nothing
    }
    printf("Consumer: flag is set!\n");
    return NULL;
}
```

This works correctly (the consumer eventually sees `flag = 1`), but it is **wasteful and unscalable**. With N consumers, N cores spin. On a laptop with 4 cores, this melts the battery.

### Step 2: Producer-Consumer with CV (C)

Replace the spin with `pthread_cond_wait`. The consumer sleeps until the producer signals.

```c
pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
pthread_cond_t cv = PTHREAD_COND_INITIALIZER;
int flag = 0;

void* consumer(void* arg) {
    pthread_mutex_lock(&lock);
    while (flag == 0) {
        pthread_cond_wait(&cv, &lock);  // atomically: unlock + sleep
    }
    pthread_mutex_unlock(&lock);
    printf("Consumer: flag is set!\n");
    return NULL;
}

void* producer(void* arg) {
    sleep(1);
    pthread_mutex_lock(&lock);
    flag = 1;
    pthread_cond_signal(&cv);           // wake the consumer
    pthread_mutex_unlock(&lock);
    return NULL;
}
```

The full example in `code/main.c` builds a **bounded buffer** (ring buffer) with a mutex, one CV (`can_consume`), proper `while` loops, and a demonstration of the lost-wakeup bug (signal before wait).

### Step 3: Monitor Pattern (Rust)

File: `code/main.rs` (run with `cargo run` or `rustc main.rs && ./main`)

A struct wraps a `Mutex<Vec<u32>>` and a `Condvar`. The public API guarantees the user cannot forget to signal:

```rust
pub struct Channel {
    items: Mutex<Vec<u32>>,
    ready: Condvar,
}

impl Channel {
    pub fn send(&self, msg: u32) {
        let mut guard = self.items.lock().unwrap();
        guard.push(msg);
        self.ready.notify_one();
    }

    pub fn recv(&self) -> u32 {
        let mut guard = self.items.lock().unwrap();
        while guard.is_empty() {
            guard = self.ready.wait(guard).unwrap();
        }
        guard.remove(0)
    }
}
```

The monitor pattern hides the locking and signaling behind method calls. Callers of `send()` and `recv()` cannot forget to lock or signal — the API enforces correctness at the type level.

### Step 4: Multiple Conditions (Rust)

A bounded queue with **two** condition variables: `can_read` (data available) and `can_write` (space available). This is the classic bounded buffer pattern used in real I/O pipelines.

```rust
pub struct BoundedQueue<T> {
    buf: Mutex<Inner<T>>,
    can_read: Condvar,
    can_write: Condvar,
}

struct Inner<T> {
    data: VecDeque<T>,
    capacity: usize,
}
```

`push()` waits on `can_write` until space exists, then signals `can_read`. `pop()` waits on `can_read` until data exists, then signals `can_write`. Two CVs ensure producers don't wake consumers only to find no space (and vice versa).

## Use It

### pthread_cond — C (POSIX threads)

The POSIX condition variable API is the foundation:

| Function | Purpose |
|----------|---------|
| `pthread_cond_wait(&cv, &mutex)` | Atomically unlock + sleep. Re-lock on wake. |
| `pthread_cond_timedwait(&cv, &mutex, &ts)` | Wait with timeout. |
| `pthread_cond_signal(&cv)` | Wake one waiter. |
| `pthread_cond_broadcast(&cv)` | Wake all waiters. |

**Key rules:**
- Always call `pthread_cond_wait` inside a `while` loop (spurious wakeups + Mesa semantics).
- The caller of `pthread_cond_signal` does not need to hold the lock (but usually does for correctness).
- A signal is **lost** if no thread is waiting at the moment of the call. This is the source of the lost-wakeup bug: if the producer signals *before* the consumer waits, the consumer waits forever.

### std::sync::Condvar — Rust

Rust's `Condvar` has a type-safe API that prevents common mistakes:

| Method | Purpose |
|--------|---------|
| `wait(guard)` | Consumes the `MutexGuard`, atomically unlocks + sleeps. Returns a new `MutexGuard` on wake. |
| `wait_while(guard, predicate)` | Combines the `while` loop and `wait` into one call. |
| `notify_one()` | Wake one waiter. |
| `notify_all()` | Wake all waiters. |

The `Condvar::wait` method takes ownership of the `MutexGuard`, preventing use-after-unlock bugs:

```rust
// The guard is consumed — you cannot forget to re-lock.
let guard = self.ready.wait(guard).unwrap();
// guard is back, lock is held.
```

### Java Object.wait() / notify()

Java's built-in monitor model uses `synchronized` blocks and `Object` methods:

```java
synchronized (this) {
    while (!condition) {
        this.wait();           // like pthread_cond_wait
    }
    // condition is true
}
// elsewhere:
synchronized (this) {
    condition = true;
    this.notify();             // like pthread_cond_signal
}
```

Every Java object has an implicit condition variable. The `wait()` method must be called inside a `synchronized` block. The `while` loop is required for the same reason: spurious wakeups and Mesa semantics.

## Read the Source

- **glibc nptl:** `sysdeps/unix/sysv/linux/pthread_cond_wait.c` — the actual futex-based implementation of `pthread_cond_wait`. Look at how it uses the Linux `futex` syscall to avoid the thundering-herd problem.
- **Rust std:** `library/std/src/sys/pal/common/condvar.rs` — the cross-platform `Condvar` implementation. It uses `pthread_cond_t` on Linux and `SRWLOCK` + `CONDITION_VARIABLE` on Windows.
- **Linux kernel:** `kernel/sched/wait.c` — the kernel's own wait-queue mechanism, which inspired the pthread API.

## Ship It

The reusable artifact for this lesson lives in `outputs/`. It is:

- **A bounded-channel snippet (C)** — A drop-in producer-consumer queue using `pthread_cond_t`, suitable for reuse in phase 14 (work-stealing scheduler).
- **A bounded-channel snippet (Rust)** — A type-safe monitor with two condition variables, ready to use in your own concurrent pipelines.

Both handle spurious wakeups, use Mesa-correct `while` loops, and clearly separate the "has data" vs "has space" wait queues.

## Exercises

1. **Easy** — Reproduce the bounded buffer in C from memory. Introduce the lost-wakeup bug by moving the `signal()` call to before the mutex lock. What happens?

2. **Medium** — Add a `timed_pop()` method to the Rust `BoundedQueue` that returns `Option<T>` after a timeout. Use `Condvar::wait_timeout` or `park_timeout`.

3. **Hard** — Implement a **readers-writers lock** using a mutex + two condition variables. A reader acquires if no writer is active; a writer acquires if no readers are active and no writer is waiting. (This is the classic "writers-preference" solution from most OS textbooks.)

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Condition variable | "A way for threads to wait for a condition" | A queue where threads can atomically release a mutex and sleep, then be woken when another thread signals that the condition may be true. |
| Monitor | "A thread-safe object" | A mutex + one or more condition variables that together guard shared state, exposing a safe API. |
| Mesa semantics | "Signal keeps the lock" | The signaler continues holding the mutex; the waker must re-acquire it. The condition may be false when the waker runs — hence the `while` loop. |
| Hoare semantics | "Signal transfers the lock" | The signaler immediately hands the mutex to the waker. The condition is guaranteed true when the waker runs. Rare in practice. |
| Spurious wakeup | "Thread woke up for no reason" | A `wait()` returns even though nobody called `signal()`. The OS or runtime may do this for implementation reasons. The `while` loop handles it. |
| Signal / notify | "Wake one waiter" | Move one thread from the CV's wait queue to the ready queue. The thread will run when it re-acquires the mutex. |
| Broadcast / notifyAll | "Wake all waiters" | Move every thread on the CV's wait queue to the ready queue. One will get the lock; the others will re-enter the wait loop. |
| Producer-consumer | "One thread makes items, another consumes them" | The classic bounded-buffer coordination problem, solved elegantly with two condition variables. |
| Bounded buffer | "A fixed-size ring buffer" | A queue with a maximum capacity that blocks the producer when full and the consumer when empty. |

## Further Reading

- *Operating Systems: Three Easy Pieces* (Chapters 30) — The definitive treatment of condition variables.
- *The Little Book of Semaphores* — Dozens of puzzles solved with semaphores and condition variables.
- POSIX specification: `pthread_cond_wait` — The standard that defines spurious-wakeup legality.
- Java `Object.wait()` Javadoc — The classic "wait/notify" pattern with detailed correctness reasoning.
- Rust `std::sync::Condvar` docs — The standard-library documentation with examples.
