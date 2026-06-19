# Lesson 07 — PEG Parsers and Packrat

## What Is PEG?

A **Parsing Expression Grammar** (PEG) describes a language using **ordered** parsing rules rather than the unordered production rules of a context-free grammar. PEG is deterministic — given an input, there is exactly one parse or a failure, never an ambiguity.

Bryan Ford introduced PEG in 2004 as a practical alternative to CFG-based parsing. The key insight: for most programming languages, you don't *want* ambiguity. You want a single, predictable parse. PEG gives you that by design.

## PEG Operators

PEG uses a small set of combinators, each with precise semantics:

| Operator | Syntax | Meaning |
|----------|--------|---------|
| **Sequence** | `a b` | Match `a` then `b`. Return both results. |
| **Ordered choice** | `a / b` | Try `a` first. If it succeeds, done. Otherwise try `b`. |
| **Optional** | `a?` | Match zero or one occurrences of `a`. |
| **Zero-or-more** | `a*` | Match `a` as many times as possible. |
| **One-or-more** | `a+` | Match `a` at least once, then as many times as possible. |
| **Not-predicate** | `!a` | Succeed if `a` fails, without consuming input. |
| **And-predicate** | `&a` | Succeed if `a` succeeds, without consuming input. |
| **Any** | `.` | Match any single character. |
| **Character class** | `[a-z]` | Match one character in the range. |
| **Literal** | `"text"` | Match the exact string. |

## Ordered Choice vs. CFG Alternation

This is the critical difference. In a CFG, `A → 'a' | 'a'` is fine — the parser picks one. In PEG, `A ← 'a' / 'a'` *always* picks the first alternative. The second branch is dead code.

This eliminates ambiguity but changes what grammars you can express:

```
# CFG — ambiguous (both derivations valid for "a + b + c")
E → E '+' T | T

# PEG — ordered: first alternative tried, no ambiguity
E ← E '+' T / T
```

PEG also supports **left recursion** in some implementations (via the Warth et al. technique), but naive PEG parsers cannot handle it.

## Lookahead: & and !

The **and-predicate** `&a` peeks ahead: does `a` match here? It does *not* consume input.

The **not-predicate** `!a` peeks ahead: does `a` fail here? It also does not consume input.

These are used for context-sensitive matching without leaving the PEG framework:

```
# Match a keyword only if not followed by an identifier character
keyword ← "if" ![a-zA-Z0-9_]
```

## PEG vs. CFG

| Property | CFG | PEG |
|----------|-----|-----|
| Ambiguity | Allowed (must be resolved) | Impossible by construction |
| Left recursion | Supported natively | Requires special technique |
| Ordering | Unordered alternation | Ordered choice (first match wins) |
| Expressiveness | OI languages, nested structures | Most practical languages |
| Formal power | Chomsky Type-2 | Strictly different, incomparable |

PEG cannot express every CFG. For example, the language `{a^n b^n c^m} ∪ {a^n b^m c^m}` (intersection of two non-regular, non-CFL languages) is beyond both CFG and PEG. But virtually every real programming language grammar is in both.

## PEG vs. Regular Expressions

PEG is strictly more powerful than regex:

- **PEG is recursive**: rules can reference themselves, enabling nested structures like matching parentheses.
- **Regex is not recursive** (standard POSIX/PCRE): you cannot match `((...))` with arbitrary depth using pure regex.
- **PEG tracks position** in a structured way; regex operates on a flat character stream.

Think of PEG as the natural "upgrade path" from regex: when your matching problem requires nested structure or context, reach for PEG.

## Packrat Parsing

A naive recursive PEG parser can take **exponential time** on certain inputs because it re-parses the same position with the same rule repeatedly. **Packrat parsing** solves this with **memoization**:

- For every `(rule, position)` pair, cache the result (success with consumed length, or failure).
- Before trying to parse rule `R` at position `i`, check the cache.
- **Guarantee**: O(n) time for any input of length n.
- **Cost**: O(n) space per rule — O(n × |rules|) total.

This makes PEG parsing as fast as any LR parser in practice, with the simplicity of recursive descent.

