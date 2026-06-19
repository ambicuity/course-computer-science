# Out-of-Order Execution & Tomasulo

> How modern CPUs hide latency by executing instructions in a different order than they appear in the program.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 06 lessons 01–12
**Time:** ~90 minutes

## Learning Objectives

- Understand why in-order pipelines stall and how out-of-order execution solves it.
- Implement Tomasulo's algorithm with reservation stations, CDB, and reorder buffer.
- Compare in-order vs out-of-order performance on dependency-heavy code.
- Ship a working Tomasulo simulator you can extend with branch prediction and scoreboarding.

## The Problem

Consider this sequence on a simple in-order pipeline:

```
LD  F1, 0(R1)     ; load takes 3 cycles
ADD F2, F1, F3    ; needs F1 — stalls
MUL F4, F5, F6    ; independent, but the pipeline is stuck waiting
SUB F7, F8, F9    ; also independent — still waiting
```

The `ADD` must wait for the load. On an in-order pipeline, `MUL` and `SUB` stall too even though they have nothing to do with `F1`. Functional units sit idle while perfectly good work waits in line.

The fix: let instructions **execute as soon as their operands are ready**, regardless of program order. Results retire in program order to keep exceptions precise. This is **out-of-order execution**, and the classic implementation is **Tomasulo's algorithm**, invented by Robert Tomasulo at IBM in 1967 for the IBM 360/91 floating-point unit.

## The Concept

Tomasulo's algorithm decouples **issue** (fetch and decode) from **execute** (running on hardware). Three structures make this possible.

### Reservation Stations (RS)

Each functional unit has reservation stations — small buffers holding instructions waiting for operands. Each entry stores:

| Field | Meaning |
|-------|---------|
| `op` | Operation (ADD, MUL, LD, etc.) |
| `vj`, `vk` | Operand *values* (if available) |
| `qj`, `qk` | **Tags** — which RS will produce the operand (if pending) |
| `dest` | ROB entry name |
| `busy` | Whether this station is occupied |

An instruction is ready when both tags are zero. This is **register renaming** — tags replace register names, eliminating false WAR/WAW dependencies because each result is identified by its RS tag, not its architectural register.

### Common Data Bus (CDB)

When a functional unit finishes, it broadcasts the result and its tag on the CDB. Every RS watches: if its `qj` or `qk` matches, it captures the value and clears the tag. One broadcast resolves multiple dependencies simultaneously.

### Reorder Buffer (ROB)

A FIFO queue ensuring instructions commit in program order. Each entry tracks state (Issue → Execute → Writeback → Commit), destination register, and result value. The ROB guarantees **precise exceptions** and eliminates WAW hazards — writes go to the ROB, not the register file, so two writes to the same register never conflict.

### The Four Phases

1. **Issue**: Decode instruction. Allocate a free RS and ROB entry. Read register status to fill `vj`/`vk` or `qj`/`qk`. Update register status to point to this ROB entry.
2. **Execute**: When both operands are ready, dispatch to the functional unit. Multi-cycle ops take multiple cycles.
3. **Writeback**: Broadcast result on the CDB. All waiting RS entries capture the value. Free the producing RS.
4. **Commit**: When the ROB head reaches Writeback state, write the value to the register file. Free the ROB entry. Strict program order.

### Worked Example

```
LD  F1, 0(R1)     ; cycle 1: issues, RS1. Latency = 3
ADD F2, F1, F3    ; cycle 2: issues, RS2. qj = RS1 (waiting)
MUL F4, F2, F5    ; cycle 3: issues, RS3. qj = RS2 (waiting)
SUB F6, F7, F8    ; cycle 3: issues, RS4. operands ready → executes immediately
```

| Cycle | Event |
|-------|-------|
| 1 | LD issues to RS1 |
| 2 | ADD issues, F1 not ready → qj = RS1 |
| 3 | MUL issues, F2 not ready → qj = RS2. SUB issues and executes (1-cycle ALU) |
| 4 | LD finishes, CDB broadcasts. ADD captures F1, qj cleared. SUB commits. |
| 5 | ADD executes, CDB broadcasts. MUL captures F2. LD commits. |
| 7 | MUL executes. ADD commits. |

