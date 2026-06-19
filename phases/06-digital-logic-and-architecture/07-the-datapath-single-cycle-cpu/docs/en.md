# The Datapath — Single-Cycle CPU

> Every instruction completes in one clock cycle. Simple to understand, slow to execute — the foundation before pipelining.

**Type:** Build
**Languages:** SystemVerilog (HDL)
**Prerequisites:** Phase 06 lessons 01–06
**Time:** ~90 minutes

## Learning Objectives

- Understand the single-cycle datapath and why CPI = 1 comes at the cost of clock period.
- Implement a RISC-V single-cycle CPU from ALU, register file, and memory modules.
- Identify every control signal and trace instruction flow through the datapath.
- Ship the reusable single-cycle datapath module for the capstone.

## The Problem

You have built gates, adders, muxes, flip-flops, registers, an ALU, a register file, and memory. They are isolated modules sitting in separate files. A CPU is what happens when you wire them together so that bits flow from instruction memory, through decoders and ALUs, into registers and data memory — every clock cycle.

Without a datapath, the capstone (a 5-stage pipelined RISC-V CPU) has nothing to pipeline. This lesson builds the unpipelined version first.

## The Concept

### What is a Datapath?

A **datapath** is the collection of functional units (ALU, registers, memories, muxes) and the buses that connect them. The **control unit** (covered in Lesson 08) drives the mux selects, enable lines, and ALU operation codes that steer data through the datapath.

In a **single-cycle** design, every instruction — from fetch through writeback — completes within one clock cycle. The clock period must be long enough for the slowest instruction (typically `lw`, which must: fetch from instruction memory, read register file, compute address in ALU, read data memory, and write back to register file).

```
CPI = 1  (always)
Clock period = delay of slowest instruction
Throughput = 1 / clock period  (worse than pipelining)
```

### The Big Picture

```
                 ┌─────────────┐
          PC+4 ──┤  PC Register├── pc
                 └──────┬──────┘
                        │
                 ┌──────▼──────┐
                 │ Instruction │
                 │   Memory    │  (ROM)
                 └──────┬──────┘
                        │ instruction[31:0]
           ┌────────────┼────────────────┐
           │            │                │
     ┌─────▼─────┐ ┌───▼────┐  ┌────────▼────────┐
     │ Imm Gen / │ │ Control│  │  Register File   │
     │ Sign-Ext  │ │  Unit  │  │ rs1, rs2 → rd   │
     └─────┬─────┘ └───┬────┘  └──┬─────────┬────┘
           │            │          │         │
           │      ctrl signals     │         │
           │            │    ┌─────┘         │
           │      ┌─────▼────▼───┐           │
           │      │    ALUSrc MUX│           │
           └─────►│  (imm / rs2) │           │
                  └──────┬───────┘           │
                  ┌──────▼───────┐           │
                  │     ALU      │           │
                  └──┬───────┬───┘           │
                     │       │               │
              ALU result   zero             │
                     │       │               │
              ┌──────▼───────▼──┐            │
              │  Data Memory    │            │
              │  (RAM)          │            │
              └──────┬──────────┘            │
              ┌──────▼───────┐               │
              │ MemToReg MUX │               │
              └──────┬───────┘               │
                     │                       │
                     └───────► write data ───┘
```

### Instruction Flow (all in one cycle)

Every instruction goes through five conceptual stages, but all five happen in a single clock cycle:

| Stage | Name        | What happens                                              |
|-------|-------------|-----------------------------------------------------------|
| IF    | Fetch       | Read `instruction = IMEM[PC]`, compute `PC + 4`           |
| ID    | Decode      | Extract fields (opcode, rs1, rs2, rd, funct3, funct7, imm) |
| EX    | Execute     | ALU computes result (address, arithmetic, comparison)     |
| MEM   | Memory      | Read/write data memory (only for lw/sw)                   |
| WB    | Writeback   | Write result to register file (rd)                        |

### R-Type (add, sub, and, or, slt)

