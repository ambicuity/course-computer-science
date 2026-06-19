"""truth_table.py — standalone CLI for propositional logic truth tables.

Usage:
    python3 truth_table.py "P -> Q"
    python3 truth_table.py "(P & Q) | (~P & R)"

Operators (ASCII):  ~  &  |  ->  <->     (precedence: ~ > & > | > -> <->)
Variables: any identifier starting with a letter, e.g. P, Q, x1, alpha.
"""
import argparse
import itertools
import re
import sys
from typing import Dict, List, Set, Tuple


# ── Lexer ──────────────────────────────────────────────────────────

TOKEN_RE = re.compile(r"\s*(<->|->|[A-Za-z][A-Za-z0-9_]*|[~&|()])")

def tokenize(src: str) -> List[str]:
    out, i = [], 0
    while i < len(src):
        m = TOKEN_RE.match(src, i)
        if not m:
            raise SyntaxError(f"unexpected at pos {i}: {src[i:i+10]!r}")
        out.append(m.group(1))
        i = m.end()
    return out


# ── Parser (recursive descent over precedence) ──────────────────────

class Parser:
    def __init__(self, tokens: List[str]):
        self.toks = tokens
        self.i = 0

    def peek(self): return self.toks[self.i] if self.i < len(self.toks) else None
    def eat(self):  t = self.toks[self.i]; self.i += 1; return t

    def parse(self):
        f = self.iff_()
        if self.i != len(self.toks):
            raise SyntaxError(f"extra tokens: {self.toks[self.i:]}")
        return f

    def iff_(self):
        left = self.imp()
        while self.peek() == "<->":
            self.eat(); right = self.imp(); left = ("iff", left, right)
        return left

    def imp(self):
        left = self.or_()
        if self.peek() == "->":     # right-associative
            self.eat(); right = self.imp(); return ("imp", left, right)
        return left

    def or_(self):
        left = self.and_()
        while self.peek() == "|":
            self.eat(); right = self.and_(); left = ("or", left, right)
        return left

    def and_(self):
        left = self.not_()
        while self.peek() == "&":
            self.eat(); right = self.not_(); left = ("and", left, right)
        return left

    def not_(self):
        if self.peek() == "~":
            self.eat(); return ("not", self.not_())
        return self.atom()

    def atom(self):
        t = self.eat()
        if t == "(":
            f = self.iff_()
            if self.eat() != ")":
                raise SyntaxError("expected )")
            return f
        if re.match(r"^[A-Za-z]", t):
            return ("var", t)
        raise SyntaxError(f"unexpected {t!r}")


def evaluate(f, env: Dict[str, bool]) -> bool:
    op = f[0]
    if op == "var": return env[f[1]]
    if op == "not": return not evaluate(f[1], env)
    if op == "and": return evaluate(f[1], env) and evaluate(f[2], env)
    if op == "or":  return evaluate(f[1], env) or evaluate(f[2], env)
    if op == "imp": return (not evaluate(f[1], env)) or evaluate(f[2], env)
    if op == "iff": return evaluate(f[1], env) == evaluate(f[2], env)
    raise ValueError(op)


def variables(f) -> Set[str]:
    op = f[0]
    if op == "var": return {f[1]}
    if op == "not": return variables(f[1])
    return variables(f[1]) | variables(f[2])


def main():
    ap = argparse.ArgumentParser(description="Propositional truth table")
    ap.add_argument("expr", help='Formula in ASCII syntax, e.g. "P -> (Q | ~R)"')
    args = ap.parse_args()

    formula = Parser(tokenize(args.expr)).parse()
    vars_sorted = sorted(variables(formula))

    header = "  ".join(vars_sorted) + "  |  " + args.expr
    print(header); print("─" * len(header))
    all_true = all_false = True
    for bits in itertools.product([False, True], repeat=len(vars_sorted)):
        env = dict(zip(vars_sorted, bits))
        out = evaluate(formula, env)
        all_true &= out; all_false &= (not out)
        row = "  ".join("T" if env[v] else "F" for v in vars_sorted)
        print(f"{row}  |  {'T' if out else 'F'}")

    print()
    if   all_true:  print("→ tautology")
    elif all_false: print("→ contradiction")
    else:           print("→ contingency (satisfiable + falsifiable)")


if __name__ == "__main__":
    main()
