# Work-Stealing Schedulers

> Efficient load balancing for parallel computations with provably good
> performance: each worker maintains a private deque of tasks and steals from
> others only when idle.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 13 lessons 01–16 (especially atomics, CAS, locking,
and lock-free data structures)
**Time:** ~90 minutes

---

## Learning Objectives

After completing this lesson you will be able to:

1. Explain the difference between work-first and help-first scheduling.
2. Describe the Cilk scheduler: each worker owns a double-ended queue (deque),
   pushes/pops from the bottom (LIFO), and steals from the top (FIFO) of a
   random victim.
3. Implement a Chase-Lev lock-free work-stealing deque using atomic
   operations and CAS.
4. Build a minimal work-stealing thread pool and benchmark it against a
   conventional mutex-based pool.
5. State and interpret the work-stealing theorem:
   expected completion time = T₁/P + O(T∞), where T₁ is serial time and T∞
   is the critical-path length (span).
6. Recognize work-stealing in production: Tokio (async tasks), rayon
   (parallel iterators), and the Go scheduler (goroutines).

---

## The Problem

Parallel programs must divide work across hardware threads. The naive approach
is **static partitioning**: split the input into P chunks and assign each
chunk to a thread. This fails when chunks finish at different times (load
imbalance) or when work is generated dynamically (recursive fork-join,
divide-and-conquer, producer-consumer pipelines).

A **centralized work queue** solves load balancing: all workers grab the next
task from a single shared queue protected by a mutex. But the mutex becomes a
contention bottleneck as core count grows, and every worker pays the cache
miss cost of fetching remote cache lines.

*Without work-stealing, you cannot scale divide-and-conquer or recursive
parallel algorithms beyond a handful of cores without severe contention or
load imbalance.*

---

## The Concept

### Work-First vs. Help-First

| Strategy | Who executes spawned tasks? | When do other workers help? |
|----------|---------------------------|----------------------------|
| **Work-first** | The spawning thread executes the continuation; spawned tasks sit in the deque for thieves. | Thieves steal spawned tasks from idle workers. |
| **Help-first** | The spawning thread executes the spawned task; the continuation is enqueued for thieves. | Thieves steal continuations. |

Cilk (the seminal work-stealing runtime from MIT) uses **work-first**. The
key insight: a thread that just spawned a task is likely to have the
continuation hot in cache, so running the continuation locally improves
locality. Spawned-but-not-yet-executed tasks are fair game for thieves.

### The Cilk Scheduler

Each worker (OS thread pinned to a core) owns:

- A **deque** of tasks that are ready to execute.
- **`top`** index (read by thieves, written by CAS during steal).
- **`bottom`** index (written by owner during push/pop).

```
Worker A's deque:        Worker B's deque:
+-----------+            +-----------+
|  task 7   |  <-- top   |  task 3   |  <-- top
|  task 6   |            |  task 2   |
|  task 5   |            |  task 1   |
|  ...      |            |  ...      |
|  task 0   |  <-- bottom|           |  <-- bottom
+-----------+            +-----------+
```

**Owner operations:**
- `push(task)`: write at `buffer[bottom % mask]`; `bottom++`.
- `pop()`: `bottom--`; read `buffer[bottom]`. If `top == bottom` after
  decrement, race with stealers — must CAS `top` to resolve.

**Thief operations:**
- `steal()`: read `top`; if `top < bottom`, read
  `buffer[top % mask]` and CAS `top` forward.

The deque is **lock-free** for thieves: a thief retries its CAS if it
loses a race. The owner only synchronizes with CAS when it tries to pop
the *last* element while thieves may be stealing it.

### Chase-Lev Deque

The Chase-Lev deque (1994) is the canonical lock-free deque design. Key
properties:

- **Lock-free**: at least one thread always makes progress.
- **Bounded contention**: only the last-element pop and all steals use CAS;
  normal pushes and non-last pops use only atomic loads/stores.
- **Growable**: when the circular buffer is full, the owner allocates a new
  buffer twice the size and atomically swaps the pointer. Old buffers are
  never freed in the basic algorithm (leaked to prevent use-after-free with
  concurrent stealers).
- **ABA-safe** in practice via tagged pointers or leak-based reclamation.

### The Work-Stealing Theorem

For a multithreaded computation with work T₁ (execution time on one core)
and span T∞ (length of the critical path — the longest chain of dependent
tasks), the **expected** completion time on P cores with randomized
work-stealing is:

