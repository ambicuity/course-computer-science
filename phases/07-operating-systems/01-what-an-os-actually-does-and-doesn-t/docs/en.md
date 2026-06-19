# Lesson 01: What an OS Actually Does (and Doesn't)

## Overview

An operating system is the invisible layer that sits between your hardware and your applications. Most developers interact with it daily—spawning processes, reading files, opening sockets—without understanding what it's actually doing. This lesson draws a precise boundary around what the OS is responsible for and what it isn't.

---

## The OS as Resource Manager

Every computer has finite resources: CPU cycles, RAM, disk space, network bandwidth. Without an OS, programs would have to fight over these resources directly, coordinate access to shared hardware, and agree on memory layouts. The OS prevents this chaos by acting as a referee.

```
┌─────────────────────────────────────────────────┐
│                  Applications                    │
│         (Web browsers, editors, servers)         │
├─────────────────────────────────────────────────┤
│                                                  │
│               OPERATING SYSTEM                   │
│                                                  │
│   ┌──────────┐ ┌──────────┐ ┌───────────────┐   │
│   │ Process  │ │  Memory  │ │   File System │   │
│   │ Manager  │ │ Manager  │ │    (VFS)      │   │
│   └──────────┘ └──────────┘ └───────────────┘   │
│   ┌──────────┐ ┌──────────┐ ┌───────────────┐   │
│   │   I/O    │ │ Network  │ │   Security    │   │
│   │ Manager  │ │  Stack   │ │  & Isolation  │   │
│   └──────────┘ └──────────┘ └───────────────┘   │
│                                                  │
├─────────────────────────────────────────────────┤
│   CPU  │  RAM  │  Disk  │  NIC  │  GPU  │  USB │
└─────────────────────────────────────────────────┘
                        Hardware
```

The OS performs two fundamental jobs simultaneously:

1. **Resource management** — Allocate and multiplex scarce hardware across competing programs.
2. **Abstraction** — Present simple, consistent interfaces (files, sockets, processes) over complex, heterogeneous hardware.

---

## The Six Core Responsibilities

### 1. Process Management

A **process** is a running instance of a program. The OS is responsible for the entire process lifecycle:

- **Creation** — `fork()`, `exec()`, `CreateProcess()`. The OS allocates a PID, sets up address space, and loads the binary.
- **Scheduling** — Decides which process runs on which CPU core and for how long. Common schedulers: CFS (Linux), thread director (Windows), GCD-aware (macOS).
- **Context switching** — Saves/restores CPU registers, TLB entries, and stack pointers when switching between processes.
- **Termination** — Reclaims memory, closes file descriptors, notifies parent via `wait()`.

```
            ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐
            │ P1  │  │ P2  │  │ P3  │  │ P4  │
            │ready│  │ wait│  │ready│  │ run │
            └──┬──┘  └─────┘  └──┬──┘  └──┬──┘
               │                 │         │
               ▼                 ▼         ▼
         ┌─────────────────────────────────────┐
         │         CPU Scheduler               │
         │   (decides who runs when)           │
         └─────────────────┬───────────────────┘
                           │
                    ┌──────┴──────┐
                    │  CPU Core   │
                    │  executes   │
                    │  one at a   │
                    │  time       │
                    └─────────────┘
```

Key insight: even with 8 cores, you might have 200 processes. The scheduler creates the *illusion* that each process has its own CPU through rapid time-slicing.

### 2. Memory Management

Every process believes it has a private, contiguous block of memory. This is **virtual memory**, and it's one of the OS's most important tricks.

- **Virtual address spaces** — Each process gets its own view of memory starting at address 0. The MMU (Memory Management Unit) translates virtual addresses to physical addresses.
- **Paging** — Memory is divided into fixed-size pages (typically 4 KB). The OS maintains page tables mapping virtual → physical pages.
- **Demand paging** — Pages are loaded from disk only when accessed, not when the process starts.
- **Swapping** — Infrequently used pages can be written to disk to free physical RAM.

