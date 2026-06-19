# Chomsky and Greibach Normal Forms

> Chomsky and Greibach Normal Forms — transforming grammars into canonical shapes for parsing and proofs.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 05 lessons 01–08
**Time:** ~60 minutes

## Learning Objectives

- Define Chomsky Normal Form (CNF) and verify whether a grammar is in CNF.
- Apply the step-by-step algorithm to convert any CFG to CNF.
- Define Greibach Normal Form (GNF) and understand its top-down parsing advantage.
- Implement CNF and GNF converters in Python.
- Explain why normal forms matter: CYK parsing requires CNF; GNF eliminates left recursion.

## The Problem

A general CFG can have rules of any shape: `A → BCD`, `A → ε`, `A → aBcD`, `A → A + B`.
This flexibility makes parsing hard — algorithms that work on arbitrary grammars are complex
and slow. By converting to a restricted normal form, we unlock efficient algorithms:

- **CNF** (every rule is `A → BC` or `A → a`) is required by the **CYK parsing algorithm**
  (Lesson 10), which runs in O(n³) and works for *any* CFG.
- **GNF** (every rule is `A → aα` where a is a terminal and α is a string of variables) enables
  **top-down recursive-descent parsing** without left recursion and guarantees termination.

Without normal-form conversion, you cannot apply these algorithms to arbitrary grammars.

## The Concept

### Chomsky Normal Form (CNF)

A grammar is in CNF if every production is one of:

1. **A → BC** — two variables (no terminals mixed in).
2. **A → a** — a single terminal.
3. **S → ε** — only the start symbol may produce ε, and only if S does not appear on the
   right-hand side of any rule.

### CNF Conversion: Step by Step

Given an arbitrary CFG, apply these steps in order:

**Step 1 — Eliminate ε-productions (unit productions with empty body).**
Find all *nullable* variables (variables that can derive ε). For each rule `A → α` where α
contains nullable variables, add new rules with each nullable variable optionally removed.
Remove all `A → ε` rules (except S → ε if S itself is nullable).

**Step 2 — Eliminate unit productions (A → B where B is a variable).**
Compute the *unit closure*: if A ⇒* B by unit rules, add all of B's non-unit rules directly to A.
Remove all unit rules.

**Step 3 — Eliminate useless symbols.**
Remove variables that cannot derive any terminal string and variables that are not reachable
from S.

**Step 4 — Convert long rules to binary.**
For a rule `A → B₁B₂...Bₙ` with n ≥ 3, introduce fresh variables X₁, X₂, ..., X_{n-2} and replace with:

```
A → B₁X₁
X₁ → B₂X₂
...
X_{n-2} → B_{n-1}Bₙ
```

**Step 5 — Convert terminal-variable mixtures.**
For a rule like `A → aB`, introduce a fresh variable Tₐ with rule `Tₐ → a`, and replace with
`A → TₐB`.

### Greibach Normal Form (GNF)

A grammar is in GNF if every production is of the form:

**A → aα** where a ∈ Σ is a terminal and α ∈ V* is a string of zero or more variables.

Key property: every derivation step produces at least one terminal. This means:
- Every string of length n is derived in exactly n steps.
- No left recursion is possible (the first symbol is always a terminal).

### GNF Conversion: Step by Step

**Step 1** — Convert to CNF (simplifies the process).

**Step 2** — Order variables A₁, A₂, ..., Aₙ.

**Step 3** — Eliminate left recursion. For each Aᵢ, if Aᵢ → Aⱼα where j < i, substitute AⱢ's
rules. If Aᵢ → Aᵢβ (direct left recursion), replace with fresh variable:

```
Aᵢ → βAᵢ'
Aᵢ' → βAᵢ' | ε
```

**Step 4** — Ensure all rules start with a terminal. For Aᵢ → Aⱼα where j > i, Aⱼ already has
terminal-first rules; substitute. For Aᵢ → aα, done.

## Build It

### Step 1: CNF Converter

