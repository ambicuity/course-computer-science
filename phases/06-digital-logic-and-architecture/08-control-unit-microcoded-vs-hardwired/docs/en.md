# Control Unit — Microcoded vs Hardwired

> The control unit is the CPU's traffic cop: it reads an instruction's opcode and decides which datapath signals to assert each cycle.

**Type:** Learn
**Languages:** SystemVerilog (HDL)
**Prerequisites:** Phase 06 lessons 01–07
**Time:** ~75 minutes

## Learning Objectives

- Decode RISC-V opcodes into the seven standard control signals.
- Implement both a hardwired and a microcoded control unit in SystemVerilog.
- Explain the speed/flexibility trade-off and identify which approach real CPUs use.

## The Problem

Lesson 07 built the datapath — registers, ALU, memory. But nothing tells the datapath *what to do*. An `add` must enable register write-back; a `lw` must route memory output to the register file; a `beq` must redirect the PC. Without a control unit, the datapath is wires going nowhere.

The control unit's job: take the 7-bit `opcode` and produce a bundle of control signals that steer every multiplexer, enable, and write in the datapath.

## The Concept

### The Control Signal Bundle

For RV32I we need seven signals:

| Signal | Width | Meaning |
|--------|-------|---------|
| `RegWrite` | 1 | Write result to register file |
| `ALUSrc` | 1 | ALU input B = immediate (1) or rs2 (0) |
| `MemRead` | 1 | Read from data memory |
| `MemWrite` | 1 | Write to data memory |
| `Branch` | 1 | Conditional branch instruction |
| `MemToReg` | 1 | Write-back = memory (1) or ALU (0) |
| `ALUOp` | 2 | Encodes ALU operation class |

### Control Signal Truth Table

| Type | Opcode | RegWrite | ALUSrc | MemRead | MemWrite | Branch | MemToReg | ALUOp |
|------|--------|----------|--------|---------|----------|--------|----------|-------|
| R-type | `0110011` | 1 | 0 | 0 | 0 | 0 | 0 | 10 |
| I-type | `0010011` | 1 | 1 | 0 | 0 | 0 | 0 | 11 |
| Load | `0000011` | 1 | 1 | 1 | 0 | 0 | 1 | 00 |
| Store | `0100011` | 0 | 1 | 0 | 1 | 0 | 0 | 00 |
| Branch | `1100011` | 0 | 0 | 0 | 0 | 1 | 0 | 01 |

Each opcode maps to exactly one row. The question is *how* to implement that mapping.

### Approach 1: Hardwired Control

A combinational `case` statement maps opcodes to control vectors. Pure gates — one gate delay from opcode to outputs.

```
opcode → [combinational logic] → control signals
```

**Pros:** Fast, small area, predictable timing.
**Cons:** Adding instructions means re-synthesizing hardware.

### Approach 2: Microcoded Control

Store control words in a ROM. Each opcode is an address; the ROM outputs the control vector. Complex instructions span multiple micro-ops by sequencing through the ROM.

```
opcode → [ROM / sequencer] → control signals
```

**Pros:** Easy to patch, handles complex multi-cycle instructions.
**Cons:** Slower (ROM + sequencer overhead), larger area, power-hungry.

### Real-World Usage

- **RISC-V, ARM, MIPS** — hardwired. Simple ISA, combinational decode suffices.
- **Intel x86, AMD Zen** — hybrid. Hardwired fast-path for common instructions; microcode for complex ops like string moves and `CPUID`. Bug fixes delivered via microcode updates.

## Build It

### Step 1: Hardwired Control Unit

