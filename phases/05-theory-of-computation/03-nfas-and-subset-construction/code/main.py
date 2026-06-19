"""
Lesson 03: NFAs and Subset Construction
========================================
NFA class with accepts() via simultaneous path simulation.
Subset construction to convert NFA → DFA.
Worst-case blowup demonstration.
"""

from __future__ import annotations
from collections import deque
from typing import FrozenSet, Dict, Set, Tuple, Optional


# ──────────────────────────────────────────────
#  DFA (reused from Lesson 02 for comparison)
# ──────────────────────────────────────────────

class DFA:
    """Deterministic Finite Automaton."""

    def __init__(
        self,
        states: Set[str],
        alphabet: Set[str],
        transitions: Dict[Tuple[str, str], str],
        start: str,
        accepting: Set[str],
    ):
        self.states = states
        self.alphabet = alphabet
        self.transitions = transitions
        self.start = start
        self.accepting = accepting

    def accepts(self, input_string: str) -> bool:
        state = self.start
        for ch in input_string:
            if (state, ch) not in self.transitions:
                return False
            state = self.transitions[(state, ch)]
        return state in self.accepting

    def transition_table(self) -> str:
        header = f"{'State':<10}" + "".join(f"{a:<8}" for a in sorted(self.alphabet))
        lines = [header, "-" * len(header)]
        for s in sorted(self.states):
            row = f"{s:<10}"
            for a in sorted(self.alphabet):
                dest = self.transitions.get((s, a), "—")
                marker = "*" if s in self.accepting else " "
                if s == self.start:
                    marker = ">" + marker
                else:
                    marker = " " + marker
                row += f"{dest:<8}"
            lines.append(marker + row)
        return "\n".join(lines)


# ──────────────────────────────────────────────
#  NFA
# ──────────────────────────────────────────────

class NFA:
    """Non-deterministic Finite Automaton with ε-transitions.

    Transition function: δ(state, symbol) → set of states.
    Symbol None represents ε-transitions.
    """

    def __init__(
        self,
        states: Set[str],
        alphabet: Set[str],
        transitions: Dict[Tuple[str, Optional[str]], Set[str]],
        start: str,
        accepting: Set[str],
    ):
        self.states = states
        self.alphabet = alphabet
        self.transitions = transitions  # (state, symbol_or_None) → {states}
        self.start = start
        self.accepting = accepting

    # ── ε-closure ─────────────────────────────

    def epsilon_closure(self, state_set: Set[str]) -> Set[str]:
        """Compute ε-closure of a set of states."""
        closure = set(state_set)
        stack = list(state_set)
        while stack:
            q = stack.pop()
            for nxt in self.transitions.get((q, None), set()):
                if nxt not in closure:
                    closure.add(nxt)
                    stack.append(nxt)
        return closure

    # ── Simulation ────────────────────────────

    def accepts(self, input_string: str) -> bool:
        """Simulate all paths simultaneously. Accept if any path succeeds."""
        current = self.epsilon_closure({self.start})
        for ch in input_string:
            next_states: Set[str] = set()
            for q in current:
                next_states |= self.transitions.get((q, ch), set())
            current = self.epsilon_closure(next_states)
            if not current:
                return False
        return bool(current & self.accepting)

    # ── Subset Construction ───────────────────

    def to_dfa(self) -> DFA:
        """Convert this NFA to an equivalent DFA using subset construction."""
        alphabet = self.alphabet - {None}  # no ε in DFA alphabet
        dfa_start_set = frozenset(self.epsilon_closure({self.start}))

        worklist: deque[FrozenSet[str]] = deque([dfa_start_set])
        dfa_states: Set[str] = set()
        dfa_transitions: Dict[Tuple[str, str], str] = {}
        dfa_accepting: Set[str] = set()

        # Map frozenset → DFA state name
        set_to_name: Dict[FrozenSet[str], str] = {dfa_start_set: _name(dfa_start_set)}
        dfa_states.add(set_to_name[dfa_start_set])

        # Check if start is accepting
        if dfa_start_set & self.accepting:
            dfa_accepting.add(set_to_name[dfa_start_set])

        while worklist:
            T = worklist.popleft()
            T_name = set_to_name[T]
            for a in sorted(alphabet):
                # move(T, a)
                next_set: Set[str] = set()
                for q in T:
                    next_set |= self.transitions.get((q, a), set())
                U = frozenset(self.epsilon_closure(next_set))
                if not U:
                    continue  # dead state — implicit
                if U not in set_to_name:
                    set_to_name[U] = _name(U)
                    dfa_states.add(set_to_name[U])
                    worklist.append(U)
                    if U & self.accepting:
                        dfa_accepting.add(set_to_name[U])
                dfa_transitions[(T_name, a)] = set_to_name[U]

        return DFA(
            states=dfa_states,
            alphabet=alphabet,
            transitions=dfa_transitions,
            start=set_to_name[dfa_start_set],
            accepting=dfa_accepting,
        )


def _name(state_set: FrozenSet[str]) -> str:
    """Create a readable name for a DFA state (set of NFA states)."""
    if not state_set:
        return "∅"
    return "{" + ",".join(sorted(state_set)) + "}"


# ──────────────────────────────────────────────
#  Demo: NFA for "strings ending in 'a'"
# ──────────────────────────────────────────────

