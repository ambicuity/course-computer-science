# Tokio and the Async Runtime in Rust

> Tokio is Rust's most widely used async runtime — a multi-threaded, work-stealing scheduler built on top of `mio` (the OS I/O multiplexer). It multiplexes lightweight tasks onto a pool of OS threads using cooperative scheduling, driving millions of concurrent I/O operations from a handful of threads. This is how every production Rust async service works under the hood.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 13 lessons 11–12 (Futures/Promises, Reactor/Proactor)
**Time:** ~75 minutes

## Learning Objectives

- Explain the architecture of Tokio: work-stealing scheduler, I/O driver (mio), timer wheel, and task lifecycle.
- Distinguish Tokio tasks from OS threads — what they cost, how they are scheduled, and why the difference matters.
- Write basic async programs using `tokio::spawn` and `JoinHandle` to run concurrent tasks.
- Build a TCP echo server using `TcpListener` with a task-per-connection model.
- Synchronize shared state across tasks using `tokio::sync` (Mutex, RwLock, mpsc, broadcast).
- Compose concurrent futures with `join!` (all must complete) and `select!` (first wins).
- Customize a Tokio runtime using `Runtime::Builder` to control worker thread count, event interval, and driver selection.
- Read real Tokio source code to understand how the runtime is built.

## The Problem

Rust's async model is **lazy** — futures do nothing until polled. Something has to do the polling, and that something is the **executor** (or **runtime**). Without a runtime, you can `async fn` all day but never make progress.

Building your own executor is instructive (see lesson 11) but impractical for real work. Production runtimes must:

1. **Drive millions of concurrent tasks** on a small number of OS threads.
2. **Integrate with the OS I/O subsystem** (epoll on Linux, kqueue on macOS, io_uring on modern Linux, IOCP on Windows) so that waiting for a socket does not waste a thread.
3. **Schedule tasks cooperatively** — a task that does not yield should not starve others.
4. **Support timers, channels, synchronization, and file I/O** — the building blocks of real async code.

Tokio solves all four. It is the de facto standard async runtime in Rust, used by hyper (HTTP), tonic (gRPC), axum (web frameworks), and most production async services.

## The Concept

### Tokio's Architecture

Tokio is composed of four layers:

```
┌──────────────────────────────────────┐
│         Application Tasks            │  ← user code: async fns
│   (tokio::spawn, async { ... })      │
├──────────────────────────────────────┤
│      Work-Stealing Scheduler         │  ← distributes tasks across workers
│   (global + per-worker local queues) │
├──────────────────────────────────────┤
│     I/O Driver (mio) + Timer         │  ← epoll/kqueue, timer wheel
│   (reactor: drives I/O events)       │
├──────────────────────────────────────┤
│       OS: epoll / kqueue / iocp      │  ← kernel-level I/O multiplexing
└──────────────────────────────────────┘
```

### Multi-Threaded Work-Stealing Runtime

Tokio's default runtime (`new_multi_thread`) creates a pool of OS worker threads. Each worker has:

- A **local task queue** (a lock-free deque, similar to the Chase-Lev deque from lesson 08).
- A **shared global queue** for tasks spawned from outside any worker.

A worker runs tasks from its local queue in FIFO order. When its local queue is empty, it **steals** from the back of another worker's local queue. Work-stealing keeps all cores utilized even under uneven load.

This is the same fundamental idea as the work-stealing schedulers covered in lesson 17, but integrated with async I/O.

### Tasks vs Threads

| Property | OS Thread | Tokio Task |
|----------|-----------|------------|
| Creation cost | ~µs (syscall) | ~ns (allocation) |
| Memory per unit | ~MB (stack) | ~tens of bytes (state machine) |
| Scheduling | Preemptive (kernel decides) | Cooperative (task yields at .await) |
| Max count on 8 GB | ~8,000 | Millions |
| Context switch | ~µs (kernel mode) | ~ns (user mode) |

A Tokio task is an async block or function passed to `tokio::spawn`. It is compiled into a state machine (by the async/await desugaring) and polled by the scheduler. The task's state lives on the heap in an allocation that is tiny compared to a thread stack.

### Cooperative Scheduling

Tokio tasks are **not** preemptively scheduled. A task runs until it reaches an `.await` point that returns `Poll::Pending`. If a task never yields (e.g., an infinite loop without .await), it monopolizes the worker thread forever.

