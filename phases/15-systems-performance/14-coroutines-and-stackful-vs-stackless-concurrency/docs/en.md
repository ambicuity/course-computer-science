# Coroutines and Stackful vs Stackless Concurrency

> Cooperative multitasking, suspension points, and why the stack makes all the difference.

**Type:** Learn
**Languages:** C++, Rust
**Prerequisites:** Phase 15 lessons 01–13
**Time:** ~75 minutes

## Learning Objectives

- Explain cooperative vs preemptive multitasking and why coroutines enable lightweight concurrency.
- Distinguish stackful coroutines (goroutines, fibers) from stackless coroutines (C++20 coroutines, Rust async/await).
- Implement a C++20 coroutine generator and task with `co_await`.
- Implement a Rust async function and understand `Pin`, `Future`, `Poll`, and `Waker`.
- Benchmark coroutine vs thread performance for I/O-bound workloads and interpret the results.
- Choose between threads, coroutines, and green threads based on workload characteristics.

## The Problem

This lesson sits in **Phase 15 — Systems Programming & Performance**. Imagine you need to handle 100,000 simultaneous network connections. Creating 100,000 OS threads would consume 100,000 × 1 MB = 100 GB of stack memory alone — before doing any work. Each thread context switch costs ~1–10 μs of kernel overhead. The hardware can't keep up.

Coroutines solve this by letting you suspend and resume execution without a full stack per unit of work. A coroutine that's waiting for I/O simply yields control, and the runtime picks up another coroutine that's ready. The memory cost drops from megabytes to kilobytes per concurrent task, and context switches become function calls instead of kernel transitions.

Without understanding coroutines — stackful vs stackless, how suspension works, and when each model wins — you can't tune cache, branches, or I/O. You can't win 10× by knowing the machine if you're burning megabytes per idle connection.

## The Concept

### Cooperative vs Preemptive Multitasking

**Preemptive multitasking** (OS threads): The kernel decides when to switch tasks. A thread can be interrupted at any instruction. Each thread gets its own full stack (1–8 MB default). Context switch involves saving/restoring all register state through the kernel.

**Cooperative multitasking** (coroutines): The program decides when to yield. A coroutine can only suspend at explicit yield points (`co_await`, `.await`, `yield`). This makes reasoning about shared state simpler — data races are harder because only one coroutine runs at a time on a given thread, and suspension points are visible in code.

### Stackful Coroutines

A stackful coroutine has its own complete call stack, just like an OS thread but smaller.

```
┌──────────────────┐
│   OS Thread      │  1–8 MB stack
│  ┌────────────┐  │
│  │  func_a()  │  │
│  │  func_b()  │  │
│  │  func_c()  │  │
│  └────────────┘  │
└──────────────────┘

┌──────────────┐
│ Goroutine    │  4 KB initial stack (grows as needed)
│  ┌────────┐  │
│  │ func() │  │
│  └────────┘  │
└──────────────┘
```

**Key property**: Suspension can happen at any point in the call stack, not just in the coroutine function itself. If `func_a()` calls `func_b()` which calls `func_c()`, and `func_c()` suspends, the entire call stack is preserved.

**Examples**: Goroutines (Go), fibers (Windows), Lua coroutines, Python generators (pre-asyncio).

**Stack growth**: Go goroutines start at 4 KB and grow by allocating a new larger stack and copying the old one (split stacks were abandoned in Go 1.3; now contiguous stacks grow by realloc+copy at ~70–80% utilization).

### Stackless Coroutines

A stackless coroutine does not have its own stack. Instead, the compiler transforms the coroutine body into a state machine and stores the state in a heap-allocated frame.

```
┌──────────────────────┐
│  Coroutine Frame     │  ~48–256 bytes
│  ┌────────────────┐  │
│  │ state: 2       │  │  ← which suspend point to resume at
│  │ local_a: 42    │  │  ← saved local variables
│  │ promise: ...   │  │  ← result channel
│  └────────────────┘  │
└──────────────────────┘
```

