#!/usr/bin/env python3
from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Var:
    n: str


@dataclass(frozen=True)
class Lam:
    p: str
    b: object


@dataclass(frozen=True)
class App:
    f: object
    a: object


def subst(t, x: str, v):
    if isinstance(t, Var):
        return v if t.n == x else t
    if isinstance(t, Lam):
        if t.p == x:
            return t
        return Lam(t.p, subst(t.b, x, v))
    if isinstance(t, App):
        return App(subst(t.f, x, v), subst(t.a, x, v))
    return t


def step(t):
    if isinstance(t, App) and isinstance(t.f, Lam):
        return subst(t.f.b, t.f.p, t.a)
    if isinstance(t, App):
        nf = step(t.f)
        if nf != t.f:
            return App(nf, t.a)
        na = step(t.a)
        return App(t.f, na)
    return t


def normalize(t):
    while True:
        nt = step(t)
        if nt == t:
            return t
        t = nt


def main() -> None:
    term = App(Lam("x", Var("x")), Var("y"))
    print(normalize(term))


if __name__ == "__main__":
    main()
