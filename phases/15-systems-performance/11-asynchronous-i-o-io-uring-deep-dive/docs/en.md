# Asynchronous I/O — io_uring Deep Dive

> io_uring is the most significant Linux kernel interface since epoll — a shared-memory ring buffer that lets you submit and complete I/O without ever entering the kernel.

**Type:** Learn
**Languages:** C, Rust
**Prerequisites:** Phase 15 lessons 01–10 (especially lesson 05 — event-driven I/O with epoll)
**Time:** ~90 minutes

## Learning Objectives

- Explain why synchronous I/O blocks and how that hurts throughput at scale.
- Describe the Linux AIO interface's limitations that motivated io_uring.
- Diagram the shared-memory ring buffer design: submission queue, completion queue, SQE, CQE.
- Implement file and network I/O with liburing and raw io_uring syscalls.
- Contrast polling (busy-wait) vs interrupt-driven completion and know when to use each.
- Use fixed buffers, fixed files, linked timeouts, and multishot accept for zero-overhead I/O.
- Decide when io_uring beats epoll and vice versa, backed by benchmark numbers.

## The Problem

Every read() or write() on a file descriptor is a syscall. A syscall costs ~100–300 ns on modern x86: the kernel transitions from user space to kernel space, validates arguments, performs the I/O, copies data, and returns. At 1 million I/O operations per second, that is 0.1–0.3 seconds spent *just on syscall overhead* — before the device even touches the data.

Synchronous (blocking) I/O makes this worse: your thread sleeps until the kernel finishes every operation. You can't pipeline. You can't overlap I/O with computation. The thread that called `read()` is frozen, even if it has other work to do.

**Linux AIO** (`io_submit`, `io_getevents`) was the first attempt at async I/O on Linux. It failed in practice:

1. **Only O_DIRECT works asynchronously.** Buffered reads still block the calling thread inside `io_submit` — the exact thing async I/O is supposed to avoid.
2. **Three syscalls per batch.** `io_setup` (once), then `io_submit` + `io_getevents` for every batch. You still pay the kernel transition cost.
3. **No network I/O.** Linux AIO only handles file I/O. Sockets are excluded.
4. **Clunky API.** A 208-byte `iocb` struct per operation, and you must manage an `aio_context_t` that doesn't integrate with epoll.

The kernel community needed something better. Jens Axboe (the block layer maintainer) designed io_uring and merged it in Linux 5.1 (2019).

## The Concept

### Ring Buffer Design

io_uring's insight: **share memory between user and kernel, so neither has to copy data or make syscalls just to communicate.**

```
+-------------------+     shared memory region     +-------------------+
|                   | <=========================> |                   |
|    USER SPACE     |                               |   KERNEL SPACE    |
|                   |                               |                   |
|  submission queue|  ---- SQEs flow right --->   |  reads SQEs,     |
|  (SQ ring)       |                               |  processes I/O   |
|                   |                               |                   |
|  completion queue|  <--- CQEs flow left -----   |  writes CQEs      |
|  (CQ ring)       |                               |                   |
+-------------------+                               +-------------------+
```

There are two ring buffers in a single shared memory region:

- **Submission Queue (SQ)** — an array of `io_uring_sqe` entries. The user app writes SQEs here to request I/O.
- **Completion Queue (CQ)** — an array of `io_uring_cqe` entries. The kernel writes CQEs here when I/O finishes.

Both rings are circular. Head and tail indices advance monotonically; the ring size is a power of 2, so `index % ring_size` wraps.

### SQE — Submission Queue Entry

