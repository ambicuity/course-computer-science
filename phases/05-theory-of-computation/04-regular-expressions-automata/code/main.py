"""
Lesson 04: Regular Expressions ↔ Automata
============================================
Recursive-descent regex parser.
Thompson construction: regex → NFA.
Verification against Python's re module.
"""

from __future__ import annotations
from collections import deque
import re
from typing import FrozenSet, Dict, Set, Tuple, Optional


# ──────────────────────────────────────────────
#  NFA (reused from Lesson 03)
# ──────────────────────────────────────────────

class NFA:
    """Non-deterministic Finite Automaton with ε-transitions."""

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
        self.transitions = transitions
        self.start = start
        self.accepting = accepting

    def epsilon_closure(self, state_set: Set[str]) -> Set[str]:
        closure = set(state_set)
        stack = list(state_set)
        while stack:
            q = stack.pop()
            for nxt in self.transitions.get((q, None), set()):
                if nxt not in closure:
                    closure.add(nxt)
                    stack.append(nxt)
        return closure

    def accepts(self, input_string: str) -> bool:
        current = self.epsilon_closure({self.start})
        for ch in input_string:
            next_states: Set[str] = set()
            for q in current:
                next_states |= self.transitions.get((q, ch), set())
            current = self.epsilon_closure(next_states)
            if not current:
                return False
        return bool(current & self.accepting)


# ──────────────────────────────────────────────
#  Regex Parser (recursive descent)
# ──────────────────────────────────────────────
#
# Grammar:
#   expr     → term ('|' term)*
#   term     → factor*
#   factor   → base ('*')?
#   base     → CHAR | '(' expr ')'

class RegexParser:
    """Recursive-descent parser for regex.

    Supported syntax:
      - Literal characters (any printable except | * ( ) )
      - | for alternation
      - * for Kleene star
      - ( ) for grouping
      - Concatenation is implicit (juxtaposition)
    """

    def __init__(self, pattern: str):
        self.pattern = pattern
        self.pos = 0

    def parse(self):
        node = self._expr()
        if self.pos < len(self.pattern):
            raise SyntaxError(
                f"Unexpected character '{self.pattern[self.pos]}' at position {self.pos}"
            )
        return node

    def _peek(self) -> Optional[str]:
        if self.pos < len(self.pattern):
            return self.pattern[self.pos]
        return None

    def _consume(self) -> str:
        ch = self.pattern[self.pos]
        self.pos += 1
        return ch

    def _expr(self):
        """expr → term ('|' term)*"""
        left = self._term()
        while self._peek() == "|":
            self._consume()  # consume '|'
            right = self._term()
            left = ("ALT", left, right)
        return left

    def _term(self):
        """term → factor*"""
        factors = []
        while self._peek() is not None and self._peek() not in ("|", ")"):
            factors.append(self._factor())
        if not factors:
            return ("LIT", "")  # empty — matches ε
        result = factors[0]
        for f in factors[1:]:
            result = ("CONCAT", result, f)
        return result

    def _factor(self):
        """factor → base ('*')?"""
        base = self._base()
        if self._peek() == "*":
            self._consume()
            return ("STAR", base)
        return base

    def _base(self):
        """base → CHAR | '(' expr ')'"""
        ch = self._peek()
        if ch == "(":
            self._consume()  # consume '('
            node = self._expr()
            if self._peek() != ")":
                raise SyntaxError("Missing closing parenthesis")
            self._consume()  # consume ')'
            return node
        if ch is None or ch in ("|", ")", "*"):
            raise SyntaxError(f"Unexpected character '{ch}' at position {self.pos}")
        self._consume()
        return ("LIT", ch)


# ──────────────────────────────────────────────
#  Thompson Construction
# ──────────────────────────────────────────────

_state_counter = 0


def _fresh_state() -> str:
    global _state_counter
    name = f"s{_state_counter}"
    _state_counter += 1
    return name


def _reset_counter():
    global _state_counter
    _state_counter = 0


def _add_trans(trans, src, sym, dest):
    """Add a transition to the dictionary."""
    key = (src, sym)
    if key not in trans:
        trans[key] = set()
    trans[key].add(dest)


def thompson(node) -> Tuple[Set[str], Set[str], Dict, str, str]:
    """Build an NFA fragment from an AST node.

    Returns (states, alphabet, transitions, start, accept).
    """
    tag = node[0]

    if tag == "LIT":
        lit = node[1]
        if lit == "":
            # ε — single state that is both start and accept
            s = _fresh_state()
            return {s}, set(), {}, s, s
        s1, s2 = _fresh_state(), _fresh_state()
        trans = {(s1, lit): {s2}}
        return {s1, s2}, {lit}, trans, s1, s2

    elif tag == "CONCAT":
        left_states, left_alpha, left_trans, left_start, left_accept = thompson(node[1])
        right_states, right_alpha, right_trans, right_start, right_accept = thompson(node[2])
        states = left_states | right_states
        alpha = left_alpha | right_alpha
        trans = {**left_trans, **right_trans}
        _add_trans(trans, left_accept, None, right_start)
        return states, alpha, trans, left_start, right_accept

    elif tag == "ALT":
        left_states, left_alpha, left_trans, left_start, left_accept = thompson(node[1])
        right_states, right_alpha, right_trans, right_start, right_accept = thompson(node[2])
        s = _fresh_state()
        f = _fresh_state()
        states = left_states | right_states | {s, f}
        alpha = left_alpha | right_alpha
        trans = {**left_trans, **right_trans}
        _add_trans(trans, s, None, left_start)
        _add_trans(trans, s, None, right_start)
        _add_trans(trans, left_accept, None, f)
        _add_trans(trans, right_accept, None, f)
        return states, alpha, trans, s, f

    elif tag == "STAR":
        inner_states, inner_alpha, inner_trans, inner_start, inner_accept = thompson(node[1])
        s = _fresh_state()
        f = _fresh_state()
        states = inner_states | {s, f}
        alpha = inner_alpha
        trans = {**inner_trans}
        _add_trans(trans, s, None, inner_start)
        _add_trans(trans, s, None, f)
        _add_trans(trans, inner_accept, None, inner_start)
        _add_trans(trans, inner_accept, None, f)
        return states, alpha, trans, s, f

    raise ValueError(f"Unknown AST node: {tag}")


