# Lesson 11: Turing Machines & Variants

## Why This Matters

The Turing machine is *the* formal model of computation. The Church–Turing thesis states that anything computable is computable by a TM. Every algorithm, every programming language, every processor — all can be simulated by this simple machine with a tape and a head. Understanding TMs means understanding both the power and the limits of what machines can do, from the halting problem to complexity classes.

---

## Formal Definition

A Turing machine is a 7-tuple:

**M = (Q, Σ, Γ, δ, q₀, q_accept, q_reject)**

| Component | Meaning |
|-----------|---------|
| Q | Finite set of states |
| Σ | Input alphabet (does not contain blank ␣) |
| Γ | Tape alphabet (Σ ⊂ Γ, ␣ ∈ Γ) |
| δ | Transition function: Q × Γ → Q × Γ × {L, R} |
| q₀ ∈ Q | Start state |
| q_accept ∈ Q | Accept state |
| q_reject ∈ Q | Reject state (q_reject ≠ q_accept) |

### Execution Model

1. The input string is placed on the tape, one symbol per cell. All other cells contain the blank symbol ␣.
2. The head starts at position 0 (the leftmost input symbol). The machine is in state q₀.
3. At each step, the machine reads the symbol under the head.
4. It consults δ(current_state, symbol) to get (new_state, write_symbol, direction).
5. It writes write_symbol on the current cell, moves the head one cell left (L) or right (R), and enters new_state.
6. If new_state = q_accept, the machine **accepts** and halts.
7. If new_state = q_reject, the machine **rejects** and halts.
8. Otherwise, repeat from step 3.

The machine **halts** if it reaches q_accept or q_reject. Crucially, a TM may run forever without halting — this is the source of the halting problem.

### Why Infinite Tape?

The infinite tape is essential. A finite tape would make the TM equivalent to a finite automaton with more states. The tape provides unbounded memory, which is what separates TMs from simpler models. In practice, we simulate infinity with a step limit.

### Configuration

A **configuration** of a TM is a triple (state, tape_contents, head_position). Two configurations are equal if they agree on all three. A computation is a sequence of configurations starting from the initial configuration.

---

## Variants and Equivalence

### Multitape TM

A k-tape TM has k tapes, each with its own independently-moving head. The transition function reads all k symbols simultaneously: δ: Q × Γᵏ → Q × Γᵏ × {L, R}ᵏ.

**Equivalence to single-tape:** Encode all k tapes on one tape using a 2k-track encoding: `# a₁^ h₁ # b₁^ # a₂^ h₂ # ...` where `hᵢ` marks the head position on tape i. Each step of the multitape machine is simulated by a single-tape scan to read all tracks, then another scan to write. Simulation overhead: O(t²) time for a machine that runs in t steps.

### Nondeterministic TM (NTM)

The transition function maps to a *set* of possible transitions: δ: Q × Γ → 2^(Q × Γ × {L, R}). At each step, the machine "branches" into multiple computation paths. The NTM **accepts** if *any* path reaches q_accept.

**Equivalence to deterministic TM:** Use breadth-first search over the tree of configurations. Maintain a queue of unexplored configurations. Dequeue one, simulate one step on each branch, enqueue the results. If any path accepts, halt and accept. An NTM running in time t(n) can be simulated by a DTM in O(2^t(n)) time and O(t(n)) space.

### Doubly-Infinite Tape

The tape extends infinitely in both directions (left and right from the head's starting position).

**Equivalence to one-way-infinite:** Use two tracks on a one-way-infinite tape. Track 1 represents positions 0, 1, 2, … and track 2 represents −1, −2, −3, … (reversed). An additional flag marks which track the head is currently on. The simulation is straightforward with constant overhead.

### TM with Stay-Still

Some definitions allow δ to include S (stay in place) as a direction. This is strictly equivalent to the standard L/R model: replace S with a two-step sequence (write, move R, then move L and write again). The stay-still variant is sometimes used in textbook presentations because it simplifies certain machine descriptions, but it adds no computational power.

---

## Universal Turing Machine

A Universal Turing Machine (UTM) takes as input the encoding ⟨M, w⟩ of any TM M and input w, and simulates M on w. Alan Turing described this concept in 1936. The UTM's tape stores M's description (states, transitions) alongside a simulated copy of M's tape. Each simulated step requires the UTM to look up M's transition table and apply it.

This is the theoretical foundation of programmable computers: the UTM is a stored-program machine. Your laptop is a physical UTM — it loads different programs and simulates them.

---

## TM vs PDA

| Machine | Languages Recognized |
|---------|---------------------|
| DFA/NFA | Regular |
| PDA | Context-free |
| TM | Recursively enumerable (Turing-recognizable) |

TMs can also **decide** (halt on all inputs) languages that PDAs cannot, like {aⁿbⁿcⁿ} and {ww | w ∈ Σ*}. The gap between CFLs and RE languages is vast — TMs can recognize non-context-free, non-context-sensitive, and even uncomputable-looking languages (though they cannot decide all RE languages).

---

## Build It: TM Class

See `code/main.py` for a `TuringMachine` class that:
- Takes states, tape alphabet, transitions, start/accept/reject as constructor arguments.
- Runs on an input string with a configurable step limit.
- Returns (result, tape contents as a dict, steps taken).
- Visualizes the tape and head position with ASCII art.

Three example machines are provided: {aⁿbⁿcⁿ}, binary increment, and palindrome detector. Each machine includes a complete transition table and a demonstration run showing tape evolution.

---

## Use It

The TM is the yardstick for all of computer science:
- **Complexity classes** (P, NP, PSPACE, EXPTIME) are defined by resource-bounded TMs — what can a TM decide in polynomial time? Polynomial space?
- **Halting problem undecidability** is proved by diagonalization on TMs — no TM can decide whether an arbitrary TM halts on an arbitrary input.
- **Reductions** between problems use TM computation to show that if you could solve one problem, you could solve another.

---

## Ship It

A TM simulator that runs arbitrary machine definitions, reports accept/reject/non-halting (with a step limit), and shows step-by-step tape evolution. Lesson 12 builds this into a full-featured tool.

---

## Exercises

### Level 1 — Recall

Write the full transition table for a TM that accepts the language {wwᴿ | w ∈ {0,1}*} (palindromes of even length). How many states does your machine need?

### Level 2 — Application

Design a TM that computes unary multiplication: on input `aᵐ$bⁿ`, halts with `a^(m·n)` on the tape. Trace its execution on the input `aa$aaa` step by step, showing the tape after each transition.

### Level 3 — Creation

Prove informally that a 2-tape TM can be simulated by a 1-tape TM. Describe the encoding of two tapes on one, and outline the simulation procedure. Then implement this simulator and verify it works by running the {aⁿbⁿcⁿ} machine on both the 1-tape and 2-tape versions.
