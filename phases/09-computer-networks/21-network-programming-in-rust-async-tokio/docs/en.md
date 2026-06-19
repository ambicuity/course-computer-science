# Network Programming in Rust (async + tokio)

> Drive 10,000 concurrent connections from a handful of threads — async I/O is how production servers scale.

**Type:** Learn
**Languages:** Rust
**Prerequisites:** Phase 09 lessons 01–11 (especially lesson 10 for sockets and lesson 11 for TCP stack)
**Time:** ~75 minutes

## Learning Objectives

- Understand async I/O vs threaded I/O for network servers
- Implement a TCP echo server using Tokio's async runtime
- Compare async Rust with epoll/kqueue and Go goroutines
- Build a small async proxy that bridges two connections

## The Problem

Lesson 10 built a TCP echo server using `std::thread::spawn` — one OS thread per client. That works for a few hundred connections. But what happens when you need 10,000? 100,000? A million?

The **C10K problem** (10,000 concurrent connections) exposes the limits of thread-per-connection:

| Resource | Per thread | 1M connections |
|----------|-----------|----------------|
| Stack memory (Linux default) | 2 MB | 2 TB |
| Thread creation overhead | ~50 µs | ~50 seconds |
| Context switch (cross-core) | ~1 µs | cascading thrash |
| Page table overhead | full mm_struct | scheduler meltdown |

The numbers are brutal: 1M connections × 2 MB stack = 2 TB of RAM just for stacks, most of it completely unused. The kernel's scheduler was never designed for a million threads — context switching alone destroys cache locality.

Production servers (nginx, Redis, HAProxy) solve this with **event-driven I/O**: a single thread monitors thousands of sockets using the kernel's I/O readiness notification system (`epoll` on Linux, `kqueue` on macOS/BSD). When a socket has data ready, the thread processes it; when no socket is ready, the thread sleeps.

Rust's async/await brings this model to user code without manual callback registration or state machines. You write code that looks sequential — `socket.read(&mut buf).await` — and the compiler transforms it into a state machine that yields control when it can't make progress.

## The Concept

### Epoll/kqueue — the kernel foundation

The OS kernel provides system calls for scalable I/O multiplexing:

- **`epoll`** (Linux): Create an epoll fd, register interest in sockets with `EPOLLIN`/`EPOLLOUT`/`EPOLLERR`, then call `epoll_wait()` to block until any registered socket is ready. Returns a batch of ready events. O(1) registration and O(1) collection — independent of total connection count.
- **`kqueue`** (macOS/BSD): Same idea, different API. `kqueue()` creates a kernel event queue, `kevent()` registers filters and waits for events.

Both solve the same problem: instead of passing all 10,000 fds to `select()` (which scans them linearly every time), you register once and the kernel notifies you when something happens.

### Futures — the user-space abstraction

A Rust `Future` is a pollable computation:

```rust
pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}
```

Instead of blocking, a future returns `Poll::Pending` when it can't make progress. The `Context` contains a **Waker** — a callback the future uses to signal "I'm ready now, poll me again." This is cooperative scheduling: futures yield voluntarily at `.await` points, not preemptively.

### Waker-based cooperative scheduling

The flow for an async TCP read:

```
1. User code: socket.read(&mut buf).await
2. Runtime calls Future::poll() on the read future
3. Read future calls epoll_ctl() to register interest in EPOLLIN
4. Read future returns Poll::Pending
5. Runtime yields this task, polls something else
6. Kernel delivers data → epoll event fires
7. Runtime's I/O driver calls the Waker
8. Waker tells the scheduler: "this task is ready"
9. Runtime polls the read future again → returns Poll::Ready(n)
10. User code resumes after .await with the data
```

The key insight: **no thread is blocked during step 4**. The thread is off doing other work (polling other tasks). When data arrives, the waker notification pulls the original task back onto the run queue.

### Tokio's multi-threaded work-stealing runtime