**Key property**: Suspension can only happen in the coroutine function itself (or in functions the compiler inlines). A regular function called from a stackless coroutine cannot independently suspend — it must be an `async` function too for the suspension to propagate.

**Examples**: C++20 coroutines, Rust async/await, JavaScript async/await, C# async/await.

**State machine transformation**: The compiler converts:
```cpp
task<int> example() {
    auto a = co_await op1();
    auto b = co_await op2();
    co_return a + b;
}
```

Into roughly:
```cpp
int example_sm(example_frame* frame) {
    switch (frame->state) {
        case 0: frame->a = op1(); frame->state = 1; return suspended;
        case 1: frame->b = op2(); frame->state = 2; return suspended;
        case 2: return frame->a + frame->b;
    }
}
```

### Comparison Table

| Property | OS Thread | Stackful Coroutine (Goroutine) | Stackless Coroutine (C++/Rust) |
|----------|-----------|-------------------------------|-------------------------------|
| Stack/frame size | 1–8 MB | 4 KB initial, grows | 48–256 B frame |
| Switch cost | 1–10 μs (kernel) | ~100 ns (user-space) | ~10 ns (function call) |
| Can suspend in callee | Yes | Yes | No (unless async call chain) |
| Stack trace in debugger | Full stack | Full stack per goroutine | Corrupted/fragmented |
| Creation cost | ~50 μs | ~0.3 μs | ~0.05 μs |
| Preemption | Kernel-preempted | Cooperative (Go preempts at safepoints) | Cooperative |
| Max practical concurrent | ~10,000 (memory-bound) | ~1,000,000+ | ~10,000,000+ |

### Context Switch Cost Breakdown

A thread context switch requires:
1. Trap to kernel (syscall)
2. Save all general-purpose registers (16–32 registers)
3. Save floating-point/SIMD state (512 bytes on x86-64 with AVX-512)
4. Switch page tables (TLB flush potential)
5. Update kernel scheduler data structures
6. Return to user space

A coroutine switch requires:
1. Save callee-saved registers (6–8 registers on x86-64)
2. Jump to next coroutine (function call overhead)

That's why coroutine switches are ~100× faster than thread switches.

### When Coroutines Are Faster (I/O-Bound)

I/O-bound work spends most of its time waiting — for network, disk, or other external resources. Coroutines excel because:
- Thousands of coroutines can wait simultaneously with near-zero overhead.
- No kernel transition penalty when switching between waiting tasks.
- Memory overhead per waiting task is tiny, fitting in cache.

### When Threads Are Faster (CPU-Bound)

CPU-bound work spends most of its time computing. Threads win because:
- No coroutine scheduling overhead — the thread just runs.
- OS can optimally schedule threads across cores (work-stealing runtimes approximate this).
- No indirection through promise/future machinery.
- Preemption prevents one task from monopolizing a core.

**Rule of thumb**: If your workload is >90% I/O, use coroutines. If it's >90% CPU, use threads. If it's mixed, use coroutines on top of a thread pool.

## Build It

### Step 1: C++20 Coroutine — Minimal Generator

```cpp
#include <coroutine>
#include <iostream>

template<typename T>
struct Generator {
    struct promise_type {
        T current_value;
        auto yield_value(T value) {
            current_value = value;
            return std::suspend_always{};
        }
        auto get_return_object() {
            return Generator{std::coroutine_handle<promise_type>::from_promise(*this)};
        }
        auto initial_suspend() { return std::suspend_always{}; }
        auto final_suspend() noexcept { return std::suspend_always{}; }
        void return_void() {}
        void unhandled_exception() { throw; }
    };
    std::coroutine_handle<promise_type> handle;
    bool next() {
        if (handle && !handle.done()) {
            handle.resume();
            return !handle.done();
        }
        return false;
    }
    T value() { return handle.promise().current_value; }
    ~Generator() { if (handle) handle.destroy(); }
};

Generator<int> fibonacci() {
    int a = 0, b = 1;
    while (true) {
        co_yield a;
        auto tmp = a;
        a = b;
        b = tmp + b;
    }
}

int main() {
    auto gen = fibonacci();
    for (int i = 0; i < 10; ++i) {
        gen.next();
        std::cout << gen.value() << " ";
    }
}
```

