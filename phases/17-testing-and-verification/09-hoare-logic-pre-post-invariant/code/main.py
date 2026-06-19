#!/usr/bin/env python3
from __future__ import annotations


def sum_upto(n: int) -> int:
    if n < 0:
        raise ValueError("precondition: n >= 0")

    i = 0
    acc = 0
    while i <= n:
        # Invariant: acc == sum(0..i-1), with 0 <= i <= n+1
        assert 0 <= i <= n + 1
        expected = i * (i - 1) // 2
        assert acc == expected

        acc += i
        i += 1

    # Postcondition: acc == sum(0..n)
    assert acc == n * (n + 1) // 2
    return acc


def main() -> None:
    for n in [0, 1, 5, 10]:
        print(n, sum_upto(n))


if __name__ == "__main__":
    main()
