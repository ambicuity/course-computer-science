# Semaphores and the Classics (Producer/Consumer, Dining)

> One primitive to rule them all: Dijkstra's semaphore solves signaling, mutual exclusion, and resource counting with a single integer and two atomic operations. Master it, and you master the three canonical synchronization problems every OS textbook reprints.

**Type:** Build
**Languages:** C, Go
**Prerequisites:** Phase 13 lessons 01–05 (threads, mutexes, condition variables, deadlock, data races)
**Time:** ~60 minutes

## Learning Objectives

- Explain what a semaphore is and distinguish counting semaphores from binary semaphores and mutexes.
- Implement the three classic synchronization problems (producer–consumer, dining philosophers, readers–writers) using POSIX semaphores in C and channels/`sync.Mutex` in Go.
- Identify and fix the two cardinal sins of semaphore use: deadlock (circular wait) and starvation (thread indefinitely bypassed).
- Read and understand production semaphore usage in real codebases (Linux, PostgreSQL, Go runtime).

## The Problem

You have multiple threads that need to coordinate access to a shared resource. A mutex gives you mutual exclusion — only one thread at a time. But what if you have a *pool* of identical resources (three database connections, ten buffer slots, five chopsticks)? What if one thread must *wait for another to produce data* before it can consume it?

You could cobble together a solution with condition variables and a mutex, but the pattern is so universal that Edsger Dijkstra formalised it in 1965 as the **semaphore** — a single integer protected by two atomic operations. Every textbook on operating systems since then has taught it. Every production kernel uses it. And yet, because the API is deceptively simple, it is also one of the most misused primitives in concurrent programming.

The three "classic problems" — producer–consumer, dining philosophers, readers–writers — are not academic toys. They are minimal models of real systems:

| Classic problem | Real-world analog |
|----------------|-------------------|
| Producer–consumer (bounded buffer) | Web server request queue, Kafka topic, audio ring buffer |
| Dining philosophers | Resource allocation with partial ownership (e.g., two-phase locking in databases) |
| Readers–writers | File cache, configuration store, concurrent hash map |

If you can solve these three, you can reason about the vast majority of concurrent coordination patterns.

## The Concept

### Semaphore definition

A **semaphore** `S` is an integer variable that supports two atomic operations:

- **P** (*prolaag* / "proberen te verlagen" — Dutch for "try to decrease"): wait until `S > 0`, then decrement `S`. If `S == 0`, the calling thread blocks until another thread increments `S`.
- **V** (*verhoog* / "increase"): increment `S`. If any threads are blocked waiting on `S`, one of them is woken.

POSIX spells these `sem_wait` and `sem_post`. C# spells them `WaitOne` and `Release`. The Java standard library deprecated its `Semaphore` constructor that defaults to fair ordering! But the abstract machine is always the same:

```
P(S):   ⟨await S > 0; S = S - 1⟩
V(S):   ⟨S = S + 1⟩
```

The angle brackets denote *atomicity* — the check-and-decrement is not interruptible.

### Binary vs. counting semaphores

- **Binary semaphore**: `S` can only be 0 or 1. This is *almost* a mutex, with one critical difference: a mutex has *ownership* — the thread that locked it must unlock it. A semaphore can be V'd by any thread, not just the one that P'd it. Use a mutex for mutual exclusion; use a binary semaphore for signaling.
- **Counting semaphore**: `S` can be any non-negative integer. The initial value represents the number of resources available. A pool of N identical resources → initialize to N.

### Semaphore vs. mutex

| Property | Mutex | Binary semaphore |
|----------|-------|-----------------|
| Ownership | Yes — locker must unlock | No — any thread can V |
| Recursive locking | Optional (often supported) | Would deadlock (P then P on same thread) |
| Priority inheritance | Often supported | Usually not |
| Use case | Mutual exclusion | Signaling / event completion |

**Rule of thumb:** if you are protecting a critical section, use a mutex. If you are signaling between threads, use a semaphore. The bounded-buffer problem uses *both*: mutex for buffer access, semaphores for slot availability.

## Build It

All three implementations live in `code/main.c` (POSIX threads + `semaphore.h`) and `code/main.go` (goroutines + channels). Compile and run:

```bash
# C
gcc -pthread -o prodcon code/main.c && ./prodcon

# Go
go run code/main.go
```

### 1. Producer–Consumer (Bounded Buffer)

**The scenario:** One or more producer threads generate items and place them into a fixed-size ring buffer. One or more consumer threads remove and process them. Producers must block when the buffer is full; consumers must block when it is empty.

**The semaphore solution:**

