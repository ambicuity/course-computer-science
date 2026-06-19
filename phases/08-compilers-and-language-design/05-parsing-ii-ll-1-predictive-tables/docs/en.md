# Lesson 05 — Parsing II: LL(1) and Predictive Tables

## From Recursive Descent to Tables

Recursive descent works well when a human writes the parser by hand. But what if we want to **generate** a parser automatically from a grammar? That leads to **predictive parsing** — a table-driven approach that makes exactly one production choice per non-terminal and lookahead token.

## LL(1): What It Means

**LL(1)** stands for:

- **L**eft-to-right scan of input
- **L**eftmost derivation
- **1** token of lookahead

An LL(1) parser never needs to backtrack. Given the current non-terminal and the next input token, it can deterministically pick the correct production.

## FIRST Sets

The **FIRST** set of a sequence of grammar symbols is the set of terminals that can *begin* a string derived from that sequence.

Rules for computing FIRST(X):

1. If X is a terminal: FIRST(X) = { X }
2. If X → ε is a production: add ε to FIRST(X)
3. If X → Y₁ Y₂ … Yₖ: add FIRST(Y₁) − {ε} to FIRST(X). If Y₁ can derive ε, also add FIRST(Y₂), and so on.

Example grammar:

```
E  → T E'
E' → '+' T E' | ε
T  → F T'
T' → '*' F T' | ε
F  → '(' E ')' | id
```

FIRST sets:

| Symbol | FIRST |
|--------|-------|
| E, T, F | { `(`, `id` } |
| E' | { `+`, ε } |
| T' | { `*`, ε } |

## FOLLOW Sets

The **FOLLOW** set of a non-terminal A is the set of terminals that can appear *immediately after* A in some sentential form.

Rules:

1. Put `$` (end marker) in FOLLOW(S), where S is the start symbol.
2. If A → α B β, add FIRST(β) − {ε} to FOLLOW(B).
3. If A → α B, or A → α B β where β ⇒* ε, add FOLLOW(A) to FOLLOW(B).

For our grammar:

| Non-terminal | FOLLOW |
|-------------|--------|
| E | { `)`, `$` } |
| E' | { `)`, `$` } |
| T | { `+`, `)`, `$` } |
| T' | { `+`, `)`, `$` } |
| F | { `*`, `+`, `)`, `$` } |

## Building the Predictive Parsing Table

For each production A → α and each terminal `a` in FIRST(α):

- Add A → α to table[A][a]

If ε ∈ FIRST(α), also add A → α to table[A][b] for each `b` in FOLLOW(A).

**LL(1) condition**: The grammar is LL(1) if and only if every cell in the table contains at most one production. Two conflict types:

- **FIRST/FIRST conflict**: two productions for the same non-terminal and lookahead → grammar is ambiguous for that lookahead.
- **FIRST/FOLLOW conflict**: a production can be chosen by both a FIRST entry and a FOLLOW (ε) entry.

## The LL(1) Parsing Algorithm

Uses a **stack** and the parsing table:

```
push $ onto stack
push start symbol onto stack
read first token

while stack.top != $:
    X = stack.top
    if X is a terminal:
        if X == current_token: pop, advance
        else: error
    else:
        look up table[X][current_token]
        if entry is X → Y1 Y2 ... Yk:
            pop X, push Yk, ..., Y1 (reverse order)
        else: error

if current_token == $: accept
else: error
```

The parser always knows what to do — no backtracking, no ambiguity.

## Build It: LL(1) Parser Generator

Our Python implementation will:

1. Accept a grammar in a simple dictionary format
2. Compute FIRST sets
3. Compute FOLLOW sets
4. Build the predictive parsing table
5. Check the LL(1) condition
6. Parse input using the table-driven algorithm

## Use It

LL(1) grammars are used in educational settings and some hand-coded parsers. Many practical languages are not strictly LL(1), which is why tools like ANTLR (which uses adaptive LL(\*) with lookahead beyond 1 token) are popular. The LL(1) theory, however, is foundational.

## Ship It

Our code produces a standalone LL(1) parser. Given a grammar and an input string, it either produces a leftmost derivation or reports a syntax error.

## Exercises

**Level 1 — Warm-Up:**
Add FIRST and FOLLOW set computation for a new grammar of your choosing (e.g., a simple assignment statement grammar). Verify the sets by hand before checking the code.

**Level 2 — Intermediate:**
Extend the parser to output the full **leftmost derivation** as a sequence of sentential forms, not just accept/reject. Show each production applied.

**Level 3 — Challenge:**
Implement error recovery: when the parser detects an error, skip tokens until a token in FOLLOW(current non-terminal) is found, then pop the non-terminal and continue. Report all errors, not just the first.
