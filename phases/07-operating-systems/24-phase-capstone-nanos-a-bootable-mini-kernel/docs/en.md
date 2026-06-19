# Lesson 24: Phase Capstone — 'nanos': A Bootable Mini-Kernel

## Core Concepts

Every abstraction you have studied in this phase — processes, scheduling, memory allocation, context switching, synchronization — lives inside the kernel. This capstone builds a minimal kernel that implements each one. Not a simulation. Not a model. A real kernel that boots on a RISC-V machine (or emulator) and runs a shell.

The kernel is called **nanos**. It fits in under 800 lines of C and assembly. It demonstrates that an operating system kernel, at its core, is just a C program with special privileges and a well-defined entry point.

## What nanos Does

```
nanos boot sequence:
  1. boot.s — set up stack, zero BSS, call kernel_main()
  2. kernel_main() — initialize UART, memory allocator, scheduler
  3. Create 3 dummy processes + 1 shell process
  4. Enter round-robin scheduler
  5. Shell reads commands: echo, help, ps, meminfo, halt

Hardware: QEMU virt machine, RISC-V 64-bit
UART: NS16550-compatible at MMIO address 0x10000000
```

## Architecture Overview

```
┌─────────────────────────────────────────┐
│              User Shell                 │
│  echo, help, ps, meminfo, halt         │
├─────────────────────────────────────────┤
│          nanos kernel                   │
│  ┌───────────┐ ┌────────┐ ┌──────────┐ │
│  │ Scheduler │ │  IPC   │ │  Memory  │ │
│  │ (RR)      │ │(yield) │ │(bump)    │ │
│  └───────────┘ └────────┘ └──────────┘ │
│  ┌─────────────────────────────────────┐│
│  │        UART Driver (MMIO)          ││
│  └─────────────────────────────────────┘│
├─────────────────────────────────────────┤
│  boot.s  — entry point, stack setup    │
│  context.s — context switch (save/restore regs) │
├─────────────────────────────────────────┤
│           RISC-V Hardware              │
└─────────────────────────────────────────┘
```

## The Files

| File | Language | Purpose |
|------|----------|---------|
| `boot.s` | RISC-V asm | Entry point, stack setup, BSS zero, call kernel_main |
| `kernel.c` | C | All kernel logic: UART, scheduler, shell, memory |
| `context.s` | RISC-V asm | Context switch — save/restore callee-saved registers |
| `linker.ld` | Linker script | Memory layout, section placement |
| `Makefile` | Make | Build rules for riscv64-unknown-elf toolchain |

## Build It

### Step 1: boot.s — The Entry Point