```
sem_t empty;   // count of empty slots, initialized to BUFFER_SIZE
sem_t full;    // count of filled slots, initialized to 0
pthread_mutex_t mutex;  // protects buffer metadata

Producer:
    P(empty)            // wait for an empty slot
    P(mutex)            // lock buffer
    // add item to buffer
    V(mutex)            // unlock buffer
    V(full)             // signal that a slot is now full

Consumer:
    P(full)             // wait for a filled slot
    P(mutex)            // lock buffer
    // remove item from buffer
    V(mutex)            // unlock buffer
    V(empty)            // signal that a slot is now empty
```

Why two semaphores? The `empty` and `full` semaphores decouple *space* from *data*. Producers compete for space, consumers compete for data. The mutex only protects the pointer manipulations inside the critical section. With a lock-free ring buffer (Phase 13 later), you could even eliminate the mutex.

### 2. Dining Philosophers

**The scenario:** Five philosophers sit at a round table. Each needs two chopsticks to eat. Between each pair of philosophers lies one chopstick. They alternate between thinking (no chopsticks) and eating (both chopsticks). The problem is to allocate the chopsticks so that no deadlock occurs and no philosopher starves.

**Naive solution (deadlocks):** Each philosopher picks up left chopstick, then right. If all five pick up left simultaneously, all five wait for the right — deadlock.

**Deadlock-free solution (resource ordering):** Number the chopsticks 0–4. Each philosopher picks up the *lower-numbered* chopstick first, then the higher-numbered one. This breaks the circular wait — the last philosopher (holding chopstick 4) cannot pick up chopstick 0 because 0 is lower-numbered and already taken. At least one philosopher can always eat.

```
Philosopher i:
    left  = min(i, (i+1) % 5)
    right = max(i, (i+1) % 5)

    P(chopstick[left])
    P(chopstick[right])
    // eat
    V(chopstick[left])
    V(chopstick[right])
```

**Starvation:** The resource-ordering solution prevents deadlock but does not guarantee fairness. A philosopher could be repeatedly bypassed. Real solutions use a *fair* semaphore (FIFO-ordered wakeup) or a *footman* (a mutex limiting how many philosophers can attempt to pick up chopsticks at once).

### 3. Readers–Writers

**The scenario:** Multiple threads read a shared data structure. Any number of readers can proceed concurrently. But a writer needs exclusive access — no readers and no other writers while writing.

**The semaphore solution (first readers–writers problem):**

```
sem_t rw_mutex;   // mutual exclusion for writers
sem_t mutex;      // protects read_count
int read_count = 0;

Writer:
    P(rw_mutex)
    // write
    V(rw_mutex)

Reader:
    P(mutex)
    read_count++
    if (read_count == 1) P(rw_mutex)   // first reader locks writers out
    V(mutex)
    // read
    P(mutex)
    read_count--
    if (read_count == 0) V(rw_mutex)   // last reader lets writers in
    V(mutex)
```

This gives readers priority — if a steady stream of readers arrives, writers starve. The "second readers–writers problem" gives writers priority (or fair ordering) by adding an additional semaphore. In production, use `pthread_rwlock_t` or Go's `sync.RWMutex` — they handle the priority policy correctly.

## Use It

### POSIX semaphores (`semaphore.h`)

| Function | Description |
|----------|-------------|
| `sem_init(sem_t *s, int pshared, unsigned int value)` | Initialize semaphore with `value`. `pshared=0` for threads in the same process. |
| `sem_wait(sem_t *s)` | P operation. Blocks if `s == 0`. |
| `sem_trywait(sem_t *s)` | Non-blocking P. Returns -1 with `errno=EAGAIN` if `s == 0`. |
| `sem_timedwait(sem_t *s, const struct timespec *abs_timeout)` | P with timeout. |
| `sem_post(sem_t *s)` | V operation. |
| `sem_getvalue(sem_t *s, int *val)` | Read current value (racy — for debugging only). |
| `sem_destroy(sem_t *s)` | Clean up. |

### Go: buffered channels as counting semaphores

A Go channel with capacity N behaves exactly like a counting semaphore:

```go
var sem = make(chan struct{}, 5)   // initial count = 5

sem <- struct{}{}   // P: send blocks if channel full
<-sem               // V: receive frees a slot
```

The Go standard library also provides `sync.Mutex` (mutual exclusion) and `sync.RWMutex` (readers–writer lock). Use `sync.WaitGroup` when you need to wait for a collection of goroutines to finish — that is also a counting semaphore under the hood.

### Production codebase pointers

- **Linux kernel:** `kernel/locking/semaphore.c` — the `struct semaphore` implementation, including `down()` (P) and `up()` (V) with the MCS lock-based slow path.
- **PostgreSQL:** `src/backend/storage/lmgr/s_lock.c` and spinlock/semaphore wrappers — PostgreSQL uses semaphores to implement lightweight locks for its shared buffer pool.
- **Go runtime:** `runtime/sema.go` — the runtime's semaphore implementation used by `sync.Mutex` and channels.
- **Redis:** `src/syncio.c` — uses `sem_wait` / `sem_post` for the event loop synchronization.

