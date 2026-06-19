# Phase Capstone — A Work-Stealing Scheduler + Lock-Free Queue

> Phase Capstone — A Work-Stealing Scheduler + Lock-Free Queue — the part of CS you can't skip.

**Type:** Build (Capstone)
**Languages:** Rust
**Prerequisites:** Phase 13 lessons 01–21 (atomics, CAS, memory ordering, MS queue, Chase-Lev, thread pools, async, actors, STM, GPU, MPI)
**Time:** ~150 minutes

## Learning Objectives

- Integrate Chase-Lev work-stealing deque (lock-free) with a Michael-Scott lock-free queue into a unified thread-pool scheduler.
- Implement random-victim work stealing: idle workers steal tasks from busy workers' deques to achieve dynamic load balancing.
- Understand the trade-offs between work-stealing, thread-per-task, and mutex-based thread pools through empirical benchmarking.
- Build a benchmark suite (Fibonacci, parallel map, tree traversal) that measures throughput, steal rate, and scalability.
- Ship a reusable work-stealing scheduler artifact suitable for embedding in concurrent Rust programs.

## The Problem

Through Phase 13 you have built atomics, locks, lock-free data structures, channels, actors, async runtimes, and GPU kernels. These are the building blocks. But a real parallel runtime — the kind that powers Rayon, Tokio, Java's ForkJoinPool, or .NET's Task Parallel Library — must integrate them into a single cohesive system.

The core challenge is **load balancing**. Consider 8 cores processing 1000 tasks. Some tasks finish quickly (cache hit, simple computation), others take 100x longer (cache miss, complex recursion). If you naively partition tasks round-robin at start, cores with easy tasks sit idle while cores with hard tasks lag behind. Static partitioning wastes cores.

**Work stealing** solves this dynamically: each worker thread has its own deque of tasks. Workers push/pop from the bottom of their own deque (LIFO — good cache behavior). When a worker runs out of tasks, it **steals** from the top of a random victim's deque (FIFO — the oldest, coldest tasks). This automatically redistributes work from busy to idle cores.

Work stealing achieves three properties simultaneously:
1. **Low overhead**: pushing/popping from the local deque is fast (no contention, no CAS in the common case).
2. **Automatic load balancing**: idle workers steal from busy ones with no centralized coordinator.
3. **Cache-friendly**: workers process their own deque LIFO, reusing cache-hot data.

This capstone integrates everything from Phase 13: atomic memory ordering, CAS-based lock-free structures, memory reclamation (tagged pointers), thread synchronization, parallelism patterns, and benchmarking methodology.

### The Integration Challenge

A work-stealing scheduler requires three components that must work together correctly:

```
External submitter ──→ MS Queue ──→ Workers (pop + execute)
                                         │
                                    ┌─────┴─────┐
                                    │  Deque[0]  │  ← Worker 0 pushes/pops here
                                    │  Deque[1]  │  ← Worker 1 pushes/pops here
                                    │  Deque[2]  │  ← Worker 2 pushes/pops here
                                    │  Deque[3]  │  ← Worker 3 pushes/pops here
                                    └────────────┘
                                         │
                                    Idle worker steals
                                    from random victim's deque
```

The **Chase-Lev deque** is the data structure for each worker. The **Michael-Scott queue** is the submission queue for tasks arriving from external (non-worker) threads. The **thread-local state** determines which deque an incoming task goes to. The **XorShift PRNG** selects random victims for stealing.

Each component's correctness depends on precise atomic memory ordering:
- **Push (owner)**: store value, then `Release`-increment bottom. The store must be visible to stealers before bottom is updated.
- **Pop (owner)**: `Relaxed`-decrement bottom, then `SeqCst`-fence, then read top. The fence ensures the decrement is visible before top is read, preventing a race with stealers on the last item.
- **Steal (thief)**: `Acquire`-load top, then `SeqCst`-fence, then `Acquire`-load bottom. The fence ensures top is visible before bottom is read. Then CAS to claim the item.

Get any of these orderings wrong, and the deque corrupts under contention.

## The Concept

### Chase-Lev Work-Stealing Deque

The Chase-Lev deque (1994) is a lock-free double-ended queue optimized for the work-stealing pattern:

