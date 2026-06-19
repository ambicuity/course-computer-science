# RISC-V Assembly — Hands-On

> Lesson 09 gave you the ISA contract. This lesson makes you write the code — data movement, arithmetic, control flow, function calls, syscalls — until the register file feels like home.

**Type:** Build
**Languages:** RISC-V Assembly
**Prerequisites:** Phase 06 lessons 01–09 (especially lesson 09: ISA Design)
**Time:** ~75 minutes

## Learning Objectives

- Move data between registers, memory, and immediates using `lw`, `sw`, `addi`, `lui`, `auipc`.
- Implement arithmetic and logic with `add`, `sub`, `and`, `or`, `xor`, `sll`, `srl`, `slt`.
- Control program flow with conditional branches (`beq`, `bne`, `blt`, `bge`) and jumps (`jal`, `jalr`).
- Write functions that follow the RISC-V calling convention: stack frames, `jal`/`ret`, argument passing.
- Use Linux/RARS ecalls for I/O: `a7=1` print_int, `a7=4` print_string, `a7=10` exit.

## The Problem

Knowing the ISA specification means you can *read* assembly. But the capstone — building a pipelined RISC-V CPU — demands you can *write* it fluently. You need to think in registers, manage the stack by hand, and translate C logic into instruction sequences. There is no compiler to help when you are designing the hardware that *runs* the compiler's output.

## The Concept

### Data Movement

Every RISC-V program begins by loading values into registers. Unlike x86, you cannot operate directly on memory — you must load, compute, then store.

| Instruction | Meaning |
|-------------|---------|
| `li rd, imm` | Load immediate (pseudo → `lui` + `addi`) |
| `lw rd, offset(rs1)` | Load word from memory |
| `sw rs2, offset(rs1)` | Store word to memory |
| `lb rd, offset(rs1)` | Load byte (sign-extended) |
| `la rd, symbol` | Load address (pseudo) |
| `mv rd, rs` | Copy register (pseudo) |
| `lui rd, imm` | Load upper 20 bits |

### Arithmetic and Logic

All arithmetic operates on registers. The only exception is `addi`, which accepts a 12-bit signed immediate.

- **Add/Sub:** `add rd, rs1, rs2` / `sub rd, rs1, rs2` / `addi rd, rs1, imm`
- **Bitwise:** `and`, `or`, `xor` (register-register) and `andi`, `ori`, `xori` (immediate)
- **Shift:** `sll` (left), `srl` (right logical), `sra` (right arithmetic, preserves sign)
- **Compare:** `slt rd, rs1, rs2` — set rd = 1 if rs1 < rs2 (signed)
- **Multiply/Remainder:** `mul`, `rem` — M extension, widely supported

### Control Flow

Branches compare two registers and jump if the condition holds. The offset is PC-relative, encoded in the B-type format.

| Instruction | Condition |
|-------------|-----------|
| `beq rs1, rs2, label` | Branch if equal |
| `bne rs1, rs2, label` | Branch if not equal |
| `blt rs1, rs2, label` | Branch if less than (signed) |
| `bge rs1, rs2, label` | Branch if greater or equal (signed) |
| `bltu rs1, rs2, label` | Branch if less than (unsigned) |
| `bgeu rs1, rs2, label` | Branch if greater or equal (unsigned) |

Unconditional jumps: `jal rd, label` (jump-and-link for calls), `jalr rd, rs1, offset` (indirect jump), and `ret` (pseudo for `jalr x0, ra, 0`).

### The Stack and Function Calls

A function's **stack frame** holds its saved registers, return address, and local variables. The convention:

```
          ┌──────────────────┐  ← old sp (before call)
          │  return address  │
          │  saved s-reg     │
          │  local variables │
  sp ───→ └──────────────────┘  ← new sp (after prologue)
```

**Prologue** (function entry):
```
addi  sp, sp, -N      # allocate N bytes
sw    ra, N-4(sp)     # save return address
sw    s0, N-8(sp)     # save callee-saved registers
```

**Epilogue** (function exit):
```
lw    ra, N-4(sp)     # restore return address
lw    s0, N-8(sp)     # restore callee-saved registers
addi  sp, sp, N       # deallocate
ret                   # jr ra
```

### Syscalls (RARS / Linux)

On RARS or Linux, `ecall` invokes a system service based on `a7`:

| a7 | Service | Inputs |
|----|---------|--------|
| 1 | Print integer | a0 = integer to print |
| 4 | Print string | a0 = address of null-terminated string |
| 5 | Read integer | → a0 = read value |
| 8 | Read string | a0 = buffer, a1 = max length |
| 10 | Exit program | — |
| 11 | Print character | a0 = ASCII code |

## Build It

The `code/` directory contains five programs, each building on the last:

### Step 1: Hello World (`code/hello.s`)

