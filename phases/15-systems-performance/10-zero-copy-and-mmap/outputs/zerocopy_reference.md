# Zero-Copy & mmap Reference Card

## API Comparison

| Mechanism | Header | Zero-copy? | Direction | User-space data? | Best for |
|-----------|--------|-----------|-----------|-------------------|----------|
| `read`/`write` | `<unistd.h>` | No | Bidirectional | Yes | General-purpose I/O |
| `mmap` | `<sys/mman.h>` | Yes (read) | Bidirectional | No (direct access) | Random access, shared memory |
| `sendfile` | `<sys/sendfile.h>` | Yes | File → Socket | No | Static file serving |
| `splice` | `<fcntl.h>` | Yes | Pipe ↔ FD | No | Proxying, piping |
| `tee` | `<fcntl.h>` | Yes | Pipe → Pipe | No | Multicast, logging |
| `O_DIRECT` + `read` | `<fcntl.h>` | No* | Bidirectional | Yes | Self-managed caches |
| `MSG_ZEROCOPY` | `<sys/socket.h>` | Yes | User → Socket | No (pinned) | Large network sends |

\* `O_DIRECT` eliminates the page cache, not the kernel↔user copy.

## Function Signatures

```c
// Memory mapping
void *mmap(void *addr, size_t length, int prot, int flags, int fd, off_t offset);
int munmap(void *addr, size_t length);
int msync(void *addr, size_t length, int flags);

// File→socket transfer
ssize_t sendfile(int out_fd, int in_fd, off_t *offset, size_t count);

// Pipe-based zero-copy
long splice(int fd_in, off_t *off_in, int fd_out, off_t *off_out,
            size_t len, unsigned int flags);
long tee(int fd_in, int fd_out, size_t len, unsigned int flags);

// Zero-copy network send (Linux 4.14+)
ssize_t sendmsg(int sockfd, const struct msadr *msg, int flags);
// Use: setsockopt(sockfd, SOL_SOCKET, SO_ZEROCOPY, &one, sizeof(one));
// Then: sendmsg(sockfd, &msg, MSG_ZEROCOPY);
// Completion: recvmsg(sockfd, MSG_ERRQUEUE) to get notification

// File advice (optimize mmap)
int madvise(void *addr, size_t length, int advice);
// MADV_SEQUENTIAL, MADV_RANDOM, MADV_WILLNEED, MADV_DONTNEED

// Huge pages
void *addr = mmap(NULL, len, PROT_READ | PROT_WRITE,
                  MAP_PRIVATE | MAP_ANONYMOUS | MAP_HUGETLB, -1, 0);
// Or explicit size:
//   MAP_HUGETLB | MAP_HUGE_2MB
//   MAP_HUGETLB | MAP_HUGE_1GB
```

## mmap Flags

| Flag | Effect | Use when |
|------|--------|----------|
| `MAP_PRIVATE` | Writes create COW copies; file unchanged | Executables, config files, temporary mappings |
| `MAP_SHARED` | Writes modify the file (visible to other processes) | Shared memory IPC, persistent data, inter-process sync |
| `MAP_ANONYMOUS` | No file backing; zero-filled | malloc implementations, private allocations |
| `MAP_HUGETLB` | Use huge pages (2 MB default) | Large long-lived allocations |
| `MAP_POPULATE` | Pre-fault all pages at mmap time | Known sequential access, avoiding runtime faults |
| `MAP_FIXED` | Map at exact address (dangerous) | Reproducible layouts, JIT compilers |

## O_DIRECT Requirements

1. Buffer must be aligned to logical block size (use `posix_memalign` or `aligned_alloc`).
2. File offset must be aligned to logical block size.
3. Read/write length must be a multiple of logical block size.
4. Check: `posix_memalign(&buf, 4096, read_size)`.
5. Not guaranteed on all OS versions; check kernel docs.

## When to Use What

```
Need: Read a file sequentially once
  → read() with large buffer (64 KB+). Simpler than mmap.

Need: Random access to a file that fits in RAM
  → mmap() with MAP_PRIVATE. Direct access, no copies.

Need: Random access to a file larger than RAM
  → read() with explicit buffering. mmap page faults are unpredictable.

Need: Send file over network
  → sendfile(). Zero-copy if kernel and NIC support it.

Need: Proxy data between two sockets
  → splice() through a pipe pair. Data never enters user space.

Need: Share memory between processes
  → mmap() with MAP_SHARED. Fastest IPC for large data.

Need: Process copy of shared data (fork)
  → Rely on COW from fork(). Only modified pages consume memory.

Need: Large allocation with low TLB pressure
  → mmap() with MAP_HUGETLB (after pre-allocating huge pages).

Need: Database with own buffer management
  → O_DIRECT + read(). Bypass page cache, avoid double-caching.

Need: Send large buffers over network
  → MSG_ZEROCOPY. Only beneficial for sends > ~10 KB.
```

## Common Pitfalls

### mmap on Truncated Files → SIGBUS
If a file is truncated after mmap, accessing pages beyond the new file size causes SIGBUS.
**Fix:** Use `flock(LOCK_SH)` or check `fstat` before access.

### COW Explosion with MAP_PRIVATE Writes
Writing to every page in a MAP_PRIVATE mapping uses as much physical memory as the original.
**Fix:** Only use MAP_PRIVATE when write ratio is low; use MAP_SHARED for in-place updates.

### Huge Page Pool Exhaustion
`mmap` with `MAP_HUGETLB` fails with `ENOMEM` if the pool is empty.
**Fix:** Pre-allocate pool: `echo N > /proc/sys/vm/nr_hugepages`. Check `/proc/meminfo`.

### MSG_ZEROCOPY Buffer Corruption
Modifying a buffer before the NIC finishes DMA corrupts the send.
**Fix:** Always `recvmsg(MSG_ERRQUEUE)` to get completion before modifying the buffer.

### O_DIRECT Alignment Violations
Misaligned buffers or offsets cause `EINVAL`.
**Fix:** Always `posix_memalign(&buf, 4096, size)` and align file offsets.

### Page Fault Storm on Cold mmap
Touching a large mmap all at once triggers thousands of page faults in sequence.
**Fix:** Use `madvise(MADV_SEQUENTIAL)` or `MAP_POPULATE` for known access patterns.

## Data Path Comparison

```
Traditional: read() + write()
  disk → [kernel page cache] → memcpy → [user buffer] → memcpy → [kernel socket buf] → NIC
          ───── copy 1 ───────────────── ───── copy 2 ──────────────────

sendfile():
  disk → [kernel page cache] → [kernel socket buf] → NIC
          ── same page, no user copy ── ──── copy 1 ────

sendfile() + DMA scatter-gather:
  disk → [kernel page cache] → NIC (DMA points to page cache pages)
          ─────────── zero copies ──────────

mmap() + write():
  disk → [kernel page cache] ← (mmap points here)
          read access: zero copies
          write(fd, addr+off, len): one copy to kernel socket buf

splice():
  [kernel page cache] → [pipe buffer] → [socket buffer] → NIC
    ── page ref only ── ── page ref only ── ── copy ──
```