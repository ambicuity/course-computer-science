# Sequential Logic — Latches, Flip-Flops, FSMs

> Memory in silicon. Every register, counter, and state machine starts here.

**Type:** Learn
**Languages:** SystemVerilog (HDL)
**Prerequisites:** Phase 06 lessons 01–03
**Time:** ~75 minutes

## Learning Objectives

- Distinguish combinational circuits (memoryless) from sequential circuits (stateful).
- Build SR latches, D latches, D flip-flops, and JK flip-flops in SystemVerilog.
- Design Mealy and Moore finite state machines and encode states correctly.
- Implement a traffic light controller FSM and a sequence detector FSM.
- Connect sequential logic to its role inside a CPU (registers, pipeline stages).

## The Problem

This lesson sits in **Phase 06 — Digital Logic & Computer Architecture**. Without the concept it teaches, you cannot
build the phase's capstone (A 5-stage pipelined RISC-V CPU in HDL with assembler.). Concretely, *not* knowing this means you get stuck the
moment you try to build registers, pipeline stages, counters, or any circuit that must remember what happened one cycle ago.

Lesson 03 gave you combinational logic — adders, multiplexers, decoders. Their output is a pure function of current inputs. Change the input, the output changes immediately. That is powerful but limited. You cannot build a counter, a register, or a CPU without *memory* — a circuit whose output depends on both current inputs **and past history**. That is sequential logic.

## The Concept

### Combinational vs. Sequential

| | Combinational | Sequential |
|---|---|---|
| Output depends on | Current inputs only | Current inputs + current state |
| Memory? | No | Yes — stored in feedback loops or clocked elements |
| Clock? | Not required | Almost always (synchronous design) |
| Examples | Adder, mux, decoder | Counter, register, FSM |

The fundamental building block is the **flip-flop**: a 1-bit memory element. Wire 32 of them together and you have a register. Wire registers to an ALU and you have a datapath. Sequential logic is how circuits gain a notion of *time*.

### The SR Latch (NOR-based)

The simplest memory element. Two cross-coupled NOR gates.

```
S ---|>o---\
      NOR   +--- Q
     /  |<--+
    |   |
    +-->|
      NOR   +--- Q̄
R ---|>o---/
```

- **S = 1, R = 0** → Set: Q = 1
- **S = 0, R = 1** → Reset: Q = 0
- **S = 0, R = 0** → Hold: Q keeps its previous value (memory!)
- **S = 1, R = 1** → **Forbidden** (race condition — both outputs try to go to 0 simultaneously)

The race condition at S = R = 1 is the whole reason more sophisticated latches exist.

### The D Latch

Solves the forbidden state by using a single data input `D` and an enable `EN`.

- **EN = 1**: Q follows D (transparent mode)
- **EN = 0**: Q holds its value (opaque mode)

Problem: while EN = 1, any glitch on D propagates straight through to Q. This makes timing analysis difficult in complex circuits.

### The D Flip-Flop (Edge-Triggered)

The workhorse of synchronous digital design. Captures D only on the **rising edge** of `clk` (posedge). At all other times, Q is stable.

```
D ----+
      |
     [D Q]---- Q
      |
clk --+> (posedge detector)
```

This is the element behind every register in a CPU. When your RISC-V pipeline says "latch the ALU result into EX/MEM register," it means 32 D flip-flops fire on the same clock edge.

**Setup time**: D must be stable *before* the clock edge (typically ~0.1 ns).
**Hold time**: D must remain stable *after* the clock edge (typically ~0.05 ns).

Violating either causes metastability — the flip-flop output oscillates unpredictably.

### The JK Flip-Flop

Eliminates the SR latch's forbidden state:

| J | K | Action |
|---|---|--------|
| 0 | 0 | Hold |
| 0 | 1 | Reset |
| 1 | 0 | Set |
| 1 | 1 | **Toggle** (Q = ~Q) |

The toggle behavior makes JK flip-flops natural building blocks for counters and frequency dividers. In modern ASIC/FPGA design, D flip-flops dominate, but JK is the historical bridge from latches to synchronous design.

### Finite State Machines (FSMs)

An FSM is a circuit that cycles through a fixed set of states based on inputs and clock edges. Two variants:

**Moore machine** — output depends on *state only*.

```
Input → [Next-State Logic] → [State Register] → [Output Logic] → Output
                  ↑                                      |
                  +---- clk ----+--------+
```

**Mealy machine** — output depends on *state and input*.

```
Input → [Next-State + Output Logic] → [State Register] → Output
                  ↑                           |
                  +-------- clk ------+-------+
```

