# Zero-Copy and mmap

> How to avoid the single most expensive operation in computing: copying data.

**Type:** Learn
**Languages:** C
**Prerequisites:** Phase 15 lessons 01–09
**Time:** ~60 minutes

## Learning Objectives

- Understand why data copying between kernel and user space is a major performance bottleneck.
- Use `mmap` to map files directly into process address space, eliminating kernel→user copies.
- Know when `mmap` is faster *and* when it's slower than `read`/`write`.
- Use `sendfile`, `splice`, and `tee` for zero-copy I/O between file descriptors.
- Explain copy-on-write (COW) semantics for `fork` and `MAP_PRIVATE`.
- Configure and use huge pages (`HUGETLB`) for reduced TLB pressure.
- Use `O_DIRECT` to bypass the page cache entirely.
- Apply `MSG_ZEROCOPY` for zero-copy network sends.

## The Problem

Every time your program calls `read()`, the kernel copies data from the page cache into a
user-space buffer. Every `write()` copies data from user space back into the kernel. For a
web server streaming a 1 GB file, that's 2 GB of memcpy—data that was already in the right
place (the kernel's page cache) gets copied out and back for no semantic reason.

The same pattern repeats across systems:

| Operation | Copies involved | Wasted bandwidth |
|-----------|-----------------|------------------|
| `read` → process → `write` | 4 | 2× the data |
| `sendfile` | 0–1 | 0–1× |
| `mmap` + `write` from mapped addr | 1 | 1× |
| `splice` (pipe) | 0 | 0× |

This lesson eliminates those redundant copies.

## How read/write Works (The Baseline)

When you call `read(fd, buf, len)`:

1. The kernel checks the page cache. If the page isn't cached, it issues a disk read into
   **page-cache buffers owned by the kernel**.
2. The kernel `memcpy`s data from the page cache into your `buf` in user space.
3. Control returns to your program.

`write(fd, buf, len)` reverses this: the kernel copies `buf` from user space into the page
cache, then marks the page dirty for eventual writeback.

For a proxy that reads a file and writes it to a socket, the data path is:

```
disk → kernel page cache → user buffer → kernel socket buffer → NIC
         copy 1                      copy 2
```

Two full copies through CPU registers. For high-throughput workloads (video streaming, storage
servers, databases), this is the dominant cost.

## mmap: Zero-Copy File Access

`mmap` maps a file directly into the process's virtual address space:

```c
void *addr = mmap(NULL, len, PROT_READ, MAP_PRIVATE, fd, 0);
```

After `mmap`, the file's pages appear at `addr`. No `read()` copy happens. When you access
`addr[0]`, the CPU issues a page fault, the kernel loads the page from disk (or finds it in the
page cache), and maps it directly into your address space. You read from the same physical
memory the kernel uses for the page cache—zero copies.

### Demand Paging

`mmap` does **not** read the entire file at `mmap` time. Pages are loaded on demand via page
faults. This is crucial:

- **Sequential access**: Page faults are roughly predictable; the kernel can prefetch.
- **Random access on large files**: Page faults become unpredictable; the kernel can't
  prefetch effectively, and every access may trigger a fault with TLB misses. This is where
  `mmap` can be *slower* than `read` with a user buffer, because `read` amortizes the
  syscall overhead across larger chunks while `mmap` pays a page fault per page.

### Page Fault Overhead

Each page fault costs ~1–5 μs on modern hardware. If you touch 256 KB of data randomly in a
10 GB file:

- `mmap`: ~64 page faults × 3 μs = ~192 μs just in faults, plus TLB shootdowns on context
  switch.
- `read` with 256 KB buffer: 1 syscall, data already in cache if sequential, no faults.

For random access patterns on files much larger than RAM, `mmap` often loses because the page
fault overhead exceeds the `read` syscall overhead.

## MAP_PRIVATE vs MAP_SHARED

| Flag | Write behavior | Use case |
|------|---------------|----------|
| `MAP_PRIVATE` | Copy-on-write: writes go to a private copy, original unchanged | Loading executables, config files |
| `MAP_SHARED` | Writes modify the underlying file (visible to other processes) | Shared memory IPC, persistent data structures |

### MAP_PRIVATE and Copy-on-Write

When you `mmap` a file with `MAP_PRIVATE` and write to a page, the kernel:

1. Allocates a new page frame.
2. Copies the original page content into the new frame.
3. Updates your page table to point at the new frame.
4. Marks the new frame dirty.

The original page in the page cache is untouched. Other processes that `mmap` the same file
with `MAP_PRIVATE` see the original data. This is the same COW mechanism used by `fork()`.

## Copy-on-Write and fork()

When a process calls `fork()`, the kernel:

1. Creates a new page table for the child, pointing at the **same physical pages** as the
   parent.
2. Marks all pages read-only in both parent and child.
3. When either process writes to a page → page fault → kernel copies the page → each process
   gets its own private copy.

This means `fork()` is O(1) in physical memory usage. Only pages that are actually modified
incur a copy. For workloads where the child immediately calls `exec()`, COW means almost zero
copying occurs.

### COW Pitfall: Large Private Mappings

If a process `mmap`s a 1 GB file with `MAP_PRIVATE` and then writes to every page, it will
allocate 1 GB of private memory—exactly what you were trying to avoid. COW saves memory only
when the write ratio is low.

## sendfile: Zero-Copy File-to-Socket Transfers

```c
ssize_t sendfile(int out_fd, int in_fd, off_t *offset, size_t count);
```

`sendfile` copies data directly from `in_fd` (a file) to `out_fd` (a socket) entirely within
kernel space. No data ever enters user space. The data path becomes:

```
disk → kernel page cache → kernel socket buffer → NIC
         (same page, no copy)       copy 1
```

With scatter-gather DMA capable NICs, even the copy into the socket buffer can be eliminated:

```
disk → kernel page cache → NIC (via DMA scatter-gather)
         zero copies
```

Limitations:
- `in_fd` must be a file (not a socket).
- `out_fd` must be a socket (on Linux ≤ 2.6.33; later kernels allow any fd).
- No transformation of data (compression, encryption) is possible without copying to userspace.

## splice and tee: Zero-Copy Pipe Operations

```c
long splice(int fd_in, off_t *off_in, int fd_out, off_t *off_out,
            size_t len, unsigned int flags);
long tee(int fd_in, int fd_out, size_t len, unsigned int flags);
```

- **`splice`**: Moves data between two file descriptors where at least one is a pipe. Data
  stays in kernel space the entire time.
- **`tee`**: Duplicates data from one pipe to another without copying—like the Unix `tee`
  command but in kernel space.

Typical pattern for a proxy:

```c
// Client socket → pipe → server socket (and vice versa)
splice(client_fd, NULL, pipefd[1], NULL, 4096, 0);
splice(pipefd[0], NULL, server_fd, NULL, 4096, 0);
```

The pipe acts as a kernel-space buffer. Data never enters user space.

## Huge Pages (HUGETLB)

Standard pages on x86-64 are 4 KB. A process with 1 GB of mapped data has 262,144 page table
entries, creating significant TLB pressure (TLB misses cost ~20–40 cycles each).

Huge pages are 2 MB (or 1 GB with `MAP_HUGETLB` + `MAP_HUGE_1GB`). Benefits:

- 512× fewer page table entries for the same memory.
- Fewer TLB misses → faster page walks.
- Reduced page table memory (page tables themselves consume memory).

### Using Huge Pages with mmap

```c
void *addr = mmap(NULL, len, PROT_READ | PROT_WRITE,
                  MAP_PRIVATE | MAP_ANONYMOUS | MAP_HUGETLB, -1, 0);
```

Or with explicit 2 MB pages:

```c
void *addr = mmap(NULL, len, PROT_READ | PROT_WRITE,
                  MAP_PRIVATE | MAP_ANONYMOUS | MAP_HUGETLB | MAP_HUGE_2MB, -1, 0);
```

Prerequisites:
- Huge pages must be pre-allocated in the kernel's huge page pool:
  `echo 1024 > /proc/sys/vm/nr_hugepages`
- `/proc/meminfo` shows `HugePages_Total`, `HugePages_Free`, `Hugepagesize`.
- If the pool is exhausted, `mmap` with `MAP_HUGETLB` fails with `ENOMEM`.

### When to Use Huge Pages

- Database buffer pools (PostgreSQL `huge_pages = on`).
- In-memory caches with long-lived allocations.
- Scientific computing with large arrays.
- Avoid for short-lived or small allocations—huge pages can waste memory due to internal
  fragmentation (a 3 MB allocation uses a 2 MB page, wasting most of the second page).

## O_DIRECT: Bypassing the Page Cache

```c
int fd = open("data.bin", O_RDONLY | O_DIRECT);
```

`O_DIRECT` attempts to bypass the kernel page cache entirely, transferring data directly
between the user buffer and the storage device. Use cases:

- Databases that manage their own cache (PostgreSQL, MySQL InnoDB).
- Backup software that reads files sequentially once (no need to pollute the page cache).
- Real-time applications where page cache eviction causes latency spikes.

Requirements:
- The user buffer must be aligned to the logical block size (typically 512 bytes or 4 KB).
- Use `posix_memalign` or `aligned_alloc` for the buffer.
- The file offset must also be aligned.
- Not all OS/kernel versions guarantee full bypass—some still use the page cache partially.

### O_DIRECT vs mmap vs read

| Method | Copies to user space | Uses page cache | Alignment required |
|--------|---------------------|-----------------|-------------------|
| `read` | Yes | Yes | No |
| `mmap` | No (direct access) | Yes | No |
| `O_DIRECT` + `read` | Yes | No | Yes (buffer + offset) |

## MSG_ZEROCOPY: Zero-Copy Network Sends

Linux 4.14+ adds `MSG_ZEROCOPY` to `sendmsg`:

```c
int sock = socket(AF_INET, SOCK_STREAM, 0);
int opt = 1;
setsockopt(sock, SOL_SOCKET, SO_ZEROCOPY, &opt, sizeof(opt));

sendmsg(sock, &msg, MSG_ZEROCOPY);
```

Without `MSG_ZEROCOPY`, `send` copies data from the user buffer into the socket buffer. With
it, the kernel pins the user pages and tells the NIC to DMA directly from them. When the NIC
finishes, the kernel generates a completion notification on the error queue.

Trade-offs:
- Completion notification overhead: you must `recvmsg` on `sock`'s error queue after each
  `MSG_ZEROCOPY` send to know when it's safe to modify the buffer.
- Only beneficial for large sends (> ~10 KB). For small messages, the copy is cheaper than
  the notification mechanism.
- Requires `SO_ZEROCOPY` to be set on the socket before use.

## When mmap Is Slower Than read

`mmap` is not a magic bullet. It's slower when:

1. **Random access on files larger than RAM**: Every access may trigger a page fault and a
   disk read. `read` with a well-sized buffer can be more predictable.
2. **Very small files**: The `mmap` setup cost (VMA creation, page table manipulation) exceeds
   the benefit of avoiding one small `read` copy.
3. **Single-pass sequential processing**: If you `mmap` a file, process it once, and `munmap`,
   the page fault overhead per page can exceed the one-time `read` cost, especially if the
   kernel can't prefetch effectively.
4. **Signals and page faults**: `mmap` makes I/O errors manifest as `SIGBUS`. If the file is
   truncated after `mmap`, accessing the truncated region kills the process. `read` returns
   a clean error code.
5. **Memory pressure**: `mmap` creates VMAs that consume kernel memory. Thousands of `mmap`
   calls can exhaust the kernel's VMA limit (`/proc/sys/vm/max_map_count`).

## Build It

### Step 1: Minimal Version — mmap vs read Benchmark

Compare sequential file access through `mmap` vs `read` for a test file.

```c
#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>
#include <time.h>

#define FILE_SIZE (64 * 1024 * 1024)
#define BUF_SIZE  (64 * 1024)

static double now_sec(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

int main(void) {
    // Create test file
    int fd = open("/tmp/zerocopy_test", O_RDWR | O_CREAT | O_TRUNC, 0644);
    if (fd < 0) { perror("open"); return 1; }
    ftruncate(fd, FILE_SIZE);

    // Benchmark read()
    char *buf = malloc(BUF_SIZE);
    double t0 = now_sec();
    lseek(fd, 0, SEEK_SET);
    ssize_t total = 0;
    ssize_t n;
    while ((n = read(fd, buf, BUF_SIZE)) > 0) total += n;
    double read_time = now_sec() - t0;

    // Benchmark mmap()
    struct stat st;
    fstat(fd, &st);
    t0 = now_sec();
    char *addr = mmap(NULL, st.st_size, PROT_READ, MAP_PRIVATE, fd, 0);
    volatile char sink = 0;
    for (off_t i = 0; i < st.st_size; i += 4096)
        sink = addr[i];
    double mmap_time = now_sec() - t0;
    munmap(addr, st.st_size);
    (void)sink;

    printf("read:  %.3f ms for %zd bytes\n", read_time * 1000, total);
    printf("mmap:  %.3f ms for %lld bytes\n", mmap_time * 1000, (long long)st.st_size);

    free(buf);
    close(fd);
    unlink("/tmp/zerocopy_test");
    return 0;
}
```

### Step 2: Realistic Version — Full Benchmark Suite

See `code/main.c` for the complete implementation with:
- Sequential `read` vs `mmap` benchmark
- Page fault overhead measurement
- MAP_PRIVATE COW behavior demonstration
- Huge page comparison
- Timing comparison table

## Use It

In production systems:

- **Nginx** uses `sendfile` by default (`sendfile on;` in `nginx.conf`). This is why it can
  serve static files at wire speed—the data path from disk to NIC never touches user space.
- **PostgreSQL** uses `mmap` for some code paths, but primarily uses `read` with `O_DIRECT`
  for the WAL and shared buffers, because the database manages its own cache and wants
  predictable I/O patterns.
- **Linux kernel's `do_sendfile`** in `fs/splice.c` is the implementation that makes
  `sendfile` zero-copy. It uses the pipe buffer infrastructure to move pages between file
  and socket without copying.

## Read the Source

- **Linux `sendfile` implementation**: `fs/splice.c` — look at `do_splice_direct()` and the
  `splice_pipe_buf_ops` that enable zero-copy page references.
- **Linux `mmap` implementation**: `mm/mmap.c` — `do_mmap()` and `mmap_region()` show how VMA
  structures are created and how page faults are connected.
- **Linux COW**: `mm/memory.c` — `do_wp_page()` handles the write-protect fault that triggers
  copy-on-write.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A zero-copy API reference card** (`zerocopy_reference.md`) — a quick-reference for which
  zero-copy mechanism to use when, with API signatures and common pitfalls.

## Exercises

1. **Easy** — Modify the benchmark to use `MAP_SHARED` instead of `MAP_PRIVATE` and measure
   the difference. Explain what you observe.
2. **Medium** — Implement a proxy using `splice` that forwards data between a client socket
   and a server socket without any data entering user space. Measure throughput.
3. **Hard** — Build a file server that uses `sendfile` for small files and `mmap` + write for
   large files with range requests. Justify the threshold you choose with benchmarks.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Zero-copy | "No copying at all" | No copy between kernel and user space; copies within the kernel may still exist |
| mmap | "Map a file into memory" | Create a virtual memory mapping where page faults load file contents on demand |
| sendfile | "Send a file fast" | Kernel-internal data transfer from file fd to socket fd with no user-space copy |
| COW | "Copy only when you write" | Page table trick: share physical pages until a write triggers an alloc + memcpy |
| Huge page | "Bigger pages" | 2 MB or 1 GB pages that reduce TLB misses and page table size |
| O_DIRECT | "Direct I/O" | Bypass the page cache; data goes directly between aligned user buffer and device |
| Demand paging | "Lazy loading" | Pages are loaded into memory only when accessed, not at mmap time |
| splice | "Pipe-based zero-copy" | Kernel-space data movement between a pipe and another fd with no user copy |

## Further Reading

- *Linux Kernel Development* (Robert Love), Chapter 15 — The Page Cache and File I/O
- *Understanding the Linux Kernel* (Bovet & Cesati), Chapter 15 — Page Cache and I/O
- Linux man pages: `mmap(2)`, `sendfile(2)`, `splice(2)`, `tee(2)`, `msync(2)`
- Linux kernel source: `mm/memory.c`, `fs/splice.c`, `net/ipv4/tcp.c` (MSG_ZEROCOPY)
- LWN articles: "Zero-copy networking" (2017), "The state of zero-copy" (2020)