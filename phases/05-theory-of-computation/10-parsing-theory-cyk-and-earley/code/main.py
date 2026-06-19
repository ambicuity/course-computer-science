"""Lesson 10: CYK and Earley Parsers."""

from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass, field
from typing import Dict, List, Set, Tuple


# ──────────────────────────────────────────────
#  CYK Parser
# ──────────────────────────────────────────────

@dataclass
class CNFGrammar:
    """Chomsky Normal Form grammar.

    Rules are stored as:
        binary[A] = {(B, C), ...}   for  A → B C
        unit[A]   = {terminal, ...}  for  A → a
    """

    start: str
    binary: Dict[str, Set[Tuple[str, str]]] = field(default_factory=lambda: defaultdict(set))
    unit: Dict[str, Set[str]] = field(default_factory=lambda: defaultdict(set))
    variables: Set[str] = field(default_factory=set)

    @classmethod
    def from_rules(cls, start: str, rules: list[tuple[str, list[str]]]) -> "CNFGrammar":
        """Build from list of (head, body) pairs.  Body length must be 1 or 2."""
        g = cls(start=start)
        for head, body in rules:
            g.variables.add(head)
            if len(body) == 1 and body[0].islower():
                g.unit[head].add(body[0])
            elif len(body) == 2:
                g.variables.add(body[0])
                g.variables.add(body[1])
                g.binary[head].add((body[0], body[1]))
            else:
                raise ValueError(f"Rule not in CNF: {head} → {' '.join(body)}")
        return g


def cyk_parse(
    grammar: CNFGrammar, string: str
) -> tuple[list[list[set[str]]], bool]:
    """Run CYK algorithm.

    Returns (table, accepted) where table[i][j] is the set of variables
    deriving string[i..j] (inclusive).  Trace output is printed.
    """
    n = len(string)
    if n == 0:
        empty: list[list[set[str]]] = [[set()]]
        # Special case: check if start derives ε (not in strict CNF, but
        # callers can handle it).
        return empty, False

    T: list[list[set[str]]] = [[set() for _ in range(n)] for _ in range(n)]

    # Base case: length-1 substrings
    print("=== CYK Parse ===")
    print(f"Input: '{string}'  (length {n})\n")
    print("Step 1 — Fill diagonal (length-1 substrings)")
    for j in range(n):
        for var, terminals in grammar.unit.items():
            if string[j] in terminals:
                T[j][j].add(var)
        print(f"  T[{j}][{j}] ('{string[j]}') = {T[j][j] or '∅'}")

    # Inductive case
    print("\nStep 2 — Fill upper triangle (longer substrings)")
    for span in range(2, n + 1):
        print(f"\n  Span length {span}:")
        for i in range(n - span + 1):
            j = i + span - 1
            for k in range(i, j):
                for var, pairs in grammar.binary.items():
                    for B, C in pairs:
                        if B in T[i][k] and C in T[k + 1][j]:
                            if var not in T[i][j]:
                                T[i][j].add(var)
                                print(
                                    f"    T[{i}][{j}]: {var} → {B} C  "
                                    f"(split at k={k}, {B}∈T[{i}][{k}], "
                                    f"{C}∈T[{k+1}][{j}])"
                                )

    accepted = grammar.start in T[0][n - 1]
    print(f"\nResult: {'ACCEPTED' if accepted else 'REJECTED'}")
    if accepted:
        print(f"  '{grammar.start}' ∈ T[0][{n - 1}]")
    return T, accepted


# ──────────────────────────────────────────────
#  Earley Parser
# ──────────────────────────────────────────────

@dataclass(frozen=True)
class EarleyItem:
    """A dotted rule with origin index."""

    head: str
    body: tuple[str, ...]
    dot: int
    origin: int

    def __repr__(self) -> str:
        dotted = " ".join(
            [f"· {b}" if i == self.dot else b for i, b in enumerate(self.body)]
        )
        if self.dot == len(self.body):
            dotted += " ·"
        return f"({self.head} → {dotted}, {self.origin})"