```c
struct io_uring_sqe {
    __u8  opcode;        /* e.g. IORING_OP_READ, IORING_OP_WRITE */
    __u8  flags;         /* IOSQE_FIXED_FILE, IOSQE_IO_LINK, ... */
    __u16 ioprio;
    __s32 fd;            /* file descriptor (or fixed file index) */
    __u64 off;           /* offset for read/write */
    __u64 addr;          /* buffer address or remote address for send/recv */
    __u32 len;           /* buffer length */
    union {
        __kernel_rwf_t rw_flags;
        __u32 fsync_flags;
        __u16 poll_events;
        __u32 sync_range_flags;
        __u32 msg_flags;
    };
    __u64 user_data;     /* copied to CQE — identifies which request completed */
    __u16 buf_index;     /* fixed-buffer index, if using IOSQE_BUFFER_SELECT */
    __u16 personality;
    union { __u32 splice_fd_in; __u32 file_index; };
    __u64 __pad2[1];
};
```

Key fields: `opcode` tells the kernel *what* to do, `fd`/`off`/`addr`/`len` tell it *where* and *how much*, and `user_data` is an opaque cookie that flows into the completion so you can match requests to responses.

### CQE — Completion Queue Entry

```c
struct io_uring_cqe {
    __u64 user_data;   /* copied from the SQE — matches the request */
    __s32 res;         /* result: bytes transferred, or negative errno */
    __u32 flags;       /* CQE flags (e.g. IORING_CQE_F_BUFFER) */
};
```

Tiny. 16 bytes. The kernel writes one CQE per completed operation. `res` holds the return value (positive = bytes, negative = `-errno`). `user_data` is the same value you put in the SQE, so you don't need a lookup table.

### How io_uring Avoids Syscalls

The classic flow with liburing:

```c
io_uring_submit(&ring);   /* only syscall: flush SQ ring tail to kernel */
io_uring_wait_cqe(&ring, &cqe);  /* only if you block; can poll instead */
```

- If you batch 100 SQEs and then call `io_uring_submit()` once, that's **one syscall for 100 I/O operations**.
- If the kernel is already polling the SQ ring (with `IORING_SETUP_SQPOLL`), even `io_uring_submit()` is a no-op — the kernel thread reads the ring from shared memory with zero syscalls.
- CQE consumption never requires a syscall; you read from shared memory.

**Comparison:**

| Interface            | Syscalls per 100 ops | Blocks on buffered I/O? | Supports sockets? |
|----------------------|---------------------|--------------------------|-------------------|
| read/write           | 100                 | Yes                      | Yes               |
| Linux AIO            | 2 (submit + get)    | Yes (for buffered)       | No                |
| io_uring (default)  | 1 (submit)          | No                       | Yes               |
| io_uring (SQPOLL)   | 0                   | No                       | Yes               |

## Build It

### Step 1: Minimal File Read with liburing

```c
#include <liburing.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

int main(int argc, char *argv[]) {
    const char *path = argc > 1 ? argv[1] : "/etc/hostname";
    struct io_uring ring;
    int fd, ret;

    fd = open(path, O_RDONLY);
    if (fd < 0) { perror("open"); return 1; }

    /* 1. Setup: allocate shared ring with 1 entry */
    ret = io_uring_queue_init(1, &ring, 0);
    if (ret < 0) { fprintf(stderr, "queue_init: %s\n", strerror(-ret)); return 1; }

    /* 2. Prepare: get an SQE and fill it */
    char buf[4096];
    struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
    io_uring_prep_read(sqe, fd, buf, sizeof(buf), 0);
    io_uring_sqe_set_data(sqe, (void *)0xDEADBEEF); /* user_data cookie */

    /* 3. Submit: one syscall to flush SQ ring */
    ret = io_uring_submit(&ring);
    if (ret < 0) { fprintf(stderr, "submit: %s\n", strerror(-ret)); return 1; }

    /* 4. Complete: read CQE from shared memory */
    struct io_uring_cqe *cqe;
    ret = io_uring_wait_cqe(&ring, &cqe);
    if (ret < 0) { fprintf(stderr, "wait_cqe: %s\n", strerror(-ret)); return 1; }

    printf("user_data=0x%lx res=%d\n", (unsigned long)cqe->user_data, cqe->res);
    if (cqe->res > 0) fwrite(buf, 1, cqe->res, stdout);

    io_uring_cqe_seen(&ring, cqe);
    io_uring_queue_exit(&ring);
    close(fd);
    return 0;
}
```