**Yield points** are `.await` calls that might return Pending:
- `time::sleep(...).await`
- `socket.read(...).await`
- `mpsc::Receiver::recv().await`
- `tokio::task::yield_now().await` (explicit yield)

The runtime's event loop polls tasks in batches. Between batches, it checks for new I/O events. The `event_interval` parameter (default 61) controls how many tasks are polled before each I/O check, balancing throughput against I/O latency.

### I/O Driver (mio)

Tokio does not talk to epoll/kqueue directly. It uses the **mio** crate, a safe abstraction over OS I/O multiplexers:

- **Linux**: epoll (edge-triggered)
- **macOS/iOS**: kqueue
- **Windows**: IOCP (I/O completion ports)
- **Linux 5.1+**: io_uring (via tokio-uring, not in core Tokio)

The I/O driver (also called the **reactor**) maintains a registry of file descriptors. When a task calls `TcpStream::read().await`, the stream registers its fd with the reactor and returns Pending. When the reactor detects the fd is readable (via epoll_wait), it wakes the task. The task is then polled again, and this time the read completes.

This is the Reactor pattern from lesson 12, adapted to Rust's async model.

### Timer Wheel

Tokio's timer subsystem manages millions of concurrent timeouts efficiently. Internally it uses a hierarchical hashed timer wheel:

- Time is divided into slots (like a clock with multiple hands: second, minute, hour).
- Each timeout is placed in the slot corresponding to its expiration time.
- The runtime advances through slots as time passes, firing expired timers.
- Insertion and removal are O(1) amortized.

This is the same data structure used in Netty (Java) and the Linux kernel for managing large numbers of timers.

## Build It

The code in `code/src/main.rs` implements five progressive steps. Run it with:

```bash
cd code
cargo run
```

### Step 1: `tokio::spawn` — Basic Async Tasks

```rust
let compute = tokio::spawn(async {
    let mut sum = 0u64;
    for i in 1..=100 {
        sum += i;
        if i % 50 == 0 {
            tokio::task::yield_now().await;
        }
    }
    sum
});
println!("{}", compute.await.expect("join failed"));
```

Key observations:

- `tokio::spawn` returns a `JoinHandle<T>`. The `.await` on the handle blocks the current task until the spawned task completes and returns `Result<T, JoinError>`.
- If the spawned task panics, the panic is caught and returned as `JoinError`. The runtime is not affected.
- `yield_now()` is an explicit yield point. Without it, a tight loop would monopolize the worker thread.
- Multiple spawned tasks run concurrently on the thread pool. The `join!` macro shows they complete in the time of the slowest, not the sum.

### Step 2: TCP Echo Server

```rust
let listener = TcpListener::bind("127.0.0.1:0").await?;
let server = tokio::spawn(async move {
    loop {
        let (mut stream, peer) = listener.accept().await?;
        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            loop {
                let n = stream.read(&mut buf).await?;
                if n == 0 { return; }
                stream.write_all(&buf[..n]).await?;
            }
        });
    }
});
```

This is the canonical **task-per-connection** pattern:

1. **Accept loop** runs in its own task, accepting connections.
2. Each accepted connection spawns a **handler task** that reads and echoes.
3. The handler reads into a buffer. On EOF (n == 0), the task exits.
4. The handler writes back the same bytes — this is the echo.

The lesson code tests with a single client and three concurrent clients, verifying that each gets its own echo session.

**Production note**: Real echo servers buffer more carefully (framing, backpressure) and use structured protocols. This minimalist version teaches the core pattern.

### Step 3: Shared State with `tokio::sync`

```rust
// Mutex — safe to hold across .await
let counter = Arc::new(Mutex::new(0u64));
for i in 0..10 {
    let c = counter.clone();
    tokio::spawn(async move {
        *c.lock().await += 1;
    });
}

// RwLock — concurrent reads
let data = Arc::new(RwLock::new(vec![0u64; 5]));
// Writer
let mut guard = w_data.write().await;
// Readers (concurrent)
let guard = d.read().await;

// mpsc — multi-producer, single-consumer
let (tx, mut rx) = mpsc::channel::<u64>(32);

// broadcast — fan-out
let (tx, mut rx1) = broadcast::channel::<i32>(16);
let mut rx2 = tx.subscribe();
```

Why `tokio::sync::Mutex` and not `std::sync::Mutex`?

