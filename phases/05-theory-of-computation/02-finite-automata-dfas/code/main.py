"""Finite Automata — DFAs.

A general-purpose DFA class supporting:
- accepts(string) — run the machine and check acceptance.
- complement() — swap accepting/non-accepting states.
- intersect(other) — product construction (∩).
- union(other) — product construction (∪).

Plus hand-built DFAs for five+ classic languages.
"""
from __future__ import annotations
from dataclasses import dataclass, field


@dataclass
class DFA:
    """Deterministic Finite Automaton: (Q, Σ, δ, q₀, F)."""

    states: set[str]
    alphabet: set[str]
    transitions: dict[tuple[str, str], str]
    start: str
    accept: set[str]

    def __post_init__(self) -> None:
        if self.start not in self.states:
            raise ValueError(f"Start state '{self.start}' not in Q")
        if not self.accept.issubset(self.states):
            raise ValueError("Accept states must be a subset of Q")
        for (q, a), q_next in self.transitions.items():
            if q not in self.states:
                raise ValueError(f"Transition source '{q}' not in Q")
            if a not in self.alphabet:
                raise ValueError(f"Symbol '{a}' not in Σ")
            if q_next not in self.states:
                raise ValueError(f"Transition target '{q_next}' not in Q")

    def accepts(self, input_string: str) -> bool:
        """Return True iff the machine accepts input_string."""
        state = self.start
        for ch in input_string:
            if ch not in self.alphabet:
                return False
            key = (state, ch)
            if key not in self.transitions:
                return False
            state = self.transitions[key]
        return state in self.accept

    def complement(self) -> DFA:
        """Return a DFA that recognizes Σ* \\ L(M)."""
        return DFA(
            states=set(self.states),
            alphabet=set(self.alphabet),
            transitions=dict(self.transitions),
            start=self.start,
            accept=self.states - self.accept,
        )

    def intersect(self, other: DFA) -> DFA:
        """Product construction for intersection L(self) ∩ L(other)."""
        return self._product(other, mode="and")

    def union(self, other: DFA) -> DFA:
        """Product construction for union L(self) ∪ L(other)."""
        return self._product(other, mode="or")

    def _product(self, other: DFA, mode: str) -> DFA:
        """Build product DFA. mode='and' → intersection, mode='or' → union."""
        if self.alphabet != other.alphabet:
            raise ValueError("Product construction requires identical alphabets")

        new_states: set[str] = set()
        new_transitions: dict[tuple[str, str], str] = {}
        new_accept: set[str] = set()

        pairs: list[tuple[str, str]] = [(self.start, other.start)]
        visited: set[tuple[str, str]] = set()
        queue = list(pairs)

        while queue:
            q1, q2 = queue.pop(0)
            if (q1, q2) in visited:
                continue
            visited.add((q1, q2))
            state_name = f"({q1},{q2})"
            new_states.add(state_name)

            is_accept = (
                (q1 in self.accept and q2 in other.accept)
                if mode == "and"
                else (q1 in self.accept or q2 in other.accept)
            )
            if is_accept:
                new_accept.add(state_name)

            for a in self.alphabet:
                n1 = self.transitions.get((q1, a))
                n2 = other.transitions.get((q2, a))
                if n1 is not None and n2 is not None:
                    target = f"({n1},{n2})"
                    new_transitions[(state_name, a)] = target
                    if (n1, n2) not in visited:
                        queue.append((n1, n2))

        return DFA(
            states=new_states,
            alphabet=set(self.alphabet),
            transitions=new_transitions,
            start=f"({self.start},{other.start})",
            accept=new_accept,
        )

    def __repr__(self) -> str:
        return (
            f"DFA(|Q|={len(self.states)}, |Σ|={len(self.alphabet)}, "
            f"|F|={len(self.accept)})"
        )


# ---------------------------------------------------------------------------
# DFA constructions for classic languages
# ---------------------------------------------------------------------------

def dfa_strings_ending_in(suffix: str, alphabet: set[str]) -> DFA:
    """DFA for strings ending in a given suffix (e.g., '01')."""
    n = len(suffix)
    states = {f"q{i}" for i in range(n + 1)}
    transitions: dict[tuple[str, str], str] = {}

    for i in range(n):
        for a in alphabet:
            # Longest prefix of suffix that is a suffix of (suffix[:i] + a)
            candidate = suffix[:i] + a
            next_state = 0
            for j in range(min(i + 1, n), 0, -1):
                if candidate.endswith(suffix[:j]):
                    next_state = j
                    break
            transitions[(f"q{i}", a)] = f"q{next_state}"

    # From the full match state
    for a in alphabet:
        candidate = suffix + a
        next_state = 0
        for j in range(n, 0, -1):
            if candidate.endswith(suffix[:j]):
                next_state = j
                break
        transitions[(f"q{n}", a)] = f"q{next_state}"

    return DFA(
        states=states,
        alphabet=alphabet,
        transitions=transitions,
        start="q0",
        accept={f"q{n}"},
    )