```systemverilog
module control_unit_hardwired (
  input  logic [6:0] opcode,
  output logic       reg_write,
  output logic       alu_src,
  output logic       mem_read,
  output logic       mem_write,
  output logic       branch,
  output logic       mem_to_reg,
  output logic [1:0] alu_op
);
  always_comb begin
    reg_write  = 0;  alu_src    = 0;
    mem_read   = 0;  mem_write  = 0;
    branch     = 0;  mem_to_reg = 0;
    alu_op     = 2'b00;

    case (opcode)
      7'b0110011: begin reg_write = 1; alu_op = 2'b10; end // R-type
      7'b0010011: begin reg_write = 1; alu_src = 1; alu_op = 2'b11; end // I-type
      7'b0000011: begin reg_write = 1; alu_src = 1; mem_read = 1; mem_to_reg = 1; end // Load
      7'b0100011: begin alu_src = 1; mem_write = 1; end // Store
      7'b1100011: begin branch = 1; alu_op = 2'b01; end // Branch
      default: ;
    endcase
  end
endmodule
```

### Step 2: Microcoded Control Unit

Same truth table stored in a ROM.

```systemverilog
module control_unit_microcoded (
  input  logic [6:0] opcode,
  output logic [7:0] control_word
);
  logic [7:0] microcode_rom [0:127];

  initial begin
    for (int i = 0; i < 128; i++)
      microcode_rom[i] = 8'h00;
    microcode_rom[7'b0110011] = 8'b1_0_0_0_0_0_10; // R-type
    microcode_rom[7'b0010011] = 8'b1_1_0_0_0_0_11; // I-type
    microcode_rom[7'b0000011] = 8'b1_1_1_0_0_1_00; // Load
    microcode_rom[7'b0100011] = 8'b0_1_0_1_0_0_00; // Store
    microcode_rom[7'b1100011] = 8'b0_0_0_0_1_0_01; // Branch
  end

  assign control_word = microcode_rom[opcode[6:0]];
endmodule
```

### Step 3: Testbench — Verify Equivalence

Pack the hardwired outputs into the same 8-bit format and compare against the microcoded ROM output for every valid opcode. See `code/control.sv` for the full testbench including JAL/JALR and unknown-opcode checks.

## Use It

**RISC-V:** Berkeley's Rocket and SiFive's U54 use purely hardwired decode — a `case` statement in Chisel. No microcode at all.

**x86:** Intel's frontend cracks complex instructions (e.g., `REP MOVSB`) into micro-ops via a microcode sequencer. Simple instructions like `ADD reg, reg` take a hardwired fast-path, bypassing microcode. AMD Zen 3 adds a **Micro-op Cache** that caches decoded micro-ops so repeated complex instructions skip the sequencer entirely.

## Read the Source

- **Rocket Chip** `Control.scala` — hardwired RISC-V decode, a single `switch` mapping opcodes to control bundles.
- **Linux microcode loader** — `/drivers/platform/x86/intel_microcode.c` — applies Intel/AMD microcode patches at boot.

## Ship It

The reusable artifact is `code/control.sv` — both hardwired and microcoded control units plus a testbench proving they agree. Drop it into any RV32I CPU project.

## Exercises

1. **Easy** — Add control signals for JAL (`1101111`) and JALR (`1100111`). JAL writes PC+4 to rd; JALR does the same but with rs1+imm as the target. Which signals change?

2. **Medium** — Design a microcode sequence for `REP LODSD` (repeat: load dword from `[ESI]` into `EAX`, increment `ESI`, decrement `ECX` until zero). How many micro-ops? What does each assert?

3. **Hard** — Estimate gate count for both approaches. The hardwired version needs combinational logic per opcode; the microcoded version needs a 128x8 ROM plus decode. Which is smaller at 5 instructions? At what count does microcode win?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Control unit | "The brain of the CPU" | Logic that maps opcodes to datapath control signals |
| Hardwired control | "Fixed logic" | Signals generated by gates/case — fast, inflexible |
| Microcode | "Software inside hardware" | ROM of control words; complex ops span multiple micro-ops |
| Micro-op (uop) | "Micro" | One atomic operation executable in a single cycle |
| Control word | "Control vector" | All control signals packed into one bus |
| Fast-path decode | "Short decode" | x86: simple instructions bypass microcode |

## Further Reading

- Patterson & Hennessy, *Computer Organization and Design* (RISC-V ed.), Ch. 4.
- Shen & Lipasti, *Modern Processor Design*, Ch. 2 — microcode in superscalar CPUs.
- Intel SDM Vol. 3A, Section 9.11 — microcode update mechanism.