Mealy machines can produce outputs faster (one cycle earlier) but are susceptible to glitches on inputs propagating to outputs. Moore machines are more predictable in timing.

**State encoding strategies:**

| Encoding | 4 states use | Flip-flops | Trade-off |
|----------|-------------|------------|-----------|
| Binary | 00, 01, 10, 11 | 2 | Minimum flip-flops, complex next-state logic |
| One-hot | 0001, 0010, 0100, 1000 | 4 | More flip-flops, simpler decoding (fast on FPGAs) |
| Gray | 00, 01, 11, 10 | 2 | Adjacent states differ by 1 bit — fewer glitches |

### Traffic Light Controller FSM

A concrete Moore machine. Four states cycling through a two-way intersection:

```
NS_GREEN → NS_YELLOW → EW_GREEN → EW_YELLOW → NS_GREEN ...
```

Each state produces four outputs: `ns_green`, `ns_yellow`, `ew_green`, `ew_yellow`. The FSM advances on each clock edge (in practice, a timer would trigger transitions every few seconds).

## Build It

All code is in `code/sequential.sv`. Walk through each module in order.

### Step 1: SR Latch (Combinational Feedback)

```systemverilog
module sr_latch (
    input  logic s, r,
    output logic q, qbar
);
    assign q    = ~(r | qbar);
    assign qbar = ~(s | q);
endmodule
```

This uses continuous assignment with feedback — the output depends on itself. Simulation requires an initial value or the latch stays at `x`.

### Step 2: D Latch (Level-Sensitive)

```systemverilog
module d_latch (
    input  logic d, en,
    output logic q
);
    always_latch begin
        if (en) q <= d;
    end
endmodule
```

`always_latch` is SystemVerilog's way of saying "this block implements combinational logic with feedback — the synthesizer should infer a latch."

### Step 3: D Flip-Flop (Edge-Triggered)

```systemverilog
module dff (
    input  logic       d, clk,
    output logic       q
);
    always_ff @(posedge clk) begin
        q <= d;
    end
endmodule
```

`always_ff` tells the synthesizer this is sequential (flip-flop) logic. The `posedge clk` sensitivity list means `q` updates only on the rising edge.

### Step 4: JK Flip-Flop

```systemverilog
module jk_ff (
    input  logic       j, k, clk,
    output logic       q
);
    always_ff @(posedge clk) begin
        case ({j, k})
            2'b00: q <= q;
            2'b01: q <= 1'b0;
            2'b10: q <= 1'b1;
            2'b11: q <= ~q;
        endcase
    end
endmodule
```

### Step 5: 4-Bit Counter (D Flip-Flops in Chain)

```systemverilog
module counter_4bit (
    input  logic       clk, rst,
    output logic [3:0] count
);
    always_ff @(posedge clk) begin
        if (rst) count <= 4'd0;
        else     count <= count + 4'd1;
    end
endmodule
```

Four D flip-flops share a clock. Each cycle, the count increments. The `rst` signal is asynchronous (checked before the clock edge in simulation).

### Step 6: Traffic Light FSM (Moore Machine)

Four states encoded in 2 bits. The output logic maps each state to the four light signals.

```systemverilog
module traffic_light_fsm (
    input  logic clk, rst,
    output logic ns_green, ns_yellow, ew_green, ew_yellow
);
    typedef enum logic [1:0] {
        NS_GREEN  = 2'b00,
        NS_YELLOW = 2'b01,
        EW_GREEN  = 2'b10,
        EW_YELLOW = 2'b11
    } state_t;

    state_t state, next_state;

    always_ff @(posedge clk) begin
        if (rst) state <= NS_GREEN;
        else     state <= next_state;
    end

    always_comb begin
        case (state)
            NS_GREEN:  next_state = NS_YELLOW;
            NS_YELLOW: next_state = EW_GREEN;
            EW_GREEN:  next_state = EW_YELLOW;
            EW_YELLOW: next_state = NS_GREEN;
            default:   next_state = NS_GREEN;
        endcase
    end

    always_comb begin
        ns_green  = (state == NS_GREEN);
        ns_yellow = (state == NS_YELLOW);
        ew_green  = (state == EW_GREEN);
        ew_yellow = (state == EW_YELLOW);
    end
endmodule
```

### Step 7: Sequence Detector "1011" (Mealy Machine)

Detects the bit pattern 1→0→1→1 on a serial input `din`, one bit per clock cycle. Output `detected` goes high for one cycle when the full pattern is seen.

