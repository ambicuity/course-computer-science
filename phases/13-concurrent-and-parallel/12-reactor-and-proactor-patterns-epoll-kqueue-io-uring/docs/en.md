# Reactor and Proactor Patterns — epoll, kqueue, io_uring

> Reactor and Proactor Patterns — epoll, kqueue, io_uring — the part of CS you can't skip.

**Type:** Build
**Languages:** C, Rust
**Prerequisites:** Phase 13, Lessons 01–11
**Time:** ~90 minutes

## Learning Objectives

- Distinguish the Reactor pattern (readiness notification) from the Proactor pattern (completion notification).
- Implement an epoll-based echo server in C: register fds, epoll_wait loop, accept/read/write.
- Implement an io_uring echo server in C: submission queue, completion queue, readv/writev with SQEs.
- Implement a Tokio-based async echo server in Rust and understand how Tokio wraps epoll/kqueue.
- Compare epoll vs io_uring vs Tokio throughput at different concurrency levels.

## The Problem

You are building a web server that must handle 10,000 concurrent connections. The naive approach — one thread per connection — fails at this scale: each thread consumes ~8 MB of stack, so 10,000 threads need 80 GB just for stacks. Context switching between 10,000 threads saturates the CPU with overhead, not work.

The solution is *event-driven I/O*: a single thread monitors many file descriptors and only processes the ones that are ready. But "monitors" hides a crucial distinction:

- **Reactor:** The system tells you a socket is *ready to read*. You still have to call `read()` yourself.
- **Proactor:** The system tells you the data is *already in your buffer*. You don't call `read()` — it was done for you.

The wrong choice means:
- Plain `select()`: O(n) scan of all fds, limits at FD_SETSIZE (1024).
- epoll (Linux reactor): O(1) readiness notification. Fast. But you still pay for the `read()`/`write()` syscalls.
- kqueue (BSD reactor): Same pattern as epoll but with a richer event filter system.
- IOCP (Windows proactor): True completion-based I/O. No extra syscall — the kernel writes directly into your buffer.
- io_uring (Linux proactor): The new hotness. Submission queue + completion queue. Zero or near-zero syscall overhead by sharing ring buffers with the kernel.

This lesson builds all three: epoll, io_uring, and Tokio's reactor-based async runtime. You will run the same echo server workload on each and measure the difference.

## The Concept

### Reactor Pattern

A **Reactor** runs an event loop that monitors a set of file descriptors for readiness:

```
loop:
    ready_fds = poll(fds)          // block until at least one fd is ready
    for fd in ready_fds:
        if fd is readable:
            data = read(fd)         // YOU must issue the read
            process(data)
        if fd is writable:
            write(fd, response)     // YOU must issue the write
```

The reactor tells you *when* to do I/O. It does not do the I/O for you.

**Key characteristic:** Every I/O operation requires two interactions with the kernel: one to discover readiness (epoll_wait), and one to perform the I/O (read/write).

Platform implementations:

| OS | Mechanism | Type | Max fds | Scalability |
|----|-----------|------|---------|-------------|
| Linux | epoll | Reactor | Unlimited | O(1) — red-black tree + ready list |
| BSD/macOS | kqueue | Reactor | Unlimited | O(1) — filter-based event queue |
| Solaris | event ports | Reactor | Unlimited | O(1) |
| Windows | IOCP | Proactor | Unlimited | O(1) — completion-based |

### Proactor Pattern

A **Proactor** submits I/O operations to the kernel and receives notifications when they *complete*:

```
buffer = malloc(4096)
read(fd, buffer)                    // submit the read; returns immediately
loop:
    completion = get_completion()   // block until *any* submitted op finishes
    if completion.op == READ:
        process(completion.buffer)  // data is already in buffer
        read(fd, new_buffer)        // submit next read
    if completion.op == WRITE:
        free(completion.buffer)
```

The proactor tells you that I/O *has happened*. The data is already where you asked for it.

**Key characteristic:** One interaction with the kernel per I/O (submission). Completion arrives asynchronously. No separate readiness check.

### Square Peg, Round Hole

Think of it as a shape analogy:

- **Reactor (square):** The kernel says "this hole is ready for a square peg." You still have to pick up the square peg and insert it. If the notification says "ready to read," you still call `read()`.
- **Proactor (round):** The kernel says "the round peg is already in the hole." The I/O is done. Your buffer is filled. You just consume it.

The reactor gives you *readiness*. The proactor gives you *completion*.

### epoll in Detail

epoll operates on an **epoll instance** — a kernel data structure that holds:

1. **Interest list** — file descriptors you want to monitor (added via `epoll_ctl`).
2. **Ready list** — file descriptors that have events pending.

The interest list is stored as a **red-black tree** (O(log n) insert/remove). The ready list is a **doubly-linked list** (O(1) read). This is why epoll scales to millions of fds while `select()`/`poll()` degrade linearly.

Two **trigger modes**:

| Mode | Semantics | When to use |
|------|-----------|-------------|
| **Level-triggered (LT)** | epoll_wait returns as long as data is available. If you read 1 byte out of 100, it fires again. | Simpler code; less likely to miss events; default mode. |
| **Edge-triggered (ET)** | epoll_wait returns only when new data arrives. You must read until EAGAIN. | Higher throughput; fewer wakeups; requires non-blocking I/O and careful loops. |

### kqueue in Detail

kqueue uses a **filter** system. You register kevent structs with `kevent()`, specifying a filter:

| Filter | Purpose |
|--------|---------|
| `EVFILT_READ` | Socket or file is readable |
| `EVFILT_WRITE` | Socket or file is writable |
| `EVFILT_TIMER` | Timer expiration |
| `EVFILT_SIGNAL` | Signal delivery |
| `EVFILT_PROC` | Process events (fork/exec/exit) |

Unlike epoll's two-mode system, kqueue returns per-filter **data** and **fflags** that give fine-grained information (e.g., exactly how many bytes are readable without a separate `ioctl`).

### io_uring — The New Proactor for Linux

io_uring is Linux's modern async I/O interface, introduced in kernel 5.1. It uses **two shared ring buffers** between user space and the kernel:

```
Submission Queue (SQ)         Completion Queue (CQ)
┌─────────────────────┐       ┌─────────────────────┐
│ SQE 0: read(fd,buf) │──────→│ CQE 0: read done, 4K│
│ SQE 1: write(fd,..) │       │ CQE 1: write done   │
│ SQE 2: accept(...)  │       │ CQE 2: new fd=7     │
└─────────────────────┘       └─────────────────────┘
      ↑ user writes               ↑ kernel writes
      ↓ kernel reads              ↓ user reads
```

The **submission queue (SQ)** holds Submission Queue Entries (SQEs). User space writes an SQE specifying an operation (read, write, accept, openat, etc.), then advances the tail pointer. The kernel picks it up and begins the operation.

The **completion queue (CQ)** holds Completion Queue Events (CQEs). When the kernel finishes an operation, it writes the result (return value, buffer location) into the CQ and advances the tail. User space reads completed events.

**Key innovation:** No syscalls in the fast path. The only system call is `io_uring_enter()` (or `io_uring_submit()` in liburing), which is needed only when the kernel hasn't noticed new SQ entries yet. With `IORING_SETUP_SQPOLL`, even that syscall is eliminated — the kernel polls the SQ in the background.

| Feature | epoll | io_uring |
|---------|-------|----------|
| Type | Reactor | Proactor |
| Syscalls per I/O | 2 (epoll_wait + read/write) | 0-1 (submit batch, poll completions) |
| Supported ops | read/write readiness | read, write, accept, openat, statx, sendmsg, recvmsg, fallocate, etc. |
| Kernel min | 2.6 | 5.1 (5.19 for full feature set) |
| Buffer management | You manage buffers | Registered buffers for zero-copy |

## Build It

You will implement three echo servers and a benchmark harness. Open `code/main.c` for the epoll and io_uring servers, and `code/main.rs` for the Tokio-based server.

### Step 1: epoll Echo Server (C)

An epoll echo server follows this flow:

```
socket → bind → listen
epoll_create → epoll_ctl(ADD, listen_fd, EPOLLIN)
loop:
    n = epoll_wait(epfd, events, max_events, timeout)
    for each event:
        if fd == listen_fd → accept → epoll_ctl(ADD, client, EPOLLIN|EPOLLET)
        else → read(fd) → write(fd, buf, nread) → close on EOF or error
```