```
Process A's View          Physical RAM
┌──────────────┐         ┌──────────────┐
│  0x00000000  │         │  Frame 0     │ ← maps A's page 0
│  (page 0)    │──┐      │  Frame 1     │ ← maps B's page 0
│  0x00001000  │  │      │  Frame 2     │ ← maps A's page 1
│  (page 1)    │──┤      │  Frame 3     │ ← free
│  0x00002000  │  ├──►   │  Frame 4     │ ← maps A's page 2
│  (page 2)    │──┤      │  Frame 5     │ ← maps B's page 1
│  ...         │  │      │  ...         │
└──────────────┘  │      └──────────────┘
                  │
Process B's View  │
┌──────────────┐  │      Page Table (A)
│  0x00000000  │──┤      ├─ page 0 → frame 0
│  (page 0)    │──┘      ├─ page 1 → frame 2
│  0x00001000  │────────  ├─ page 2 → frame 4
│  (page 1)    │         └─ page 3 → (swap)
│  ...         │
└──────────────┘
```

### 3. File Systems

The file system abstracts raw block storage into a hierarchical namespace of files and directories.

- **Abstraction** — Applications see `open()`, `read()`, `write()`, `close()`. They don't need to know about disk geometry, sectors, or blocks.
- **Virtual File System (VFS)** — A common interface that sits on top of concrete implementations (ext4, NTFS, APFS, ZFS). You can mount different file systems and access them uniformly.
- **Metadata** — Inodes (Unix) or MFT entries (NTFS) store permissions, timestamps, size, and block pointers.
- **Journaling** — Modern file systems log changes before committing them, protecting against corruption from crashes.

```
  Application
      │
      │  open("/home/user/file.txt")
      ▼
┌──────────────┐
│     VFS      │  ← uniform interface
└──────┬───────┘
       │
  ┌────┴────┬──────────┬──────────┐
  ▼         ▼          ▼          ▼
 ext4     NTFS      procfs     tmpfs
  │         │          │          │
  ▼         ▼          ▼          ▼
  SSD      HDD      kernel     RAM
```

### 4. I/O Management

Hardware devices (keyboards, disks, GPUs, sensors) have wildly different interfaces. The OS provides uniform access through **device drivers** and the **I/O subsystem**.

- **Device drivers** — Kernel modules that translate generic requests (read block 42) into device-specific commands.
- **Buffering and caching** — The kernel caches disk blocks in memory (page cache) to avoid repeated physical reads.
- **Interrupt handling** — When a device finishes an operation, it raises an interrupt. The kernel's interrupt handler processes the result and wakes the waiting process.
- **DMA (Direct Memory Access)** — For large transfers, the device writes directly to RAM without CPU involvement.

### 5. Networking

The OS implements the network protocol stack, typically modeled on TCP/IP:

```
Application     │  HTTP, FTP, SSH, DNS
────────────────┤
Transport       │  TCP, UDP
────────────────┤
Network         │  IP, ICMP, ARP
────────────────┤
Data Link       │  Ethernet, Wi-Fi
────────────────┤
Physical        │  Electrical signals, radio waves
```

The OS handles packet routing, connection management, buffering, retransmission, and exposes the **socket API** (`socket()`, `bind()`, `listen()`, `accept()`). Every web server, database connection, and API call passes through this stack.

### 6. Security and Isolation

The OS enforces boundaries between processes and users:

- **User/kernel mode** — The CPU has privilege levels. User code runs in ring 3 (restricted). The kernel runs in ring 0 (full access). System calls are the controlled gateway between them.
- **File permissions** — Unix rwx bits, ACLs, SELinux/AppArmor mandatory access control.
- **Process isolation** — Virtual memory prevents one process from reading another's memory.
- **Capabilities** — Fine-grained privileges beyond simple root/user dichotomy.

```
  ┌──────────────────────────────────────────┐
  │            User Space (Ring 3)           │
  │  ┌────────┐  ┌────────┐  ┌────────┐    │
  │  │ App A  │  │ App B  │  │ App C  │    │
  │  │cannot  │  │cannot  │  │cannot  │    │
  │  │see B   │  │see A   │  │see A/B │    │
  │  └────┬───┘  └────┬───┘  └────┬───┘    │
  │       │  syscall   │  syscall  │        │
  ├───────┼────────────┼───────────┼────────┤
  │       ▼            ▼           ▼        │
  │            Kernel Space (Ring 0)        │
  │  ┌─────────────────────────────────┐    │
  │  │  Has full hardware access       │    │
  │  │  Enforces isolation between     │    │
  │  │  processes above                │    │
  │  └─────────────────────────────────┘    │
  └──────────────────────────────────────────┘
```

