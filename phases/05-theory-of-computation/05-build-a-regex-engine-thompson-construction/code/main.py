"""
Thompson Construction Regex Engine
===================================
Builds an NFA from a regex using Thompson's construction,
then simulates it to find all matches in a text.

Supports: literal, ., *, +, ?, |, (), [abc], [^abc]
"""

from __future__ import annotations
from dataclasses import dataclass, field
from typing import Iterator


# ---------------------------------------------------------------------------
# AST nodes
# ---------------------------------------------------------------------------

class Node:
    pass

@dataclass
class Lit(Node):
    char: str  # single character or '.' for any

@dataclass
class Alt(Node):
    left: Node
    right: Node

@dataclass
class Concat(Node):
    nodes: list[Node]

@dataclass
class Star(Node):
    node: Node

@dataclass
class Plus(Node):
    node: Node

@dataclass
class Opt(Node):
    node: Node

@dataclass
class CharClass(Node):
    chars: set[str]
    negated: bool = False


# ---------------------------------------------------------------------------
# Lexer — turns regex string into tokens
# ---------------------------------------------------------------------------

class Token:
    LPAREN = '('
    RPAREN = ')'
    LBRACKET = '['
    RBRACKET = ']'
    STAR = '*'
    PLUS = '+'
    QUESTION = '?'
    PIPE = '|'
    DOT = '.'
    BACKSLASH = '\\'
    CHAR = 'CHAR'
    EOF = 'EOF'

    def __init__(self, type_: str, value: str = ''):
        self.type = type_
        self.value = value


def lex(regex: str) -> list[Token]:
    tokens: list[Token] = []
    i = 0
    while i < len(regex):
        c = regex[i]
        if c == '\\' and i + 1 < len(regex):
            tokens.append(Token(Token.CHAR, regex[i + 1]))
            i += 2
        elif c in '()|*+?.':
            tokens.append(Token(c))
            i += 1
        elif c == '[':
            tokens.append(Token(Token.LBRACKET))
            i += 1
        elif c == ']':
            tokens.append(Token(Token.RBRACKET))
            i += 1
        else:
            tokens.append(Token(Token.CHAR, c))
            i += 1
    tokens.append(Token(Token.EOF))
    return tokens


# ---------------------------------------------------------------------------
# Parser — recursive descent
# ---------------------------------------------------------------------------

class Parser:
    def __init__(self, tokens: list[Token]):
        self.tokens = tokens
        self.pos = 0

    def peek(self) -> Token:
        return self.tokens[self.pos]

    def advance(self) -> Token:
        t = self.tokens[self.pos]
        self.pos += 1
        return t

    def parse(self) -> Node:
        node = self.alt()
        if self.peek().type != Token.EOF:
            raise SyntaxError(f"Unexpected token: {self.peek().type}")
        return node

    def alt(self) -> Node:
        node = self.concat()
        while self.peek().type == Token.PIPE:
            self.advance()  # consume |
            right = self.concat()
            node = Alt(node, right)
        return node

    def concat(self) -> Node:
        nodes: list[Node] = []
        while self.peek().type not in (Token.EOF, Token.PIPE, Token.RPAREN):
            nodes.append(self.repeat())
        if len(nodes) == 0:
            raise SyntaxError("Empty concat")
        if len(nodes) == 1:
            return nodes[0]
        return Concat(nodes)

    def repeat(self) -> Node:
        node = self.atom()
        while self.peek().type in (Token.STAR, Token.PLUS, Token.QUESTION):
            t = self.advance()
            if t.type == Token.STAR:
                node = Star(node)
            elif t.type == Token.PLUS:
                node = Plus(node)
            elif t.type == Token.QUESTION:
                node = Opt(node)
        return node

    def atom(self) -> Node:
        t = self.peek()
        if t.type == Token.LPAREN:
            self.advance()  # consume (
            node = self.alt()
            if self.peek().type != Token.RPAREN:
                raise SyntaxError("Missing )")
            self.advance()  # consume )
            return node
        if t.type == Token.LBRACKET:
            return self.char_class()
        if t.type == Token.DOT:
            self.advance()
            return Lit('.')
        if t.type == Token.CHAR:
            self.advance()
            return Lit(t.value)
        raise SyntaxError(f"Unexpected token: {t.type}")

    def char_class(self) -> Node:
        self.advance()  # consume [
        negated = False
        if self.peek().type == Token.CHAR and self.peek().value == '^':
            negated = True
            self.advance()
        chars: set[str] = set()
        while self.peek().type != Token.RBRACKET:
            if self.peek().type == Token.EOF:
                raise SyntaxError("Missing ]")
            t = self.advance()
            if t.type == Token.CHAR:
                chars.add(t.value)
            elif t.type == Token.DOT:
                chars.add('.')
            else:
                chars.add(t.value)
        self.advance()  # consume ]
        return CharClass(chars, negated)