```
Packrat:  Time = O(n), Space = O(n × rules)
LL/LR:   Time = O(n), Space = O(n)
```

The space overhead is manageable for most grammars and is a fair trade for guaranteed linear-time parsing.

## Build It: PEG Parser Combinator Library

We'll build a PEG combinator library in Rust. The core abstraction is a `Parser` trait that takes an input string and a position, and returns either a success (with the new position and parsed value) or a failure.

### Step 1: Core Parser Trait and Combinators

The `Parser` trait is the foundation. Every combinator (`literal`, `seq`, `choice`, `optional`, `many`, `not`, etc.) produces a new `Parser` from existing ones.

### Step 2: Packrat Memoization

We add a `PackratCache` that stores results keyed by `(rule_id, position)`. Before executing a parser, we check the cache. After, we store the result.

### Step 3: Arithmetic Expression Grammar

We compose combinators to parse arithmetic expressions — demonstrating that PEG handles operator precedence through recursive rule structure, just like recursive descent.

## Use It

Production PEG tools:

- **`pest`** (Rust): A PEG parser generator that compiles `.pest` grammar files into Rust code. Used by `cargo-geiger`, `rslint`, and others. Supports procedural grammar, error reporting, and pairs of tokens.
- **PEG.js** (JavaScript): Browser and Node.js PEG parser generator. Write a grammar, get a JavaScript parser.
- **`parsec`** (Python): Monadic parser combinator library inspired by Haskell's Parsec. PEG-style combinators.
- **`nom`** (Rust): While technically a combinator library rather than a PEG generator, it follows the same principles — composition of parsers with ordered choice and lookahead.

Our hand-built version is closest to `pest` in philosophy but much simpler. A production tool adds: grammar file compilation, optimized code generation, error recovery, and span tracking.

## Read the Source

- `pest` grammar format: `pest/pest/src/parser.rs` — shows how PEG rules are compiled to Rust parsing code.
- Bryan Ford's original paper: "Parsing Expression Grammars: A Recognition-Based Syntactic Foundation" (2004).

## Ship It

The reusable artifact is a PEG parser combinator library. Given any set of composable parsers, you can build a complete parser for a language by combining `literal`, `seq`, `choice`, `many`, `not`, and `map`. The packrat cache makes it O(n).

## Exercises

**Level 1 — Warm-Up:**
Using the combinator library, write a PEG grammar that matches a simple identifier: a letter followed by zero or more letters, digits, or underscores. Test it on `"x"`, `"foo_bar"`, `"3abc"` (should fail).

**Level 2 — Intermediate:**
Add a `sep_by(parser, separator)` combinator that matches a list of `parser` results separated by `separator`. Use it to parse comma-separated numbers: `"1, 2, 3"` → `[1, 2, 3]`.

**Level 3 — Challenge:**
Implement the Warth et al. algorithm for **left-recursive PEG rules**. Extend the packrat cache to handle left recursion by iteratively growing the result until a fixed point. Test with a left-recursive expression grammar: `E ← E '+' T / T`.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| PEG | "Parsing Expression Grammar" | A formalism for defining parsers using ordered choice, sequence, and repetition — deterministic by construction |
| Ordered choice | "Try first, then fall back" | The `/` operator: try left alternative; if it fails, try right. First match wins, no backtracking across alternatives |
| Packrat | "Memoized PEG parser" | A PEG parser that caches the result of every (rule, position) pair to guarantee O(n) time |
| Not-predicate | "Negative lookahead" | `!a` — succeeds (without consuming input) only if parser `a` fails at the current position |
| And-predicate | "Positive lookahead" | `&a` — succeeds (without consuming input) only if parser `a` succeeds at the current position |

## Further Reading

- Bryan Ford, "Parsing Expression Grammars: A Recognition-Based Syntactic Foundation" (POPL 2004)
- Warth, Douglass, Millstein, "Packrat Parsers Can Support Left Recursion" (PEPM 2008)
- `pest` documentation: https://pest.rs
- Medeiros et al., "From PEG to Packrat Parsing" (SBLP 2010)
