"""Propositional logic AST + evaluator + truth-table generator + equivalence checker.

Run:  python3 main.py
"""
from __future__ import annotations

import itertools
from dataclasses import dataclass
from typing import Dict, List, Set, Tuple, Union


# ── AST ─────────────────────────────────────────────────────────────

@dataclass(frozen=True)
class Var:
    name: str

@dataclass(frozen=True)
class Not:
    child: "Formula"

@dataclass(frozen=True)
class And:
    left: "Formula"
    right: "Formula"

@dataclass(frozen=True)
class Or:
    left: "Formula"
    right: "Formula"

@dataclass(frozen=True)
class Imp:
    left: "Formula"
    right: "Formula"

@dataclass(frozen=True)
class Iff:
    left: "Formula"
    right: "Formula"


Formula = Union[Var, Not, And, Or, Imp, Iff]


# ── Pretty-print ────────────────────────────────────────────────────

def fmt(f: Formula) -> str:
    if isinstance(f, Var):
        return f.name
    if isinstance(f, Not):
        return f"¬{fmt(f.child)}"
    if isinstance(f, And):
        return f"({fmt(f.left)} ∧ {fmt(f.right)})"
    if isinstance(f, Or):
        return f"({fmt(f.left)} ∨ {fmt(f.right)})"
    if isinstance(f, Imp):
        return f"({fmt(f.left)} → {fmt(f.right)})"
    if isinstance(f, Iff):
        return f"({fmt(f.left)} ↔ {fmt(f.right)})"
    raise TypeError(f)


# ── Eval + helpers ──────────────────────────────────────────────────

def evaluate(f: Formula, env: Dict[str, bool]) -> bool:
    if isinstance(f, Var):
        return env[f.name]
    if isinstance(f, Not):
        return not evaluate(f.child, env)
    if isinstance(f, And):
        return evaluate(f.left, env) and evaluate(f.right, env)
    if isinstance(f, Or):
        return evaluate(f.left, env) or evaluate(f.right, env)
    if isinstance(f, Imp):
        return (not evaluate(f.left, env)) or evaluate(f.right, env)
    if isinstance(f, Iff):
        return evaluate(f.left, env) == evaluate(f.right, env)
    raise TypeError(f)


def variables(f: Formula) -> Set[str]:
    if isinstance(f, Var):
        return {f.name}
    if isinstance(f, Not):
        return variables(f.child)
    return variables(f.left) | variables(f.right)  # And/Or/Imp/Iff


def truth_table(f: Formula) -> Tuple[List[str], List[Tuple[Dict[str, bool], bool]]]:
    vars_sorted = sorted(variables(f))
    rows: List[Tuple[Dict[str, bool], bool]] = []
    for bits in itertools.product([False, True], repeat=len(vars_sorted)):
        env = dict(zip(vars_sorted, bits))
        rows.append((env, evaluate(f, env)))
    return vars_sorted, rows


def print_table(f: Formula) -> None:
    vars_sorted, rows = truth_table(f)
    formula_str = fmt(f)
    header = "  ".join(vars_sorted) + "  |  " + formula_str
    print(header)
    print("─" * len(header))
    for env, result in rows:
        row = "  ".join("T" if env[v] else "F" for v in vars_sorted)
        print(f"{row}  |  {'T' if result else 'F'}")


def equivalent(f: Formula, g: Formula) -> bool:
    vars_all = sorted(variables(f) | variables(g))
    for bits in itertools.product([False, True], repeat=len(vars_all)):
        env = dict(zip(vars_all, bits))
        if evaluate(f, env) != evaluate(g, env):
            return False
    return True


def is_tautology(f: Formula) -> bool:
    return all(r for _, r in truth_table(f)[1])


def is_contradiction(f: Formula) -> bool:
    return all(not r for _, r in truth_table(f)[1])


# ── Demo ────────────────────────────────────────────────────────────

def main() -> None:
    P, Q, R = Var("P"), Var("Q"), Var("R")

    print("== Truth table: P → Q ==")
    print_table(Imp(P, Q))

    print("\n== De Morgan: ¬(P ∧ Q) ≡ ¬P ∨ ¬Q ==")
    lhs = Not(And(P, Q))
    rhs = Or(Not(P), Not(Q))
    print(f"{fmt(lhs)}  ≡  {fmt(rhs)}   →   {equivalent(lhs, rhs)}")

    print("\n== Contrapositive: P → Q ≡ ¬Q → ¬P ==")
    print(f"{equivalent(Imp(P, Q), Imp(Not(Q), Not(P)))}")

    print("\n== Hypothetical syllogism: ((P→Q) ∧ (Q→R)) → (P→R) — tautology? ==")
    f = Imp(And(Imp(P, Q), Imp(Q, R)), Imp(P, R))
    print(f"  {fmt(f)}")
    print(f"  tautology = {is_tautology(f)}")

    print("\n== P ∧ ¬P — contradiction? ==")
    f = And(P, Not(P))
    print(f"  {fmt(f)}: contradiction = {is_contradiction(f)}")


if __name__ == "__main__":
    main()
