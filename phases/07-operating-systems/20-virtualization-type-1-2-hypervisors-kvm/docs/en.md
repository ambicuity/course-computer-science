# Lesson 20: Virtualization — Type 1/2 Hypervisors, KVM

## Why This Matters

Every cloud provider (AWS, GCP, Azure) runs your code on virtual machines. Virtualization is the technology that makes cloud computing possible — it lets a single physical machine run multiple isolated operating systems. Understanding hypervisors, hardware virtualization extensions, and KVM means you understand the foundation that all cloud infrastructure is built on.

## What Is Virtualization?

Virtualization creates the illusion of a complete, dedicated machine. A **hypervisor** (Virtual Machine Monitor) manages multiple **guest** operating systems, each believing it has exclusive access to the hardware.

```
  ┌─────────┐  ┌─────────┐  ┌─────────┐
  │ Guest 1 │  │ Guest 2 │  │ Guest 3 │
  │ (Linux) │  │ (Win)   │  │ (Linux) │
  └────┬────┘  └────┬────┘  └────┬────┘
       │            │            │
  ┌────┴────────────┴────────────┴────┐
  │         Hypervisor (VMM)          │
  └────────────────┬──────────────────┘
                   │
  ┌────────────────┴──────────────────┐
  │           Hardware                │
  │    CPU  │  Memory  │  Disk  │ NIC │
  └───────────────────────────────────┘
```

## Type 1 vs Type 2 Hypervisors

### Type 1 — Bare-Metal

The hypervisor runs **directly on the hardware** with no host OS underneath. It is the first thing the bootloader loads.

```
  ┌─────────┐  ┌─────────┐  ┌─────────┐
  │ Guest A │  │ Guest B │  │   VM    │
  │  Linux  │  │ Windows │  │ Manager │
  └────┬────┘  └────┬────┘  └────┬────┘
  ┌────┴────────────┴────────────┴────┐
  │       Type 1 Hypervisor           │  ← runs on bare metal
  └────────────────┬──────────────────┘
                   │
  ┌────────────────┴──────────────────┐
  │            Hardware               │
  └───────────────────────────────────┘
```

