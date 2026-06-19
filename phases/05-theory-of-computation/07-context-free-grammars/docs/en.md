# Context-Free Grammars

> Context-Free Grammars — the formal notation that underpins every programming language parser.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 05 lessons 01–06
**Time:** ~60 minutes

## Learning Objectives

- Define a CFG formally as the 4-tuple (V, Σ, R, S) and distinguish terminals from nonterminals.
- Produce leftmost and rightmost derivations and construct parse trees.
- Recognize ambiguous grammars and explain why ambiguity matters for language design.
- Implement a CFG class with derivation and parse-tree generation in Python.
- Compare context-free languages to regular languages and understand the strict hierarchy.

## The Problem

Regular languages and finite automata handle patterns with bounded memory — you can check whether
a string matches `[a-z]+@[a-z]+\.com` but you cannot match nested parentheses `((()))` because
that requires counting. The pumping lemma from Lesson 06 proved this limitation rigorously. To
describe the nested, recursive structure found in real programming languages — balanced brackets,
nested `if` statements, arithmetic expressions with parentheses — we need a more powerful formalism.
That formalism is the **context-free grammar (CFG)**.

## The Concept

A **context-free grammar** is a 4-tuple G = (V, Σ, R, S) where:

- **V** — a finite set of *variables* (nonterminals), written as uppercase letters: `S, E, T, F`.
- **Σ** — a finite set of *terminals* (the alphabet of the language), disjoint from V: `a, b, (, ), +, *`.
- **R** — a finite set of *production rules*, each of the form `A → α` where A ∈ V and α ∈ (V ∪ Σ)*.
- **S ∈ V** — the *start symbol*.

A grammar is *context-free* because the left side of every rule is a single variable — the
replacement does not depend on surrounding context.

### Derivations

A **derivation** is a sequence of rule applications starting from S that produces a terminal string.

- **Leftmost derivation**: always expand the leftmost variable first.
- **Rightmost derivation**: always expand the rightmost variable first.

Example grammar for balanced parentheses:

```
S → (S)S | ε
```

Derivation of `()()`:

```
S ⇒ (S)S ⇒ ()S ⇒ ()(S)S ⇒ ()()S ⇒ ()()
```

### Parse Trees

A **parse tree** is a rooted tree where the root is labeled S, interior nodes are variables,
leaves are terminals (read left-to-right give the derived string), and each node's children
match some production rule.

### Ambiguity

A grammar is **ambiguous** if some terminal string has *two or more distinct parse trees*
(equivalently, two or more leftmost derivations). Classic example — the dangling-else problem:

```
S → if E then S | if E then S else S | other
```

The string `if E then if E then S else S` has two parse trees depending on which `if` the
`else` attaches to. Languages like C and Java resolve this with the "else matches nearest if"
convention, but the *grammar itself* remains ambiguous.

A language is **inherently ambiguous** if *every* CFG that generates it is ambiguous. Example:
{ aⁿbⁿcᵐ } ∪ { aⁿbᵐcᵐ }.

### CFG vs Regular Languages

Every regular language is context-free (you can convert a DFA to a right-linear grammar), but not
vice versa. The language { aⁿbⁿ | n ≥ 0 } is context-free (grammar: `S → aSb | ε`) but not
regular (provable via the pumping lemma). CFGs are strictly more powerful.

## Build It

### Step 1: Minimal Version

A CFG class with rule storage, simple derivation, and membership testing by exhaustive search.

```python
from __future__ import annotations
import random
from collections import defaultdict
from typing import Optional


class CFG:
    def __init__(self, start: str = "S"):
        self.start = start
        self.rules: dict[str, list[list[str]]] = defaultdict(list)
        self.terminals: set[str] = set()

    def add_rule(self, head: str, body: list[str]) -> None:
        self.rules[head].append(body)
        for sym in body:
            if sym not in self.rules and sym.islower():
                self.terminals.add(sym)

    def _derive(self, sentential: list[str], depth: int) -> Optional[list[str]]:
        if depth > 50:
            return None
        if all(s in self.terminals or s == "" for s in sentential):
            return sentential
        for i, sym in enumerate(sentential):
            if sym in self.rules:
                for body in self.rules[sym]:
                    new = sentential[:i] + body + sentential[i + 1:]
                    result = self._derive(new, depth + 1)
                    if result is not None:
                        return result
        return None

    def generate(self) -> str:
        result = self._derive([self.start], 0)
        if result is None:
            return ""
        return "".join(s for s in result if s != "")

    def derives(self, target: str, max_depth: int = 50) -> bool:
        return self._check([self.start], target, 0, max_depth)

    def _check(self, sent: list[str], target: str, depth: int, max_depth: int) -> bool:
        if depth > max_depth:
            return False
        prefix = "".join(s for s in sent if s != "")
        if len(prefix) > len(target):
            return False
        if all(s in self.terminals or s == "" for s in sent):
            return prefix == target
        for i, sym in enumerate(sent):
            if sym in self.rules:
                for body in self.rules[sym]:
                    new = sent[:i] + body + sent[i + 1:]
                    if self._check(new, target, depth + 1, max_depth):
                        return True
        return False
```

