# Lesson 10: Parsing Theory — CYK and Earley

## Why This Matters

Every compiler, every query language, every natural-language assistant relies on parsing. Given a string and a grammar, does the string belong to the language? And if so, *how* is it structured? Two classic algorithms answer this from opposite directions: CYK builds meaning from the bottom up, Earley works top-down and bottom-up simultaneously. Understanding both gives you the theoretical foundation behind every parser generator you will ever use — from yacc and ANTLR to tree-sitter.

---

## CYK Algorithm

The Cocke–Younger–Kasami algorithm is a **bottom-up dynamic programming** parser. Independently discovered by John Cocke, Daniel Younger, and Tadao Kasami around 1965, it was the first polynomial-time parsing algorithm for arbitrary context-free grammars. It works on grammars in **Chomsky Normal Form (CNF)** — every rule is either `A → BC` or `A → a`.

### Key Idea

For a string `s = s₁s₂…sₙ`, maintain a table `T[i][j]` = the set of nonterminals that derive the substring `s[i..j]` (0-indexed, inclusive). The algorithm fills this table bottom-up: first the diagonal (single characters), then length-2 substrings, and so on.

### Algorithm

```
for j = 0 to n-1:
    T[j][j] = { A : A → s[j] is a rule }

for span = 2 to n:
    for i = 0 to n - span:
        j = i + span - 1
        T[i][j] = {}
        for k = i to j - 1:
            for each rule A → B C:
                if B ∈ T[i][k] and C ∈ T[k+1][j]:
                    T[i][j] = T[i][j] ∪ {A}
```

### Worked Example

Grammar: `S → AB, A → a, B → b`. Input: `"ab"`.

- `T[0][0] = {A}` because `A → a`.
- `T[1][1] = {B}` because `B → b`.
- `T[0][1]`: try `k=0`. `S → AB` — is `A ∈ T[0][0]`? Yes. Is `B ∈ T[1][1]`? Yes. So `T[0][1] = {S}`.
- `S ∈ T[0][1]` → accepted.

### Complexity

- **Time:** O(n³ · |G|) — three nested loops over the string, multiplied by the grammar size for rule checks at each step.
- **Space:** O(n² · |V|) — the table stores a set of variables at each (i, j) pair.

### CNF Conversion

Any CFG (without ε-productions, except possibly S → ε) can be converted to CNF in polynomial time. The conversion preserves the generated language (minus ε). Steps:

1. **Eliminate ε-productions:** Find all nullable nonterminals and add new rules that skip them.
2. **Eliminate unit productions:** Remove chains like `A → B` by replacing them with `A → α` for every `B → α`.
3. **Remove useless symbols:** Delete nonterminals that either derive nothing or are unreachable from S.
4. **Convert remaining rules:** Terminals get their own nonterminal (`A → a` becomes `A → T_a, T_a → a`), and long right-hand sides are broken into binary chains using fresh nonterminals.

Each step runs in O(|G|²) time, so the total conversion is polynomial.

---

## Earley Parser

Jay Earley's 1970 parser handles **any CFG** — including ambiguous and ε-producing grammars — without CNF conversion. It blends top-down prediction (like recursive descent) with bottom-up recognition (like CYK), making it one of the most general parsing algorithms known.

### Items

An **Earley item** is a dotted rule paired with an origin index: `(A → α · β, i)` where `i` is the position in the input where this item's recognition began. The dot `·` marks how much of the right-hand side has been recognized so far.

- `(S → · A B, 0)` — prediction, nothing recognized yet.
- `(S → A · B, 0)` — A has been recognized starting at position 0.
- `(S → A B ·, 0)` — entire rule recognized; this is a completed item.

### Three Operations (at position k)

At each input position, the algorithm applies three operations until the chart set `Sₖ` stops growing:

1. **Predictor:** If item `(A → α · B γ, i)` is in `Sₖ` and B is a nonterminal, add `(B → · δ, k)` for every B-production. This is top-down prediction: "B might start here."

2. **Scanner:** If `(A → α · a β, i)` is in `Sₖ` and the next input symbol `s[k] = a`, add `(A → α a · β, i)` to `Sₖ₊₁`. This consumes a terminal.

3. **Completer:** If `(A → γ ·, j)` is a completed item in `Sₖ` and `(B → α · A β, i)` is in `Sⱼ`, add `(B → α A · β, i)` to `Sₖ`. This is bottom-up completion: "A has been found spanning [j, k], so advance the dot past A in any item that was waiting for it."

### Acceptance

The input is accepted if `(S' → S ·, 0)` appears in `Sₙ` (the chart set at the end of input), where S' is an augmented start symbol.

### Complexity

- **General:** O(n³) — each chart set can contain O(n) items, and each completer operation can scan the set.
- **Unambiguous grammars:** O(n) — each chart set is bounded by |G|, and each item is processed once.
- **Space:** O(n²) — n+1 chart sets, each with O(n) items.

### Earley vs CYK

| Property | CYK | Earley |
|----------|-----|--------|
| Grammar restriction | CNF required | Any CFG |
| Direction | Purely bottom-up | Mixed top-down + bottom-up |
| Best for | Dense grammars | Sparse/ambiguous grammars |
| Parse tree recovery | Back-pointers in table | Shared packed parse forest |

---

## Build It: From Scratch

Both algorithms are implemented in `code/main.py`. The CYK parser builds a parse table and traces each cell fill, showing exactly which rule applications produced which entries. The Earley parser prints the full chart set at each position so you can see predictor, scanner, and completer operations in action.

---

## Use It

Natural-language processing libraries like spaCy and NLTK use Earley-style parsers for grammar-based chunking and feature-structure unification. Compiler frameworks like Bison and ANTLR use LR/LALR algorithms (descendants of CYK's bottom-up philosophy) for programming language grammars. When a grammar is ambiguous, GLR parsing — a generalization of LR — uses CYK-like dynamic programming to explore all possible parses simultaneously.

---

## Ship It

A reusable parsing library that takes a grammar in BNF notation, automatically converts to CNF for CYK, and exposes both parsers via a unified API. Add parse-tree extraction that walks the table/chart to build a derivation tree.

---

## Exercises

### Level 1 — Recall

Convert the grammar `S → AB, A → a, B → b` to CNF (if needed) and run CYK on the string `"ab"`. Show the full parse table and mark which cell proves acceptance.

### Level 2 — Application

Given grammar:

```
S → S S | a
```

Trace the Earley chart for input `"aaa"`. List every item in each chart set S₀ through S₃. How many parse trees does the string have?

### Level 3 — Creation

Implement CNF conversion as a standalone function. Test it by converting an arbitrary 4-rule grammar, running CYK on the converted grammar, and verifying that the result matches a brute-force recursive-descent check on the original grammar.
