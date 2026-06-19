# Build an ALU in HDL

> The ALU is the brain's brain — every arithmetic comparison, logic gate, and bit shift a CPU performs routes through this one module.

**Type:** Build
**Languages:** SystemVerilog (HDL)
**Prerequisites:** Phase 06 lessons 01–04
**Time:** ~90 minutes

## Learning Objectives

- Understand the ALU's role as the computational heart of the CPU datapath.
- Implement a complete 32-bit RISC-V ALU with all arithmetic, logic, and shift operations.
- Understand the zero, carry, and overflow flags and why branch instructions depend on them.
- Build the ALU module and testbench, then integrate it into a future CPU datapath.

## The Problem

This lesson sits in **Phase 06 — Digital Logic & Computer Architecture**. Without the concept it teaches, you cannot
build the phase's capstone (A 5-stage pipelined RISC-V CPU in HDL with assembler.). Concretely, *not* knowing this means you get stuck the
moment you try to compute `add x1, x2, x3` in hardware: the ALU is the module that *does the math*.

Every instruction that computes a value — `add`, `sub`, `and`, `or`, `slt`, shifts — executes inside the ALU. Every branch instruction (`beq`, `bne`) reads the ALU's zero flag to decide whether to jump. Without an ALU, your CPU is a fetch-and-decode machine that cannot compute.

## The Concept

### What Is an ALU?

The **Arithmetic-Logic Unit (ALU)** is a combinational circuit that takes two operands and an opcode, then produces a result and status flags. It is the single most referenced module in any CPU datapath.

```
        a [31:0] ─────┐
                      ├──► [ ALU ] ──► result [31:0]
        b [31:0] ─────┤              ► zero
                      │              ► carry
   alu_op [3:0] ──────┘              ► overflow
```

### RISC-V ALU Operations

RISC-V base integer ISA (RV32I) needs these ALU operations, encoded in 4 bits:

| Opcode | Operation | Verilog Expression | Notes |
|--------|-----------|-------------------|-------|
| `0000` | ADD | `a + b` | Also used for address calculation |
| `1000` | SUB | `a - b` | Used by `sub` and `beq` (a - b == 0?) |
| `0111` | AND | `a & b` | Bitwise AND |
| `0110` | OR | `a \| b` | Bitwise OR |
| `0100` | XOR | `a ^ b` | Bitwise XOR |
| `0010` | SLT | `(a < b) ? 1 : 0` | Signed comparison |
| `0011` | SLTU | `(a < b) ? 1 : 0` | Unsigned comparison |
| `0001` | SLL | `a << b[4:0]` | Shift left logical |
| `0101` | SRL | `a >> b[4:0]` | Shift right logical |
| `1101` | SRA | `a >>> b[4:0]` | Shift right arithmetic (sign-extend) |

The opcode bits are not arbitrary — they mirror the `funct3` and `funct7` fields from the RISC-V instruction encoding, so the control unit can wire them directly.

### The Zero Flag

```
zero = (result == 32'b0);
```

One bit. Trivial to compute. Enormously important. When the CPU executes `beq x1, x2, offset`, the ALU computes `x1 - x2`. If the result is zero, the operands are equal, and the branch is taken. `bne` inverts the flag. Without the zero flag, conditional branching requires a separate comparator.

### Carry and Overflow Flags

**Carry flag (unsigned):** Set when an unsigned addition wraps past `2^32 - 1` or a subtraction borrows. Used by `sltu` and multi-word arithmetic.

```
carry = (a + b) overflows 32 bits  // for ADD
```

**Overflow flag (signed):** Set when a signed result exceeds the representable range `[-2^31, 2^31 - 1]`. Two cases:
- Positive + Positive = Negative → overflow
- Negative + Negative = Positive → overflow

```
overflow = (a[31] == b[31]) && (result[31] != a[31])
```

### Implementation Approach

The ALU is a pure combinational circuit — no clock, no state. A `case` statement on the opcode selects which operation produces the result:

```systemverilog
always_comb begin
    case (alu_op)
        ALU_ADD: result = a + b;
        ALU_SUB: result = a - b;
        ALU_AND: result = a & b;
        // ... etc
    endcase
end
```

Each branch is independent and completes in one clock cycle (the result is ready within the same cycle the inputs are stable).

## Build It

All code is in `code/alu.sv`. Walk through each section.

### Step 1: Opcode Parameters

