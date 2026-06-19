# Lesson 13: I/O Architecture — Block, Char, syscalls, vfs

## The Problem

Every program must interact with the outside world — disks, networks, keyboards, displays. Without a uniform abstraction, each program would need device-specific code for every piece of hardware. The OS provides a layered I/O stack that hides hardware complexity behind a clean file-descriptor interface.

## Device Abstraction

### Block Devices

Block devices store data in fixed-size sectors (typically 512 bytes or 4 KiB). Disks, SSDs, and USB mass-storage devices are block devices. The kernel exposes them as arrays of numbered blocks. Reads and writes operate on whole sectors. The block layer adds buffering, scheduling, and caching (the page cache) to optimize access patterns.

```
User: read(fd, buf, 4096)
  → VFS → block layer → I/O scheduler → device driver → disk controller
  ← sector data ← DMA ← controller ← driver ← block layer ← VFS
```

### Character Devices

Character devices produce or consume an unbounded byte stream — keyboards, serial ports, `/dev/null`, pseudo-terminals. There is no concept of "sector" or "block number." Each `read()` or `read()` call transfers whatever bytes are available (or blocks until some arrive).

```
User: read(fd, buf, 16)
  → VFS → tty driver → keyboard controller
  ← keystroke bytes ← interrupt ← driver ← VFS
```

### Other Device Classes

Modern systems also expose network devices (not addressable via `read`/`write` — they use sockets), memory-mapped devices, and pseudo-devices (`/dev/urandom`, `/dev/zero`).

## File Descriptors and the File Table

When a process calls `open("/dev/sda1", O_RDONLY)`, the kernel:

1. Looks up the path via VFS → finds the inode/vnode for the device.
2. Allocates a **file table entry** — records the current offset, access mode, and a pointer to the vnode.
3. Installs the file table entry into the process's **file descriptor table** and returns the index — the file descriptor `fd`.

```
Process FD table          File table             Inode/vnode table
┌────┬──────────┐    ┌──────────────────┐    ┌──────────────────┐
│  0 │ → stdin  │    │ offset: 0        │    │ type: char       │
│  1 │ → stdout │    │ mode: O_RDONLY   │──→ │ device: tty0     │
│  2 │ → stderr │    │ vnode: ──────────│    │ ops: tty_fops    │
│  3 │ →────────│──→ │ offset: 1024     │──→ │ type: block      │
│  4 │ →────────│──→ │ offset: 0        │──→ │ type: char       │
└────┴──────────┘    │ mode: O_WRONLY   │    │ device: /dev/null│
                      └──────────────────┘    └──────────────────┘
```

Multiple file descriptors (even across different processes) can point to the same file table entry — this is how `dup2()` and `fork()` work.

## The VFS Layer

The Virtual File System is a dispatch layer. It defines a uniform interface (`struct file_operations` in Linux) with methods like `read`, `write`, `open`, `close`, `ioctl`. Every filesystem and device driver implements this interface. The VFS routes every syscall to the correct implementation:

```
  read(fd, buf, n)
       │
       ▼
  ┌─────────┐
  │  VFS    │  fd → file table → vnode → file_operations
  └────┬────┘
       │ .read()
       ├──────────→ ext4_read()     (filesystem on disk)
       ├──────────→ tty_read()      (character device)
       └──────────→ pipe_read()     (pipe)
```

This is the essence of **polymorphism in the kernel** — one interface, many implementations.

## System Calls

The user↔kernel boundary is crossed via system calls. On x86-64 Linux:

| Syscall | Purpose |
|---------|---------|
| `open(path, flags, mode)` | Open file, return fd |
| `read(fd, buf, count)` | Read bytes from fd |
| `write(fd, buf, count)` | Write bytes to fd |
| `close(fd)` | Release fd |
| `lseek(fd, offset, whence)` | Reposition file offset |
| `ioctl(fd, req, ...)` | Device-specific control |
| `select(n, rfds, wfds, efds, timeout)` | Multiplexed I/O |
| `poll(fds, nfds, timeout)` | Multiplexed I/O (scale-friendly) |
| `epoll_create1(flags)` | Scalable event notification (Linux) |

Each syscall traps into kernel mode, validates arguments, dispatches through VFS, and returns.

## Buffered vs Unbuffered I/O

**Unbuffered** (syscall-level): `read()` / `write()` go directly to the kernel. Each call is a syscall — expensive if done in small chunks.

**Buffered** (C stdio): `fread()` / `fwrite()` accumulate data in a user-space buffer. The library issues fewer syscalls. `setvbuf()` controls buffering mode:
- `_IOFBF` — full buffering (flush on buffer full or `fflush()`)
- `_IOLBF` — line buffering (flush on newline)
- `_IONBF` — no buffering (each call → syscall)

## Multiplexed I/O

A server with 10,000 connections cannot call `read()` on each fd sequentially. Multiplexing lets one thread wait on many fds:

- **`select()`**: bitmap of fds, O(n) scan, limited to `FD_SETSIZE` (typically 1024).
- **`poll()`**: array of `struct pollfd`, no fixed limit, still O(n) scan.
- **`epoll`** (Linux): kernel-maintained event list, O(1) notification. The application registers interest; the kernel returns only ready fds.

```
epoll_create1() → epoll_fd
epoll_ctl(epoll_fd, EPOLL_CTL_ADD, client_fd, &event)
while (1) {
    n = epoll_wait(epoll_fd, events, MAX, timeout);
    for (i = 0; i < n; i++) handle(events[i].data.fd);
}
```

## Asynchronous I/O

True async I/O lets the kernel perform the operation while the application continues. On Linux:
- **`io_uring`** (Linux 5.1+): shared ring buffers between kernel and user space. Submission Queue (SQ) and Completion Queue (CQ) avoid syscall overhead entirely for batched operations.
- **POSIX AIO** (`aio_read`, `aio_write`): older, limited; uses thread pool internally on many implementations.

## Build It: Device Simulation + File Descriptor Demo

The code simulates the I/O stack in userspace:
1. A simulated block device with sector-based read/write.
2. A simulated character device with byte-stream read/write.
3. A file descriptor table mapping fds to device operations.
4. A VFS dispatch layer routing calls to the correct device.
5. A `select()` simulation watching multiple fds simultaneously.

## Use It

Every file operation you perform — `cat file.txt`, writing to a socket, reading from a pipe — traverses this exact stack: syscall → VFS → device driver → hardware. Understanding this stack is essential for systems programming, performance tuning, and writing device drivers.

## Ship It

The I/O architecture demo shows that the same `read(fd, buf, n)` call can route to entirely different backends. This is the power of VFS — it is one of the most important abstractions in any operating system.

## Exercises

**Level 1 — Trace the Stack:**
Write a C program that opens `/dev/null`, writes data to it, and reads from `/dev/zero`. For each operation, print the fd number and bytes transferred. Confirm that the same `write()` interface works for both.

**Level 2 — Implement a Simple VFS:**
Extend the demo code to add a third device type: a "memory device" that stores bytes in a dynamically allocated buffer. Register it with the VFS and verify that `read()` and `write()` work through the same dispatch layer.

**Level 3 — Event Loop with select():**
Write a chat server using `select()` (or `poll()`). The server should accept multiple client connections, read messages from any client, and broadcast them to all others. Trace which syscall path each operation takes.
