# Lesson 21: Microkernels and Unikernels

## Why This Matters

Not all operating systems are built like Linux. The monolithic kernel design — where everything runs in kernel space — is just one point on a spectrum of OS architectures. Microkernels push most services to user space for reliability and security. Unikernels strip away everything except what one application needs, producing a single-purpose OS image. Understanding these alternatives helps you reason about trade-offs in real systems: QNX runs your car's brakes, seL4 is formally verified, and MirageOS unikernels boot in microseconds.

## The Kernel Design Spectrum

```
  Monolithic          Microkernel          Unikernel
  ┌───────────────┐   ┌───────────────┐   ┌───────────────┐
  │ Kernel Space  │   │ Kernel Space  │   │  Single       │
  │               │   │               │   │  Address      │
  │ - Process mgmt│   │ - IPC         │   │  Space        │
  │ - Memory mgmt │   │ - Scheduling  │   │               │
  │ - File system │   │ - Basic VM    │   │ App + minimal │
  │ - Network stack│   │               │   │ OS libraries  │
  │ - Device drv  │   ├───────────────┤   │ compiled into  │
  │ - IPC         │   │ User Space    │   │ one binary    │
  │               │   │ - File system │   │               │
  │               │   │ - Network     │   │ No process    │
  │               │   │ - Drivers     │   │ isolation     │
  │               │   │ - IPC servers │   │               │
  └───────────────┘   └───────────────┘   └───────────────┘
  Linux, FreeBSD      seL4, QNX, L4       MirageOS, Unikraft
```

## Monolithic Kernel

Everything runs in **kernel space** — one big privileged program. Device drivers, file systems, network stacks, process management, memory management — all in the same address space with full hardware access.

**How it works:** System calls trap into the kernel. Inside the kernel, services call each other via direct function calls — fast, no context switch needed.

```
  User Program
      │
      │  syscall (e.g., read())
      ▼
  ┌─────────────────────────────────────────┐
  │              Kernel Space               │
  │                                         │
  │  syscall entry → VFS → ext4 → block    │
  │                  │              │       │
  │                  ▼              ▼       │
  │              scheduler     device drv   │
  │                                         │
  │  All share same address space           │
  │  Direct function calls between services │
  └─────────────────────────────────────────┘
```

**Advantages:**
- **Performance:** Direct function calls between components — no IPC overhead, no context switches for internal communication
- **Mature ecosystem:** Decades of driver development, hardware support
- **Simplicity of internal API:** Components call each other directly

**Disadvantages:**
- **Large Trusted Computing Base (TCB):** A bug in any driver can crash or compromise the entire kernel
- **Difficult to verify:** Millions of lines of code running in privileged mode
- **Driver bugs = kernel panic:** A faulty network driver can bring down the whole system

**Examples:** Linux, FreeBSD, traditional UNIX

### Linux Kernel Size (approximate)

| Component | Lines of Code |
|-----------|--------------|
| Core kernel (sched, mm, etc.) | ~2M |
| Drivers | ~15M |
| Architecture code | ~2M |
| Filesystems | ~1.5M |
| Networking | ~1M |
| **Total** | **~25M+ lines** |

All 25 million lines run in Ring 0 with full hardware access.

## Microkernel

Only the **bare minimum** runs in kernel space: inter-process communication (IPC), scheduling, and basic memory management. Everything else — drivers, file systems, network stacks — runs as **user-space servers**.

```
  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
  │  App A   │  │  App B   │  │  FS Srv  │  │  Net Srv │
  │          │  │          │  │ (ext4)   │  │ (TCP/IP) │
  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘
       │  IPC        │  IPC        │  IPC         │  IPC
  ┌────┴─────────────┴─────────────┴──────────────┴──────┐
  │                  Microkernel                          │
  │                                                       │
  │  - IPC (message passing)                              │
  │  - Thread scheduling                                  │
  │  - Address space management                           │
  │  - Interrupt routing                                  │
  │                                                       │
  │  Minimal TCB — small enough to formally verify        │
  └───────────────────────────────┬───────────────────────┘
                                  │
  ┌───────────────────────────────┴───────────────────────┐
  │                       Hardware                        │
  └───────────────────────────────────────────────────────┘
```

**How a file read works in a microkernel:**

```
  App                FS Server           Driver
  │                    │                   │
  │  IPC: "read file"  │                   │
  │───────────────────►│                   │
  │                    │  IPC: "read disk" │
  │                    │──────────────────►│
  │                    │                   │  (DMA)
  │                    │  IPC: "data"      │
  │                    │◄──────────────────│
  │  IPC: "data"       │                   │
  │◄───────────────────│                   │
```

Three IPC round-trips instead of one direct function call. This is the fundamental cost of microkernels.

**Advantages:**
- **Small TCB:** Only a few thousand lines in privileged mode — can be formally verified
- **Fault isolation:** A driver crash doesn't kill the kernel — restart the driver server
- **Flexibility:** Swap file systems, network stacks, or drivers at runtime
- **Security:** Less privileged code = smaller attack surface