- **Owner** (the thread that owns the deque) pushes and pops from the **bottom**.
- **Thieves** (other threads) steal from the **top**.

The owner's push/pop operations are uncontended in the common case — no CAS required (except on the last item, where a race with thieves is possible). Only the steal operation uses CAS (on `top`).

**Memory layout:**

```
bottom ──→ [slot 0] [slot 1] [slot 2] ... [slot N-1]  ←── top
               ↑                                  ↑
           thieves steal                      owner pushes/popp
           from here (FIFO)                   here (LIFO)
```

The deque uses a circular buffer with power-of-two size. Indices grow monotonically; the actual array index is `index & mask`. This avoids wraparound modulus operations.

**Push algorithm (owner only, no contention):**
```
buffer[bottom & mask] = value
bottom += 1            // Release store
```

If `bottom - top >= capacity`, the deque is full. The owner can either grow the buffer or execute the task directly.

**Pop algorithm (owner, may race with stealers on last item):**
```
bottom -= 1             // Relaxed, then SeqCst fence
if top <= bottom:
    value = buffer[bottom & mask]
    if top == bottom:   // Last item — race with stealers!
        if CAS(top, bottom, bottom + 1):  // we won
            return value
        else:            // stealer won
            bottom = top + 1
            return EMPTY
    return value
else:                   // Empty or already stolen
    bottom = top
    return EMPTY
```

The key insight: the owner decrements bottom *before* reading top. This ensures that when a stealer reads `top` then `bottom`, it sees consistent state — the stealer cannot miss the last item because the owner has already reserved it by decrementing.

**Steal algorithm (thief, full CAS):**
```
top_old = top           // Acquire load, then SeqCst fence
bottom = bottom         // Acquire load
if top_old < bottom:
    value = buffer[top_old & mask]
    if CAS(top, top_old, top_old + 1):  // claim it
        return value
    // CAS failed — another thief or owner on last item
return EMPTY
```

### Michael-Scott Queue (Submission Queue)

The MS queue provides a **lock-free FIFO** for tasks submitted from external threads (threads that are not workers). It uses a dummy node invariant:

- `head` always points to a dummy node.
- `tail` points to the last node (or the dummy if empty).

Enqueue: CAS on `tail->next` to link the new node, then best-effort CAS to advance `tail`. Dequeue: CAS on `head` to advance past the dummy, returning the first real node's data.

The MS queue is not wait-free (threads may retry CAS), but it is lock-free — at least one thread makes progress per CAS attempt. It is ideal for the submission queue because external spawns are relatively infrequent compared to local task dispatch.

### Work-Stealing Scheduler Architecture

The thread pool follows the **LIFO/FIFO split**:

1. **Spawn from worker thread**: push to the calling worker's deque (bottom, LIFO). This task is likely to be popped next by the same worker, maximizing cache reuse.
2. **Spawn from external thread**: enqueue to the global MS submission queue. Any worker may dequeue it when its own deque is empty.
3. **Worker main loop**:
   - Pop from own deque (bottom, LIFO — hot tasks).
   - If empty, try dequeue from submission queue.
   - If empty, steal from a random victim's deque (top, FIFO — cold tasks).
   - If nothing, yield the CPU.

**Victim selection** uses random uniform selection. XorShift64 provides a fast, high-quality PRNG. The randomness ensures that no single worker is disproportionately targeted for stealing.

### Why No Locks?

The entire scheduler is lock-free:
- **Chase-Lev deque**: only CAS on steal (and pop for the last item). Push is CAS-free.
- **MS queue**: CAS on enqueue and dequeue. No locks anywhere.
- **Thread state**: `thread_local!` variables — no synchronization needed per access.

This means the scheduler cannot deadlock, cannot suffer priority inversion, and does not convoy. A thread descheduled mid-operation does not block any other thread.

### The ABA Problem

Both the Chase-Lev deque and the MS queue are vulnerable to the **ABA problem**: a pointer value changes and cycles back to the same bit pattern between a read and a CAS.

The MS queue in this implementation uses tagged pointers (a generation counter packed into the high 16 bits of the pointer value). Each successful CAS increments the tag. Even if a memory address is freed and reallocated, the tag will not match and the CAS will fail.

