# Memory Models — Sequential Consistency vs Relaxed

> The contract between your code, the compiler, and the CPU — without it, concurrent programs are unpredictable.

**Type:** Build
**Languages:** C++, Rust
**Prerequisites:** Phase 13 lessons 01–02 (concurrency vs parallelism, race conditions and atomicity)
**Time:** ~90 minutes

## Learning Objectives

- Distinguish sequential consistency, TSO, and relaxed memory models
- Explain what reorderings x86 and ARM permit and forbid
- Use C++ `std::memory_order` and Rust `Ordering` correctly
- Demonstrate that relaxed atomics can produce counterintuitive results
- Implement correct message passing with acquire/release semantics

## The Problem

You write a simple concurrent program: thread A sets `data = 42` then flips a `ready` flag; thread B spins on `ready` then reads `data`. With plain integers and no synchronization, thread B can see `ready == true` but `data == 0`. The CPU and compiler reordered the writes.

This is not a bug in your code — it is a bug in *your mental model* of how memory works. Without a *memory model* that defines which reorderings are allowed and which are forbidden, every concurrent program is at the mercy of whatever the hardware and compiler decide to do.

Modern CPUs are aggressively out-of-order. Compilers are even more aggressive. The x86 can buffer stores and reorder a store with a later load to a different address. ARM and Power can reorder almost anything. If you write lock-free code without understanding the memory model, you are writing a race condition and calling it an algorithm.

This lesson builds the mental model you need to reason about concurrent memory accesses correctly. The phase capstone (a work-stealing scheduler with a lock-free queue) depends on this understanding — without it, you will write a queue that works on x86 and silently corrupts data on ARM.

## The Concept

A **memory model** defines what values a read can return in a concurrent program. It sits between the language/architecture specification and the hardware implementation.

### Three Levels of Memory Model

**1. Sequential Consistency (SC)**

The "textbook" model. All operations from all threads appear to execute in some global sequential order, and that order is consistent with the program order on each thread. Every read sees the most recent write to that location in the global order.

```
Thread 1:  x.store(1) ──→ y.load()
Thread 2:  y.store(1) ──→ x.load()
```

Under SC, at least one thread must see the other's write. The result `(0, 0)` — both threads see the old values — is forbidden because there is no sequential order that produces it.

No real CPU implements full SC. It prevents too many optimizations.

**2. x86 Total Store Order (TSO)**

x86 is stronger than most architectures but weaker than SC:

- **Store → Load reordering allowed**: A store can be reordered with a later load to a *different* address. The store buffer writes to cache but the load can bypass it.
- **Store → Store**: Never reordered (stores are observed in program order).
- **Load → Load**: Never reordered.
- **Load → Store**: Never reordered.

TSO is equivalent to SC plus a FIFO store buffer per core. Writes become visible when the store drains to cache.

```
x86 allowed:  store X; load Y   →   load Y; store X  (different addresses)
x86 forbidden: load X; load Y   →   load Y; load X
```

**3. ARM/Power Relaxed Model**

Much weaker than x86. Almost any reordering is permitted:

| Reordering | x86 | ARM/Power |
|-----------|-----|-----------|
| Load→Load | No | Yes |
| Load→Store | No | Yes |
| Store→Load | Yes | Yes |
| Store→Store | No | Yes |

ARM and Power preserve address dependencies (if `p` points to `x`, loading `*p` after loading `p` is not reordered with the load of `p`), but beyond that, hardware can reorder freely. This gives the CPU maximum freedom for optimization.

### Language-Level Memory Models

C++11 and C11 introduced a formal memory model with six ordering levels. Rust inherited the same model via LLVM.

| Order | C++ Name | Rust Name | Meaning |
|-------|----------|-----------|---------|
| 0 | `memory_order_relaxed` | `Ordering::Relaxed` | No ordering constraints. Fastest, most dangerous |
| 1 | `memory_order_consume` | `Ordering::Consume` | Data-dependent ordering (deprecated, avoid) |
| 2 | `memory_order_acquire` | `Ordering::Acquire` | Subsequent loads/stores cannot move before this |
| 3 | `memory_order_release` | `Ordering::Release` | Prior loads/stores cannot move after this |
| 4 | `memory_order_acq_rel` | `Ordering::AcqRel` | Acquire + release for read-modify-write ops |
| 5 | `memory_order_seq_cst` | `Ordering::SeqCst` | Sequential consistency (strongest, default) |

**Acquire/Release pairing** is the workhorse of correct concurrent code:

```
Thread A (producer):       Thread B (consumer):
data = 42;                 while (!ready.load(acquire));
ready.store(true, release); assert(data == 42);
```

The release in thread A prevents the write to `data` from moving after `ready.store`. The acquire in thread B prevents the read of `data` from moving before `ready.load`. Together, they establish a *happens-before* relationship.

### Litmus Tests

Three canonical patterns test your understanding of memory models:

**Dekker's Algorithm**: Two threads each write a flag then read the other's flag. Under SC, mutual exclusion works. Under TSO, it needs barriers.

```
Thread 1:  flag1 = 1;  if (flag2 == 0) → critical section
Thread 2:  flag2 = 1;  if (flag1 == 0) → critical section
```

Without barriers, both threads can enter the critical section on x86.

**IRIW (Independent Reads of Independent Writes)**: Two threads each write to their own variable. Two reader threads observe the writes. Under ARM/Power, the readers can disagree about the order of the writes — one reader sees `(x=1, y=0)` while the other sees `(x=0, y=1)`. This violates sequential consistency but is allowed on relaxed hardware.

**Message Passing**: The producer/consumer pattern above. Under relaxed ordering, the assertion can fail. Under acquire/release, it always passes.

### Hardware Memory Barriers

To force ordering, CPUs provide barrier instructions:

| CPU | Full Barrier | Store Barrier | Load Barrier |
|-----|-------------|---------------|--------------|
| x86 | `mfence` | `sfence` | `lfence` |
| ARM | `dmb` (data memory barrier) | `dmb st` | `dmb ld` |
| ARM | `dsb` (data sync barrier) | `dsb st` | `dsb ld` |
| GCC | `__sync_synchronize()` | — | — |

C++ `std::atomic_thread_fence(std::memory_order_seq_cst)` generates a full barrier on all platforms.

## Build It

We will build a memory model litmus test suite in three steps, implemented in both C++ and Rust.

### Step 1: Sequential Consistency Demo (C++)

This program demonstrates Dekker's pattern under `seq_cst` ordering. With sequential consistency, the outcome `(r1=0, r2=0)` — both threads see the other's flag as not set — is impossible.

```cpp
// From code/main.cpp — Part 1
// Dekker pattern with seq_cst: (r1=0, r2=0) never occurs
#include <atomic>
#include <thread>
#include <iostream>
#include <cassert>

std::atomic<int> x{0}, y{0};
int r1{0}, r2{0};

void thread1() {
    x.store(1, std::memory_order_seq_cst);
    r1 = y.load(std::memory_order_seq_cst);
}

void thread2() {
    y.store(1, std::memory_order_seq_cst);
    r2 = x.load(std::memory_order_seq_cst);
}

int main() {
    constexpr int ITERATIONS = 100000;
    int outcomes[4] = {0};

    for (int i = 0; i < ITERATIONS; ++i) {
        x.store(0, std::memory_order_seq_cst);
        y.store(0, std::memory_order_seq_cst);
        r1 = r2 = 0;

        std::thread t1(thread1);
        std::thread t2(thread2);
        t1.join();
        t2.join();

        outcomes[r1 * 2 + r2]++;
    }

    std::cout << "SC Dekker outcomes (" << ITERATIONS << " runs):\n";
    std::cout << "(0,0): " << outcomes[0] << "  (should be 0)\n";
    std::cout << "(0,1): " << outcomes[1] << "\n";
    std::cout << "(1,0): " << outcomes[2] << "\n";
    std::cout << "(1,1): " << outcomes[3] << "\n";
    assert(outcomes[0] == 0 && "(0,0) is impossible under SC");
    return 0;
}
```

Key insight: Under `seq_cst`, the total order of all atomic operations guarantees that at least one thread's store is visible to the other thread's load. The outcome `(0,0)` is forbidden because no global sequential order can produce it.

### Step 2: Relaxed Atomics — When It Breaks (C++)

Now change `seq_cst` to `relaxed`. The compiler and CPU can reorder freely. The result `(0,0)` becomes possible, and more surprisingly, the outcome `(1,0)` — thread 2 sees `b=1` before `a=1` even though thread 1 writes `a` first — occurs regularly.

