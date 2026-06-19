# Build a Kernel That Boots, Schedules, Pages

> Kernel capstones succeed by proving one hardware-software boundary at a time.

**Type:** Build
**Languages:** C, RISC-V Assembly
**Prerequisites:** Phase 19 lesson 01
**Time:** ~960 minutes

## Learning Objectives

- Design a minimal kernel bring-up sequence.
- Implement bootstrap, basic scheduler loop, and paging scaffold.
- Organize architecture-specific and generic kernel code boundaries.
- Define validation strategy for boot traces and invariants.

## The Problem

Kernel projects fail when too many subsystems are attempted simultaneously. Someone starts with "I'll build an OS," writes a bootloader, gets stuck on protected mode, adds a scheduler before the boot path works, tries to implement page tables while debugging interrupt handling, and ends up with a pile of broken assembly that boots into nothing.

The root cause is that a kernel is not one program. It is a sequence of hardware-software handshake agreements: the firmware hands off to the bootloader, the bootloader sets up the CPU state, the kernel initializes memory management, then processes, then devices. Each transition has precise requirements. Miss one bit in a page table entry and the CPU triple-faults. Miss one register save in a context switch and you corrupt a process's stack.

A phased boot-to-schedule-to-memory plan keeps progress measurable. The first milestone: print "hello" from bare metal. The second: switch from assembly to C. The third: run two toy tasks in a round-robin loop. Each milestone is testable in isolation.

## The Concept

A minimal kernel has three responsibilities, built in order:

```
Power on
  │
  ▼
┌─────────────────┐
│ Boot entry (asm) │  Set up stack, clear BSS, jump to C
└─────────────────┘
  │
  ▼
┌─────────────────┐
│ Trap vector      │  Handle interrupts/exceptions
└─────────────────┘
  │
  ▼
┌─────────────────┐
│ Memory init      │  Set up page tables, enable paging
└─────────────────┘
  │
  ▼
┌─────────────────┐
│ Scheduler        │  Round-robin over kernel tasks
└─────────────────┘
```

On RISC-V, the boot sequence is cleaner than x86: no real-mode-to-protected-mode dance. The CPU starts in M-mode (machine mode), you configure a few CSRs (control/status registers), and jump to S-mode (supervisor mode) where your kernel runs. The RISC-V privileged spec defines exactly what each mode can do.

The scheduler is a function that saves the current CPU register state (the context), picks the next task from a run queue, and restores that task's registers. The timer interrupt drives preemption: every N milliseconds, the hardware fires an interrupt, the trap handler saves context, and the scheduler picks the next task.

Paging translates virtual addresses to physical addresses through a page table. On RISC-V Sv39, the page table is a 3-level radix tree. Each virtual address is split into three 9-bit indices that walk the tree, plus a 12-bit page offset. The final page table entry contains the physical page frame number and permission bits.

## Build It

### Step 1: Boot Entry in Assembly

The bootloader (or firmware) jumps to `_start`. We set up the stack pointer, clear BSS, and call the C entry point.

```asm
# boot.S — RISC-V boot entry
.section .text.init
.global _start

_start:
    # Disable interrupts
    csrw sie, zero
    csrw sip, zero

    # Set up stack pointer
    la   sp, _stack_top

    # Clear BSS section
    la   t0, _bss_start
    la   t1, _bss_end
clear_bss:
    bge  t0, t1, bss_done
    sd   zero, 0(t0)
    addi t0, t0, 8
    j    clear_bss
bss_done:

    # Jump to C kernel entry
    call kmain

    # If kmain returns, spin forever
spin:
    wfi
    j    spin

.section .bss
.align 12
_stack_bottom:
    .space 4096 * 4    # 16 KB kernel stack
_stack_top:
```

The linker script places `.text.init` at the entry address (typically `0x80000000` on QEMU virt). The stack grows downward from `_stack_top`. BSS is zero-initialized memory for uninitialized globals.

### Step 2: Trap Handler

When an interrupt or exception occurs, the CPU jumps to the address in `stvec`. We save all registers, determine the cause, handle timer interrupts, and restore.