```
add x1, x2, x3    →  x1 = x2 + x3
```

Path: IMEM[PC] → decode rd, rs1, rs2 → read rs1, rs2 from register file → ALU computes → write result to rd.

Control signals: `ALUSrc = 0` (use rs2), `MemToReg = 0` (use ALU), `RegWrite = 1`, `MemRead = 0`, `MemWrite = 0`, `Branch = 0`.

### I-Type (lw, addi)

```
lw  x1, 0(x2)    →  x1 = MEM[x2 + 0]
addi x1, x2, 5   →  x1 = x2 + 5
```

Path: IMEM[PC] → decode rd, rs1, imm → read rs1 → sign-extend imm → ALU computes `rs1 + imm` → for lw, read data memory → write to rd.

Control signals: `ALUSrc = 1` (use immediate), `MemToReg = 1` for lw / `0` for addi, `RegWrite = 1`, `MemRead = 1` for lw / `0` for addi.

### S-Type (sw)

```
sw  x1, 0(x2)    →  MEM[x2 + 0] = x1
```

Path: IMEM[PC] → decode rs1, rs2, imm → read rs1, rs2 → ALU computes `rs1 + imm` (address) → write rs2 to data memory at that address.

Control signals: `ALUSrc = 1`, `MemWrite = 1`, `RegWrite = 0`, `Branch = 0`.

### B-Type (beq)

```
beq x1, x2, offset  →  if (x1 == x2) PC = PC + offset else PC = PC + 4
```

Path: IMEM[PC] → decode rs1, rs2, imm → read rs1, rs2 → ALU subtracts → if `zero` flag set, PC = PC + imm (branch target), else PC = PC + 4.

Control signals: `ALUSrc = 0` (sub compares), `Branch = 1`, `RegWrite = 0`. A branch-taken MUX selects between `PC + 4` and `PC + imm` for the next PC.

### Control Signals Summary

| Signal    | R-type | I-type (lw) | I-type (addi) | S-type (sw) | B-type (beq) |
|-----------|--------|-------------|---------------|-------------|--------------|
| ALUSrc    | 0      | 1           | 1             | 1           | 0            |
| MemToReg  | 0      | 1           | 0             | X           | X            |
| RegWrite  | 1      | 1           | 1             | 0           | 0            |
| MemRead   | 0      | 1           | 0             | 0           | 0            |
| MemWrite  | 0      | 0           | 0             | 1           | 0            |
| Branch    | 0      | 0           | 0             | 0           | 1            |
| ALUOp     | 10     | 00          | 00            | 00          | 01           |

X = don't care.

### Build It — Connecting the Pieces

The datapath wiring for each instruction type:

1. **PC register** — clocked register holding the current program counter. Input = next PC (either `PC + 4` or branch target). Output drives instruction memory address.

2. **Instruction memory** — ROM indexed by PC. Output = 32-bit instruction word.

3. **Immediate generator** — extracts and sign-extends the immediate from the instruction based on the format (I, S, B).

4. **Register file** — two read ports (rs1, rs2), one write port (rd). Read is combinational; write is clocked.

5. **ALUSrc MUX** — selects between register file read data 2 and the sign-extended immediate as the ALU's second operand.

6. **ALU** — computes the operation determined by ALU control (derived from ALUOp + funct3/funct7).

7. **Data memory** — RAM. Address = ALU result. Write data = register file read data 2. Read data feeds back to MemToReg MUX.

8. **MemToReg MUX** — selects between ALU result and data memory read data as the value written to the register file.

9. **Branch MUX** — selects between `PC + 4` and `PC + imm` as the next PC value, gated by `Branch & zero`.

### Critical Path

The longest combinational path determines the minimum clock period:

```
PC → IMEM read → register file read → ALU → data memory read → MUX → register file write setup
```

This is the `lw` path. A single-cycle CPU must allow this path to complete within one clock period, even though R-type instructions finish much faster.

## Build It

