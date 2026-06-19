# Lesson 11: Pipelining — 5-Stage, Hazards, Forwarding

## Why Pipeline?

A single-cycle CPU finishes one instruction before starting the next. Pipelining overlaps execution of multiple instructions to increase **throughput** (instructions per second) at the cost of slightly increased **latency** (time per instruction due to pipeline register overhead).

A laundry analogy: washing, drying, folding, storing are stages. You don't wait for one load to finish all four steps before starting the next load. Once the pipeline is full, one instruction completes every clock cycle.

## The 5-Stage Pipeline

Classic RISC pipeline stages:

| Stage | Name | Purpose |
|-------|------|---------|
| IF | Instruction Fetch | Read instruction from memory at PC, increment PC |
| ID | Instruction Decode | Decode opcode, read register file, sign-extend immediate |
| EX | Execute | ALU operation or compute branch target |
| MEM | Memory Access | Read/write data memory (load/store) |
| WB | Write Back | Write result to register file |

Each stage is separated by **pipeline registers** that hold intermediate results:

```
PC → [IF] → IF/ID → [ID] → ID/EX → [EX] → EX/MEM → [MEM] → MEM/WB → [WB] → Register File
```

### Pipeline Register Fields

**IF/ID**: `instr`, `pc_plus4`
**ID/EX**: `pc_plus4`, `rs1_data`, `rs2_data`, `imm`, `rs1`, `rs2`, `rd`, `ctrl_signals`
**EX/MEM**: `alu_result`, `rs2_data` (for store), `rd`, `ctrl_signals`
**MEM/WB**: `mem_data`, `alu_result`, `rd`, `ctrl_signals`

## Hazards

Hazards prevent the next instruction from executing in its designated cycle.

### Data Hazards (RAW — Read After Write)

When an instruction needs a value that a previous instruction hasn't yet written back:

```
ADD  x1, x2, x3    # writes x1 in WB (cycle 5)
SUB  x4, x1, x5    # reads x1 in ID (cycle 3) — x1 not yet written!
```

Three types based on pipeline distance:
- **EX hazard**: result from EX stage needed (1 instruction apart)
- **MEM hazard**: result from MEM stage needed (2 instructions apart)

### Control Hazards (Branch)

Branch outcome is resolved in EX or MEM. Instructions fetched after the branch but before resolution are **wrong-path** — they must be flushed.

### Structural Hazards

Two instructions need the same hardware resource simultaneously (e.g., two writes to register file in same cycle). Avoided by design: write in first half, read in second half.

## Forwarding (Bypassing)

Instead of stalling, route results directly from pipeline registers to ALU inputs:

```
Forward from EX/MEM (previous instruction's ALU result)  → EX stage
Forward from MEM/WB  (two-back instruction's result)     → EX stage
```

### Forwarding Conditions

```verilog
// EX hazard: previous instruction writes rd, rd matches rs1 or rs2
if (EX/MEM.reg_write && EX/MEM.rd != 0 && EX/MEM.rd == ID/EX.rs1)
    forward_A = 2'b10;  // from EX/MEM

// MEM hazard: two-back instruction writes rd, rd matches rs1 or rs2
if (MEM/WB.reg_write && MEM/WB.rd != 0 && MEM/WB.rd == ID/EX.rs1)
    forward_A = 2'b01;  // from MEM/WB
```

### Load-Use Hazard: When Forwarding Cannot Save You

```
LW   x1, 0(x2)     # x1 available after MEM stage (cycle 4)
ADD  x4, x1, x5    # needs x1 at start of EX (cycle 3) — 1 cycle too early!
```

No forwarding path can retrieve a load result before it exits MEM. Solution: **stall** — insert a bubble (NOP) into the pipeline for one cycle.

## Stall (Bubble) Insertion

The hazard detection unit detects load-use and:
1. Freezes PC (stop incrementing)
2. Freezes IF/ID register
3. Inserts NOP into ID/EX (all control signals zeroed)

```
Cycle 3: IF   ID   EX  MEM  WB
               LW   —   —    —    —    ← stall inserts bubble
Cycle 4: IF   ID   EX  MEM  WB
               LW   ADD  —   —    —    ← ADD can now forward from LW's MEM
```

## Branch Penalty

On a taken branch, the next 2-3 fetched instructions are wrong-path. They must be **flushed** (discarded). The penalty is the number of wasted cycles.

**CPI (Cycles Per Instruction)** with hazards:

```
CPI = 1 + stall_penalty + branch_penalty
```

Ideal CPI = 1 (one instruction per cycle). Hazards push it higher.

## Building It

We implement a 5-stage pipeline in SystemVerilog with:
- Pipeline registers between each stage
- Forwarding unit that routes EX/MEM and MEM/WB results back
- Hazard detection unit for load-use stalls
- Branch flush on mispredict

See `code/pipeline.sv`.

## Using It

Every modern CPU pipelines. Intel Pentium (1993) used a 5-stage pipeline. Modern cores have 14-20+ stages with deeper speculation. GPUs and DSPs also pipeline extensively.

## Shipping It

A correct pipelined CPU is the foundation of every high-performance processor. Understanding forwarding and hazards is essential for CPU design, performance modeling, and compiler optimization (instruction scheduling).

## Exercises

**Level 1 — Recall:**
Trace this sequence through a 5-stage pipeline. Mark each stage (IF/ID/EX/MEM/WB) per cycle. Identify all hazards and show where forwarding resolves them:
```
ADD  x1, x2, x3
SUB  x4, x1, x5
AND  x6, x1, x7
```

**Level 2 — Application:**
For this code, identify the load-use hazard. Explain why a stall is required and show the pipeline diagram with the bubble:
```
LW   x1, 0(x2)
ADD  x3, x1, x4
OR   x5, x6, x7
```

**Level 3 — Creation:**
Write the forwarding control equations for both `forward_A` and `forward_B` given these EX/MEM and MEM/WB signals. Account for priority when both EX and MEM hazards exist simultaneously for the same register.