```asm
# trap.S — Trap entry/exit
.global trap_entry
.global trap_exit

.align 4
trap_entry:
    # Save all 31 general-purpose registers to the trap frame
    # (stored at the address in tp, which points to current task's context)
    sd   ra,  0*8(tp)
    sd   sp,  1*8(tp)
    sd   gp,  2*8(tp)
    # tp is reserved for the trap frame pointer
    sd   t0,  4*8(tp)
    sd   t1,  5*8(tp)
    sd   t2,  6*8(tp)
    sd   s0,  7*8(tp)
    sd   s1,  8*8(tp)
    sd   a0,  9*8(tp)
    sd   a1, 10*8(tp)
    sd   a2, 11*8(tp)
    sd   a3, 12*8(tp)
    sd   a4, 13*8(tp)
    sd   a5, 14*8(tp)
    sd   a6, 15*8(tp)
    sd   a7, 16*8(tp)
    sd   s2, 17*8(tp)
    sd   s3, 18*8(tp)
    sd   s4, 19*8(tp)
    sd   s5, 20*8(tp)
    sd   s6, 21*8(tp)
    sd   s7, 22*8(tp)
    sd   s8, 23*8(tp)
    sd   s9, 24*8(tp)
    sd  s10, 25*8(tp)
    sd  s11, 26*8(tp)
    sd   t3, 27*8(tp)
    sd   t4, 28*8(tp)
    sd   t5, 29*8(tp)
    sd   t6, 30*8(tp)

    # Save sepc (the interrupted PC)
    csrr t0, sepc
    sd   t0, 31*8(tp)

    # Call C trap handler
    call trap_handler

trap_exit:
    # Restore sepc
    ld   t0, 31*8(tp)
    csrw sepc, t0

    # Restore all registers (same pattern as save, but with ld)
    ld   ra,  0*8(tp)
    ld   sp,  1*8(tp)
    ld   gp,  2*8(tp)
    ld   t0,  4*8(tp)
    ld   t1,  5*8(tp)
    ld   t2,  6*8(tp)
    ld   s0,  7*8(tp)
    ld   s1,  8*8(tp)
    ld   a0,  9*8(tp)
    ld   a1, 10*8(tp)
    ld   a2, 11*8(tp)
    ld   a3, 12*8(tp)
    ld   a4, 13*8(tp)
    ld   a5, 14*8(tp)
    ld   a6, 15*8(tp)
    ld   a7, 16*8(tp)
    ld   s2, 17*8(tp)
    ld   s3, 18*8(tp)
    ld   s4, 19*8(tp)
    ld   s5, 20*8(tp)
    ld   s6, 21*8(tp)
    ld   s7, 22*8(tp)
    ld   s8, 23*8(tp)
    ld   s9, 24*8(tp)
    ld  s10, 25*8(tp)
    ld  s11, 26*8(tp)
    ld   t3, 27*8(tp)
    ld   t4, 28*8(tp)
    ld   t5, 29*8(tp)
    ld   t6, 30*8(tp)

    sret
```

### Step 3: Round-Robin Scheduler

The scheduler is a simple C function that picks the next task from a circular list and context-switches to it.

```c
// scheduler.c — Round-robin kernel scheduler
#include <stdint.h>

#define MAX_TASKS   8
#define STACK_SIZE  4096

typedef enum { TASK_UNUSED, TASK_READY, TASK_RUNNING, TASK_BLOCKED } task_state_t;

typedef struct {
    uint64_t regs[32];     // Saved registers (ra, sp, ..., sepc)
    uint64_t stack[STACK_SIZE / 8];
    task_state_t state;
    int id;
} task_t;

static task_t tasks[MAX_TASKS];
static int current_task = 0;
static int task_count = 0;

// Create a new task running the given function
int task_create(void (*entry)(void)) {
    if (task_count >= MAX_TASKS) return -1;

    task_t *t = &tasks[task_count];
    t->id = task_count;
    t->state = TASK_READY;

    // Set up the initial stack frame
    uint64_t sp = (uint64_t)&t->stack[STACK_SIZE / 8];
    sp &= ~0xF; // Align to 16 bytes
    t->regs[1]  = sp;           // sp
    t->regs[0]  = (uint64_t)task_exit;  // ra (return address)
    t->regs[31] = (uint64_t)entry;      // sepc (entry PC)

    task_count++;
    return t->id;
}

// Pick the next READY task in round-robin order
static int scheduler_pick(void) {
    for (int i = 1; i <= task_count; i++) {
        int idx = (current_task + i) % task_count;
        if (tasks[idx].state == TASK_READY) {
            return idx;
        }
    }
    return current_task; // No other ready task, stay on current
}

// Context switch: save current, restore next
void schedule(void) {
    int next = scheduler_pick();
    if (next == current_task) return;

    tasks[current_task].state = TASK_READY;
    tasks[next].state = TASK_RUNNING;

    task_t *old = &tasks[current_task];
    task_t *new_task = &tasks[next];
    current_task = next;

    // The actual switch happens in assembly: load tp = &new_task->regs,
    // then jump to trap_exit which restores all registers from tp.
    context_switch(old->regs, new_task->regs);
}

// Called by the timer interrupt handler
void timer_tick(void) {
    schedule();
}

// Task cleanup when a task returns
void task_exit(void) {
    tasks[current_task].state = TASK_UNUSED;
    schedule();
    // Should never reach here
    while (1) { __asm__ volatile("wfi"); }
}
```

