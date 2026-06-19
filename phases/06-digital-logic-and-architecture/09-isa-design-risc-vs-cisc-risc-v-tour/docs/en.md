# ISA Design — RISC vs CISC, RISC-V Tour

> An ISA is the contract between hardware and software — the vocabulary and grammar a CPU speaks. Understanding this contract is the hinge between "I built a datapath" and "I can write programs that run on it."

**Type:** Learn
**Languages:** Markdown, RISC-V Assembly
**Prerequisites:** Phase 06 lessons 01–08
**Time:** ~60 minutes

## Learning Objectives

- Define an ISA and explain why it is the hardware/software boundary.
- Contrast RISC and CISC design philosophies on instruction width, operand access, and decode complexity.
- Name the 32 RV32I registers and state their conventional uses.
- Read and write short RISC-V assembly programs using I/R/S/B/U/J formats.
- Apply the RISC-V calling convention for function calls and stack frames.

## The Problem

Lessons 01–08 built up from bits and transistors to ALUs, register files, and datapaths — all hardware. But a datapath without instructions is an engine with no fuel. The ISA defines *which* instructions exist, *what* they do, and *where* operands live. Get the ISA wrong and software must work around hardware forever.

## The Concept

### What Is an ISA?

The **Instruction Set Architecture** is a specification — a contract — that tells software what hardware can do. It defines:

- **Registers** — how many, how wide, which ones have special roles.
- **Instructions** — opcodes, operand encoding, supported operations.
- **Memory model** — address space, endianness, alignment rules.
- **Exceptions/interrupts** — how the CPU reports errors.

Crucially, the ISA says nothing about *how* the hardware implements those instructions. Two chips can share an ISA but differ wildly in pipelining, cache design, and clock speed.

### RISC vs CISC

| Aspect | RISC (Reduced) | CISC (Complex) |
|--------|---------------|----------------|
| Instruction width | Fixed (32-bit) | Variable (1–15 bytes on x86) |
| Operand access | Load/store only — arithmetic operates on registers | Memory operands allowed in ALU instructions |
| Instruction count | Small (~50 base) | Large (hundreds) |
| Decode | Simple, single-cycle | Complex, often microcoded internally |
| Registers | Many (32+) | Fewer (8–16 visible) |
| Examples | RISC-V, ARM, MIPS, SPARC | x86, VAX, 68k |

**Key insight:** Modern CISC CPUs (x86) internally break complex instructions into RISC-like micro-ops. The ISA boundary is "CISC outside, RISC inside." RISC-V skips this translation entirely.

### RISC-V: The Open ISA

RISC-V (pronounced "risk-five") is an open-source ISA maintained by the RISC-V International foundation. No license fees, no patents blocking use. The base integer ISA is called **RV32I** (32-bit) or **RV64I** (64-bit).

#### The 32 Registers

RV32I provides 32 general-purpose registers, each 32 bits wide:

| Register | ABI Name | Role | Preserved? |
|----------|----------|------|------------|
| x0 | zero | Hardwired to 0 | — |
| x1 | ra | Return address | Caller |
| x2 | sp | Stack pointer | Callee |
| x3 | gp | Global pointer | — |
| x4 | tp | Thread pointer | — |
| x5–x7 | t0–t2 | Temporaries | Caller |
| x8 | s0/fp | Saved / frame pointer | Callee |
| x9 | s1 | Saved register | Callee |
| x10–x11 | a0–a1 | Function args / return values | Caller |
| x12–x17 | a2–a7 | Function arguments | Caller |
| x18–x27 | s2–s11 | Saved registers | Callee |
| x28–x31 | t3–t6 | Temporaries | Caller |

**x0 is special:** Every read returns zero; writes are discarded. This eliminates the need for a dedicated "nop" encoding (`addi x0, x0, 0`), clears, and zero registers.

#### Instruction Formats

All RV32I instructions are exactly 32 bits wide. There are six formats:

```
 31      25 24    20 19    15 14 12 11     7 6      0
┌─────────┬────────┬────────┬─────┬────────┬────────┐
│ funct7  │  rs2   │  rs1   │funct3│   rd  │ opcode │  R-type
├─────────┼────────┬────────┼─────┬────────┬────────┤
│ imm[11:0]       │  rs1   │funct3│   rd  │ opcode │  I-type
├─────────┼────────┬────────┼─────┬────────┬────────┤
│imm[11:5]│  rs2   │  rs1   │funct3│imm[4:0]│opcode │  S-type
├─────────┼────────┬────────┼─────┬────────┬────────┤
│imm[12│  │  rs2   │  rs1   │funct3│imm[4:1││opcode │  B-type
│ 10:5]  │        │        │     │11]     │        │
├─────────┼────────┬────────┼─────┬────────┬────────┤
│ imm[31:12]                 │   rd  │ opcode │  U-type
├─────────┼────────┬────────┼─────┬────────┬────────┤
│imm[20│  │imm[10:1│imm[11  │   rd  │ opcode │  J-type
│10:1│11│ 19:12]  │]│imm[19:12]│        │
└─────────┴────────┴────────┴─────┴────────┴────────┘
```

