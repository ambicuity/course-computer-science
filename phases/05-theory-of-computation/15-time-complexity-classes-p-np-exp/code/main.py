"""Lesson 15: Time Complexity Classes — P, NP, EXP
Phase 05 — Theory of Computation

Verifiers for NP problems, brute-force solver, empirical complexity check.
"""

from __future__ import annotations

import math
import time
from itertools import product


# ---------------------------------------------------------------------------
# 1. NP verifiers
# ---------------------------------------------------------------------------

def sat_verifier(assignment: dict[str, bool],
                 formula: list[list[tuple[str, bool]]]) -> bool:
    """Verify a SAT certificate against a CNF formula.

    formula is a list of clauses; each clause is a list of (var, is_positive).
    """
    for clause in formula:
        if not any((assignment.get(var, False) if pos else
                    not assignment.get(var, False))
                   for var, pos in clause):
            return False
    return True


def hamiltonian_path_verifier(path: list[int],
                              graph: dict[int, list[int]]) -> bool:
    """Verify a Hamiltonian path in a graph."""
    n = len(graph)
    if len(path) != n or len(set(path)) != n:
        return False
    for i in range(len(path) - 1):
        if path[i + 1] not in graph.get(path[i], []):
            return False
    return True


def clique_verifier(vertices: list[int], k: int,
                    graph: dict[int, list[int]]) -> bool:
    """Verify a k-clique in a graph."""
    if len(vertices) != k:
        return False
    for i, u in enumerate(vertices):
        for v in vertices[i + 1:]:
            if v not in graph.get(u, []):
                return False
    return True


def subset_sum_verifier(subset: list[int],
                        numbers: list[int],
                        target: int) -> bool:
    """Verify a subset-sum certificate."""
    return all(s in numbers for s in subset) and sum(subset) == target


def graph_coloring_verifier(coloring: dict[int, int],
                            k: int,
                            graph: dict[int, list[int]]) -> bool:
    """Verify a k-coloring of a graph."""
    for u, neighbors in graph.items():
        if coloring.get(u, -1) < 0 or coloring[u] >= k:
            return False
        for v in neighbors:
            if coloring.get(u) == coloring.get(v):
                return False
    return True


# ---------------------------------------------------------------------------
# 2. Brute-force SAT solver (exponential)
# ---------------------------------------------------------------------------

def brute_force_sat(formula: list[list[tuple[str, bool]]],
                    variables: list[str]) -> dict[str, bool] | None:
    """Exhaustive SAT solver — O(2^n) in the number of variables."""
    for values in product([False, True], repeat=len(variables)):
        assignment = dict(zip(variables, values))
        if sat_verifier(assignment, formula):
            return assignment
    return None


# ---------------------------------------------------------------------------
# 3. Empirical complexity checker
# ---------------------------------------------------------------------------

def is_in_p(algorithm, input_sizes: list[int],
            max_time: float = 5.0) -> bool:
    """Empirically test if *algorithm(n)* grows polynomially.

    Returns True if the estimated exponent is reasonable (< 10),
    False if growth appears exponential or exceeds *max_time*.
    """
    times: list[float] = []
    for n in input_sizes:
        start = time.perf_counter()
        algorithm(n)
        elapsed = time.perf_counter() - start
        times.append(elapsed)
        print(f"  n={n:>6}  time={elapsed:.6f}s")
        if elapsed > max_time:
            print(f"  Exceeded {max_time}s — likely super-polynomial.")
            return False

    # Estimate exponent: T(n) ≈ c · n^k  →  log(T) = k·log(n) + log(c)
    exponents: list[float] = []
    for i in range(1, len(times)):
        if times[i] > 0 and times[i - 1] > 0:
            log_n = math.log(input_sizes[i] / input_sizes[i - 1])
            log_t = math.log(times[i] / times[i - 1])
            if log_n > 0:
                exponents.append(log_t / log_n)

    if exponents:
        avg = sum(exponents) / len(exponents)
        print(f"  Estimated exponent: O(n^{avg:.1f})")
        return avg < 10

    return True


# ---------------------------------------------------------------------------
# 4. Demonstration
# ---------------------------------------------------------------------------

def demonstrate_verification_gap() -> None:
    """Compare verification (fast) vs brute-force solving (exponential)."""
    print("Verification vs Solution Gap for SAT")
    print("=" * 45)

    variables = ["x1", "x2", "x3", "x4", "x5"]
    formula = [
        [("x1", True), ("x2", False)],
        [("x2", True), ("x3", True)],
        [("x3", False), ("x4", True)],
        [("x4", False), ("x5", False)],
        [("x1", True), ("x5", True)],
    ]

    cert = {"x1": True, "x2": False, "x3": True, "x4": True, "x5": True}
    start = time.perf_counter()
    ok = sat_verifier(cert, formula)
    v_time = time.perf_counter() - start
    print(f"Verification: {ok} in {v_time:.8f}s")

    start = time.perf_counter()
    sol = brute_force_sat(formula, variables)
    s_time = time.perf_counter() - start
    print(f"Brute-force:  {sol} in {s_time:.8f}s")
    if v_time > 0:
        print(f"Ratio: ~{s_time / v_time:.0f}x\n")
    else:
        print()


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    # --- Verifier demos ---
    print("=== SAT Verifier ===")
    formula = [
        [("x1", True), ("x2", False)],
        [("x2", True), ("x3", True)],
    ]
    good = {"x1": True, "x2": True, "x3": False}
    bad = {"x1": False, "x2": False, "x3": False}
    print(f"  Good assignment: {sat_verifier(good, formula)}")  # True
    print(f"  Bad assignment:  {sat_verifier(bad, formula)}")   # False

    print("\n=== Hamiltonian Path Verifier ===")
    graph = {0: [1, 2], 1: [0, 2, 3], 2: [0, 1, 3], 3: [1, 2]}
    print(f"  [0,1,3,2]: {hamiltonian_path_verifier([0, 1, 3, 2], graph)}")
    print(f"  [0,1,2]:   {hamiltonian_path_verifier([0, 1, 2], graph)}")

    print("\n=== Clique Verifier ===")
    print(f"  clique [0,1,2], k=3: {clique_verifier([0, 1, 2], 3, graph)}")

    print("\n=== Subset Sum Verifier ===")
    print(f"  subset [3,7], target=10: {subset_sum_verifier([3, 7], [1, 3, 5, 7], 10)}")

    demonstrate_verification_gap()

    # --- Empirical check ---
    print("=== Empirical Complexity Check ===")
    print("Trial division (polynomial):")
    is_in_p(lambda n: [d for d in range(2, min(n, 10000))
                        if n % d == 0],
            [1000, 5000, 10000, 50000, 100000])


if __name__ == "__main__":
    main()
