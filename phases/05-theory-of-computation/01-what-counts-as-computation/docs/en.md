# What Counts as Computation?

> Before you can ask "can a computer solve this?" you need a precise definition of what "computer" means. Theory of Computation gives you that definition — and tells you the answer is sometimes *no*.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 04
**Time:** ~45 minutes

## Learning Objectives

- Define "computation" formally as a model that maps inputs to outputs via a finite set of deterministic steps.
- Explain the Church-Turing thesis and why it unifies disparate models of computation.
- Build a finite-state-machine simulator (a vending machine) and identify what it *cannot* compute.
- Place finite automata, pushdown automata, and Turing machines on a power hierarchy.

## The Problem

Every program you have written so far runs on a real machine — a CPU with finite registers, finite RAM, a clock. But when we study computation *theoretically*, we strip away the hardware and ask: **what is the simplest mathematical model that captures the idea of "computing"?**

Why does this matter? Because without a formal model, you cannot prove that a problem is unsolvable. You might spend years trying to write a program that detects infinite loops in arbitrary code — but the halting problem (Lesson 13) will show you that *no* such program exists, for *any* machine. That result depends entirely on having a precise definition of "machine."

The first step is understanding what models exist, how they relate, and where a vending machine fits.

## The Concept

### What is a computation model?

A **computation model** is a mathematical object with three parts:

1. **Input** — a finite string over some alphabet.
2. **State** — the internal configuration of the machine.
3. **Transition function** — rules that say "in state *q*, seeing input *a*, go to state *q'* and (optionally) produce output."

The model is *deterministic* if the transition function always gives exactly one next state. It is *nondeterministic* if it may offer several (we explore this in Lesson 03).

### Finite vs infinite models

The critical axis of power is **memory**:

| Model | Memory | Can recognize | Can't recognize |
|-------|--------|---------------|-----------------|
| Finite Automaton (DFA/NFA) | Fixed number of states (finite) | Regular languages (e.g., "strings ending in 01") | {0ⁿ1ⁿ \| n ≥ 0} |
| Pushdown Automaton (PDA) | Finite states + unbounded stack | Context-free languages (e.g., balanced parentheses) | {aⁿbⁿcⁿ \| n ≥ 0} |
| Turing Machine | Finite states + unbounded tape (read/write) | *Everything* that is "computable" | Nothing known to be beyond it (by definition) |

Each row strictly subsumes the previous. The jump from PDA to Turing machine is the biggest: a Turing machine can simulate *any* algorithm any real computer can run.

### The Church-Turing thesis

In 1936, Alonzo Church (lambda calculus) and Alan Turing (Turing machines) independently proposed models of computation. Church's was algebraic; Turing's was mechanical. They proved their models were **equivalent**: anything one can compute, the other can too.

This led to the **Church-Turing thesis**:

> Any function that is "effectively calculable" (i.e., computable by any conceivable mechanical procedure) is computable by a Turing machine.

This is not a theorem — it is a *thesis*, because "effectively calculable" is informal. But every model ever proposed (lambda calculus, Post systems, recursive functions, RAM machines, your Python interpreter) has been shown equivalent to Turing machines. No counterexample exists.

**Why it matters:** When we later prove something is *not* Turing-computable (the halting problem), that result applies to every reasonable model of computation — Python, C, Haskell, your laptop, a hypothetical quantum computer with infinite memory. The limits are fundamental, not engineering.

### The hierarchy preview

```
Finite Automata  ⊂  Pushdown Automata  ⊂  Turing Machines  ⊃  ???
   (regular)           (context-free)        (recursively      (undecidable
                                              enumerable)        problems)
```

Phase 05 walks this hierarchy bottom to top. Today we start at the bottom.

### A concrete example: the vending machine

A vending machine is a finite automaton. It has a small fixed number of states (idle, 5¢ inserted, 10¢ inserted, 15¢ inserted, dispensing). It reads a sequence of coins (5¢, 10¢, 25¢) and transitions between states. When it reaches ≥ 25¢ in credit, it dispenses a product and returns to idle.

