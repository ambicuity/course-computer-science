"""Boolean algebra demos: truth-table → SOP, K-map ASCII, Quine-McCluskey.

Run:  python3 main.py
"""
from __future__ import annotations

from itertools import product
from typing import Callable, List, Tuple


# ── Truth table → SOP ─────────────────────────────────────────────

def truth_table(f: Callable[..., int], n_vars: int) -> List[Tuple[Tuple[int, ...], int]]:
    return [(bits, f(*bits)) for bits in product([0, 1], repeat=n_vars)]


def sop_from_table(table, var_names) -> str:
    terms = []
    for bits, val in table:
        if val == 1:
            term = "·".join(f"¬{v}" if b == 0 else v for v, b in zip(var_names, bits))
            terms.append(term)
    return " + ".join(terms) if terms else "0"


# ── Quine-McCluskey ───────────────────────────────────────────────

def differ_by_one(a: str, b: str) -> int:
    diffs = [i for i in range(len(a)) if a[i] != b[i]]
    return diffs[0] if len(diffs) == 1 else -1


def combine(a: str, b: str, pos: int) -> str:
    return a[:pos] + "-" + a[pos + 1:]


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
                    next_set.add(combine(cur[i], cur[j], pos))
                    used.add(cur[i]); used.add(cur[j])
        for term in current:
            if term not in used:
                primes.add(term)
        if not next_set:
            break
        current = next_set
    return sorted(primes)


def implicant_to_term(implicant: str, var_names: List[str]) -> str:
    parts = []
    for v, c in zip(var_names, implicant):
        if c == "-":
            continue
        parts.append(f"¬{v}" if c == "0" else v)
    return "·".join(parts) if parts else "1"


def cover_minterms(implicants: List[str], minterms: List[int], n_vars: int) -> List[str]:
    remaining = set(minterms)
    chosen: List[str] = []

    def covers(imp: str, m: int) -> bool:
        bits = format(m, f"0{n_vars}b")
        return all(c == "-" or c == b for c, b in zip(imp, bits))

    while remaining:
        best, best_count = None, -1
        for imp in implicants:
            count = sum(1 for m in remaining if covers(imp, m))
            if count > best_count:
                best, best_count = imp, count
        chosen.append(best)
        remaining = {m for m in remaining if not covers(best, m)}
    return chosen


def minimize(f: Callable[..., int], var_names: List[str]) -> str:
    n = len(var_names)
    table = truth_table(f, n)
    minterms = [int("".join(map(str, bits)), 2) for bits, val in table if val == 1]
    if not minterms:
        return "0"
    primes = quine_mccluskey(minterms, n)
    cover = cover_minterms(primes, minterms, n)
    return " + ".join(implicant_to_term(c, var_names) for c in cover)


# ── 3-variable K-map ASCII ────────────────────────────────────────

def kmap_3(f, var_names):
    a_label, b_label, c_label = var_names
    print(f"  K-map of f({a_label}, {b_label}, {c_label}):")
    print(f"           {b_label}{c_label}=00  01   11   10")
    for a in [0, 1]:
        row = []
        for bc in [(0, 0), (0, 1), (1, 1), (1, 0)]:
            row.append(str(f(a, *bc)))
        print(f"    {a_label}={a} |    {row[0]}    {row[1]}    {row[2]}    {row[3]}")


# ── Demo ──────────────────────────────────────────────────────────

def main():
    print("== Majority of 3 ==")
    def majority(a, b, c): return 1 if a + b + c >= 2 else 0
    var_names = ["a", "b", "c"]
    table = truth_table(majority, 3)
    print("  Truth table:")
    for bits, val in table:
        print(f"    {bits} → {val}")
    print(f"\n  Canonical SOP:  {sop_from_table(table, var_names)}")
    print()
    kmap_3(majority, var_names)
    print(f"\n  Minimized via QM: {minimize(majority, var_names)}    (expected: a·b + a·c + b·c)")

    print("\n== a·b + a·c + a·d ==")
    def f1(a, b, c, d): return 1 if ((a and b) or (a and c) or (a and d)) else 0
    print(f"  Minimized: {minimize(f1, ['a', 'b', 'c', 'd'])}")

    print("\n== Exactly two of four ==")
    def exactly_two(a, b, c, d): return 1 if (a + b + c + d) == 2 else 0
    result = minimize(exactly_two, ['a', 'b', 'c', 'd'])
    print(f"  Minimized: {result}")
    print(f"  Number of terms: {result.count('+') + 1}    (expected C(4,2) = 6)")


if __name__ == "__main__":
    main()