def parse(regex: str) -> Node:
    tokens = lex(regex)
    return Parser(tokens).parse()


# ---------------------------------------------------------------------------
# NFA
# ---------------------------------------------------------------------------

class State:
    _id_counter = 0

    def __init__(self):
        self.id = State._id_counter
        State._id_counter += 1

    def __repr__(self):
        return f"s{self.id}"

    def __hash__(self):
        return self.id

    def __eq__(self, other):
        return isinstance(other, State) and self.id == other.id


@dataclass
class NFA:
    start: State
    accept: State
    transitions: dict[tuple[State, str], set[State]] = field(default_factory=dict)

    def add(self, src: State, char: str, dst: State):
        key = (src, char)
        if key not in self.transitions:
            self.transitions[key] = set()
        self.transitions[key].add(dst)

    def next_states(self, state: State, char: str) -> set[State]:
        return self.transitions.get((state, char), set())

    def eps_closure(self, states: set[State]) -> set[State]:
        """Compute ε-closure of a set of states."""
        stack = list(states)
        closure = set(states)
        while stack:
            s = stack.pop()
            for t in self.next_states(s, ''):
                if t not in closure:
                    closure.add(t)
                    stack.append(t)
        return closure


# ---------------------------------------------------------------------------
# Thompson Construction — AST → NFA
# ---------------------------------------------------------------------------

def thompson(node: Node) -> NFA:
    if isinstance(node, Lit):
        start = State()
        accept = State()
        nfa = NFA(start, accept)
        nfa.add(start, node.char, accept)
        return nfa

    if isinstance(node, CharClass):
        start = State()
        accept = State()
        nfa = NFA(start, accept)
        if node.negated:
            nfa.add(start, f'[^{"".join(sorted(node.chars))}]', accept)
        else:
            for c in sorted(node.chars):
                nfa.add(start, c, accept)
        return nfa

    if isinstance(node, Alt):
        nfa_left = thompson(node.left)
        nfa_right = thompson(node.right)
        start = State()
        accept = State()
        nfa = NFA(start, accept)
        nfa.transitions.update(nfa_left.transitions)
        nfa.transitions.update(nfa_right.transitions)
        nfa.add(start, '', nfa_left.start)
        nfa.add(start, '', nfa_right.start)
        nfa.add(nfa_left.accept, '', accept)
        nfa.add(nfa_right.accept, '', accept)
        return nfa

    if isinstance(node, Concat):
        nfas = [thompson(n) for n in node.nodes]
        for i in range(len(nfas) - 1):
            nfas[i].add(nfas[i].accept, '', nfas[i + 1].start)
        start = nfas[0].start
        accept = nfas[-1].accept
        nfa = NFA(start, accept)
        for nf in nfas:
            nfa.transitions.update(nf.transitions)
        return nfa

    if isinstance(node, Star):
        inner = thompson(node.node)
        start = State()
        accept = State()
        nfa = NFA(start, accept)
        nfa.transitions.update(inner.transitions)
        nfa.add(start, '', inner.start)
        nfa.add(start, '', accept)
        nfa.add(inner.accept, '', inner.start)
        nfa.add(inner.accept, '', accept)
        return nfa

    if isinstance(node, Plus):
        # A+ = A A*
        inner = thompson(node.node)
        star_start = State()
        star_accept = State()
        nfa = NFA(inner.start, star_accept)
        nfa.transitions.update(inner.transitions)
        nfa.add(inner.accept, '', star_start)
        nfa.add(star_start, '', inner.start)
        nfa.add(star_start, '', star_accept)
        nfa.add(inner.accept, '', star_accept)
        return nfa

    if isinstance(node, Opt):
        # A? = A | ε
        inner = thompson(node.node)
        start = State()
        accept = State()
        nfa = NFA(start, accept)
        nfa.transitions.update(inner.transitions)
        nfa.add(start, '', inner.start)
        nfa.add(start, '', accept)
        nfa.add(inner.accept, '', accept)
        return nfa

    raise TypeError(f"Unknown node type: {type(node)}")