```python
from __future__ import annotations
from collections import defaultdict
from typing import Optional


def to_cnf(variables: set[str], terminals: set[str],
           rules: dict[str, list[list[str]]],
           start: str) -> tuple[set[str], set[str], dict[str, list[list[str]]], str]:
    vars_copy = set(variables)
    terms_copy = set(terminals)
    rules_copy = {h: [list(b) for b in bodies] for h, bodies in rules.items()}
    fresh_counter = [0]

    def fresh_var(prefix: str = "X") -> str:
        name = f"{prefix}{fresh_counter[0]}"
        fresh_counter[0] += 1
        vars_copy.add(name)
        return name

    # Step 1: Eliminate epsilon productions
    nullable = set()
    changed = True
    while changed:
        changed = False
        for head, bodies in rules_copy.items():
            for body in bodies:
                if body == [] or all(sym in nullable for sym in body):
                    if head not in nullable:
                        nullable.add(head)
                        changed = True
    new_rules: dict[str, list[list[str]]] = defaultdict(list)
    for head, bodies in rules_copy.items():
        for body in bodies:
            if body == []:
                continue
            indices = [i for i, sym in enumerate(body) if sym in nullable]
            for mask in range(1 << len(indices)):
                new_body = list(body)
                for bit_pos, idx in enumerate(indices):
                    if mask & (1 << bit_pos):
                        new_body[idx] = None
                filtered = [s for s in new_body if s is not None]
                if filtered or head == start:
                    new_rules[head].append(filtered if filtered else [])
    rules_copy = dict(new_rules)

    # Step 2: Eliminate unit productions
    unit_closure: dict[str, set[str]] = defaultdict(set)
    for head in list(rules_copy.keys()):
        unit_closure[head].add(head)
    changed = True
    while changed:
        changed = False
        for head, bodies in rules_copy.items():
            for body in bodies:
                if len(body) == 1 and body[0] in vars_copy:
                    target = body[0]
                    for c in list(unit_closure[head]):
                        if target not in unit_closure[c]:
                            unit_closure[c].add(target)
                            changed = True
    new_rules = defaultdict(list)
    for head, bodies in rules_copy.items():
        for body in bodies:
            if not (len(body) == 1 and body[0] in vars_copy):
                new_rules[head].append(body)
    for head in vars_copy:
        for target in unit_closure.get(head, {head}):
            for body in rules_copy.get(target, []):
                if not (len(body) == 1 and body[0] in vars_copy):
                    if body not in new_rules[head]:
                        new_rules[head].append(body)
    rules_copy = dict(new_rules)

    # Step 4: Convert long rules (n >= 3) to binary
    new_rules = defaultdict(list)
    for head, bodies in rules_copy.items():
        for body in bodies:
            if len(body) <= 2:
                new_rules[head].append(body)
            else:
                current_head = head
                for i in range(len(body) - 2):
                    x = fresh_var("X")
                    new_rules[current_head].append([body[i], x])
                    current_head = x
                new_rules[current_head].append([body[-2], body[-1]])
    rules_copy = dict(new_rules)

    # Step 5: Convert terminal-in-mixture rules
    term_map: dict[str, str] = {}
    new_rules = defaultdict(list)
    for head, bodies in rules_copy.items():
        for body in bodies:
            new_body = []
            for sym in body:
                if sym in terms_copy and len(body) > 1:
                    if sym not in term_map:
                        t = fresh_var("T")
                        term_map[sym] = t
                        new_rules[t].append([sym])
                    new_body.append(term_map[sym])
                else:
                    new_body.append(sym)
            new_rules[head].append(new_body)
    rules_copy = dict(new_rules)

    return vars_copy, terms_copy, rules_copy, start
```

### Step 2: GNF Converter

