"""Lesson 11: Turing Machine Simulator."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, Optional, Tuple


@dataclass
class TuringMachine:
    """A deterministic single-tape Turing machine.

    Transitions: (state, symbol) → (new_state, write_symbol, direction)
    direction is 'L' (left) or 'R' (right).
    """

    states: set[str]
    tape_alphabet: set[str]
    transitions: Dict[Tuple[str, str], Tuple[str, str, str]]
    start: str
    accept: str
    reject: str

    def __post_init__(self) -> None:
        assert self.accept not in (self.start, self.reject)
        assert self.reject not in (self.start, self.accept)

    def run(
        self, input_string: str, max_steps: int = 10_000
    ) -> tuple[str, dict[int, str], int]:
        """Run the TM on input_string.

        Returns (result, tape_dict, steps) where result is 'accept',
        'reject', or 'limit'.  tape_dict maps position → symbol.
        Blank cells are omitted.
        """
        tape: dict[int, str] = {i: ch for i, ch in enumerate(input_string)}
        head = 0
        state = self.start
        steps = 0

        while steps < max_steps:
            steps += 1
            symbol = tape.get(head, "␣")

            key = (state, symbol)
            if key not in self.transitions:
                return "reject", tape, steps

            new_state, write_sym, direction = self.transitions[key]
            tape[head] = write_sym

            if new_state == self.accept:
                return "accept", tape, steps
            if new_state == self.reject:
                return "reject", tape, steps

            state = new_state
            head += 1 if direction == "R" else -1

        return "limit", tape, steps

    def visualize_tape(self, tape: dict[int, str], head: int) -> str:
        """Return ASCII visualization of the tape."""
        if not tape:
            return "[ ␣ ]\n  ^"

        lo = min(min(tape.keys()), head)
        hi = max(max(tape.keys()), head)

        cells = ""
        pointer = ""
        for pos in range(lo, hi + 1):
            sym = tape.get(pos, "␣")
            cells += f"[{sym}]"
            pointer += " ^ " if pos == head else "   "
        return cells + "\n" + pointer


# ──────────────────────────────────────────────
#  Example Machines
# ──────────────────────────────────────────────

def tm_anbncn() -> TuringMachine:
    """Accepts { aⁿbⁿcⁿ | n ≥ 1 }."""
    # States: q0=scan for a, q1=scan for b, q2=scan for c,
    #         q3=rewind left, q4=check all marked
    transitions = {
        # Read a, mark as X, go find b
        ("q0", "a"): ("q1", "X", "R"),
        ("q0", "X"): ("q0", "X", "R"),
        ("q0", "Y"): ("q0", "Y", "R"),
        ("q0", "␣"): ("q4", "␣", "L"),  # done marking, verify
        # Find first b
        ("q1", "a"): ("q1", "a", "R"),
        ("q1", "X"): ("q1", "X", "R"),
        ("q1", "b"): ("q2", "Y", "R"),
        ("q1", "Y"): ("q1", "Y", "R"),
        # Find first c
        ("q2", "b"): ("q2", "b", "R"),
        ("q2", "Y"): ("q2", "Y", "R"),
        ("q2", "c"): ("q3", "Z", "L"),
        ("q2", "Z"): ("q2", "Z", "R"),
        # Rewind to start
        ("q3", "a"): ("q3", "a", "L"),
        ("q3", "b"): ("q3", "b", "L"),
        ("q3", "X"): ("q3", "X", "L"),
        ("q3", "Y"): ("q3", "Y", "L"),
        ("q3", "Z"): ("q3", "Z", "L"),
        ("q3", "␣"): ("q0", "␣", "R"),
        # Verify: scan right, ensure only X, Y, Z remain
        ("q4", "X"): ("q4", "X", "L"),
        ("q4", "Y"): ("q4", "Y", "L"),
        ("q4", "Z"): ("q4", "Z", "L"),
        ("q4", "␣"): ("q_accept", "␣", "R"),
    }
    return TuringMachine(
        states={"q0", "q1", "q2", "q3", "q4", "q_accept", "q_reject"},
        tape_alphabet={"a", "b", "c", "X", "Y", "Z", "␣"},
        transitions=transitions,
        start="q0",
        accept="q_accept",
        reject="q_reject",
    )


def tm_binary_increment() -> TuringMachine:
    """Increments a binary number by 1.  Input e.g. '1011' → '1100'."""
    transitions = {
        # Move to the rightmost digit
        ("go_right", "0"): ("go_right", "0", "R"),
        ("go_right", "1"): ("go_right", "1", "R"),
        ("go_right", "␣"): ("carry", "␣", "L"),
        # Carry: add 1
        ("carry", "0"): ("done", "1", "L"),
        ("carry", "1"): ("carry", "0", "L"),
        ("carry", "␣"): ("done", "1", "L"),
        # Move left (cleanup, halt at leftmost)
        ("done", "0"): ("done", "0", "L"),
        ("done", "1"): ("done", "1", "L"),
        ("done", "␣"): ("q_accept", "␣", "R"),
    }
    return TuringMachine(
        states={"go_right", "carry", "done", "q_accept", "q_reject"},
        tape_alphabet={"0", "1", "␣"},
        transitions=transitions,
        start="go_right",
        accept="q_accept",
        reject="q_reject",
    )


def tm_palindrome() -> TuringMachine:
    """Accepts palindromes over {0,1}."""
    transitions = {
        # Scan right to find last symbol
        ("find_end", "0"): ("mark_0_end", "␣", "L"),
        ("find_end", "1"): ("mark_1_end", "␣", "L"),
        ("find_end", "␣"): ("q_accept", "␣", "R"),
        # Rewind to start for 0-match
        ("mark_0_end", "0"): ("mark_0_end", "0", "L"),
        ("mark_0_end", "1"): ("mark_0_end", "1", "L"),
        ("mark_0_end", "␣"): ("match_0", "␣", "R"),
        # Rewind to start for 1-match
        ("mark_1_end", "0"): ("mark_1_end", "0", "L"),
        ("mark_1_end", "1"): ("mark_1_end", "1", "L"),
        ("mark_1_end", "␣"): ("match_1", "␣", "R"),
        # Check first symbol is 0
        ("match_0", "0"): ("find_end", "␣", "R"),
        ("match_0", "1"): ("q_reject", "1", "R"),
        ("match_0", "␣"): ("q_accept", "␣", "R"),
        # Check first symbol is 1
        ("match_1", "0"): ("q_reject", "0", "R"),
        ("match_1", "1"): ("find_end", "␣", "R"),
        ("match_1", "␣"): ("q_accept", "␣", "R"),
    }
    return TuringMachine(
        states={
            "find_end", "mark_0_end", "mark_1_end",
            "match_0", "match_1", "q_accept", "q_reject",
        },
        tape_alphabet={"0", "1", "␣"},
        transitions=transitions,
        start="find_end",
        accept="q_accept",
        reject="q_reject",
    )


# ──────────────────────────────────────────────
#  Demo
# ──────────────────────────────────────────────

def demo() -> None:
    machines = [
        ("aⁿbⁿcⁿ (n=2)", tm_anbncn(), "aabbcc"),
        ("Binary increment", tm_binary_increment(), "1011"),
        ("Palindrome", tm_palindrome(), "1001"),
    ]
    for name, tm, inp in machines:
        print(f"=== {name} | input='{inp}' ===")
        result, tape, steps = tm.run(inp)
        print(tm.visualize_tape(tape, 0))
        print(f"Result: {result} in {steps} steps\n")


if __name__ == "__main__":
    demo()
