# Kernel Bypass Reference Card — DPDK, SPDK, AF_XDP

## Architecture Overview

### Traditional Kernel Path (Baseline)

```
Application                    Kernel                         Hardware
    |                            |                               |
    |-- syscall (read/recv) --->|                               |
    |                            |-- copy from kernel buf ------->|
    |                            |   (context switch)            |
    |<-- data in userspace buf --|                               |
    |                            |<-- interrupt from HW ----------|
    |                            |    (context switch, memcpy)   |
```

Cost per operation:
- 2 context switches (user→kernel→user)
- 1+ memcpy (kernel ↔ userspace)
- 1 interrupt (or NAPI poll)
- **Total: ~1–10 µs per packet at minimum**

### DPDK Path

```
Application                    Kernel                         Hardware
    |                            |                               |
    |-- rte_eth_rx_burst() ----->|  (bypassed entirely)          |
    |   spin on receive ring     |                               |
    |<-- data from DMA buf ------|------------------- DMA ------|
    |   (zero-copy via huge pages)|                               |
    |                            |                               |
    |   No syscall               |   No interrupt               |
    |   No context switch        |   No memcpy                  |
```

Key components:
- **PMD**: Poll-mode driver — spins on NIC receive ring, no interrupts
- **Huge pages**: 2 MB/1 GB pages for TLB efficiency and DMA contiguity
- **rte_ring**: Lock-free ring buffers for inter-core communication
- **Core pinning**: Each thread locked to a dedicated CPU core
- **VFIO/UIO**: Kernel modules that map PCI device registers to userspace

### SPDK Path

```
Application                    Kernel                         Hardware
    |                            |                               |
    |-- spdk_nvme_cmd ---------->|  (bypassed entirely)          |
    |   write to SQ tail        |                               |
    |   poll CQ head            |                               |
    |<-- completion from CQ ----|------------------- DMA -------|
    |                            |                               |
    |   No syscall               |   No block layer              |
    |   No I/O scheduler         |   No interrupt                 |
```

Key components:
- **NVMe PMD**: Direct submission/completion queue access from userspace
- **No block layer**: Bypasses Linux's I/O scheduler, request merging, etc.
- **bdev**: SPDK's block device abstraction (pluggable backends: NVMe, NVMe-oF, malloc, etc.)
- Latency: ~2 µs (SPDK) vs ~10 µs (kernel NVMe)

### AF_XDP Path

```
Application                    Kernel (XDP BPF)               Hardware
    |                            |                               |
    |   UMEM fill ring          |                               |
    |--- descriptor ----------->|  XDP program runs             |
    |                            |  at NIC driver level           |
    |                            |  XDP_REDIRECT:                |
    |<-- packet in UMEM --------|<- zero-copy from DMA ---------|
    |   (no memcpy)             |   BPF-verified safety          |
    |                            |                                |
    |  BPF program decides:     |  Kernel retains control         |
    |  PASS / DROP / TX /       |  Can inspect all packets       |
    |  REDIRECT                  |                                |
```

Key components:
- **XDP program**: BPF program at NIC driver level, JIT-compiled (~50-100ns/pkt)
- **UMEM**: Shared memory region registered by userspace, receives DMA'd packets directly
- **Fill ring / Completion ring**: Ring buffers for UMEM buffer management
- **BPF verifier**: Kernel guarantees safety — no out-of-bounds, no infinite loops

### io_uring Path (Middle Ground)

```
Application                    Kernel                         Hardware
    |                            |                               |
    |  Fill submission ring      |                               |
    |-- io_uring_enter() ------->|                               |
    |   (1 syscall per batch)    |-- kernel processes batch ---->|
    |                            |   (amortized cost)            |
    |<-- completion ring --------|<-- completions ----------------|
    |                            |                               |
    |  Can use SQPOLL:           |                               |
    |  0 syscalls                |  Kernel thread polls ring     |
```

## Comparison Matrix

| Feature | DPDK | SPDK | AF_XDP | io_uring |
|---------|------|------|--------|----------|
| **Domain** | Networking | Storage | Networking | General I/O |
| **Kernel bypass** | Full (NIC takeover) | Full (NVMe takeover) | Cooperative (XDP hook) | None (kernel path) |
| **Zero-copy** | Yes | Yes | Yes (UMEM) | Optional (fixed bufs) |
| **Syscalls per op** | 0 | 0 | 0 in fast path | 0 (SQPOLL) or 1/batch |
| **Interrupt-free** | Yes (PMD) | Yes (PMD) | Yes (poll ring) | Optional (SQPOLL) |
| **Port/NIC sharing** | No (exclusive) | N/A | Yes | Yes |
| **Security model** | Raw device access (trust userspace) | Raw device access | BPF-verified, kernel retains control | Standard kernel security |
| **Linux features** | None (no iptables, conntrack, etc.) | None (no block layer features) | Full kernel stack for non-XDP traffic | Full kernel stack |
| **Deploy complexity** | High (huge pages, VFIO, core pinning) | High (huge pages, NVMe detach) | Medium (BPF program + UMEM) | Low (standard Linux API) |
| **Debugging** | Custom tools (dpdk-procinfo) | Custom tools (spdk_top) | Standard (tcpdump sees non-XDP) | Standard (strace, perf) |
| **Performance** | ★★★★★ (best) | ★★★★★ (best) | ★★★★☆ (~85-90% of DPDK) | ★★★☆☆ (~80-90% for I/O) |