`std::sync::MutexGuard` is not `Send`. If you hold a guard across an `.await` point, the compiler rejects it because the future might be polled by a different thread after the await. `tokio::sync::Mutex` solves this by making `lock()` an async fn that releases the lock when the task would block. For short, non-async critical sections, `std::sync::Mutex` is still fine and is faster.

The broadcast channel demonstrates fan-out: every message sent is received by every subscriber. The mpsc channel demonstrates producer-consumer: one consumer receives all messages.

### Step 4: `join!` and `select!`

```rust
// join! — wait for all
let (a, b) = tokio::join!(
    async { sleep(20ms).await; "alpha" },
    async { sleep(30ms).await; "beta" },
);

// select! — first to complete wins
let winner = tokio::select! {
    v = async { sleep(30ms).await; "slow" } => v,
    v = async { sleep(10ms).await; "fast" } => v,
};

// select! as timeout
let result = tokio::select! {
    v = async { sleep(100ms).await; "done" } => v,
    _ = sleep(30ms) => "TIMEOUT",
};
```

- `join!` runs all futures concurrently and returns a tuple of all outputs. The total time equals the slowest future, not the sum.
- `select!` runs all futures and returns the output of the first one to complete. The other branches are cancelled.
- `select!` with a timeout branch is the idiomatic way to add deadlines to async operations.

### Step 5: Custom Runtime Builder

```rust
let rt = Builder::new_multi_thread()
    .worker_threads(4)
    .thread_name("my-rt")
    .enable_io()
    .enable_time()
    .event_interval(61)
    .build()?;

rt.block_on(async { /* ... */ });

let rt_st = Builder::new_current_thread()
    .enable_all()
    .build()?;

rt_st.block_on(async { /* ... */ });
```

The `#[tokio::main]` macro expands to a default multi-threaded runtime. When you need control, use `Builder`.

Key builder parameters:

| Parameter | Default | Effect |
|-----------|---------|--------|
| `worker_threads(n)` | CPU core count | Number of OS worker threads in the pool. |
| `thread_name(name)` | "tokio-runtime-worker" | Prefix for worker thread names (useful for debugging with `ps` / `top`). |
| `enable_io()` | off | Enables the I/O driver (mio reactor). Required for `TcpListener`, `TcpStream`, etc. |
| `enable_time()` | off | Enables the timer driver. Required for `time::sleep`, `time::timeout`, etc. |
| `event_interval(n)` | 61 | Max scheduler ticks between I/O event checks. Lower = more responsive I/O, higher = better throughput for CPU-bound tasks. |
| `max_io_events_per_tick(n)` | 1024 | Max I/O events processed per tick. |

## Use It

In production, Tokio usage follows these patterns:

**Web services** (axum, actix-web, warp):
```rust
#[tokio::main]
async fn main() {
    let app = axum::Router::new()
        .route("/", axum::routing::get(handler));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
}
```

**Async database clients** (sqlx, tokio-postgres):
```rust
let pool = sqlx::PgPool::connect("postgres://...").await?;
let row: (i64,) = sqlx::query_as("SELECT $1")
    .bind(42)
    .fetch_one(&pool).await?;
```

**Background job processors**:
```rust
let (tx, mut rx) = mpsc::channel(1024);
// Producer
tokio::spawn(async move { tx.send(job).await; });
// Consumer pool
for _ in 0..num_workers {
    let mut rx = rx.resubscribe();
    tokio::spawn(async move {
        while let Some(job) = rx.recv().await {
            process(job).await;
        }
    });
}
```

Most projects use `#[tokio::main]` and never touch the builder. Custom runtimes are useful when you need:
- **Thread count control** (embedding, resource-constrained environments).
- **Multiple isolated runtimes** (mixing I/O patterns with different latency requirements).
- **Single-threaded execution** (CLI tools, WASM, tests).

## Read the Source

The following files in the Tokio source tree are directly relevant to this lesson:

- **`tokio/src/runtime/scheduler/multi_thread/worker.rs`** — The core worker loop. Shows how each worker polls its local queue, checks the global queue, and attempts to steal. This is the heart of the work-stealing scheduler.

- **`tokio/src/runtime/io/mod.rs`** — The I/O driver (reactor) that wraps mio. See how fds are registered, how events are polled, and how wakers are delivered.

- **`tokio/src/runtime/task/mod.rs`** — The task abstraction: how a future is wrapped into a `Task` cell with its lifecycle state (Idle, Running, Completed, Cancelled).

