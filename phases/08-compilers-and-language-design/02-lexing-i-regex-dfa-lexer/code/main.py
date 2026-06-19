"""Lesson 02: Regex-based Lexer — Python implementation.

A token-stream lexer that converts source text into a sequence of
classified tokens. Supports identifiers, keywords, integers, floats,
strings with escape sequences, operators, punctuation, and comments.
"""

from dataclasses import dataclass
from enum import Enum, auto
from typing import Optional


class TokenKind(Enum):
    KEYWORD = auto()
    IDENT = auto()
    INT = auto()
    FLOAT = auto()
    STRING = auto()
    OP = auto()
    LPAREN = auto()
    RPAREN = auto()
    LBRACE = auto()
    RBRACE = auto()
    SEMI = auto()
    COMMA = auto()
    ASSIGN = auto()
    EOF = auto()
    ERROR = auto()


@dataclass
class Token:
    kind: TokenKind
    value: str = ""
    line: int = 0
    col: int = 0

    def __str__(self) -> str:
        if self.kind == TokenKind.ERROR:
            return f"ERROR({self.line}:{self.col}: {self.value})"
        if self.kind == TokenKind.EOF:
            return "EOF"
        if self.value:
            return f"{self.kind.name}({self.value})"
        return self.kind.name


KEYWORDS = frozenset([
    "if", "else", "while", "for", "return", "fn", "let", "mut", "true", "false",
])


