# Registers, Register Files, Memory Banks

> The CPU's working memory. Every instruction reads and writes through these structures.

**Type:** Learn
**Languages:** SystemVerilog (HDL)
**Prerequisites:** Phase 06 lessons 01–05
**Time:** ~60 minutes

## Learning Objectives

- Build an N-bit register from D flip-flops with shared clock and enable.
- Implement a RISC-V-style register file with dual-port read and single-port write.
- Understand x0 hardwiring, synchronous vs. asynchronous read, and SRAM vs. DRAM trade-offs.
- Connect the register file to an ALU as part of a CPU datapath.

## The Problem

This lesson sits in **Phase 06 — Digital Logic & Computer Architecture**. Without the concept it teaches, you cannot
build the phase's capstone (A 5-stage pipelined RISC-V CPU in HDL with assembler.). Concretely, *not* knowing this means you get stuck the
moment you try to build the datapath, pipeline registers, or connect the ALU to working storage.

Lesson 04 gave you the D flip-flop — a 1-bit memory element. Lesson 05 gave you the ALU — combinational logic that computes. But an ALU with nowhere to store results is useless. You need **registers**: fast, clocked storage that holds operands between cycles. Wire 32 of them together with read/write ports and you have a **register file** — the heart of every processor.

## The Concept

### From Flip-Flop to Register

A single D flip-flop stores one bit. To store an N-bit value, wire N D flip-flops together with a **shared clock** and a **shared enable** signal:

```
         clk
          |
    +-----+-----+-----+--- ...
    |     |     |     |
  [DFF] [DFF] [DFF] [DFF]   ← N copies
    |     |     |     |
   d[0]  d[1]  d[2]  d[3]   ← data inputs
    |     |     |     |
   q[0]  q[1]  q[2]  q[3]   ← data outputs
```

- **clk**: all flip-flops sample on the same rising edge.
- **en** (write enable): when 1, the register loads `d` on the next clock edge. When 0, the register holds its current value.
- **rst** (reset): synchronous reset sets all bits to 0.

This is a **synchronous register** — the standard building block in modern digital design.

### The Register File

A register file is an **array of registers** with addressing logic for reads and writes. The RISC-V ISA defines 32 general-purpose registers (`x0`–`x31`), each 32 bits wide.

A typical RISC-V register file has:

| Port | Direction | Width | Purpose |
|------|-----------|-------|---------|
| `rs1_addr` | input | 5 bits | Address of first source register (0–31) |
| `rs1_data` | output | 32 bits | Value of register at `rs1_addr` |
| `rs2_addr` | input | 5 bits | Address of second source register |
| `rs2_data` | output | 32 bits | Value of register at `rs2_addr` |
| `rd_addr` | input | 5 bits | Address of destination register |
| `rd_data` | input | 32 bits | Data to write into `rd_addr` |
| `we` | input | 1 bit | Write enable |

This is a **dual-port read, single-port write** design. Two reads happen in parallel (combinational), one write happens on the clock edge (synchronous). This matches RISC-V instruction format: most instructions read two source registers (`rs1`, `rs2`) and write one destination (`rd`).

### x0 Hardwired to Zero

RISC-V register `x0` is special: it always reads as zero, and writes to it are silently discarded. This eliminates the need for a `NOP` opcode — any instruction that writes to `x0` is effectively a no-op. The implementation is simple:

```systemverilog
assign rs1_data = (rs1_addr == 5'b0) ? 32'b0 : regs[rs1_addr];
```

### Synchronous vs. Asynchronous Read

| Style | Read behavior | Use case |
|-------|--------------|----------|
| **Synchronous read** | Address is sampled on clock edge; data appears next cycle | FPGA block RAM (BRAM) |
| **Asynchronous read** | Data appears immediately when address changes | ASIC register files, small SRAM |

RISC-V implementations typically use **asynchronous read** for the register file (data available within the same cycle the address is presented) and **synchronous write** (data stored on clock edge). This is the style we implement below.

### SRAM vs. DRAM

| | SRAM | DRAM |
|---|---|---|
| Storage element | 6 transistors per bit | 1 transistor + 1 capacitor per bit |
| Speed | Very fast (< 1 ns) | Slower (~10–50 ns) |
| Density | Low (large cell) | High (small cell) |
| Cost | Expensive | Cheap |
| Use in CPU | Register file, caches (L1/L2/L3) | Main memory (RAM sticks) |
| Refresh needed? | No | Yes (capacitor leaks) |

The register file is SRAM: small, fast, no refresh. Main memory is DRAM: large, cheap, needs periodic refresh. The entire memory hierarchy (registers → L1 → L2 → L3 → DRAM) exists to bridge the speed/cost gap between these two technologies.

## Build It

### Step 1: N-bit Register

A single register — N D flip-flops with shared clock, enable, and reset.

```systemverilog
module register #(
  parameter WIDTH = 32
) (
  input  logic              clk,
  input  logic              rst,
  input  logic              en,
  input  logic [WIDTH-1:0]  d,
  output logic [WIDTH-1:0]  q
);
  always_ff @(posedge clk) begin
    if (rst)
      q <= '0;
    else if (en)
      q <= d;
  end
endmodule
```

