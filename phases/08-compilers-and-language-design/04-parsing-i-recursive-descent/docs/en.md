# Lesson 04 — Parsing I: Recursive Descent

## What Is Parsing?

A **parser** converts a stream of tokens (from the lexer) into an **Abstract Syntax Tree (AST)** — a tree that captures the grammatical structure of the input. The parser enforces the rules of a **grammar**, rejecting strings that violate the language's syntax.

```
Source → [Lexer] → tokens → [Parser] → AST → [Code Gen]
```

If lexing answers *"what words are here?"*, parsing answers *"what does this sentence mean structurally?"*

## Grammars: BNF and EBNF

A **context-free grammar** consists of **production rules** that describe how to build strings from a set of non-terminals and terminals.

**BNF** (Backus–Naur Form):

```
expr   ::= term (('+' | '-') term)*
term   ::= factor (('*' | '/') factor)*
factor ::= '-' factor | atom
atom   ::= NUMBER | '(' expr ')'
```

**EBNF** adds `*` (zero or more), `+` (one or more), `?` (optional), and grouping `(...)`. These are shorthand — every EBNF rule can be rewritten in pure BNF.

Key terms:

- **Non-terminal**: a symbol that appears on the left side of a rule (`expr`, `term`, `factor`, `atom`).
- **Terminal**: a literal token (`'+'`, `NUMBER`).
- **Production**: a single rule (`factor ::= atom`).
- **Start symbol**: the root non-terminal you begin deriving from.

## Recursive Descent: The Idea

**Recursive descent** is the simplest parsing strategy. The rule:

> **Write one function per non-terminal. Each function tries to match its production by calling other functions for non-terminals it references.**

For the grammar above, you'd write:

- `parse_expr()` — matches a term, then loops over `+ term` or `- term`
- `parse_term()` — matches a factor, then loops over `* factor` or `/ factor`
- `parse_factor()` — checks for unary `-`, otherwise calls `parse_atom()`
- `parse_atom()` — matches a `NUMBER` or `'(' expr ')'`

Each function consumes tokens and returns an AST node. The call stack mirrors the parse tree.

## Left Recursion Must Be Eliminated

A grammar rule is **left-recursive** if a non-terminal can derive itself as the first symbol:

```
expr ::= expr '+' term | term      ← left-recursive!
```

Recursive descent would call `parse_expr()` from `parse_expr()` with no tokens consumed — infinite loop.

**Fix**: rewrite using iteration (Kleene star):

```
expr ::= term (('+' | '-') term)*
```

This is equivalent. Every left-recursive rule can be mechanically rewritten. Right recursion (`factor ::= '-' factor | atom`) is fine because it consumes a token before recursing.

## Operator Precedence via Grammar Structure

Precedence is encoded by **nesting non-terminals**:

| Level   | Grammar rule                   | Operators |
|---------|--------------------------------|-----------|
| Lowest  | `expr  → term (('+' '-') term)*` | `+ -`   |
| Medium  | `term  → factor (('* '/') factor)*` | `* /` |
| Highest | `factor → '-' factor \| atom`   | unary `-` |
| Atom    | `atom  → NUMBER \| '(' expr ')'` | grouping |

Because `parse_expr` calls `parse_term`, which calls `parse_factor`, multiplication binds tighter than addition. No explicit precedence table needed — the grammar encodes it structurally.

## Error Recovery: Synchronize on Expected Tokens

When the parser encounters an unexpected token, it can:

1. **Report** the error with position and expected vs. found token.
2. **Synchronize**: skip tokens until a known "safe" token appears (e.g., `;`, `)`, or an operator). Resume parsing from there.

Simple recovery avoids cascading errors from a single mistake. More advanced strategies include **panic-mode recovery** (skip to a synchronizing token), **phrase-level recovery** (insert or delete tokens locally), and **error productions** (add grammar rules for common mistakes).

## Associativity

Operators can be left-associative (`1 - 2 - 3` = `(1 - 2) - 3`) or right-associative (`2 ^ 3 ^ 4` = `2 ^ (3 ^ 4)`). The grammar encodes this:

- **Left-associative**: use iteration (`term (('+' | '-') term)*`) — builds the tree left-heavy.
- **Right-associative**: use direct right recursion (`factor → atom ('^' factor)?`) — builds the tree right-heavy.

Our grammar uses iteration for `+ - * /` (left-associative) and right recursion for unary `-`.

## Recursive Descent vs. Table-Driven

| Property | Recursive Descent | Table-Driven (LR, LL) |
|----------|-------------------|-----------------------|
| Implementation | One function per rule | Table + stack loop |
| Grammar class | LL(k) — limited | LR(1) — larger |
| Left recursion | Must be eliminated | LR handles natively |
| Error messages | Full control | Harder to customise |
| Debugging | Step through functions | Trace stack states |
| Used in | GCC, Clang, Go, V8 | Yacc, Bison, tree-sitter |

Hand-written recursive descent is the dominant choice for production compilers today, despite LR's theoretical advantages.

## Build It: Recursive Descent Arithmetic Parser

We'll build a complete parser in Rust that handles:

- Integers and parentheses
- Unary negation
- `+ - * /` with correct precedence
- Clean error messages with byte positions

## Use It

**GCC** and **Clang** use hand-written recursive-descent parsers for C and C++. Most "real" production compilers favour this style because:

- The parser is easy to read, debug, and extend by hand.
- Error messages can be custom-tailored.
- No code-generation tool (like Yacc/Bison) is required.

## Ship It: Parser as a Library

Our code compiles as a Rust library crate. Downstream code calls `parse(tokens) -> Result<Expr, String>`, making it embeddable in any pipeline.

## Exercises

**Level 1 — Warm-Up:**
Extend the parser to support the exponentiation operator `^`. `^` should be right-associative and have the highest precedence (above unary `-`). Update the grammar, AST, and parser functions.

**Level 2 — Intermediate:**
Add support for variable names as atoms. A variable is an identifier token. Extend `Atom` to include `Var(String)`. Parse expressions like `x + 2 * y`.

**Level 3 — Challenge:**
Add function call syntax: `NAME '(' args ')'` where `args` is a comma-separated list of expressions. `f(x, y + 1)` should produce a `Call` AST node. Make sure `f` without parens is still parsed as a variable, not a call.
