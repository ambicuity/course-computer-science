"""
Context-Free Grammars
Phase 05 — Theory of Computation, Lesson 07

CFG class with rule storage, derivation, parse-tree generation, and ambiguity detection.
Demonstrates balanced parentheses grammar, arithmetic expressions, and the dangling-else ambiguity.
"""

from __future__ import annotations
import random
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Optional


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

    def depth(self) -> int:
        if self.is_leaf():
            return 0
        return 1 + max(child.depth() for child in self.children)

    def __repr__(self) -> str:
        if self.is_leaf():
            return self.label
        kids = " ".join(repr(c) for c in self.children)
        return f"({self.label} {kids})"


class CFG:
    def __init__(self, start: str = "S"):
        self.start = start
        self.rules: dict[str, list[list[str]]] = defaultdict(list)
        self.variables: set[str] = set()
        self.terminals: set[str] = set()
        self.variables.add(start)

    def add_rule(self, head: str, body: list[str]) -> None:
        self.variables.add(head)
        self.rules[head].append(body)
        for sym in body:
            if sym.isupper():
                self.variables.add(sym)
            elif sym != "":
                self.terminals.add(sym)

    def derive(self, strategy: str = "leftmost", max_steps: int = 30) -> list[str]:
        if strategy == "leftmost":
            return self._leftmost([self.start], 0, max_steps)
        return self._rightmost([self.start], 0, max_steps)

    def _leftmost(self, sent: list[str], depth: int, max_steps: int) -> list[str]:
        if depth > max_steps:
            return sent
        if all(s in self.terminals or s == "" for s in sent):
            return sent
        for i, sym in enumerate(sent):
            if sym in self.rules:
                body = random.choice(self.rules[sym])
                new_sent = sent[:i] + body + sent[i + 1:]
                return self._leftmost(new_sent, depth + 1, max_steps)
        return sent

    def _rightmost(self, sent: list[str], depth: int, max_steps: int) -> list[str]:
        if depth > max_steps:
            return sent
        if all(s in self.terminals or s == "" for s in sent):
            return sent
        for i in range(len(sent) - 1, -1, -1):
            if sent[i] in self.rules:
                body = random.choice(self.rules[sent[i]])
                new_sent = sent[:i] + body + sent[i + 1:]
                return self._rightmost(new_sent, depth + 1, max_steps)
        return sent

    def derives(self, target: str, max_depth: int = 30) -> bool:
        return self._check([self.start], target, 0, max_depth)

    def _check(self, sent: list[str], target: str, depth: int, max_depth: int) -> bool:
        if depth > max_depth:
            return False
        # Prune: count terminals so far — if they already exceed target, bail
        term_count = sum(1 for s in sent if s in self.terminals)
        if term_count > len(target):
            return False
        for i, sym in enumerate(sent):
            if sym in self.rules:
                for body in self.rules[sym]:
                    new_sent = sent[:i] + body + sent[i + 1:]
                    if self._check(new_sent, target, depth + 1, max_depth):
                        return True
                return False
        current = "".join(s for s in sent if s != "")
        return current == target

    def random_derivation(self, max_steps: int = 30) -> str:
        sentential = [self.start]
        for _ in range(max_steps):
            var_positions = [i for i, s in enumerate(sentential) if s in self.rules]
            if not var_positions:
                break
            pos = random.choice(var_positions)
            body = random.choice(self.rules[sentential[pos]])
            sentential = sentential[:pos] + body + sentential[pos + 1:]
        return "".join(s for s in sentential if s != "")

    def parse_tree(self, target: str, max_depth: int = 30) -> Optional[ParseNode]:
        return self._build_tree([self.start], target, 0, max_depth)

    def _build_tree(self, sent: list[str], target: str,
                    depth: int, max_depth: int) -> Optional[ParseNode]:
        if depth > max_depth:
            return None
        term_count = sum(1 for s in sent if s in self.terminals)
        if term_count > len(target):
            return None
        for i, sym in enumerate(sent):
            if sym in self.rules:
                for body in self.rules[sym]:
                    new_sent = sent[:i] + body + sent[i + 1:]
                    result = self._build_tree(new_sent, target, depth + 1, max_depth)
                    if result is not None:
                        return result
                return None
        current = "".join(s for s in sent if s != "")
        if current == target:
            leaves = [ParseNode(s) for s in sent if s != ""]
            if not leaves:
                return ParseNode("")
            if len(leaves) == 1:
                return leaves[0]
            return ParseNode(self.start, leaves)
        return None

    def all_parse_trees(self, target: str, max_depth: int = 30,
                        max_trees: int = 100) -> list[ParseNode]:
        results = []
        for tree in self._gen_trees([self.start], target, 0, max_depth):
            results.append(tree)
            if len(results) >= max_trees:
                break
        return results

    def _gen_trees(self, sent: list[str], target: str,
                   depth: int, max_depth: int):
        if depth > max_depth:
            return
        term_count = sum(1 for s in sent if s in self.terminals)
        if term_count > len(target):
            return
        for i, sym in enumerate(sent):
            if sym in self.rules:
                for body in self.rules[sym]:
                    new_sent = sent[:i] + body + sent[i + 1:]
                    yield from self._gen_trees(new_sent, target, depth + 1, max_depth)
                return
        current = "".join(s for s in sent if s != "")
        if current == target:
            leaves = [ParseNode(s) for s in sent if s != ""]
            if not leaves:
                yield ParseNode("")
            elif len(leaves) == 1:
                yield leaves[0]
            else:
                yield ParseNode(self.start, leaves)

    def is_ambiguous(self, target: str, max_depth: int = 30) -> bool:
        count = 0
        for _ in self._gen_trees([self.start], target, 0, max_depth):
            count += 1
            if count >= 2:
                return True
        return False

    def __repr__(self) -> str:
        lines = []
        for head in sorted(self.rules):
            bodies = [" ".join(b) if b else "ε" for b in self.rules[head]]
            lines.append(f"  {head} → {' | '.join(bodies)}")
        return f"CFG(start={self.start}):\n" + "\n".join(lines)


