"""NP-Completeness: Cook-Levin, reductions, and verification toolkit.

Run:  python3 main.py
"""
from __future__ import annotations

import itertools
from typing import Dict, FrozenSet, List, Optional, Set, Tuple

# Type aliases
Literal = Tuple[str, bool]  # (variable_name, is_negated)
Clause = FrozenSet[Literal]
Formula = List[Clause]


# ── Brute-Force SAT Solver ──────────────────────────────────────────

def brute_force_sat(formula: Formula) -> Optional[Dict[str, bool]]:
    """Try all 2^n assignments. Return a satisfying assignment or None."""
    variables: Set[str] = set()
    for clause in formula:
        for var, _ in clause:
            variables.add(var)
    var_list = sorted(variables)

    for bits in itertools.product([False, True], repeat=len(var_list)):
        assignment = dict(zip(var_list, bits))
        if all(
            any(assignment[v] != neg for v, neg in clause)
            for clause in formula
        ):
            return assignment
    return None


# ── SAT → 3-SAT Reduction ──────────────────────────────────────────

def sat_to_3sat(formula: Formula) -> Formula:
    """Reduce arbitrary CNF to 3-CNF using auxiliary variables."""
    new_clauses: Formula = []
    aux_counter = 0

    for clause in formula:
        literals = list(clause)
        if len(literals) == 3:
            new_clauses.append(frozenset(literals))
            continue
        if len(literals) < 3:
            # Pad to exactly 3 by duplicating last literal
            padded = literals + [literals[-1]] * (3 - len(literals))
            new_clauses.append(frozenset(padded))
            continue

        # Split clause (l1 ∨ l2 ∨ ... ∨ lk) with k > 3:
        # (l1 ∨ l2 ∨ z1) ∧ (¬z1 ∨ l3 ∨ z2) ∧ ... ∧ (¬z_{k-3} ∨ l_{k-1} ∨ lk)
        prev_aux: Optional[str] = None
        for i in range(len(literals) - 3):
            aux = f"_aux{aux_counter}"
            aux_counter += 1
            if i == 0:
                new_clauses.append(frozenset([
                    literals[0], literals[1], (aux, False)
                ]))
            else:
                new_clauses.append(frozenset([
                    (prev_aux, True), literals[i + 1], (aux, False)
                ]))
            prev_aux = aux

        new_clauses.append(frozenset([
            (prev_aux, True), literals[-2], literals[-1]
        ]))

    return new_clauses


# ── 3-SAT → Vertex Cover Reduction ─────────────────────────────────

Edge = Tuple[Tuple[int, int], Tuple[int, int]]

def three_sat_to_vc(clauses: Formula) -> Tuple[List[Edge], int]:
    """Reduce 3-SAT to Vertex Cover.

    Returns (edges, k) where k is the target cover size.
    Each clause becomes a triangle; cross-clause edges link (v, pos) <-> (v, neg).
    """
    edges: List[Edge] = []
    var_positions: Dict[Tuple[str, bool], List[Tuple[int, int]]] = {}

    for ci, clause in enumerate(clauses):
        literals = list(clause)
        # Pad short clauses to exactly 3 by duplicating literals
        if len(literals) < 3:
            literals = literals + [literals[-1]] * (3 - len(literals))
        assert len(literals) == 3, f"Clause {ci} has {len(literals)} literals, expected 3"

        nodes = [(ci, j) for j in range(3)]
        # Triangle edges
        edges.append((nodes[0], nodes[1]))
        edges.append((nodes[1], nodes[2]))
        edges.append((nodes[2], nodes[0]))

        for j, (v, neg) in enumerate(literals):
            key = (v, neg)
            var_positions.setdefault(key, []).append(nodes[j])

    # Cross-clause edges
    vars_set: Set[str] = set()
    for clause in clauses:
        for v, _ in clause:
            vars_set.add(v)

    for v in vars_set:
        pos_nodes = var_positions.get((v, False), [])
        neg_nodes = var_positions.get((v, True), [])
        for pn in pos_nodes:
            for nn in neg_nodes:
                edges.append((pn, nn))

    n_vars = len(vars_set)
    k = n_vars + 2 * len(clauses)
    return edges, k


# ── Reduction Verifier ──────────────────────────────────────────────

def verify_reduction(
    sat_formula: Formula,
    vc_edges: List[Edge],
    vc_k: int,
    assignment: Dict[str, bool],
) -> bool:
    """Verify: if assignment satisfies the SAT formula, construct a vertex cover of size <= k."""
    selected: Set[Tuple[int, int]] = set()

    for ci, clause in enumerate(sat_formula):
        literals = list(clause)
        # Find the satisfied literal index (if any)
        satisfied_idx = None
        for j, (v, neg) in enumerate(literals):
            if assignment[v] != neg:
                satisfied_idx = j
                break

        # Pick nodes to cover the triangle: need at least 2 of 3
        if satisfied_idx is not None:
            # Pick the satisfied node + one other
            selected.add((ci, satisfied_idx))
            for j in range(3):
                if j != satisfied_idx and (ci, j) not in selected:
                    selected.add((ci, j))
                    break
        else:
            # Clause not satisfied — pick any 2
            selected.add((ci, 0))
            selected.add((ci, 1))

    # Verify all edges covered
    covered = all(u in selected or v in selected for u, v in vc_edges)
    return len(selected) <= vc_k and covered


# ── Display Helpers ─────────────────────────────────────────────────

