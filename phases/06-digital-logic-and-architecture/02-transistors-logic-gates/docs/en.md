# Transistors → Logic Gates

> Transistors → Logic Gates — the part of CS you can't skip.

**Type:** Learn
**Languages:** SystemVerilog (HDL)
**Prerequisites:** Phase 06 lessons 01–01
**Time:** ~60 minutes

## Learning Objectives

- Understand the core concept introduced in this lesson and why it matters.
- Implement the lesson's "Build It" artifact from scratch in one of: SystemVerilog (HDL).
- Compare your from-scratch implementation against the production tool used in industry.
- Ship the reusable artifact (see "Ship It") and add it to your toolbox.

## The Problem

This lesson sits in **Phase 06 — Digital Logic & Computer Architecture**. Without the concept it teaches, you cannot
build the phase's capstone (A 5-stage pipelined RISC-V CPU in HDL with assembler.). Concretely, *not* knowing this means you get stuck the
moment you try to walk down from instruction to transistor, then back up: alu, pipeline, cache, mmu.

The next few sections walk through the smallest concrete scenario where this gap hurts, then build
the mental model, then the code, then the production equivalent.

## The Concept

A CPU executes billions of instructions per second. Every one of those instructions resolves to a pattern of voltages on wires — HIGH (logic 1, ~1V) or LOW (logic 0, ~0V). Logic gates are the combinational circuits that compute on those voltages, and CMOS transistors are the physical switches that build the gates.

### MOSFET basics

A **MOSFET** (Metal-Oxide Semiconductor Field-Effect Transistor) has three terminals: **gate**, **source**, and **drain**. The gate voltage controls whether current flows between source and drain.

- **NMOS transistor** — conducts when gate is HIGH (voltage applied → channel forms). Connects its source/drain path to ground (LOW).
- **PMOS transistor** — conducts when gate is LOW (no voltage → channel forms). Connects its source/drain path to VDD (HIGH).

Think of NMOS as a "normally-open switch that closes on HIGH" and PMOS as a "normally-open switch that closes on LOW." They are complements of each other.

### CMOS inverter (NOT gate)

The simplest CMOS gate is the **inverter**:

```
        VDD
         |
     [PMOS]    ← pull-up network
         |
  A ──gate──+── output Y
         |
     [NMOS]    ← pull-down network
         |
        GND
```

- When A = 0: PMOS conducts (pulls Y to VDD → 1), NMOS off. Output = 1.
- When A = 1: NMOS conducts (pulls Y to GND → 0), PMOS off. Output = 0.

**Rule of thumb for CMOS:** PMOS transistors form the pull-up network (PUN) to VDD, NMOS form the pull-down network (PDN) to GND. They always work in complementary pairs — exactly one network conducts at a time, so there is no static power draw.

### NAND as the fundamental gate

The **NAND gate** is the workhorse of CMOS design. Its CMOS implementation uses 4 transistors (2 PMOS in parallel for PUN, 2 NMOS in series for PDN):

- A=0, B=0 → PMOS pair pulls up → Y=1
- A=0, B=1 → at least one PMOS on → Y=1
- A=1, B=0 → at least one PMOS on → Y=1
- A=1, B=1 → both NMOS conduct in series → Y=0

NAND requires only 4 transistors. An AND gate would need 6 (NAND + inverter). This is why NAND is considered the "universal" CMOS gate.

### Gate truth tables

| Gate | Symbol | A=0 B=0 | A=0 B=1 | A=1 B=0 | A=1 B=1 |
|------|--------|---------|---------|---------|---------|
| NOT  | ~A     | 1       | —       | 0       | —       |
| AND  | A & B  | 0       | 0       | 0       | 1       |
| OR   | A \| B | 0       | 1       | 1       | 1       |
| NAND | ~(A&B) | 1       | 1       | 1       | 0       |
| NOR  | ~(A\|B)| 1       | 0       | 0       | 0       |
| XOR  | A ^ B  | 0       | 1       | 1       | 0       |
| XNOR | ~(A^B) | 1       | 0       | 0       | 1       |

### De Morgan's laws

De Morgan's laws show that AND and OR are duals when you invert inputs and outputs:

- **NAND = bubbled OR:** `~(A & B)` ≡ `~A | ~B` — a NAND gate is an OR gate with inverted inputs.
- **NOR = bubbled AND:** `~(A | B)` ≡ `~A & ~B` — a NOR gate is an AND gate with inverted inputs.

This is why you see "bubbles" on gate diagrams: an inversion bubble on an OR gate's inputs turns it into a NAND, and vice versa.

### Gate delay, fan-in, fan-out

Real gates don't switch instantly. **Propagation delay** (t_pd) is the time from an input change to a stable output. For CMOS gates this is typically 10–100 ps in modern processes. Delays add up across gate chains — a 64-bit ripple-carry adder built from 1-bit full adders has 64 × gate-delay worth of propagation.