The Chase-Lev deque uses indices (integers), not pointers, for `top` and `bottom`. Indices grow monotonically and never cycle, so the ABA problem does not apply to the top/bottom CAS. The buffer entries are read from known-good indices, so no ABA issue arises from the data itself.

## Build It

### Step 1: Chase-Lev Deque Implementation

The implementation uses `AtomicIsize` for `top` and `bottom` (to allow the `b - 1` decrement below 0, though in practice indices stay positive). The buffer is a `Box<[MaybeUninit<T>]>` inside `UnsafeCell` to allow shared mutation.

Key functions:

- **`push(&self, value: T) -> bool`**: writes `value` at `buffer[bottom & mask]`, then `Release`-stores `bottom + 1`. Returns `false` if the deque is full (failure path: caller falls back to executing directly or growing).

- **`pop(&self) -> Option<T>`**: `Relaxed`-reads bottom, decrements, `Relaxed`-stores, then issues a `SeqCst` fence. Reads top. If `top <= bottom`, reads the value from the buffer. If `top == bottom` (last item), attempts CAS on top to claim exclusivity. On CAS failure, a stealer won — returns `None` and resets bottom.

- **`steal(&self) -> Option<T>`**: `Acquire`-loads top, `SeqCst`-fence, `Acquire`-loads bottom. If `top < bottom`, reads the value and CAS top forward. Returns `None` on failure or empty deque.

The memory ordering is critical: the `SeqCst` fence in both `pop` and `steal` creates a total order on the top/bottom operations that prevents races. Without it, a stealer could miss items or two threads could claim the same item.

### Step 2: Michael-Scott Queue Implementation

Reuses the implementation from Lesson 8 with tagged pointers for ABA protection. Two atomics: `head` and `tail`, both packed `(pointer + tag)`.

