# io_uring Reference Card

## API Overview

### Setup / Teardown

| Function | Purpose |
|----------|---------|
| `io_uring_queue_init(depth, &ring, flags)` | Allocate and mmap shared ring buffers |
| `io_uring_queue_exit(&ring)` | Unmap and free ring buffers |
| `io_uring_setup(entries, &params)` | Raw syscall — use via liburing unless you need params |

### Submission (SQE)

| Helper | Opcode | What it does |
|--------|--------|-------------|
| `io_uring_prep_read(sqe, fd, buf, len, off)` | READ | Read from file at offset |
| `io_uring_prep_write(sqe, fd, buf, len, off)` | WRITE | Write to file at offset |
| `io_uring_prep_accept(sqe, fd, addr, addrlen, flags)` | ACCEPT | Accept a connection |
| `io_uring_prep_connect(sqe, fd, addr, addrlen)` | CONNECT | Initiate a connection |
| `io_uring_prep_recv(sqe, fd, buf, len, flags)` | RECV | Receive from socket |
| `io_uring_prep_send(sqe, fd, buf, len, flags)` | SEND | Send to socket |
| `io_uring_prep_fsync(sqe, fd, flags)` | FSYNC | Sync file data/metadata |
| `io_uring_prep_poll_add(sqe, fd, poll_mask)` | POLL_ADD | Monitor fd for events |
| `io_uring_prep_timeout(sqe, &ts, count, flags)` | TIMEOUT | Fire CQE after timeout |
| `io_uring_prep_link_timeout(sqe, &ts, flags)` | LINK_TIMEOUT | Timeout for previous linked SQE |

### Completion (CQE)

| Field | Meaning |
|-------|---------|
| `cqe->user_data` | Cookie copied from corresponding SQE |
| `cqe->res` | Result: positive = bytes transferred, negative = `-errno` |
| `cqe->flags` | Extra flags (e.g. `IORING_CQE_F_BUFFER` for buffer select) |

### Registration (zero-overhead paths)

| Function | What it registers |
|----------|-------------------|
| `io_uring_register_buffers(&ring, &iovec, nr)` | Fixed buffers — skip per-op page pinning |
| `io_uring_unregister_buffers(&ring)` | Unregister fixed buffers |
| `io_uring_register_files(&ring, &fds, nr)` | Fixed files — skip per-op fget() |
| `io_uring_unregister_files(&ring)` | Unregister fixed files |

## Setup Steps (C with liburing)

```c
#include <liburing.h>

struct io_uring ring;
/* 1. Init ring with depth and optional flags */
io_uring_queue_init(256, &ring, 0);

/* 2. (Optional) Register fixed files */
int fds[1] = { my_fd };
io_uring_register_files(&ring, fds, 1);

/* 3. (Optional) Register fixed buffers */
struct iovec iov = { .iov_base = buf, .iov_len = BUFSZ };
io_uring_register_buffers(&ring, &iov, 1);

/* ... submit and complete ... */

/* 4. Cleanup */
io_uring_unregister_buffers(&ring);
io_uring_unregister_files(&ring);
io_uring_queue_exit(&ring);
```

## Common Patterns

### Batch Submit

```c
for (int i = 0; i < N; i++) {
    struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
    io_uring_prep_read(sqe, fd, bufs[i], BUFSZ, offsets[i]);
    io_uring_sqe_set_data64(sqe, i);
}
int submitted = io_uring_submit(&ring);   /* single syscall */
```

### Harvest Completions

```c
for (int i = 0; i < submitted; i++) {
    struct io_uring_cqe *cqe;
    io_uring_wait_cqe(&ring, &cqe);  /* or io_uring_peek_cqe for non-blocking */
    process(cqe->user_data, cqe->res);
    io_uring_cqe_seen(&ring, cqe);   /* advance CQ head */
}
```

### Linked Timeout (abort I/O after N seconds)

```c
struct __kernel_timespec ts = { .tv_sec = 5, .tv_nsec = 0 };

struct io_uring_sqe *io_sqe = io_uring_get_sqe(&ring);
io_uring_prep_read(io_sqe, fd, buf, len, off);
io_sqe->flags |= IOSQE_IO_LINK;

struct io_uring_sqe *tmo_sqe = io_uring_get_sqe(&ring);
io_uring_prep_link_timeout(tmo_sqe, &ts, 0);
/* NO IOSQE_IO_LINK on tmo_sqe — timeout is a separate chain end */
```

### Multishot Accept (one SQE for all connections)

```c
struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
io_uring_prep_multishot_accept(sqe, listen_fd, &addr, &addrlen, 0);
io_uring_sqe_set_data64(sqe, 0xACC);
io_uring_submit(&ring);

/* Each new connection produces one CQE — no re-submit needed */
while (1) {
    struct io_uring_cqe *cqe;
    io_uring_wait_cqe(&ring, &cqe);
    int new_fd = cqe->res;
    handle_client(new_fd);
    io_uring_cqe_seen(&ring, cqe);
}
```

### Fixed Buffer + Fixed File Read

```c
struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
io_uring_prep_read_fixed(sqe, 0, buf, BUFSZ, off, 0);
sqe->flags |= IOSQE_FIXED_FILE;
/* fd=0 means fixed_files[0], buf_index=0 means registered_buffers[0] */
```

## Rust (io_uring crate)

```rust
use io_uring::{opcode, types, IoUring};

let mut ring = IoUring::new(256)?;
let fd = types::Fd(file.as_raw_fd());

let read_e = opcode::Read::new(fd, buf.as_mut_ptr(), len as u32)
    .offset(off)
    .build()
    .user_data(42);

unsafe { ring.submission().push(&read_e)?; }
ring.submit()?;

let cqe = ring.completion().next().unwrap();
assert_eq!(cqe.user_data(), 42);
let bytes_read = cqe.result() as usize;
```

## When to Use io_uring vs epoll

| Use io_uring | Use epoll |
|-------------|-----------|
| High-volume file I/O (>50K ops/s) | Portable code (macOS, BSD) |
| Thread-per-core storage engine | Simple event loop with few connections |
| Batched network I/O | EPOLLEXCLUSIVE multi-thread accept |
| Zero-syscall path (SQPOLL) | Low I/O rate where setup cost dominates |
| Multishot accept for connection storms | Already-working epoll codebase |

## Key Flags

| Flag | Level | Effect |
|------|-------|--------|
| `IORING_SETUP_SQPOLL` | Ring init | Kernel thread polls SQ ring — no submit syscall |
| `IORING_SETUP_IOPOLL` | Ring init | Kernel busy-polls for completions |
| `IOSQE_FIXED_FILE` | Per-SQE | Use registered file index instead of fd |
| `IOSQE_IO_LINK` | Per-SQE | Chain this SQE to the previous one |
| `IOSQE_BUFFER_SELECT` | Per-SQE | Select from a registered buffer group |