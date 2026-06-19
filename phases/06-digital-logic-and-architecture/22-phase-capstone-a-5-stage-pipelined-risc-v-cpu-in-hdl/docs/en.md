# Phase Capstone — A 5-Stage Pipelined RISC-V CPU in HDL

> Everything from this phase converges here: gates become an ALU, the ALU becomes a datapath, the datapath becomes a pipeline. This is where you build a real CPU.

**Type:** Build
**Languages:** SystemVerilog (HDL), RISC-V Assembly
**Prerequisites:** Phase 06 lessons 01–21
**Time:** ~150 minutes

## Learning Objectives

- Implement a complete 5-stage pipelined RISC-V CPU supporting the RV32I instruction set.
- Wire hazard detection and forwarding logic so the pipeline handles data and control hazards correctly.
- Verify the design with assembly test programs that exercise arithmetic, memory, and branch instructions.
- Understand how this simplified model relates to real RISC-V cores (BOOM, Rocket, CV32E40P).

## The Problem

Lessons 07 and 11 gave you the pieces separately: a single-cycle datapath, then pipelining theory with forwarding and hazard diagrams. But theory without a running implementation is just diagrams on a page. You cannot claim to understand computer architecture until you have built a CPU that actually runs programs — instructions flowing through five stages, forwarding rescuing dependent instructions from stalls, branches flushing wrong-path work.

This capstone combines every module from the phase into one working design: fetch, decode, execute, memory, write-back, hazard unit, forwarding unit, and a testbench that boots assembly programs and checks register output.

## The Concept

### The Five Stages

```
IF → [IF/ID] → ID → [ID/EX] → EX → [EX/MEM] → MEM → [MEM/WB] → WB
 └─────────────────────────────────────────────────────────────────┘
                     forwarding paths (EX→EX, MEM→EX)
```

| Stage | Name | What It Does |
|-------|------|-------------|
| IF | Instruction Fetch | Read instruction memory at PC, PC = PC + 4 |
| ID | Instruction Decode | Decode opcode, read rs1/rs2, generate immediate |
| EX | Execute | ALU computation, branch target/condition resolution |
| MEM | Memory Access | Load: read data memory. Store: write data memory. |
| WB | Write Back | Write ALU result or loaded data to register file |

### Pipeline Registers

Each pair of stages is separated by a register that latches values on the rising clock edge:

- **IF/ID**: `instr[31:0]`, `pc_plus4[31:0]`
- **ID/EX**: `pc_plus4`, `rs1_data`, `rs2_data`, `imm`, `rs1_addr`, `rs2_addr`, `rd_addr`, `ctrl` (RegWrite, ALUSrc, MemWrite, MemRead, Branch, ALUOp, MemToReg)
- **EX/MEM**: `alu_result`, `rs2_data`, `rd_addr`, `ctrl`
- **MEM/WB**: `mem_data`, `alu_result`, `rd_addr`, `ctrl`

### Hazard Handling

**Data forwarding** resolves most RAW hazards without stalling:

```
EX forward:  EX/MEM.alu_result → EX stage ALU input   (previous instruction)
MEM forward: MEM/WB result      → EX stage ALU input   (two-back instruction)
```

When forwarding cannot help — a load immediately followed by an instruction that uses the loaded value (load-use hazard) — the **hazard unit** stalls the pipeline for one cycle by freezing PC and IF/ID and inserting a bubble into ID/EX.

**Branch flush**: when a branch is taken, the two instructions in IF and ID are wrong-path and must be squashed (their pipeline registers are cleared to NOP).

### Supported Instructions (RV32I)

| Category | Instructions |
|----------|-------------|
| R-type arithmetic | ADD, SUB, AND, OR, XOR, SLL, SRL, SRA, SLT, SLTU |
| I-type arithmetic | ADDI, ANDI, ORI, XORI, SLTI, SLTIU, SLLI, SRLI, SRAI |
| Load | LW |
| Store | SW |
| Branch | BEQ, BNE, BLT, BGE |
| Jump | JAL, JALR |
| Upper-immediate | LUI, AUIPC |

## Build It

The complete design lives in `code/pipelined_cpu.sv`. Here is the architecture:

### Top-Level Wiring

```systemverilog
module pipelined_cpu (
    input  logic clk,
    input  logic rst_n
);
    // Pipeline stage instances
    fetch_stage    u_if  (.*);
    decode_stage   u_id  (.*);
    execute_stage  u_ex  (.*);
    memory_stage   u_mem (.*);
    writeback_stage u_wb (.*);

    // Pipeline registers
    if_id_reg  u_if_id  (.*);
    id_ex_reg  u_id_ex  (.*);
    ex_mem_reg u_ex_mem (.*);
    mem_wb_reg u_mem_wb (.*);

    // Hazard and forwarding
    hazard_unit     u_hz (.*);
    forwarding_unit u_fwd(.*);
endmodule
```

### Key Design Decisions

1. **Branch resolved in EX** — the ALU computes the branch condition. On taken branch, flush IF/ID and ID/EX (2-cycle penalty). This matches a classic 5-stage pipeline; real CPUs resolve earlier with dedicated comparators.

