# Race Conditions, Atomicity, Visibility

> Race Conditions, Atomicity, Visibility — the part of CS you can't skip.

**Type:** Build
**Languages:** C, Rust
**Prerequisites:** Phase 13, Lesson 01 (Concurrency vs Parallelism)
**Time:** ~60 minutes

## Learning Objectives

- Define a data race and distinguish it from a generic race condition.
- Explain why `counter++` is not atomic and what that means for concurrent threads.
- Understand how CPU caches and compiler optimizations break visibility between threads.
- Fix race conditions using mutexes and C11 atomics (C) or `Arc<Mutex<>>` and atomics (Rust).
- Explain the happens-before relationship and why acquire/release semantics matter.
- Understand why Rust eliminates data races at compile time while C does not.

## The Problem

Imagine two threads each incrementing a shared counter 1,000,000 times. You expect 2,000,000. Instead you get 1,023,417. The next run gives 998,211. The third gives 1,456,003. The result changes every run and is never correct.

This is not bad luck. This is a **race condition** — the fundamental hazard of shared-memory concurrency. When multiple threads read and write the same memory without synchronization, the outcome depends on the exact interleaving of machine instructions. Since the operating system scheduler controls interleaving (preempting threads at arbitrary points), the behavior is non-deterministic.

Race conditions cause:
- **Corrupted data** (bank balances, game state, sensor readings)
- **Crashes** (segfaults from corrupted pointers)
- **Security vulnerabilities** (TOCTOU: time-of-check to time-of-use)
- **Heisenbugs** — bugs that disappear when you try to debug them (adding a `printf` changes timing and "fixes" the race)

The three pillars of shared-memory correctness — race freedom, atomicity, and visibility — form the foundation for everything else in concurrent programming: locks, lock-free data structures, async runtimes, and GPU programming.

## The Concept

Shared-memory concurrency has three independent failure modes. Understanding all three is necessary (and together sufficient) to write correct concurrent code.

### 1. Race Conditions (Data Races)

A **data race** occurs when:
- Two or more threads access the same memory location **concurrently**, and
- At least one access is a **write**, and
- There is no **synchronization** (locking, atomic operations, or barriers) to order them.

**The classic example — `counter++`:**
On x86-64, `counter++` compiles to three machine instructions:

```
LOAD   [counter] → register    // read from memory
ADD    register, 1              // increment in register
STORE  register → [counter]    // write back to memory
```

When two threads execute this concurrently:

```
Thread A: LOAD counter (0)               → regA = 0
Thread A: ADD regA, 1                    → regA = 1
            ← scheduler preempts A, runs B →
Thread B: LOAD counter (0)               → regB = 0  (A's store not visible yet!)
Thread B: ADD regB, 1                    → regB = 1
Thread B: STORE regB → counter           → counter = 1
            ← scheduler resumes A →
Thread A: STORE regA → counter           → counter = 1  (OVERWRITES B's work!)
```

Two increments, one net effect. One lost update. This is the **lost update problem**.

Not all race conditions are data races. A **check-then-act race** (also called a TOCTOU race) involves non-atomic check-and-act:

```c
if (account->balance >= amount) {   // check
    withdraw(account, amount);       // act — but balance may have changed!
}
```

Between the check and the act, another thread may have modified `balance`. The check is stale. This is a race condition but not technically a data race (the data structures involved may be properly synchronized individually — the race is in the *logic*, not the memory access).

### 2. Atomicity

An operation is **atomic** if it appears to execute **indivisibly** from the perspective of all other threads. Either the entire operation is visible, or none of it is. There is no intermediate state.

`counter++` is NOT atomic — it's three machine instructions. Atomic operations in modern hardware are limited to:
- **Load** (read a word atomically)
- **Store** (write a word atomically)
- **Read-modify-write** (RMW): atomic increment, compare-and-swap (CAS), fetch-and-add, test-and-set

C11 provides `_Atomic` types and a set of atomic operations:

```c
_Atomic int counter = 0;
counter++;  // now compiles to lock xadd or CAS loop — atomic!
```

The hardware implements atomicity through:
- **Cache-coherence protocols** (MESI/MOESI) that ensure atomic reads/writes to aligned words
- **Locked instructions** (`LOCK` prefix on x86) that grab the bus/memory controller lock
- **Load-link/store-conditional** (LL/SC) pairs on ARM/Power that detect concurrent modifications

