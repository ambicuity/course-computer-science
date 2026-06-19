# Finite Automata — DFAs

> A DFA is the simplest model of computation with a formal definition — and it is exactly powerful enough to be the backbone of every lexer, regex engine, and protocol state machine you will ever use.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 05 lesson 01
**Time:** ~60 minutes

## Learning Objectives

- Define a DFA formally as the 5-tuple (Q, Σ, δ, q₀, F) and trace acceptance of a string by repeated application of δ*.
- Implement a general-purpose `DFA` class with `accepts()`, `complement()`, `intersect()`, and `union()` using product construction.
- Build DFAs for at least five languages by hand from English descriptions.
- Recognize where DFAs appear in production systems (lexical analysis, protocol validation).

## The Problem

Lesson 01 introduced computation models and showed that finite automata are the weakest useful model. But "weakest" does not mean "useless" — DFAs are everywhere. The question is: **given a precise definition of a DFA, how do you build one for a specific language, and how do you combine DFAs to recognize the intersection, union, or complement of their languages?**

Without a formal DFA class, you cannot answer these questions programmatically. And without product construction, you are stuck building every DFA from scratch instead of composing simpler ones.

## The Concept

### Formal definition

A **deterministic finite automaton (DFA)** is a 5-tuple M = (Q, Σ, δ, q₀, F) where:

- **Q** — a finite set of states.
- **Σ** — a finite input alphabet.
- **δ** — the transition function: Q × Σ → Q. For every state and every symbol, exactly one next state.
- **q₀** ∈ Q — the start state.
- **F** ⊆ Q — the set of accepting (final) states.

The machine reads an input string w = a₁a₂…aₙ one symbol at a time. Starting at q₀, it applies δ repeatedly:

```
δ*(q₀, ε) = q₀
δ*(q₀, a₁a₂…aₙ) = δ(δ*(q₀, a₁a₂…aₙ₋₁), aₙ)
```

The string **w is accepted** if δ*(q₀, w) ∈ F. The **language of M** is L(M) = {w ∈ Σ* | δ*(q₀, w) ∈ F}.

### Transition diagrams

A DFA can be drawn as a directed graph: nodes are states, edges are labeled with input symbols. Double circles mark accepting states. An arrow from nowhere points to the start state.

**Example: strings over {0,1} ending in "01"**

```
       0        1
→ q₀ ──→ q₁ ──→ q₂ (accept)
  ↑  1   │  1    │ 0,1
  └──────┘  0    ↓
          q₀ ←── q₂
```

More precisely:
- Q = {q₀, q₁, q₂}
- Σ = {0, 1}
- δ(q₀, 0) = q₁,  δ(q₀, 1) = q₀
- δ(q₁, 0) = q₁,  δ(q₁, 1) = q₂
- δ(q₂, 0) = q₁,  δ(q₂, 1) = q₀
- q₀ is start; F = {q₂}

### Transition table

| State | 0 | 1 |
|-------|---|---|
| →q₀ | q₁ | q₀ |
| q₁ | q₁ | q₂* |
| q₂ | q₁ | q₀ |

This is the same machine — easier to encode in code.

### Classic examples

**L₁: strings with an even number of 1s.** Two states: E (even, accept) and O (odd). Reading 1 flips between them; reading 0 does nothing.

| State | 0 | 1 |
|-------|---|---|
| →E* | E | O |
| O | O | E |

**L₂: binary strings that represent multiples of 3.** States are the remainders 0, 1, 2. Reading bit b when at remainder r transitions to remainder (2r + b) mod 3.

| State | 0 | 1 |
|-------|---|---|
| →0* | 0 | 1 |
| 1 | 2 | 0 |
| 2 | 1 | 2 |

### Product construction

Given DFAs M₁ = (Q₁, Σ, δ₁, s₁, F₁) and M₂ = (Q₂, Σ, δ₂, s₂, F₂) over the **same alphabet**:

| Operation | Accepting states of product |
|-----------|----------------------------|
| **Intersection** M₁ ∩ M₂ | F = {(q₁, q₂) ∈ Q₁ × Q₂ \| q₁ ∈ F₁ AND q₂ ∈ F₂} |
| **Union** M₁ ∪ M₂ | F = {(q₁, q₂) ∈ Q₁ × Q₂ \| q₁ ∈ F₁ OR q₂ ∈ F₂} |
| **Complement** M̄₁ | Q same, δ same, F = Q₁ \ F₁ |