def grammar_balanced_parens() -> CFG:
    g = CFG("S")
    g.add_rule("S", ["(", "S", ")", "S"])
    g.add_rule("S", [])
    return g


def grammar_arithmetic() -> CFG:
    g = CFG("E")
    g.add_rule("E", ["E", "+", "T"])
    g.add_rule("E", ["T"])
    g.add_rule("T", ["T", "*", "F"])
    g.add_rule("T", ["F"])
    g.add_rule("F", ["(", "E", ")"])
    g.add_rule("F", ["i"])
    return g


def grammar_anbn() -> CFG:
    g = CFG("S")
    g.add_rule("S", ["a", "S", "b"])
    g.add_rule("S", [])
    return g


def grammar_dangling_else() -> CFG:
    g = CFG("S")
    g.add_rule("S", ["i", "E", "t", "S"])
    g.add_rule("S", ["i", "E", "t", "S", "e", "S"])
    g.add_rule("S", ["o"])
    return g


def main() -> None:
    print("=" * 60)
    print("Lesson 07: Context-Free Grammars")
    print("=" * 60)

    # --- Balanced Parentheses ---
    print("\n1. Balanced Parentheses Grammar")
    parens = grammar_balanced_parens()
    print(parens)
    for _ in range(5):
        s = parens.random_derivation()
        print(f"  Generated: '{s}' (len={len(s)})")
    print(f"  derives '(()())'? {parens.derives('(()())')}")
    print(f"  derives '(()'?     {parens.derives('(()')}")

    # --- Arithmetic Expressions ---
    print("\n2. Arithmetic Expression Grammar")
    arith = grammar_arithmetic()
    print(arith)
    print(f"  derives 'i+i*i'? {arith.derives('i+i*i')}")
    print(f"  derives 'i'? {arith.derives('i')}")
    print(f"  derives '(i+i)*i'? {arith.derives('(i+i)*i')}")

    # --- aⁿbⁿ ---
    print("\n3. {aⁿbⁿ} Grammar")
    anbn = grammar_anbn()
    print(anbn)
    for n in range(5):
        target = "a" * n + "b" * n
        print(f"  derives '{target}'? {anbn.derives(target)}")
    print(f"  derives 'abab'? {anbn.derives('abab')}")

    # --- Dangling Else ---
    print("\n4. Dangling-Else Grammar")
    de = grammar_dangling_else()
    print(de)
    print(f"  derives 'o'? {de.derives('o')}")
    print(f"  derives 'iEto'? {de.derives('iEto')}")

    # --- Parse Tree ---
    print("\n5. Parse Tree Construction")
    tree = parens.parse_tree("(())")
    if tree:
        print(f"  Parse tree for '(())': {tree}")
    else:
        print("  No parse tree found for '(())'")

    tree2 = parens.parse_tree("()()")
    if tree2:
        print(f"  Parse tree for '()()': {tree2}")

    # --- Ambiguity Check ---
    print("\n6. Ambiguity Detection")
    print(f"  Parens grammar ambiguous for '()()'? {parens.is_ambiguous('()()')}")
    print(f"  Parens grammar ambiguous for '(())'? {parens.is_ambiguous('(())')}")
    trees = parens.all_parse_trees("()()", max_trees=5)
    print(f"  Parse trees for '()()': {len(trees)}")
    for idx, t in enumerate(trees):
        print(f"    Tree {idx + 1}: {t}")


if __name__ == "__main__":
    main()