### Step 2: C++20 Coroutine — Task with `co_await`

```cpp
#include <coroutine>
#include <iostream>
#include <chrono>

struct Task {
    struct promise_type {
        int result;
        auto get_return_object() { return Task{std::coroutine_handle<promise_type>::from_promise(*this)}; }
        auto initial_suspend() { return std::suspend_never{}; }
        auto final_suspend() noexcept { return std::suspend_always{}; }
        void return_value(int val) { result = val; }
        void unhandled_exception() { throw; }
    };
    std::coroutine_handle<promise_type> handle;
    int get() { return handle.promise().result; }
    ~Task() { if (handle) handle.destroy(); }
};

struct Awaitable {
    int ms;
    bool await_ready() { return false; }
    void await_suspend(std::coroutine_handle<>) {}
    void await_resume() {}
    auto operator co_await() { return *this; }
};

Task compute() {
    co_await Awaitable{100};
    co_return 42;
}
```

### Step 3: Rust Async/Await — Minimal Future

```rust
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct Delay { ms: u64 }

impl Future for Delay {
    type Output = ();
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        Poll::Ready(())
    }
}

async fn compute() -> i32 {
    Delay { ms: 100 }.await;
    42
}
```

### Step 4: Rust — Understanding `Pin`

`Pin` prevents a value from being moved in memory. This is critical because:

1. Async functions are compiled to state machines stored on the heap.
2. The state machine may contain self-referential pointers (a local variable points to another local in the same frame).
3. If the state machine were moved, those self-referential pointers would dangle.

```rust
use std::pin::Pin;

async fn self_referential() {
    let data = String::from("hello");
    let reference = &data; // reference points into the async frame
    Delay { ms: 10 }.await; // suspension point — frame must not move
    println!("{}", reference);
}
// The compiler enforces Pin<> at the .await boundary
```

Without `Pin`, moving the async frame after `reference` was created would invalidate `reference`. `Pin<P>` where `P: DerefMut` guarantees the pointee won't be moved, making self-referential structs safe.

## Use It

### Production: C++20 Coroutines in the Wild

- **CppCoro** (Lewis Baker): The reference C++20 coroutine library. Provides `task<T>`, `generator<T>`, and async I/O primitives. Key files to study: `include/cppcoro/task.hpp` for the promise_type implementation.
- **LLVM coroutine transformation**: The LLVM backend lowers `coro.save`, `coro.suspend`, and `coro.resume` intrinsics into state machine code. See `llvm/lib/Transforms/Coroutines/CoroSplit.cpp` — this is where the compiler decides how to allocate and lay out coroutine frames.

### Production: Rust Async Runtime

- **Tokio**: The de facto async runtime for Rust. Key types: `tokio::spawn` (schedule a future on the thread pool), `tokio::net::TcpStream` (async I/O), `tokio::select!` (wait on multiple futures).
- **Glommio**: A thread-per-core async runtime. Avoids cross-thread synchronization by pinning each executor to a CPU core. Better for workloads with high cache affinity requirements.
- The `Future::poll` method is never called directly by users — the executor (Tokio) calls it. The `Waker` inside `Context` tells the executor when to poll again.

### Production: Goroutines

- **Go runtime scheduler** (`runtime/proc.go`): The M:N scheduler maps M goroutines onto N OS threads. Key structures: `g` (goroutine struct with 4 KB stack), `m` (machine/thread), `p` (processor, holds the run queue).
- **Stack growth** (`runtime/stack.go`): When a goroutine's stack overflows its 4 KB initial allocation, `newstack()` allocates a 2× larger stack and copies the old stack frame by frame, adjusting pointers using `adjustpointers()`.
- Go preempted goroutines at function calls (cooperative) until Go 1.14, which added asynchronous preemption using signals (SIGURG). This prevents infinite loops from freezing the scheduler.

### Stack Size Comparison in Practice