```cpp
// From code/main.cpp — Part 2
// Relaxed atomics: all 4 outcomes possible
std::atomic<int> a{0}, b{0};
int observed[4] = {0};

void relaxed_writer() {
    a.store(1, std::memory_order_relaxed);
    b.store(1, std::memory_order_relaxed);
}

void relaxed_reader() {
    int i = b.load(std::memory_order_relaxed);
    int j = a.load(std::memory_order_relaxed);
    observed[i * 2 + j]++;
}

int main() {
    // Run many iterations, accumulating observed outcomes
    for (int iter = 0; iter < 100000; ++iter) {
        a.store(0, std::memory_order_relaxed);
        b.store(0, std::memory_order_relaxed);

        std::thread w(relaxed_writer);
        std::thread r(relaxed_reader);
        w.join();
        r.join();
    }

    std::cout << "Relaxed outcomes:\n";
    std::cout << "(a=0,b=0): " << observed[0] << "\n";
    std::cout << "(a=0,b=1): " << observed[1] << "\n";
    std::cout << "(a=1,b=0): " << observed[2]
              << "  (reordered! b=1 seen before a=1)\n";
    std::cout << "(a=1,b=1): " << observed[3] << "\n";
    return 0;
}
```

The outcome `(1,0)` means the reader saw `b=1` but `a=0`. This is the store→store reordering in action: thread 1's store to `b` became visible before its store to `a`, even though the source code writes `a` first.

### Step 3: C++ vs Rust — Same Model, Different Syntax (Rust)

Rust's atomics map directly to C++ atomics via LLVM. The syntax differs but the semantics are identical.

```rust
// From code/main.rs — Part 1: SeqCst Dekker
use std::sync::atomic::{AtomicIsize, Ordering};
use std::thread;

fn main() {
    let x = AtomicIsize::new(0);
    let y = AtomicIsize::new(0);
    let r1 = AtomicIsize::new(0);
    let r2 = AtomicIsize::new(0);

    let t1 = thread::spawn(|| {
        x.store(1, Ordering::SeqCst);
        r1.store(y.load(Ordering::SeqCst), Ordering::SeqCst);
    });

    let t2 = thread::spawn(|| {
        y.store(1, Ordering::SeqCst);
        r2.store(x.load(Ordering::SeqCst), Ordering::SeqCst);
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!("r1={}, r2={}", r1.load(Ordering::SeqCst), r2.load(Ordering::SeqCst));
}
```

Rust's message passing with acquire/release:

```rust
// From code/main.rs — Part 2: Message Passing
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::sync::Arc;

fn main() {
    let ready = Arc::new(AtomicBool::new(false));
    let ready_clone = ready.clone();
    let data = Arc::new(AtomicUsize::new(0));
    let data_clone = data.clone();

    let producer = thread::spawn(move || {
        data_clone.store(42, Ordering::Relaxed);
        ready_clone.store(true, Ordering::Release);
    });

    let consumer = thread::spawn(move || {
        while !ready.load(Ordering::Acquire) {}
        let val = data.load(Ordering::Relaxed);
        assert_eq!(val, 42, "release/acquire guarantees visibility");
        println!("Consumer got: {}", val);
    });

    producer.join().unwrap();
    consumer.join().unwrap();
}
```

Rust's type system prevents data races on plain shared state — you cannot write `data = 42` from one thread and read it from another without `unsafe` or atomics. But atomics give you explicit control over *ordering*, which is what this lesson is about.

## Use It

Real-world memory model usage appears throughout systems programming:

**Linux kernel**: Uses `smp_mb()` (full memory barrier, expands to `mfence`/`dmb` depending on architecture) extensively in lock-free data structures. The `seqlock` uses `smp_wmb()` (write barrier) between sequence counter increments and data updates. The RCU (Read-Copy-Update) mechanism depends on memory barriers between publishing pointers and reading them.

