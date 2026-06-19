# Lesson 12: Branch Prediction — Static, Dynamic, Tournament

## The Branch Penalty Problem

In a 5-stage pipeline, branches are resolved in the EX (or MEM) stage. If a branch is taken, the 2-3 instructions already fetched after it are wrong-path and must be flushed. This **branch penalty** wastes cycles:

```
CPI = 1 + stall_penalty + branch_penalty
```

With branches comprising ~20% of instructions and 2-cycle penalty, unpipelined CPI ≈ 1.4. **Branch prediction** speculates on branch direction *before* resolution, reducing wasted cycles.

## Static Prediction

Fixed rules that do not learn from execution history.

### Always-Taken / Always-Not-Taken

Predict every branch as taken or not-taken. Always-taken achieves ~70% accuracy because most branches (especially loops) are taken.

### Backward-Taken, Forward-Not-Taken (BTFN)

Uses the sign of the branch offset:
- **Backward** branch (negative offset → likely a loop) → predict **taken**
- **Forward** branch (positive offset → likely an if/else) → predict **not-taken**

Very effective for inner loops. Accuracy: ~80-85%.

## Dynamic Prediction

Learns from past branch outcomes. Uses hardware tables indexed by branch PC.

### 1-Bit Predictor

Stores one bit per branch: last outcome. Toggle on mispredict.

```
Predict taken → mispredict → predict not-taken
```

Problem: a loop with N iterations mispredicts twice (enter and exit) because the bit toggles.

### 2-Bit Saturating Counter (Bimodal Predictor)

Four states that resist single mispredicts:

```
Strongly Taken (11)
    ↓ mispredict
Weakly Taken (10)
    ↓ mispredict
Weakly Not-Taken (01)
    ↓ mispredict
Strongly Not-Taken (00)
    ↑ predict correctly twice to climb back
```

A loop branch stays in "Strongly Taken" through many iterations and only moves to "Weakly Taken" on exit — one mispredict instead of two. Accuracy: ~85-90%.

**Table design**: `N` entries indexed by `PC[log2(N)+1 : 2]` (ignore low 2 bits since instructions are 4-byte aligned).

### Global History: Gshare

1-bit/2-bit predictors ignore *correlation* between branches. Gshare XORs a **global branch history register** (shift register of recent branch outcomes) with the PC to index the table:

```
index = PC[bits] XOR global_history[bits]
```

Two branches that execute together will share history context. Accuracy: ~90-95%.

## Tournament Predictor

Combines multiple predictors with a **selector/meta-predictor**:

```
                  ┌──────────────┐
      PC ────────►│ Local Predict │──────┐
                  │ (per-branch   │      │
                  │  history)     │      ▼
                  └──────────────┘   Selector ──► final prediction
                  ┌──────────────┐  (2-bit       ▲
      PC ────────►│Global Predict │  counter      │
                  │ (gshare)     │  per branch) ──┘
                  └──────────────┘
```

- **Local predictor**: per-branch history table. Good for correlated branches.
- **Global predictor**: gshare-style. Good for branches correlated with other branches.
- **Selector**: 2-bit counter per branch — learns which predictor is more accurate for that branch.

Accuracy: >95%. Used in Alpha 21264, Pentium III and beyond.

## Branch Target Buffer (BTB)

Prediction gives direction (taken/not-taken). The **BTB** provides the target address:

```
BTB = small cache indexed by PC
      Each entry: { valid, tag, target_address }
```

On instruction fetch, look up PC in BTB:
- Hit + predict taken → redirect PC to target_address
- Miss or predict not-taken → sequential PC (PC + 4)

Modern CPUs combine BTB with direction predictor for zero-cycle penalty on predicted-taken branches.

## Building It

We provide:
- SystemVerilog modules for bimodal, gshare, and tournament predictors
- A C simulator that reads branch traces and evaluates each predictor

See `code/branch_predictor.sv` and `code/simulator.c`.

## Using It

Modern processors use sophisticated predictors:
- **TAGE** (Tagged Geometric History): multiple tables with different history lengths, selected by tag match. >97% accuracy.
- **Perceptron predictors**: neural-network-style, used in AMD Zen and Samsung Exynos.
- **Deeply pipelined CPUs** (14-20 stages) have 3-5 cycle branch penalties, making prediction accuracy critical.

## Shipping It

A branch prediction subsystem is essential for any pipelined CPU achieving CPI close to 1. The hardware cost is modest (a few KB of SRAM) relative to the performance gain.

## Exercises

**Level 1 — Recall:**
For a 2-bit saturating counter predictor starting in state `10` (weakly taken), trace the state transitions for this sequence of outcomes: T, T, T, NT, NT, T. What is the prediction accuracy?

**Level 2 — Application:**
A branch at address `0x1004` is taken 9 times in a loop and not-taken on exit. Compare the number of mispredicts for: (a) 1-bit predictor, (b) 2-bit predictor, (c) BTFN. Which is best?

**Level 3 — Creation:**
Design a gshare predictor with a 10-bit global history and a 1024-entry table of 2-bit counters. Write the Verilog module, including update logic. What is the total storage in bits?
