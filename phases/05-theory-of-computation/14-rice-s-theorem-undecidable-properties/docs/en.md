# Rice's Theorem & Undecidable Properties

> Rice's Theorem — every interesting question about what a program does is undecidable.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 05 lessons 01–13
**Time:** ~60 minutes

## Learning Objectives

- State Rice's theorem precisely.
- Distinguish semantic properties (about the language) from syntactic properties (about the machine).
- Apply Rice's theorem to classify properties as decidable or undecidable.
- Understand the proof sketch by reduction from the halting problem.

## The Problem

This lesson sits in **Phase 05 — Theory of Computation**. Without the concept it teaches, you cannot
build the phase's capstone (A regex engine plus a Turing-machine simulator.). Concretely, *not* knowing this means you get stuck the
moment you try to ask "what can we prove about programs?" — Rice's theorem tells you the answer is "almost nothing, in general."

Lesson 13 showed that "does this TM halt?" is undecidable. Rice's theorem generalizes to **any** non-trivial property of the language a TM recognizes.

## The Concept

### Semantic vs Syntactic Properties

A property *P* of TMs is **semantic** if it depends only on the language recognized, not on the machine itself.

```
Semantic:    "Does M accept at least one string?"  (depends on L(M))
Syntactic:   "Does M have fewer than 10 states?"   (depends on M's description)
```

### Rice's Theorem (1953)

**Theorem:** Let *P* be any non-trivial semantic property of Turing-recognizable languages. Then `L_P = { ⟨M⟩ | L(M) has property P }` is undecidable. **Non-trivial** means *P* is true for some TM languages and false for others.

### Classification Table

| Property | Semantic? | Non-trivial? | Decidable? |
|---|---|---|---|
| L(M) = ∅ | yes | yes | undecidable (EMPTY) |
| L(M) = Σ* | yes | yes | undecidable (ALL) |
| L(M) is finite | yes | yes | undecidable (FINITE) |
| L(M) contains ε | yes | yes | undecidable |
| L(M) is regular | yes | yes | undecidable |
| L(M) is context-free | yes | yes | undecidable |
| M has ≤ 100 states | no | — | decidable (just count) |
| M's first transition is to state 3 | no | — | decidable (just read) |
| M always halts | yes* | yes | undecidable (HALT reformulation) |

### Proof Sketch (Reduction from HALT)

1. Let *P* be non-trivial semantic. WLOG assume ∅ does *not* have *P*.
2. There exists TM *M_yes* whose language has *P*.
3. Given ⟨M, w⟩, construct ⟨M'⟩: M' simulates M on w first. If M halts, M' runs M_yes (L(M') has P). If M loops, M' never accepts (L(M') = ∅, no P).
4. A decider for *L_P* would decide HALT — contradiction. ∎

### Implications for Program Analysis

- "Does this program ever crash?" — undecidable
- "Does this function always return an integer?" — undecidable
- "Is this code dead?" — undecidable in general

Practical static analyzers are **conservative**: they may produce false positives and accept false negatives.

### Connection to the Halting Problem

Rice's theorem subsumes the halting problem. HALT asks "does M halt on w?" — a property about behavior, not structure. Reformulated, HALT asks: "does L(M') contain w?" where M' ignores its input and simulates M on w. This is a non-trivial semantic property, so Rice's theorem applies directly.

In fact, Rice's theorem is proved by reducing HALT to any non-trivial semantic property *P*, so HALT is the foundational undecidable problem from which all Rice-theorem undecidability flows.

## Build It

### Rice's Theorem Demonstrator

`rice_theorem_examples()` in `code/main.py` classifies 13 properties as semantic/syntactic, trivial/non-trivial, and decidable/undecidable. It prints a formatted table showing that every non-trivial semantic property is undecidable.

### Reduction from HALT

`reduce_to_halting(property_decider)` in `code/main.py` demonstrates the proof: given a hypothetical decider for any non-trivial semantic property, it constructs a function that solves HALT — showing the decider cannot exist.

```python
# Given (M, w), build M':
#   M'(x): simulate M on w; if halts, accept x (L(M') = Σ*)
#           if loops, never accept (L(M') = ∅)
# property_decider(M') answers HALT(M, w)
```

## Use It

Every time someone asks "can we build a tool that checks if X is true about any program?", Rice's theorem is the first filter. If X is non-trivial and semantic, the answer is **no — not exactly**.

- **ESLint / static checkers**: Decide syntactic and trivial properties only.
- **TypeScript's type checker**: Decides syntax properties; cannot guarantee runtime behavior across all inputs.
- **Rust borrow checker**: Conservative approximation of "is there a use-after-free?" — rejects some safe programs to avoid accepting unsafe ones.

Every such tool must be incomplete, unsound, or both.

## Ship It

The `rice_theorem_examples` catalogue and `reduce_to_halting` template are your go-to references. When you encounter a proposed program analysis tool, ask: is the property semantic? Non-trivial? If both yes, the tool cannot be both sound and complete. Keep the quick-reference table from `print_undecidability_summary()` handy.

## Exercises

### Level 1 — Classify

State whether each is semantic or syntactic, and whether Rice's theorem applies:
1. "M has an even number of states"
2. "L(M) contains only palindromes"
3. "M uses tape alphabet {0, 1, B}"

### Level 2 — Implement

Write `make_reduced_machine_code(tm_desc, w, property_type)` that generates Python source for the reduced machine M' used in the proof sketch.

### Level 3 — Prove

Prove: if *P* is non-trivial semantic, then { ⟨M⟩ | L(M) does NOT have *P* } is also undecidable. (Hint: complement of a decidable language is decidable.)