**ConcurrencyKit** (<https://concurrencykit.org/>): A C library that provides portable memory barriers. A barrier macro like `ck_pr_fence_store()` expands to `sfence` on x86 and `dmb st` on ARM, giving the programmer a single API regardless of target.

**Crossbeam (Rust)**: Epoch-based reclamation uses careful ordering to coordinate garbage collection across threads. `crossbeam::atomic::AtomicCell` wraps C++ atomics with a Rust-friendly interface.

**Lock-free queues**: Most require at least acquire/release semantics. A Treiber stack (lock-free LIFO) uses `CAS` with `acq_rel` to ensure the pointer being swapped is visible to the thread that reads it next.

**Production comparison**: Your litmus test suite is simplified but correct. Production tools like `herdtools` (<https://github.com/herd/herdtools>) run thousands of litmus tests to validate hardware behavior. The Linux kernel's `tools/memory-model/` directory contains formalized memory model rules checked with the herd7 simulator.

## Read the Source

- **C++ standard [atomics.order]**: The formal definition of all six memory orders. Available in the C++ working draft at <https://eel.is/c++draft/atomics.order>.
- **"Memory Barriers: a Hardware View for Software Hackers"** — Paul E. McKenney: The definitive introduction to why CPUs need memory barriers and what they do. Published by Linux Weekly News.
- **"A Tutorial Introduction to the ARM and POWER Relaxed Memory Models"** — Sewell et al.: The formal specification of ARM/Power memory models, readable for practitioners.
- **Rust Nomicon — Atomics and Memory Ordering**: <https://doc.rust-lang.org/nomicon/atomics.html> — Concise explanation of Rust's atomic types and ordering guarantees.
- **Intel SDM Volume 3, Chapter 8**: "Multiple-Processor Management" — Documents the x86 memory ordering model (TSO) and the effect of each fence instruction.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A memory model litmus test suite** — compile and run `code/main.cpp` (C++17) or `code/main.rs` (Rust 2021) to observe hardware memory ordering behavior on your machine. Use this in later phases to verify that your lock-free data structures are correct on the target architecture.

```bash
cd code
g++ -O2 -pthread -std=c++20 -o memmodel main.cpp && ./memmodel
rustc main.rs -O && ./main
```

## Exercises

1. **Easy** — Run the SC Dekker demo. Replace `seq_cst` with `relaxed`. Does `(0,0)` now occur on your machine? Run at least 10,000 iterations.

2. **Medium** — Implement a Peterson lock (mutual exclusion for 2 threads using two flags and a turn variable) with `memory_order_seq_cst`. Then convert it to use `memory_order_acquire`/`release` only. Prove that your conversion is correct using the happens-before relation.

3. **Hard** — Write a C++ program that detects whether it is running on an x86 or ARM machine by running a relaxed litmus test. On x86, `(a=1,b=0)` in the writer/reader pattern should be rare or absent due to TSO's store→store ordering. On ARM, it should appear regularly. Verify your detection program on both architectures.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Sequential consistency | "All threads see the same order" | Every execution has a global total order consistent with each thread's program order |
| Total Store Order | "x86's memory model" | Stores are visible in order, but store→load reordering is allowed via the store buffer |
| Relaxed memory model | "Anything goes" | The CPU/compiler may reorder almost any memory operations; only address dependencies are preserved |
| Acquire semantics | "No moving reads before this" | Every subsequent memory operation in this thread must happen after this load |
| Release semantics | "No moving writes after this" | Every prior memory operation in this thread must happen before this store |
| Acquire-release | "Synchronizes-with" | A pair of acquire/release operations establish a happens-before relationship between threads |
| Memory barrier (fence) | "A CPU instruction that orders memory" | An explicit instruction (`mfence`, `dmb`) that prevents specific types of reordering |
| Reordering | "The CPU changed the order of my operations" | The processor or compiler executed memory operations in a different order than the source code specifies |
| Litmus test | "A small program that tests memory ordering" | A minimal concurrent program designed to produce a specific outcome if the hardware performs a certain reordering |
| Dekker's algorithm | "A mutual exclusion protocol" | Two-thread mutual exclusion using only two boolean flags; requires memory barriers to work correctly |
| IRIW | "Independent Reads of Independent Writes" | A litmus test that checks whether two readers can disagree about the order of two independent writes |
| Happens-before | "A happens before B" | The partial order that defines which writes are visible to which reads in a concurrent program |

## Further Reading

1. [C++ reference: `std::memory_order`](https://en.cppreference.com/w/cpp/atomic/memory_order) — The standard reference for all six memory orders with annotated examples.
2. ["Memory Barriers: a Hardware View for Software Hackers"](http://www.rdrop.com/users/paulmck/scalability/paper/whymb.2010.07.23a.pdf) — Paul E. McKenney's classic article explaining why CPUs need barriers, with practical examples.
3. ["A Tutorial Introduction to the ARM and POWER Relaxed Memory Models"](https://www.cl.cam.ac.uk/~pes20/ppc-supplemental/test7.pdf) — The formal model explained through litmus tests.
4. [Rust Atomics and Locks](https://marabos.nl/atomics/) — Mara Bos's book on low-level concurrency in Rust, with excellent coverage of memory ordering.
5. [Linux Kernel Memory Barriers Documentation](https://www.kernel.org/doc/Documentation/memory-barriers.txt) — The kernel's exhaustive guide to memory barriers, covering all supported architectures.