def demo_basic_nfa():
    """NFA over {a, b} accepting strings ending in 'a'."""
    print("=" * 60)
    print("DEMO 1: NFA for strings over {a,b}* ending in 'a'")
    print("=" * 60)

    nfa = NFA(
        states={"q0", "q1"},
        alphabet={"a", "b"},
        transitions={
            ("q0", "a"): {"q0", "q1"},
            ("q0", "b"): {"q0"},
            ("q1", "a"): set(),
            ("q1", "b"): set(),
        },
        start="q0",
        accepting={"q1"},
    )

    test_strings = ["a", "ab", "aba", "baba", "bbb", ""]
    print("\nNFA simulation:")
    for s in test_strings:
        result = nfa.accepts(s)
        print(f"  '{s}' → {'ACCEPT' if result else 'REJECT'}")

    # Convert to DFA
    dfa = nfa.to_dfa()
    print(f"\nDFA has {len(dfa.states)} states:")
    for s in sorted(dfa.states):
        acc = " (accepting)" if s in dfa.accepting else ""
        start = " [start]" if s == dfa.start else ""
        print(f"  {s}{start}{acc}")

    print("\nDFA transitions:")
    for (state, sym), dest in sorted(dfa.transitions.items()):
        print(f"  δ({state}, {sym}) = {dest}")

    print("\nDFA verification:")
    for s in test_strings:
        result = dfa.accepts(s)
        print(f"  '{s}' → {'ACCEPT' if result else 'REJECT'}")


# ──────────────────────────────────────────────
#  Demo: NFA with ε-transitions
# ──────────────────────────────────────────────

def demo_epsilon_nfa():
    """NFA with ε-transitions: language {ab, ac}."""
    print("\n" + "=" * 60)
    print("DEMO 2: NFA with ε-transitions for language {ab, ac}")
    print("=" * 60)

    nfa = NFA(
        states={"q0", "q1", "q2", "q3", "q4"},
        alphabet={"a", "b", "c"},
        transitions={
            ("q0", "a"): {"q1"},
            ("q1", None): {"q2", "q3"},  # ε-branch
            ("q2", "b"): {"q4"},
            ("q3", "c"): {"q4"},
        },
        start="q0",
        accepting={"q4"},
    )

    test_strings = ["ab", "ac", "abc", "a", ""]
    print("\nNFA simulation:")
    for s in test_strings:
        result = nfa.accepts(s)
        print(f"  '{s}' → {'ACCEPT' if result else 'REJECT'}")

    dfa = nfa.to_dfa()
    print(f"\nDFA has {len(dfa.states)} states after subset construction.")


# ──────────────────────────────────────────────
#  Demo: Worst-case exponential blowup
# ──────────────────────────────────────────────

def demo_blowup():
    """Construct an NFA where subset construction produces 2^n DFA states."""
    print("\n" + "=" * 60)
    print("DEMO 3: Worst-case exponential blowup")
    print("=" * 60)

    n = 5  # number of NFA states
    states = {f"q{i}" for i in range(n)}
    alphabet = {"a"}

    # Each state on 'a' transitions to itself AND to every state with higher index.
    # This creates a structure where many subsets are reachable.
    transitions: Dict[Tuple[str, Optional[str]], Set[str]] = {}
    for i in range(n):
        # On 'a', q_i goes to q_i, q_{i+1}, ..., q_{n-1}
        transitions[(f"q{i}", "a")] = {f"q{j}" for j in range(i, n)}

    nfa = NFA(
        states=states,
        alphabet=alphabet,
        transitions=transitions,
        start="q0",
        accepting={f"q{n - 1}"},
    )

    print(f"\nNFA: {n} states, alphabet = {{a}}")
    dfa = nfa.to_dfa()
    print(f"DFA: {len(dfa.states)} states (theoretical max: 2^{n} = {2**n})")

    if len(dfa.states) == 2**n:
        print("  → Full blowup achieved! Every subset is reachable.")
    else:
        print(f"  → {len(dfa.states)} of {2**n} possible subsets are reachable.")


# ──────────────────────────────────────────────
#  Demo: Second-to-last position is 0
# ──────────────────────────────────────────────

def demo_second_to_last():
    """NFA for 'strings with 0 in the second-to-last position'."""
    print("\n" + "=" * 60)
    print("DEMO 4: NFA for strings with 0 in the second-to-last position")
    print("=" * 60)

    nfa = NFA(
        states={"q0", "q1", "q2"},
        alphabet={"0", "1"},
        transitions={
            ("q0", "0"): {"q0", "q1"},
            ("q0", "1"): {"q0"},
            ("q1", "0"): {"q2"},
            ("q1", "1"): {"q2"},
            ("q2", "0"): set(),
            ("q2", "1"): set(),
        },
        start="q0",
        accepting={"q2"},
    )

    test_strings = ["00", "01", "10", "11", "010", "100", "101", "110", "0000", "1111"]
    print("\nNFA simulation:")
    for s in test_strings:
        result = nfa.accepts(s)
        # Manual check: second-to-last char is '0'?
        expected = len(s) >= 2 and s[-2] == "0"
        match = "✓" if result == expected else "✗"
        print(f"  '{s}' → {'ACCEPT' if result else 'REJECT'}  "
              f"(expected: {'ACCEPT' if expected else 'REJECT'}) {match}")

    dfa = nfa.to_dfa()
    print(f"\nEquivalent DFA has {len(dfa.states)} states.")


# ──────────────────────────────────────────────
#  Main
# ──────────────────────────────────────────────

if __name__ == "__main__":
    demo_basic_nfa()
    demo_epsilon_nfa()
    demo_second_to_last()
    demo_blowup()