2. **Forwarding priority** — EX hazard (1-cycle distance) takes priority over MEM hazard (2-cycle distance), because the EX/MEM result is more recent.

3. **x0 hardwired to zero** — the register file never writes to x0, and the forwarding unit ignores writes to x0.

### Step 1: Pipeline Registers

Each pipeline register is a simple always_ff block that latches on `clk` posedge and clears on flush or reset:

```systemverilog
always_ff @(posedge clk or negedge rst_n) begin
    if (!rst_n || flush)
        {instr, pc_plus4} <= '0;
    else if (!stall)
        {instr, pc_plus4} <= {instr_in, pc_plus4_in};
end
```

### Step 2: Control Unit

Decodes `instr[6:0]` opcode and `funct3`/`funct7` to produce: `RegWrite`, `ALUSrc`, `MemWrite`, `MemRead`, `Branch`, `MemToReg`, `ALUOp`, and `Jump`.

### Step 3: ALU

4-bit `ALUOp` selects operation. Implements ADD, SUB, AND, OR, XOR, shifts, and comparisons.

### Step 4: Hazard and Forwarding Units

```systemverilog
// Load-use: LW in EX, dependent instruction in ID
assign stall = id_ex_ctrl.MemRead &&
               ((id_ex_rd == if_id_rs1) || (id_ex_rd == if_id_rs2));

// Forward from EX/MEM
if (ex_mem_ctrl.RegWrite && ex_mem_rd != 0 && ex_mem_rd == id_ex_rs1)
    forward_A = 2'b10;
// Forward from MEM/WB
else if (mem_wb_ctrl.RegWrite && mem_wb_rd != 0 && mem_wb_rd == id_ex_rs1)
    forward_A = 2'b01;
```

### Step 5: Testbench

The testbench instantiates the CPU, loads an instruction memory from a hex file, resets, runs for a configurable number of cycles, then dumps all 32 registers. Programs in `code/programs.s` test different instruction mixes.

## Use It

This design is structurally identical to educational RISC-V cores:

- **[BOOM (Berkeley Out-of-Order Machine)](https://github.com/riscv-boom/riscv-boom)** — same 5-stage backbone, then adds out-of-order execution.
- **[CV32E40P (formerly RI5CY)](https://github.com/openhwgroup/cv32e40p)** — production RISC-V core with a similar pipeline, plus hardware loops and post-increment addressing.
- **[Rocket Chip](https://github.com/chipsalliance/rocket-chip)** — in-order 5-stage variant used in SiFive cores.

Our simplified version omits: branch prediction (we always predict not-taken), caches (direct memory), privilege modes, and CSRs. Each of these is a natural extension.

## Read the Source

- `code/pipelined_cpu.sv` — every module in one file for clarity. Real designs split across files with `include` or a build system.
- `code/programs.s` — four test programs covering arithmetic, recursion, sorting, and string manipulation.

## Ship It

The reusable artifact lives in `outputs/`:

- **`pipelined_cpu.sv`** — synthesizable 5-stage pipeline with hazard handling.
- **`tb_pipelined_cpu.sv`** — testbench that boots and runs test programs.

## Exercises

1. **Easy — Add M Extension**: extend the ALU and control unit to support `MUL`, `DIV`, `DIVU`, `REM`, `REMU`. Add a multi-cycle multiply/divide unit that stalls the pipeline for 2-4 cycles.

2. **Medium — Interrupt Support**: add a `irq` input. When asserted mid-pipeline, save PC to a `mepc` register, set `mtvec` as the new PC, and flush the pipeline. Implement `MRET` to restore.

3. **Hard — L1 Cache**: replace direct instruction/data memory with a 2-way set-associative cache (1 KB, 16-byte lines). On cache miss, stall the pipeline and simulate a 10-cycle memory latency.

## Key Terms

| Term | What People Say | What It Actually Means |
|------|----------------|------------------------|
| Pipeline register | "latch between stages" | D-flop array that holds stage outputs for one cycle |
| Forwarding / bypassing | "data comes from EX/MEM back to EX" | MUX selecting a newer result from pipeline registers instead of the register file |
| Stall / bubble | "pipeline freezes for a cycle" | PC and IF/ID frozen; ID/EX gets a NOP (all control signals zeroed) |
| Flush | "squash wrong-path instructions" | Clear pipeline register contents to zero (NOP) after a taken branch or exception |
| Load-use hazard | "LW right before ADD that uses the load" | Only case where forwarding cannot resolve the RAW dependency — must stall 1 cycle |
| CPI | "cycles per instruction" | 1 + stall cycles + branch penalty cycles per instruction |

## Further Reading

- Patterson & Hennessy, *Computer Organization and Design: RISC-V Edition*, Chapter 4 — the definitive pipeline treatment.
- [RISC-V ISA Specification](https://riscv.org/technical/specifications/) — Volume I (unprivileged ISA).
- [PULP Platform cores](https://github.com/pulp-platform) — open-source RISC-V implementations in SystemVerilog.
- [RISC-V Formal Verification Framework](https://github.com/SymbioticEDA/riscv-formal) — formal checks for RISC-V cores.
