"""Lesson 12: Full-featured Turing Machine Simulator."""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple


# ──────────────────────────────────────────────
#  TM Definition
# ──────────────────────────────────────────────

@dataclass
class TMDefinition:
    """Serializable Turing machine definition."""

    name: str
    states: List[str]
    tape_alphabet: List[str]
    transitions: Dict[str, Dict[str, List[str]]]
    # transitions: { state: { symbol: [new_state, write, direction] } }
    start: str
    accept: str
    reject: str
    blank: str = "␣"

    def delta(self, state: str, symbol: str) -> Optional[Tuple[str, str, str]]:
        state_trans = self.transitions.get(state)
        if state_trans is None:
            return None
        action = state_trans.get(symbol)
        if action is None:
            return None
        return action[0], action[1], action[2]

    @classmethod
    def from_dict(cls, d: dict) -> "TMDefinition":
        return cls(
            name=d["name"],
            states=d["states"],
            tape_alphabet=d["tape_alphabet"],
            transitions=d["transitions"],
            start=d["start"],
            accept=d["accept"],
            reject=d["reject"],
            blank=d.get("blank", "␣"),
        )

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "states": self.states,
            "tape_alphabet": self.tape_alphabet,
            "transitions": self.transitions,
            "start": self.start,
            "accept": self.accept,
            "reject": self.reject,
            "blank": self.blank,
        }


# ──────────────────────────────────────────────
#  Simulator
# ──────────────────────────────────────────────

@dataclass
class Snapshot:
    state: str
    head: int
    tape: Dict[int, str]


class Simulator:
    """Interactive Turing machine simulator."""

    def __init__(self, tm_def: TMDefinition) -> None:
        self.tm = tm_def
        self.tape: Dict[int, str] = {}
        self.head = 0
        self.state = tm_def.start
        self.steps = 0
        self.history: List[Snapshot] = []
        self.breakpoints: set[str] = set()
        self.trace_log: List[str] = []

    def reset(self, input_string: str = "") -> None:
        self.tape = {i: ch for i, ch in enumerate(input_string)}
        self.head = 0
        self.state = self.tm.start
        self.steps = 0
        self.history = []
        self.trace_log = []

    def set_breakpoint(self, state: str) -> None:
        self.breakpoints.add(state)

    def clear_breakpoint(self, state: str) -> None:
        self.breakpoints.discard(state)

    def step(self) -> Optional[str]:
        """Execute one transition. Returns result string or None if running."""
        symbol = self.tape.get(self.head, self.tm.blank)
        action = self.tm.delta(self.state, symbol)

        # Record snapshot
        self.history.append(
            Snapshot(self.state, self.head, dict(self.tape))
        )

        if action is None:
            self.trace_log.append(
                f"Step {self.steps + 1}: ({self.state}, {symbol}) → REJECT (no transition)"
            )
            self.steps += 1
            return "reject"

        new_state, write_sym, direction = action
        self.trace_log.append(
            f"Step {self.steps + 1}: ({self.state}, {symbol}) → "
            f"({new_state}, {write_sym}, {direction})"
        )

        self.tape[self.head] = write_sym
        self.head += 1 if direction == "R" else -1
        self.state = new_state
        self.steps += 1

        if self.state == self.tm.accept:
            return "accept"
        if self.state == self.tm.reject:
            return "reject"
        if self.state in self.breakpoints:
            return "breakpoint"
        return None

    def run(self, max_steps: int = 50_000) -> str:
        """Run until halt or step limit."""
        for _ in range(max_steps):
            result = self.step()
            if result is not None:
                return result
        return "limit"

    def tape_display(self, window: int = 10) -> str:
        """ASCII visualization of the tape around the head."""
        if not self.tape:
            blank = self.tm.blank
            return f"[{blank}]\n ^ "

        lo = min(min(self.tape.keys()), self.head) - window
        hi = max(max(self.tape.keys()), self.head) + window

        cells = ""
        pointer = ""
        for pos in range(lo, hi + 1):
            sym = self.tape.get(pos, self.tm.blank)
            marker = "^" if pos == self.head else " "
            cells += f"[{sym}]"
            pointer += f" {marker} "

        header = f"State: {self.state}  Step: {self.steps}  Head: {self.head}"
        return f"{header}\n{cells}\n{pointer}"

    def trace(self) -> str:
        return "\n".join(self.trace_log)

    def to_json(self) -> str:
        return json.dumps(self.tm.to_dict(), indent=2, ensure_ascii=False)

    def to_dot(self) -> str:
        """Export TM as Graphviz DOT graph."""
        lines = [f'digraph "{self.tm.name}" {{']
        lines.append("  rankdir=LR;")
        lines.append(f'  node [shape=doublecircle]; {self.tm.accept};')
        lines.append(f'  node [shape=circle];')
        for state, trans in self.tm.transitions.items():
            for symbol, action in trans.items():
                label = f"{symbol}→{action[1]},{action[2]}"
                lines.append(f'  {state} -> {action[0]} [label="{label}"];')
        lines.append("}")
        return "\n".join(lines)

    def undo(self) -> None:
        """Undo last step."""
        if self.history:
            snap = self.history.pop()
            self.state = snap.state
            self.head = snap.head
            self.tape = snap.tape
            self.steps -= 1
            if self.trace_log:
                self.trace_log.pop()