It has *no tape, no stack, no variable storage*. It cannot count arbitrary quantities — it can only track a fixed range. That is the hallmark of finite computation.

## Build It

### Step 1: Minimal VendingMachine

The `code/main.py` file implements a `VendingMachine` class as a finite-state machine. States are simple strings; transitions are a dictionary mapping `(state, input)` → `next_state`. The machine tracks credit up to a maximum and dispenses when credit ≥ price.

```python
from code.main import VendingMachine

vm = VendingMachine(price=25)
vm.insert(10)  # state: credit_10
vm.insert(10)  # state: credit_20
vm.insert(5)   # state: credit_25 → DISPENSE → idle
```

### Step 2: What can't it compute?

The vending machine **cannot** recognize the language {5¢, 10¢}ⁿ where the number of 5¢ coins equals the number of 10¢ coins. That requires *counting* to an arbitrary number — infinite memory. A finite automaton has only finitely many states, so by the pigeonhole principle (Phase 01!), on long enough inputs it must repeat a state and lose count.

This is a preview of the pumping lemma (Lesson 06).

## Use It

Real vending machines are finite automata — their control logic is literally a state machine implemented in firmware or even relay logic. But the same pattern appears everywhere:

- **TCP connection states** (CLOSED → SYN_SENT → ESTABLISHED → FIN_WAIT → …) — a finite automaton with ~11 states.
- **Regular expressions** in your editor — the regex engine compiles the pattern into a DFA (Lesson 02).
- **Lexical analysis** in compilers — the lexer is a DFA that tokenizes source code character by character (Phase 08).

In each case, the system has a fixed, small number of states and processes a stream of inputs one symbol at a time. No stack, no tape — pure finite-state behavior.

## Read the Source

- **`code/main.py`** — The `VendingMachine` class. Note how `transitions` is a plain dict and `step()` is a one-liner lookup. That is the entire finite automaton execution model.
- Sipser, *Introduction to the Theory of Computation*, Chapter 1 — the standard textbook treatment.

## Ship It

The reusable artifact produced by this lesson is the `VendingMachine` class in `code/main.py`. It demonstrates:

- How to represent any finite automaton as a transition table.
- Why finite memory limits the languages a machine can recognize.
- The connection between real-world state machines and formal computation models.

## Exercises

1. **Easy** — Add a "cancel" input to the vending machine that returns to idle and prints the refunded amount. Verify the machine still has finitely many states.
2. **Medium** — Modify the vending machine to give change (products cost 30¢; machine accepts 5¢, 10¢, 25¢). Track the number of extra states needed. Can you make it handle *any* price without adding states?
3. **Hard** — Prove that no finite automaton can recognize the language L = {w ∈ {0,1}* | w has equal numbers of 0s and 1s}. Hint: use the pigeonhole principle — assume M has k states and consider the input 0ᵏ.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Computation model | "A way to compute things" | A formal tuple (states, alphabet, transition function, start state, accept/reject) that defines what "computing" means for that model |
| Finite automaton | "A state machine" | A 5-tuple (Q, Σ, δ, q₀, F) with finitely many states and no external memory |
| Church-Turing thesis | "All reasonable models are equivalent" | The unprovable but universally-supported claim that Turing machines capture every notion of "effective computation" |
| Deterministic | "One path through the computation" | Every (state, input) pair maps to exactly one next state |
| Hierarchy | "Some models are more powerful" | DFA ⊂ PDA ⊂ TM — each model can recognize strictly more languages than the one below it |

## Further Reading

- Sipser, *Introduction to the Theory of Computation*, 3rd ed., Chapters 1–3.
- Turing, "On Computable Numbers, with an Application to the Entscheidungsproblem" (1936) — the original paper; remarkably readable.
- Church, "An Unsolvable Problem of Elementary Number Theory" (1936).
- Stanford Encyclopedia of Philosophy: [The Church-Turing Thesis](https://plato.stanford.edu/entries/church-turing/).
