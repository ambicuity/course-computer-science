# Lesson 03: Lexing II — Hand-Written Scanners

## Overview

Lexer generators like `flex` and `logos` are powerful, but most production compilers — GCC, Clang, Go, rustc — **hand-write** their lexers. This lesson explains why, and teaches you to build a scanner from scratch using a state-machine approach.

---

## Why Hand-Write a Lexer?

Lexer generators are convenient but have limitations:

- **Better error messages** — a hand-written lexer can produce precise, context-aware diagnostics ("expected closing quote for string started on line 5").
- **Context-sensitive tokens** — Python's indentation, C's preprocessor directives, and template language delimiters require state that DFAs cannot easily express.
- **Performance** — a tight loop with a `match`/`switch` on the current character is faster than table-driven DFA dispatch.
- **No external dependency** — no build-time code generation step.
- **Easier debugging** — the code is plain, readable logic, not generated tables.

The trade-off: more code to write and maintain.

---

## The State Machine Approach

A hand-written scanner is a **state machine**. At each point, the scanner examines the current character and decides:

1. Which token type to begin scanning.
2. How far to advance.
3. When the token ends.

The core loop:

```
loop:
    skip whitespace
    if at end: return EOF
    match current_char:
        alphabetic → scan_identifier()
        digit      → scan_number()
        '"'        → scan_string()
        '+'        → advance(); return PLUS
        ...
```

Each `scan_*` function is itself a mini state machine that consumes characters until the token is complete.

---

## Keywords vs Identifiers

The most common trick in hand-written lexers: **scan the full identifier, then check a keyword table**.

```
scan_identifier():
    start = position
    while current_char is alphanumeric or '_':
        advance()
    text = source[start..position]
    if text in KEYWORD_TABLE:
        return KEYWORD(text)
    else:
        return IDENT(text)
```

This is simpler than hard-coding keyword patterns. Adding a new keyword means adding one entry to the table.

---

## String Scanning with Escape Sequences

Strings require handling escape sequences inside quoted delimiters:

```
scan_string(quote_char):
    advance()  // skip opening quote
    buffer = ""
    loop:
        if at end: error "unterminated string"
        if current_char == quote_char:
            advance()
            return STRING(buffer)
        if current_char == '\\':
            advance()
            match current_char:
                'n' → buffer += '\n'; advance()
                't' → buffer += '\t'; advance()
                '\\' → buffer += '\\'; advance()
                '"' → buffer += '"'; advance()
                ...
        else:
            buffer += current_char
            advance()
```

---

## Number Scanning

Numbers come in multiple forms:

| Form | Example | Rule |
|------|---------|------|
| Decimal integer | `42` | `[0-9]+` |
| Float | `3.14` | `[0-9]+\.[0-9]+` |
| Hex integer | `0xFF` | `0x[0-9a-fA-F]+` |
| Binary integer | `0b1010` | `0b[01]+` |
| Octal integer | `0o755` | `0o[0-7]+` |

The scanner peeks ahead to distinguish:

1. Start with `0` and next char is `x`, `b`, or `o` → special prefix format.
2. Digits followed by `.` and more digits → float.
3. Otherwise → integer.

---

## Error Recovery

A good scanner does not halt on the first error. It records the error, skips invalid characters, and continues scanning. This lets the compiler report multiple errors in one pass:

```
if current_char is unrecognized:
    emit_error("unexpected character: '{}'", current_char)
    advance()
    continue scanning
```

---

## Build It: Hand-Written Scanner

Design a scanner for a mini-language with:

- Keywords: `if`, `else`, `while`, `fn`, `let`, `return`, `true`, `false`
- Identifiers: `[a-zA-Z_][a-zA-Z0-9_]*`
- Integers: decimal, hex (`0x`), binary (`0b`)
- Floats: `[0-9]+\.[0-9]+`
- Strings: double-quoted with `\n`, `\t`, `\\`, `\"`
- Operators: `+`, `-`, `*`, `/`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`
- Punctuation: `(`, `)`, `{`, `}`, `;`, `,`, `=`
- Single-line comments: `// ...`
- Error reporting with line and column

---

## Use It: Production Hand-Written Lexers

### GCC

GCC's lexer (`libcpp/`) is entirely hand-written. It handles C's preprocessor directives as part of lexing, requiring state tracking that a DFA cannot manage.

### Clang

Clang's lexer (`clang/lib/Lex/`) is hand-written. It handles trigraphs, raw string literals, and preprocessor tokens with context-dependent rules.

### Go

The Go compiler's scanner (`go/scanner/`) is a clean, compact hand-written lexer — about 800 lines. It demonstrates that a production lexer does not need to be enormous.

---

## Ship It: Scanner Library

A reusable scanner library provides:

1. A `Token` type with variant data.
2. A `Scanner` struct holding input, position, line, column.
3. `scan_token()` dispatching on the current character.
4. Helper methods: `scan_identifier`, `scan_number`, `scan_string`.
5. Error accumulation (do not stop on first error).

---

## Exercises

### Level 1: Extend

Take the Rust scanner from this lesson and add support for:

- Multi-line comments (`/* ... */`)
- Character literals (`'a'`, `'\n'`)
- Underscores in number literals (`1_000_000`)

### Level 2: Port

Rewrite the C scanner in Python. Add indentation tracking: when the scanner encounters a newline followed by spaces, emit `INDENT(n)` and `DEDENT(n)` tokens (like Python).

### Level 3: Build

Design and implement a scanner for a JSON-like language:

- Objects: `{ "key": value }`
- Arrays: `[1, 2, 3]`
- Strings with full escape support (`\uXXXX` unicode escapes)
- Numbers: integer, float, scientific notation (`1.5e10`)
- Booleans: `true`, `false`
- Null: `null`
- Whitespace: skip

The scanner must report all errors with line and column, and continue scanning after errors.