# ──────────────────────────────────────────────
#  Example Machines
# ──────────────────────────────────────────────

def example_anbncn() -> TMDefinition:
    return TMDefinition(
        name="aⁿbⁿcⁿ",
        states=["q0", "q1", "q2", "q3", "q4", "q_accept", "q_reject"],
        tape_alphabet=["a", "b", "c", "X", "Y", "Z", "␣"],
        transitions={
            "q0": {"a": ["q1", "X", "R"], "X": ["q0", "X", "R"], "Y": ["q0", "Y", "R"], "␣": ["q4", "␣", "L"]},
            "q1": {"a": ["q1", "a", "R"], "X": ["q1", "X", "R"], "b": ["q2", "Y", "R"], "Y": ["q1", "Y", "R"]},
            "q2": {"b": ["q2", "b", "R"], "Y": ["q2", "Y", "R"], "c": ["q3", "Z", "L"], "Z": ["q2", "Z", "R"]},
            "q3": {"a": ["q3", "a", "L"], "b": ["q3", "b", "L"], "X": ["q3", "X", "L"], "Y": ["q3", "Y", "L"], "Z": ["q3", "Z", "L"], "␣": ["q0", "␣", "R"]},
            "q4": {"X": ["q4", "X", "L"], "Y": ["q4", "Y", "L"], "Z": ["q4", "Z", "L"], "␣": ["q_accept", "␣", "R"]},
        },
        start="q0",
        accept="q_accept",
        reject="q_reject",
    )


def example_binary_addition() -> TMDefinition:
    return TMDefinition(
        name="Binary Addition",
        states=["scan", "carry", "rewind", "q_accept", "q_reject"],
        tape_alphabet=["0", "1", "+", "␣"],
        transitions={
            "scan":   {"0": ["scan", "0", "R"], "1": ["scan", "1", "R"], "+": ["scan", "+", "R"], "␣": ["carry", "␣", "L"]},
            "carry":  {"0": ["rewind", "1", "L"], "1": ["carry", "0", "L"], "+": ["rewind", "1", "L"], "␣": ["q_accept", "␣", "R"]},
            "rewind": {"0": ["rewind", "0", "L"], "1": ["rewind", "1", "L"], "+": ["rewind", "+", "L"], "␣": ["q_accept", "␣", "R"]},
        },
        start="scan",
        accept="q_accept",
        reject="q_reject",
    )