The complete single-cycle datapath in SystemVerilog connects modules from lessons 05 (ALU) and 06 (register file, memory). See `code/single_cycle.sv` for the full implementation with testbench.

### Step 1: Datapath Skeleton

```systemverilog
module single_cycle_cpu #(
    parameter IMEM_WORDS = 64,
    parameter DMEM_WORDS = 256
)(
    input  logic clk,
    input  logic rst
);
    // PC
    logic [31:0] pc, pc_next, pc_plus4;
    always_ff @(posedge clk) begin
        if (rst) pc <= 32'h0;
        else     pc <= pc_next;
    end
    assign pc_plus4 = pc + 4;

    // Instruction memory (ROM)
    logic [31:0] instr;
    imem #(.WORDS(IMEM_WORDS)) rom (.addr(pc), .rdata(instr));

    // Decode
    logic [6:0]  opcode = instr[6:0];
    logic [4:0]  rd     = instr[11:7];
    logic [2:0]  funct3 = instr[14:12];
    logic [4:0]  rs1    = instr[19:15];
    logic [4:0]  rs2    = instr[24:20];
    logic [6:0]  funct7 = instr[31:25];

    // Immediate generator
    logic [31:0] imm;
    imm_gen gen (.instr(instr), .imm(imm));

    // Control
    logic ALUSrc, MemToReg, RegWrite, MemRead, MemWrite, Branch;
    logic [1:0] ALUOp;
    control_unit ctrl (.opcode(opcode),
                       .ALUSrc, .MemToReg, .RegWrite,
                       .MemRead, .MemWrite, .Branch, .ALUOp);

    // Register file
    logic [31:0] rs1_data, rs2_data, write_data;
    reg_file rf (.clk(clk), .rst(rst),
                 .rs1(rs1), .rs2(rs2), .rd(rd),
                 .rd_data(write_data), .wr_en(RegWrite),
                 .rs1_data(rs1_data), .rs2_data(rs2_data));

    // ALU
    logic [3:0]  alu_ctrl;
    alu_control ac (.ALUOp(ALUOp), .funct3(funct3), .funct7(funct7), .alu_ctrl(alu_ctrl));
    logic [31:0] alu_b, alu_result;
    logic        alu_zero;
    assign alu_b = ALUSrc ? imm : rs2_data;
    alu_main alu (.a(rs1_data), .b(alu_b), .alu_ctrl(alu_ctrl),
                  .result(alu_result), .zero(alu_zero));

    // Data memory
    logic [31:0] dmem_rdata;
    dmem #(.WORDS(DMEM_WORDS)) ram (.clk(clk), .addr(alu_result),
                                     .wdata(rs2_data), .wr_en(MemWrite),
                                     .rd_en(MemRead), .rdata(dmem_rdata));

    // Write-back mux
    assign write_data = MemToReg ? dmem_rdata : alu_result;

    // Next-PC mux
    logic branch_taken;
    assign branch_taken = Branch & alu_zero;
    assign pc_next = branch_taken ? (pc + imm) : pc_plus4;
endmodule
```

### Step 2: Control Units

