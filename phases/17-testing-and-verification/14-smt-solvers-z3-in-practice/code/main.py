#!/usr/bin/env python3
from __future__ import annotations


def feasible_schedule() -> tuple[bool, dict[str, int]]:
    # A tiny hand-rolled solver-like search for demonstration when z3 is unavailable.
    for a in range(1, 6):
        for b in range(1, 6):
            for c in range(1, 6):
                if a + b + c > 8:
                    continue
                if not (a <= b <= c):
                    continue
                return True, {"A": a, "B": b, "C": c}
    return False, {}


def infeasible_constraints() -> bool:
    # Encodes contradiction: x > 5 and x < 3 over integers.
    for x in range(-10, 11):
        if x > 5 and x < 3:
            return True
    return False


def main() -> None:
    sat, model = feasible_schedule()
    print("sat" if sat else "unsat", model)
    print("unsat_example", not infeasible_constraints())


if __name__ == "__main__":
    main()
