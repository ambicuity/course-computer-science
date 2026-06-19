# Kernel Bypass — DPDK, SPDK, AF_XDP

> The kernel's job is fairness and generality. Your job is throughput. Sometimes those goals collide.

**Type:** Learn
**Languages:** C
**Prerequisites:** Phase 15 lessons 01–11
**Time:** ~75 minutes

## Learning Objectives

- Explain *why* kernel bypass exists: syscall overhead, context-switch cost, and the kernel-to-userspace memcpy tax.
- Describe the architecture of DPDK (poll-mode drivers, huge pages, ring buffers, core pinning), SPDK (NVMe poll-mode driver, no kernel block layer), and AF_XDP (XDP programs, BPF, zero-copy from NIC to userspace).
- Decide when kernel bypass is worth the complexity and when it isn't.
- Compare kernel bypass with io_uring as a middle-ground approach.
- Implement a conceptual benchmark that measures the cost of kernel syscalls vs userspace polling.

## The Problem

Every packet your NIC receives, every block your SSD serves, triggers a journey through the kernel:

1. The NIC raises an **interrupt**. The CPU switches to kernel mode (context switch ≈ 1–5 µs on modern x86).
2. The kernel's network stack parses headers, routes the packet, and **copies** it into a userspace buffer (`memcpy` from kernel memory to the socket's receive buffer).
3. Your application calls `recv()` — another syscall, another context switch.

At 10 Gbps line rate (~14.88 Mpps for minimum-sized packets), you have ~67 ns per packet. A single context switch costs 15–75× that budget. Add the copy, the interrupt handling, the mutex on the socket buffer — and the kernel becomes the bottleneck.

**The same story plays out for storage.** A single NVMe SSD can deliver 800K IOPS. The kernel block layer adds latency through its request queue, I/O scheduler, and block-device abstraction — all designed for rotating media where a few extra microseconds didn't matter.

Kernel bypass asks: *what if we cut the kernel out of the data path entirely?*

## The Concept

### Why Kernel Bypass Exists

#### Syscall Overhead

Every system call triggers a `syscall` instruction (or `int 0x80` on 32-bit). This:

- Switches from user mode (ring 3) to kernel mode (ring 0).
- Saves/restores all general-purpose registers.
- Walks the kernel's syscall table to find the handler.
- Runs the handler, then returns — restoring registers and switching back to ring 3.

On a modern x86 core, a null syscall (that does nothing) costs ~100–200 ns. Meaningful syscalls like `read()` or `sendmsg()` cost more because they do real work *inside* the kernel.

#### Context-Switch Cost

A context switch is worse than a syscall because it changes the running task:

- CPU pipeline flush (dozens of cycles).
- L1/L2 cache pollution — the new task's working set evicts the old one.
- TLB flush (or at least TLB misses) if the new task has a different address space.
- Scheduler overhead: picking the next task, updating run-queues.

Measured cost: **1–5 µs** per switch on modern hardware. At 10 million switches/second, you've consumed 10–50 ms — which is 1–5 seconds of CPU time per second. You can't afford that.

#### The Memory-Copy Tax

The kernel sits between the NIC and your application. Data arriving from the network must be copied from kernel memory (where the DMA engine placed it) to the userspace buffer you provided. For a 1500-byte Ethernet frame, that's a 1500-byte `memcpy`. At 14.88 Mpps, that's ~22 GB/s of pure copy bandwidth — on a machine that might only have ~50 GB/s of memory bandwidth total.

Zero-copy techniques (like `splice()`, `MSG_ZEROCOPY`) help for forwarding workloads, but they still go *through* the kernel. Kernel bypass eliminates the copy by giving userspace direct access to the DMA buffer.

### DPDK Architecture

**DPDK** (Data Plane Development Kit) is the original kernel-bypass framework, originally developed by Intel and now a Linux Foundation project.

#### Poll-Mode Drivers (PMDs)

Traditional NIC drivers are interrupt-driven: the NIC raises an interrupt per packet (or per batch), and the kernel's interrupt handler runs. DPDK replaces this with **poll-mode drivers**:

- The application spins on a ring buffer: "is there a new packet?" No interrupts, no kernel involvement.
- The CPU core running the PMD is **pinned** (dedicated) — it never yields, never context-switches.
- Batch processing: check the ring, process 32 packets, check the ring again.

This trades CPU utilization (the pinned core is always 100% busy) for deterministic latency (no interrupt scheduling jitter).

#### Huge Pages

Normal x86 pages are 4 KB. The TLB (Translation Lookaside Buffer) holds ~64–128 entries. A 4 KB page means 1 GB of memory requires 262,144 TLB entries — far more than the hardware can hold.

DPDK uses **2 MB or 1 GB huge pages**:

- 2 MB pages: 1 GB requires only 512 TLB entries. TLB miss rate drops dramatically.
- 1 GB pages: 1 GB fits in a single TLB entry.
- Huge pages are also required for the DMA-mapping layer — the NIC needs physically contiguous memory for DMA.

#### Ring Buffers

DPDK uses lock-free ring buffers (based on the `rte_ring` library) for passing packet descriptors between cores:

- Single-producer, single-consumer (SPSC): the simplest and fastest case.
- Multi-producer, single-consumer (MPSC): for multiple RX cores feeding one processing core.
- The ring uses a `head`/`tail` index pair updated with atomic CAS operations — no mutexes.

#### Core Pinning and CPU Affinity

DPDK requires you to **pin** threads to specific CPU cores:

- `rte_lcore_id()` returns the current logical core.
- `rte_eal_init()` sets up the core mapping.
- Each pinned core runs exactly one PMD instance or processing pipeline stage.

This eliminates cache-line bouncing between cores and ensures `sched_getcpu()` is a no-op.

### SPDK Architecture

**SPDK** (Storage Performance Development Kit) applies the same kernel-bypass philosophy to storage:

#### NVMe Poll-Mode Driver

NVMe devices expose **submission queues** (SQ) and **completion queues** (CQ) in memory-mapped registers. SPDK's NVMe PMD:

- Maps the NVMe register space directly into userspace via `mmap(/dev/uioX)` or VFIO.
- Writes commands directly to the SQ tail pointer — no `ioctl()`, no kernel block layer.
- Polls the CQ head pointer for completions — no interrupts, no `epoll()`.

Latency drops from ~10 µs (kernel NVMe) to ~2 µs (SPDK NVMe PMD).

#### No Kernel Block Layer

The Linux block layer provides:

- I/O scheduling (CFQ, mq-deadline, bfq).
- Partition management.
- Request merging and sorting.
-blk-mq infrastructure.

For NVMe SSDs (which have internal parallelism and their own scheduling), most of this is overhead. SPDK bypasses it entirely, submitting commands directly to the hardware.

### AF_XDP Architecture

**AF_XDP** (Address Family eXpress Data Path) is the kernel's answer to DPDK — a *cooperative* bypass:

#### XDP Programs

XDP (eXpress Data Path) runs a BPF program *at the NIC driver level*, before the kernel's network stack:

```
Packet arrives → NIC driver → XDP BPF program runs → decision:
  - XDP_PASS: continue through normal kernel stack
  - XDP_DROP: drop immediately
  - XDP_TX: bounce back out the same NIC
  - XDP_REDIRECT: send to AF_XDP socket
```

The BPF program is JIT-compiled to native code and runs in ~50–100 ns per packet.

#### Zero-Copy from NIC to Userspace

When the XDP program returns `XDP_REDIRECT` to an AF_XDP socket:

- The NIC's DMA buffer is placed directly into a **UMEM** — a shared memory region registered by the userspace application.
- No `memcpy` from kernel to userspace. The packet data never leaves the memory it was DMA'd into.
- The userspace application reads from the UMEM's fill ring and writes completions to the completion ring.

#### BPF Verification

AF_XDP uses BPF, which means the kernel's verifier checks the program before loading it:

- Guarantees the program terminates (no infinite loops).
- Guarantees memory safety (no out-of-bounds access).
- This is a *security* advantage over DPDK — the kernel retains control over what the program can do.

#### Why AF_XDP Instead of DPDK?