SUB ran in parallel with the load chain — impossible on an in-order machine.

## Build It

See `code/main.py` for the complete simulator. Key classes:

- `ReservationStation`: op, vj, vk, qj, qk, dest, busy, cycles_left
- `ROBEntry`: instruction, state, dest, value
- `TomasuloSimulator`: issue(), execute(), writeback(), commit(), run(program)

The simulator supports ADD, SUB, MUL, and LD operations with configurable numbers of functional units. It logs every phase transition and prints full RS/ROB/register-status state.

### Running the Demo

```bash
python code/main.py
```

Three demos run automatically:
1. **Dependency chain** — LD→ADD→MUL with independent SUB sneaking in during the wait
2. **Independent burst** — five independent ALU ops showing maximum parallelism
3. **WAW hazard** — two instructions writing F1; ROB commits in program order

## Use It

Every modern high-performance CPU uses out-of-order execution with Tomasulo-derived structures:

- **Intel** Core/Alder Lake: 512+ entry ROB, 12+ execution ports, multiple reservation station clusters. Intel calls RS entries "Scheduler entries."
- **AMD** Zen 4: 320-entry ROB, 6 integer + 4 FP execution units, unified reservation stations.
- **ARM** Cortex-X4: 320-entry ROB. ARM's big.LITTLE pairs OoO "big" cores with in-order "LITTLE" cores for power efficiency.
- **Apple** M-series: 600+ entry ROB — among the deepest in the industry, contributing to single-thread performance leadership.

The core idea is identical to our simulator: issue to reservation stations, wait via tag matching, broadcast on CDB, commit in order from the ROB. Real CPUs add memory disambiguation, branch prediction, and load/store queues on top.

## Read the Source

- **Hennessy & Patterson, *Computer Architecture: A Quantitative Approach*, Ch. 3** — textbook treatment of Tomasulo, ROB, and modern extensions.
- **Tomasulo (1967), "An Efficient Algorithm for Exploiting Multiple Arithmetic Units"** — the original IBM paper. Remarkably clear.

## Ship It

The reusable artifact lives in `outputs/`:

- **`tomasulo_sim.py`** — A self-contained Tomasulo simulator. Import it, define an instruction list, and call `run()` to get cycle-by-cycle execution traces with full RS/ROB state dumps.

## Exercises

1. **Easy** — Reproduce `TomasuloSimulator` from scratch without looking at the code. Verify matching cycle counts.
2. **Medium** — Write `in_order_run(program)` that stalls on unavailable operands. Compare speedup vs Tomasulo on a dependency-heavy program.
3. **Hard** — Add branch prediction (2-bit saturating counter). Speculatively issue from the predicted path. On misprediction, flush the ROB.
4. **Hard** — Implement scoreboarding (CDC 6600-style) as an alternative. Compare throughput to Tomasulo — where does it bottleneck?

## Key Terms

| Term | What it actually means |
|------|------------------------|
| Reservation Station | Buffer entry holding an instruction + operand values or dependency tags |
| Common Data Bus | Shared broadcast bus carrying results to all waiting RS entries |
| Reorder Buffer | FIFO queue ensuring in-order commit of out-of-order results |
| Register Renaming | Mapping architectural registers to RS tags to eliminate false dependencies |
| Precise Exception | Exception model where all prior instructions have committed, none after |
| WAW Hazard | Two writes to same register — resolved by ROB (writes go to buffer, not register file) |
| WAR Hazard | Later write overtaking earlier read — resolved because operands are captured at issue |

## Further Reading

- Hennessy & Patterson, *Computer Architecture: A Quantitative Approach*, 6th ed., Ch. 3
- Shen & Lipasti, *Modern Processor Design*, Ch. 3
- Agner Fog's microarchitecture manuals (https://agner.org/optimize/) — real RS/ROB sizes