- **R-type:** Register-register arithmetic (`add`, `sub`, `and`, `or`, `xor`, `sll`, `srl`, `sra`, `slt`, `sltu`)
- **I-type:** Immediate arithmetic, loads, JALR, ECALL (`addi`, `lw`, `jalr`)
- **S-type:** Stores (`sw`, `sh`, `sb`)
- **B-type:** Branches (`beq`, `bne`, `blt`, `bge`, `bltu`, `bgeu`)
- **U-type:** Upper immediate (`lui`, `auipc`)
- **J-type:** Jumps (`jal`)

#### Calling Convention Summary

| What | Where |
|------|-------|
| Arguments 0–7 | a0–a7 |
| Return value | a0 (and a1 for 64-bit pairs) |
| Return address | ra (x1) |
| Stack pointer | sp (x2) — grows **downward** |
| Caller-saved | t0–t6, a0–a7 |
| Callee-saved | s0–s11 |
| Function call | `jal ra, func` |
| Function return | `ret` (pseudo for `jalr x0, ra, 0`) |

## Build It

The file `code/programs.s` contains five programs that exercise every core RV32I pattern:

1. **`sum_1_to_n`** — iterative loop with a counter and accumulator. Demonstrates `li`, `add`, `addi`, `bgt`, `j`.
2. **`factorial`** — recursive function. Demonstrates stack frame setup (`addi sp, sp, -8`), `sw`/`lw` to save `ra` and `a0`, `jal` for recursion, `mul`.
3. **`fibonacci`** — iterative with three running variables. Demonstrates `mv`, `blt`, `bgt`, register shifting.
4. **`string_length`** — walk a null-terminated string byte-by-byte. Demonstrates `la`, `lb`, `beqz`, pointer arithmetic.
5. **`array_sum`** — sum a `.word` array. Demonstrates `lw` with stride 4, `beqz` loop termination.

Study each one. Trace the register values by hand for the first 3 iterations. This is how you build fluency.

## Use It

- **ARM** (Apple Silicon, Cortex-A/M) is also RISC. AArch64 has 31 general-purpose registers (X0–X30), fixed 32-bit instructions, and a load/store architecture. If you understand RISC-V, ARM will feel familiar.
- **x86-64** is CISC. A single `add [rbx+rcx*4+16], eax` does a memory read-modify-write in one instruction. Internally the CPU cracks this into 3+ micro-ops. x86-64 has only 16 visible registers (RAX–R15), so code spills to the stack constantly.
- **MIPS** pioneered many ideas RISC-V adopted: 32 registers, fixed-width, load/store. RISC-V simplified MIPS by removing branch-delay slots and load-delay slots.

## Read the Source

- **RISC-V ISA Specification** — [riscv.org/technical/specifications](https://riscv.org/technical/specifications/) — the definitive reference.
- **RISC-V Card** — [github.com/riscv/riscv-card](https://github.com/riscv/riscv-card) — one-page cheat sheet.

## Ship It

The reusable artifact produced by this lesson lives in `code/programs.s`. It is:

- **A reference file of five RISC-V assembly programs** covering loops, recursion, strings, and arrays — the building blocks for every assembly task in lessons 10–22.

## Exercises

1. **Easy** — Write a RISC-V assembly function `max(a0, a1)` that returns the larger of two values. Use only `blt` and `mv`.

2. **Medium** — Implement `strlen` using a recursive approach instead of a loop. Follow the full calling convention (save `ra`, allocate stack frame). Compare the instruction count against the iterative version.

3. **Hard** — Write a program that computes `pow(base, exp)` using repeated squaring (not naive multiplication). Handle `exp = 0` returning 1. The algorithm should run in O(log n) multiplications.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| ISA | "The CPU's language" | The specification of registers, instructions, and memory model that software targets |
| RISC | "Simple instructions" | Fixed-width, load/store-only, many registers, simple decode |
| CISC | "Complex instructions" | Variable-width, memory operands, fewer registers, microcoded decode |
| ABI | "Calling convention" | Agreement on register roles, stack layout, and how functions pass arguments |
| Micro-op | "uop" | The RISC-like internal operation a CISC decoder emits for a single complex instruction |
| Pseudo-instruction | "Pseudo" | Assembly shorthand expanded by the assembler (e.g., `li` → `lui`+`addi`, `mv` → `addi`) |
| x0 / zero | "Hardwired zero" | Register that always reads 0; writes are discarded — simplifies encoding of nop, move, compare |

## Further Reading

- Patterson & Hennessy, *Computer Organization and Design: RISC-V Edition*, Ch. 1–2.
- Waterman & Asanović, *The RISC-V Reader* (free PDF).
- [RISC-V Bytes](https://danielmangum.com/categories/risc-v-bytes/) — blog series on RISC-V instruction encoding.