```systemverilog
module control_unit (
    input  logic [6:0] opcode,
    output logic       ALUSrc, MemToReg, RegWrite,
                       MemRead, MemWrite, Branch,
    output logic [1:0] ALUOp
);
    always_comb begin
        // Defaults
        {ALUSrc, MemToReg, RegWrite, MemRead, MemWrite, Branch, ALUOp} = '0;
        case (opcode)
            7'b0110011: begin // R-type
                ALUSrc=0; RegWrite=1; ALUOp=2'b10;
            end
            7'b0000011: begin // lw
                ALUSrc=1; MemToReg=1; RegWrite=1; MemRead=1; ALUOp=2'b00;
            end
            7'b0010011: begin // addi
                ALUSrc=1; RegWrite=1; ALUOp=2'b00;
            end
            7'b0100011: begin // sw
                ALUSrc=1; MemWrite=1; ALUOp=2'b00;
            end
            7'b1100011: begin // beq
                Branch=1; ALUOp=2'b01;
            end
            default: ;
        endcase
    end
endmodule

module alu_control (
    input  logic [1:0] ALUOp,
    input  logic [2:0] funct3,
    input  logic [6:0] funct7,
    output logic [3:0] alu_ctrl
);
    always_comb begin
        case (ALUOp)
            2'b00: alu_ctrl = 4'b0010; // add (lw/sw/addi)
            2'b01: alu_ctrl = 4'b0110; // sub (beq)
            2'b10: begin // R-type
                case ({funct7, funct3})
                    10'b0000000_000: alu_ctrl = 4'b0010; // add
                    10'b0100000_000: alu_ctrl = 4'b0110; // sub
                    10'b0000000_111: alu_ctrl = 4'b0000; // and
                    10'b0000000_110: alu_ctrl = 4'b0001; // or
                    10'b0000000_010: alu_ctrl = 4'b0111; // slt
                    default:         alu_ctrl = 4'bxxxx;
                endcase
            end
            default: alu_ctrl = 4'bxxxx;
        endcase
    end
endmodule
```

## Use It

Real processors (even simple microcontrollers) do not use single-cycle datapaths in production. The reason is the clock period: because `lw` sets the pace, every `add` wastes most of the cycle waiting for `lw`'s timing budget to expire.

However, single-cycle datapaths appear in:

- **Educational CPUs**: RISC-V reference designs in Patterson & Hennessy's *Computer Organization and Design*.
- **Tiny embedded cores**: Some ultra-low-power 8-bit cores (e.g., AVR) use near-single-cycle execution for most instructions, with multi-cycle exceptions for multiply/divide.
- **FPGA soft cores**: Quick prototypes where simplicity matters more than throughput.

The production move is **pipelining** (Lesson 11), which overlaps the five stages across instructions, achieving CPI ≈ 1 with a much shorter clock period.

## Read the Source

- Patterson & Hennessy, *Computer Organization and Design RISC-V Edition*, Chapter 4 — the canonical single-cycle datapath diagram and control truth table.
- Berkeley's `rv32ui` test suite — bare-metal RISC-V programs that test every instruction used in this datapath.

## Ship It

The reusable artifact produced by this lesson lives in `code/single_cycle.sv`. It is:

- **A complete single-cycle RISC-V CPU** connecting ALU, register file, and memory with control logic.
- Reuse this as the base for Lesson 11 (pipelining) by splitting the combinational path into pipeline registers.

## Exercises

1. **Easy** — Add **JAL** (jump and link) support: `PC = PC + imm`, `rd = PC + 4`. Wire a new MUX for next-PC and add a `Jump` control signal.
2. **Medium** — Add **LUI** (load upper immediate): `rd = imm << 12`. This needs a new datapath path that writes the shifted immediate directly to the register file, bypassing the ALU.
3. **Hard** — Measure the **critical path delay** of your datapath using your simulator's timing analysis (e.g., `Verilator --timing` or Quartus TimeQuest). Which instruction is the bottleneck? Quantify how much faster the clock could be if only R-type instructions existed.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Single-cycle | "One instruction per cycle" | Every instruction completes in one clock cycle; CPI = 1, but clock period = slowest instruction |
| CPI | "Cycles per instruction" | Average number of clock cycles each instruction takes to execute |
| Critical path | "The slowest path" | The longest combinational delay from one register output to the next register input |
| Datapath | "The data highway" | Hardware units and buses that process and move data during instruction execution |
| Control signals | "The steering bits" | Signals generated by the control unit that select MUX inputs, enable writes, and choose ALU operations |

## Further Reading

- Patterson & Hennessy, *Computer Organization and Design RISC-V Edition*, Chapter 4.1–4.4
- Harris & Harris, *Digital Design and Computer Architecture RISC-V Edition*, Chapter 7
- David Harris's single-cycle CPU reference on GitHub: `riscv-single-cycle`
