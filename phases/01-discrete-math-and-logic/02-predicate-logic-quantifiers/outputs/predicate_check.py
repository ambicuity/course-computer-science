"""predicate_check.py — reusable library + CLI for predicate-logic formulas
over finite domains, with counterexample reporting.

Library usage:
    from predicate_check import Interpretation, ForAll, Exists, Imp, And, Or, Not, PredVar, evaluate

CLI (very small DSL, for the lesson's needs):
    The CLI is illustrative; production work should use Z3 / TLA+ instead.
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Callable, Dict, List, Optional, Tuple, Union

# ── ASTs (same shape as the lesson's main.py) ──────────────────────

@dataclass(frozen=True)
class PredVar:
    name: str
    args: Tuple[str, ...]

@dataclass(frozen=True)
class Not:   child: "Formula"
@dataclass(frozen=True)
class And:   left: "Formula"; right: "Formula"
@dataclass(frozen=True)
class Or:    left: "Formula"; right: "Formula"
@dataclass(frozen=True)
class Imp:   left: "Formula"; right: "Formula"
@dataclass(frozen=True)
class Iff:   left: "Formula"; right: "Formula"
@dataclass(frozen=True)
class ForAll: var: str; body: "Formula"
@dataclass(frozen=True)
class Exists: var: str; body: "Formula"

Formula = Union[PredVar, Not, And, Or, Imp, Iff, ForAll, Exists]


@dataclass
class Interpretation:
    domain: List[object]
    predicates: Dict[str, Callable[..., bool]]


def evaluate(f: Formula, interp: Interpretation, env: Dict[str, object]) -> bool:
    if isinstance(f, PredVar):
        return bool(interp.predicates[f.name](*(env[a] for a in f.args)))
    if isinstance(f, Not):   return not evaluate(f.child, interp, env)
    if isinstance(f, And):   return evaluate(f.left, interp, env) and evaluate(f.right, interp, env)
    if isinstance(f, Or):    return evaluate(f.left, interp, env) or evaluate(f.right, interp, env)
    if isinstance(f, Imp):   return (not evaluate(f.left, interp, env)) or evaluate(f.right, interp, env)
    if isinstance(f, Iff):   return evaluate(f.left, interp, env) == evaluate(f.right, interp, env)
    if isinstance(f, ForAll):
        return all(evaluate(f.body, interp, {**env, f.var: d}) for d in interp.domain)
    if isinstance(f, Exists):
        return any(evaluate(f.body, interp, {**env, f.var: d}) for d in interp.domain)
    raise TypeError(f)


def witness_for_failure(f: Formula, interp: Interpretation,
                         env: Dict[str, object]) -> Optional[Dict[str, object]]:
    """If f is `ForAll(x, body)`, find the first x that falsifies body.
    If f is `Exists(x, body)` and the formula is True, find a witness.
    Returns None if no witness is meaningful (e.g., quantifier-free root)."""
    if isinstance(f, ForAll):
        for d in interp.domain:
            new_env = {**env, f.var: d}
            if not evaluate(f.body, interp, new_env):
                return {f.var: d}
        return None
    if isinstance(f, Exists):
        for d in interp.domain:
            new_env = {**env, f.var: d}
            if evaluate(f.body, interp, new_env):
                return {f.var: d}
        return None
    return None


def check(f: Formula, interp: Interpretation, env: Dict[str, object] | None = None) -> None:
    env = env or {}
    val = evaluate(f, interp, env)
    print(f"value: {val}")
    w = witness_for_failure(f, interp, env)
    if w is not None:
        kind = "counterexample" if isinstance(f, ForAll) else "witness"
        print(f"{kind}: {w}")


if __name__ == "__main__":
    # Demo: same as main.py, but reusable as a library.
    interp = Interpretation(
        domain=list(range(0, 30)),
        predicates={
            "Even": lambda x: x % 2 == 0,
            "Gt":   lambda x, y: x > y,
        },
    )
    print("(demo) Every x has an x+1 in the domain that's greater than x:")
    check(ForAll("x", Exists("y", PredVar("Gt", ("y", "x")))), interp)
