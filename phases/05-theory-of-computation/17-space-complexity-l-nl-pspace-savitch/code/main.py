"""Space Complexity: L, NL, PSPACE, Savitch's theorem.

Run:  python3 main.py
"""
from __future__ import annotations

import math
from typing import Any, Dict, List, Optional, Set, Tuple


# ── Space-Bounded TM Simulator ─────────────────────────────────────

def logspace_tm_simulator(
    tm: Dict[str, Any],
    input_str: str,
    space_bound: int,
    max_steps: int = 10000,
) -> Tuple[bool, int]:
    """Simulate a TM with a bounded work tape.

    The input tape is read-only. The work tape has `space_bound` cells.
    Returns (accepted, max_cells_used).
    """
    tape = list(input_str) + ["_"]  # extra blank for readability
    work_tape = ["_"] * space_bound
    head = 0
    work_head = 0
    state = tm["start"]
    max_cells = 0

    for step in range(max_steps):
        if state == tm.get("accept"):
            return True, max_cells
        if state == tm.get("reject"):
            return False, max_cells

        read_input = tape[head] if 0 <= head < len(tape) else "_"
        read_work = work_tape[work_head]
        key = (state, read_input, read_work)

        if key not in tm.get("transitions", {}):
            return False, max_cells

        new_state, write_work, input_dir, work_dir = tm["transitions"][key]
        work_tape[work_head] = write_work

        # Move input head
        if input_dir == "R":
            head += 1
        elif input_dir == "L":
            head = max(head - 1, 0)

        # Move work head
        if work_dir == "R":
            work_head += 1
            if work_head >= space_bound:
                return False, space_bound
        elif work_dir == "L":
            work_head = max(work_head - 1, 0)

        max_cells = max(max_cells, work_head + 1)
        state = new_state

    return False, max_cells


# ── Savitch's Reachability Algorithm ────────────────────────────────

