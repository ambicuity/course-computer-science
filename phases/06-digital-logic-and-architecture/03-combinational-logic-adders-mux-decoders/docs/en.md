# Combinational Logic — Adders, Mux, Decoders

> From single gates to complete arithmetic and routing circuits — the building blocks of every processor datapath.

**Type:** Learn
**Languages:** SystemVerilog (HDL)
**Prerequisites:** Phase 06 lessons 01–02
**Time:** ~75 minutes

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

Combinational circuits compute outputs from inputs with **no memory** — the output is a pure function of the current inputs. Three families dominate processor design.

### Adders

**Half adder** — single-bit addition with no carry input. `sum = a XOR b`, `carry = a AND b`. Cannot chain.

**Full adder** — adds `a + b + carry_in`, producing `sum` and `carry_out`. Built from two half adders plus an OR gate:

```
sum      = a XOR b XOR carry_in
carry_out = (a AND b) OR (carry_in AND (a XOR b))
```

**Ripple-carry adder** — chain N full adders. Each `carry_out` feeds the next stage's `carry_in`. Simple but **O(N)** delay — the carry must ripple through every stage.

**Carry-lookahead adder (CLA)** — computes carries in parallel using two signals per bit:

```
Generate  G_i = a_i AND b_i          (this stage produces a carry)
Propagate P_i = a_i XOR b_i          (this stage passes a carry through)
```

Carries: `c_{i+1} = G_i OR (P_i AND c_i)`, expanded recursively. All carries resolve in **O(log N)** delay.

### Multiplexers (MUX)

Selects one of N inputs using log2(N) select bits. A 2:1 mux: `y = sel ? b : a`. Larger muxes compose 2:1 muxes in a tree.

| Type  | Inputs | Select bits |
|-------|--------|-------------|
| 2:1   |   2    |      1      |
| 4:1   |   4    |      2      |
| 8:1   |   8    |      3      |

### Decoders and Encoders

**Decoder** — n-bit input activates exactly one of 2^n outputs (one-hot). A 2-to-4 decoder with enable:

| en | a[1:0] | y[3:0] |
|----|--------|--------|
| 0  |  xx    | 0000   |
| 1  |  00    | 0001   |
| 1  |  01    | 0010   |
| 1  |  10    | 0100   |
| 1  |  11    | 1000   |

**Encoder** — inverse of decoder. A **priority encoder** handles multiple active inputs by selecting the highest-indexed one and asserting a `valid` flag.

## Build It

All modules are implemented in `code/combinational.sv`.

### Step 1: Half and Full Adders

The half adder is a single assign statement. The full adder instantiates two half adders — demonstrating structural composition. See `half_adder` and `full_adder` in `code/combinational.sv`.

### Step 2: Ripple-Carry and Carry-Lookahead Adders

`ripple_carry_adder_4bit` chains four full adders. `cla_adder_4bit` computes G/P signals and resolves all carries in two gate levels — same result, lower delay. The testbench exhaustively verifies all 512 input combinations match.

### Step 3: Multiplexers

`mux_2to1` is a ternary operator. `mux_4to1` builds a tree of three 2:1 muxes — demonstrating hierarchical composition.

### Step 4: Decoder and Priority Encoder

`decoder_2to4` uses a shifted one-hot output: `y = en ? (1 << a) : 0`. `encoder_4to2` scans from MSB down in a priority chain, asserting `valid` when any input is active.

## Use It

Combinational building blocks are the atoms of processor datapaths:

- **ALU** (Lesson 05) uses adders for arithmetic, muxes to select the operation result.
- **Register file** (Lesson 06) uses decoders for write-enable selection and muxes for read-port routing.
- **Instruction decode** (Lesson 08) uses decoders to break opcodes into one-hot control signals.

Production processors use optimized variants — Kogge-Stone/Brent-Kung adders for O(log N) carry delay, barrel shifters from mux arrays, PLAs replacing simple decoders for complex opcode mappings.

## Read the Source

- `code/combinational.sv` — full SystemVerilog implementation plus testbench covering all modules.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A combinational logic module library** — half adder through carry-lookahead adder, mux, decoder, and encoder. Import these into later lessons when building the ALU, register file, and datapath.

## Exercises

1. **Easy** — Build a 4-bit carry-lookahead adder in SystemVerilog without referencing the lesson code. Verify correctness against the ripple-carry adder for all 512 input combinations.
2. **Medium** — Build a 4:1 multiplexer using only 2:1 multiplexers as primitives. Prove that the number of 2:1 muxes needed is always N-1 for an N:1 mux.
3. **Hard** — Implement a BCD (binary-coded decimal) to 7-segment display decoder. Input: 4-bit BCD digit (0–9). Output: 7-bit segment pattern (a–g). Handle invalid BCD inputs (10–15) by displaying blank.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Combinational | "pure logic, no state" | Output depends only on current inputs; no memory elements |
| Half adder | "single-bit add, no carry in" | XOR for sum, AND for carry — cannot chain without carry input |
| Full adder | "adds three bits" | Two half adders + OR gate; accepts carry_in for chaining |
| Ripple-carry | "serial add, simple but slow" | N full adders chained; O(N) carry propagation delay |
| Carry-lookahead | "parallel carry, fast" | Generate/propagate signals resolve carries in O(log N) |
| Multiplexer | "mux, selector" | Selects one of N inputs using log2(N) select bits |
| Decoder | "one-hot enabler" | n-bit input activates exactly one of 2^n outputs |
| Priority encoder | "highest-wins encoder" | Converts highest-active input to binary index with valid flag |

## Further Reading

- Harris & Harris, *Digital Design and Computer Architecture*, Ch. 5 — Combinational Logic Design.
- Patterson & Hennessy, *Computer Organization and Design*, Appendix B — combinational building blocks.
- Wikipedia: [Carry-lookahead adder](https://en.wikipedia.org/wiki/Carry-lookahead_adder)
