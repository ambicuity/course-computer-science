# Lesson 03: Hello World as a Bootloader (in asm + C)

## Overview

Every kernel, from Linux to Windows, starts the same way: assembly code sets up the minimal CPU state, then calls into C where the real work begins. In this lesson you'll build that from scratch—a RISC-V bootloader in assembly that jumps to a C kernel, which prints "Hello, Kernel!" through a UART. No OS, no standard library, no runtime. Just hardware and your code.

---

## Why RISC-V?

RISC-V is an open ISA (Instruction Set Architecture) with clean, orthogonal instructions. Its simplicity makes it ideal for learning boot concepts without drowning in x86's decades of legacy (real mode, segmented memory, A20 gate). The boot principles transfer directly to ARM, x86, or any architecture.

### RISC-V Boot: Where Does the CPU Start?

In the QEMU `virt` machine (the standard RISC-V emulator platform), the CPU begins execution at address `0x80000000`. This is where your bootloader code must be placed.

```
RISC-V Memory Map (QEMU virt)
┌────────────────────┐ 0x100000000 (4 GB)
│                    │
├────────────────────┤ 0x80000000 + ROM size
│  Kernel/Bootloader │ ← loaded here
│  (your code)       │
├────────────────────┤ 0x80000000 (2 GB) ← CPU starts here
│                    │
├────────────────────┤ 0x10000000 + UART regs
│  UART0 (NS16550)   │ ← memory-mapped I/O
├────────────────────┤ 0x10000000 (256 MB)
│                    │
├────────────────────┤ 0x00000000
└────────────────────┘
```

---

## The Architecture

Our minimal kernel has three components:

```
┌─────────────────────────────────────────┐
│                                         │
│  boot.s (RISC-V assembly)               │
│  - Set stack pointer                    │
│  - Call kernel_main()                   │
│  - Infinite loop if kernel_main returns │
│                                         │
│         │                               │
│         ▼                               │
│  kernel.c (C)                           │
│  - putchar() via UART MMIO             │
│  - puts() to print strings             │
│  - kernel_main() prints "Hello, Kernel!"│
│                                         │
│         │                               │
│         ▼                               │
│  linker.ld (Linker Script)              │
│  - Places code at 0x80000000           │
│  - Defines stack location              │
│  - Sets entry point (_start)           │
│                                         │
└─────────────────────────────────────────┘
```

---

## The Bootloader: boot.s

The assembly does three things: declare the entry point, set up the stack, and call the C function.

```asm
# boot.s — RISC-V bootloader
# Assembles to machine code placed at 0x80000000

    .section .text.start          # Place in special section
    .globl _start                 # Export entry point

_start:
    # Set up the stack pointer
    # Stack grows downward from _stack_top (defined in linker script)
    la      sp, _stack_top

    # Call the C kernel entry point
    call    kernel_main

    # If kernel_main ever returns, halt the CPU
halt:
    wfi                           # Wait For Interrupt (low power)
    j       halt                  # Loop forever
```

### What each instruction does:

| Instruction | Purpose |
|---|---|
| `.section .text.start` | Places this code at the very beginning of the binary |
| `.globl _start` | Makes `_start` visible to the linker as the entry point |
| `la sp, _stack_top` | Load address: sets stack pointer to top of stack memory |
| `call kernel_main` | Jump to `kernel_main` and save return address in `ra` |
| `wfi` | Wait For Interrupt — puts CPU in low-power state |
| `j halt` | Unconditional jump — loops forever |

### The Stack

C code requires a stack for local variables, function calls, and saved registers. The assembly sets `sp` (stack pointer) before calling any C function. The stack grows downward in RISC-V: pushing decrements `sp`.

```
Memory
┌──────────────┐  ← _stack_top (sp starts here)
│              │
│   Stack      │  ← grows downward
│   (4 KB)     │
│              │
├──────────────┤  ← _stack_bottom
│              │
│   Kernel     │  ← kernel_main() code
│   code       │
│              │
└──────────────┘  ← 0x80000000
```

---

## The Kernel: kernel.c

The C kernel communicates with the outside world through a UART (Universal Asynchronous Receiver-Transmitter) — a serial device that sends characters one at a time.

### Memory-Mapped I/O

The UART is controlled by reading and writing specific memory addresses. On the QEMU `virt` machine, UART0 lives at `0x10000000`. Writing a byte to this address transmits that character.

```
CPU                    UART Device
 │                        │
 │  Write 0x48 ('H')     │
 │  ─────────────────►    │  THR (Transmit Holding Register)
 │  to address            │  at 0x10000000
 │  0x10000000            │
 │                        │
 │                        │  Character sent over serial
 │                        │  to terminal/console
```

This is called **memory-mapped I/O** — hardware registers are accessed through regular memory addresses. No special I/O instructions needed.

---

## The Code

All four files for this lesson are provided in the `code/` directory:

```
code/
├── boot.s          # RISC-V assembly bootloader
├── kernel.c        # C kernel with UART output
├── linker.ld       # Linker script (memory layout)
└── Makefile        # Build automation
```

### boot.s

The bootloader sets up the stack pointer and calls into C. When `kernel_main()` returns (which it shouldn't), the CPU halts in an infinite loop.

### kernel.c

The kernel defines `putchar()` to write a byte to the UART, `puts()` to print a null-terminated string, and `kernel_main()` as the entry point. It uses `volatile` pointers to ensure the compiler doesn't optimize away memory-mapped I/O writes.

### linker.ld

The linker script tells the linker where each section of the binary goes in memory. The critical line is the starting address `0x80000000`. It also reserves space for the stack.

### Makefile

The Makefile assembles `boot.s`, compiles `kernel.c`, links them with the linker script, and produces a flat binary that QEMU can load directly.

---

## Building and Running

### Prerequisites

You need a RISC-V cross-compilation toolchain and QEMU:

```bash
# Ubuntu/Debian
sudo apt install gcc-riscv64-unknown-elf qemu-system-riscv64

# macOS (Homebrew)
brew tap riscv-software-src/riscv
brew install riscv-gnu-toolchain qemu

# Fedora
sudo dnf install gcc-riscv64-linux-gnu qemu-system-riscv
```

### Build

```bash
cd code/
make
```

This produces `kernel.bin`, a flat binary with the bootloader at the start.

### Run

```bash
make run
```

QEMU starts, the CPU executes from `0x80000000`, the assembly sets up the stack and calls `kernel_main()`, and you see:

```
Hello, Kernel!
```

### Clean

```bash
make clean
```

---

## How It Works: Step by Step

```
1. QEMU loads kernel.bin at 0x80000000
       │
2. CPU begins executing at 0x80000000
       │
3. _start: la sp, _stack_top
       │    Stack pointer is now set
       │
4. call kernel_main
       │    Jump to C code, ra = return address
       │
5. kernel_main:
       │    putchar('H') → write 0x48 to 0x10000000
       │    putchar('e') → write 0x65 to 0x10000000
       │    ... (for each character)
       │    putchar('\n') → write 0x0A to 0x10000000
       │
6. return from kernel_main
       │
7. halt: wfi → j halt
       │    CPU waits forever
       │
8. QEMU shows "Hello, Kernel!" on serial console
```

---

## Debugging with GDB

You can step through your bootloader instruction by instruction:

```bash
# Terminal 1: Start QEMU with GDB server
make debug

# Terminal 2: Connect GDB
riscv64-unknown-elf-gdb kernel.elf
(gdb) target remote :1234
(gdb) break _start
(gdb) continue
(gdb) stepi           # Step one instruction
(gdb) info registers   # See CPU state
(gdb) x/10i $pc       # Disassemble at program counter
```

This lets you watch the stack pointer get set, see the jump to `kernel_main`, and verify UART writes.

---

## Use It

This is how every kernel starts. Linux's `arch/riscv/kernel/head.S` does the same thing at a larger scale: set up page tables, initialize the stack for each CPU core, then call `start_kernel()` in C. The x86 version is more complex (switching from real mode to protected mode to long mode), but the principle is identical.

Understanding this gives you the foundation to:
- Write bare-metal embedded firmware
- Debug kernel boot failures
- Contribute to operating system development
- Understand what happens before `main()` in any program

---

## Ship It

Your minimal kernel is a complete, bootable system:

```
boot.s ──► sets stack, calls C
kernel.c ──► prints "Hello, Kernel!" via UART
linker.ld ──► places code at 0x80000000
Makefile ──► builds and runs in QEMU
```

This is the irreducible core of an operating system. Everything else—process scheduling, virtual memory, file systems—is built on top of this foundation.

---

## Exercises

### Level 1 — Recall

1. At what memory address does the RISC-V CPU begin execution on the QEMU `virt` machine?
2. What is memory-mapped I/O, and what address is UART0 mapped to?
3. What does the `wfi` instruction do?

### Level 2 — Comprehension

1. Explain why the `volatile` keyword is required for the UART pointer in `kernel.c`. What would happen without it?
2. Why does the linker script place `.text` at `0x80000000` instead of `0x0`? What would happen if it used `0x0`?
3. Trace the execution from `_start` to the character `'H'` appearing on the terminal. List every step.

### Level 3 — Application

1. Modify `kernel.c` to print your name instead of "Hello, Kernel!". Add a `puthex()` function that prints a 32-bit value in hexadecimal (useful for debugging addresses).
2. Add a second UART function that reads a character (check the LSR register at offset 5, bit 0 for data ready, then read RBR at offset 0). Make an interactive shell that echoes typed characters.
3. Modify the bootloader to clear the BSS section (zero out memory between `_bss_start` and `_bss_end` from the linker script) before calling `kernel_main()`. Why is this necessary for C programs with global variables?