| Factor | DPDK | AF_XDP |
|--------|------|--------|
| Kernel involvement | None (takes over NIC entirely) | Cooperative (XDP program in kernel) |
| Port sharing | No (NIC is exclusive) | Yes (XDP_REDIRECT only for matched packets) |
| Security | Userspace has raw device access | BPF-verified, kernel retains control |
| Performance ceiling | Slightly higher (no BPF indirection) | Close to DPDK, within ~10-15% |
| Deployment complexity | High (huge pages, VFIO, NIC takeover) | Lower (standard Linux interfaces) |

AF_XDP is the better choice when you need most of the performance but still want to coexist with the kernel's network stack.

### When Kernel Bypass Is Worth It

Kernel bypass makes sense when:

1. **High packet rates** — 10 Gbps+ line rates with small packets. The kernel can't handle 14.88 Mpps.
2. **Ultra-low latency** — Financial trading (HFT), where every microsecond costs money. Context switches are the enemy.
3. **Storage appliances** — NVMe-oF targets, software-defined storage (Ceph, SPDK bdev), where IOPS matter more than general-purpose fairness.
4. **Network function virtualization (NFV)** — Software routers, firewalls, load balancers that must process packets at line rate.

### When Kernel Bypass Is NOT Worth It

Kernel bypass is a bad idea when:

1. **General-purpose workloads** — Web servers, databases, APIs. The kernel's TCP/IP stack is battle-tested and well-optimized for these patterns.
2. **Management overhead** — DPDK requires dedicated cores, huge pages, VFIO setup. Operations teams hate this.
3. **Security considerations** — Kernel bypass means no kernel-enforced isolation. A bug in DPDK code crashes the whole application, not just one process. No SELinux, no cgroups, no standard firewalling.
4. **Feature compatibility** — Kernel bypass applications can't use iptables, conntrack, socket filters, or any kernel network feature. You must reimplement everything.
5. **Debugging difficulty** — Standard tools (`tcpdump`, `ss`, `netstat`) don't see DPDK-managed traffic.

### io_uring: The Middle Ground

**io_uring** (Linux 5.1+) is a kernel interface that gives you *some* bypass benefits *without* leaving the kernel:

- **Shared ring buffers** — The application and kernel communicate via two ring buffers (submission queue and completion queue) mapped into userspace.
- **Batching** — Submit multiple I/O operations in a single `io_uring_enter()` syscall instead of one syscall per operation.
- **Zero-copy reads** — With `IORING_OP_READ_FIXED`, the kernel reads directly into pre-registered buffers — no intermediate copy.
- **Polling mode** — `IORING_SETUP_SQPOLL` uses a kernel thread that polls the submission queue, so the application doesn't even need to make syscalls for submission.

Performance: io_uring can reach ~80–90% of DPDK's throughput for many workloads, while keeping all the kernel's security, networking, and debugging features intact.

### Real-World Users

- **CloudFlare** — Uses DPDK in their edge network for DDoS mitigation and L4 load balancing. Their `gatekeeper` and `bpftool` projects are open source.
- **NVIDIA/Mellanox** — The ConnectX NIC family has native DPDK and AF_XDP support. NVIDIA maintains the `mlx5` PMD in DPDK.
- **Intel** — Created both DPDK and SPDK. Uses them internally for NVMe-oF targets, software-defined storage, and network function virtualization.
- **Facebook/Meta** — Uses AF_XDP for L4 load balancing (Katran), choosing cooperative bypass over full DPDK for easier ops.
- **Seastar** — The framework behind ScyllaDB uses DPDK-style poll-mode I/O for its storage engine, achieving millions of IOPS.

## Build It

We'll build a conceptual benchmark that demonstrates the cost of kernel syscalls vs userspace polling. This is *not* full DPDK — that requires a DPDK-capable NIC and driver setup. Instead, we:

1. Measure `read()` from `/dev/urandom` (kernel syscall path) vs `mmap()` + polling (userspace path).
2. Count context switches via `/proc/self/status`.
3. Show the raw numbers that motivate kernel bypass.

### Step 1: Minimal Version

Read `/dev/urandom` in a loop, count syscalls and context switches.

### Step 2: Realistic Version

Compare the syscall path against an `mmap()`'d buffer where userspace polls for data. Measure throughput and context-switch overhead.

