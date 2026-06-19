#!/usr/bin/env python3
from __future__ import annotations

from collections import Counter


def buggy_sort(values: list[int]) -> list[int]:
    out = sorted(values)
    if len(out) >= 2 and out[0] == out[1]:
        return out[1:]
    return out


def property_sortedness(values: list[int]) -> bool:
    out = buggy_sort(values)
    return all(out[i] <= out[i + 1] for i in range(len(out) - 1))


def property_permutation(values: list[int]) -> bool:
    return Counter(values) == Counter(buggy_sort(values))


def property_idempotent(values: list[int]) -> bool:
    return buggy_sort(buggy_sort(values)) == buggy_sort(values)


def run_manual_search() -> None:
    candidates = [
        [],
        [1],
        [2, 1],
        [0, 0],
        [4, 4, 3],
        [9, 2, 2, 1],
        [5, 4, 3, 2, 1],
    ]

    for case in candidates:
        s_ok = property_sortedness(case)
        p_ok = property_permutation(case)
        i_ok = property_idempotent(case)
        print(f"case={case} sortedness={s_ok} permutation={p_ok} idempotent={i_ok}")


if __name__ == "__main__":
    run_manual_search()