**Examples:** Xen, VMware ESXi, Microsoft Hyper-V, KVM (debated — it's a kernel module, but the kernel IS the hypervisor).

**Advantages:** Better performance, better security (smaller attack surface — no host OS), used in production clouds.

### Type 2 — Hosted

The hypervisor runs **on top of a host operating system** as a regular process.

```
  ┌─────────┐  ┌─────────┐
  │ Guest A │  │ Guest B │
  └────┬────┘  └────┬────┘
  ┌────┴────────────┴────┐
  │  Type 2 Hypervisor   │  ← runs as a process
  ├───────────────────────┤
  │     Host OS           │  ← Linux, Windows, macOS
  ├───────────────────────┤
  │     Hardware          │
  └───────────────────────┘
```

**Examples:** VirtualBox, VMware Workstation/Fusion, Parallels, QEMU (in emulation mode).

**Advantages:** Easy to install, no special hardware needed. **Disadvantages:** Extra overhead from the host OS layer.

## Hardware Virtualization — VT-x / AMD-V

Early x86 CPUs were not designed for virtualization. Some privileged instructions (like `POPFlags`) behaved differently in user mode vs kernel mode, making full virtualization impossible without binary translation.

Intel VT-x and AMD-V added **two modes of operation** to the CPU:

| Mode | Ring Level | Who runs here |
|------|-----------|---------------|
| **VMX root** (root mode) | Ring 0 | Hypervisor — full control |
| **VMX non-root** (non-root mode) | Ring 0–3 | Guest OS — thinks it has full control |

The guest runs in Ring 0 inside non-root mode. Privileged instructions trap to the hypervisor instead of executing directly.

**VMCS (Virtual Machine Control Structure)** — Intel's data structure that holds the complete state of a guest:
- Guest register state (RAX, RBX, RIP, RSP, CR3, etc.)
- Host register state (to restore on VM exit)
- Execution control bits (which instructions cause VM exits)
- Exit reason and qualification

**VMCB (Virtual Machine Control Block)** — AMD's equivalent.

### VM Entry / VM Exit

```
  Hypervisor (root mode)           Guest (non-root mode)
  ┌──────────────────┐             ┌──────────────────┐
  │                  │  VMLAUNCH/  │                  │
  │  Set up VMCS     │  VMRESUME   │  Guest runs      │
  │                  │────────────►│  (Ring 0!)       │
  │                  │             │                  │
  │  Handle exit     │  VM exit    │  Privileged inst │
  │  (emulate I/O,   │◄────────────│  causes trap     │
  │   fix page fault)│             │                  │
  └──────────────────┘             └──────────────────┘
```

- **VM entry:** Hypervisor loads guest state from VMCS, runs guest code.
- **VM exit:** Guest executes a privileged instruction or hits a configured condition. CPU saves guest state to VMCS, loads host state, jumps to hypervisor's exit handler.

Common VM exit reasons: I/O port access, CR3 change, CPUID, HLT, page fault, external interrupt.

## KVM — Kernel-based Virtual Machine

KVM turns the Linux kernel itself into a Type 1 hypervisor. It's a kernel module that uses VT-x/AMD-V to run guests.

```
  ┌──────────────────────────────────────┐
  │  QEMU (user-space device emulation)  │
  ├──────────────────────────────────────┤
  │  /dev/kvm (ioctl interface)          │
  ├──────────────────────────────────────┤
  │  KVM kernel module                   │
  │  - vcpu scheduling                   │
  │  - VMCS management                   │
  │  - Memory virtualization (EPT/NPT)   │
  ├──────────────────────────────────────┤
  │  Linux kernel (memory, scheduling,   │
  │  networking, storage)                │
  ├──────────────────────────────────────┤
  │  Hardware (VT-x/AMD-V)              │
  └──────────────────────────────────────┘
```

**KVM provides:** CPU and memory virtualization via hardware extensions.
**QEMU provides:** Device emulation (disk, network, USB, display) and VM management.

The interface is through `/dev/kvm` using `ioctl()` calls:
1. `open("/dev/kvm")` — get KVM fd
2. `ioctl(kvm_fd, KVM_CREATE_VM)` — create a VM, get VM fd
3. `ioctl(vm_fd, KVM_CREATE_VCPU)` — create a vCPU, get vCPU fd
4. `mmap()` the vCPU's run structure for communication
5. Load guest code into guest memory
6. `ioctl(vcpu_fd, KVM_RUN)` — run the guest until VM exit
7. Handle exit reason, repeat

## Paravirtualization

In paravirtualization, the **guest OS is modified** to know it's running in a VM. Instead of executing trapped privileged instructions, it makes **hypercalls** — direct calls to the hypervisor.

**Xen** pioneered this approach. The guest kernel is ported to use hypercalls for operations like page table updates, interrupt handling, and I/O.

**Advantages:** No hardware support needed, lower overhead per operation.
**Disadvantages:** Requires modifying the guest OS — can't run unmodified Windows.

**VirtIO** — a modern paravirtualization framework for I/O devices. The guest uses a paravirtualized network/disk driver (virtio-net, virtio-blk) instead of emulating a real device. This is why KVM/QEMU VMs are much faster with virtio drivers.

## Nested Virtualization

Running a VM inside a VM — the guest hypervisor itself uses VT-x to run nested guests.

```
  ┌──────────────────┐
  │  Nested Guest    │
  └───────┬──────────┘
  ┌───────┴──────────┐
  │  L2 Hypervisor   │  (runs in guest)
  └───────┬──────────┘
  ┌───────┴──────────┐
  │  L1 Hypervisor   │  (KVM)
  └───────┬──────────┘
  ┌───────┴──────────┐
  │  Hardware        │
  └──────────────────┘
```

Intel VT-x supports nested virtualization via **VMCS shadowing** — the L1 hypervisor's VMCS operations are intercepted and virtualized by the host. Enabled with `kvm-intel.nested=1` module parameter.

Used for: testing hypervisor software, running Kubernetes-in-Docker, cloud provider development.

## Build It

See `code/main.c` for a minimal KVM program that creates a VM, loads a small guest program, and runs it. The guest writes "Hello from guest!" to the I/O port, which QEMU/KVM intercepts and prints.

The program demonstrates:
- Opening `/dev/kvm` and creating a VM
- Allocating guest memory and loading code
- Creating a vCPU and setting up registers
- Running the guest in a loop, handling VM exits (I/O, HLT)

## Use It

| Provider | Hypervisor | Details |
|----------|-----------|---------|
| AWS | KVM (Nitro) | Custom hardware for I/O offload |
| GCP | KVM | Live migration support |
| Azure | Hyper-V | Type 1, Windows-based |
| DigitalOcean | KVM | Standard KVM |

On your Linux machine, check KVM availability:

```bash
ls -la /dev/kvm                    # KVM device exists?
kvm-ok                             # CPU supports VT-x/AMD-V?
lsmod | grep kvm                   # KVM module loaded?
```

## Read the Source

- Linux kernel: `virt/kvm/kvm_main.c` — KVM core: VM creation, vCPU management, ioctl dispatch
- Linux kernel: `arch/x86/kvm/vmx/vmx.c` — Intel VT-x (VMX) implementation, VMCS operations
- Linux kernel: `arch/x86/kvm/svm.c` — AMD-V (SVM) implementation, VMCB operations
- QEMU source: `target/i386/kvm/` — QEMU's KVM integration for x86

## Ship It

The KVM guest program in `code/main.c` demonstrates the fundamental loop of hardware virtualization: set up guest state, run guest, handle exits, repeat.

## Exercises

### Level 1 — Recall

What is the difference between Type 1 and Type 2 hypervisors? What is VMCS and what does it contain? Why can't early x86 CPUs be fully virtualized without hardware extensions?

### Level 2 — Application

Extend the KVM program in `code/main.c` to handle the `KVM_EXIT_IO` exit reason. Have the guest write a sequence of bytes to an I/O port, and have the host read and print them. What is the guest's view of the port write vs the host's?

### Level 3 — Build

Implement a KVM program that:
1. Loads a guest that uses CPUID to detect it's running in a VM (CPUID leaf 0x40000000 is the KVM signature)
2. Sets up a minimal page table in guest memory so the guest can use virtual addresses
3. Handles `KVM_EXIT_MMIO` to emulate a simple virtual device
4. Implements a virtio-style ring buffer for guest-to-host communication

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Hypervisor | "Virtual machine monitor" | Software that creates and manages virtual machines |
| Type 1 hypervisor | "Bare-metal" | Hypervisor running directly on hardware (Xen, ESXi, Hyper-V) |
| Type 2 hypervisor | "Hosted" | Hypervisor running on a host OS (VirtualBox, VMware Workstation) |
| VT-x / AMD-V | "Hardware virtualization" | CPU extensions providing root/non-root modes for virtualization |
| VMCS / VMCB | "Guest state" | Hardware data structure storing complete guest CPU state |
| VM exit | "Trap to hypervisor" | CPU transitions from non-root to root mode |
| Paravirtualization | "Modified guest" | Guest OS is aware of virtualization and uses hypercalls |
| Nested virtualization | "VM in a VM" | Running a hypervisor inside a guest VM |

## Further Reading

- Intel SDM Volume 3C, Chapter 24–33 — VMX specification
- AMD APM Volume 2, Chapter 15 — SVM specification
- `man 4 kvm` — KVM API overview
- KVM Forum talks — youtube.com/kaboratory
- Kishor Barde, *An Updated Performance Comparison of Virtual Machines and Linux Containers*
- Linux kernel: `Documentation/virt/kvm/` — KVM architecture documentation
