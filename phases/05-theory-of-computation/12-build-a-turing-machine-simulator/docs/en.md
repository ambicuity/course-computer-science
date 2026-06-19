# Lesson 12: Build a Turing Machine Simulator

## Why This Matters

Building a TM simulator isn't just an exercise — it's the computational equivalent of building a universal engine. A simulator that can load any TM definition and run it *is* a Universal Turing Machine. This project cements your understanding of the formal model, gives you a tool for exploring decidability and complexity, and produces something genuinely educational: a hands-on way to see computation unfold step by step.

---

## Feature Specification

Your simulator must support:

1. **Step-by-step execution** — advance one transition at a time, inspecting state and tape.
2. **Run to completion** — execute until accept, reject, or a configurable step limit.
3. **Tape visualization** — ASCII art showing cells, symbols, and head position.
4. **Trace mode** — log every transition with state, symbol read, symbol written, and direction.
5. **Breakpoints** — pause execution when the machine enters a specified state.
6. **JSON import/export** — save and load TM definitions as portable JSON files.
7. **State diagram export** — produce a DOT-language graph of the transition function for Graphviz rendering.
8. **Undo** — step backward through the execution history.

---

## Architecture

```
Simulator
├── TMDefinition (serializable machine description)
│   ├── states, tape_alphabet, transitions
│   ├── start, accept, reject
│   └── delta(state, symbol) → (new_state, write, direction)
├── Execution engine
│   ├── step() → result | None
│   ├── run(max_steps) → result
│   └── undo() — pop from history
├── Tape (dict: position → symbol, sparse representation)
├── History (stack of snapshots for undo)
└── I/O
    ├── to_json() / from_json()
    ├── to_dot() — Graphviz export
    └── tape_display() — ASCII visualization
```

### The `Simulator` Class

| Method | Purpose |
|--------|---------|
| `__init__(tm_def)` | Load TM from a TMDefinition object |
| `step()` | Execute one transition; returns 'accept', 'reject', 'breakpoint', or None |
| `run(max_steps)` | Run to halt or step limit; returns result string |
| `reset(input_string)` | Re-initialize tape with input and reset state/head |
| `set_breakpoint(state)` | Add a breakpoint at the given state |
| `tape_display(window)` | ASCII view of tape around head with state/step info |
| `trace()` | Return the full execution log as a string |
| `to_json()` | Export TM definition as JSON string |
| `to_dot()` | Export TM as Graphviz DOT graph |
| `undo()` | Revert to previous snapshot |

The tape uses a sparse dictionary representation: only non-blank cells are stored. Reading a blank returns the machine's blank symbol. This gives theoretically infinite tape with finite memory.

---

## Example Machines (5+)

| # | Machine | Language / Function |
|---|---------|---------------------|
| 1 | aⁿbⁿcⁿ | { aⁿbⁿcⁿ | n ≥ 1 } — marks a, finds b, finds c, repeats |
| 2 | Binary addition | Adds two binary numbers separated by `+` |
| 3 | Unary multiplication | `aᵐ$bⁿ → a^(m·n)` — repeated addition |
| 4 | Palindrome detector | Palindromes over {0, 1} — matches first and last symbols |
| 5 | 3-state Busy Beaver | Maximizes 1s on blank tape before halting (6 steps, 6 ones) |

Each machine is defined as a `TMDefinition` dataclass with a complete transition table. You can export any machine to JSON and reload it later.

---

## Build It

Both `code/main.py` (Python) and `code/main.rs` (Rust) implement the simulator. Python is the full-featured version with JSON import/export, DOT graph generation, breakpoints, and undo. Rust provides a leaner, type-safe implementation with the same core execution engine and tape visualization.

The Python version is approximately 350 lines. The Rust version is approximately 300 lines. Both include all five example machines ready to run.

---

## Use It

Educational TM simulators like [turingmachinesimulator.com](https://turingmachinesimulator.com) use exactly this architecture. They let students load predefined machines, step through execution, and visualize the tape. Such tools are used in university courses worldwide to teach:
- How TMs recognize non-context-free languages like {aⁿbⁿcⁿ}.
- The difference between accepting and rejecting computations.
- How the Busy Beaver function grows faster than any computable function.

Your simulator can serve the same purpose. Load a machine, provide input, and step through to see computation in action.

---

## Ship It

A CLI tool with subcommands:

```bash
$ tm-sim run examples/anbncn.json --input "aabbcc" --trace
$ tm-sim export examples/binary_add.json --format dot | dot -Tpng > machine.png
$ tm-sim step examples/busy_beaver.json --breakpoint HALT
$ tm-sim undo                          # go back one step
$ tm-sim list                          # show available example machines
```

The CLI reads TM definitions from JSON files, runs them on given input, and outputs results. The `--trace` flag prints every transition. The `--format dot` flag outputs a Graphviz graph. The `--breakpoint` flag pauses at a named state.

---

## Design Decisions

- **Sparse tape:** Using a dictionary instead of a list means the tape grows only as needed. Reading an absent position returns the blank symbol.
- **Snapshot history:** Each step records (state, head, tape_copy). Undo pops the last snapshot. This trades memory for simplicity — no need to reverse transitions.
- **Step limit:** TMs can loop forever. The `max_steps` parameter prevents infinite execution. Returning "limit" signals that the machine did not halt within budget.
- **JSON format:** The TM definition is a plain dictionary with keys for name, states, tape_alphabet, transitions, start, accept, reject, and blank. Transitions are nested dicts: `{state: {symbol: [new_state, write, direction]}}`.

---

## Exercises

### Level 1 — Recall

Load the `binary_addition` machine from JSON. Run it on the input `"101+11"` and verify the output tape shows the correct sum. What binary number does the result represent?

### Level 2 — Application

Add a **replay** feature: record every (state, tape, head) tuple during execution. Implement a command that lets the user scrub forward and backward through the execution history, displaying the tape at each step.

### Level 3 — Creation

Implement two-tape TM support. Extend your simulator so it can load and run 2-tape machines with a transition function δ: Q × Γ² → Q × Γ² × {L, R}². Demonstrate equivalence by implementing the {aⁿbⁿcⁿ} language on both a 1-tape and a 2-tape machine — the 2-tape version should have a simpler transition table.