def fmt_formula(formula: Formula) -> str:
    """Pretty-print a CNF formula."""
    parts = []
    for clause in formula:
        lits = []
        for v, neg in sorted(clause):
            lits.append(f"¬{v}" if neg else v)
        parts.append(f"({' ∨ '.join(lits)})")
    return " ∧ ".join(parts)


def fmt_assignment(assignment: Dict[str, bool]) -> str:
    """Pretty-print a variable assignment."""
    return ", ".join(f"{v}={'T' if val else 'F'}" for v, val in sorted(assignment.items()))


# ── Demonstration ───────────────────────────────────────────────────

def demo_sat_solver() -> None:
    print("=" * 60)
    print("DEMO 1: Brute-Force SAT Solver")
    print("=" * 60)

    # (A ∨ ¬B) ∧ (¬A ∨ C) ∧ (B ∨ ¬C)
    formula = [
        frozenset([("A", False), ("B", True)]),
        frozenset([("A", True), ("C", False)]),
        frozenset([("B", False), ("C", True)]),
    ]

    print(f"\nFormula: {fmt_formula(formula)}")
    result = brute_force_sat(formula)
    if result:
        print(f"SAT — satisfying assignment: {fmt_assignment(result)}")
    else:
        print("UNSAT — no satisfying assignment exists")

    # Unsatisfiable formula: (A) ∧ (¬A) ∧ (B) ∧ (¬B)
    unsat_formula = [
        frozenset([("A", False)]),
        frozenset([("A", True)]),
        frozenset([("B", False)]),
        frozenset([("B", True)]),
    ]
    print(f"\nFormula: {fmt_formula(unsat_formula)}")
    result2 = brute_force_sat(unsat_formula)
    print(f"Result: {'SAT — ' + fmt_assignment(result2) if result2 else 'UNSAT'}")


def demo_sat_to_3sat() -> None:
    print("\n" + "=" * 60)
    print("DEMO 2: SAT → 3-SAT Reduction")
    print("=" * 60)

    # A clause with 5 literals: (A ∨ B ∨ C ∨ D ∨ ¬E)
    long_clause = frozenset([
        ("A", False), ("B", False), ("C", False),
        ("D", False), ("E", True),
    ])
    formula = [long_clause]

    print(f"\nOriginal clause: {fmt_formula(formula)}")
    reduced = sat_to_3sat(formula)
    print(f"3-SAT form:      {fmt_formula(reduced)}")
    print(f"Original clauses: 1 → 3-SAT clauses: {len(reduced)}")

    # Verify equivalence: both should have same satisfiability
    orig_result = brute_force_sat(formula)
    reduced_result = brute_force_sat(reduced)
    print(f"\nOriginal SAT? {orig_result is not None}")
    print(f"3-SAT SAT?    {reduced_result is not None}")
    print(f"Satisfiability preserved: {(orig_result is not None) == (reduced_result is not None)}")


def demo_3sat_to_vc() -> None:
    print("\n" + "=" * 60)
    print("DEMO 3: 3-SAT → Vertex Cover Reduction")
    print("=" * 60)

    # 3-SAT: (A ∨ B ∨ ¬C) ∧ (¬A ∨ C ∨ D)
    clauses = [
        frozenset([("A", False), ("B", False), ("C", True)]),
        frozenset([("A", True), ("C", False), ("D", False)]),
    ]

    print(f"\n3-SAT formula: {fmt_formula(clauses)}")
    edges, k = three_sat_to_vc(clauses)
    print(f"Vertex Cover instance: {len(edges)} edges, k = {k}")
    print(f"Edges:")
    for u, v in edges:
        print(f"  {u} — {v}")

    # Find a satisfying assignment
    assignment = brute_force_sat(clauses)
    if assignment:
        print(f"\nSatisfying assignment: {fmt_assignment(assignment)}")
        valid = verify_reduction(clauses, edges, k, assignment)
        print(f"Reduction verified (cover exists with size ≤ {k}): {valid}")


def demo_reduction_chain() -> None:
    print("\n" + "=" * 60)
    print("DEMO 4: Full Reduction Chain — SAT → 3-SAT → Vertex Cover")
    print("=" * 60)

    # Start with arbitrary SAT: (A ∨ B ∨ C ∨ D) ∧ (¬A ∨ ¬B)
    original = [
        frozenset([("A", False), ("B", False), ("C", False), ("D", False)]),
        frozenset([("A", True), ("B", True)]),
    ]

    print(f"\nStep 0 — Original SAT: {fmt_formula(original)}")

    # Step 1: SAT → 3-SAT
    three_sat = sat_to_3sat(original)
    print(f"\nStep 1 — 3-SAT: {fmt_formula(three_sat)}")

    # Step 2: 3-SAT → Vertex Cover
    edges, k = three_sat_to_vc(three_sat)
    print(f"\nStep 2 — Vertex Cover: {len(edges)} edges, k = {k}")

    # Verify chain
    sat_result = brute_force_sat(original)
    tsat_result = brute_force_sat(three_sat)

    print(f"\nSatisfiability preserved through chain:")
    print(f"  Original SAT: {sat_result is not None}")
    print(f"  3-SAT:        {tsat_result is not None}")
    print(f"  Chain valid:  {(sat_result is not None) == (tsat_result is not None)}")

    if sat_result:
        print(f"\n  Assignment: {fmt_assignment(sat_result)}")


# ── Main ────────────────────────────────────────────────────────────

def main() -> None:
    demo_sat_solver()
    demo_sat_to_3sat()
    demo_3sat_to_vc()
    demo_reduction_chain()


if __name__ == "__main__":
    main()