def example_unary_multiply() -> TMDefinition:
    return TMDefinition(
        name="Unary Multiplication",
        states=["find_b", "mark_a", "rewind_a", "check_done", "q_accept", "q_reject"],
        tape_alphabet=["a", "b", "X", "Y", "$", "␣"],
        transitions={
            "find_b":     {"a": ["find_b", "a", "R"], "X": ["find_b", "X", "R"], "$": ["find_b", "$", "R"], "b": ["mark_a", "Y", "L"], "Y": ["find_b", "Y", "R"], "␣": ["check_done", "␣", "L"]},
            "mark_a":     {"a": ["rewind_a", "X", "L"], "X": ["mark_a", "X", "L"], "$": ["check_done", "$", "R"], "Y": ["mark_a", "Y", "L"]},
            "rewind_a":   {"a": ["rewind_a", "a", "L"], "X": ["rewind_a", "X", "L"], "$": ["rewind_a", "$", "L"], "Y": ["rewind_a", "Y", "L"], "␣": ["find_b", "␣", "R"]},
            "check_done": {"X": ["check_done", "X", "R"], "$": ["check_done", "$", "R"], "Y": ["check_done", "Y", "R"], "a": ["find_b", "a", "R"], "␣": ["q_accept", "␣", "L"]},
        },
        start="find_b",
        accept="q_accept",
        reject="q_reject",
    )


def example_palindrome() -> TMDefinition:
    return TMDefinition(
        name="Palindrome Detector",
        states=["find_end", "mark_0_end", "mark_1_end", "match_0", "match_1", "q_accept", "q_reject"],
        tape_alphabet=["0", "1", "␣"],
        transitions={
            "find_end":    {"0": ["mark_0_end", "␣", "L"], "1": ["mark_1_end", "␣", "L"], "␣": ["q_accept", "␣", "R"]},
            "mark_0_end":  {"0": ["mark_0_end", "0", "L"], "1": ["mark_0_end", "1", "L"], "␣": ["match_0", "␣", "R"]},
            "mark_1_end":  {"0": ["mark_1_end", "0", "L"], "1": ["mark_1_end", "1", "L"], "␣": ["match_1", "␣", "R"]},
            "match_0":     {"0": ["find_end", "␣", "R"], "1": ["q_reject", "1", "R"], "␣": ["q_accept", "␣", "R"]},
            "match_1":     {"0": ["q_reject", "0", "R"], "1": ["find_end", "␣", "R"], "␣": ["q_accept", "␣", "R"]},
        },
        start="find_end",
        accept="q_accept",
        reject="q_reject",
    )


def example_busy_beaver_3() -> TMDefinition:
    """3-state Busy Beaver — maximizes 1s in ≤ 6 steps on blank tape."""
    return TMDefinition(
        name="3-State Busy Beaver",
        states=["A", "B", "C", "HALT"],
        tape_alphabet=["0", "1", "␣"],
        transitions={
            "A": {"0": ["B", "1", "R"], "1": ["HALT", "1", "R"]},
            "B": {"0": ["C", "0", "R"], "1": ["B", "1", "R"]},
            "C": {"0": ["C", "1", "L"], "1": ["A", "1", "L"]},
        },
        start="A",
        accept="HALT",
        reject="HALT",
        blank="0",
    )


EXAMPLES = {
    "anbncn": example_anbncn,
    "binary_add": example_binary_addition,
    "unary_mul": example_unary_multiply,
    "palindrome": example_palindrome,
    "busy_beaver_3": example_busy_beaver_3,
}


# ──────────────────────────────────────────────
#  CLI Demo
# ──────────────────────────────────────────────

def demo() -> None:
    tests = [
        ("aⁿbⁿcⁿ", "anbncn", "aabbcc"),
        ("Binary Addition", "binary_add", "101+11"),
        ("Palindrome", "palindrome", "1001"),
        ("Busy Beaver", "busy_beaver_3", ""),
    ]
    for name, key, inp in tests:
        print(f"{'='*50}")
        print(f"  {name}  |  input='{inp}'")
        print(f"{'='*50}")
        tm_def = EXAMPLES[key]()
        sim = Simulator(tm_def)
        sim.reset(inp)
        result = sim.run(max_steps=200)
        print(sim.tape_display())
        print(sim.trace())
        print(f"Result: {result} in {sim.steps} steps\n")


if __name__ == "__main__":
    demo()