---

## What the OS Does NOT Do

Understanding the OS also means understanding its limits:

| The OS does NOT... | Because... |
|---|---|
| Write your code | That's your job (or your compiler's) |
| Optimize your algorithms | An O(n²) sort is slow regardless of the OS |
| Guarantee correctness | A race condition in your code stays a race condition |
| Prevent all crashes | A segfault is your bug; the OS just catches it |
| Manage application-level state | Databases, caches, sessions are application concerns |
| Choose your architecture | Monolith vs microservice is your design decision |

The OS gives you processes, memory, and I/O. What you build with them is entirely up to you.

---

## Kernel Architectures

Not all operating systems are structured the same way.

### Monolithic Kernel

Everything—scheduling, memory, file systems, drivers, networking—runs in a single address space in kernel mode.

```
┌────────────────────────────────────────┐
│             Kernel Space               │
│  ┌──────┬──────┬──────┬──────┬──────┐  │
│  │Sched │ Mem  │  FS  │ Drvr │ Net  │  │
│  │      │      │      │      │      │  │
│  └──────┴──────┴──────┴──────┴──────┘  │
│      All in one address space          │
├────────────────────────────────────────┤
│             Hardware                   │
└────────────────────────────────────────┘
```

**Examples:** Linux, original Unix
**Pros:** Fast (no message passing overhead), direct function calls between components.
**Cons:** A bug in any driver can crash the entire system. Huge codebase runs in privileged mode.

### Microkernel

Only the bare minimum (scheduling, IPC, basic memory management) runs in kernel mode. Everything else—drivers, file systems, network stacks—runs as user-space servers.

```
┌────────────────────────────────────────┐
│             User Space                 │
│  ┌──────┐  ┌──────┐  ┌──────┐        │
│  │ FS   │  │ Drvr │  │ Net  │        │
│  │server│  │server│  │server│        │
│  └──┬───┘  └──┬───┘  └──┬───┘        │
│     │  IPC    │  IPC    │             │
├─────┼─────────┼─────────┼─────────────┤
│     ▼         ▼         ▼             │
│  ┌─────────────────────────────┐      │
│  │  Microkernel                │      │
│  │  (IPC, scheduling, memory)  │      │
│  └─────────────────────────────┘      │
│             Kernel Space               │
├────────────────────────────────────────┤
│             Hardware                   │
└────────────────────────────────────────┘
```