@dataclass
class CFGrammar:
    """Arbitrary context-free grammar (no CNF restriction)."""

    start: str
    rules: Dict[str, List[Tuple[str, ...]]] = field(
        default_factory=lambda: defaultdict(list)
    )

    def add_rule(self, head: str, body: list[str]) -> None:
        self.rules[head].append(tuple(body))


def earley_parse(
    grammar: CFGrammar, string: str
) -> list[set[EarleyItem]]:
    """Run the Earley algorithm.

    Returns the chart — a list of sets of EarleyItems, one per position
    0..n.  Trace output is printed.
    """
    n = len(string)
    S: list[set[EarleyItem]] = [set() for _ in range(n + 1)]

    # Augmented start
    augmented_start = "S'"
    S0_rules = [(augmented_start, (grammar.start,))]

    # Seed
    for head, body in S0_rules:
        S[0].add(EarleyItem(head, body, 0, 0))
    for body in grammar.rules.get(grammar.start, []):
        S[0].add(EarleyItem(grammar.start, body, 0, 0))

    print("=== Earley Parse ===")
    print(f"Input: '{string}'  (length {n})\n")

    for k in range(n + 1):
        changed = True
        agenda: list[EarleyItem] = list(S[k])
        visited: set[EarleyItem] = set()

        while agenda:
            item = agenda.pop()
            if item in visited:
                continue
            visited.add(item)

            # Predictor: dot before a nonterminal
            if item.dot < len(item.body):
                symbol = item.body[item.dot]
                if symbol in grammar.rules and symbol[0].isupper():
                    for body in grammar.rules[symbol]:
                        new = EarleyItem(symbol, body, 0, k)
                        if new not in visited:
                            S[k].add(new)
                            agenda.append(new)

            # Scanner: dot before a terminal (advance to next column)
            if item.dot < len(item.body) and k < n:
                symbol = item.body[item.dot]
                if symbol not in grammar.rules or symbol[0].islower():
                    if symbol == string[k]:
                        new = EarleyItem(
                            item.head, item.body, item.dot + 1, item.origin
                        )
                        S[k + 1].add(new)

            # Completer: dot at end
            if item.dot == len(item.body):
                for parent in list(S[item.origin]):
                    if (
                        parent.dot < len(parent.body)
                        and parent.body[parent.dot] == item.head
                    ):
                        new = EarleyItem(
                            parent.head,
                            parent.body,
                            parent.dot + 1,
                            parent.origin,
                        )
                        if new not in visited:
                            S[k].add(new)
                            agenda.append(new)

        # Print chart set
        print(f"Chart S[{k}]:")
        for it in sorted(S[k], key=lambda x: (x.origin, x.head)):
            print(f"  {it}")
        print()

    accepted = any(
        it.head == augmented_start and it.dot == len(it.body) and it.origin == 0
        for it in S[n]
    )
    print(f"Result: {'ACCEPTED' if accepted else 'REJECTED'}")
    return S


# ──────────────────────────────────────────────
#  Examples
# ──────────────────────────────────────────────

def example_cyk() -> None:
    """Grammar (CNF) for { aⁿbⁿcⁿ | n ≥ 1 } subset:

        S  → A B
        A  → A C | a
        B  → b  B₁ | b c
        B₁ → b c
        C  → a C₁
        C₁ → b c

    Simplified demo: S → A B, A → a, B → b
    """
    grammar = CNFGrammar.from_rules(
        start="S",
        rules=[
            ("S", ["A", "B"]),
            ("A", ["a"]),
            ("B", ["b"]),
        ],
    )
    cyk_parse(grammar, "ab")
    print("\n" + "=" * 40 + "\n")


def example_earley() -> None:
    """Ambiguous grammar: S → S S | a  (input 'aaa')."""
    grammar = CFGrammar(start="S")
    grammar.add_rule("S", ["S", "S"])
    grammar.add_rule("S", ["a"])
    earley_parse(grammar, "aaa")
    print("\n" + "=" * 40 + "\n")


if __name__ == "__main__":
    example_cyk()
    example_earley()