### Step 2: Batch I/O — Sequential vs io_uring Benchmark

The real advantage appears when you batch many operations. This program reads 128 blocks from a file twice: once with sequential `read()` calls, once with a single `io_uring_submit()`, and reports elapsed time for both.

See `code/main.c` for the full benchmark.

### Step 3: Network I/O — Multishot Accept

io_uring supports socket operations natively. The `IORING_OP_ACCEPT` opcode submits an `accept4()`-equivalent request. With `IOSQE_IO_LINK`, you can chain accept → read → write in a single submission batch.

Even better: `IORING_OP_MULTISHOT_ACCEPT` (Linux 5.19+) re-arms automatically. One SQE accepts *all* incoming connections, delivering one CQE per new socket — no need to re-submit.

### Step 4: Polling vs Interrupt-Driven Completion

By default, io_uring uses **interrupt-driven** completion: the kernel raises a hardware interrupt when the device finishes I/O, and the interrupt handler writes the CQE. This is optimal for slow devices (disks, networks) where busy-waiting wastes CPU.

For ultra-low-latency devices (NVMe with μs-scale latency, or kernel-side polling), enable **hybrid polling**:

```c
io_uring_prep_read(sqe, fd, buf, len, offset);
sqe->flags |= IOSQE_IO_LINK;
/* or set IORING_SETUP_IOPOLL on ring init for all ops */
```

`IORING_SETUP_IOPOLL` tells the kernel to busy-poll for completions instead of waiting for interrupts. This trades CPU time for latency — use it only when I/O latency dominates compute time.

### Step 5: Fixed Buffers and Fixed Files

Every SQE includes a raw pointer and fd. The kernel must validate both on every operation. io_uring lets you **pre-register** files and buffers to skip this cost:

- **Fixed files** (`IORING_OP_FILES_UPDATE`, `IORING_REGISTER_FILES`): Register up to 1024 file descriptors. SQEs reference them by index instead of fd number. Skips `fget()` lookup.
- **Fixed buffers** (`IORING_REGISTER_BUFFERS`): Register an array of `{addr, len}` pairs. SQEs reference them by index. The kernel pins the pages and skips page-table walks.

```c
struct iovec iov = { .iov_base = buf, .iov_len = BUFSZ };
io_uring_register_buffers(&ring, &iov, 1);
/* Now SQEs can use IOSQE_FIXED_BUFFER and buf_index=0 */
```

### Step 6: Linked Timeouts

Want a timeout on an I/O operation? Chain an `IORING_OP_LINK_TIMEOUT` SQE after the real operation using `IOSQE_IO_LINK`. If the first operation doesn't complete within the timeout, the kernel cancels it and delivers a CQE with `-ETIME`.

```c
sqe1 = io_uring_get_sqe(&ring);
io_uring_prep_read(sqe1, fd, buf, len, off);
sqe1->flags |= IOSQE_IO_LINK;

sqe2 = io_uring_get_sqe(&ring);
io_uring_prep_link_timeout(sqe2, &ts, 0);
/* No IOSQE_IO_LINK on sqe2 — timeout is not linked to further ops */
```

## Use It

### Linux Kernel Source

The io_uring implementation lives in `io_uring/` at the root of the kernel tree:

- `io_uring/io_uring.c` — core ring setup, submit, completion, and syscall handlers.
- `io_uring/opdef.c` — opcode table mapping `IORING_OP_*` to handlers.
- `io_uring/rw.c` — read/write handling, including fixed-buffer fast paths.
- `io_uring/net.c` — accept, connect, send, recv, and multishot accept.

Trace a `IORING_OP_READ` from user SQE → `io_uring_issue_sqe()` → `io_async_read()` → `iter_iovec()` → `file->f_op->read_iter()` → block driver.

