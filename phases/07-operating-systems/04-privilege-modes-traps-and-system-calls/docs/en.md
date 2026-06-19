# Lesson 04: Privilege Modes, Traps, and System Calls

## Why This Matters

Every time you call `printf()`, the CPU switches from your program's limited privileges to the kernel's full privileges, executes a write to the screen, and switches back. Understanding this mechanism — traps and system calls — is understanding how the OS enforces isolation between hundreds of processes.

## Privilege Levels

Hardware enforces multiple privilege levels. The CPU runs at different levels depending on whose code it's executing.

### RISC-V Privilege Modes

| Mode | Who | What It Can Do |
|------|-----|----------------|
| M-mode (Machine) | Firmware / bootloader | Full access to hardware, memory, CSRs. Bare metal. |
| S-mode (Supervisor) | Kernel | Virtual memory, interrupt handling, device access. |
| U-mode (User) | Applications | Restricted memory access, no hardware control. |

### x86 Ring Model

| Ring | Who | Use |
|------|-----|-----|
| Ring 0 | Kernel | Full hardware access |
| Ring 1–2 | Device drivers (rarely used) | Partial access |
| Ring 3 | Applications | Restricted |

The key rule: **code running at a lower privilege level cannot directly access hardware or other processes' memory**. It must request service from a higher privilege level.

## Kernel Mode vs User Mode

```
┌─────────────────────────────────────────┐
│              User Mode (U/S)            │
│  ┌─────────┐  ┌─────────┐  ┌────────┐  │
│  │  App A   │  │  App B   │  │ App C  │  │
│  │ (printf) │  │ (read)   │  │(write) │  │
│  └────┬─────┘  └────┬─────┘  └───┬────┘  │
│       │ecall        │ecall       │ecall   │
├───────┼─────────────┼────────────┼────────┤
│       ▼             ▼            ▼        │
│          Kernel Mode (S/Ring 0)           │
│  ┌────────────────────────────────────┐   │
│  │  Trap Handler → Syscall Dispatcher │   │
│  │  sys_write  sys_read  sys_fork    │   │
│  └────────────────────────────────────┘   │
│                  │ sret / iret             │
└──────────────────┼────────────────────────┘
                   ▼
              User Mode (resume)
```

## Traps

A **trap** is a synchronous transfer of control from user mode to kernel mode. Three causes:

### 1. Interrupts (Asynchronous, External)

Hardware signals the CPU. Examples: timer interrupt, keyboard input, network packet arrival. The CPU pauses the current program and jumps to the interrupt handler.

### 2. Exceptions (Synchronous, Internal)

The program itself caused a problem. Examples: divide by zero, page fault (accessing unmapped memory), invalid instruction. The CPU jumps to the exception handler.

### 3. System Calls (Synchronous, Intentional)

The program *asks* the kernel for a service. The `ecall` (RISC-V) or `syscall` (x86) instruction triggers a deliberate trap into kernel mode.

## The Trap Handler

When a trap fires, the kernel must:

```
1. SAVE CONTEXT
   - Save all general-purpose registers (a0–a31, t0–t6, s0–s11, ra, sp, gp, tp)
   - Save the program counter (PC) — where to resume
   - Save the status register (current privilege level)

2. DISPATCH
   - Classify: interrupt? exception? syscall?
   - If syscall: read a7 for syscall number, jump to handler table[a7]

3. EXECUTE
   - Perform the requested operation
   - Place return value in a0

4. RESTORE CONTEXT
   - Reload all saved registers
   - Execute sret (RISC-V) or iret (x86) to return to user mode
```

### RISC-V Trap Registers (CSRs)

| Register | Purpose |
|----------|---------|
| `stvec` | Address of the trap handler |
| `sepc` | PC where the trap occurred |
| `scause` | Trap cause (interrupt/exception code) |
| `stval` | Additional trap info (e.g., faulting address) |
| `sscratch` | Scratch register (pointer to trap frame) |
| `sstatus` | Status register (previous privilege mode) |

## System Call Interface

A user program makes a syscall by:

1. Loading the syscall number into register `a7`
2. Loading arguments into `a0` through `a6` (up to 7 args)
3. Executing `ecall`
4. Receiving the return value in `a6`

Common RISC-V Linux syscall numbers:

| Number | Name | Description |
|--------|------|-------------|
| 63 | `sys_read` | Read from file descriptor |
| 64 | `sys_write` | Write to file descriptor |
| 93 | `sys_exit` | Terminate process |
| 220 | `sys_fork` | Create child process |

## Build It

We'll write a user-space program that makes system calls using inline RISC-V assembly, and a skeleton trap handler showing context save/restore.

## Use It

Every C library call eventually funnels through this path:

- `printf("hello")` → `write(1, "hello", 5)` → `ecall` with `a7=64`
- `scanf(...)` → `read(0, buf, n)` → `ecall` with `a7=63`
- `exit(0)` → `ecall` with `a7=93`

The kernel's `trap_handler` reads `a7`, indexes into a syscall table, calls the right kernel function, and returns via `sret`.

## Ship It

See `code/main.c` for a working syscall demonstration with inline assembly and a trap handler skeleton.

## Exercises

### Level 1 — Recall

List the three privilege modes on RISC-V. Which mode does your application run in? Which mode does the kernel run in?

### Level 2 — Application

Trace the execution path when your program calls `write(1, "hi", 2)`. At each step, specify: which privilege mode, which registers hold the syscall number and arguments, and what instruction causes the transition.

### Level 3 — Build

Extend the trap handler skeleton in `code/main.c` to handle at least three syscall numbers (write, exit, and one custom syscall). The custom syscall should return the current value of a counter that increments on each call. Test it from user code.