**Compare-and-swap (CAS):**

```c
bool atomic_compare_exchange_strong(_Atomic int* obj, int* expected, int desired);
```

CAS atomically: if `*obj == expected`, set `*obj = desired` and return true; otherwise set `*expected = *obj` and return false. CAS is the foundation of lock-free data structures.

**The ABA Problem:** CAS can be fooled if a value changes from A to B and back to A between the load and the CAS. The CAS succeeds even though the memory has been modified. Solutions: use a version tag or hazard pointers.

### 3. Visibility

**Visibility** is the guarantee that a write by one thread becomes visible to reads by other threads. Without it, a thread might write a value that another thread **never sees**, even "after" the write in program order.

Why writes aren't immediately visible to all threads:

**CPU caches:** Each core has its own L1/L2 cache. When core 0 writes to address X, the new value sits in core 0's L1 cache. Core 1, reading from its own L1 cache, still sees the old value. Cache-coherence protocols (MESI) eventually propagate the write, but "eventually" is not "before the consumer thread runs."

**Compiler optimizations:** The compiler may:
- Cache a variable in a register across a loop (never re-reading from memory)
- Reorder independent operations for better pipelining
- Eliminate writes that seem unused (a write that another thread depends on is "unused" from the single-threaded compiler's perspective)

**CPU instruction reordering:** Modern CPUs execute instructions out of order. A store to `ready` might complete before a store to `data` even though the source code says `ready = 1` comes after `data = 42`.

The ordering rules vary by architecture:

| Architecture | Model | Strength |
|---|---|---|
| x86/amd64 | TSO (Total Store Order) | Strong — only stores can be reordered with later loads |
| ARMv8 | Weakly ordered | Full reordering; requires explicit barriers |
| RISC-V | Weakly ordered | Full reordering; requires explicit fences |
| PowerPC | Weakly ordered | Even weaker than ARM; complex memory model |

This architecture dependence is why "it works on my machine" is a dangerous assumption for concurrent code.

**The happens-before relationship:** A write to variable X **happens-before** a read of X if:
1. They are in the same thread and the write precedes the read in program order, OR
2. The write is in a synchronized-with relationship with the read (e.g., thread A unlocks a mutex, thread B locks the same mutex), OR
3. The transitive closure of (1) and (2).

If X happens-before Y, then X's effects are guaranteed to be visible to Y. Without happens-before, there is no guarantee.

**Memory barriers (fences):**
- `atomic_thread_fence(memory_order_acquire)`: prevents reordering of later reads before this fence
- `atomic_thread_fence(memory_order_release)`: prevents reordering of earlier writes after this fence
- `atomic_thread_fence(memory_order_seq_cst)`: full barrier

**Memory ordering modes for atomics:**

| Ordering | What it guarantees | Cost |
|---|---|---|
| `relaxed` | Atomicity only. No ordering constraints. | Free (on x86, same as regular load/store) |
| `acquire` | Reads after this point see all writes from the releasing thread | One-time cost on weak architectures |
| `release` | All writes before this point are visible to the acquiring thread | One-time cost on weak architectures |
| `acq_rel` | Both acquire and release (for RMW operations) | One-time cost |
| `seq_cst` | Global sequential consistency. All threads agree on the order of all seq_cst operations. | Full barrier on all architectures |

### Putting It Together: Double-Checked Locking

A classic example that demonstrates all three concepts. This looks correct but is broken without proper atomics:

```c
// BROKEN — data race + visibility issue
static int* instance = NULL;
pthread_mutex_t lock;

int* get_instance(void) {
    if (instance == NULL) {         // check (read)
        pthread_mutex_lock(&lock);
        if (instance == NULL) {     // double check
            instance = malloc(sizeof(int));  // write
            *instance = 42;
        }
        pthread_mutex_unlock(&lock);
    }
    return instance;
}
```

The problem: the read `instance == NULL` on line 4 is outside the lock. Thread A creates `instance` and writes `*instance = 42`. Due to CPU reordering or cache effects, Thread B sees `instance != NULL` but reads `*instance == 0`. Or worse, `instance` is a partially-constructed object.

The fix: make `instance` an atomic pointer with acquire/release ordering on the read and write.

## Build It

You'll implement three demos in C and their equivalents in Rust:

1. **Counter race** — demonstrate lost updates
2. **Visibility demo** — demonstrate broken flag-based communication
3. **Rust equivalents** — see how Rust's type system prevents data races

### Step 1: Race Condition Demo (C)

Open `code/main.c`. The first demo creates two threads, each incrementing a shared `int counter = 0` one million times.

**The broken version:**
```c
int counter = 0;  // shared — no synchronization

void* increment(void* arg) {
    for (int i = 0; i < 1000000; i++) counter++;  // RACE!
    return NULL;
}
```

Compile and run:
```bash
gcc -O2 -pthread -o race main.c && ./race
```

Expected output:
```
=== Demo 1: Counter Race ===
  Expected: 2000000
  Actual:   1428317
  (Lost updates due to race condition)
```

The actual number will differ every run. On some runs it might be close to 2,000,000 (if threads happen to not interleave much), but it is never guaranteed to be correct.

**Key insight:** This is a data race. `counter++` compiles to three instructions, and the threads interleave at arbitrary points. The compiler optimization (-O2) has no effect on the fix — the race exists at any optimization level because the problem is in the instruction interleaving, not in the compiler's behavior.

**Adding `volatile` doesn't fix it:**
```c
volatile int counter = 0;
```
`volatile` prevents the compiler from caching the value in a register, but it does NOT make `counter++` atomic. The three instructions (LOAD, ADD, STORE) can still be interleaved.

### Step 2: Fix with Mutex and Atomics

**Fix with mutex:**
```c
pthread_mutex_t lock;
int counter_mutex = 0;

void* increment_mutex(void* arg) {
    for (int i = 0; i < 1000000; i++) {
        pthread_mutex_lock(&lock);
        counter_mutex++;
        pthread_mutex_unlock(&lock);
    }
    return NULL;
}
```

This always gives 2,000,000. The mutex creates mutual exclusion — only one thread executes `counter_mutex++` at a time. It also establishes happens-before: the unlock in thread A synchronizes-with the lock in thread B.

Cost: Each increment involves two system calls (lock + unlock). The overhead is enormous (~200 ns per operation vs ~1 ns for a plain increment).

**Fix with C11 atomics:**
```c
#include <stdatomic.h>

_Atomic int counter_atomic = 0;

void* increment_atomic(void* arg) {
    for (int i = 0; i < 1000000; i++) counter_atomic++;
    return NULL;
}
```

`counter_atomic++` compiles to a single atomic RMW instruction (`lock xadd` on x86). No system calls. No context switches. Much faster than a mutex but not as fast as a plain increment (the `LOCK` prefix adds ~50-100 ns per operation and serializes the memory bus).

### Step 3: Visibility Demo (C)

The second problem: thread A writes `data = 42; ready = 1;` and thread B spins on `ready` then reads `data`.

**The broken version (in `code/main.c`, Demo 4):**
```c
int data = 0;
int ready = 0;  // no volatile, no atomic

void* producer(void* arg) {
    data = 42;
    ready = 1;   // can be reordered before data=42!
    return NULL;
}

void* consumer(void* arg) {
    while (ready == 0);  // compiler may cache ready in register → never exits
    printf("data = %d\n", data);  // may print 0 on weak arch
    return NULL;
}
```

What goes wrong:
1. **Compiler hoisting**: With `-O2`, the compiler may load `ready` into a register once, transforming the loop into `if (!ready) while(1);` — an infinite loop.
2. **CPU reordering**: On ARM/Power, the CPU may make `ready=1` visible before `data=42`. Thread B sees `ready==1` but reads `data==0`.
3. **Cache staleness**: Even without reordering, thread B may read stale values from its L1 cache.

**The fix with atomics:**
```c
_Atomic int ready_atomic = ATOMIC_VAR_INIT(0);

void* producer_fixed(void* arg) {
    data_fixed = 42;
    atomic_store_explicit(&ready_atomic, 1, memory_order_release);
    return NULL;
}

void* consumer_fixed(void* arg) {
    while (atomic_load_explicit(&ready_atomic, 0, memory_order_acquire) == 0);
    printf("data = %d\n", data_fixed);
    return NULL;
}
```

The `release` store and `acquire` load establish a happens-before relationship:
- All writes before `ready_atomic.store(1, release)` are visible to all reads after `ready_atomic.load(acquire)` returns 1.
- This means `data_fixed = 42` (which happens-before the release) is guaranteed visible to `printf("...%d", data_fixed)` (which happens-after the acquire).

### Step 4: Rust Version

Open `code/main.rs`. Rust takes a fundamentally different approach to data races: **the type system prevents them at compile time.**

**Attempting a data race in Rust:**
```rust
let counter = 0;
thread::spawn(move || {
    counter += 1;  // COMPILE ERROR: cannot mutate captured variable
});
```

Rust's ownership rules prevent sharing mutable state without synchronization. A variable can have either one mutable reference (`&mut`) or multiple immutable references (`&`), but not both. Since `thread::spawn` requires the closure to own its captured data (via `move`), you cannot accidentally share a mutable integer between threads.

**To share mutable state, you must explicitly choose a synchronization primitive:**

```rust
use std::sync::{Arc, Mutex};

let counter = Arc::new(Mutex::new(0));
let mut handles = vec![];

for _ in 0..2 {
    let c = Arc::clone(&counter);
    handles.push(thread::spawn(move || {
        for _ in 0..1_000_000 {
            *c.lock().unwrap() += 1;
        }
    }));
}
for h in handles { h.join().unwrap(); }
println!("Counter: {}", *counter.lock().unwrap());  // Always 2,000,000
```

`Arc` provides thread-safe reference counting. `Mutex` provides mutual exclusion. The type system ensures you can't forget the synchronization — the code won't compile without it.

**Atomic version in Rust:**
```rust
use std::sync::atomic::{AtomicUsize, Ordering};

let counter = Arc::new(AtomicUsize::new(0));
let c = Arc::clone(&counter);
thread::spawn(move || {
    for _ in 0..1_000_000 {
        c.fetch_add(1, Ordering::Relaxed);
    }
});
```

`fetch_add` compiles to the same `lock xadd` instruction as C's `_Atomic int` increment. The `Relaxed` ordering provides atomicity (no lost updates for the counter itself), but does not provide visibility guarantees for other memory operations.

**Correct visibility with acquire/release:**
```rust
let ready = Arc::new(AtomicBool::new(false));
let data = Arc::new(AtomicUsize::new(0));

let r = Arc::clone(&ready);
let d = Arc::clone(&data);
let producer = thread::spawn(move || {
    d.store(42, Ordering::Relaxed);
    r.store(true, Ordering::Release);  // Release barrier
});

let r = Arc::clone(&ready);
let d = Arc::clone(&data);
let consumer = thread::spawn(move || {
    while !r.load(Ordering::Acquire) {}  // Acquire barrier
    assert_eq!(d.load(Ordering::Relaxed), 42);  // guaranteed!
});
```

**Rust's compile-time advantage:**
```rust
// This code DOES NOT COMPILE:
let mut data = 0;
let ptr = &data;
thread::spawn(move || {
    data += 1;       // error: can't mutate captured variable
    *ptr += 1;       // error: can't dereference immutable reference
});
```

In C, the compiler lets you write a data race with no errors or warnings. The bug manifests at runtime, possibly hours later, possibly only on a different CPU architecture. In Rust, the same bug is a **compile error** caught in milliseconds. This is the strongest data race prevention mechanism in any mainstream systems language.

## Use It

Race conditions are not academic. They are responsible for some of the most famous (and expensive) software bugs in history.

**MySQL replication race (CVE-2016-6662):** A race condition in MySQL's replication handling allowed an attacker with SQL access to execute arbitrary code as the `mysql` user. The race was between a privilege check and an action: between the `SELECT` that verified permissions and the `CREATE FUNCTION` that loaded a shared library, the permissions could be changed.

**Apple's goto fail:** While not a concurrency bug, the infamous `goto fail; goto fail;` SSL vulnerability shows how a single duplicated line in C creates a security hole. In the concurrent context, the same class of bugs (check-then-act races) is amplified because the window between check and act can be widened by thread scheduling.

**ThreadSanitizer (TSan):** This is the production tool for detecting data races. Integrated into LLVM/GCC via `-fsanitize=thread`, TSan instruments every memory access at compile time and detects races at runtime:

```bash
gcc -fsanitize=thread -O1 -g -pthread -o race main.c && ./race
```

TSan reports the exact line of the conflicting accesses and the stack trace of both threads. It is the first tool to reach for when debugging concurrent code.

**The Linux kernel's approach:** The kernel uses `ACCESS_ONCE()` (now `READ_ONCE()`/`WRITE_ONCE()`) to mark shared variable accesses that must not be optimized, paired with explicit memory barriers (`smp_mb()`, `smp_rmb()`, `smp_wmb()`). See `include/asm-generic/barrier.h`.

## Read the Source

- **Linux kernel:** `include/asm-generic/barrier.h` — the full set of memory barrier macros used in the kernel. Each macro has a comment explaining the ordering guarantees.
- **C11 standard, §7.17 (Atomics):** The formal specification of atomic operations, memory ordering, and fences. Readable online at the WG14 draft.
- **Rust `std::sync::atomic` documentation:** https://doc.rust-lang.org/std/sync/atomic/index.html — explains each ordering variant with examples.
- **"Memory Barriers: a Hardware View for Software Hackers"** by Paul E. McKenney (Linux kernel developer). The definitive article on how CPU memory models work.
- **ThreadSanitizer algorithm:** https://github.com/google/sanitizers/wiki/ThreadSanitizerAlgorithm — how TSan detects races using happens-before analysis.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A race condition detection and prevention demo suite** — run the compiled binary to see lost updates, then fix them with mutexes and atomics. The demo suite includes both C and Rust implementations, making it useful as a reference for later phases.

## Exercises

1. **Easy** — Run `code/main.c` with `-O0` and `-O2`. Observe the counter values. Then run with `-fsanitize=thread` and read TSan's output. What lines does it flag?

2. **Medium** — Modify the C counter demo to use 4 threads instead of 2. How does the number of lost updates change? (Hint: more threads = more interleaving = more lost updates.) Now fix it with a `double` instead of `int`. Is the double version race-free?

3. **Hard** — Implement a "check-then-act" race: two threads check `array[index] == EMPTY` and then write `array[index] = value`. Without synchronization, both threads may write to the same slot. Fix with CAS (`atomic_compare_exchange_strong`) in C, or using `AtomicPtr::compare_exchange` in Rust.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Race condition | "Stuff crashing when threads run" | Any bug where the outcome depends on non-deterministic thread interleaving. Broader than data race. |
| Data race | "Two threads touching the same memory" | Two concurrent accesses to the same memory, at least one write, no synchronization. UB in C/C++. |
| Atomicity | "It happens all at once" | An operation appears indivisible to other threads. No thread sees a partial result. |
| Visibility | "Thread B can see what Thread A wrote" | A write is guaranteed to become visible to other threads within a bounded time (or immediately if happens-before). |
| Happens-before | "This happened before that" | A partial order on operations. If A happens-before B, A's effects are visible to B. |
| Memory barrier | "Fence" | A CPU instruction that prevents reordering of memory operations across the barrier. |
| CAS | "Compare and swap" | Atomic RMW: if `*p == expected`, set `*p = desired` and return true. Foundation of lock-free data structures. |
| ABA problem | "A changed to B and back to A" | CAS succeeds even though the value was modified in between. Solvable with version tags. |
| Lost update | "The increment that disappeared" | One thread's write is overwritten by another thread's stale write. The classic `counter++` bug. |
| TOCTOU | "Time of check, time of use" | A value changes between the check and the action. Classic security race pattern. |
| ThreadSanitizer | "TSan" | Compiler instrumentation that detects data races at runtime by tracking happens-before edges. |

## Further Reading

1. **"Computer Architecture: A Quantitative Approach"** by Hennessy & Patterson, 6th ed., Chapter 5 (Memory Hierarchy and Coherence). Explains MESI cache coherence and the hardware mechanisms for atomicity.

2. **C++ Standard, §32 (Atomics) and §6.9.2 (Memory Model):** The formal definition of the C++ memory model. C and C++ share the same memory model.

3. **"Preshing on Programming" blog series on memory ordering:** https://preshing.com/archives/ — The clearest practical explanations of acquire/release semantics, memory barriers, and lock-free programming.

4. **Rust's `std::sync` module source:** https://github.com/rust-lang/rust/tree/master/library/std/src/sync — Read the implementation of `Mutex`, `AtomicUsize`, and the barrier primitives.

5. **"The Art of Multiprocessor Programming"** by Herlihy & Shavit, 2nd ed. Chapters 3-5 cover shared-memory correctness, atomicity, and the foundations of concurrent data structures.
