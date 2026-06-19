# Lesson 23: Linux Internals Tour — The Source Tree

## Core Concepts

The Linux kernel is ~30 million lines of C and assembly. It is the most widely deployed operating system kernel in the world — running on servers, phones, embedded devices, supercomputers, and Mars rovers. Reading the source is how you go from "I understand OS theory" to "I understand how real systems work."

This lesson is a guided tour. No code to write. You will navigate the source tree, read key files, and build a mental map of where everything lives.

## Getting the Source

```bash
# Clone the kernel (or a shallow clone — ~2 GB full, ~200 MB shallow)
git clone --depth=1 https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git
cd linux

# Or browse online at: https://elixir.bootlin.com/linux/latest/source
```

## Top-Level Directory Structure

```
linux/
├── arch/          Architecture-specific code (x86, arm64, riscv, etc.)
├── block/         Block layer (I/O scheduling, bio structs)
├── certs/         Module signing certificates
├── crypto/        Cryptographic API (AES, SHA, etc.)
├── Documentation/ Kernel documentation (start here for subsystems)
├── drivers/       Device drivers (~60% of the tree)
├── fs/            File systems (ext4, btrfs, xfs, proc, etc.)
├── include/       Header files
├── init/          Kernel initialization (start_kernel)
├── ipc/           Inter-process communication
├── kernel/        Core kernel (scheduler, signals, timers, kthreads)
├── lib/           Library routines (crc32, sorting, etc.)
├── mm/            Memory management (page allocator, slab, vm)
├── net/           Networking stack (TCP/IP, netfilter, sockets)
├── rust/          Rust infrastructure (new)
├── scripts/       Build scripts, kconfig, etc.
├── security/      LSM frameworks (SELinux, AppArmor)
├── sound/         ALSA audio subsystem
├── tools/         Userspace tools (perf, bpf, etc.)
├── usr/           initramfs support
├── virt/          Virtualization (kvm)
├── Kbuild          Top-level kbuild file
├── Kconfig         Top-level configuration
├── Makefile        Top-level makefile
└── README          "This is the Linux kernel"
```