def reachability_savitch(
    graph: Dict[int, List[int]],
    s: int,
    t: int,
    n: int,
) -> bool:
    """Deterministic reachability in O(log² n) space.

    Divide-and-conquer: is there a path from u to v using ≤ 2^steps edges?
    Each recursive frame stores a node ID (O(log n) bits).
    Recursion depth is O(log n), total space O(log² n).
    """
    memo: Dict[Tuple[int, int, int], bool] = {}

    def reachable(u: int, v: int, steps: int, depth: int = 0) -> bool:
        key = (u, v, steps)
        if key in memo:
            return memo[key]

        if steps == 0:
            result = u == v
        elif steps == 1:
            result = u == v or v in graph.get(u, [])
        else:
            result = False
            for m in range(n):
                if reachable(u, m, steps // 2, depth + 1) and \
                   reachable(m, v, steps - steps // 2, depth + 1):
                    result = True
                    break

        memo[key] = result
        return result

    return reachable(s, t, n)


def reachability_savitch_space_tracking(
    graph: Dict[int, List[int]],
    s: int,
    t: int,
    n: int,
) -> Tuple[bool, int, int]:
    """Same as reachability_savitch but tracks space usage.

    Returns (is_reachable, max_recursion_depth, peak_space_bytes).
    """
    peak_depth = 0
    call_count = 0

    def reachable(u: int, v: int, steps: int, depth: int = 0) -> bool:
        nonlocal peak_depth, call_count
        call_count += 1
        peak_depth = max(peak_depth, depth)

        if steps == 0:
            return u == v
        if steps == 1:
            return u == v or v in graph.get(u, [])

        for m in range(n):
            if reachable(u, m, steps // 2, depth + 1) and \
               reachable(m, v, steps - steps // 2, depth + 1):
                return True
        return False

    result = reachable(s, t, n)
    # Each frame uses O(log n) bits ≈ math.ceil(math.log2(max(n, 2)))
    bits_per_frame = math.ceil(math.log2(max(n, 2)))
    peak_space_bits = peak_depth * bits_per_frame
    return result, peak_depth, call_count


# ── TQBF Evaluator ─────────────────────────────────────────────────

def tqbf_evaluator(formula: Dict[str, Any]) -> bool:
    """Evaluate a quantified Boolean formula.

    formula = {
        "quantifiers": [("x1", "forall"), ("x2", "exists"), ...],
        "body": lambda env: <bool expression over env["x1"], env["x2"], ...>
    }
    """
    quantifiers = formula["quantifiers"]
    body = formula["body"]

    def eval_inner(remaining: List[Tuple[str, str]], env: Dict[str, bool]) -> bool:
        if not remaining:
            return body(env)

        var, qtype = remaining[0]
        rest = remaining[1:]

        if qtype == "forall":
            return (eval_inner(rest, {**env, var: True}) and
                    eval_inner(rest, {**env, var: False}))
        else:  # exists
            return (eval_inner(rest, {**env, var: True}) or
                    eval_inner(rest, {**env, var: False}))

    return eval_inner(quantifiers, {})


# ── Display Helpers ─────────────────────────────────────────────────

def fmt_graph(graph: Dict[int, List[int]]) -> str:
    lines = []
    for node in sorted(graph):
        for neighbor in graph[node]:
            lines.append(f"  {node} → {neighbor}")
    return "\n".join(lines) if lines else "  (empty)"


# ── Demonstrations ──────────────────────────────────────────────────

def demo_space_bounded_tm() -> None:
    print("=" * 60)
    print("DEMO 1: Space-Bounded TM Simulator")
    print("=" * 60)

    # TM that checks if input has even number of 1s (uses 1 work cell)
    tm = {
        "start": "q0",
        "accept": "q_accept",
        "reject": "q_reject",
        "transitions": {
            # State, read_input, read_work -> new_state, write_work, input_dir, work_dir
            ("q0", "0", "_"): ("q0", "_", "R", "S"),
            ("q0", "1", "_"): ("q1", "_", "R", "S"),
            ("q0", "_", "_"): ("q_accept", "_", "S", "S"),
            ("q1", "0", "_"): ("q1", "_", "R", "S"),
            ("q1", "1", "_"): ("q0", "_", "R", "S"),
            ("q1", "_", "_"): ("q_reject", "_", "S", "S"),
        },
    }

    for test_input in ["", "0", "1", "11", "101", "111", "000", "1010"]:
        accepted, cells = logspace_tm_simulator(tm, test_input, space_bound=2)
        parity = "even" if test_input.count("1") % 2 == 0 else "odd"
        print(f"  Input '{test_input}' ({parity} 1s): "
              f"accepted={accepted}, work_cells={cells}")


def demo_savitch_reachability() -> None:
    print("\n" + "=" * 60)
    print("DEMO 2: Savitch's Reachability Algorithm")
    print("=" * 60)

    graph = {
        0: [1, 2],
        1: [3],
        2: [3],
        3: [4],
        4: [],
    }
    n = 5

    print(f"\nGraph ({n} nodes):")
    print(fmt_graph(graph))

    pairs = [(0, 4), (0, 3), (1, 4), (2, 0), (4, 0)]
    for s, t in pairs:
        result, depth, calls = reachability_savitch_space_tracking(graph, s, t, n)
        bits_per_frame = math.ceil(math.log2(max(n, 2)))
        space_bits = depth * bits_per_frame
        print(f"\n  Reachable({s} → {t}) = {result}")
        print(f"    Max recursion depth: {depth}")
        print(f"    Total calls: {calls}")
        print(f"    Peak space: {depth} frames × {bits_per_frame} bits = {space_bits} bits")


def demo_tqbf() -> None:
    print("\n" + "=" * 60)
    print("DEMO 3: TQBF Evaluator")
    print("=" * 60)

    # ∀x ∃y (x ∨ y)
    # True: for x=T, pick y=T; for x=F, pick y=T (or y=F since x∨y is T when x=F and y=F... wait)
    # x=F, y=F: F∨F = F. But ∃y: y=T works. So yes, ∀x∃y(x∨y) is True.
    formula1 = {
        "quantifiers": [("x", "forall"), ("y", "exists")],
        "body": lambda env: env["x"] or env["y"],
    }
    print(f"\n∀x ∃y (x ∨ y) = {tqbf_evaluator(formula1)}")

    # ∃x ∀y (x ∧ y)
    # For x=T: need ∀y(T∧y) — but y=F gives F. So no.
    # For x=F: need ∀y(F∧y) — F always. So ∀y(F∧y) = F. No.
    # So ∃x ∀y (x ∧ y) = False.
    formula2 = {
        "quantifiers": [("x", "exists"), ("y", "forall")],
        "body": lambda env: env["x"] and env["y"],
    }
    print(f"∃x ∀y (x ∧ y) = {tqbf_evaluator(formula2)}")

    # ∀x ∃y ∀z ((x ∨ y) ∧ (¬y ∨ z))
    # Truth-table check: enumerate all 8 assignments
    formula3 = {
        "quantifiers": [("x", "forall"), ("y", "exists"), ("z", "forall")],
        "body": lambda env: (env["x"] or env["y"]) and (not env["y"] or env["z"]),
    }
    print(f"∀x ∃y ∀z ((x ∨ y) ∧ (¬y ∨ z)) = {tqbf_evaluator(formula3)}")


def demo_space_hierarchy() -> None:
    print("\n" + "=" * 60)
    print("DEMO 4: Space Complexity Hierarchy Visualization")
    print("=" * 60)

    classes = [
        ("L",    "O(log n)",      "Pointers, counters"),
        ("NL",   "O(log n) nondet", "Guess-and-verify paths"),
        ("P",    "O(n^k) time",   "Polynomial algorithms"),
        ("PSPACE", "O(n^k) space", "Game solving, model checking"),
        ("EXP",  "O(2^(n^k))",    "Brute-force search"),
    ]

    print("\n  L ⊆ NL ⊆ P ⊆ PSPACE ⊆ EXP\n")
    print(f"  {'Class':<12} {'Bound':<20} {'Example'}")
    print(f"  {'─'*12} {'─'*20} {'─'*30}")
    for name, bound, example in classes:
        print(f"  {name:<12} {bound:<20} {example}")

    print("\n  Key results:")
    print("  • Savitch: NSPACE(f(n)) ⊆ SPACE(f(n)²)  →  NL ⊆ L²")
    print("  • Immerman-Szelepcsényi: NL = coNL")
    print("  • TQBF is PSPACE-complete (game solving, model checking)")


# ── Main ────────────────────────────────────────────────────────────

def main() -> None:
    demo_space_bounded_tm()
    demo_savitch_reachability()
    demo_tqbf()
    demo_space_hierarchy()


if __name__ == "__main__":
    main()