class Lexer:
    def __init__(self, source: str) -> None:
        self._chars = list(source)
        self._pos = 0
        self._line = 1
        self._col = 1

    # ------------------------------------------------------------------ helpers

    def _current(self) -> Optional[str]:
        if self._pos < len(self._chars):
            return self._chars[self._pos]
        return None

    def _peek(self) -> Optional[str]:
        nxt = self._pos + 1
        if nxt < len(self._chars):
            return self._chars[nxt]
        return None

    def _advance(self) -> Optional[str]:
        ch = self._current()
        if ch is None:
            return None
        self._pos += 1
        if ch == "\n":
            self._line += 1
            self._col = 1
        else:
            self._col += 1
        return ch

    # ------------------------------------------------------------------ skip

    def _skip_whitespace(self) -> None:
        while True:
            ch = self._current()
            if ch is None:
                return
            if ch.isspace():
                self._advance()
            elif ch == "/" and self._peek() == "/":
                while True:
                    c = self._advance()
                    if c is None or c == "\n":
                        break
            else:
                return

    # ------------------------------------------------------------------ scanners

    def _scan_identifier(self) -> Token:
        line, col = self._line, self._col
        name_chars: list[str] = []
        while True:
            ch = self._current()
            if ch is not None and (ch.isalnum() or ch == "_"):
                name_chars.append(ch)
                self._advance()
            else:
                break
        name = "".join(name_chars)
        kind = TokenKind.KEYWORD if name in KEYWORDS else TokenKind.IDENT
        return Token(kind, name, line, col)

    def _scan_number(self) -> Token:
        line, col = self._line, self._col
        parts: list[str] = []

        # prefix detection for hex / bin / oct
        if self._current() == "0":
            parts.append("0")
            self._advance()
            nxt = self._current()
            if nxt in ("x", "X"):
                parts.append(self._advance() or "")  # type: ignore[arg-type]
                while True:
                    ch = self._current()
                    if ch is not None and ch.isascii() and ch.isalnum():
                        parts.append(self._advance() or "")  # type: ignore[arg-type]
                    else:
                        break
                try:
                    int("".join(parts)[2:], 16)
                except ValueError:
                    return Token(TokenKind.ERROR, f"invalid hex literal: {''.join(parts)}", line, col)
                return Token(TokenKind.INT, "".join(parts), line, col)
            if nxt in ("b", "B"):
                parts.append(self._advance() or "")  # type: ignore[arg-type]
                while True:
                    ch = self._current()
                    if ch in ("0", "1"):
                        parts.append(self._advance() or "")  # type: ignore[arg-type]
                    else:
                        break
                return Token(TokenKind.INT, "".join(parts), line, col)
            if nxt in ("o", "O"):
                parts.append(self._advance() or "")  # type: ignore[arg-type]
                while True:
                    ch = self._current()
                    if ch is not None and "0" <= ch <= "7":
                        parts.append(self._advance() or "")  # type: ignore[arg-type]
                    else:
                        break
                return Token(TokenKind.INT, "".join(parts), line, col)

        # decimal digits
        while True:
            ch = self._current()
            if ch is not None and ch.isdigit():
                parts.append(self._advance() or "")  # type: ignore[arg-type]
            else:
                break

        # float
        if self._current() == "." and (self._peek() or "").isdigit():
            parts.append(self._advance() or "")  # type: ignore[arg-type]
            while True:
                ch = self._current()
                if ch is not None and ch.isdigit():
                    parts.append(self._advance() or "")  # type: ignore[arg-type]
                else:
                    break
            return Token(TokenKind.FLOAT, "".join(parts), line, col)

        return Token(TokenKind.INT, "".join(parts), line, col)

    def _scan_string(self, quote: str) -> Token:
        line, col = self._line, self._col
        self._advance()  # opening quote
        buf: list[str] = []
        while True:
            ch = self._current()
            if ch is None:
                return Token(TokenKind.ERROR, "unterminated string literal", line, col)
            if ch == quote:
                self._advance()  # closing quote
                return Token(TokenKind.STRING, "".join(buf), line, col)
            if ch == "\\":
                self._advance()
                esc = self._current()
                escapes = {"n": "\n", "t": "\t", "r": "\r", "\\": "\\", '"': '"', "'": "'", "0": "\0"}
                if esc in escapes:
                    buf.append(escapes[esc])
                    self._advance()
                elif esc is not None:
                    buf.append("\\")
                    buf.append(esc)
                    self._advance()
                else:
                    return Token(TokenKind.ERROR, "unterminated string literal", line, col)
            else:
                buf.append(ch)
                self._advance()

    def _scan_operator(self) -> Token:
        ch = self._advance()
        op = ch or ""
        nxt = self._current()
        two_char = {"==", "!=", "<=", ">=", "&&", "||"}
        if nxt is not None and (op + nxt) in two_char:
            op += self._advance()  # type: ignore[operator]
        return Token(TokenKind.OP, op, self._line, self._col)

    # ------------------------------------------------------------------ public

    def next_token(self) -> Token:
        self._skip_whitespace()
        ch = self._current()
        if ch is None:
            return Token(TokenKind.EOF, "", self._line, self._col)
        if ch.isalpha() or ch == "_":
            return self._scan_identifier()
        if ch.isdigit():
            return self._scan_number()
        if ch == '"':
            return self._scan_string('"')
        if ch == "'":
            return self._scan_string("'")
        simple = {
            "(": TokenKind.LPAREN,
            ")": TokenKind.RPAREN,
            "{": TokenKind.LBRACE,
            "}": TokenKind.RBRACE,
            ";": TokenKind.SEMI,
            ",": TokenKind.COMMA,
        }
        if ch in simple:
            self._advance()
            return Token(simple[ch], "", self._line, self._col)
        if ch == "=" and self._peek() != "=":
            self._advance()
            return Token(TokenKind.ASSIGN, "", self._line, self._col)
        if ch in "+-*/=!<>|&":
            return self._scan_operator()
        self._advance()
        return Token(TokenKind.ERROR, f"unexpected character: '{ch}'", self._line, self._col)

    def tokenize(self) -> list[Token]:
        tokens: list[Token] = []
        while True:
            tok = self.next_token()
            tokens.append(tok)
            if tok.kind == TokenKind.EOF:
                break
        return tokens


# --------------------------------------------------------------------------- main

def main() -> None:
    source = '''
fn main() {
    let x = 42 + 0xFF;
    let pi = 3.14;
    let name = "hello\\nworld";
    // This is a comment
    if x == 100 {
        return true;
    }
}
'''
    lexer = Lexer(source)
    for tok in lexer.tokenize():
        print(tok)


if __name__ == "__main__":
    main()
