# Lesson 06: Threads, TLS, Context Switching

## Why This Matters

A web server handling 10,000 connections can't afford 10,000 processes. Threads solve this: lightweight execution units sharing the same address space, with minimal overhead. Understanding threads, thread-local storage, and context switching is essential for writing concurrent programs that actually perform.

## Threads vs Processes

```
Process                          Thread
┌──────────────────────┐
│ Code (shared)        │  ┌─ Thread 1 ──────────────┐
│ Data (shared)        │  │  Stack (private)         │
│ Heap (shared)        │  │  Registers (private)     │
│ Open Files (shared)  │  │  TLS (private)           │
│                      │  └──────────────────────────┘
│                      │  ┌─ Thread 2 ──────────────┐
│                      │  │  Stack (private)         │
│                      │  │  Registers (private)     │
│                      │  │  TLS (private)           │
│                      │  └──────────────────────────┘
└──────────────────────┘
```

| Attribute | Process | Thread |
|-----------|---------|--------|
| Address space | Separate | Shared |
| Creation cost | High (fork copies memory) | Low (share existing memory) |
| Context switch | Slow (TLB flush, cache cold) | Fast (same address space) |
| Communication | IPC (pipes, sockets, shm) | Direct (shared memory) |
| Isolation | Full | None (one crash kills all) |

## Thread Control Block (TCB)

Each thread has a TCB storing its private state:

```
┌────────────────────────────┐
│      Thread Control Block  │
├────────────────────────────┤
│  Thread ID                 │
│  Stack pointer (SP)        │
│  Program counter (PC)      │
│  General registers         │
│  TLS pointer               │
│  Signal mask               │
│  Errno (per-thread)        │
│  Stack base + size         │
└────────────────────────────┘
```

## Threading Models

### Kernel Threads (1:1)

Each user thread maps to one kernel thread. The kernel scheduler handles all threads. This is what Linux (pthreads), Windows, and macOS use.

```
User:    T1   T2   T3   T4
          │    │    │    │
Kernel:  KT1  KT2  KT3  KT4    (1:1 model)
```

### User Threads (N:1)

N user threads multiplex onto one kernel thread. Fast scheduling (no kernel involvement) but one blocking syscall blocks all threads. Rarely used today.

```
User:    T1  T2  T3  T4
          \   │   /   │
Kernel:    KT1────────┘         (N:1 model)
```

### Hybrid (M:N)

M user threads multiplexed onto N kernel threads. Complex but used in Go's runtime and old Java implementations.

## POSIX Threads (pthreads)

```c
#include <pthread.h>

void *worker(void *arg) {
    int id = *(int *)arg;
    printf("Thread %d running\n", id);
    return NULL;
}

int main(void) {
    pthread_t t;
    int id = 42;
    pthread_create(&t, NULL, worker, &id);
    pthread_join(t, NULL);  /* wait for thread to finish */
    return 0;
}
```

| Function | Purpose |
|----------|---------|
| `pthread_create()` | Create a new thread |
| `pthread_join()` | Wait for a thread to finish |
| `pthread_exit()` | Terminate the calling thread |
| `pthread_self()` | Get the calling thread's ID |
| `pthread_mutex_lock()` | Acquire a mutex lock |
| `pthread_mutex_unlock()` | Release a mutex lock |

## Race Conditions

When multiple threads access shared data without synchronization, the result depends on timing:

```c
int counter = 0;

void *increment(void *arg) {
    for (int i = 0; i < 100000; i++) {
        counter++;  /* NOT atomic: load, add, store */
    }
    return NULL;
}
```

Two threads running this will likely produce a final count less than 200,000 because the increments interleave at the instruction level.

## Thread-Local Storage (TLS)

TLS gives each thread its own copy of a variable:

```c
__thread int my_errno = 0;   /* each thread gets its own copy */
```

Use cases: `errno`, random number generators, per-thread caching. TLS is stored on the thread's stack or in a dedicated TLS segment.

## Context Switching

Switching between threads (or processes) means saving one thread's CPU state and loading another's:

```
Thread A running
    │
    ▼
Save A's registers to TCB_A
Save A's SP to TCB_A
    │
    ▼
Load B's SP from TCB_B
Load B's registers from TCB_B
    │
    ▼
Thread B running
```

### Cost of Context Switch

| Cost | Description |
|------|-------------|
| Register save/restore | ~50–100 registers, microseconds |
| Cache pollution | Thread B's data evicts Thread A's cached lines |
| TLB flush | Different address spaces require flushing TLB (threads within same process avoid this) |
| Branch predictor pollution | CPU's branch predictor trained on Thread A is now wrong for Thread B |

Thread-to-thread switch within the same process is cheaper than process-to-process switch because the TLB doesn't need flushing and the cache is partially warm.

## Build It

We'll write pthread programs showing thread creation, race conditions, TLS, and measure context switch overhead.

## Use It

Web servers like Apache and Nginx use thread pools. Each incoming connection is assigned to a thread from the pool, avoiding the overhead of creating/destroying threads per request.

## Ship It

See `code/main.c` for working demos of threading, race conditions, TLS, and context switch measurement.

## Exercises

### Level 1 — Recall

What is shared between threads in the same process? What is private to each thread?

### Level 2 — Application

Write a program that creates 8 threads, each incrementing a shared counter 1,000,000 times. Observe the final count without a mutex. Then add a mutex and verify the count is exactly 8,000,000.

### Level 3 — Build

Implement a simple thread pool:

1. Create a fixed number of worker threads (e.g., 4)
2. Maintain a shared task queue (linked list protected by a mutex + condition variable)
3. Workers block on the condition variable when the queue is empty
4. Submit tasks as function pointers + arguments
5. The main thread submits 20 tasks and waits for all to complete

This is the foundation of how production web servers and task schedulers work.