## Read the Source

- **Linux `kernel/locking/semaphore.c`** (`down()` and `up()`): read how the kernel implements semaphore with a wait queue and the MCS lock for cache-friendly spinning before sleeping. Compare the ~100 lines of C with your implementation.
- **Go `src/runtime/sema.go`** (`runtime_Semacquire` / `runtime_Semrelease`): the semaphore underpins the entire Go scheduler. The `sudog` wait list is the canonical example of a fair semaphore.
- **PostgreSQL `src/include/storage/s_lock.h`**: the spinlock-and-semaphore abstraction that makes PostgreSQL's shared memory concurrent.

## Ship It

The reusable artifact for this lesson lives in `outputs/`. It includes:

- **Compiled binaries** (`prodcon_c`, `prodcon_go`) that demonstrate all three synchronization problems.
- **Reference snippets** for the three patterns: producer–consumer, dining philosophers, readers–writers.
- **A performance comparison** — how the C POSIX semaphore implementation compares with Go's channel-based approach under contention.

You will reuse the producer–consumer ring buffer in Phase 14 (lock-free data structures) and the readers–writers pattern in Phase 15 (transactional memory).

## Exercises

1. **Easy** — Re-implement the bounded buffer with *multiple* producers and *multiple* consumers. Verify that the `empty` and `full` semaphores correctly serialize access, while the mutex only protects pointer updates. Remove the mutex — does the program still work? Why or why not?

2. **Medium** — Add a *fairness* guarantee to dining philosophers so that no philosopher waits more than twice as long as any other. One approach: a "footman" mutex that limits the number of philosophers at the table to 4. Another: use a priority queue of chopstick requests. Implement both and compare.

3. **Hard** — Implement the *second readers–writers problem* (writers-preference). Add a `sem_t write_try` semaphore that new readers must acquire before incrementing `read_count`. Now a writer that is waiting will block new readers from entering. Prove (informally) that writer starvation is eliminated. Measure read vs. write throughput.

4. **Challenge** — Replace the POSIX semaphore with a *futex* (fast userspace mutex) from `linux/futex.h`. A futex is the building block for most modern semaphores. Implement your own `my_sem_wait` and `my_sem_post` using `futex_wait` and `futex_wake` with spin-on-atomic-flag before sleeping.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Semaphore | A synchronization primitive with a counter | An integer with two atomic operations: P (wait/decrement) and V (signal/increment) |
| Counting semaphore | A semaphore with a value > 1 | Tracks multiple identical resources; initialization value sets the pool size |
| Binary semaphore | A semaphore that is only 0 or 1 | Same API as counting, but used for signaling rather than resource counting |
| P / V | Wait and signal | From Dutch *prolaag* (try to lower) and *verhoog* (increase); Dijkstra's original terminology |
| Dijkstra | The guy who made semaphores | Edsger W. Dijkstra, 1930–2002; invented semaphores in 1965; also known for Dijkstra's algorithm, guarded commands, and "Go To Statement Considered Harmful" |
| Producer–consumer | The bounded buffer problem | Type 1 producers generate data, type 2 consumers process it; solved with empty/full semaphores plus a mutex |
| Dining philosophers | Five philosophers with five chopsticks | A resource allocation problem that forces you to think about deadlock (circular wait) and starvation |
| Readers–writers | Multiple readers, exclusive writers | Solves read concurrency vs. write exclusivity; first problem favors readers, second problem favors writers |
| Deadlock | Programs freeze | Circular wait for resources; prevented by resource ordering (dining philosophers) or by acquiring all resources at once |
| Starvation | Some threads never make progress | A thread is repeatedly bypassed; prevented by fair semaphores (FIFO ordering) or explicit priority mechanisms |

## Further Reading

- Dijkstra, E. W. (1965). "Cooperating Sequential Processes." — The original semaphore paper. EWD-123. Available at the University of Texas archive.
- Silberschatz, A., Galvin, P. B., & Gagne, G. *Operating System Concepts* (10th ed.). Chapter 6: Synchronization. — The canonical textbook treatment of the three classic problems.
- Tanenbaum, A. S. & Bos, H. *Modern Operating Systems* (4th ed.). Chapter 2: Processes and Threads. — Excellent diagrams of the bounded buffer state machine.
- Herlihy, M. & Shavit, N. *The Art of Multiprocessor Programming* (2nd ed.). Chapter 8: Monitors and Blocking Synchronization. — Shows how semaphores relate to monitors and condition variables.
- Linux man pages: `sem_overview(7)`, `sem_init(3)`, `sem_wait(3)`, `sem_post(3)`.
- Go blog: "Share Memory By Communicating" (2010) — Explains the channel-as-semaphore idiom used in the Go community.