- **Fan-in** — the number of inputs a gate can accept. CMOS NAND with >2 inputs degrades performance (more series transistors → higher resistance).
- **Fan-out** — the number of gate inputs a single output can drive. Each driven input adds capacitance, increasing delay.

### Universality of NAND and NOR

A remarkable property: **NAND alone** (or NOR alone) is sufficient to build any combinational Boolean function. The proof is constructive:

- NOT(x) = NAND(x, x)
- AND(x, y) = NOT(NAND(x, y))
- OR(x, y) = NAND(NOT(x), NOT(y))  [De Morgan]

Since {NOT, AND, OR} is functionally complete, and NAND can produce all three, NAND is universal. The same argument works for NOR.

In practice, ASIC standard-cell libraries provide all gate types for convenience, but internally they are all built from CMOS transistor pairs.

## Build It

See `code/gates.sv` for the full implementation. Each gate is a one-line `assign`:

```systemverilog
module not_gate  (input logic a,      output logic y); assign y = ~a;       endmodule
module and_gate  (input logic a, b,   output logic y); assign y = a & b;    endmodule
module or_gate   (input logic a, b,   output logic y); assign y = a | b;    endmodule
module nand_gate (input logic a, b,   output logic y); assign y = ~(a & b); endmodule
module nor_gate  (input logic a, b,   output logic y); assign y = ~(a | b); endmodule
module xor_gate  (input logic a, b,   output logic y); assign y = a ^ b;    endmodule
module xnor_gate (input logic a, b,   output logic y); assign y = ~(a ^ b); endmodule
```

The testbench (`tb_gates` in the same file) instantiates all 7 gates, loops through all 4 input combinations, and compares outputs against expected truth tables. Run with: `iverilog -g2012 code/gates.sv -o gates && vvp gates`.

## Use It

Real chip design uses **standard-cell libraries** (e.g., TSMC N7, Samsung 5nm). Each cell is a pre-characterized CMOS circuit with known area, power, and timing:

- A `NAND2X1` cell: 2-input NAND, 1× drive strength — ~6 transistors including tap cells.
- A `INVX4` cell: inverter, 4× drive strength — wider transistors for higher fan-out.
- An `XOR2X1` cell: 2-input XOR — typically 8–12 transistors internally.

When you write `assign y = a & b;` in SystemVerilog, the synthesis tool (Yosys, Synopsys Design Compiler, Cadence Genus) maps it to a library cell from your target process. The tool picks the cell size based on timing constraints and load capacitance.

The gap between HDL code and transistor layout is exactly what synthesis and place-and-route tools fill in. You write logical intent; the tool selects physical implementations.

## Read the Source

- `code/gates.sv` — complete gate implementations with exhaustive testbench.
- [Nangate Open-Cell Library](https://github.com/nangate/nangate45nm) — a real 45nm standard-cell library used in open-source EDA tools.

## Ship It

The reusable artifact produced by this lesson lives in `code/gates.sv`. For this lesson it is:

- **A self-contained reference snippet you can reuse in later phases.**

The gate modules from `gates.sv` can be imported directly into lessons 03 (combinational logic), 05 (ALU), and beyond.

## Exercises

1. **Easy** — Build an XOR gate using only NAND gates (no XOR operator allowed). Draw the schematic and verify with a testbench.
2. **Medium** — Prove NAND universality by constructing NOT, AND, OR, and XOR from NAND-only, then write a SystemVerilog module `nand_only_xor` that implements XOR using only `nand_gate` instances. Test it exhaustively.
3. **Hard** — A 4-bit ripple-carry adder is a chain of 1-bit full adders. If each gate has t_pd = 50 ps and a full adder uses 2 gate levels (sum path) + 1 carry gate, what is the worst-case propagation delay for all 4 bits? Build the adder in SystemVerilog and use `$time` in a testbench to measure the actual delay from input to final carry-out.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| CMOS | "Complementary MOS" | IC technology using paired NMOS/PMOS transistors; near-zero static power |
| NMOS | "N-channel MOSFET" | MOSFET that conducts when gate voltage is HIGH |
| PMOS | "P-channel MOSFET" | MOSFET that conducts when gate voltage is LOW |
| Propagation delay | "Gate delay" | Time from input transition to valid output, typically 10–100 ps |
| Fan-in | "Number of inputs" | How many input signals a single gate accepts |
| Fan-out | "Drive strength" | How many downstream inputs a gate's output can reliably drive |
| Universality | "NAND can do everything" | A single gate type is sufficient to build any Boolean function |

## Further Reading

- Harris & Harris, *Digital Design and Computer Architecture*, Ch. 1–2 (CMOS transistors and gate implementations).
- Weste & Harris, *CMOS VLSI Design*, Ch. 2 (transistor theory and gate design).
- Rabaey, *Digital Integrated Circuits*, Ch. 6 (combinational gate circuit design).