The simplest program: store a string in `.data`, load its address, and call `ecall` with `a7=4` to print it. Demonstrates `la`, `li`, `ecall`, and `.data`/`.text` segments.

### Step 2: Recursive GCD (`code/gcd.s`)

Implements the Euclidean algorithm recursively. Every recursive call follows the full calling convention: allocate a stack frame, save `ra` and original arguments, set up new arguments, call `jal ra, gcd`, restore, and return. Demonstrates `addi sp, sp, -N`, `sw`/`lw` with offsets, `rem`, `beqz`.

### Step 3: Bubble Sort (`code/bubblesort.s`)

Sorts an array in-place with nested loops. The outer loop counts passes; the inner loop compares adjacent elements and swaps if out of order. Demonstrates `slli` for address calculation (index × 4), `ble` for conditional skip, and memory-mapped array access.

### Step 4: Matrix Multiply (`code/matrix_multiply.s`)

Multiplies two 3×3 matrices using triple-nested loops. Demonstrates `mul` for index computation, complex address arithmetic (`base + (i*3 + k) * 4`), and callee-saved registers.

### Step 5: Palindrome Checker (`code/palindrome.s`)

Checks if a string is a palindrome using two pointers moving inward. Demonstrates `lb`, null-terminator scanning, and `bne` for mismatch detection.

Build all programs with:
```
make all
```

Or assemble individually:
```
riscv64-unknown-elf-gcc -march=rv32i -mabi=ilp32 -nostartfiles -o hello hello.s
```

## Use It

**GCC compiles C to exactly this.** To see for yourself:

```bash
echo 'int add(int a, int b) { return a + b; }' > add.c
riscv64-unknown-elf-gcc -S -O1 add.c
cat add.s
```

You will see `add a0, a0, a1` and `ret` — the compiler follows the same calling convention. For complex code, GCC generates stack frames, saves callee-saved registers, and uses `slli`/`add` for array indexing — exactly as shown above.

The Linux kernel (`arch/riscv/kernel/head.S`) uses this same instruction set to boot: stack setup, trap vectors, jump to `start_kernel`.

## Read the Source

- **Linux `arch/riscv/kernel/head.S`** — boot entry for RISC-V Linux: stack setup, trap vector initialization, jump to C code.
- **glibc `sysdeps/riscv/`** — calling convention implementation: `setjmp.S`, `longjmp.S`, function call trampolines.

## Ship It

The reusable artifact is `code/` — five assembly programs that serve as templates for any RV32I task:

- `hello.s` — syscall boilerplate
- `gcd.s` — recursive function with stack frame
- `bubblesort.s` — nested loops and array access
- `matrix_multiply.s` — triple-nested loops and 2D indexing
- `palindrome.s` — string processing and two-pointer technique

Copy any of these as a starting point for new assembly work.

## Exercises

1. **Easy** — Modify `bubblesort.s` to sort in *descending* order. Change only the comparison instruction.

2. **Medium** — Write `fibonacci.s` that prints the first 15 Fibonacci numbers using `ecall` (a7=1 for each number, a7=4 for a newline between them). Follow the calling convention: `fib(n)` takes `n` in `a0` and returns `fib(n)` in `a0`.

3. **Hard** — Implement `matrix_multiply.s` for arbitrary N×N matrices passed as arguments: `a0` = pointer to A, `a1` = pointer to B, `a2` = pointer to C (output), `a3` = N. Use a separate `dot_product` subroutine that computes the dot product of row `i` of A and column `j` of B. The caller must pass `i` and `j` as additional arguments.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Stack frame | "Stack allocation" | Region of stack for one function call: saved ra, saved registers, locals |
| Prologue | "Function setup" | Instructions at function entry: allocate stack, save registers |
| Epilogue | "Function teardown" | Instructions at function exit: restore registers, deallocate stack, return |
| Callee-saved | "Preserved across calls" | Registers the called function must restore (s0–s11) before returning |
| Caller-saved | "Volatile / scratch" | Registers the calling function must save before a call if it needs them (t0–t6, a0–a7) |
| Pseudo-instruction | "Assembler shorthand" | Not a real opcode — the assembler expands it (e.g., `li` → `lui`+`addi`, `ret` → `jalr x0,ra,0`) |
| Ecall | "Environment call / trap" | Instruction that transfers control to the execution environment (OS, simulator) for I/O or exit |

## Further Reading

- Patterson & Hennessy, *Computer Organization and Design: RISC-V Edition*, Ch. 2 — full treatment of assembly language programming.
- RISC-V ISA Specification, Unprivileged ISA, Sections 2.1–2.6 — instruction formats and encodings.
- [RISC-V Assembly Programmer's Manual](https://github.com/riscv-non-isa/riscv-asm-manual) — pseudo-instructions, assembler directives, and conventions.