The C implementation in `main.c` (`run_epoll()`) does exactly this. Key points:

- Use `SOCK_NONBLOCK` on the listening socket. Edge-triggered mode requires non-blocking I/O.
- In edge-triggered mode, you must read until `EAGAIN` or you miss data.
- The loop re-arms with `EPOLL_CTL_MOD` after each read/write to catch the next event.

```c
// Core loop structure (simplified):
struct epoll_event events[128];
while (run) {
    int n = epoll_wait(epfd, events, 128, -1);
    for (int i = 0; i < n; i++) {
        handle_event(epfd, &events[i]);
    }
}
```

**File:** `main.c` — function `run_epoll()`.

### Step 2: io_uring Echo Server (C)

An io_uring echo server is a true proactor. You submit read operations and the kernel writes data directly into your buffers:

```
io_uring_queue_init(ENTRIES, &ring, 0)
socket → bind → listen
submit_accept(&ring, listen_fd)
loop:
    cqe = io_uring_wait_cqe(&ring, &cqe)   // wait for completion
    if cqe->res == listen_fd or accept:
        new_fd = cqe->res                   // accept completed
        submit_read(&ring, new_fd, buf)     // submit a read for this client
        submit_accept(&ring, listen_fd)     // submit next accept
    else:
        // read completed: cqe->res is bytes read
        if bytes_read == 0 → close
        else → submit_write(&ring, fd, buf, bytes_read)
    io_uring_cqe_seen(&ring, cqe)
```

The io_uring approach has zero syscalls for I/O in the fast path. You batch-submit SQEs, then wait on the CQ. The kernel performs reads and writes asynchronously.

Key io_uring operations used:
- `IORING_OP_ACCEPT` — submit an accept. Completion returns the new fd.
- `IORING_OP_READV` — submit a vectored read. Completion returns bytes read.
- `IORING_OP_WRITEV` — submit a vectored write. Completion returns bytes written.

**Multishot accept (kernel 5.19+):** With `IOSQE_BUFFER_SELECT`, you can submit a single accept SQE that handles multiple connections without re-submission. This example uses single-shot for clarity.

**File:** `main.c` — functions `run_io_uring()`, `submit_accept()`, `submit_read()`, `submit_write()`.

### Step 3: Tokio Async Echo Server (Rust)

Tokio is a Rust async runtime built on top of epoll (Linux), kqueue (macOS), or IOCP (Windows). It provides **reactor-based** async I/O — under the hood, Tokio's *reactor* thread uses epoll to check readiness, then wakes the appropriate task.

```rust
#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                match socket.read(&mut buf).await {
                    Ok(0) => return,        // EOF
                    Ok(n) => {
                        if socket.write_all(&buf[..n]).await.is_err() {
                            return;
                        }
                    }
                    Err(_) => return,
                }
            }
        });
    }
}
```

What happens inside Tokio:

1. `listener.accept().await` registers the listening socket with the reactor (epoll) and yields.
2. When epoll reports the socket is readable, the reactor wakes the task.
3. The task calls `accept()`, which returns immediately (the reactor already checked readiness).
4. Each spawned task gets its own socket. The reactor tracks all of them via epoll.

**Tokio vs raw epoll:** Tokio adds ~2-3 µs overhead per wakeup due to task scheduling. For workloads with long-lived connections doing large reads, the overhead is negligible. For millions of tiny reads per second, raw epoll or io_uring wins.

**File:** `main.rs` — function `run_tokio_echo()`.

### Step 4: Throughput Comparison

The `main()` function in both `main.c` and `main.rs` lets you choose which server to run. Compile and test:

```bash
# C — epoll echo server:
clang -std=c11 -O2 -o echo_epoll code/main.c -DEFOLL && ./echo_epoll

# C — io_uring echo server (Linux 5.1+):
clang -std=c11 -O2 -luring -o echo_uring code/main.c -DIO_URING && ./echo_uring

# Rust — tokio echo server:
cd code && cargo run --release

# In another terminal, benchmark with a simple client:
# (Or use the built-in benchmark_client helper)
for i in {1..4}; do (echo "hello" | nc localhost 8080) &; done; wait
```

Expected throughput (rough, hardware-dependent):

