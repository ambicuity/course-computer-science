#!/usr/bin/env python3
from __future__ import annotations


def fee(amount: int) -> int:
    if amount <= 0:
        return 0
    if amount < 100:
        return 5
    return amount // 20


def mutant_lt_to_le(amount: int) -> int:
    if amount <= 0:
        return 0
    if amount <= 100:
        return 5
    return amount // 20


def mutant_plus(amount: int) -> int:
    if amount <= 0:
        return 0
    if amount < 100:
        return 5
    return amount // 20 + 1


def tests(fn) -> bool:
    return (
        fn(-1) == 0
        and fn(0) == 0
        and fn(50) == 5
        and fn(100) == 5
        and fn(200) == 10
    )


def run() -> None:
    mutants = {
        "original": fee,
        "mutant_lt_to_le": mutant_lt_to_le,
        "mutant_plus": mutant_plus,
    }
    killed = 0
    total = 0
    for name, fn in mutants.items():
        ok = tests(fn)
        if name != "original":
            total += 1
            if not ok:
                killed += 1
        print(f"{name}: {'PASS' if ok else 'FAIL'}")
    print(f"mutation_score={killed}/{total}")


if __name__ == "__main__":
    run()