The full implementation is in `code/main.c`.

## Use It

In production, kernel bypass is not a DIY project:

- **DPDK** provides the full poll-mode driver framework: `rte_eth_dev`, `rte_mbuf`, `rte_ring`, `rte_lcore`.
- **SPDK** provides the NVMe PMD: `spdk_nvme_ctrlr`, `spdk_bdev`, `spdk_io_channel`.
- **AF_XDP** uses standard socket APIs with the `AF_XDP` address family and a BPF program loaded via `libbpf`.

Compare our conceptual benchmark (which demonstrates *why* bypass matters) against the production frameworks (which *implement* bypass):

| What | Our demo | Production |
|------|----------|-------------|
| NIC access | `/dev/urandom` read | DPDK `rte_eth_rx_burst()` |
| Zero-copy | `mmap` + polling | `AF_XDP` UMEM |
| Context switch count | `/proc/self/status` | `perf stat -e context-switches` |
| Core pinning | `sched_setaffinity()` | DPDK `rte_lcore` framework |

## Read the Source

- **DPDK**: `drivers/net/mlx5/mlx5_rxq.c` — the Mellanox ConnectX receive queue implementation. Look at how `mlx5_rx_burst()` polls the CQ without syscalls.
- **SPDK**: `lib/nvme/nvme_qpair.c` — the NVMe queue pair submission/completion logic. Compare `spdk_nvme_qpair_process_completions()` with the kernel's `blk_mq_ops` — same commands, no kernel.
- **AF_XDP**: `net/xdp/xsk.c` in the Linux kernel — the AF_XDP socket implementation. Note how `xsk_recvmsg()` reads from the UMEM fill ring.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`kernelbypass_reference.md`** — A comparison reference card for DPDK, SPDK, and AF_XDP, with architecture diagrams and decision guidance.

## Exercises

1. **Easy** — Modify the benchmark to compare `read()` vs `fread()` (buffered I/O). Does userspace buffering reduce context switches?
2. **Medium** — Add `io_uring` to the benchmark. Use `liburing` to submit reads and measure throughput vs the `read()` and `mmap` paths. How close does io_uring get to the mmap polling path?
3. **Hard** — Set up DPDK on a test machine. Write an `rte_eth_rx_burst()` / `rte_eth_tx_burst()` ping-pong application. Measure the latency distribution and compare it against the kernel's raw socket path.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Kernel bypass | "Skip the kernel" | Taking the data path out of the kernel — direct hardware access from userspace |
| DPDK | Intel's fast networking thing | A framework of poll-mode NIC drivers, huge pages, and lock-free rings that eliminates kernel networking |
| SPDK | DPDK for storage | An NVMe poll-mode driver and block-device framework that bypasses the Linux block layer |
| AF_XDP | Fast sockets | A Linux kernel feature where XDP BPF programs redirect packets to a zero-copy userspace socket |
| PMD | Poll-mode driver | A driver that spins checking for new work instead of waiting for interrupts |
| Huge pages | Big memory pages | 2 MB or 1 GB pages that reduce TLB pressure and enable DMA-contiguous allocations |
| Context switch | Mode switch | CPU transitioning between kernel mode and user mode (or between processes) — costs 1–5 µs each |
| Zero-copy | No memcpy | Data stays in the same memory location from NIC DMA through to userspace processing |
| io_uring | The new async thing | A Linux async I/O interface using shared ring buffers that reduces syscall overhead without full bypass |
| UMEM | DPDK memory region | In AF_XDP, a userspace-registered memory region where NIC packets are DMA'd directly |

## Further Reading

- [DPDK Official Documentation](https://doc.dpdk.org/)
- [SPDK Official Documentation](https://spdk.io/doc/)
- [AF_XDP Kernel Documentation](https://docs.kernel.org/networking/af_xdp.html)
- [io_uring man page](https://man7.org/linux/man-pages/man7/io_uring.7.html)
- [CloudFlare's AF_XDP blog series](https://blog.cloudflare.com/xdp-based-load-balancer/)
- [Intel DPDK Programmer's Guide](https://doc.dpdk.org/guides/prog_guide/)