### Step 2: Parse Tree and Ambiguity Detection

Extend with full parse-tree generation and multi-derivation enumeration.

```python
from dataclasses import dataclass, field


@dataclass
class ParseNode:
    label: str
    children: list["ParseNode"] = field(default_factory=list)

    def is_leaf(self) -> bool:
        return len(self.children) == 0

    def yield_string(self) -> str:
        if self.is_leaf():
            return self.label
        return "".join(child.yield_string() for child in self.children)

    def __repr__(self) -> str:
        if self.is_leaf():
            return self.label
        kids = " ".join(repr(c) for c in self.children)
        return f"({self.label} {kids})"


class CFGFull(CFG):
    def parse_tree(self, target: str) -> Optional[ParseNode]:
        trees = list(self._parse_trees([self.start], target, 0))
        return trees[0] if trees else None

    def all_parse_trees(self, target: str, max_depth: int = 50) -> list[ParseNode]:
        return list(self._parse_trees([self.start], target, 0, max_depth))

    def is_ambiguous(self, target: str, max_depth: int = 50) -> bool:
        count = 0
        for _ in self._parse_trees([self.start], target, 0, max_depth):
            count += 1
            if count >= 2:
                return True
        return False

    def _parse_trees(self, sent: list[str], target: str,
                     depth: int, max_depth: int = 50):
        if depth > max_depth:
            return
        current = "".join(s for s in sent if s != "")
        if len(current) > len(target):
            return
        if all(s in self.terminals or s == "" for s in sent):
            if current == target:
                leaves = [ParseNode(s) for s in sent if s != ""]
                if not leaves:
                    leaves = [ParseNode("")]
                yield self._build_tree_from_leaves(leaves)
            return
        for i, sym in enumerate(sent):
            if sym in self.rules:
                for body in self.rules[sym]:
                    new = sent[:i] + body + sent[i + 1:]
                    for subtree in self._parse_trees(new, target, depth + 1, max_depth):
                        yield subtree
                break

    def _build_tree_from_leaves(self, leaves: list[ParseNode]) -> ParseNode:
        return ParseNode("S", leaves) if len(leaves) > 1 else leaves[0]
```

## Use It

Real programming language parsers use CFGs written in **BNF** (Backus-Naur Form) or its
variant **EBNF**. Here is a fragment of Python's grammar (from `Grammar/python.gram` in
CPython's source):

```
statement     ::= assignment | return_stmt | if_stmt | ...
if_stmt       ::= 'if' expression ':' suite ('elif' expression ':' suite)* ['else' ':' suite]
expression    ::= term (('+' | '-') term)*
term          ::= factor (('*' | '/') factor)*
factor        ::= '(' expression ')' | NAME | NUMBER
```

Tools like **ANTLR**, **Yacc/Bison**, and **Lark** take a grammar specification and generate
parser code automatically. Your hand-built `CFG` class can enumerate parse trees by brute-force
search; production parsers use deterministic algorithms (LR, LALR, LL) that run in O(n) time.

**Read the Source**: CPython's `Grammar/python.gram` — the PEG grammar that drives the `pegen`
parser since Python 3.9.

## Ship It

The reusable artifact produced by this lesson is a **CFG library** that you can use in
Lesson 09 (normal forms) and Lesson 10 (CYK parsing). It provides:

- Rule storage and derivation
- Parse-tree construction
- Ambiguity detection

## Exercises

1. **Easy** — Write a CFG for the language { aⁿbⁿ | n ≥ 0 }. Verify your `derives` method
   accepts `"aabb"` and rejects `"abab"`.
2. **Medium** — Write an unambiguous CFG for arithmetic expressions with `+`, `*`, parentheses,
   and integer literals. Prove it is unambiguous by showing every string has exactly one parse tree.
3. **Hard** — Implement the **Chomsky normal form** check: verify every rule is either `A → BC`
   or `A → a`. Write a function that detects whether a given grammar is already in CNF.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Context-free grammar | "A grammar for a programming language" | A 4-tuple (V, Σ, R, S) where every production has a single variable on the left |
| Sentential form | "A step in a derivation" | Any string in (V ∪ Σ)* derivable from S |
| Parse tree | "Syntax tree" | A tree showing which rules were applied and in what order |
| Ambiguity | "The grammar is ambiguous" | Some string has two or more distinct parse trees |
| Inherent ambiguity | "The language itself is ambiguous" | No unambiguous CFG exists for the language |
| Leftmost derivation | "Expand left to right" | At each step, replace the leftmost variable |

## Further Reading

- Hopcroft, Motwani, Ullman — *Introduction to Automata Theory, Languages, and Computation*, Ch. 5–7
- Sipser — *Introduction to the Theory of Computation*, Ch. 2
- CPython PEG parser: `https://docs.python.org/3/reference/grammar.html`