| Server | 1 conn | 100 conn | 1000 conn | Note |
|--------|--------|----------|-----------|------|
| epoll (C) | ~800K req/s | ~600K req/s | ~200K req/s | Degrades as epoll_wait scans ready list |
| io_uring (C) | ~900K req/s | ~800K req/s | ~600K req/s | Better batching; fewer syscalls |
| Tokio (Rust) | ~500K req/s | ~400K req/s | ~150K req/s | Task scheduling overhead; memory safety |

Real servers (nginx, Redis) use epoll or io_uring directly. Tokio is used in application-level services where productivity matters more than peak throughput.

## Use It

### Production epoll: nginx, Redis, libevent

- **nginx** uses epoll in edge-triggered mode. The event loop (`ngx_epoll_module.c`) processes thousands of connections per worker. Each worker is single-threaded; multiple workers scale across cores.
- **Redis** uses a simple epoll-based event loop (`ae.c`). Single-threaded, epoll handles everything: client connections, timer events, and periodic persistence.
- **libevent** and **libuv** are portable event loop libraries. libevent wraps epoll/kqueue/IOCP behind a unified API. libuv (used by Node.js) does the same. These libraries add **edge-triggered** semantics and per-fd callbacks.

### Production io_uring

- **qemu** uses io_uring for virtual disk I/O. The virtio-blk driver submits I/O requests via io_uring SQEs and processes completions from the CQ, achieving near-native disk performance.
- **Seastar** (ScyllaDB's framework) uses io_uring for its I/O subsystem. Combined with its share-nothing architecture, it achieves millions of IOPS per core.
- **liburing** is the recommended userspace library for io_uring. It provides `io_uring_queue_init()`, `io_uring_get_sqe()`, `io_uring_submit()`, `io_uring_wait_cqe()`, and `io_uring_cqe_seen()` — the same operations you implemented above, but with better ergonomics.

### Our vs Production

| Aspect | Our epoll server | nginx |
|--------|-----------------|-------|
| Event model | One-shot edge-triggered | Edge-triggered with event resampling |
| Accept | Single accept() per event | Accept mutex in multi-worker mode |
| Buffers | Stack-allocated 4K | Memory pool with size classification |
| Error handling | Minimal | Comprehensive with recovery |

| Aspect | Our io_uring server | Seastar |
|--------|--------------------|---------|
| Setup | Direct syscall | liburing |
| Buffer mgmt | Per-operation buffers | Registered buffers (IORING_REGISTER_BUFFERS) |
| SQ submission | One-at-a-time | Batch submission with SQ fill |
| Polling | CQ wait | SQPOLL for zero-syscall mode |

## Read the Source

- **nginx epoll module:** `src/os/unix/ngx_epoll_module.c` — the production implementation. Note `ngx_epoll_process_events()`, the use of `EPOLLRDHUP` for early connection close detection, and the `ngx_epoll_post_event()` for inter-worker event distribution.
- **Linux kernel io_uring:** `io_uring/io_uring.c` — the kernel-side implementation. The `io_submit_sqes()` function processes submission entries. The `io_complete_rw()` function writes completion events. Look at how SQ and CQ ring pointers are managed without locking.
- **Tokio reactor source:** `tokio/src/runtime/io.rs` and `tokio/src/io/driver/mod.rs` — Tokio's epoll-based reactor. The `Driver::poll()` method calls `epoll_wait()`, then distributes readiness events to registered wakers.
- **liburing:** `https://github.com/axboe/liburing` — the reference userspace library for io_uring. `src/queue.c` shows how SQ and CQ ring pointers are managed.
- **Redis event loop:** `src/ae.c` — a minimal but complete reactor implementation. The `aeProcessEvents()` function calls `aeApiPoll()` (which wraps epoll), then dispatches to file event and timer callbacks. One file, ~500 lines.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A three-way echo server benchmark** — epoll (C), io_uring (C), and Tokio (Rust). Drop it into a project to test event loop throughput at various concurrency levels. The `outputs/README.md` documents the build, run, and measurement instructions.

## Exercises

1. **Easy** — Run the epoll server and connect 10 concurrent clients using `nc`. Use `strace -e epoll_wait,read,write` to count syscalls per echo. Run the same test on io_uring with `strace -e io_uring_enter` — how many syscalls per operation does each require?

2. **Medium** — Modify the epoll server to use **level-triggered** instead of edge-triggered. Benchmark both modes at 100 concurrent connections. Which has fewer `epoll_wait` returns? Why? (Hint: think about how many times epoll fires for a partial read.)

3. **Hard** — Implement a Rust Tokio echo server that uses `tokio::io::AsyncReadExt::read_buf` with a **pool of pre-allocated buffers** instead of stack buffers. Compare allocation counts with `dhat` or `valgrind --tool=massif`. How much allocation overhead does the stack-buffer version avoid? (Write-up: 1 paragraph explaining the trade-off between stack allocation and memory pooling in an async context.)

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Reactor | "Event loop that tells you when I/O is ready" | A pattern where the event loop monitors fds and notifies readiness. You still issue the read/write syscall. Examples: epoll, kqueue. |
| Proactor | "Async I/O that completes in your buffer" | A pattern where you submit I/O operations and the kernel notifies on completion. The data is already in your buffer. Examples: IOCP, io_uring. |
| epoll | "Linux's scalable poll" | A reactor mechanism using a red-black tree (interest list) and doubly-linked list (ready list). O(1) readiness notification. Level-triggered or edge-triggered. |
| kqueue | "BSD's event notification" | A reactor mechanism using filter-based event queues. Supports fd events, timers, signals, and process events. More flexible than epoll. |
| io_uring | "Linux's async I/O with shared rings" | A proactor mechanism using submission queue (SQ) and completion queue (CQ) ring buffers shared between userspace and kernel. Zero syscall overhead in the fast path. |
| SQ | "Submission Queue" | The ring buffer where user space writes I/O operations (SQEs) for the kernel to execute. |
| CQ | "Completion Queue" | The ring buffer where the kernel writes results (CQEs) after completing I/O operations submitted via the SQ. |
| Event loop | "The infinite loop that drives async I/O" | A loop that repeatedly polls for events (or completions) and dispatches them to handlers. The core of both reactor and proactor patterns. |
| Non-blocking I/O | "I/O that returns immediately" | A file descriptor mode where read/write return -1 with EAGAIN instead of blocking. Required for reactor-based event loops. |
| Readiness | "The kernel says 'you may read now'" | A reactor notification indicating that a read syscall on a particular fd is likely to return data immediately (but is not guaranteed to). |
| Completion | "The kernel says 'your data is ready'" | A proactor notification indicating that a previously submitted I/O operation has finished and the data is in the specified buffer. |

## Further Reading

1. **Schmidt et al., "Pattern-Oriented Software Architecture, Vol. 2: Patterns for Concurrent and Networked Objects" (2000)** — The canonical description of Reactor and Proactor patterns. Chapter 3 covers Reactor (event demultiplexing + dispatch). Chapter 4 covers Proactor (asynchronous operation completion).

2. **Linux kernel io_uring documentation:** `Documentation/filesystems/io_uring.rst` — The kernel documentation for io_uring. Covers setup flags, supported operations, and advanced features (registered buffers, fixed files, IORING_SETUP_SQPOLL).

3. **liburing man pages:** `man 3 io_uring_queue_init`, `man 3 io_uring_get_sqe`, etc. — The official liburing API reference. Read `io_uring_submit()` for batch submission mechanics and `io_uring_wait_cqe()` for completion handling.

4. **Brendan Gregg's io_uring talk** — Performance analysis of io_uring vs epoll vs AIO for storage workloads. Shows io_uring beating epoll by 3-10x for random reads on NVMe. Available on YouTube.

5. **Tokio internals guide:** `https://tokio.rs/tokio/topics/bridging` — How Tokio bridges sync and async code. The "Runtime" section explains how Tokio's reactor drives I/O events and wakes tasks. The "Understanding Tokio" documentation shows the integration with epoll/kqueue.

6. **The `io_uring` syscall interface:** `man 2 io_uring_setup`, `man 2 io_uring_enter`, `man 2 io_uring_register` — The raw syscall interface. `io_uring_setup` initializes the SQ and CQ rings. `io_uring_enter` submits SQEs and/or waits for CQEs. `io_uring_register` registers pre-mapped buffers and files for zero-copy I/O.