```
┌───────────┐  ┌───────────┐      ┌───────────┐
│  Task 1   │  │  Task 2   │      │  Task N   │
│ (conn A)  │  │ (conn B)  │ ...  │ (conn N)  │
└─────┬─────┘  └─────┬─────┘      └─────┬─────┘
      │              │                   │
      ▼              ▼                   ▼
┌─────────────────────────────────────────────┐
│           Tokio Multi-Thread Scheduler       │
│  (work-stealing deque per worker thread)     │
├─────────────────────────────────────────────┤
│ Worker 1     │ Worker 2     │ Worker N       │
│ (core 0)     │ (core 1)     │ (core N-1)     │
└──────┬───────┴──────┬───────┴───────┬────────┘
       │              │               │
       ▼              ▼               ▼
┌─────────────────────────────────────────────┐
│              I/O Driver (epoll)              │
│  monitors all registered socket FDs          │
└─────────────────────────────────────────────┘
       │              │               │
       ▼              ▼               ▼
┌─────────────────────────────────────────────┐
│               Linux Kernel                  │
│         (TCP/IP, NIC, epoll)                 │
└─────────────────────────────────────────────┘
```

Many tasks → Tokio scheduler → worker threads → epoll → kernel. Each worker thread has a local task queue. When a worker runs out of tasks, it **steals** from another worker's queue — keeping all cores busy without requiring a central scheduler lock.

## Build It

### Step 1: Single-threaded async echo server with Tokio