The product machine has |Q₁| × |Q₂| states. Its transition is δ((q₁,q₂), a) = (δ₁(q₁,a), δ₂(q₂,a)). This is a **constructive proof** that regular languages are closed under ∩, ∪, and complement.

## Build It

### Step 1: Minimal DFA class

`code/main.py` implements a `DFA` class. Core method `accepts(input_string)` applies δ symbol by symbol and checks membership in F.

```python
dfa = DFA(
    states={"q0", "q1", "q2"},
    alphabet={"0", "1"},
    transitions={
        ("q0", "0"): "q1", ("q0", "1"): "q0",
        ("q1", "0"): "q1", ("q1", "1"): "q2",
        ("q2", "0"): "q1", ("q2", "1"): "q0",
    },
    start="q0",
    accept={"q2"},
)
assert dfa.accepts("101")    # ends in "01"
assert not dfa.accepts("110") # ends in "10"
```

### Step 2: Product construction

`complement()`, `intersect(other)`, and `union(other)` build a new DFA using the product of states.

```python
even_ones = ...  # DFA for even number of 1s
ends_in_01 = ... # DFA for strings ending in "01"
both = even_ones.intersect(ends_in_01)
assert both.accepts("1001")   # even 1s AND ends in 01
assert not both.accepts("101") # odd 1s
```

## Use It

Lexical analysis in compilers is the canonical production use of DFAs. The lexer in GCC, LLVM, and V8 takes regular-expression-like token patterns, compiles each into a DFA, then merges them into one DFA via union. As the scanner reads source characters, it runs the DFA and emits tokens at accepting states.

**Concrete example:** Python's `re` module compiles regular expressions into a DFA (or NFA with simulation — see Lesson 03). The `tokenize` module in CPython uses hand-written DFA-like scanners to classify Python source into `NAME`, `NUMBER`, `STRING`, `OP` tokens.

## Read the Source

- **`code/main.py`** — `DFA` class with full product construction. Note how `accepts()` is a 4-line loop; `complement()` is a one-liner; `intersect()` and `union()` are symmetric.
- CPython `Lib/tokenize.py` — the `tokenize` function is essentially a DFA reading characters one at a time.

## Ship It

The reusable artifact is the `DFA` class in `code/main.py`. You can drop it into any project that needs:

- Pattern matching over finite alphabets.
- Combining simple pattern matchers via intersection/union/complement.
- Verifying that a string satisfies multiple independent regular constraints simultaneously.

## Exercises

1. **Easy** — Build a DFA that accepts binary strings containing the substring "101". Verify it with at least 5 accepting and 5 rejecting test strings.
2. **Medium** — Build a DFA for binary strings representing numbers divisible by 4. Use the product construction to intersect it with "strings with an odd number of 1s." Verify the result.
3. **Hard** — Prove that the class of regular languages is closed under concatenation: given DFAs M₁ and M₂, construct an NFA (or ε-NFA) for L(M₁)·L(M₂) and convert it to a DFA. Implement the construction in Python.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| DFA | "A state machine" | A 5-tuple (Q, Σ, δ, q₀, F) with deterministic transitions — exactly one next state per (state, input) pair |
| δ* (extended transition) | "Run the machine on a string" | The recursive extension of δ to process an entire string: δ*(q, wa) = δ(δ*(q, w), a) |
| L(M) | "The language of the machine" | The set of all strings w such that δ*(q₀, w) ∈ F |
| Product construction | "Combine two DFAs" | Build a new DFA whose states are pairs (q₁, q₂) from the two input DFAs; adjust accepting set for ∩, ∪, or complement |
| Regular language | "A language a DFA can recognize" | Any language L for which there exists a DFA M with L(M) = L — equivalently, any language definable by a regular expression |

## Further Reading

- Sipser, *Introduction to the Theory of Computation*, 3rd ed., Chapter 1.1–1.2.
- Hopcroft, Motwani, Ullman, *Introduction to Automata Theory, Languages, and Computation*, Chapter 2.
- Aho, Lam, Sethi, Ullman, *Compilers: Principles, Techniques, and Tools* (the Dragon Book), Chapter 3 — DFA-based lexical analysis.
