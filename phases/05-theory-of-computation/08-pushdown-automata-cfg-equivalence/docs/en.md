# Pushdown Automata & CFG Equivalence

> Pushdown Automata & CFG Equivalence — proving that grammars and machines capture the same class of languages.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 05 lessons 01–07
**Time:** ~75 minutes

## Learning Objectives

- Define a PDA formally as the 7-tuple (Q, Σ, Γ, δ, q₀, Z₀, F).
- Simulate PDA execution step-by-step on an input string.
- Convert any CFG to an equivalent PDA and vice versa.
- Distinguish deterministic from nondeterministic PDAs and understand why nondeterminism matters.
- Implement a PDA simulator and the CFG ↔ PDA conversions in Python.

## The Problem

Lesson 07 showed that CFGs describe languages beyond the reach of regular expressions — nested
structures, matched parentheses, arithmetic expressions. But a grammar is a *generative* device:
it produces strings by derivation. We need a *recognition* device — a machine that reads an input
string left to right and decides membership. For regular languages, that machine is the DFA/NFA.
For context-free languages, the machine is the **pushdown automaton (PDA)**: a finite automaton
equipped with a stack.

Proving CFG ↔ PDA equivalence is critical: it links the generative and recognition perspectives,
showing that every language describable by a grammar can be recognized by a stack machine and
vice versa.

## The Concept

A **pushdown automaton** is a 7-tuple M = (Q, Σ, Γ, δ, q₀, Z₀, F) where:

- **Q** — finite set of states.
- **Σ** — input alphabet.
- **Γ** — stack alphabet (can be larger than Σ).
- **δ** — transition function.
- **q₀ ∈ Q** — start state.
- **Z₀ ∈ Γ** — initial stack symbol (bottom marker).
- **F ⊆ Q** — set of accepting states.

### Stack Operations

The transition function δ maps (state, input symbol, stack top) to a set of
(state, stack replacement) pairs. The stack operations are:

- **Push**: replace top with a longer string — e.g., push `AA` onto stack.
- **Pop**: replace top with a shorter string — e.g., pop one symbol.
- **Replace**: swap top symbol for another.

Transitions can also read **ε** (no input) or operate on **ε** (without popping).

### Deterministic vs Nondeterministic

A PDA is **deterministic (DPDA)** if for every (state, input, stack top), at most one transition
exists, and no ε-transitions compete with input-consuming transitions. DPDA languages form a
proper subset of CFLs — for instance, { wwᴿ | w ∈ {a,b}* } is deterministic but { ww | w ∈ {a,b}* }
is not even context-free.

**Crucially**: nondeterministic PDAs are strictly more powerful than deterministic PDAs. The
language of palindromes { wwᴿ } is deterministic, but { w | w is a palindrome } over {a,b} is
not deterministic — you must "guess" the midpoint.

### Equivalence: CFG ↔ PDA

**CFG → PDA (construction)**: Given grammar G = (V, Σ, R, S), build PDA M with two states
{q, f}:

1. Start: push S onto stack, go to q.
2. At q, if stack top is variable A, nondeterministically choose a rule A → α and replace A with α.
3. At q, if stack top is terminal a and current input is a, pop a and advance input.
4. Accept when stack is empty (or enter state f when stack is empty).

**PDA → CFG (state-based construction)**: Given PDA M, create variables A_{pq} for each pair
of states (p, q), representing "strings that take M from state p with empty stack to state q
with empty stack." Build rules from M's transitions.

Both constructions are mechanical and preserve the language exactly.

## Build It

### Step 1: PDA Simulator