**drivers/** is the largest directory — roughly 60% of all kernel code. The kernel supports thousands of hardware devices, each with its own driver.

## Key Files to Read

### 1. `init/main.c` — The Boot Sequence

This is the entry point. Every kernel developer reads this file first.

```c
// init/main.c — simplified flow
start_kernel() {
    setup_arch();           // architecture-specific setup
    mm_init();              // memory management initialization
    sched_init();           // scheduler initialization
    rest_init();            // creates the first user-space process
}

rest_init() {
    // Creates PID 1 (init) via kernel_thread()
    // Creates the kthreadd (PID 2) for deferred work
    // The boot CPU enters the idle loop
}
```

Follow `start_kernel()` line by line. Each call initializes a subsystem. The order matters — you cannot initialize the scheduler before memory management.

### 2. `kernel/sched/core.c` — The CFS Scheduler

The Completely Fair Scheduler (CFS) is the default scheduler since Linux 2.6.23.

```c
// kernel/sched/core.c — key functions
enqueue_task_fair()     // add task to CFS runqueue
pick_next_task_fair()   // select task with smallest vruntime
task_tick_fair()        // called on timer tick, may preempt
```

CFS uses a **red-black tree** keyed by `vruntime` (virtual runtime). The leftmost node has the smallest vruntime and runs next. This gives O(log n) enqueue/dequeue and O(1) pick (just the leftmost node).

```c
// kernel/sched/fair.c — the heart of CFS
struct sched_entity {
    u64         vruntime;       // virtual runtime
    u64         exec_start;     // when current execution started
    struct rb_node  run_node;   // red-black tree node
    // ...
};
```

### 3. `mm/memory.c` — Page Fault Handler

```c
// mm/memory.c
handle_mm_fault() {
    // Determine fault type (read/write)
    // Check VMA (virtual memory area) permissions
    // Allocate page frame if needed
    // Copy data from file/swap
    // Update page table entry
}
```

The page fault handler is one of the most complex functions in the kernel. It handles anonymous pages (heap), file-backed pages (mmap), copy-on-write (fork), swap, NUMA balancing, and more.

### 4. `fs/ext4/` — The ext4 File System

```c
// fs/ext4/inode.c — reading an inode
ext4_iget()         // get inode from disk
ext4_readpage()     // read a page of file data

// fs/ext4/super.c — mounting
ext4_fill_super()   // read superblock, initialize structures

// fs/ext4/dir.c — directory operations
ext4_readdir()      // iterate directory entries
```

ext4 uses **extent trees** (instead of the old indirect block pointers in ext2/ext3). An extent is a contiguous run of blocks: `{start_block, length}`. This reduces metadata overhead for large files.

### 5. `arch/x86/entry/entry_64.S` — Syscall Entry Point

```asm
# arch/x86/entry/entry_64.S (simplified)
# When user code executes SYSCALL instruction:
#   RIP -> entry_SYSCALL_64
#   CS  -> kernel CS
#   RFLAGS saved in R11
#   RIP  saved in RCX

entry_SYSCALL_64:
    swapgs                      # swap GS to kernel GS
    mov %rsp, PER_CPU_VAR(rsp_scratch)  # save user RSP
    mov PER_CPU_VAR(cpu_tss_rw + TSS_sp2), %rsp  # load kernel RSP
    # ... save registers, call do_syscall_64()
    # ... on return, restore user RSP, SYSRETQ
```

Every syscall goes through this assembly stub. It switches from user stack to kernel stack, saves registers, calls the C handler `do_syscall_64()`, and returns with `SYSRETQ`.

### 6. `kernel/fork.c` — Process Creation

```c
// kernel/fork.c
kernel_clone() {
    copy_process() {
        dup_task_struct();      // duplicate task_struct
        copy_mm();              // copy or share address space (COW)
        copy_files();           // duplicate or share file descriptors
        copy_thread();          // set up kernel stack for new task
        // ...
    }
    wake_up_new_task();         // add to runqueue
}
```

`copy_mm()` is where `fork()`'s copy-on-write magic happens. The parent's page tables are copied with write-protected entries. Both processes share physical pages until one writes, triggering a COW fault.

## The Build Process

```bash
# Configure (use default for your arch)
make defconfig

# Or interactive menu
make menuconfig

# Build (use all cores)
make -j$(nproc)

# Output: arch/x86/boot/bzImage (x86) or arch/arm64/boot/Image (arm64)
```

Build steps:
1. **Kconfig** reads `.config` — which features/drivers are enabled
2. **Kbuild** compiles each directory's source into `.o` files
3. **Link** combines all objects into `vmlinux` (ELF binary)
4. **Compress** into `bzImage` (bootable, compressed)

```bash
# Time a full build (clean)
time make -j$(nproc)
# Expect 5-30 minutes depending on CPU and config
```

## Reading Strategy

### Start at init/main.c

```
start_kernel()
  ├── setup_arch()           → arch/x86/kernel/setup.c
  ├── mm_init()              → mm/init.c, mm/mmap.c
  ├── sched_init()           → kernel/sched/core.c
  ├── vfs_caches_init()      → fs/dcache.c
  ├── rest_init()
  │     ├── kernel_thread(kernel_init)  → PID 1
  │     └── kernel_thread(kthreadd)     → PID 2
  └── cpu_startup_entry()    → idle loop
```

### Follow a Syscall End-to-End

Pick `read()`. Trace it:
1. User calls `read(fd, buf, count)`
2. glibc wrapper → `syscall(SYS_read, ...)`
3. `arch/x86/entry/entry_64.S` → `entry_SYSCALL_64`
4. `do_syscall_64()` → `sys_read()`
5. `ksys_read()` → `vfs_read()` → `file->f_op->read()`
6. For ext4: `ext4_file_read_iter()` → `generic_file_read_iter()` → `page_cache_read()`

### Follow a Fork

1. `sys_clone()` → `kernel_clone()` (kernel/fork.c)
2. `copy_process()` → `dup_task_struct()` (duplicate kernel stack)
3. `copy_mm()` → COW page table copy
4. `copy_files()` → duplicate `files_struct`
5. `copy_thread()` → set return value to 0 for child
6. `wake_up_new_task()` → `enqueue_task_fair()`

## Useful Tools for Browsing

```bash
# grep the whole tree
grep -rn "SYSCALL_DEFINE.*read" --include="*.c"

# cscope — interactive source browser
cscope -R
# Then search: function definition, global symbol, grep, etc.

# git log for a file
git log --oneline mm/memory.c | head -20

# git blame for a function
git blame -L 100,150 kernel/sched/core.c

# LXR / Elixir — online cross-reference
# https://elixir.bootlin.com/linux/latest/source
```

## Use It

Understanding Linux source helps with:
- **Systems programming** — writing code that interacts with the kernel (syscalls, io_uring, eBPF)
- **Debugging** — reading kernel oops messages, understanding dmesg output
- **Performance tuning** — understanding how the scheduler, memory manager, and I/O stack actually work
- **Contributing** — fixing bugs, adding features, improving documentation

The kernel-newbies project (https://kernelnewbies.org/) maintains an excellent "kernel reading" guide and beginner-friendly mentoring.

## Read the Source

- `init/main.c` — boot sequence, `start_kernel()` flow
- `kernel/sched/core.c` — scheduler entry points, `schedule()`, `try_to_wake_up()`
- `kernel/sched/fair.c` — CFS implementation, red-black tree, `vruntime`
- `mm/memory.c` — page fault handler, `handle_mm_fault()`
- `mm/slab.c` — slab allocator
- `fs/ext4/ext4.h` — ext4 data structures (superblock, inode, extent)
- `arch/x86/entry/entry_64.S` — syscall entry/exit assembly
- `kernel/fork.c` — process creation, `copy_process()`
- `include/linux/sched.h` — `task_struct` definition (the PCB)

## Coding Style

The kernel uses a strict coding style (see `Documentation/process/coding-style.rst`):
- Tabs for indentation (8-space tab stops)
- Opening brace on the same line as `if`/`for`/`while`
- Functions limited to one or two screens
- `checkpatch.pl` enforces the style on patches

```bash
# Check a patch against coding style
./scripts/checkpatch.pl --file my_driver.c
```

## Ship It

This lesson ships a **kernel reading guide** — a reference map you can use when exploring any kernel subsystem. No compiled artifact; the value is the mental model.

## Exercises

1. **Easy** — Find where `sys_read` is defined. Trace the path from `entry_SYSCALL_64` to the first architecture-independent C function.

2. **Medium** — Read `kernel/sched/fair.c` and explain how `vruntime` is calculated. What is `sched_slice()`, and how does it relate to task weight and period?

3. **Hard** — Read the page fault handler in `mm/memory.c`. Explain the difference between a minor fault (page in memory but not mapped) and a major fault (page must be read from disk). Where in the code does this distinction occur?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| CFS | "Fair scheduler" | Completely Fair Scheduler — red-black tree keyed by virtual runtime |
| vruntime | "Virtual time" | Weighted execution time; tasks with smaller vruntime run next |
| VMA | "Memory region" | Virtual Memory Area — contiguous range of mapped pages with same permissions |
| bzImage | "Kernel image" | Compressed bootable kernel image for x86 |
| task_struct | "Process control block" | The per-task data structure containing all kernel state for a process |
| slab | "Kernel allocator" | Cache-based allocator for frequently allocated objects (inodes, dentries, etc.) |
| LSM | "Security hook" | Linux Security Module — hook framework for SELinux, AppArmor, etc. |
| checkpatch | "Style checker" | Script that enforces kernel coding style on patches |

## Further Reading

- Bovet, D.P. and Cesati, M. (2005). *Understanding the Linux Kernel*, 3rd ed. O'Reilly.
- Love, R. (2010). *Linux Kernel Development*, 3rd ed. Addison-Wesley.
- kernel-newbies.org — beginner-friendly kernel reading guide
- https://elixir.bootlin.com/linux/latest/source — online source browser with cross-references
- https://lwn.net/ — excellent kernel development news and deep dives