`enqueue`: allocates a new node, then loops reading tail, checking tail->next. If next is null, CAS tail->next from null to new node. If next is non-null, tail is lagging — CAS tail forward (helping another thread's enqueue).

`dequeue`: reads head, checks if head == tail (empty), reads head->next, CAS head from old head to next. Returns the data from the old dummy's next node.

The dummy node invariant means head and tail are never null, eliminating the need for special-case CAS logic in empty-state transitions.

### Step 3: Thread Pool with Work Stealing

The `WorkStealingPool` manages:
- **N worker threads**, each with an `Arc<ChaseLevDeque<Task>>`.
- **A shared `Arc<MSQueue<Task>>`** for external task submission.
- **An `AtomicIsize` task counter** tracking remaining tasks for `wait()`.
- **An `AtomicBool` shutdown flag** for graceful termination.
- **A `WorkerStats` struct** collecting per-worker metrics.

Worker startup sets `THREAD_STATE` thread-local variables (`is_worker`, `worker_id`), enabling `spawn()` to detect whether the calling thread is a worker.

The `spawn()` method:
1. Increments the task counter.
2. Wraps the task to decrement the counter on completion.
3. If called from a worker thread: pushes to that worker's deque. If the deque is full, executes directly.
4. If called from an external thread: enqueues to the MS submission queue.

The `wait()` method spins (with stealing) until `tasks_remaining` reaches 0. While waiting, the calling thread helps by stealing tasks — this prevents the main thread from being purely overhead.

### Step 4: Benchmark Suite

Three benchmarks exercise different parallelism patterns:

**Fibonacci (512 × fib(25))** — Embarrassingly parallel, uniform task size. Measures raw throughput of task dispatch/execution. Each task is independent with no data sharing.

**Parallel Map (1,000,000 elements in 4 chunks)** — Data-parallel work distribution. How well does the scheduler handle large-grain parallelism with minimal synchronization?

**Tree Traversal (16 trees, depth 18)** — Memory-bound irregular workload. Each tree sum traverses heap-allocated nodes with unpredictable access patterns.

Each benchmark runs against three scheduler variants:
1. **Work-Stealing Pool** (this lesson's artifact)
2. **Thread-per-Task** (one OS thread per task, naive)
3. **Mutex Pool** (threads compete for a single mutex-protected queue)

The statistics report tasks stolen, tasks executed locally, steal attempts, and success rate — showing exactly how well load balancing works.

## Use It

### Rust: Rayon

Rayon is the production work-stealing scheduler for Rust. Its internals:

- Uses `crossbeam-deque` (Chase-Lev) for per-worker deques.
- Uses `crossbeam-epoch` for memory reclamation (EBR, not tagged pointers).
- Provides parallel iterators, `join`, and `scope` for structured parallelism.
- Uses LIFO for local tasks, FIFO for steals (same as this lesson).

```rust
use rayon::prelude::*;

let sum: u64 = (0..1_000_000u64).into_par_iter()
    .map(|x| x * x)
    .sum();
```

Your implementation differs from Rayon in several ways:
- Rayon uses **work-stealing trees** (`join` splits into subtrees), not independent tasks.
- Rayon's `scope` provides structured parallelism with guaranteed completion.
- Rayon uses epoch-based reclamation (EBR) instead of tagged pointers.
- Rayon's growable `crossbeam-deque` handles overflow dynamically.

### Java: ForkJoinPool

Java's `ForkJoinPool` (JSR 166) is the original work-stealing scheduler in production:

```java
ForkJoinPool pool = new ForkJoinPool();
int result = pool.invoke(new RecursiveTask<Integer>() {
    protected Integer compute() {
        if (n <= 1) return n;
        ForkJoinTask<Integer> t = new SubTask(n - 2).fork();
        return new SubTask(n - 1).compute() + t.join();
    }
});
```

ForkJoinPool uses:
- `WorkQueue[]` — array of deques, one per worker.
- `ForkJoinWorkerThread` — the worker threads.
- **Work stealing** with random victim selection.
- **Help thieving**: if a worker is blocked waiting for a `join`, it can steal from the blocked task's deque.
- **ManagedBlocker**: if a worker blocks on external synchronization, it may spawn more workers (compensation).

### .NET: Task Parallel Library (TPL)

.NET's TPL (`Task.Run`, `Parallel.For`) uses the **.NET ThreadPool** with work stealing:

```csharp
Parallel.For(0, 1000, i => Compute(i));
```

TPL integrates work stealing with the I/O completion port thread pool, supporting both CPU-bound and I/O-bound workloads.

## Read the Source

- **crossbeam-deque**: `https://github.com/crossbeam-rs/crossbeam/blob/master/crossbeam-deque/src/deque.rs` — production Chase-Lev implementation (~600 lines). Uses growable circular buffers and EBR for memory safety. The `Injector` (MS queue variant) handles external submissions.
- **crossbeam-epoch**: `https://github.com/crossbeam-rs/crossbeam/blob/master/crossbeam-epoch/src/collector.rs` — epoch-based reclamation used to retire deque entries safely.
- **Rayon core**: `https://github.com/rayon-rs/rayon/blob/master/rayon-core/src/registry.rs` — thread pool registry with work-stealing. The `join` implementation spawns subtasks and sleeps with work-stealing wakeup.
- **Java ForkJoinPool**: `https://github.com/openjdk/jdk/blob/master/src/java.base/share/classes/java/util/concurrent/ForkJoinPool.java` — the original work-stealing thread pool (~2500 lines). Implements help-thieving, compensation, and adaptive spinning.
- **Chase & Lev, "Dynamic Circular Work-Stealing Deque" (SPAA 1994)**: the original paper. Describes the deque algorithm and proves its correctness under the release-consistency memory model.
- **Herlihy & Shavit, "The Art of Multiprocessor Programming," Chapters 16–17**: covers work-stealing deques, schedulers, and the ForkJoinPool pattern in detail.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained work-stealing scheduler binary** in Rust (`main.rs`) with Chase-Lev deque, MS submission queue, thread pool with random-victim stealing, and benchmark suite. Compile with `cargo build --release` from the `code/` directory. The binary produces timing and statistics output for Fibonacci, parallel map, and tree traversal benchmarks, comparing work-stealing against thread-per-task and mutex pool baselines.

## Exercises

1. **Easy** — Run the benchmark with 1, 2, 4, 8 workers (`pool.new(num_workers)`). Record throughput and steal rate for each. At what worker count does the steal rate peak? Why?

2. **Medium** — Add backoff to the steal loop: spin for a few iterations before calling `thread::yield_now()`. Use exponential backoff (1, 2, 4, 8... spins). Measure how backoff affects steal success rate and overall throughput at 8 workers.

3. **Hard** — Implement growable Chase-Lev deques using atomic buffer swap. When a deque fills (bottom - top == capacity), allocate a new buffer twice as large, copy live entries (top..bottom), and atomically replace the buffer pointer. This requires RCU-style memory reclamation since stealers may hold references to the old buffer.

4. **Hard** — Add priority tiers to the scheduler: high-priority tasks go into a separate MS queue that workers check before stealing. Low-priority tasks are only executed when no high-priority or regular tasks exist. Measure how priority inversion is avoided compared to a lock-based priority queue.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Work stealing | "Idle threads steal tasks from busy ones" | Load-balancing strategy: each worker has its own deque. Idle workers steal from the top of a random victim's deque (FIFO). Owner pushes/pops from the bottom (LIFO). |
| Chase-Lev deque | "Lock-free double-ended queue for work stealing" | A lock-free deque using atomic indices (top, bottom) with a circular buffer. Owner push/pop is CAS-free in the common case. Steal requires CAS on top. |
| Michael-Scott queue | "Lock-free FIFO queue" | First practical lock-free queue. Uses a dummy node so head and tail are never null. Requires CAS on tail->next (enqueue) and head (dequeue). |
| Tagged pointer | "Pointer + generation counter" | ABA protection: pack a monotonic tag into unused pointer bits. Even if a memory address is reused, the tag won't match. |
| Memory ordering | "Acquire/Release/SeqCst" | The guarantees that prevent reordering of memory operations across threads. Crucial for lock-free correctness: Release ensures prior stores are visible; Acquire ensures subsequent loads see latest; SeqCst creates a total order. |
| Victim selection | "Which worker to steal from" | Random uniform selection using XorShift64. Low overhead, good statistical properties. Alternative: round-robin (predictable but can target hot deques). |
| LIFO/FIFO split | "Owner pops LIFO, thieves steal FIFO" | Owner processes its own deque LIFO — good for cache locality (most recent task is hottest). Stealers take FIFO — oldest tasks, which are likeliest to have cold cache and least harmful to steal. |
| Thread-per-task | "One OS thread per task" | Naive baseline: spawn a thread for each task, join all. High overhead (OS thread creation/teardown), no reuse. Does not scale beyond a few hundred tasks. |
| Mutex pool | "Threads share a queue behind a mutex" | Simple thread pool: all tasks go into a single mutex-protected queue. Workers contend on the mutex. At high thread counts, contention dominates. |
| Task | "Unit of work" | `Box<dyn FnOnce() + Send>` — a heap-allocated closure that can be sent across threads and called exactly once. |

## Further Reading

1. **Blumofe & Leiserson, "Scheduling Multithreaded Computations by Work Stealing" (FOCS 1994)** — the paper that established work stealing as the dominant scheduler for multithreaded computation. Proves that work stealing achieves `T_P = O(T_1 / P + T_inf)` expected time.

2. **Chase & Lev, "Dynamic Circular Work-Stealing Deque" (SPAA 1994)** — the first Chase-Lev deque paper. Describes the circular buffer algorithm and proves correctness under release consistency.

3. **Herlihy & Shavit, "The Art of Multiprocessor Programming," 2nd ed., Chapters 9–11, 16–18** — lock-free data structures (Ch. 9–11) and work-stealing schedulers (Ch. 16–18). The ForkJoinPool chapter shows how to combine these into a complete runtime.

4. **Lea, "A Java Fork/Join Framework" (JavaOne 2000)** — the original design document for java.util.concurrent.ForkJoinPool. Describes work-stealing with help-thieving and adaptive parallelism.

5. **Michael & Scott, "Simple, Fast, and Practical Non-Blocking and Blocking Concurrent Queue Algorithms" (PODC 1996)** — the MS queue paper. Compares lock-free vs. lock-based queue implementations under various contention levels.

6. **Rust std::sync::atomic docs**: https://doc.rust-lang.org/std/sync/atomic/ — the standard library's atomic types and memory ordering.

7. **Rayon documentation**: https://docs.rs/rayon/latest/rayon/ — production work-stealing in Rust. The `join` and `scope` APIs are the primary entry points.

8. **Frigo, Leiserson, & Randall, "The Implementation of the Cilk-5 Multithreaded Language" (PLDI 1998)** — describes the Cilk work-stealing runtime, which introduced the "Cactus stack" and provably efficient scheduling.