```python
from __future__ import annotations
from dataclasses import dataclass
from typing import Optional


@dataclass(frozen=True)
class Transition:
    next_state: str
    stack_push: tuple[str, ...]


class PDA:
    def __init__(self):
        self.states: set[str] = set()
        self.input_alphabet: set[str] = set()
        self.stack_alphabet: set[str] = set()
        self.transitions: dict[tuple[str, str, str], list[Transition]] = {}
        self.start_state: str = ""
        self.start_stack: str = "Z"
        self.accept_states: set[str] = set()

    def add_transition(self, state: str, input_sym: str, stack_top: str,
                       next_state: str, stack_push: tuple[str, ...]) -> None:
        self.states.update([state, next_state])
        if input_sym != "":
            self.input_alphabet.add(input_sym)
        self.stack_alphabet.update(stack_push)
        key = (state, input_sym, stack_top)
        if key not in self.transitions:
            self.transitions[key] = []
        self.transitions[key].append(Transition(next_state, stack_push))

    def accepts(self, input_string: str) -> bool:
        configs = [(self.start_state, 0, [self.start_stack])]
        visited = set()
        max_steps = 10000
        steps = 0
        while configs and steps < max_steps:
            state, pos, stack = configs.pop()
            steps += 1
            config_key = (state, pos, tuple(stack))
            if config_key in visited:
                continue
            visited.add(config_key)
            if pos == len(input_string) and state in self.accept_states:
                return True
            stack_top = stack[-1] if stack else ""
            candidates = []
            if pos < len(input_string):
                candidates.extend(
                    self.transitions.get((state, input_string[pos], stack_top), [])
                )
            candidates.extend(
                self.transitions.get((state, "", stack_top), [])
            )
            for tr in candidates:
                new_stack = stack[:-1] if stack else []
                new_stack = list(reversed(tr.stack_push)) + new_stack
                new_pos = pos + (0 if "" == input_string[pos:pos + 1] and
                                 (state, "", stack_top) in self.transitions else 0)
                if (state, input_string[pos] if pos < len(input_string) else "", stack_top) in self.transitions:
                    input_consuming = any(
                        t.next_state == tr.next_state and t.stack_push == tr.stack_push
                        for t in self.transitions.get(
                            (state, input_string[pos] if pos < len(input_string) else "", stack_top), [])
                    )
                    if input_consuming and pos < len(input_string):
                        new_pos = pos + 1
                    elif pos < len(input_string) and tr in self.transitions.get((state, "", stack_top), []):
                        new_pos = pos
                    else:
                        new_pos = pos + (1 if pos < len(input_string) else 0)
                else:
                    new_pos = pos
                configs.append((tr.next_state, new_pos, new_stack))
        return False

    def simulate(self, input_string: str, max_steps: int = 1000) -> list[dict]:
        trace = []
        configs = [(self.start_state, 0, [self.start_stack])]
        visited = set()
        steps = 0
        while configs and steps < max_steps:
            state, pos, stack = configs.pop()
            steps += 1
            config_key = (state, pos, tuple(stack))
            if config_key in visited:
                continue
            visited.add(config_key)
            trace.append({
                "step": steps,
                "state": state,
                "position": pos,
                "remaining_input": input_string[pos:],
                "stack": list(reversed(stack)),
            })
            if pos == len(input_string) and state in self.accept_states:
                trace[-1]["result"] = "ACCEPT"
                return trace
            stack_top = stack[-1] if stack else ""
            candidates = []
            if pos < len(input_string):
                candidates.extend(
                    self.transitions.get((state, input_string[pos], stack_top), [])
                )
            candidates.extend(
                self.transitions.get((state, "", stack_top), [])
            )
            for tr in candidates:
                new_stack = stack[:-1] if stack else []
                new_stack = list(reversed(tr.stack_push)) + new_stack
                is_eps = (state, "", stack_top) in self.transitions and tr in self.transitions[(state, "", stack_top)]
                new_pos = pos if is_eps else pos + (1 if pos < len(input_string) else 0)
                configs.append((tr.next_state, new_pos, new_stack))
        trace.append({"result": "REJECT" if steps >= max_steps else "REJECT"})
        return trace
```

### Step 2: CFG ↔ PDA Conversion

