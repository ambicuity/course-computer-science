#!/usr/bin/env python3
from __future__ import annotations


def transfer(src_balance: int, dst_balance: int, amount: int) -> tuple[int, int]:
    if amount <= 0:
        raise ValueError("precondition: amount > 0")
    if src_balance < amount:
        raise ValueError("precondition: source has enough funds")

    new_src = src_balance - amount
    new_dst = dst_balance + amount

    if new_src < 0:
        raise AssertionError("invariant violated: src balance non-negative")
    if new_src + new_dst != src_balance + dst_balance:
        raise AssertionError("postcondition violated: conservation")

    return new_src, new_dst


def main() -> None:
    print(transfer(100, 20, 30))


if __name__ == "__main__":
    main()
