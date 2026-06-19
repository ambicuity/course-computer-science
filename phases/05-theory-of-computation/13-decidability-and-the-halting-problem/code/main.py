"""Lesson 13: Decidability and the Halting Problem
Phase 05 — Theory of Computation

Diagonalization simulator, halting-problem proof walkthrough,
semi-decider, and decidable/undecidable problem catalogue.
"""

from __future__ import annotations

import threading
from itertools import product


# ---------------------------------------------------------------------------
# 1. Cantor-style diagonalization on finite boolean functions
# ---------------------------------------------------------------------------

def diagonalization_demo(n: int = 8, listed_count: int = 10) -> tuple:
    """Show that no finite list of boolean functions on {0..n-1} is exhaustive.

    Returns the diagonal function as a tuple so callers can inspect it.
    """
    all_functions = list(product([0, 1], repeat=n))
    listed = all_functions[:listed_count]

    # g(i) = 1 - f_i(i)
    diagonal = tuple(1 - listed[i][i] for i in range(n))

    print(f"Diagonalization demo (domain size = {n}, list size = {listed_count})")
    print("-" * 50)
    for i, f in enumerate(listed):
        print(f"  f_{i}: {f}")
    print(f"\n  Diagonal g: {diagonal}")
    print(f"  g in listed set?  {diagonal in listed}")
    print(f"  g in full set?    {diagonal in all_functions}")
    print(f"\n  g differs from every f_i at position i.\n")
    return diagonal


# ---------------------------------------------------------------------------
# 2. Interactive halting-problem undecidability walkthrough
# ---------------------------------------------------------------------------

def halting_undecidable_proof() -> None:
    """Print a step-by-step walkthrough of Turing's diagonalization proof."""
    steps = [
        ("Assume", "H(⟨M, w⟩) decides HALT — returns True iff M halts on w."),
        ("Build D", "D(⟨M⟩): if H(⟨M, ⟨M⟩⟩) → True then loop; else halt."),
        ("Run D(⟨D⟩)", "Does D halt on its own encoding?"),
        ("Case 1",
         "D halts on ⟨D⟩  ⟹  H said True  ⟹  D loops.  CONTRADICTION."),
        ("Case 2",
         "D loops on ⟨D⟩  ⟹  H said False ⟹  D halts.   CONTRADICTION."),
        ("Conclusion", "H cannot exist. HALT is undecidable. ∎"),
    ]
    print("Halting Problem — Undecidability Proof")
    print("=" * 50)
    for label, detail in steps:
        print(f"\n[{label}]")
        print(f"  {detail}")

    print("\nKey insight: D uses H's own answer to do the opposite.")
    print("No oracle H can handle every possible input because D always\n"
          "constructs a counterexample from whatever H provides.\n")


# ---------------------------------------------------------------------------
# 3. Semi-decider with timeout
# ---------------------------------------------------------------------------

def semi_decider(program_code: str, inp: str, timeout: float = 2.0) -> bool | None:
    """Attempt to detect whether *program_code* halts on *inp*.

    Returns:
        True  — program halted within the timeout window.
        None  — program did not halt within the timeout (unknown if infinite).
    """
    result = {"halted": False}

    def _run() -> None:
        try:
            namespace = {"__input__": inp}
            exec(program_code, namespace)
            result["halted"] = True
        except Exception:
            result["halted"] = True

    t = threading.Thread(target=_run, daemon=True)
    t.start()
    t.join(timeout=timeout)
    return result["halted"] if not t.is_alive() else None


# ---------------------------------------------------------------------------
# 4. Decidable vs undecidable problem catalogue
# ---------------------------------------------------------------------------

DECIDABLE_PROBLEMS: list[str] = [
    "Is a string accepted by a given DFA?",
    "Is a regular language empty?",
    "Two DFAs accept the same language?",
    "Is a CFG language empty (reachability test)?",
    "Is a given string in a CFL (CYK / Earley parser)?",
    "Does a DFA accept the empty string?",
]

UNDECIDABLE_PROBLEMS: list[str] = [
    "HALT: Does TM M halt on input w?",
    "EMPTY_TM: Does TM M accept nothing?",
    "ALL_TM: Does TM M accept every string?",
    "EQ_TM: Do TMs M1 and M2 accept the same language?",
    "REGULAR_TM: Does TM M accept a regular language?",
    "FINITE_TM: Does TM M accept a finite language?",
]


def print_problem_catalogue() -> None:
    """Print the decidable / undecidable lists side-by-side."""
    print("Decidable vs Undecidable Problems")
    print("=" * 50)
    print("\nDECIDABLE:")
    for p in DECIDABLE_PROBLEMS:
        print(f"  ✓ {p}")
    print("\nUNDECIDABLE:")
    for p in UNDECIDABLE_PROBLEMS:
        print(f"  ✗ {p}")
    print()


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    diagonalization_demo()
    halting_undecidable_proof()

    print("Semi-decider test (halting program):")
    result = semi_decider("x = 0\nfor i in range(10): x += 1", "")
    print(f"  Result: {result}\n")

    print("Semi-decider test (non-halting program, 0.5s timeout):")
    result = semi_decider("while True: pass", "", timeout=0.5)
    print(f"  Result: {result}\n")

    print_problem_catalogue()


if __name__ == "__main__":
    main()