**Disadvantages:**
- **IPC overhead:** Every service interaction requires message passing + context switches
- **Complexity:** System becomes a distributed system on a single machine
- **Performance:** Typically 5–30% slower than monolithic for I/O-heavy workloads

### Microkernel Examples

**L4 family:**
- Liedtke's original L4 (1993): IPC in ~100 cycles
- L4Ka::Pistachio, OKL4 (used in mobile baseband processors)
- Focus on **extremely fast IPC** — the key to microkernel performance

**seL4:**
- The world's **first formally verified** general-purpose OS kernel
- Mathematical proof that the C implementation matches the specification
- ~9,000 lines of C + 200 lines of assembly
- Proof guarantees: no buffer overflows, no null pointer derefs, no information leaks
- Used in military drones, medical devices, autonomous vehicles

**QNX:**
- Real-time microkernel used in automotive (instrument clusters, ADAS), industrial control
- BlackBerry's BB10 was built on QNX
- Known for deterministic latency and high reliability

**MINIX 3:**
- Designed by Andrew Tanenbaum for education and high reliability
- Drivers run as unprivileged user-space processes
- If a driver crashes, the OS automatically restarts it
- Self-healing: automatically recovers from driver and server failures

## Hybrid Kernels

Some kernels take a pragmatic middle ground: mostly monolithic, but with some microkernel principles.

**Windows NT:**
- Kernel-mode: scheduler, memory manager, IPC, some drivers
- User-mode: subsystem servers (Win32, POSIX — historically), some drivers (UMDF)
- Not a true microkernel, but uses a layered design with HAL (Hardware Abstraction Layer)

**macOS XNU:**
- Combines Mach microkernel (IPC, scheduling) with BSD subsystem (file systems, networking, POSIX)
- IOKit driver framework is in kernel space (C++ subset)
- More monolithic in practice — most services run in kernel for performance

```
  ┌──────────────────────────────────────────────┐
  │              macOS XNU Kernel                │
  │                                              │
  │  ┌──────────────┐   ┌──────────────────┐    │
  │  │  Mach         │   │  BSD             │    │
  │  │  - IPC        │   │  - VFS           │    │
  │  │  - Scheduling │   │  - TCP/IP        │    │
  │  │  - VM         │   │  - POSIX API     │    │
  │  │  - Tasks      │   │  - Process mgmt  │    │
  │  └──────────────┘   └──────────────────┘    │
  │                                              │
  │  ┌──────────────────────────────────────┐   │
  │  │  IOKit (C++ driver framework)        │   │
  │  │  Device drivers in kernel space      │   │
  │  └──────────────────────────────────────┘   │
  └──────────────────────────────────────────────┘
```

**Why hybrid?** Pure microkernels pay too high a performance cost for I/O-intensive workloads. Moving drivers to user space means every disk read, every network packet requires multiple IPC round-trips. Hybrid kernels keep performance-critical paths in kernel space while gaining modularity elsewhere.

## Unikernels

A unikernel compiles an application and the **minimal OS libraries** it needs into a **single, bootable binary**. There is no process isolation, no user/kernel boundary, no shell, no multi-user — just one program running directly on the hypervisor or bare metal.

```
  Traditional Stack                  Unikernel
  ┌───────────────┐                 ┌───────────────────────┐
  │  Application  │                 │  Application          │
  ├───────────────┤                 │  + language runtime    │
  │  libc / lang  │                 │  + minimal libc       │
  ├───────────────┤                 │  + TCP/IP stack       │
  │  Kernel       │                 │  + memory allocator   │
  │  (25M lines)  │                 │  + device driver      │
  ├───────────────┤                 │                       │
  │  Drivers      │                 │  All compiled to one  │
  ├───────────────┤                 │  binary (~1MB)        │
  │  Bootloader   │                 └──────────┬────────────┘
  └───────────────┘                            │
                                      ┌────────┴────────┐
                                      │  Hypervisor /   │
                                      │  Hardware       │
                                      └─────────────────┘
```

**How it works:**
1. You write your application (e.g., a web server in OCaml or C)
2. The unikernel toolchain analyzes what OS functions it actually needs
3. It links your code with only those minimal libraries
4. The result is a single binary that boots directly — no OS underneath

**Examples:**

| Project | Language | Target | Use Case |
|---------|----------|--------|----------|
| MirageOS | OCaml | Xen, KVM, Virtio | Cloud microservices, IoT |
| Unikraft | C | Xen, KVM, bare metal | High-performance network functions |
| IncludeOS | C++ | KVM, VirtualBox | C++ microservices |
| NanoVMs / Edera | Go, Rust, JS | AWS, GCP, bare metal | Production unikernels |

**Advantages:**
- **Tiny size:** ~1–5 MB vs ~500 MB for a Linux container image
- **Fast boot:** Microseconds to milliseconds — ideal for serverless cold starts
- **Small attack surface:** No shell, no multi-user, no unused drivers
- **No syscall overhead:** Library OS calls are direct function calls (no context switch)
- **Immutable:** The binary IS the entire system — reproducible, tamper-evident