```
E[T_P] ≤ T₁ / P + O(T∞)
```

Interpretation:
- **Linear speedup** up to when T₁ / P dominates (coarse-grained parallelism).
- **Amdahl-limited** by T∞ when parallelism exceeds T₁ / T∞ (the
  "parallelism ceiling").
- The constant in O(T∞) is small (~2 in Cilk's analysis).
- Random victim selection with ≥ 1 steal attempt per cycle gives this
  bound in expectation.

### Where Work-Stealing Appears in Production

| System | Role | Key Difference |
|--------|------|----------------|
| **Cilk / Cilk Plus** | Original work-stealing runtime for C/C++. | Language-level spawn/sync. |
| **Tokio** | Async task scheduler for Rust. | Each worker has a local deque + a global injector queue for I/O-bound tasks. |
| **rayon** | Parallel iterator library for Rust. | Uses work-stealing for fork-join parallelism. |
| **Go scheduler** | M:N scheduling of goroutines. | Each P (logical processor) has a local run queue; idle P's steal from others. |
| **Java ForkJoinPool** | Work-stealing pool since Java 7. | Similar to Cilk; workers steal from "bottom" (actually top in Java's variant). |
| **.NET Task Parallel Library** | Managed task parallelism. | Each worker has a local work-stealing queue. |

---

## Build It

### Step 1: Chase-Lev Work-Stealing Deque

```rust
use std::sync::atomic::{AtomicIsize, AtomicPtr, Ordering, fence};
use std::mem::MaybeUninit;

struct Array<T> {
    log_cap: usize,
    data: Box<[MaybeUninit<T>]>,
}

impl<T> Array<T> {
    fn new(log_cap: usize) -> Self {
        let cap = 1 << log_cap;
        let mut data = Vec::with_capacity(cap);
        for _ in 0..cap { data.push(MaybeUninit::uninit()); }
        Array { log_cap, data: data.into_boxed_slice() }
    }

    fn cap(&self) -> isize { 1 << self.log_cap }
    fn mask(&self) -> isize { self.cap() - 1 }

    unsafe fn get(&self, i: isize) -> T {
        (self.data[(i & self.mask()) as usize].as_ptr()).read()
    }

    unsafe fn set(&self, i: isize, val: T) {
        self.data[(i & self.mask()) as usize].as_ptr().write(val);
    }
}

pub struct WorkDeque<T> {
    bottom: AtomicIsize,
    top: AtomicIsize,
    array: AtomicPtr<Array<T>>,
}

impl<T: Send> WorkDeque<T> {
    pub fn new() -> Self { /* ... */ }

    pub fn push(&self, task: T) {
        // 1. Load bottom relaxed.
        // 2. Write task to buffer[bottom & mask].
        // 3. If full, allocate larger buffer and swap.
        // 4. bottom.store(b + 1, Release).
    }

    pub fn pop(&self) -> Option<T> {
        // 1. bottom-- relaxed.
        // 2. fence(SeqCst).
        // 3. If top ≤ new_bottom: read task.
        //    - If top == new_bottom: CAS top to resolve race.
        //    - Else: return task.
        // 4. Else: restore bottom, return None.
    }

    pub fn steal(&self) -> Option<T> {
        // 1. top.load(Acquire), fence(SeqCst), bottom.load(Acquire).
        // 2. If top < bottom: read task, CAS top forward.
        // 3. Return task on success, None on failure.
    }
}
```

The push operation must be **linearizable** with respect to pop and steal:
after a push completes, a subsequent pop or steal will observe the new task.
This is guaranteed by the `Release` store on `bottom` paired with `Acquire`
loads on `bottom` in pop and steal.

The last-element pop uses a **compare-and-swap** (CAS) to decide between
owner and thieves:

```
if top == b (the only element):
    if CAS(top, b, b+1) succeeds → owner gets it
    else → a thief stole it, return None
```

### Step 2: Work-Stealing Thread Pool

A minimal work-stealing pool:
1. Create N `WorkDeque<Job>` instances (one per worker).
2. Distribute initial tasks round-robin into the deques.
3. Each worker loops:
   - Pop from its own deque (LIFO — cache friendly).
   - If empty, steal from a random victim's deque.
   - If still nothing and all tasks accounted for, exit.
4. The main thread joins all workers.

```rust
type Job = Box<dyn FnOnce() + Send>;

struct WsPool {
    deques: Arc<Vec<WorkDeque<Job>>>,
    handles: Vec<JoinHandle<()>>,
    done: Arc<AtomicUsize>,
    total: usize,
}

impl WsPool {
    fn new(num_threads: usize, tasks: Vec<Job>) -> Self {
        // allocate deques, distribute tasks, spawn workers
    }
    fn wait(self) {
        // join all worker threads
    }
}
```

**Random victim selection** is critical for the theoretical bound. A simple
XorShift PRNG per worker avoids contention on a shared RNG state.

### Step 3: Spawn and Steal

Once the pool runs, the flow for each task is:

```
Worker A's loop:
  ┌─ Pop A.deque ──→ Some(task) ──→ run task ──┐
  │                                            │
  └─ Empty? ──→ steal from random B ──→ Some──→┘
                             │
                             └─ None ──→ yield, check done flag
```

When a task spawns subtasks, those subtasks are pushed onto the *same*
worker's deque. Other workers discover them by stealing.

### Step 4: Benchmark

Compare three approaches on a compute-bound workload
(e.g., 64 × fib(42) on 4 cores):

| Approach | Expected Behavior |
|----------|-------------------|
| **thread per task** | Creates 64 OS threads — high overhead, thrashing. |
| **mutex-based pool** | Single shared queue — contention on the mutex, poor cache locality. |
| **work-stealing pool** | Each worker owns a deque — low contention, good locality. |

Expected relative performance:

```
thread-per-task  >>>  mutex-pool  >  work-stealing
   (slowest)                       (fastest)
```

Work-stealing wins because:
- The critical section is on a *per-worker* basis, not global.
- LIFO pops keep hot data in cache (worker just ran a task that created
  this one).
- Stealing only happens when a worker is truly idle.
- Stealing from the *top* (oldest tasks) minimizes migration cost — the
  oldest tasks are the least likely to have hot cache lines on the victim.

### Key Implementation Details

**Memory ordering** for correctness:

| Operation | Load/Store | Ordering |
|-----------|-----------|----------|
| `push`: read `bottom` | load | `Relaxed` |
| `push`: write task, then write `bottom` | store + store | `Release` (on bottom) |
| `pop`: write `bottom-1` | store | `Relaxed` then `fence(SeqCst)` |
| `pop`: read `top` after fence | load | `Acquire` |
| `pop`: CAS on `top` (last elem) | CAS | `SeqCst` |
| `steal`: read `top` | load | `Acquire` |
| `steal`: fence | fence | `SeqCst` |
| `steal`: read `bottom` | load | `Acquire` |
| `steal`: CAS on `top` | CAS | `SeqCst` |

The `SeqCst` fence in `pop` and `steal` pairs with the `Release` store on
`bottom` to ensure that updates to the array are visible before the index
changes.

**False sharing** occurs when `top` and `bottom` (or two adjacent deques)
land on the same cache line. In production, pad each deque to 64 or 128
bytes. We omit this for brevity.

---

## Use It

### Tokio

Tokio's async runtime uses a **work-stealing scheduler** with one key
difference from Cilk: it has a **global injector queue** (a lock-free MPMC
queue) for I/O-bound tasks that arrive from outside, plus per-worker deques
for spawned local tasks.

File: `tokio/src/runtime/scheduler/multi_thread/worker.rs`

When a worker's local deque is empty, it tries:
1. Steal from another worker's deque (half of it, not just one task).
2. Pop from the global injector queue.
3. Park (block) until new work arrives via I/O events.

### Rayon

Rayon uses a work-stealing pool for data parallelism (`par_iter`,
`par_bridge`, etc.). Unlike Cilk, rayon's tasks are typically data-parallel
iterations rather than recursive forks.

File: `rayon/src/registry.rs`

Key design decisions:
- **LIFO popping** for the worker's own tasks (good locality).
- **FIFO stealing** from victims (oldest tasks are largest/most
  steal-worthy).
- **Steal half** (a batch of tasks) to amortize steal overhead.
- **Lazy task splitting** (splitting an iterator range only when stolen).

### Go Scheduler

Go's M:N scheduler maps goroutines (G) onto logical processors (P) which
run on OS threads (M). Each P has a **local run queue** (a lock-free ring
buffer). When a P's queue is empty, it steals from another P.

File: `runtime/proc.go` — `stealWork` function

Go uses a **non-random** work-stealing order (it iterates sequentially,
starting from a random offset) and steals half the goroutines from the
victim's queue.

---

## Read the Source

- **Cilk / OpenCilk:** `cilk/opencilk/runtime/` — the original deque
  implementation (`cilk_fiber.cpp`, `cilk_deque.h`).
- **Tokio:** `tokio/src/runtime/scheduler/multi_thread/worker.rs` — the
  multi-thread scheduler (`fn steal_while`).
- **Rayon:** `rayon/src/deque.rs` — the Chase-Lev deque variant used by
  rayon's thread pool.
- **Go:** `runtime/proc.go` — the `stealWork` function and local run queue
  (`runqput`, `runqget`).
- **Chase & Lev's original paper:** "Dynamic Circular Work-Stealing Deque"
  (SPAA 1994) and "The Implementation of the Cilk-5 Multithreaded Language"
  (PLDI 1998).

---

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. It is:

- A **self-contained work-stealing deque and thread pool** (`main.rs`
  snippets) you can drop into future projects that need dynamic load
  balancing.
- A **benchmark harness** comparing work-stealing against mutex-based pools.
- A **correctness stress test** for lock-free deques.

---

## Exercises

### Easy

Reproduce the `WorkDeque` implementation from memory. Verify correctness
with the stress test. If you get stuck, re-read the push/pop/steal
pseudocode and the memory-ordering table above.

### Medium

Replace the round-robin task distribution in `WorkStealingPool` with a
single `Mutex<Vec<Job>>` global injector. Workers first pop from their own
deque, then try the global queue, then steal. Benchmark against both
existing implementations. Does the global injector help or hurt?

### Hard

Implement **steal-half**: instead of stealing one task, the thief steals
half the tasks from the victim's deque in a single atomic operation. In
Tokio and Go, this amortizes the cost of stealing and improves load
balance. Extend `WorkDeque::steal` to return a `Vec<T>` or use a
batch-steal protocol. Benchmark against single-task stealing.

---

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Work-stealing | Idle threads grab tasks from busy ones. | Each worker owns a deque; idle workers steal from the top of a random victim's deque. |
| Work-first | The spawning thread runs the continuation. | Spawned tasks go to the deque; the thread continues immediately. Maximizes cache locality. |
| Help-first | The spawning thread runs the child task. | The continuation goes to the deque; helps distribute work faster at the cost of locality. |
| Deque (double-ended queue) | A queue you can push/pop from both ends. | In Cilk: owner pushes/pops at bottom (LIFO); thieves steal from top (FIFO). |
| Chase-Lev deque | The standard lock-free deque for work-stealing. | Uses circular buffer + atomic indices; CAS only on last-element pop and all steals. |
| T₁ | Serial execution time. | Total work: sum of all task execution times on one core. |
| T∞ (span, critical path) | The longest chain of dependencies. | The minimum possible execution time with infinite cores. |
| T₁ / P + O(T∞) | Work-stealing theorem. | Randomized work-stealing achieves this expected completion time on P cores. |
| Locality | Keeping data close to the core that uses it. | LIFO popping reuses cached data from the just-completed task. |
| False sharing | Two cores contend on the same cache line. | Adjacent deques' atomic variables share a cache line. Pad to 64 B in production. |
| Random victim selection | Pick a worker uniformly at random. | Required for the O(T∞) term in the theorem. XorShift is sufficient. |

---

## Further Reading

1. Blumofe & Leiserson. *Scheduling Multithreaded Computations by Work
   Stealing*. FOCS 1994. — The original work-stealing theorem.
2. Frigo, Leiserson, & Randall. *The Implementation of the Cilk-5
   Multithreaded Language*. PLDI 1998. — How Cilk implements spawn/sync and
   the deque.
3. Chase & Lev. *Dynamic Circular Work-Stealing Deque*. SPAA 1994. — The
   lock-free deque algorithm.
4. Michael, Vechev, & Saraswat. *Idempotent Work Stealing*. PPoPP 2009. —
   Extensions for fault tolerance.
5. Acar, Charguéraud, & Rainey. *Scheduling Parallel Programs by Work
   Stealing with Private Deques*. PPoPP 2013. — Alternative designs with
   better memory footprints.
6. Tokio documentation: *How Tokio's work-stealing scheduler works* —
   https://tokio.rs/blog/2019-10-scheduler.
7. Go source: `runtime/proc.go` (the `schedule` and `stealWork` functions).
8. Rayon source: `rayon/src/deque.rs` and `rayon/src/registry.rs`.