### Step 4: Page Table Setup

On RISC-V Sv39, a page table is a 3-level radix tree. Each entry is 8 bytes containing a physical page number and flags.

```c
// vm.c — Sv39 page table setup
#include <stdint.h>

#define PAGE_SIZE   4096
#define PT_ENTRIES  512

// Page table entry flags
#define PTE_V   (1 << 0)  // Valid
#define PTE_R   (1 << 1)  // Readable
#define PTE_W   (1 << 2)  // Writable
#define PTE_X   (1 << 3)  // Executable
#define PTE_U   (1 << 4)  // User-accessible

typedef uint64_t pte_t;

// Allocate a page-aligned page table (simplified: bump allocator)
static pte_t page_table_pool[4096 * 16] __attribute__((aligned(PAGE_SIZE)));
static int pool_offset = 0;

static pte_t *alloc_page_table(void) {
    pte_t *table = &page_table_pool[pool_offset];
    pool_offset += PT_ENTRIES;
    // Zero the table
    for (int i = 0; i < PT_ENTRIES; i++) {
        table[i] = 0;
    }
    return table;
}

// Get the physical page number from a pointer
static uint64_t pa_to_ppn(void *pa) {
    return ((uint64_t)pa) >> 12;
}

// Map a virtual address range to a physical address range
// Uses Sv39: 3-level page table with 9-bit indices
void map_page(pte_t *root, uint64_t va, uint64_t pa, uint64_t flags) {
    // Extract the three 9-bit VPN indices
    uint64_t vpn2 = (va >> 30) & 0x1FF;
    uint64_t vpn1 = (va >> 21) & 0x1FF;
    uint64_t vpn0 = (va >> 12) & 0x1FF;

    // Level 2
    if (!(root[vpn2] & PTE_V)) {
        pte_t *child = alloc_page_table();
        root[vpn2] = (pa_to_ppn(child) << 10) | PTE_V;
    }
    pte_t *l1 = (pte_t *)((root[vpn2] >> 10) << 12);

    // Level 1
    if (!(l1[vpn1] & PTE_V)) {
        pte_t *child = alloc_page_table();
        l1[vpn1] = (pa_to_ppn(child) << 10) | PTE_V;
    }
    pte_t *l0 = (pte_t *)((l1[vpn1] >> 10) << 12);

    // Level 0: set the leaf entry
    l0[vpn0] = (pa_to_ppn((void *)pa) << 10) | flags | PTE_V;
}

// Set satp register to enable paging
void activate_page_table(pte_t *root) {
    uint64_t satp = (8LL << 60) | pa_to_ppn(root); // Sv39 mode
    __asm__ volatile("csrw satp, %0" : : "r"(satp));
    __asm__ volatile("sfence.vma"); // Flush TLB
}

// Kernel page table initialization
void vm_init(void) {
    pte_t *kernel_page_table = alloc_page_table();

    // Identity-map the first 128 MB of physical memory
    for (uint64_t pa = 0; pa < 128 * 1024 * 1024; pa += PAGE_SIZE) {
        map_page(kernel_page_table, pa, pa, PTE_R | PTE_W | PTE_X);
    }

    // Map UART device (for console output)
    map_page(kernel_page_table, 0x10000000, 0x10000000, PTE_R | PTE_W);

    activate_page_table(kernel_page_table);
}
```

### Step 5: Kernel Main

Tie everything together.

```c
// kmain.c — Kernel entry point
extern void vm_init(void);
extern int task_create(void (*entry)(void));
extern void schedule(void);

static volatile char *uart = (char *)0x10000000;

void putchar(char c) {
    *uart = c;
}

void puts(const char *s) {
    while (*s) putchar(*s++);
}

void task_a(void) {
    while (1) {
        puts("A");
        for (volatile int i = 0; i < 1000000; i++);
    }
}

void task_b(void) {
    while (1) {
        puts("B");
        for (volatile int i = 0; i < 1000000; i++);
    }
}

void kmain(void) {
    puts("boot: kernel started\n");

    vm_init();
    puts("vm: paging enabled\n");

    task_create(task_a);
    task_create(task_b);
    puts("sched: two tasks created\n");

    // Enable timer interrupts and start scheduling
    // (In a real kernel, configure the CLINT timer here)
    puts("sched: entering scheduler loop\n");
    while (1) {
        schedule();
    }
}
```