**Disadvantages:**
- **No process isolation:** One application per unikernel — no multi-tenancy
- **Debugging is hard:** No gdb, no strace, no shell to inspect state
- **Limited hardware support:** Only targets hypervisors or specific bare-metal platforms
- **Maturity:** Smaller ecosystem than containers or VMs

**Use cases:**
- **Serverless:** Cold start in microseconds vs milliseconds for containers
- **IoT / embedded:** Minimal footprint, no OS overhead
- **Network functions:** NFV (Network Function Virtualization) — each function is a unikernel
- **Security-sensitive:** Minimized TCB, no unnecessary attack surface

## Architecture Comparison

| Property | Monolithic | Microkernel | Hybrid | Unikernel |
|----------|-----------|-------------|--------|-----------|
| Kernel size | Millions of LOC | Thousands of LOC | Hundreds of KLOC | N/A (no kernel/user split) |
| TCB size | Very large | Very small (verifiable) | Medium | Minimal |
| Driver isolation | None (in kernel) | Full (user space) | Partial | N/A (one app) |
| IPC overhead | None (direct calls) | High (message passing) | Low–Medium | None (library calls) |
| Performance | Highest | Lower (IPC cost) | High | Highest (for single app) |
| Fault recovery | Reboot | Restart server | Depends | Restart instance |
| Verification | Impractical | seL4: formally verified | Difficult | Smaller codebase = easier |
| Examples | Linux, FreeBSD | seL4, QNX, L4, MINIX 3 | Windows NT, macOS XNU | MirageOS, Unikraft |

## When to Use What

**Monolithic kernel (Linux):** General-purpose computing. Huge ecosystem, best performance, most hardware support. Default choice.

**Microkernel (seL4, QNX):** Safety-critical systems where failure is not an option — automotive, aerospace, medical devices. When you need formal verification or fault isolation.

**Hybrid (Windows, macOS):** Desktop/server OS where you want some modularity but can't afford the performance cost of pure microkernel IPC.

**Unikernel (MirageOS, Unikraft):** Single-purpose cloud workloads where you need minimal attack surface, tiny image size, and microsecond boot times. Serverless, IoT, network functions.

## Read the Source

- seL4: `sel4kernel/` — the formally verified microkernel (~9,000 lines of C)
- MINIX 3: `minix/servers/` — user-space device driver and server implementations
- Linux: `kernel/sched/` — compare monolithic scheduler with microkernel equivalents
- MirageOS: `mirage/` — OCaml unikernel framework source
- Unikraft: `unikraft/` — C unikernel framework

## Ship It

This lesson's artifact is the **architecture comparison table** and the mental models above. No code to ship — but understanding these trade-offs directly informs decisions about container vs VM vs bare-metal deployment.

## Exercises

### Level 1 — Recall

List the three core services a microkernel provides. What is the Trusted Computing Base (TCB) and why does it matter for security? Name three microkernel-based operating systems and their use cases.

### Level 2 — Application

Compare the sequence of operations (and context switches) required to read a file from disk in:
1. A monolithic kernel (Linux)
2. A microkernel (QNX or seL4)
3. A unikernel

For each, count the number of privilege level transitions and IPC round-trips. Why does this difference matter for I/O-intensive workloads?

### Level 3 — Build

Design a minimal microkernel specification:
1. Define the IPC message format (fixed-size? variable-size? what fields?)
2. Design the system call interface (what syscalls does the microkernel provide?)
3. Sketch the architecture: how would a file system server, a network server, and a disk driver interact to serve a `GET /index.html` HTTP request?
4. What formal properties would you want to verify? (e.g., information flow, absence of deadlocks)

Compare your design with seL4's API. What does seL4 do differently and why?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Monolithic kernel | "Everything in kernel space" | All OS services share one privileged address space with direct function calls |
| Microkernel | "Minimal kernel" | Only IPC, scheduling, and basic memory management in kernel; everything else in user space |
| IPC overhead | "Microkernel tax" | Performance cost of message passing between user-space servers |
| Trusted Computing Base | "TCB" | The set of all hardware/software that must be correct for security to hold — smaller is better |
| Formal verification | "Mathematically proven" | Machine-checked proof that code satisfies a formal specification (seL4) |
| Hybrid kernel | "Best of both" | Kernel with microkernel structure but some monolithic performance optimizations |
| Unikernel | "Single-purpose OS" | Application + minimal OS compiled into one binary, no process isolation |
| Hypercall | "Paravirtualized syscall" | Direct call from guest to hypervisor, bypassing hardware trap |

## Further Reading

- Andrew Tanenbaum, *Modern Operating Systems*, Chapter 1 — kernel design trade-offs
- Jochen Liedtke, *Towards Real Microkernels* (1995) — the paper that showed microkernels could be fast
- seL4 whitepaper: `sel4.systems/Docs/seL4-whitepaper.pdf`
- Anil Madhavapeddy et al., *Unikernels: Library Operating Systems for the Cloud* (ASPLOS 2013)
- MINIX 3: `minix3.org` — self-healing microkernel OS
- QNX Neutrino: `blackberry.qnx.com` — real-time microkernel documentation