def regex_to_nfa(pattern: str) -> NFA:
    """Convert a regex pattern string to an NFA using Thompson construction."""
    _reset_counter()
    parser = RegexParser(pattern)
    ast = parser.parse()
    states, alphabet, transitions, start, accept = thompson(ast)
    return NFA(states=states, alphabet=alphabet, transitions=transitions,
               start=start, accepting={accept})


# ──────────────────────────────────────────────
#  Verification against Python's re module
# ──────────────────────────────────────────────

def verify_against_python_re(pattern: str, test_strings: list[str]) -> bool:
    """Compare NFA acceptance with Python re.fullmatch."""
    nfa = regex_to_nfa(pattern)
    all_match = True
    for s in test_strings:
        nfa_result = nfa.accepts(s)
        py_result = re.fullmatch(pattern, s) is not None
        match = "✓" if nfa_result == py_result else "✗"
        if nfa_result != py_result:
            all_match = False
        print(f"  '{s}' → NFA={'ACC' if nfa_result else 'REJ'}, "
              f"re={'ACC' if py_result else 'REJ'} {match}")
    return all_match


# ──────────────────────────────────────────────
#  Demos
# ──────────────────────────────────────────────

def demo_thompson():
    """Demonstrate Thompson construction for several regexes."""
    patterns = [
        ("a", ["", "a", "b", "aa"]),
        ("a*", ["", "a", "aa", "aaa", "b"]),
        ("a|b", ["", "a", "b", "ab", "c"]),
        ("(a|b)*", ["", "a", "b", "ab", "ba", "aab", "c"]),
        ("a(a|b)*b", ["ab", "aab", "abb", "abab", "a", "b", ""]),
        ("(ab)*", ["", "ab", "abab", "a", "b", "aba"]),
    ]

    print("=" * 60)
    print("DEMO 1: Thompson Construction — Regex → NFA")
    print("=" * 60)

    for pattern, tests in patterns:
        print(f"\nPattern: {pattern}")
        nfa = regex_to_nfa(pattern)
        print(f"  NFA: {len(nfa.states)} states, alphabet = {nfa.alphabet}")
        for s in tests:
            result = nfa.accepts(s)
            py = re.fullmatch(pattern, s) is not None
            match = "✓" if result == py else "✗"
            print(f"  '{s}' → {'ACC' if result else 'REJ'} {match}")


def demo_verification():
    """Thorough verification against Python's re module."""
    print("\n" + "=" * 60)
    print("DEMO 2: Verification Against Python re Module")
    print("=" * 60)

    test_cases = [
        ("a|b", ["", "a", "b", "ab", "c"]),
        ("a*b", ["", "b", "ab", "aab", "aaab", "a", "ba"]),
        ("a(a|b)", ["", "a", "aa", "ab", "ba", "b"]),
    ]

    for pattern, tests in test_cases:
        print(f"\nPattern: {pattern}")
        verify_against_python_re(pattern, tests)


def demo_star_concat():
    """Demonstrate complex patterns."""
    print("\n" + "=" * 60)
    print("DEMO 3: Complex Pattern — (a|b)*a(a|b)")
    print("=" * 60)

    pattern = "(a|b)*a(a|b)"
    nfa = regex_to_nfa(pattern)
    print(f"NFA: {len(nfa.states)} states")

    # Generate test strings
    import itertools
    test_strings = ["".join(p) for length in range(5) for p in itertools.product("ab", repeat=length)]
    nfa_accepts = sorted(s for s in test_strings if nfa.accepts(s))
    py_accepts = sorted(s for s in test_strings if re.fullmatch(pattern, s))

    print(f"\nStrings up to length 4 accepted by NFA ({len(nfa_accepts)}):")
    print(f"  {nfa_accepts}")

    if nfa_accepts == py_accepts:
        print("\n✓ NFA matches Python re exactly.")
    else:
        print("\n✗ Mismatch detected!")
        print(f"  NFA only: {set(nfa_accepts) - set(py_accepts)}")
        print(f"  re only:  {set(py_accepts) - set(nfa_accepts)}")


# ──────────────────────────────────────────────
#  Main
# ──────────────────────────────────────────────

if __name__ == "__main__":
    demo_thompson()
    demo_verification()
    demo_star_concat()