The simplest Tokio TCP echo server — one task per connection, spawned on the runtime:

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Echo server listening on 127.0.0.1:8080");

    loop {
        let (mut socket, addr) = listener.accept().await?;
        println!("Accepted connection from: {}", addr);

        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            loop {
                match socket.read(&mut buf).await {
                    Ok(0) => break,          // EOF
                    Ok(n) => {
                        if socket.write_all(&buf[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            println!("Connection closed: {}", addr);
        });
    }
}
```

**Cargo.toml:**
```toml
[package]
name = "async-echo"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
```

**What each part does:**

| Element | Purpose |
|---------|---------|
| `#[tokio::main]` | Transforms `fn main()` into an async runtime entry point. Expands to `fn main() { tokio::runtime::Runtime::new().unwrap().block_on(async { ... }) }` |
| `TcpListener::bind().await` | Creates a TCP listener (wraps the kernel's `socket() + bind() + listen()`) and awaits until the kernel confirms bind is complete |
| `listener.accept().await` | Registers interest in new connections, yields until a SYN is accepted |
| `tokio::spawn(async move { ... })` | Creates a new task on the runtime — roughly equivalent to `thread::spawn` but at the user-space level. The closure is an async block that becomes a `Future` |
| `socket.read(&mut buf).await` | Registers interest in `EPOLLIN` on this socket, returns `Pending` until data arrives |
| `socket.write_all(&[..n]).await` | Registers interest in `EPOLLOUT`, writes all bytes, yields between partial writes |

Test it:
```bash
# Terminal 1 — start server
cargo run

# Terminal 2 — send test messages
echo "hello async" | nc 127.0.0.1 8080

# Or run multiple concurrent connections
for i in 1 2 3; do echo "msg $i" | nc 127.0.0.1 8080 &; done
```

### Step 2: Understanding the Tokio runtime

**Spawning blocking work** — CPU-bound or blocking I/O tasks must use `spawn_blocking` to avoid starving the async runtime:

```rust
use tokio::task;

#[tokio::main]
async fn main() {
    let result = task::spawn_blocking(|| {
        // This runs on a dedicated blocking thread pool
        std::thread::sleep(std::time::Duration::from_secs(2));
        42
    })
    .await
    .unwrap();
    println!("Blocking result: {}", result);
}
```

**Async sleep** — `tokio::time::sleep` yields the task without blocking the thread:

```rust
tokio::time::sleep(Duration::from_secs(1)).await;
```

This doesn't block the thread — the runtime deschedules this task and schedules another while the timer ticks down.

**Timeouts with `tokio::select!`** — race two futures against each other:

```rust
use tokio::time::{timeout, Duration};

let result = timeout(Duration::from_secs(5), async {
    // some async operation
    Ok::<_, String>("done")
}).await;

match result {
    Ok(Ok(val)) => println!("Completed: {}", val),
    Ok(Err(e)) => println!("Operation error: {}", e),
    Err(_) => println!("Timed out after 5 seconds"),
}
```

`timeout` wraps a future: if it doesn't complete within the duration, it cancels the inner future (via drop) and returns `Err(Elapsed)`.

### Step 3: Building an async proxy

A TCP proxy forwards bytes bidirectionally between two connections. With `tokio::io::copy_bidirectional`, this becomes remarkably concise:

```rust
use tokio::io;
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8081").await?;
    println!("Proxy listening on 127.0.0.1:8081, forwarding to 127.0.0.1:8080");

    let upstream = "127.0.0.1:8080".to_string();

    loop {
        let (mut downstream, addr) = listener.accept().await?;
        let upstream = upstream.clone();

        tokio::spawn(async move {
            match TcpStream::connect(&upstream).await {
                Ok(mut upstream_stream) => {
                    // copy_bidirectional returns (from_client, from_server) bytes copied
                    match io::copy_bidirectional(&mut downstream, &mut upstream_stream).await {
                        Ok((to_server, to_client)) => {
                            println!(
                                "Proxy {}: {} → {}, {} ← {}",
                                addr, to_server, upstream, to_client, upstream
                            );
                        }
                        Err(e) => eprintln!("Proxy error for {}: {}", addr, e),
                    }
                }
                Err(e) => eprintln!("Failed to connect to upstream {}: {}", upstream, e),
            }
        });
    }
}
```

This proxy accepts connections on port 8081 and forwards everything to port 8080. `copy_bidirectional` handles the full-duplex nature of TCP — it spawns two internal tasks (one for each direction) under the hood and completes when both sides close.

**Use case**: This pattern is the foundation of TCP load balancers, TLS terminators, and HTTP reverse proxies. HAProxy and nginx do essentially this, adding health checks, connection pooling, and protocol inspection.

### Step 4: Async with the custom TCP stack from lesson 11

Lesson 11 built a userspace TCP stack implementing the TCP state machine. To make it async-compatible, you need to implement `Future` on the connection type:

```rust
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::io::IoSliceMut;

impl Future for TcpConnection {
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.has_data() {
            Poll::Ready(this.read_from_buffer())
        } else if this.is_closed() {
            Poll::Ready(Ok(0))  // EOF
        } else {
            // Register waker for when the NIC delivers a packet
            this.register_waker(cx.waker());
            Poll::Pending
        }
    }
}
```

The key trait bounds: `Pin<&mut Self>` for self-referential structs (the future may move internally), and `Context<'_>` for waker access. The runtime (whether Tokio or a custom executor) calls `poll` repeatedly; between polls, the NIC driver calls `wake()` when a TCP segment arrives for this connection.

## Use It

Compare async Rust's approach with other concurrency models:

| Model | Example | Scheduling | Stack | Max connections |
|-------|---------|------------|-------|----------------|
| Thread per connection | Lesson 10 | Kernel preemptive | 2 MB per thread | ~4,000 (16 GB RAM) |
| Async event loop | nginx | Cooperative, single-threaded | None per connection | 500,000+ |
| Goroutines | Go net/http | M:N (goroutines → threads) | ~4 KB per goroutine | 100,000+ |
| Async Rust + Tokio | This lesson | Cooperative + work-stealing | None per task | 500,000+ |

**nginx** uses a single-threaded event loop with callbacks. The callback style makes control flow hard to follow — you register a read handler, a write handler, a timeout handler, etc. Rust's `async`/`.await` lets you write the same logic as straight-line code.

**Go goroutines** closely resemble Tokio tasks: both multiplex user-space units of work onto kernel threads. The differences:
- Go's runtime uses **segmented stacks** (growable, ~4 KB minimum) so every goroutine has its own stack. A Tokio task has no stack — all state lives in the generated `Future` enum.
- Go preemptively schedules at function call boundaries. Rust's `async` functions only yield at `.await` points (cooperative).
- Go's runtime is built into the language. Tokio is a library — you can swap it for `async-std`, `smol`, `glommio`, or a custom executor.

**C with epoll** requires manual state machines. To track where you are in a protocol (reading headers, reading body, writing response), you maintain an explicit `enum state` and a `switch/case` in each event callback. Rust's `async`/`.await` generates the state machine for you automatically.

### Documentation References

- [Tokio TcpListener](https://docs.rs/tokio/latest/tokio/net/struct.TcpListener.html)
- [Tokio AsyncReadExt](https://docs.rs/tokio/latest/tokio/io/trait.AsyncReadExt.html)
- [Tokio AsyncWriteExt](https://docs.rs/tokio/latest/tokio/io/trait.AsyncWriteExt.html)
- [tokio::io::copy_bidirectional](https://docs.rs/tokio/latest/tokio/io/fn.copy_bidirectional.html)
- [tokio::time::timeout](https://docs.rs/tokio/latest/tokio/time/fn.timeout.html)

## Read the Source

- **Tokio runtime scheduler**: `tokio/src/runtime/` — the multi-threaded work-stealing scheduler (`runtime/scheduler/multi_thread/`), the blocking pool (`runtime/blocking/`), and the I/O driver (`runtime/io/`). Start with `runtime/scheduler/multi_thread/worker.rs` to see how workers pop tasks from their local queue and steal from others.
- **Tokio I/O driver**: `tokio/src/io/driver/` — how Tokio registers FDs with epoll/kqueue and delivers readiness events back to user code via wakers.
- **Glommio** ([github](https://github.com/DataDog/glommio)): An alternative async runtime built on `io_uring` — Linux's async I/O interface that bypasses epoll entirely. Shows what the next generation of async I/O looks like.
- **Monoio** ([github](https://github.com/bytedance/monoio)): Another io_uring-based runtime from ByteDance, with a thread-per-core model (no work-stealing) for maximum CPU cache efficiency.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A reusable async TCP echo server template** — add protocol parsing on top to build Redis, HTTP, or custom protocol servers.
- **An async TCP proxy** — extend into a load balancer by adding round-robin upstream selection and health checks.

Copy `code/` into new projects and extend.

## Exercises

1. **Easy** — Modify the echo server to prepend `"echo: "` to each response. Hint: concatenate the prefix before writing back.

2. **Medium** — Add a 60-second idle timeout using `tokio::time::timeout`. Wrap the inner read loop so that if no data arrives within 60 seconds, the connection is closed. If a client reconnects, it should be treated as a new session.

3. **Hard** — Implement a line-based echo server using `AsyncBufReadExt::read_line` from `tokio-util`. The server should read one line at a time (up to `\n`), echo it back prefixed with `"you said: "`, and continue until EOF. Handle partial reads correctly — the buffered reader must accumulate data across multiple TCP segments until it finds a newline.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Future | "An async value" | A pollable computation that returns `Poll::Ready(output)` or `Poll::Pending`. The `async fn` keyword desugars into a Future impl generated by the compiler |
| Waker | "Wake me up when data arrives" | A callback handle registered with the runtime. Calling `.wake()` tells the executor to poll the associated future again |
| Runtime | "The executor and I/O driver together" | The component that calls `poll()` on futures, manages task queues, drives timers, and processes I/O events from epoll/kqueue |
| Tokio | "The de facto Rust async runtime" | An event-driven, non-blocking I/O platform for Rust with a work-stealing scheduler, I/O driver, timer wheel, and synchronization primitives |
| Task | "A spawned async unit of work" | A `Box<dyn Future<Output = ()> + Send>` managed by the runtime. `tokio::spawn` creates a task; the runtime polls it until completion |
| `async`/`.await` | "Write async code that looks sync" | `async fn` returns a Future without executing. `.await` yields control: if the awaited future is Pending, the scheduler deschedules this task and runs another |
| Cooperative scheduling | "Tasks yield voluntarily" | Tasks only yield at `.await` points. A long CPU-bound loop inside an async fn without `.await` will block its worker thread indefinitely — use `spawn_blocking` or periodic `.yield_now()` |
| Work stealing | "Busy cores help idle cores" | Each worker thread has a local deque of tasks. When a worker runs out of tasks, it steals from the back of another worker's deque, balancing load with minimal contention |

## Further Reading

- [Tokio Tutorial](https://tokio.rs/tokio/tutorial) — official intro with channels, shared state, and deeper I/O patterns
- [Async: What is Rust?](https://without.boats/blog/references/) by without.boats (boats) — deep dive on the semantics of async in Rust
- [Rust Async Book](https://rust-lang.github.io/async-book/) — comprehensive guide from the Rust project
- [Linux man 7 epoll](https://man7.org/linux/man-pages/man7/epoll.7.html) — the kernel interface Tokio's I/O driver wraps
- [The C10K Problem](http://www.kegel.com/c10k.html) — the original essay that defined the problem this lesson solves
