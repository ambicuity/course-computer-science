"""Predicate logic evaluator over finite domains, with counterexample search.

Run:  python3 main.py
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Callable, Dict, List, Optional, Tuple, Union


# ── AST ─────────────────────────────────────────────────────────────

@dataclass(frozen=True)
class PredVar:
    name: str
    args: Tuple[str, ...]

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
class ForAll:
    var: str
    body: "Formula"

@dataclass(frozen=True)
class Exists:
    var: str
    body: "Formula"

Formula = Union[PredVar, Not, And, Or, Imp, ForAll, Exists]


# ── Interpretation ──────────────────────────────────────────────────

@dataclass
class Interpretation:
    domain: List[object]
    predicates: Dict[str, Callable[..., bool]]


# ── Evaluate ────────────────────────────────────────────────────────

def evaluate(f: Formula, interp: Interpretation, env: Dict[str, object]) -> bool:
    if isinstance(f, PredVar):
        args = tuple(env[a] for a in f.args)
        return bool(interp.predicates[f.name](*args))
    if isinstance(f, Not):
        return not evaluate(f.child, interp, env)
    if isinstance(f, And):
        return evaluate(f.left, interp, env) and evaluate(f.right, interp, env)
    if isinstance(f, Or):
        return evaluate(f.left, interp, env) or evaluate(f.right, interp, env)
    if isinstance(f, Imp):
        return (not evaluate(f.left, interp, env)) or evaluate(f.right, interp, env)
    if isinstance(f, ForAll):
        return all(evaluate(f.body, interp, {**env, f.var: d}) for d in interp.domain)
    if isinstance(f, Exists):
        return any(evaluate(f.body, interp, {**env, f.var: d}) for d in interp.domain)
    raise TypeError(f)


def counterexample(f: Formula, interp: Interpretation,
                   env: Dict[str, object]) -> Optional[Dict[str, object]]:
    """If f is a `ForAll`, return the first binding that falsifies the body, else None."""
    if isinstance(f, ForAll):
        for d in interp.domain:
            new_env = {**env, f.var: d}
            if not evaluate(f.body, interp, new_env):
                return {f.var: d}
    return None


# ── Demo ────────────────────────────────────────────────────────────

def is_prime(x: int) -> bool:
    if x < 2: return False
    return all(x % d for d in range(2, int(x**0.5) + 1))


def main() -> None:
    interp = Interpretation(
        domain=list(range(0, 50)),
        predicates={
            "Prime": is_prime,
            "Odd":   lambda x: x % 2 == 1,
            "Even":  lambda x: x % 2 == 0,
            "Gt":    lambda x, y: x > y,
            "Eq":    lambda x, y: x == y,
        },
    )

    # 1. Every prime > 2 is odd.
    #    ∀x. (Prime(x) ∧ x > 2) → Odd(x)
    f = ForAll("x",
            Imp(And(PredVar("Prime", ("x",)),
                    PredVar("Gt", ("x", "two"))),
                PredVar("Odd", ("x",))))
    env = {"two": 2}
    print(f"∀x. (Prime(x) ∧ x>2) → Odd(x)   :   {evaluate(f, interp, env)}")

    # 2. (Wrong) Every prime is odd — should be False; show a counterexample.
    f2 = ForAll("x", Imp(PredVar("Prime", ("x",)), PredVar("Odd", ("x",))))
    print(f"∀x. Prime(x) → Odd(x)          :   {evaluate(f2, interp, {})}")
    print(f"  counterexample: {counterexample(f2, interp, {})}")  # x=2

    # 3. Order of quantifiers matters: ∀x ∃y. y > x vs ∃y ∀x. y > x
    #    First is True (always pick y = x+1, but limited by finite domain edge);
    #    Second would say "one y greater than every x" — False on any finite domain.
    f3 = ForAll("x", Exists("y", PredVar("Gt", ("y", "x"))))
    f4 = Exists("y", ForAll("x", PredVar("Gt", ("y", "x"))))
    print(f"∀x ∃y. y > x                   :   {evaluate(f3, interp, {})}  (False at the largest x — finite-domain artifact)")
    print(f"∃y ∀x. y > x                   :   {evaluate(f4, interp, {})}  (no single y exceeds every x)")

    # 4. De Morgan for quantifiers
    #    ¬∀x. Even(x) ≡ ∃x. ¬Even(x)
    f5 = Not(ForAll("x", PredVar("Even", ("x",))))
    f6 = Exists("x", Not(PredVar("Even", ("x",))))
    print(f"\nDe Morgan: ¬∀x. Even(x)  ≡  ∃x. ¬Even(x)")
    print(f"  LHS = {evaluate(f5, interp, {})}, RHS = {evaluate(f6, interp, {})}, equiv = {evaluate(f5, interp, {}) == evaluate(f6, interp, {})}")


if __name__ == "__main__":
    main()