**Examples:** L4, Minix 3, QNX, seL4
**Pros:** Isolation (driver crash doesn't kill kernel), smaller trusted computing base.
**Cons:** IPC overhead, more complex communication patterns.

### Hybrid Kernel

A pragmatic middle ground. Core services run in kernel mode, but the architecture allows some flexibility.

```
┌────────────────────────────────────────┐
│             User Space                 │
│  ┌───────────┐      ┌───────────┐     │
│  │ IOKit     │      │ User      │     │
│  │ drivers   │      │ servers   │     │
│  └─────┬─────┘      └─────┬─────┘     │
├────────┼───────────────────┼───────────┤
│        ▼                   ▼           │
│  ┌─────────────────────────────────┐   │
│  │  Hybrid Kernel                  │   │
│  │  (Mach microkernel + BSD        │   │
│  │   subsystem + IOKit drivers)    │   │
│  └─────────────────────────────────┘   │
│             Kernel Space               │
├────────────────────────────────────────┤
│             Hardware                   │
└────────────────────────────────────────┘
```

**Examples:** Windows NT, macOS (XNU), ReactOS
**Pros:** Balance of performance and modularity.
**Cons:** Complexity; boundary between kernel and user space can blur.

---

## POSIX: The Portable Standard

POSIX (Portable Operating System Interface) defines a standard API for Unix-like systems. It specifies:

- System calls: `fork()`, `exec()`, `open()`, `read()`, `write()`, `close()`, `wait()`, `pipe()`, `socket()`
- Shell and utilities: `ls`, `grep`, `sed`, `awk`, `make`
- Command-line environment variables, signals, threading (`pthreads`)

Linux, macOS, FreeBSD, and Solaris are POSIX-compliant (or nearly so). Windows is not POSIX-native, though WSL and Cygwin provide compatibility layers.

Why it matters: code written against POSIX APIs compiles and runs across dozens of operating systems with minimal changes.

---

## Real-Time vs General-Purpose

| Property | General-Purpose (Linux, Windows) | Real-Time (FreeRTOS, QNX, VxWorks) |
|---|---|---|
| Goal | Throughput, fairness | Deterministic timing |
| Scheduling | CFS, priority-based | Rate-monotonic, EDF |
| Latency | Variable (ms range) | Bounded (μs range) |
| Use case | Desktops, servers, phones | Medical devices, avionics, robotics |
| Fairness | High priority | Deadline meeting |

A real-time OS guarantees that a task completes within a defined deadline. A general-purpose OS optimizes for overall throughput and responsiveness. Mixing both (PREEMPT_RT patches for Linux) is an active area of development.

---

## Brief History

```
Multics (1964)
  │
  ▼
Unix (1969, Ken Thompson & Dennis Ritchie at Bell Labs)
  │
  ├──► BSD (1977) ──► FreeBSD, OpenBSD, NetBSD, macOS
  │
  ├──► System V (1983) ──► Solaris, AIX, HP-UX
  │
  └──► Linux (1991, Linus Torvalds) ──► Ubuntu, Fedora, Android, etc.

Windows NT (1993, Dave Cutler) ──► Windows 10/11, Server
```

Multics gave us the idea of a multi-user, time-sharing OS. Unix distilled it into something small, elegant, and portable (written in C). Linux reimplemented Unix from scratch as open source. Windows NT took a different path with a hybrid kernel design.

---

## Use It

Understanding the OS helps you write better systems code:

- **Choosing the right system call** — `mmap()` vs `read()` for large files; `epoll` vs `select` for high-concurrency networking.
- **Debugging with OS knowledge** — `strace` shows system calls, `vmstat` shows memory pressure, `iostat` shows disk bottlenecks.
- **Performance tuning** — Understanding page cache behavior, context switch costs, and scheduler policies lets you make informed decisions.

When your server is slow, the answer is often in the OS layer, not your application code.

---

## Ship It

By the end of this lesson, you should be able to draw the concept map below from memory:

```
                    ┌──────────────┐
                    │  Operating   │
                    │   System     │
                    └──────┬───────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   ┌────┴────┐      ┌─────┴─────┐     ┌─────┴─────┐
   │Resource │      │Abstraction│     │ Security  │
   │Manager  │      │  Layer    │     │ & Isolate │
   └────┬────┘      └─────┬─────┘     └───────────┘
        │                 │
   ┌────┼────┐      ┌────┼────┐
   │    │    │      │    │    │
  CPU  Mem  I/O  Files Net  Devices
```

---

## Exercises

### Level 1 — Recall

1. List the six core responsibilities of an operating system.
2. What is the difference between virtual memory and physical memory?
3. Name three examples of monolithic kernels and two examples of microkernels.

### Level 2 — Comprehension

1. Explain why a microkernel is more fault-tolerant than a monolithic kernel, using the concept of a device driver crash as an example.
2. A process sees memory addresses starting at `0x00000000`. Explain how the OS and hardware make this possible when physical RAM is shared among many processes.
3. Why does POSIX matter for a developer writing a C program that needs to run on both Linux and macOS?

### Level 3 — Application

1. You run `top` and see 150 processes but only 4 CPU cores. Explain exactly how the OS creates the illusion that each process is running simultaneously. Mention context switching and scheduling in your answer.
2. Your web server handles 10,000 concurrent connections. Explain which OS subsystems are involved for each connection: from the NIC interrupt, through the network stack, to the socket `read()` call, to the file system if logging is enabled.
3. A colleague claims "the OS should optimize my sorting algorithm." Explain precisely why this claim misunderstands the OS's role, and where the boundary between OS responsibility and application responsibility lies.
