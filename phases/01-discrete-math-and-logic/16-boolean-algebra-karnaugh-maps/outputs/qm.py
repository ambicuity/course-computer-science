"""qm.py — Quine-McCluskey minimization for Boolean functions.

Used in Phase 06 (digital logic) and Phase 08 (compiler short-circuiting).
"""
from __future__ import annotations

from itertools import product
from typing import Callable, List, Tuple


def truth_minterms(f: Callable[..., int], n_vars: int) -> List[int]:
    """Returns the indices of input rows where f returns 1."""
    return [
        int("".join(map(str, bits)), 2)
        for bits in product([0, 1], repeat=n_vars)
        if f(*bits)
    ]


def differ_by_one(a: str, b: str) -> int:
    diffs = [i for i in range(len(a)) if a[i] != b[i]]
    return diffs[0] if len(diffs) == 1 else -1


def quine_mccluskey(minterms: List[int], n_vars: int) -> List[str]:
    current = {format(m, f"0{n_vars}b") for m in minterms}
    primes = set()
    while True:
        next_set = set()
        used = set()
        cur = list(current)
        for i in range(len(cur)):
            for j in range(i + 1, len(cur)):
                if [k for k, c in enumerate(cur[i]) if c == "-"] != \
                   [k for k, c in enumerate(cur[j]) if c == "-"]:
                    continue
                pos = differ_by_one(cur[i], cur[j])
                if pos >= 0:
                    next_set.add(cur[i][:pos] + "-" + cur[i][pos + 1:])
                    used.add(cur[i])
                    used.add(cur[j])
        for term in current:
            if term not in used:
                primes.add(term)
        if not next_set:
            break
        current = next_set
    return sorted(primes)


def cover(implicants: List[str], minterms: List[int], n_vars: int) -> List[str]:
    remaining = set(minterms)
    chosen: List[str] = []

    def covers(imp: str, m: int) -> bool:
        bits = format(m, f"0{n_vars}b")
        return all(c == "-" or c == b for c, b in zip(imp, bits))

    while remaining:
        best, best_n = None, -1
        for imp in implicants:
            n = sum(1 for m in remaining if covers(imp, m))
            if n > best_n:
                best, best_n = imp, n
        chosen.append(best)
        remaining = {m for m in remaining if not covers(best, m)}
    return chosen


def implicant_to_term(implicant: str, var_names: List[str]) -> str:
    parts = []
    for v, c in zip(var_names, implicant):
        if c == "-":
            continue
        parts.append(f"¬{v}" if c == "0" else v)
    return "·".join(parts) if parts else "1"


def minimize(f: Callable[..., int], var_names: List[str]) -> str:
    n = len(var_names)
    minterms = truth_minterms(f, n)
    if not minterms:
        return "0"
    primes = quine_mccluskey(minterms, n)
    cov = cover(primes, minterms, n)
    return " + ".join(implicant_to_term(c, var_names) for c in cov)


if __name__ == "__main__":
    def majority(a, b, c): return 1 if a + b + c >= 2 else 0
    result = minimize(majority, ["a", "b", "c"])
    assert result.count("+") == 2  # three terms
    print(f"qm library: majority(a,b,c) minimized → {result}")
