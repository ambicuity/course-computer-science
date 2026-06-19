#!/usr/bin/env python3
from __future__ import annotations


def classify(x: int) -> str:
    if x < 0:
        return "neg"
    if x == 0:
        return "zero"
    if x < 10:
        return "small"
    return "large"


def sparse_tests() -> bool:
    return classify(0) == "zero" and classify(15) == "large"


def richer_tests() -> bool:
    return (
        classify(-1) == "neg"
        and classify(0) == "zero"
        and classify(5) == "small"
        and classify(15) == "large"
    )


def main() -> None:
    print(f"sparse_tests={sparse_tests()}")
    print(f"richer_tests={richer_tests()}")
    print("note: both may execute many lines, but only richer tests cover all branch outcomes")


if __name__ == "__main__":
    main()