# ---------------------------------------------------------------------------
# NFA Simulation
# ---------------------------------------------------------------------------

def simulate(nfa: NFA, text: str) -> list[tuple[int, int]]:
    """Find all match positions in text. Returns list of (start, end) tuples."""
    matches: list[tuple[int, int]] = []

    for start_pos in range(len(text)):
        current = nfa.eps_closure({nfa.start})
        matched = nfa.accept in current

        for i in range(start_pos, len(text)):
            next_states: set[State] = set()
            for s in current:
                # Try exact char
                for t in nfa.next_states(s, text[i]):
                    next_states.add(t)
                # Try '.' wildcard
                for t in nfa.next_states(s, '.'):
                    next_states.add(t)
                # Try character class negation
                for (src, label), dsts in nfa.transitions.items():
                    if src == s and label.startswith('[^') and label.endswith(']'):
                        excluded = set(label[2:-1])
                        if text[i] not in excluded:
                            next_states.update(dsts)

            current = nfa.eps_closure(next_states)
            if nfa.accept in current:
                matched = True
                end = i + 1
                matches.append((start_pos, end))
                break

            if not current:
                break

        if matched and nfa.accept not in current and len(matches) > 0:
            pass  # already recorded

    # Deduplicate and sort
    matches = sorted(set(matches))
    return matches


def match(pattern: str, text: str) -> list[tuple[int, int]]:
    """Regex match using Thompson construction."""
    ast = parse(pattern)
    nfa = thompson(ast)
    return simulate(nfa, text)


# ---------------------------------------------------------------------------
# Demo
# ---------------------------------------------------------------------------

def demo():
    print("=" * 60)
    print("Thompson Construction Regex Engine")
    print("=" * 60)

    tests = [
        # (pattern, text, description)
        ("a", "banana", "Single literal"),
        ("a*", "aaab", "Kleene star"),
        ("a|b", "cabd", "Union"),
        ("(a|b)*abb", "aabbababbab", "Classic example"),
        ("a+b", "aaab ab", "One-or-more"),
        ("colou?r", "color colour", "Optional"),
        (".*end", "the end", "Dot wildcard"),
        ("[abc]+", "aabccba", "Character class"),
    ]

    for pattern, text, desc in tests:
        print(f"\nPattern: {pattern!r}  Text: {text!r}  ({desc})")
        results = match(pattern, text)
        if results:
            for s, e in results:
                print(f"  Match [{s}:{e}] = {text[s:e]!r}")
        else:
            print("  No match")

    print("\n" + "=" * 60)
    print("Engine supports: literal, ., *, +, ?, |, (), [abc], [^abc]")
    print("=" * 60)


if __name__ == "__main__":
    import sys
    if len(sys.argv) >= 3:
        pattern = sys.argv[1]
        text = sys.argv[2]
        results = match(pattern, text)
        if results:
            for s, e in results:
                print(f"Match [{s}:{e}]: {text[s:e]}")
        else:
            print("No match")
    else:
        demo()