```systemverilog
module sequence_detector_1011 (
    input  logic clk, rst, din,
    output logic detected
);
    typedef enum logic [1:0] {
        S0 = 2'b00, S1 = 2'b01, S2 = 2'b10, S3 = 2'b11
    } state_t;

    state_t state, next_state;

    always_ff @(posedge clk) begin
        if (rst) state <= S0;
        else     state <= next_state;
    end

    always_comb begin
        detected  = 1'b0;
        next_state = S0;
        case (state)
            S0: begin
                if (din) next_state = S1;
                else     next_state = S0;
            end
            S1: begin
                if (din) next_state = S1;  // still have a '1'
                else     next_state = S2;
            end
            S2: begin
                if (din) next_state = S3;
                else     next_state = S0;
            end
            S3: begin
                if (din) begin
                    detected   = 1'b1;
                    next_state = S1;       // overlap: last '1' can start new pattern
                end else begin
                    next_state = S2;
                end
            end
        endcase
    end
endmodule
```

## Use It

**Every register in a CPU is a D flip-flop.** When a RISC-V processor executes `add x1, x2, x3`, the register file reads `x2` and `x3` from banks of flip-flops, the ALU computes the sum, and on the next clock edge 32 D flip-flops in `x1` capture the result.

In the 5-stage pipeline you will build as the capstone, each pipeline register (IF/ID, ID/EX, EX/MEM, MEM/WB) is a collection of D flip-flops that fire simultaneously on `posedge clk`, passing the instruction and data forward one stage per cycle.

**FSMs in production hardware:**
- UART receivers use a Mealy FSM to sample bits at the correct baud rate.
- DDR memory controllers use FSMs to sequence read/write command phases.
- PCIe link training uses a complex FSM with dozens of states.

**State encoding in practice:** FPGA synthesis tools (Vivado, Quartus) let you choose encoding per FSM. One-hot is default on Xilinx FPGAs because each flip-flop maps to a dedicated LUT output. ASIC tools typically default to binary to save area.

## Read the Source

- `code/sequential.sv` — All modules for this lesson (latches, flip-flops, FSMs, testbench).
- [RISC-V spec, Chapter 2](https://riscv.org/technical/specifications/) — The ISA assumes sequential logic provides the register file and pipeline latches.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A sequential logic library: SR latch, D latch, DFF, JK flip-flop, counter, and two FSMs you can drop into any design.**

## Exercises

1. **Easy** — Build a 4-bit up/down counter using D flip-flops. Add a `dir` input: `dir=1` counts up, `dir=0` counts down. Write a testbench that verifies wrap-around from 15→0 and 0→15.

2. **Medium** — Implement the "1011" sequence detector as a **Moore** machine instead of Mealy. How many states do you need? Compare: does the Moore version detect the pattern one cycle later than the Mealy version? Prove it with a testbench waveform.

3. **Hard** — Design an FSM that controls a vending machine accepting nickels (5¢) and dimes (10¢), dispensing a 25¢ item. The FSM should track total deposited, assert `dispense` when ≥ 25¢, and assert `change` with the overage amount. Use binary encoding, then reimplement with one-hot. Synthesize both and compare LUT usage on a Lattice iCE40 (use `yosys + nextpnr`).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Sequential logic | "Circuit with memory" | Output depends on current inputs *and* stored state |
| SR latch | "Set-Reset latch" | Two cross-coupled NOR gates that hold 1 bit; forbidden state at S=R=1 |
| D latch | "Data latch" | Level-sensitive: Q follows D when EN=1, holds when EN=0 |
| D flip-flop | "DFF" | Edge-triggered: captures D on posedge clk; the atom of synchronous design |
| JK flip-flop | "Toggle flip-flop" | Like SR but J=K=1 toggles Q instead of entering forbidden state |
| Setup time | "Tsu" | D must be stable *before* clock edge to guarantee correct capture |
| Hold time | "Th" | D must remain stable *after* clock edge to avoid metastability |
| FSM | "State machine" | Circuit cycling through fixed states driven by inputs and clock |
| Mealy machine | "Output = f(state, input)" | FSM whose output depends on current state *and* current input |
| Moore machine | "Output = f(state)" | FSM whose output depends on current state only |
| One-hot encoding | "One flip-flop per state" | Each state gets a unique bit position set to 1; fast decoding, more area |
| Binary encoding | "Log₂ flip-flops per state" | States numbered 0, 1, 2, ... in minimum bits; compact but slower decode |

## Further Reading

- Harris & Harris, *Digital Design and Computer Architecture*, Ch. 3 (Latches and Flip-Flops) and Ch. 4 (FSMs).
- Wakerly, *Digital Design: Principles and Practices*, Ch. 7.
- Clifford E. Cummings, "State Machine Coding Techniques for Verilog HDL" — SNUG paper on synthesis-friendly FSM styles.