### Production Libraries

- **liburing** — the official userspace library. Handles ring setup, SQE preparation helpers, and CQE consumption. See https://github.com/axboe/liburing
- **tokio-uring** (Rust) — async runtime built on io_uring. Uses `tokio`'s `Future` trait but backs it with ring buffers, not epoll.
- **Glommio** (Rust) — thread-per-core runtime using io_uring, optimized for storage workloads.

## When io_uring Beats epoll

| Scenario | Winner | Why |
|----------|--------|-----|
| High-volume file I/O (100k+ ops/s) | io_uring | epoll doesn't support file fd readiness; io_uring batches submit |
| Many small socket reads | io_uring | Batching SQEs amortizes syscall cost |
| Thread-per-core with shared epoll | epoll | io_uring rings are per-thread; epoll `EPOLLEXCLUSIVE` spreads wakeups |
| Need portable code (macOS, BSD) | epoll/kqueue | io_uring is Linux-only |
| Latency-sensitive with few connections | epoll | Less setup overhead; per-request overhead doesn't matter at low rate |
| Storage engine (RocksDB-type) | io_uring | Fixed buffers + fixed files + SQPOLL = near-zero syscall overhead |

**Rule of thumb:** If you're doing >50K I/O ops/sec on Linux, io_uring wins. Below that, the complexity cost may not justify switching from epoll.

## Benchmarks

Typical results on x86_64 with NVMe SSD (4K random reads, QD=128):

| Method | IOPS | Avg latency | Syscalls/sec |
|--------|------|-------------|--------------|
| synchronous read() | ~450K | 280 μs | 450K |
| Linux AIO (O_DIRECT) | ~800K | 160 μs | ~1.6M (submit+get) |
| io_uring (default) | ~1.1M | 115 μs | 1.1M |
| io_uring (SQPOLL) | ~1.2M | 105 μs | ~0 |

On network I/O (TCP echo server, single core):

| Method | Requests/sec | P99 latency |
|--------|-------------|-------------|
| epoll (blocking) | ~350K | 180 μs |
| io_uring (batched) | ~480K | 120 μs |
| io_uring (SQPOLL + multishot) | ~560K | 95 μs |

Numbers are illustrative; actual results depend on hardware, kernel version, and tuning.

## Exercises

1. **Easy** — Modify the minimal example to write data to a file instead of reading. Use `io_uring_prep_write`.
2. **Medium** — Implement a file copy program that reads from one fd and writes to another using linked SQEs (read → write via `IOSQE_IO_LINK`). Measure throughput versus `cp`.
3. **Hard** — Build a multishot-accept TCP echo server that handles 10K concurrent connections on a single core. Compare against an epoll-based echo server.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| SQE | "submission entry" | A 64-byte struct the user writes into the shared ring to request one I/O operation |
| CQE | "completion entry" | A 16-byte struct the kernel writes into the shared ring to report one I/O result |
| SQPOLL | "kernel polling" | A kernel thread that busy-polls the SQ ring, so the app never makes a submit syscall |
| Fixed buffer | "registered buffer" | A pre-registered iovec whose pages are pinned in the kernel, skipping per-op page-table walks |
| Fixed file | "registered file" | A pre-registered fd replaced by a table index, skipping per-op fget() lookup |
| Multishot accept | "persistent accept" | A single SQE that re-arms itself after each completion, delivering one CQE per connection |
| Linked timeout | "timeout on a chain" | An SQE linked after another; if the first doesn't complete in time, the kernel cancels it |

## Further Reading

- Jens Axboe's io_uring introduction: https://kernel.dk/io_uring.pdf
- Linux kernel source `io_uring/` directory
- liburing repository and man pages: https://github.com/axboe/liburing
- Lord of the io_uring (tutorial): https://unixism.net/loti/
- LWN io_uring series: https://lwn.net/Articles/810414/
- tokio-uring crate: https://github.com/tokio-rs/tokio-uring