```python
def cfg_to_pda(grammar) -> PDA:
    pda = PDA()
    pda.start_state = "q0"
    pda.accept_states = {"qf"}
    pda.start_stack = grammar.start
    pda.states = {"q0", "qf"}
    pda.add_transition("q0", "", "Z", "q0", (grammar.start, "Z"))
    for head, bodies in grammar.rules.items():
        for body in bodies:
            stack_push = tuple(sym for sym in body if sym != "") or ("",)
            if stack_push == ("",):
                stack_push = ()
            pda.add_transition("q0", "", head, "q0", stack_push)
    for term in grammar.terminals:
        pda.add_transition("q0", term, term, "q0", ())
    pda.add_transition("q0", "", "Z", "qf", ("Z",))
    return pda


def pda_to_cfg(pda: PDA) -> object:
    from collections import defaultdict
    variables = set()
    rules: dict[str, list[list[str]]] = defaultdict(list)
    for p in pda.states:
        for q in pda.states:
            variables.add(f"A_{p}_{q}")
    start_var = f"A_{pda.start_state}_{''.join(pda.accept_states)}"
    if not pda.accept_states:
        start_var = list(variables)[0]
    for (state, inp, stack_top), trans_list in pda.transitions.items():
        for tr in trans_list:
            r = tr.next_state
            if len(tr.stack_push) == 0:
                for q in pda.states:
                    rules[f"A_{state}_{q}"].append([inp, f"A_{r}_{q}"] if inp else [f"A_{r}_{q}"])
            elif len(tr.stack_push) == 1:
                B = tr.stack_push[0]
                for q in pda.states:
                    for s in pda.states:
                        rules[f"A_{state}_{q}"].append(
                            ([inp] if inp else []) + [f"A_{r}_{s}", f"A_{s}_{q}"]
                        )
    class _Grammar:
        pass
    g = _Grammar()
    g.start = start_var
    g.rules = dict(rules)
    g.terminals = pda.input_alphabet
    return g
```

## Use It

Real parsers are **deterministic PDAs**. The LR(1) parser used by Yacc/Bison is a deterministic
PDA where the stack holds grammar symbols and parser states. Each step:

1. Read the next token (input symbol).
2. Look at the current state and stack top.
3. **Shift**: push the token onto the stack (move to a new state).
4. **Reduce**: pop a rule's right-hand side from the stack, push the left-hand side.

This is exactly PDA operation but guaranteed to have exactly one valid action per step —
deterministic. The stack naturally handles the nested, recursive structure of expressions like
`((1 + 2) * (3 + 4))`.

**Read the Source**: GNU Bison's `lalr1.cc` — the LALR(1) parser table that drives C, C++, and
many other language parsers. The "goto table" is the PDA's transition function.

## Ship It

The reusable artifact produced by this lesson is a **PDA simulator** with CFG ↔ PDA conversion.
You will reuse it in Lesson 10 to understand how CYK parsing relates to PDA recognition.

## Exercises

1. **Easy** — Build a PDA for { aⁿbⁿ | n ≥ 0 }. Trace its execution on `"aabb"` using the
   `simulate` method and verify it accepts.
2. **Medium** — Convert your Lesson 07 arithmetic-expression grammar to a PDA using `cfg_to_pda`.
   Verify the PDA accepts `"1+2*3"` and rejects `"1++2"`.
3. **Hard** — Implement the full PDA-to-CFG conversion (state-pair variable construction).
   Verify the resulting CFG generates the same language as the original PDA recognizes on
   a set of test strings.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Pushdown automaton | "A stack machine" | A finite automaton with an unbounded stack for memory |
| Stack alphabet | "The symbols you can push" | Γ, the set of symbols that can appear on the stack |
| Configuration | "Current state of the machine" | A triple (state, remaining input, stack contents) |
| ε-transition | "A silent move" | A transition that consumes no input symbol |
| Deterministic PDA | "No guessing needed" | At most one valid transition per configuration |
| CFL | "A context-free language" | A language recognized by some PDA (equivalently, generated by some CFG) |

## Further Reading

- Hopcroft, Motwani, Ullman — *Introduction to Automata Theory, Languages, and Computation*, Ch. 6
- Sipser — *Introduction to the Theory of Computation*, Ch. 2.2
- Bison internals: `https://www.gnu.org/software/bison/manual/bison.html`