Key points:
- `always_ff` tells the synthesizer this is a flip-flop block (not combinational).
- `<=` is **non-blocking assignment** — all RHS values are sampled simultaneously, preventing simulation glitches.
- `rst` is synchronous (checked on the clock edge). For asynchronous reset, add `or posedge rst` to the sensitivity list.

### Step 2: Register File (32 × 32-bit)

Now wire 32 registers together with address decode logic.

```systemverilog
module register_file (
  input  logic        clk,
  input  logic        rst,
  input  logic        we,
  input  logic [4:0]  rs1_addr,
  input  logic [4:0]  rs2_addr,
  input  logic [4:0]  rd_addr,
  input  logic [31:0] rd_data,
  output logic [31:0] rs1_data,
  output logic [31:0] rs2_data
);

  logic [31:0] regs [1:31];  // x0 is not stored — it is hardwired

  // Asynchronous read with x0 hardwiring
  assign rs1_data = (rs1_addr == 5'd0) ? 32'b0 : regs[rs1_addr];
  assign rs2_data = (rs2_addr == 5'd0) ? 32'b0 : regs[rs2_addr];

  // Synchronous write (skip x0)
  always_ff @(posedge clk) begin
    if (rst) begin
      for (int i = 1; i < 32; i++)
        regs[i] <= 32'b0;
    end else if (we && rd_addr != 5'd0) begin
      regs[rd_addr] <= rd_data;
    end
  end

endmodule
```

Notice we only instantiate `regs[1:31]` — 31 registers, not 32. Register `x0` exists only as the constant `0` in the read logic. This saves one register's worth of flip-flops and makes the "writes to x0 are discarded" guarantee structural.

## Use It

In a single-cycle RISC-V CPU datapath, the register file sits between the instruction decoder and the ALU:

```
Instruction ──→ Decode ──→ rs1_addr, rs2_addr ──→ Register File
                                                    │
                                          rs1_data ─┘
                                                    ├──→ ALU ──→ rd_data
                                          rs2_data ─┘         ──→ Register File (rd_addr, we)
```

1. **Fetch**: PC provides the instruction address.
2. **Decode**: extract `rs1`, `rs2`, `rd` fields from the instruction.
3. **Read**: register file outputs `rs1_data` and `rs2_data` (asynchronous).
4. **Execute**: ALU computes `rs1_data OP rs2_data`.
5. **Write-back**: if `we` is asserted, `rd_data` is stored into `rd_addr` on the next clock edge.

In a pipelined CPU, each pipeline stage has its own set of pipeline registers that buffer values between stages — these are also built from the `register` module above.

## Read the Source

- [Berkeley CS152 register file](https://github.com/sequencer/chipyard) — look at `generators/rocket-chip/src/main/scala/rocket/RocketCore.scala` for the real RISC-V register file in the Rocket core.

## Ship It

The reusable artifact produced by this lesson lives in `code/registers.sv`. It contains:

- `register` — parameterized N-bit register with clock, enable, and synchronous reset.
- `register_file` — 32×32-bit RISC-V register file with dual-port async read, single-port sync write, and x0 hardwired to zero.

## Exercises

1. **Easy** — Implement a register file with **4 read ports** and 1 write port. Hint: add `rs3_addr`/`rs3_data` and `rs4_addr`/`rs4_data` ports and duplicate the async read logic.

2. **Medium** — Implement a register file with **asynchronous read and synchronous write** where the write takes effect in the same cycle as the read (write-through / write-first semantics). If an instruction writes to `rs1` in the same cycle it reads `rs1`, the read should return the new value.

3. **Hard** — Add **register forwarding (bypass)** logic to avoid pipeline stalls. When the write-back stage is writing to a register that the decode stage is reading, bypass the value directly from the write-back stage to the decode stage output instead of waiting for the write to commit. This requires additional `bypass_*` inputs and combinational muxing.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Register | "A fast storage cell" | N D flip-flops with shared clock, holding one N-bit value |
| Register file | "The register set" | Array of registers with address-decoded read/write ports |
| x0 | "The zero register" | RISC-V register that always reads 0; writes discarded |
| Dual-port read | "Two reads per cycle" | Two independent address/data paths that operate in parallel |
| Synchronous write | "Clock-edge write" | Data is stored into the register only on the rising clock edge |
| SRAM | "Static RAM" | Fast, expensive memory using 6T cells; used for caches and register files |
| DRAM | "Dynamic RAM" | Cheap, dense memory using 1T+1C cells; needs periodic refresh |
| Bypass / Forwarding | "Forward the result" | Routing a not-yet-written result directly to a dependent instruction to avoid stalls |

## Further Reading

- Patterson & Hennessy, *Computer Organization and Design* (RISC-V edition), Ch. 4 — The Processor.
- Harris & Harris, *Digital Design and Computer Architecture* (RISC-V edition), Ch. 7 — Microarchitecture.
- RISC-V ISA Specification, §2.1 — Base Integer ISA (defines x0 behavior).
