# Lesson 06 — Parsing III: LR, SLR, LALR, GLR

## Bottom-Up Parsing

LR parsers build the parse tree **bottom-up**, starting from the leaves (tokens) and working toward the root. They perform a **rightmost derivation in reverse** — the opposite of LL parsers, which do top-down leftmost derivations.

**LR** means:

- **L**: Left-to-right scan of input
- **R**: Rightmost derivation in reverse

LR parsers handle a strictly larger class of grammars than LL, including many with left recursion and fewer restrictions.

## Shift-Reduce Parsing

An LR parser uses a **stack** and two basic actions:

| Action | Meaning |
|--------|---------|
| **Shift** | Push the current input token and its state onto the stack, advance input |
| **Reduce** | Pop the right-hand side of a production, push the left-hand side non-terminal (with a new state) |

The parser also has **Accept** (when start symbol is on stack and input is `$`) and **Error** (when no valid action exists).

The decision of shift vs. reduce is driven by an **action table** indexed by `(state, lookahead token)`.

## Items and States

An **LR item** is a production with a dot `·` marking how much has been matched:

```
E → E · '+' T      ← we've matched E, expect '+' T next
E → E '+' · T      ← we've matched E '+', expect T next
```

An **item set** (a parser **state**) is a set of items representing all possibilities at a given point. States are connected by **transitions** on grammar symbols:

- `GOTO(I, X)` = closure of all items obtained by moving the dot past symbol X in items of I

The collection of all states and transitions forms the **LR(0) automaton**.

## Closure

The **closure** of an item set I is computed by repeatedly adding items: if `[A → α · B β]` is in I and B → γ is a production, add `[B → · γ]`. This accounts for the fact that we might start matching B next.

## SLR: Simple LR

**SLR(1)** is the simplest LR variant. It builds LR(0) states, then uses **FOLLOW sets** to resolve conflicts:

- For a state s and lookahead a: **shift** if there is a transition on `a` from s. **Reduce** by `A → α` if the item `[A → α ·]` is in state s and `a ∈ FOLLOW(A)`.

SLR works well for many grammars but can have **shift-reduce** or **reduce-reduce** conflicts even for unambiguous grammars where FOLLOW sets are too coarse.

## Shift-Reduce and Reduce-Reduce Conflicts

Two types of conflicts can arise in the action table:

**Shift-reduce conflict**: A state contains a complete item `[A → α ·]` and also has a transition on some terminal `a`. The parser must choose between reducing by `A → α` or shifting `a`. Example — the dangling-else problem: after seeing `if E then S`, should we reduce or shift `else`?

**Reduce-reduce conflict**: A state contains two complete items `[A → α ·]` and `[B → β ·]` with overlapping lookaheads. The parser cannot decide which production to reduce by. This usually indicates a genuine grammar problem.

Resolution strategies:

- **Precedence declarations**: Assign precedence levels to terminals and rules (as in Bison's `%left`, `%right`, `%nonassoc`, `%prec`). Shift-reduce conflicts are resolved by comparing precedence.
- **Default actions**: Some tools prefer shift over reduce (shift wins) or reduce by the earlier production (reduce-reduce).
- **Grammar rewrite**: Eliminate the ambiguity by restructuring productions.

## LR(1): Canonical LR

An **LR(1) item** is a pair: `(production with dot, lookahead)`. For example:

```
[A → α · β, a]   ← a is the lookahead that must follow this production for reduction
```

LR(1) states are much more precise than LR(0) states because lookahead information refines reduce decisions. The downside: canonical LR(1) tables can be **enormous** — thousands of states for a real grammar.

The closure rule for LR(1) also propagates lookaheads: if `[A → α · B β, a]` is in the state, and `FIRST(βa)` contains terminal `b`, add `[B → · γ, b]` for every production `B → γ`.

## LALR(1): Lookahead LR

**LALR(1)** merges LR(1) states that share the same **core** (same items ignoring lookaheads). This produces the same number of states as SLR (typically a few hundred) while retaining most of LR(1)'s power.

Key properties:

- Tables are much smaller than LR(1).
- No more powerful than LR(1) — some LR(1) grammars develop reduce-reduce conflicts after merging.
- Equally powerful as SLR for most practical grammars.
- **Yacc/Bison** use LALR(1) by default.

## GLR: Generalized LR

**GLR** handles **ambiguous grammars** by **forking** the parser: when a conflict is encountered, the parser splits into multiple parallel stacks, each exploring a different path. Invalid paths are pruned when they hit errors. If multiple paths reach the end, the grammar is genuinely ambiguous.

GLR is used by **tree-sitter** (the parsing library behind many editor integrations) and optionally by **Bison** (`%glr-parser`).

## LR Power Hierarchy

The LR family forms a strict hierarchy of grammar classes:

```
SLR(1) ⊂ LALR(1) ⊂ LR(1) ⊂ LR(k)  for k > 1
```

Every SLR grammar is LALR, every LALR grammar is LR(1), and so on. The inclusion is strict — there exist grammars in LALR that are not SLR, and grammars in LR(1) that are not LALR.

In practice, LALR(1) handles virtually all programming language grammars. The grammars that require full LR(1) tend to be pathological or artificial.

## The GOTO Table

The **goto table** maps `(state, nonterminal) → state`. After a reduce, the parser pops the right-hand side, exposing a previous state. It then looks up `goto[previous_state][lhs]` to find the new state to push. The goto table is essentially the DFA transitions on nonterminals.

## Error Recovery in LR Parsers

When the parser hits a state with no valid action for the current lookahead, it can:

1. **Pop states** from the stack until a state is found where the error token has a valid shift or goto action.
2. **Shift a special error token** (if the grammar includes `error` productions, as in Yacc).
3. **Report** the error and continue.

This "panic-mode" approach mirrors recursive-descent synchronization but operates on the stack rather than the token stream.

## Comparison

| Variant | States | Handles | Table Size | Used By |
|---------|--------|---------|------------|---------|
| SLR | LR(0) | Simple grammars | Small | Educational |
| Canonical LR(1) | LR(1) | Most grammars | Very large | Rare in practice |
| LALR(1) | Merged LR(1) | Most practical grammars | Moderate | Yacc, Bison (default) |
| GLR | Forking | Ambiguous grammars | Moderate | tree-sitter, Bison option |

## Build It: SLR Parser Table Builder and Parser

Our Python implementation constructs LR(0) states, builds the SLR action/goto table, and runs a shift-reduce parser on example grammars — including one that exhibits shift-reduce conflicts.

## Use It

- **Yacc/Bison**: Industry-standard parser generators, LALR(1) by default.
- **tree-sitter**: Uses GLR for robust parsing of real-world source code.
- **ANTLR**: LL(\*) but conceptually inspired the same grammar-specification workflow.

## Ship It

Our code produces an SLR parser. Given a grammar and input tokens, it either produces a rightmost derivation in reverse or reports conflicts and syntax errors.

## Exercises

**Level 1 — Warm-Up:**
Build the LR(0) automaton for a simple grammar with three productions (e.g., `S → A B`, `A → a`, `B → b`). Draw the states and transitions by hand and verify them against the code output.

**Level 2 — Intermediate:**
Take a grammar that is LL(1) and convert it to one that requires LALR(1) (introduce left recursion). Show that the SLR table handles it while the LL(1) table would not.

**Level 3 — Challenge:**
Implement the **dangling-else** grammar (`S → i E t S | i E t S e S | a`, `E → b`) and observe the shift-reduce conflict. Resolve it using **precedence declarations** (like Bison's `%prec`) to prefer shift over reduce, correctly handling the dangling-else ambiguity.