def dfa_even_ones(alphabet: set[str] = {"0", "1"}) -> DFA:
    """DFA for strings with an even number of 1s."""
    return DFA(
        states={"E", "O"},
        alphabet=alphabet,
        transitions={
            ("E", "0"): "E", ("E", "1"): "O",
            ("O", "0"): "O", ("O", "1"): "E",
        },
        start="E",
        accept={"E"},
    )


def dfa_even_zeros(alphabet: set[str] = {"0", "1"}) -> DFA:
    """DFA for strings with an even number of 0s."""
    return DFA(
        states={"E", "O"},
        alphabet=alphabet,
        transitions={
            ("E", "0"): "O", ("E", "1"): "E",
            ("O", "0"): "E", ("O", "1"): "O",
        },
        start="E",
        accept={"E"},
    )


def dfa_binary_multiple_of(k: int) -> DFA:
    """DFA for binary strings (read left-to-right) representing multiples of k.

    States are remainders 0..k-1. Reading bit b at remainder r
    transitions to remainder (2*r + b) mod k.
    """
    states = {f"r{i}" for i in range(k)}
    transitions: dict[tuple[str, str], str] = {}
    for r in range(k):
        for b in ("0", "1"):
            new_r = (2 * r + int(b)) % k
            transitions[(f"r{r}", b)] = f"r{new_r}"

    return DFA(
        states=states,
        alphabet={"0", "1"},
        transitions=transitions,
        start="r0",
        accept={"r0"},
    )


def dfa_contains_substring(sub: str, alphabet: set[str]) -> DFA:
    """DFA for strings containing a given substring."""
    n = len(sub)
    states = {f"q{i}" for i in range(n + 1)}
    transitions: dict[tuple[str, str], str] = {}

    for i in range(n):
        for a in alphabet:
            candidate = sub[:i] + a
            next_state = 0
            for j in range(min(i + 1, n), 0, -1):
                if candidate.endswith(sub[:j]):
                    next_state = j
                    break
            transitions[(f"q{i}", a)] = f"q{next_state}"

    for a in alphabet:
        transitions[(f"q{n}", a)] = f"q{n}"

    return DFA(
        states=states,
        alphabet=alphabet,
        transitions=transitions,
        start="q0",
        accept={f"q{n}"},
    )


# ---------------------------------------------------------------------------
# Demo
# ---------------------------------------------------------------------------

def demo() -> None:
    print("=== DFA Library Demo ===\n")

    # DFA 1: strings ending in "01"
    m1 = dfa_strings_ending_in("01", {"0", "1"})
    print(f"DFA 1 (ends in '01'): {m1}")
    for s in ["", "0", "1", "01", "101", "110", "1001", "0011"]:
        print(f"  '{s}' → {m1.accepts(s)}")

    # DFA 2: even number of 1s
    m2 = dfa_even_ones()
    print(f"\nDFA 2 (even 1s): {m2}")
    for s in ["", "1", "11", "101", "111", "1101"]:
        print(f"  '{s}' → {m2.accepts(s)}")

    # DFA 3: binary multiples of 3
    m3 = dfa_binary_multiple_of(3)
    print(f"\nDFA 3 (binary multiple of 3): {m3}")
    for val in range(16):
        b = bin(val)[2:]
        print(f"  {b:>4s} (= {val:>2d}) → {m3.accepts(b)}")

    # DFA 4: contains "101"
    m4 = dfa_contains_substring("101", {"0", "1"})
    print(f"\nDFA 4 (contains '101'): {m4}")
    for s in ["", "0", "101", "1101", "111", "01010", "000"]:
        print(f"  '{s}' → {m4.accepts(s)}")

    # DFA 5: strings ending in "01" AND even number of 1s
    print("\n--- Product Construction ---")
    m5 = m1.intersect(m2)
    print(f"Intersection (ends in '01' ∩ even 1s): {m5}")
    for s in ["01", "101", "001", "1001", "1101", "11001"]:
        result = m5.accepts(s)
        expected = m1.accepts(s) and m2.accepts(s)
        print(f"  '{s}' → {result}  (expected: {expected})  {'✓' if result == expected else '✗'}")

    # DFA 6: union
    m6 = m1.union(m2)
    print(f"\nUnion (ends in '01' ∪ even 1s): {m6}")
    for s in ["01", "1", "11", "10", "111", "110"]:
        result = m6.accepts(s)
        expected = m1.accepts(s) or m2.accepts(s)
        print(f"  '{s}' → {result}  (expected: {expected})  {'✓' if result == expected else '✗'}")

    # DFA 7: complement
    m7 = m2.complement()
    print(f"\nComplement (NOT even 1s = odd 1s): {m7}")
    for s in ["", "1", "11", "111", "1010"]:
        result = m7.accepts(s)
        expected = not m2.accepts(s)
        print(f"  '{s}' → {result}  (expected: {expected})  {'✓' if result == expected else '✗'}")


def main() -> None:
    demo()


if __name__ == "__main__":
    main()
