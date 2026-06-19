"""Lesson 14: Rice's Theorem & Undecidable Properties
Phase 05 — Theory of Computation

Property catalogue, reduction-from-HALT demonstrator.
"""

from __future__ import annotations


# ---------------------------------------------------------------------------
# 1. Property catalogue classified by Rice's theorem
# ---------------------------------------------------------------------------

PROPERTY_TABLE: list[tuple[str, str, bool, str]] = [
    # (property description, semantic/syntactic, non-trivial, decidability)
    ("L(M) = ∅",                          "semantic",  True,  "Undecidable (EMPTY_TM)"),
    ("L(M) = Σ*",                         "semantic",  True,  "Undecidable (ALL_TM)"),
    ("L(M) is finite",                    "semantic",  True,  "Undecidable (FINITE_TM)"),
    ("L(M) contains ε",                   "semantic",  True,  "Undecidable"),
    ("L(M) is regular",                   "semantic",  True,  "Undecidable (REGULAR_TM)"),
    ("L(M) is context-free",              "semantic",  True,  "Undecidable"),
    ("L(M) has at least 5 strings",       "semantic",  True,  "Undecidable"),
    ("L(M) = L(M') for fixed M'",         "semantic",  True,  "Undecidable (EQ_TM)"),
    ("Every string in L(M) has len ≤ 3",  "semantic",  True,  "Undecidable"),
    ("M always halts",                    "semantic",  True,  "Undecidable"),
    ("M accepts within 100 steps on w",   "syntactic", True,  "Decidable"),
    ("M has ≤ 50 states",                 "syntactic", True,  "Decidable"),
    ("M's start state has no self-loop",  "syntactic", True,  "Decidable"),
]


def rice_theorem_examples() -> list[tuple[str, str, bool, str]]:
    """Print and return the property classification table."""
    print("Rice's Theorem — Property Classification")
    print("=" * 65)
    print(f"{'Property':<42} {'Type':<10} {'Trivial':<8} {'Status'}")
    print("-" * 65)
    for prop, ptype, nontrivial, status in PROPERTY_TABLE:
        nt = "no" if nontrivial else "yes"
        print(f"{prop:<42} {ptype:<10} {nt:<8} {status}")
    print()

    semantic_undecidable = [p for p, t, nt, s in PROPERTY_TABLE
                            if t == "semantic" and nt and "Undecidable" in s]
    print(f"Non-trivial semantic properties (all undecidable): "
          f"{len(semantic_undecidable)}/{len(PROPERTY_TABLE)}")
    print()
    return PROPERTY_TABLE


# ---------------------------------------------------------------------------
# 2. Reduction from HALT to any non-trivial semantic property
# ---------------------------------------------------------------------------

def reduce_to_halting(property_decider):
    """Show how a hypothetical decider for a non-trivial semantic property
    would solve HALT (and therefore cannot exist).

    Args:
        property_decider: callable(str) -> bool
            Pretends to decide whether L(M) has the target property.
            In reality no such function exists for non-trivial properties.

    Returns:
        A function halts_via_reduction(tm_desc, w) that would solve HALT
        if property_decider were real.
    """

    def halts_via_reduction(tm_description: str, input_w: str) -> bool:
        """Solve HALT using the property decider."""
        # M' on input x:
        #   simulate M on w
        #   if M halts → accept x  (so L(M') = Σ*)
        #   else → loop forever     (so L(M') = ∅)
        #
        # Choose the target property to be "L(?) = Σ*":
        #   property_decider(M') == True  → L(M') = Σ*  → M halts on w
        #   property_decider(M') == False → L(M') = ∅   → M loops on w
        reduced_machine_desc = (
            f"M_reduced(x):\n"
            f"  simulate [{tm_description}] on [{input_w}]\n"
            f"  if halts: accept x\n"
            f"  else: loop"
        )
        return property_decider(reduced_machine_desc)

    print("Reduction from HALT to any non-trivial semantic property")
    print("-" * 55)
    print("Given (M, w):")
    print("  1. Build M' that simulates M on w before doing anything.")
    print("  2. If M halts, M' acts like a TM whose language has the property.")
    print("  3. If M loops, M' acts like a TM whose language lacks the property.")
    print("  4. property_decider(M') answers HALT(M, w).")
    print("  5. Since HALT is undecidable, property_decider cannot exist. ∎\n")

    return halts_via_reduction


# ---------------------------------------------------------------------------
# 3. Quick-reference printer
# ---------------------------------------------------------------------------

def print_undecidability_summary() -> None:
    """Print a one-screen summary connecting HALT, Rice, and practice."""
    summary = """
┌──────────────────────────────────────────────────────────┐
│  UNDECIDABILITY QUICK REFERENCE                          │
├──────────────────────────────────────────────────────────┤
│  HALT = { ⟨M,w⟩ | M halts on w }      — undecidable     │
│                                                          │
│  Rice's Theorem:                                         │
│    Any non-trivial semantic property P:                  │
│    L_P = { ⟨M⟩ | L(M) has P }           — undecidable   │
│                                                          │
│  Practical impact:                                       │
│    "Does program crash?"          — undecidable          │
│    "Is this dead code?"           — undecidable          │
│    "Does it terminate?"           — undecidable          │
│    "Is type annotation correct?"  — undecidable          │
│                                                          │
│  Workaround: conservative approximations                 │
│    (sound OR complete, never both for non-trivial props) │
└──────────────────────────────────────────────────────────┘
"""
    print(summary)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    rice_theorem_examples()

    print_undecidability_summary()

    # Demonstrate the reduction framework (with a dummy decider)
    print("Reduction demo (hypothetical decider):")
    print("If we had a decider for 'L(M) = Σ*', we could solve HALT.\n")


if __name__ == "__main__":
    main()