```systemverilog
localparam ALU_ADD  = 4'b0000;
localparam ALU_SUB  = 4'b1000;
localparam ALU_AND  = 4'b0111;
localparam ALU_OR   = 4'b0110;
localparam ALU_XOR  = 4'b0100;
localparam ALU_SLT  = 4'b0010;
localparam ALU_SLTU = 4'b0011;
localparam ALU_SLL  = 4'b0001;
localparam ALU_SRL  = 4'b0101;
localparam ALU_SRA  = 4'b1101;
```

These mirror RISC-V `funct3`/`funct7` encoding so the control unit can pass them directly.

### Step 2: Module Interface

```systemverilog
module alu (
    input  logic [31:0] a,
    input  logic [31:0] b,
    input  logic [3:0]  alu_op,
    output logic [31:0] result,
    output logic        zero,
    output logic        carry,
    output logic        overflow
);
```

Two 32-bit operands, a 4-bit opcode, the result, and three status flags.

### Step 3: The ALU Body

The `always_comb` block implements all operations via `case`. The zero flag is computed from the result. Carry and overflow are computed only for arithmetic operations.

See `code/alu.sv` for the complete implementation — it handles all 10 operations, signed/unsigned comparisons, and the three flags.

## Use It

**Every CPU has an ALU.** In a single-cycle RISC-V processor, the ALU sits in the middle of the datapath:

```
Register File ──► ALU ──► Data Memory / Register File (write-back)
                  ▲
                  │
           Immediate (for I-type / S-type)
```

When `add x1, x2, x3` executes, the register file outputs `x2` and `x3`, the control unit sets `alu_op = ALU_ADD`, and the result writes back to `x1`. When `beq x1, x2, label` executes, the ALU computes `x1 - x2` and the zero flag determines whether the PC loads the branch target.

**In production silicon:** The ALU in Intel/AMD/ARM cores is far more complex — it includes a carry-lookahead adder (not ripple-carry), a Booth multiplier, a divider, and sometimes a floating-point unit. But the basic structure — opcode selects operation, combinational logic computes result — is identical to what you are building here.

## Read the Source

- `code/alu.sv` — Complete 32-bit RISC-V ALU with all operations, flags, and exhaustive testbench.
- [RISC-V spec, Chapter 2](https://riscv.org/technical/specifications/) — The ISA defines which ALU operations each instruction uses.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A 32-bit RISC-V ALU module with all RV32I operations, zero/carry/overflow flags, and a testbench. Drop it into any single-cycle or pipelined CPU design.**

## Exercises

1. **Easy** — Add `MUL` (opcode `0010` with `funct7 = 0000001`) and `DIV` operations to the ALU. Use `*` and `/` operators. Write testbenches verifying: `MUL(0xFFFFFFFF, 2)` produces the lower 32 bits of the product, and `DIV(7, 2)` produces 3 with remainder.

2. **Medium** — Replace the ripple-carry adder (the `+` operator) with a **carry-lookahead adder** inside the ALU. Implement 4-bit CLA blocks and chain 8 of them for 32 bits. Compare critical path delay with the original.

3. **Hard** — Design an ALU that can also perform single-precision IEEE 754 floating-point ADD and MUL. Add a mode input (`int_mode` / `fp_mode`). In FP mode, decode the sign/exponent/mantissa, align, add/multiply, normalize, and round. Handle special cases: infinity, NaN, denormals.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| ALU | "The compute unit" | Arithmetic-Logic Unit: combinational circuit that executes add, sub, and, or, xor, shifts, comparisons |
| Zero flag | "Z flag" | Output = 1 when the ALU result is zero; used by branch instructions to test equality |
| Carry flag | "C flag" | Output = 1 when unsigned arithmetic overflows 32 bits; used for multi-word arithmetic |
| Overflow flag | "V flag" | Output = 1 when signed arithmetic exceeds representable range; traps on signed overflow |
| Opcode | "alu_op" | 4-bit selector choosing which ALU operation to perform |
| SLT | "Set less than" | Result = 1 if a < b (signed), 0 otherwise; used by RISC-V `slt` instruction |
| SLTU | "Set less than unsigned" | Same but unsigned comparison; used by `sltu` |
| SRA | "Shift right arithmetic" | Right shift that sign-extends (fills with sign bit, not zeros) |

## Further Reading

- Patterson & Hennessy, *Computer Organization and Design RISC-V Edition*, Ch. 4 (The Processor) — builds a complete datapath around the ALU.
- Harris & Harris, *Digital Design and Computer Architecture*, Ch. 5 (Digital Building Blocks) — carry-lookahead adders, multipliers, dividers.
- [Ben Eater's 8-bit CPU series](https://eater.net/8) — builds an ALU from discrete logic gates on breadboard. Excellent intuition for how the `case` statement maps to real gates.