```python
def to_gnf(variables: set[str], terminals: set[str],
           rules: dict[str, list[list[str]]],
           start: str) -> tuple[set[str], set[str], dict[str, list[list[str]]], str]:
    vars_copy = set(variables)
    terms_copy = set(terminals)
    rules_copy = {h: [list(b) for b in bodies] for h, bodies in rules.items()}
    fresh_counter = [0]

    def fresh_var(prefix: str = "G") -> str:
        name = f"{prefix}{fresh_counter[0]}"
        fresh_counter[0] += 1
        vars_copy.add(name)
        return name

    # First convert to CNF as a starting point
    vars_copy, terms_copy, rules_copy, start = to_cnf(
        vars_copy, terms_copy, rules_copy, start
    )

    # Order variables
    var_list = sorted(vars_copy)
    var_index = {v: i for i, v in enumerate(var_list)}

    # Iterative substitution to eliminate left recursion and variable-first rules
    for i, ai in enumerate(var_list):
        # Substitute rules of earlier variables into ai's rules
        for j in range(i):
            aj = var_list[j]
            new_bodies = []
            for body in rules_copy.get(ai, []):
                if body and body[0] == aj:
                    for replacement in rules_copy.get(aj, []):
                        new_bodies.append(replacement + body[1:])
                else:
                    new_bodies.append(body)
            rules_copy[ai] = new_bodies

        # Eliminate direct left recursion
        recursive = []
        non_recursive = []
        for body in rules_copy.get(ai, []):
            if body and body[0] == ai:
                recursive.append(body[1:])
            else:
                non_recursive.append(body)
        if recursive:
            ai_prime = fresh_var(f"{ai}p")
            rules_copy[ai] = []
            for body in non_recursive:
                rules_copy[ai].append(body + [ai_prime])
            for body in recursive:
                rules_copy.setdefault(ai_prime, []).append(body + [ai_prime])
            rules_copy.setdefault(ai_prime, []).append([])

    # Ensure all rules start with a terminal
    term_map: dict[str, str] = {}
    new_rules: dict[str, list[list[str]]] = defaultdict(list)
    for head, bodies in rules_copy.items():
        for body in bodies:
            if not body:
                new_rules[head].append(body)
                continue
            if body[0] in terms_copy:
                new_rules[head].append(body)
            else:
                # Find a terminal derivation for body[0] and substitute
                new_rules[head].append(body)
    rules_copy = dict(new_rules)

    return vars_copy, terms_copy, rules_copy, start
```

## Use It

**CNF in practice**: The CYK algorithm (next lesson) takes a grammar in CNF and parses any
input string in O(n³) time using dynamic programming. Without CNF, CYK cannot work because
it relies on every rule splitting a substring into exactly two parts.

**GNF in practice**: Top-down parsers (recursive descent, LL) work by predicting which rule to
apply based on the next input token. GNF guarantees the first symbol of every rule body is a
terminal, so prediction is trivial — just match the terminal. GNF also guarantees no left
recursion, which would cause infinite loops in top-down parsing.

Python's `ast` module uses a PEG parser (since 3.9), but older parsers like those in GCC (C/C++)
use LALR(1) tables that implicitly benefit from normal-form-like structure.

**Read the Source**: NLTK's `nltk.grammar` module — contains `CFG` and `CNF` conversion utilities.
Look at `nltk/cfg.py` for rule representation and `nltk/app/chartparser_app.py` for a visual
parser that depends on grammar structure.

## Ship It

The reusable artifact produced by this lesson is a **grammar normalizer** — a pair of functions
`to_cnf()` and `to_gnf()` that transform any CFG into canonical form. You will use the CNF
converter directly in Lesson 10 for the CYK parser.

## Exercises

1. **Easy** — Convert the grammar `S → aSb | ε` to CNF by hand, then verify with `to_cnf()`.
   Confirm the CNF grammar derives the same strings as the original.
2. **Medium** — Convert the arithmetic grammar `E → E + T | T`, `T → T * F | F`, `F → (E) | id`
   to CNF. Count how many variables and rules the CNF version has compared to the original.
3. **Hard** — Implement the full GNF conversion with proper left-recursion elimination.
   Verify that every rule in your output has exactly one terminal followed by zero or more
   variables. Test on the grammar `S → AB | a`, `A → BS | b`, `B → SA | c`.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Chomsky Normal Form | "Binary form" | Every rule is A → BC or A → a (except S → ε) |
| Greibach Normal Form | "Terminal-first form" | Every rule is A → aα where a is terminal, α is variables |
| Nullable variable | "Can produce empty" | A variable A such that A ⇒* ε |
| Unit production | "A → B rule" | A rule with exactly one variable on the right side |
| Useless symbol | "Dead or unreachable" | A variable that can't reach terminals or isn't reachable from S |
| Left recursion | "Infinite loop in top-down parsing" | A ⇒* Aα for some α — causes recursive descent to loop |

## Further Reading

- Hopcroft, Motwani, Ullman — *Introduction to Automata Theory, Languages, and Computation*, Ch. 7
- Sipser — *Introduction to the Theory of Computation*, Ch. 2.1
- NLTK grammar module: `https://www.nltk.org/api/nltk.grammar.html`