## Decision Guide

### Use DPDK When:
- You need **maximum throughput** (40+ Gbps, 14+ Mpps)
- The NIC can be **dedicated** to DPDK (no sharing)
- You have **dedicated CPU cores** to burn on PMDs
- Your team can handle the **operations complexity** (huge pages, VFIO, custom monitoring)
- Examples: Software routers, hardware load balancers, NFV appliances

### Use SPDK When:
- You need **maximum IOPS** (800K+ on a single NVMe SSD)
- You're building an **NVMe-oF target** or storage appliance
- Latency matters more than general-purpose storage features
- Examples: Software-defined storage (Ceph with SPDK backend), NVMe-oF targets, vhost-user storage for VMs

### Use AF_XDP When:
- You need **most of DPDK's performance** but want to **coexist with the kernel**
- Port sharing is required (same NIC for kernel networking + fast path)
- Security teams require **kernel oversight** (BPF verification)
- Examples: L4 load balancing (Katran), DDoS mitigation, edge cloud networking

### Use io_uring When:
- You want **better I/O performance** without kernel bypass complexity
- Your workload involves **batched async I/O** (storage, network)
- You need all **standard kernel features** (security, networking, debugging)
- Examples: High-performance file I/O, async network servers, database engines

### Don't Use Kernel Bypass When:
- General-purpose web serving, APIs, or databases (kernel stack is fine)
- Traffic doesn't saturate a single core (context switch cost is negligible)
- Operations team can't manage dedicated-core, huge-page setups
- Security requires kernel enforcement (SELinux, cgroups, network policies)

## Quick Reference: Key Data Points

| Metric | Value | Source |
|--------|-------|--------|
| Null syscall cost | ~100-200 ns | x86 measurement |
| Context switch cost | ~1-5 µs | x86 Linux measurement |
| Minimum packet budget at 10 Gbps | ~67 ns | 1/(14.88 Mpps) |
| Minimum packet budget at 40 Gbps | ~17 ns | 1/(59.6 Mpps) |
| DPDK fast-path per-packet cost | ~50-100 ns | Poll + ring dequeue |
| AF_XDP BPF program cost | ~50-100 ns | JIT-compiled BPF |
| SPDK NVMe read latency | ~2 µs | Userspace NVMe PMD |
| Kernel NVMe read latency | ~10 µs | Block layer + interrupt |
| io_uring overhead vs DPDK | ~10-20% lower throughput | Amortized syscall per batch |

## Key APIs Quick Reference

### DPDK Essentials
```c
// Initialization
rte_eal_init(argc, argv);

// Receive packets (poll-mode)
struct rte_mbuf *bufs[BURST_SIZE];
uint16_t n = rte_eth_rx_burst(port_id, queue_id, bufs, BURST_SIZE);

// Process and free
for (i = 0; i < n; i++) {
    process_packet(bufs[i]);
    rte_pktmbuf_free(bufs[i]);
}
```

### SPDK Essentials
```c
// NVMe controller probe
spdk_nvme_probe(NULL, NULL, probe_cb, attach_cb, NULL);

// Submit I/O
spdk_nvme_ns_cmd_read(ns, qpair, buf, lba, num_blocks, completion_cb, NULL);
spdk_nvme_qpair_process_completions(qpair, 0); // poll for completions
```

### AF_XDP Essentials
```c
// Create XDP socket
int sock = socket(AF_XDP, SOCK_RAW, 0);

// Register UMEM
struct xdp_umem_reg umem_reg = { .addr = ... };
setsockopt(sock, SOL_XDP, XDP_UMEM_REG, &umem_reg, sizeof(umem_reg));

// Receive via fill ring
struct xdp_desc *desc = &fill_ring[fill_idx++];
recvmsg(sock, &msg, 0);
```

### io_uring Essentials
```c
// Setup
struct io_uring ring;
io_uring_queue_init(256, &ring, 0);

// Submit read
struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
io_uring_prep_read(sqe, fd, buf, size, offset);
io_uring_submit(&ring);

// Complete
struct io_uring_cqe *cqe;
io_uring_wait_cqe(&ring, &cqe);
```