- **`tokio/src/time/driver/mod.rs`** — The timer driver. Implements the hierarchical timer wheel that manages `sleep`, `timeout`, and `interval` handles.

- **`tokio/src/sync/mutex.rs`** — The async-aware mutex. Compare it with `std::sync::Mutex` to understand the locking protocol differences.

- **`mio/src/sys/unix/mod.rs`** — The mio crate's epoll/kqueue wrappers. This is what Tokio sits on top of.

Each file is well-commented. Reading even the module-level docs is educational.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained Tokio reference snippet** covering spawn, TCP I/O, shared state, join/select, and runtime builder patterns. You can adapt the echo server skeleton for any async TCP service, reuse the shared-state patterns in tokio-based applications, and copy the custom runtime setup for projects that need non-default configuration.

## Exercises

1. **Easy** — Modify the echo server to prefix each response with `"echo: "`. Add a second client that sends 1000 small messages and verify all echoes arrive correctly.

2. **Medium** — Replace the task-per-connection model in the echo server with a fixed-size task pool using `tokio::sync::Semaphore`. Limit concurrent connections to 3. When a 4th client connects, it should wait (not get refused). Measure latency under load with and without the semaphore.

3. **Hard** — Implement a simple HTTP health-check endpoint inside the echo server. On receiving `"GET /health HTTP/1.1\r\n"`, respond with `"HTTP/1.1 200 OK\r\n\r\nOK"` instead of echoing. All other data is echoed. Test with `curl` and a plain TCP client.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Tokio | "The Rust async runtime" | A multi-threaded, work-stealing scheduler built on mio (epoll/kqueue) that runs asynchronous tasks to completion. |
| Async runtime | "The thing that polls futures" | The combination of an executor (polls tasks), a reactor (drives I/O events), and a timer (manages timeouts). |
| Task | "A unit of async work" | A future submitted to the runtime via `tokio::spawn`. It is a state machine allocated on the heap, not an OS thread. |
| Spawn | "Fire and forget a task" | `tokio::spawn(future)` — queues a future on the runtime's thread pool and returns a `JoinHandle` to await its output. |
| Work-stealing | "Idle workers grab tasks from busy ones" | When a worker's local queue is empty, it steals from the back of another worker's queue. Keeps all cores utilized under uneven load. |
| mio | "Tokio's I/O layer" | A safe Rust wrapper around epoll/kqueue/IOCP. Tokio's I/O driver calls mio to register fds and poll for events. |
| Cooperative scheduling | "Tasks yield instead of being preempted" | A task runs until it hits a yield point (.await that returns Pending). If it never yields, it can monopolize a worker thread. |
| Yield | "Give up the CPU voluntarily" | An .await point that returns Pending, telling the scheduler to run another task. `tokio::task::yield_now()` is the explicit form. |
| Timer wheel | "Efficient timeout management" | A hierarchical data structure storing timers in time slots. O(1) insert and fire. Used by Tokio to manage millions of concurrent timeouts. |
| mpsc | "Multi-producer, single-consumer channel" | A channel where any number of senders can push messages, but only one receiver can pop them. `tokio::sync::mpsc`. |
| broadcast | "Fan-out channel" | A channel where every message is delivered to every receiver. `tokio::sync::broadcast`. |
| select! | "First completed wins" | A macro that polls multiple futures and returns the output of the first one to complete. The other branches are cancelled. |

## Further Reading

- Tokio documentation and guides: https://tokio.rs/tokio/tutorial — the official Tokio tutorial. Async in Action is the best starting point.
- Alice Ryhl, *The Tokio Architecture* series (tokio.rs blog, 2023). Explains the scheduler, I/O driver, and timer in detail with diagrams.
- Carl Lerche, *Structuring the Tokio Runtime for the Future* (tokio.rs blog, 2021). Design rationale for Tokio 1.x's architecture.
- Jon Gjengset, *Rust for Rustaceans*, Chapter 6 (Async/Futures). The section on executors covers the general model that Tokio implements.
- mio documentation: https://github.com/tokio-rs/mio — the OS I/O abstraction that Tokio builds on.
- `tokio/src/runtime/scheduler/multi_thread/worker.rs` — the file to read if you only read one. It is the core of the work-stealing scheduler.
- *Hashed and Hierarchical Timing Wheels* (George Varghese and Tony Lauck, 1997). The original paper describing the timer wheel data structure.
