# Lesson 03: NFAs and Subset Construction

## What You'll Learn

- How a Non-deterministic Finite Automaton (NFA) generalizes the DFA
- Why non-determinism makes automata easier to design
- The subset construction algorithm: converting any NFA to an equivalent DFA
- When exponential blowup occurs and why it rarely matters in practice

---

## 1. From DFA to NFA: Relaxing the Rules

A **DFA** has exactly one next state for every (state, symbol) pair. That rigid
determinism is powerful — it guarantees fast, unambiguous computation — but it
also makes DFA design tedious. Every branch must be spelled out.

A **Non-deterministic Finite Automaton (NFA)** relaxes two constraints:

| Rule | DFA | NFA |
|------|-----|-----|
| Transition function | δ(q, a) → q' (single state) | δ(q, a) → {q₁, q₂, …} (set of states) |
| Missing transitions | Not allowed (complete) | Allowed — dead path means reject |
| ε-transitions | Not allowed | Allowed — move without consuming input |

**Formal definition.** An NFA is a 5-tuple (Q, Σ, δ, q₀, F) where δ : Q ×
(Σ ∪ {ε}) → P(Q) returns a *set* of states. The NFA accepts a string if
**at least one** computational path ends in an accepting state.

---

## 2. ε-Transitions

An ε-transition lets the automaton jump between states without reading any input.
This is incredibly useful for composing patterns:

```
         a
    ┌─────────┐
    ▼         │
──→ (q₀) ──ε─→ (q₁) ──b─→ ((q₂))
```

Here `q₀` can consume `a` and stay, or take the ε-edge to `q₁`. The language
accepted is {ab}.

**ε-closure(q)** is the set of all states reachable from q using zero or more
ε-transitions (including q itself). When simulating an NFA, we expand the
current set to include every ε-reachable state before reading the next symbol.

---

## 3. Why NFAs Are Easier to Design

Consider the language "strings over {0,1} that contain a 0 in the second-to-last
position." A DFA requires at least 4 states. An NFA does it in 3:

```
        0,1        0        0,1
──→ (q₀) ────→ (q₁) ──→ ((q₂)) ──→ ((q₂))
```

The NFA *guesses* when it has seen the second-to-last symbol. If the guess is
wrong, that path dies — but some other path might succeed. Converting a regex
to an NFA is straightforward; converting directly to a DFA requires careful
bookkeeping.

---

## 4. Subset Construction: NFA → DFA

The **subset construction** (also called the *powerset construction*) converts
any NFA into an equivalent DFA. The idea:

> Each state in the resulting DFA represents a **set** of NFA states.

### Algorithm

```
function SUBSET-CONSTRUCT(nfa):
    dfa_start = ε-closure({nfa.start})
    worklist = [dfa_start]
    dfa_states = {dfa_start}
    dfa_transitions = {}

    while worklist is not empty:
        T = worklist.pop()
        for each symbol a in Σ:
            U = ε-closure( move(T, a) )
            if U ∉ dfa_states:
                dfa_states.add(U)
                worklist.push(U)
            dfa_transitions[(T, a)] = U

    dfa_accepting = { T ∈ dfa_states | T ∩ nfa.F ≠ ∅ }
    return DFA(dfa_states, Σ, dfa_transitions, dfa_start, dfa_accepting)
```

**move(T, a)** — union of δ(q, a) for every q ∈ T.
**ε-closure(T)** — union of ε-closure(q) for every q ∈ T.

The resulting DFA has at most 2ⁿ states (where n is the number of NFA states).
This is the **exponential blowup**.

---

## 5. Exponential Blowup: Worst Case

There exist NFAs where every subset of states is reachable, requiring 2ⁿ DFA
states. In practice, blowup is rare — most subsets are unreachable because
transitions are often deterministic and real-world patterns have limited
ambiguity. Python's `re` module uses backtracking to avoid worst-case blowup.

---

## 6. Build It: NFA Class + Subset Construction

See `code/main.py` for:

- `NFA` class with `accepts(input_string)` — simulate all paths simultaneously
- `NFA.to_dfa()` — subset construction producing an equivalent `DFA`
- Comparison of NFA vs DFA for the same language
- Demonstration of exponential blowup in the worst case

---

## 7. Use It

- **Regex engines**: Convert patterns to NFAs, simulate with Thompson's
  algorithm or subset-construct a DFA.
- **Lexical analysis**: `flex`/`lex` build DFAs from regex via subset construction.
- **Protocol verification**: Model-checking tools explore NFA-like state spaces.

## 8. Ship It: NFA-to-DFA Converter

Build a program that:

1. Reads an NFA specification (states, alphabet, transitions, start, accepting)
2. Runs subset construction
3. Prints the equivalent DFA in a readable table
4. Verifies equivalence by testing sample strings on both automata

## 9. Exercises

### Level 1 — Conceptual

1. Construct an NFA with exactly 2 states that accepts all strings over {a, b}
   ending in `a`.
2. For your NFA, trace the computation of the string `baa`. List every
   configuration (set of active states) at each step.
3. What is ε-closure(q) for each state in your NFA?

### Level 2 — Algorithm

4. Apply subset construction to your NFA from Exercise 1. Draw the resulting
   DFA's transition diagram.
5. How many states does the DFA have? Compare this to the number of states in
   the smallest DFA for the same language.
6. Construct an NFA with 3 states over {a} that has a DFA equivalent requiring
   8 states. (Hint: use ε-transitions to create every possible subset.)

### Level 3 — Implementation

7. Modify `NFA.accepts()` to return **all** accepting paths (not just True/False).
   Each path is a list of (state, symbol) pairs.
8. Implement a `minimize_dfa(dfa)` function using Hopcroft's algorithm.
9. Build an NFA from a regex using Thompson construction (see Lesson 04) and
   verify that `nfa.to_dfa()` produces a DFA with the same language.

---

## Summary

NFAs generalize DFAs by allowing multiple next states and ε-transitions. They
are easier to design and naturally arise from regular expressions. The subset
construction proves NFAs and DFAs recognize exactly the same class of languages
(regular languages) — though the DFA may be exponentially larger.