## Use It

This structure mirrors real kernel development workflows. Production kernels follow the same layered approach:

- **xv6** (MIT's teaching kernel): the closest reference to what we built. xv6 runs on RISC-V and implements boot, paging, traps, and scheduling in about 10,000 lines of C and assembly. Our design is a simplified version of xv6's architecture.
- **Linux kernel**: the boot path (`arch/riscv/boot/`) follows the same sequence: assembly entry, set up trap vector, initialize memory management, start the scheduler. The scheduler is far more complex (CFS), but the interface is the same: save context, pick next, restore.
- **rCore** (Tsinghua University): an educational RISC-V kernel written in Rust. It implements the same pipeline with Rust's type safety layered on top.
- **seL4**: a formally verified microkernel. The verification proves that the boot path, page table setup, and context switch are correct. Same primitives, mathematically guaranteed.

The key production lesson: **instrument early, instrument everything**. Real kernels log every boot stage, every page table modification, every context switch. When a kernel triple-faults, the boot trace is the only clue you have.

## Read the Source

- [xv6 book](https://pdos.csail.mit.edu/6.828/2023/xv6.html) — The definitive reference for understanding a minimal kernel. Chapter 2 (page tables), chapter 4 (traps), and chapter 7 (scheduling) are directly relevant to this lesson.
- [RISC-V Privileged Spec](https://riscv.org/technical/specifications/) — The authoritative reference for RISC-V privilege modes, page table format, and trap handling. Volume II (Privileged Architecture) defines Sv39 paging and the CSR layout.
- [rCore Tutorial](https://rcore-os.github.io/rCore-Tutorial-Book-v3/) — Build an OS in Rust from scratch on RISC-V. Follows the same boot-to-schedule progression with Rust's ownership model enforcing safety.

## Ship It

- `code/boot.S`: RISC-V boot entry that sets up the stack, clears BSS, and jumps to C.
- `code/kmain.c`: kernel main that initializes paging, creates two tasks, and enters the scheduler loop.
- `outputs/README.md`: kernel milestone checklist covering boot, traps, paging, and scheduling.

## Exercises

1. **Easy** — Add task state transitions. Implement `TASK_BLOCKED` and a `task_wait()` function that blocks the current task until another task signals it. Use a simple flag array for signaling.
2. **Medium** — Add page fault logging. When a page fault occurs (cause 13 or 15 in `scause`), log the faulting address (from `stval`) and the access type (read/write/execute from the instruction at `sepc`). Print a diagnostic message before panicking.
3. **Hard** — Add timer tick accounting. Configure the RISC-V CLINT to fire a timer interrupt every 10ms. In the trap handler, increment a per-task tick counter and call `schedule()` on every tick. Print per-task CPU usage percentages every 100 ticks.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Bootstrapping | "start kernel" | The sequence from hardware reset to the first C function call. On RISC-V: set stack, clear BSS, jump to kmain. On x86: real mode, protected mode, long mode, then C. |
| Scheduler | "task switcher" | The kernel component that decides which task runs next. It saves the current task's CPU registers (context), picks the next task, and restores its registers. Round-robin is the simplest policy. |
| Paging | "virtual memory" | The hardware mechanism that translates virtual addresses to physical addresses through a page table. On RISC-V Sv39, the page table is a 3-level radix tree walked by the MMU. |
| Trap handler | "interrupt code" | The assembly routine at the address in `stvec`. When an interrupt or exception occurs, the CPU jumps here. The handler saves all registers, calls a C function, and restores registers on return. |
| Context switch | "task swap" | Saving one task's CPU state and restoring another's. The saved state includes all 31 general-purpose registers plus the program counter (sepc). The scheduler drives this. |

## Further Reading

- [xv6 book](https://pdos.csail.mit.edu/6.828/2023/xv6.html) — Complete walkthrough of a minimal Unix-like kernel.
- [RISC-V Privileged Spec](https://riscv.org/technical/specifications/) — The hardware specification for RISC-V privilege modes and page tables.
- [Operating Systems: Three Easy Pieces](https://pages.cs.wisc.edu/~remzi/OSTEP/) — Free textbook covering scheduling, virtual memory, and concurrency with clear explanations and projects.