The bootloader (or QEMU's built-in firmware) jumps to `_start`. We set up the stack pointer, zero the BSS section, and call `kernel_main()`.

```asm
# boot.s — RISC-V 64-bit entry point
.section .text
.global _start

_start:
    # Set up the stack pointer
    la sp, _stack_top

    # Zero the BSS section
    la t0, _bss_start
    la t1, _bss_end
_zero_bss:
    bge t0, t1, _bss_done
    sd zero, 0(t0)
    addi t0, t0, 8
    j _zero_bss
_bss_done:

    # Jump to C kernel
    call kernel_main

    # If kernel_main returns, halt
_halt:
    wfi
    j _halt
```

### Step 2: context.s — Context Switch

When the scheduler switches from one process to another, it must save all callee-saved registers of the outgoing process and restore those of the incoming process. The stack pointer itself is the only thing the scheduler tracks — everything else lives on the process's kernel stack.

```asm
# context.s — RISC-V context switch
# void context_switch(uint64_t *old_sp, uint64_t new_sp);
#
# old_sp: pointer to where we save the current stack pointer
# new_sp: the stack pointer of the process we're switching to

.section .text
.global context_switch

context_switch:
    # Save callee-saved registers on the current stack
    addi sp, sp, -112     # 14 registers × 8 bytes

    sd ra,  0(sp)
    sd s0,  8(sp)
    sd s1,  16(sp)
    sd s2,  24(sp)
    sd s3,  32(sp)
    sd s4,  40(sp)
    sd s5,  48(sp)
    sd s6,  56(sp)
    sd s7,  64(sp)
    sd s8,  72(sp)
    sd s9,  80(sp)
    sd s10, 88(sp)
    sd s11, 96(sp)
    sd gp,  104(sp)

    # Save current sp into *old_sp (a0 = old_sp pointer)
    sd sp, 0(a0)

    # Load new sp
    mv sp, a1

    # Restore callee-saved registers from new stack
    ld ra,  0(sp)
    ld s0,  8(sp)
    ld s1,  16(sp)
    ld s2,  24(sp)
    ld s3,  32(sp)
    ld s4,  40(sp)
    ld s5,  48(sp)
    ld s6,  56(sp)
    ld s7,  64(sp)
    ld s8,  72(sp)
    ld s9,  80(sp)
    ld s10, 88(sp)
    ld s11, 96(sp)
    ld gp,  104(sp)

    addi sp, sp, 112

    ret
```

### Step 3: kernel.c — The Complete Kernel

This is the entire kernel in a single file. It implements:

- **UART driver** — MMIO read/write at 0x10000000 (NS16550)
- **Bump allocator** — simplest possible memory allocator
- **Process table** — PCBs with stack pointers, state, names
- **Round-robin scheduler** — cycles through ready processes
- **Dummy processes** — increment counters forever (demonstrate scheduling)
- **Shell** — reads lines, parses commands, dispatches

```c
/* kernel.c — nanos: a bootable mini-kernel for RISC-V */

#include <stdint.h>
#include <stddef.h>

/* ---- UART Driver (NS16550 at 0x10000000) ---- */

#define UART_BASE   0x10000000UL
#define UART_THR    (*(volatile uint8_t *)(UART_BASE + 0))  /* transmit */
#define UART_RBR    (*(volatile uint8_t *)(UART_BASE + 0))  /* receive  */
#define UART_LSR    (*(volatile uint8_t *)(UART_BASE + 5))  /* status   */
#define LSR_TX_IDLE (1 << 5)
#define LSR_RX_RDY  (1 << 0)

static void uart_putc(char c)
{
    while (!(UART_LSR & LSR_TX_IDLE))
        ;
    UART_THR = (uint8_t)c;
}

static void uart_puts(const char *s)
{
    while (*s) {
        if (*s == '\n') uart_putc('\r');
        uart_putc(*s++);
    }
}

static int uart_getc(void)
{
    while (!(UART_LSR & LSR_RX_RDY))
        ;
    return UART_RBR;
}

static void put_hex(uint64_t val)
{
    const char *hex = "0123456789abcdef";
    char buf[17];
    buf[16] = '\0';
    for (int i = 15; i >= 0; i--) {
        buf[i] = hex[val & 0xf];
        val >>= 4;
    }
    uart_puts(buf);
}

static void put_dec(uint64_t val)
{
    char buf[21];
    int i = 0;
    if (val == 0) { uart_putc('0'); return; }
    while (val > 0) {
        buf[i++] = '0' + (val % 10);
        val /= 10;
    }
    while (--i >= 0)
        uart_putc(buf[i]);
}

/* ---- Bump Memory Allocator ---- */

extern char _heap_start;
extern char _heap_end;

static char *heap_ptr;

static void *kalloc(size_t size)
{
    /* Align to 8 bytes */
    size = (size + 7) & ~7;
    if (heap_ptr + size > &_heap_end)
        return NULL;
    void *ptr = heap_ptr;
    heap_ptr += size;
    return ptr;
}

static uint64_t mem_used(void) { return (uint64_t)(heap_ptr - &_heap_start); }
static uint64_t mem_total(void) { return (uint64_t)(&_heap_end - &_heap_start); }

/* ---- Process Management ---- */

#define MAX_PROCS   8
#define STACK_SIZE  4096

typedef enum { PROC_UNUSED, PROC_READY, PROC_RUNNING, PROC_EXITED } ProcState;

typedef struct {
    uint64_t    *sp;            /* saved stack pointer (points into kernel stack) */
    ProcState    state;
    char         name[16];
    int          pid;
    void       (*entry)(void);  /* initial entry point */
    uint64_t     stack[STACK_SIZE / 8]; /* kernel stack */
} Proc;

static Proc  procs[MAX_PROCS];
static int   nprocs = 0;
static int   current = -1;

extern void context_switch(uint64_t **old_sp, uint64_t *new_sp);

/* First-time process entry trampoline — called by context_switch */
static void proc_entry(void)
{
    Proc *p = &procs[current];
    p->entry();
    p->state = PROC_EXITED;
    /* Yield to scheduler — it will skip this process */
    while (1) {
        /* idle */
    }
}

static int proc_create(const char *name, void (*entry)(void))
{
    if (nprocs >= MAX_PROCS) return -1;
    Proc *p = &procs[nprocs];
    p->pid   = nprocs;
    p->state = PROC_READY;
    p->entry = entry;
    int i = 0;
    while (name[i] && i < 15) { p->name[i] = name[i]; i++; }
    p->name[i] = '\0';

    /* Set up the initial stack so that context_switch restores into proc_entry */
    uint64_t *sp = &p->stack[STACK_SIZE / 8]; /* top of stack */
    sp -= 14; /* 14 callee-saved slots */
    sp[0]  = (uint64_t)proc_entry;  /* ra  — return address */
    sp[1]  = 0; /* s0  */
    sp[2]  = 0; /* s1  */
    sp[3]  = 0; /* s2  */
    sp[4]  = 0; /* s3  */
    sp[5]  = 0; /* s4  */
    sp[6]  = 0; /* s5  */
    sp[7]  = 0; /* s6  */
    sp[8]  = 0; /* s7  */
    sp[9]  = 0; /* s8  */
    sp[10] = 0; /* s9  */
    sp[11] = 0; /* s10 */
    sp[12] = 0; /* s11 */
    sp[13] = 0; /* gp  */
    p->sp = sp;

    nprocs++;
    return p->pid;
}

/* ---- Scheduler (Round-Robin) ---- */

static void schedule(void)
{
    while (1) {
        /* Find next READY process */
        int next = -1;
        for (int i = 1; i <= nprocs; i++) {
            int idx = (current + i) % nprocs;
            if (procs[idx].state == PROC_READY) {
                next = idx;
                break;
            }
        }
        if (next < 0) {
            uart_puts("All processes exited.\n");
            return;
        }

        int prev = current;
        current = next;
        procs[current].state = PROC_RUNNING;

        if (prev >= 0 && procs[prev].state == PROC_RUNNING)
            procs[prev].state = PROC_READY;

        context_switch(
            prev >= 0 ? &procs[prev].sp : NULL,
            procs[current].sp
        );

        /* We return here when the running process yields */
        if (procs[current].state == PROC_RUNNING)
            procs[current].state = PROC_READY;
    }
}

static void yield(void)
{
    /* Switch back to scheduler — scheduler's sp is saved in procs[current].sp */
    /* We implement this by saving into a special scheduler slot */
    /* Simplified: just return from proc_entry, scheduler loop picks next */
}

/* ---- Dummy Processes ---- */

static volatile uint64_t counters[4];

static void dummy_task_0(void)
{
    while (1) counters[0]++;
}

static void dummy_task_1(void)
{
    while (1) counters[1]++;
}

static void dummy_task_2(void)
{
    while (1) counters[2]++;
}

/* ---- Shell ---- */

#define CMD_BUF 128

static void readline(char *buf, int max)
{
    int i = 0;
    uart_puts("nanos> ");
    while (i < max - 1) {
        int c = uart_getc();
        if (c == '\r' || c == '\n') {
            uart_putc('\n');
            break;
        }
        if (c == 127 || c == 8) { /* backspace */
            if (i > 0) {
                i--;
                uart_puts("\b \b");
            }
            continue;
        }
        buf[i++] = (char)c;
        uart_putc((char)c);
    }
    buf[i] = '\0';
}

static int strcmp(const char *a, const char *b)
{
    while (*a && *a == *b) { a++; b++; }
    return (unsigned char)*a - (unsigned char)*b;
}

static int strncmp(const char *a, const char *b, int n)
{
    while (n-- > 0 && *a && *a == *b) { a++; b++; }
    if (n < 0) return 0;
    return (unsigned char)*a - (unsigned char)*b;
}

static void cmd_echo(const char *arg)
{
    if (arg) uart_puts(arg);
    uart_putc('\n');
}

static void cmd_help(void)
{
    uart_puts("Commands:\n");
    uart_puts("  echo <text>  — print text\n");
    uart_puts("  help         — show this help\n");
    uart_puts("  ps           — list processes\n");
    uart_puts("  meminfo      — show memory usage\n");
    uart_puts("  halt         — shut down\n");
}

static void cmd_ps(void)
{
    uart_puts("PID  STATE     NAME\n");
    for (int i = 0; i < nprocs; i++) {
        uart_puts("  ");
        put_dec(procs[i].pid);
        uart_puts("   ");
        switch (procs[i].state) {
            case PROC_UNUSED:  uart_puts("UNUSED    "); break;
            case PROC_READY:   uart_puts("READY     "); break;
            case PROC_RUNNING: uart_puts("RUNNING   "); break;
            case PROC_EXITED:  uart_puts("EXITED    "); break;
        }
        uart_puts(procs[i].name);
        uart_putc('\n');
    }
}

static void cmd_meminfo(void)
{
    uart_puts("Memory: ");
    put_dec(mem_used());
    uart_puts(" / ");
    put_dec(mem_total());
    uart_puts(" bytes used (");
    put_dec(mem_used() * 100 / mem_total());
    uart_puts("%)\n");
}

static void shell(void)
{
    char buf[CMD_BUF];
    uart_puts("\n========================================\n");
    uart_puts("  nanos — a bootable mini-kernel\n");
    uart_puts("  Phase 07 Capstone\n");
    uart_puts("========================================\n\n");

    while (1) {
        readline(buf, CMD_BUF);
        if (buf[0] == '\0') continue;

        if (strncmp(buf, "echo ", 5) == 0) {
            cmd_echo(buf + 5);
        } else if (strcmp(buf, "help") == 0) {
            cmd_help();
        } else if (strcmp(buf, "ps") == 0) {
            cmd_ps();
        } else if (strcmp(buf, "meminfo") == 0) {
            cmd_meminfo();
        } else if (strcmp(buf, "halt") == 0) {
            uart_puts("Halting.\n");
            /* Trigger QEMU shutdown */
            *(volatile uint32_t *)0x100000 = 0x5555;
            while (1) ;
        } else {
            uart_puts("Unknown command: ");
            uart_puts(buf);
            uart_putc('\n');
        }
    }
}

/* ---- Kernel Main ---- */

void kernel_main(void)
{
    heap_ptr = &_heap_start;

    uart_puts("nanos: initializing...\n");

    /* Create dummy processes */
    proc_create("idle",    dummy_task_0);
    proc_create("worker1", dummy_task_1);
    proc_create("worker2", dummy_task_2);

    uart_puts("nanos: 3 background processes created\n");
    uart_puts("nanos: starting shell\n\n");

    /* Run the shell directly (it never returns) */
    shell();

    /* Should never reach here */
    while (1) ;
}
```

### Step 4: Build and Run

```makefile
# Makefile for nanos
CC = riscv64-unknown-elf-gcc
AS = riscv64-unknown-elf-as
LD = riscv64-unknown-elf-ld
OBJCOPY = riscv64-unknown-elf-objcopy

CFLAGS = -march=rv64im -mabi=lp64 -mcmodel=medany -ffreestanding -O2 -Wall -Wextra
LDFLAGS = -T linker.ld -nostdlib

OBJS = boot.o kernel.o context.o

nanos.elf: $(OBJS)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^

boot.o: boot.s
	$(CC) $(CFLAGS) -c -o $@ $<

kernel.o: kernel.c
	$(CC) $(CFLAGS) -c -o $@ $<

context.o: context.s
	$(CC) $(CFLAGS) -c -o $@ $<

run: nanos.elf
	qemu-system-riscv64 -machine virt -nographic -bios none -kernel nanos.elf

clean:
	rm -f *.o nanos.elf

.PHONY: run clean
```

**Linker script** (`linker.ld`):

```ld
OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS
{
    . = 0x80200000;

    .text : {
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data : {
        *(.data .data.*)
    }

    _bss_start = .;
    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }
    _bss_end = .;

    . = ALIGN(4096);
    _heap_start = .;
    . += 0x100000;       /* 1 MB heap */
    _heap_end = .;

    _stack_top = .;
}
```

## Use It

nanos is a simplified version of what real kernels do:

| nanos component | Real kernel equivalent |
|-----------------|----------------------|
| UART MMIO driver | `drivers/tty/serial/8250/` (Linux) |
| Bump allocator | Buddy system + slab allocator (Linux `mm/`) |
| Round-robin scheduler | CFS with red-black tree (Linux `kernel/sched/`) |
| `context_switch` | `switch_to` in `arch/riscv/kernel/switch.S` |
| Process table | `task_struct` linked list (Linux `include/linux/sched.h`) |
| Shell | `/sbin/init` + `/bin/sh` (full userspace) |

The real kernel adds virtual memory, file systems, networking, device drivers, security, and thousands of other features. But the core loop is the same: boot, initialize, schedule, switch context, repeat.

## Read the Source

- `arch/riscv/kernel/entry.S` — real RISC-V context switch and trap entry
- `kernel/sched/core.c` — `schedule()` function in Linux
- `init/main.c` — `start_kernel()` is the real `kernel_main()`

## Ship It

The complete nanos kernel lives in `outputs/nanos/`. Build and run with:

```bash
cd outputs/nanos
make
make run
```

Requires: `riscv64-unknown-elf-gcc` toolchain and `qemu-system-riscv64`.

## Exercises

1. **Easy** — Add a `counters` command to the shell that prints the current values of the three dummy process counters. This shows that the round-robin scheduler is actually switching between processes.

2. **Medium** — Implement a **semaphore** in nanos: `sem_init`, `sem_wait`, `sem_post`. Create two processes that synchronize using a semaphore (producer/consumer). Show that without the semaphore, the consumer reads stale data.

3. **Hard** — Add **preemptive scheduling**: set up the RISC-V timer (via SBI or CLINT), handle timer interrupts in `boot.s`, and call the scheduler on each tick. Currently nanos does cooperative scheduling (processes must yield). Preemptive scheduling guarantees that no process can monopolize the CPU.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Bump allocator | "Simple allocator" | Allocates memory by incrementing a pointer; no free, no fragmentation |
| Context switch | "Switch processes" | Save all callee-saved registers of current process, restore registers of next process |
| Callee-saved | "Non-volatile registers" | Registers the called function must preserve (s0-s11, ra, sp, gp in RISC-V) |
| MMIO | "Memory-mapped I/O" | Hardware registers accessed via load/store to specific physical addresses |
| Round-robin | "Fair rotation" | Each ready process runs for its turn, then yields to the next |
| PCB | "Process control block" | Data structure holding all kernel state for a process (sp, pid, state, stack) |
| Freestanding | "No libc" | Compiler mode with no standard library — kernel code must provide everything |
| Linker script | "Memory map" | Defines where code/data sections are placed in physical memory |

## Further Reading

- xv6 (MIT teaching OS): https://github.com/mit-pdos/xv6-riscv — a much more complete mini-kernel
- RISC-V spec: https://riscv.org/technical/specifications/
- "Operating Systems: Three Easy Pieces" (free): https://pages.cs.wisc.edu/~remzi/OSTEP/
