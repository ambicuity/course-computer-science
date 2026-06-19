#!/usr/bin/env python3
from __future__ import annotations


def target(x: int, y: int) -> str:
    if x > 3:
        if y == x + 1:
            return "path_A"
        if y < 0:
            return "path_B"
    else:
        if x + y == 0:
            return "path_C"
    return "path_D"


def find_witness(name: str) -> tuple[int, int] | None:
    for x in range(-5, 8):
        for y in range(-5, 8):
            if target(x, y) == name:
                return (x, y)
    return None


def main() -> None:
    for p in ["path_A", "path_B", "path_C", "path_D"]:
        w = find_witness(p)
        print(p, w)


if __name__ == "__main__":
    main()