```
OS Thread (Linux default):     8,388,608 bytes  (8 MB)
goroutine (initial):               4,096 bytes  (4 KB)
C++20 coroutine frame:               48 bytes  (varies by captured locals)
Rust async state machine:            64 bytes  (varies by captured locals)
```

The 200× difference between goroutines and threads, and the 100× difference between C++/Rust coroutines and goroutines, is the key to concurrent I/O at scale.

## Read the Source

- **Go**: `src/runtime/proc.go` — the M:N scheduler, `schedule()`, `findrunnable()`, `execute()`. Start with `schedule()` to see how the runtime picks the next goroutine to run.
- **Go**: `src/runtime/stack.go` — `newstack()` and `copystack()` for stack growth.
- **LLVM**: `llvm/lib/Transforms/Coroutines/CoroSplit.cpp` — the pass that transforms coroutine IR into state machines.
- **Tokio**: `tokio/src/runtime/scheduler/multi_thread/worker.rs` — work-stealing scheduler implementation.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`coroutine_reference.md`** — A quick-reference cheat sheet covering stackful vs stackless comparison, C++20 coroutine keywords, Rust async primitives, stack sizes, switch costs, and decision flowchart for choosing concurrency models.

## Exercises

1. **Easy** — Modify the C++ generator example to produce prime numbers instead of Fibonacci numbers. Verify it produces: 2, 3, 5, 7, 11, 13, 17, 19, 23, 29.
2. **Medium** — Write a Rust async function that performs two sequential I/O operations (simulated with `tokio::time::sleep`), then benchmark it against spawning two threads. Record the time difference for 10,000 concurrent operations.
3. **Hard** — Implement a simple work-stealing executor in Rust: create N worker threads, each with its own deque of futures. When a worker's deque is empty, it steals from a random peer. Measure throughput for 100,000 spawned tasks.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Coroutine | "Lightweight thread" | A function that can suspend and resume, yielding control cooperatively |
| Stackful coroutine | "Green thread" | A coroutine with its own call stack, allowing suspension from any call depth |
| Stackless coroutine | "Async function" | A coroutine transformed into a state machine with a heap-allocated frame, no independent stack |
| `co_await` | "Wait for this" | Suspend the current coroutine and register it to resume when the awaitable completes |
| `co_yield` | "Produce a value" | Suspend and emit a value to the caller, like a generator's yield |
| `promise_type` | "The coroutine config" | The C++20 customization point that defines how a coroutine allocates its frame, handles exceptions, and returns results |
| `Pin` | "Pin the future" | A Rust wrapper that prevents a value from being moved, essential for self-referential async state machines |
| `Future::poll` | "Check if done" | The core Rust async trait: returns `Poll::Ready(value)` or `Poll::Pending` with a `Waker` to schedule re-polling |
| `Waker` | "Wake-up callback" | A handle passed to the executor to signal that a suspended future should be polled again |
| Goroutine | "Go's thread" | A stackful coroutine managed by the Go runtime's M:N scheduler with growable stacks starting at 4 KB |
| Context switch | "Saving state" | Changing the active execution context; thread switches involve kernel transitions, coroutine switches are user-space function calls |
| Green thread | "User-space thread" | A thread managed by a runtime (not the OS), typically stackful with cooperative or runtime-forced preemption |

## Further Reading

- **C++ Coroutines**: Lewis Baker's series — "Coroutine Theory" and "Understanding operator co_await" (CppCoro author's blog)
- **Rust Async Book**: https://rust-lang.github.io/async-book/ — "Getting Started" and "Under the Hood: Executors and Reactors"
- **Go Runtime Scheduler**: Dmitry Vyukov's "Go scheduler design doc" and "Analysis of the Go Runtime Scheduler" (IBM)
- **Stackless vs Stackful**: Simon Tatham's "Coroutines in C" — the classic explanation of state-machine transformation
- **Pin**: The Rust RFC 2677 — `pin` and `Unpin` semantics, and Jon Gjengset's "How Rust Optimizes async/await" video
- **Async Runtimes**: Tokio documentation (https://tokio.rs), Glommio design doc (Datadog)