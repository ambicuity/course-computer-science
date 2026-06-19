# Lesson 02: Lexing I — Regex → DFA → Lexer

## Overview

The lexer is the first transformation in the compilation pipeline. It takes a stream of characters and produces a stream of **tokens** — categorized chunks like `KEYWORD_IF`, `IDENTIFIER("x")`, `INTEGER(42)`, `PLUS`. This lesson builds the theoretical foundation: how regular expressions describe token patterns, how those patterns become finite automata, and how automata drive a lexer.

---

## What Is a Lexer?

A **lexer** (also called a **scanner** or **tokenizer**) reads characters one at a time and groups them into tokens. Given input:

```
int x = 42 + 7;
```

The lexer produces:

```
[KEYWORD_INT] [IDENT("x")] [ASSIGN] [INT(42)] [PLUS] [INT(7)] [SEMI] [EOF]
```

Each token has a **type** (e.g., keyword, identifier, number) and optionally a **value** (the actual text or parsed number). Tokens also carry **source location** (line, column) for error messages.

---

## Token Types

A typical lexer recognizes these token categories:

| Category | Examples | Description |
|----------|----------|-------------|
| Keywords | `if`, `while`, `return` | Reserved words with fixed spelling |
| Identifiers | `x`, `count`, `main` | User-defined names |
| Numbers | `42`, `3.14`, `0xFF` | Integer and floating-point literals |
| Strings | `"hello"`, `'world'` | Quoted text |
| Operators | `+`, `==`, `&&` | Arithmetic, comparison, logical |
| Punctuation | `(`, `)`, `{`, `;` | Delimiters |
| Whitespace | spaces, tabs, newlines | Skipped (usually) |
| Comments | `// ...`, `/* ... */` | Skipped |

The lexer's job is to **match the longest possible token** at each position. This is the **longest match rule** (also called **maximal munch**).

---

## Regular Expressions

Each token type can be described by a **regular expression**:

| Token | Regex |
|-------|-------|
| Keyword | `if\|while\|return\|...` |
| Identifier | `[a-zA-Z_][a-zA-Z0-9_]*` |
| Integer | `[0-9]+` |
| Float | `[0-9]+\.[0-9]+` |
| String | `"[^"]*"` |
| Operator | `[+\-*/=<>!&|]+` |

Regular expressions are a compact notation for **regular languages** — the simplest class in the Chomsky hierarchy. Regular languages can be recognized by **finite automata**.

---

## From Regex to NFA: Thompson's Construction

**Thompson's construction** converts any regular expression into a **Nondeterministic Finite Automaton (NFA)**:

- **Character `a`** — NFA with two states, one transition labeled `a`.
- **Concatenation `AB`** — connect the accept state of A's NFA to the start state of B's NFA.
- **Alternation `A|B`** — new start state with ε-transitions to both A and B.
- **Kleene star `A*`** — ε-transition from accept back to start, and from new start to old start.

The NFA has at most **O(n)** states where n is the regex length. NFA simulation uses ε-closure (all states reachable via ε-transitions).

---

## From NFA to DFA: Subset Construction

An NFA can be in **multiple states simultaneously**. A **Deterministic Finite Automaton (DFA)** is in exactly one state at a time. DFAs are faster to execute.

**Subset construction** converts an NFA to a DFA:

1. The start state of the DFA is the ε-closure of the NFA's start state.
2. For each DFA state (a set of NFA states) and each input symbol, compute the ε-closure of all transitions — this becomes the next DFA state.
3. Repeat until no new DFA states are created.

The resulting DFA may have **exponentially many** states in the worst case (2^n for n NFA states), but in practice is manageable.

---

## DFA Minimization: Hopcroft's Algorithm

The DFA from subset construction may have redundant states. **Hopcroft's algorithm** minimizes it:

1. Partition states into accepting and non-accepting groups.
2. For each group, check if all states transition to the same group on each input symbol.
3. If not, split the group.
4. Repeat until no more splits occur.

The resulting minimal DFA has the fewest possible states for the given language.

---

## The Lexer as a Collection of DFAs

A practical lexer is not a single DFA — it is a **collection of DFAs**, one per token type, running in parallel. At each position, all DFAs advance. When a DFAs accepts, it is a candidate. The **longest match rule** resolves ties:

1. Run all DFAs from the current position.
2. Among those that accept, pick the one that consumed the most characters.
3. If still tied, use **priority** (keyword > identifier, for example).

This is exactly how tools like `lex` and `flex` work internally.

---

## Longest Match Rule

The longest match rule states: when multiple token patterns match, the one that consumes the most characters wins.

```
Input: "whilex"
Pattern 1: "while" (keyword)
Pattern 2: [a-z]+ (identifier)
```

The identifier pattern matches all 6 characters. The keyword matches 5. Longest match selects the identifier token `IDENT("whilex")`.

This is why `whilex` is a valid identifier in most languages — the longest match rule prevents the lexer from greedily consuming `while` as a keyword.

---

## Priority Rules

When two patterns match the same length, **priority** resolves the conflict. Typically:

- Keywords have higher priority than identifiers.
- Specific operators (`==`) have higher priority than general ones (`=`).

In a `lex`-style tool, priority is determined by the **order of rules** in the specification file — earlier rules win.

---

## Build It: Regex-to-DFA Lexer Generator

A lexer generator takes a set of (regex, token-type) pairs and produces a lexer:

1. Convert each regex to an NFA (Thompson).
2. Convert each NFA to a DFA (subset construction).
3. Minimize each DFA (Hopcroft).
4. At runtime, run all DFAs in parallel, apply longest match + priority.

---

## Use It: Lexer Generator Tools

### `lex` / `flex`

The classic Unix lexer generator. You write rules like:

```
"if"        { return KEYWORD_IF; }
[a-zA-Z_]+  { return IDENTIFIER; }
[0-9]+      { return INTEGER; }
```

`flex` generates a C file implementing a table-driven DFA.

### Rust's `logos`

A modern Rust lexer generator using derive macros:

```rust
#[derive(Logos, Debug, PartialEq)]
enum Token {
    #[token("if")]
    If,
    #[regex("[a-zA-Z_]+")]
    Ident,
    #[regex("[0-9]+")]
    Number,
}
```

`logos` compiles the regexes into DFA tables at compile time, yielding a zero-copy, zero-allocation lexer.

---

## Ship It: Lexer Generator

A minimal lexer generator needs:

1. A regex parser (regex → AST).
2. Thompson construction (regex AST → NFA).
3. Subset construction (NFA → DFA).
4. DFA minimization (Hopcroft).
5. Code emitter (DFA → lexer source code).

---

## Exercises

### Level 1: Trace

Given the regex `(a|b)*abb`, trace Thompson's construction to produce an NFA with states and transitions. Then trace subset construction to produce a DFA. Draw both automata.

### Level 2: Implement

Write a lexer in your language of choice that recognizes these tokens from a string input:

- `IF`, `ELSE`, `WHILE` (keywords)
- `IDENT(name)` (letters and digits, starting with a letter)
- `INT(value)` (sequence of digits)
- `PLUS`, `MINUS`, `ASSIGN`
- Skip whitespace

Use a hand-written DFA approach (state machine, not regex library).

### Level 3: Generator

Build a minimal lexer generator: given a list of (regex-string, token-name) pairs, compile them into a DFA table and use that table to lex input strings. Support longest match and priority ordering.
