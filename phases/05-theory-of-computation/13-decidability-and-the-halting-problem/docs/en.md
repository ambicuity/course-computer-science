# Decidability and the Halting Problem

> Decidability and the Halting Problem — the boundary between what computers can and cannot solve.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 05 lessons 01–12
**Time:** ~60 minutes

## Learning Objectives

- Distinguish decidable, semi-decidable (recognizable), and undecidable languages.
- Understand the halting problem and why it is undecidable.
- Reproduce the diagonalization argument in code.
- Recognize why certain static analysis goals are provably impossible.

## The Problem

This lesson sits in **Phase 05 — Theory of Computation**. Without the concept it teaches, you cannot
build the phase's capstone (A regex engine plus a Turing-machine simulator.). Concretely, *not* knowing this means you get stuck the
moment you try to cross from decidable questions to undecidability — know what computers can't do and why.

Imagine you write a tool that promises: "Feed me any program and any input, and I'll tell you whether that program loops forever." If such a tool existed, every bug caused by infinite loops would be trivially caught. **It cannot exist.** This lesson proves why.

## The Concept

A language *L* is **decidable** (recursive) if some Turing machine *M* halts on every input and accepts exactly the strings in *L*. A language is **semi-decidable** (Turing-recognizable) if some TM accepts every string in *L* but may loop forever on strings not in *L*.

| Category | Accepts in L | Rejects not in L | Halts always |
|---|---|---|---|
| Decidable | yes | yes | yes |
| Semi-decidable | yes | may loop | no |
| Not even recognizable | — | — | no |

### The Halting Problem

```
HALT = { ⟨M, w⟩ | M is a TM that halts on input w }
```

**Theorem (Turing, 1936):** HALT is undecidable.

**Proof by diagonalization (contradiction):**

1. Suppose `H` decides HALT: takes `⟨M, w⟩`, returns True if `M` halts on `w`.
2. Build `D(⟨M⟩)` that runs `H(⟨M, ⟨M⟩⟩)`. If H says halts → D loops. If H says loops → D halts.
3. Run `D(⟨D⟩)`. Does D halt on its own encoding?
   - If D halts → H said yes → D loops. Contradiction.
   - If D loops → H said no → D halts. Contradiction.
4. Therefore H cannot exist. HALT is undecidable. ∎

### Consequences

- **No perfect debugger:** No tool detects all infinite loops.
- **No universal optimizer:** No compiler pass always preserves behavior.
- **No total-program checker:** Termination cannot be verified in general.

### Semi-Decidability of HALT

HALT is **semi-decidable** — the machine `S` below recognizes it:

```
S(⟨M, w⟩):
    simulate M on w
    if M halts, accept
```

This accepts every halting pair but loops forever on non-halting pairs — and you cannot fix that.

## Build It

### Diagonalization Simulator

`diagonalization_demo()` in `code/main.py` simulates Cantor's argument on finite boolean functions: take a list of functions on `{0..n-1}`, construct `g(i) = 1 - f_i(i)`, and show `g` is not in the list.

```python
n = 8
all_functions = list(product([0, 1], repeat=n))
listed = all_functions[:10]
diagonal = tuple(1 - listed[i][i] for i in range(n))
```

### Halting Undecidability Proof

`halting_undecidable_proof()` in `code/main.py` walks through the proof step-by-step:
1. Assume H decides HALT.
2. Build D that uses H's answer to do the opposite.
3. Feed D to itself → contradiction.
4. H cannot exist. ∎

### Semi-Decider

`semi_decider(program_code, inp, timeout)` in `code/main.py` attempts to run a program and detect halting within a timeout. Returns True (halted), or None (unknown). It can never return False — confirming the one-sided nature of semi-decidability.

```python
result = semi_decider("x = 0\nfor i in range(10): x += 1", "")   # True
result = semi_decider("while True: pass", "", timeout=0.5)        # None
```

### Decidable vs Undecidable Problems

| Decidable | Undecidable |
|---|---|
| String accepted by DFA? | HALT: Does M halt on w? |
| Two DFAs equivalent? | EMPTY_TM: Does M accept nothing? |
| CFG language empty? | ALL_TM: Does M accept all strings? |
| String in CFL (CYK)? | EQ_TM: Do M1, M2 accept same language? |

## Use It

Static analysis tools (ESLint, Coverity, Rust borrow checker) catch *specific* bug classes using decidable sub-problems. No tool can answer "does this program have any bug?" in general — that reduces to HALT.

Compilers face the same wall. Proving an optimization preserves all behaviors is undecidable. Compiler writers use conservative approximations: if they cannot prove something is safe, they skip the optimization.

## Ship It

The diagonalization simulator and semi-decider become part of your theory toolbox. Key takeaway: **semi-decidable languages have a one-sided algorithm** — you can confirm "yes" but never guarantee "no".

## Exercises

### Level 1 — Identify

Classify each as decidable, semi-decidable, or neither:
1. `{ ⟨M⟩ | M accepts the empty string }`
2. `{ ⟨M, w⟩ | M halts on w within 100 steps }`
3. `{ ⟨M⟩ | M accepts at least one string }`

### Level 2 — Implement

Modify `semi_decider` to track computation steps instead of wall-clock time. Add a `step_budget` parameter and simulate a toy two-state TM tape.

### Level 3 — Prove

Prove that if a language *L* and its complement *L̄* are both semi-decidable, then *L* is decidable. (Hint: run both recognizers in parallel.)
