# Phase Capstone — A Proof Companion CLI

> Tie Phase 01 together. A single command-line tool that takes a discrete-math claim and either *verifies* it on a finite sample, *finds a counterexample*, or *demonstrates the technique* that proves it.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 01, Lessons 01–21
**Time:** ~90 minutes

## Learning Objectives

- Integrate the libraries built in Phase 01 (`truth_table`, `predicate_check`, `relation_ops`, `dsu`, `combo`, `catalan`, `primes`, `markov`, `coding`) into a single tool.
- Build a small CLI with subcommands, argument parsing, and pretty-printed output.
- Practice the discipline of "verify a discrete-math claim end-to-end": parse → simulate → bound → report.
- Reflect: which proof technique is right for which kind of claim?

## The Problem

By the end of Phase 01 you have:

- A truth-table evaluator (L01).
- A predicate-logic checker (L02).
- A proof-by-cases / induction checker (L03).
- Relation, function, equivalence-class machinery (L04, L05).
- Poset + topo sort (L06).
- Cardinality / diagonal tricks (L07).
- A combinatorics library (L08–10).
- Recurrence + asymptotic tools (L11–12).
- Number-theory primitives — gcd, modinv, modpow, primes (L13–15).
- Boolean minimization (L16).
- Graph algorithms (L17–18).
- Probability + Markov chains (L19–20).
- Coding / entropy (L21).

This capstone wires them together into one CLI: `cs01`. Subcommands dispatch to the right verifier; the tool itself is small (~ 300 lines) because the work is done by the libraries.

## The Concept

### Architecture

```
   ┌────────────┐
   │  CLI       │ argparse, subcommands
   └─────┬──────┘
         │
         ▼
   ┌────────────────────────────────────┐
   │  Dispatchers                       │
   │  - logic   (truth table, predicate)│
   │  - count   (combo, Catalan, IE)    │
   │  - number  (gcd, primes, mod)      │
   │  - graph   (BFS, color, match)     │
   │  - prob    (entropy, expectation)  │
   └─────┬──────────────────────────────┘
         │
         ▼
   ┌────────────────────────────┐
   │  Phase 01 libraries        │
   │  (outputs/*.py from L01-21)│
   └────────────────────────────┘
```

Each subcommand parses its own arguments and calls into the relevant library.

### Subcommands

| Subcommand | What it does |
|-----------|--------------|
| `truth <expr>` | Print the truth table of a propositional formula |
| `gcd a b` | Compute GCD and Bezout coefficients |
| `prime n` | Is n prime? (Miller-Rabin) |
| `mod-pow a e n` | a^e mod n via repeated squaring |
| `count combo n k` / `count catalan n` / `count factorial n` | Counts |
| `topo <node:succ1 succ2,...>` | Topological sort a DAG |
| `entropy p1 p2 ... pn` | Shannon entropy of a distribution |
| `huffman p1 p2 ... pn` | Optimal Huffman codes for given freqs |
| `pagerank <node:succ1 succ2,...>` | PageRank of a small directed graph |
| `verify <name>` | Pre-canned demos (`fermat`, `birthday`, `coupon`, `all`) |

### Design choices

- **No external deps**: stays in stdlib so it runs anywhere Python does.
- **Output is human-readable**: ASCII tables, no color.
- **Failure is a counterexample**: when a claim doesn't hold, the CLI prints the smallest witness it can find.

## Build It

The lesson's `code/main.py` is the full CLI (~ 300 lines). Run with:

```sh
python3 main.py truth "P -> Q"
python3 main.py gcd 462 1071
python3 main.py prime 2305843009213693951      # M61 = 2^61 - 1
python3 main.py mod-pow 7 200 13
python3 main.py count combo 52 5
python3 main.py count catalan 10
python3 main.py count factorial 20
python3 main.py entropy 0.5 0.3 0.15 0.05
python3 main.py huffman 0.5 0.3 0.15 0.05
python3 main.py topo "A:B C, B:D, C:D, D:"
python3 main.py pagerank "A:B C, B:C, C:A, D:C"
python3 main.py verify fermat
python3 main.py verify birthday
python3 main.py verify all
```

### Step 1: Subcommand dispatch

A `dict[str, callable]` maps each subcommand name to its handler.

### Step 2: Inline self-contained implementations

To keep the capstone runnable as a single file, we inline thin versions of the L01–L21 algorithms rather than importing them — proving you've internalized each.

### Step 3: `verify all`

Runs every demo (`fermat`, `birthday`, `coupon`, `huffman_bound`, `master`, `hamming`); asserts each gives the expected output. This is the integration test.

## Use It

This CLI is **the artifact of Phase 01**. Keep it on your PATH. Useful for:

- Reality-checking homework or competitive-programming counts ("is this 5×4×3×2 the right formula?" → `count combo 5 2`).
- Verifying a tautology before committing a proof.
- Producing Bezout coefficients during RSA practice (`gcd e phi`).
- Sanity-checking entropy / Huffman bounds during compression experiments.

## Read the Source

- The lesson's own `code/main.py` — that *is* the source. Read it top to bottom.
- The libraries you wrote in L01–L21 — each is < 200 lines.
- Compare against [SymPy](https://github.com/sympy/sympy)'s discrete-math modules to see a production-grade equivalent.

## Ship It

This lesson ships **`outputs/cs01`** — a wrapper script that puts the tool on PATH. Drop it into `/usr/local/bin/` (or anywhere in PATH) and `cs01 prime 17` works system-wide.

## Exercises

1. **Easy.** Add a `subset-count` subcommand that, given a set size n and a constraint (e.g., "size ≥ 3"), counts subsets satisfying the constraint.
2. **Medium.** Add a `bipartite-match` subcommand that reads a bipartite graph from CSV and outputs the maximum matching.
3. **Hard.** Add a `hamming-decode` subcommand that takes a 7-bit codeword and returns the corrected 4 data bits + error position. Bonus: support `(15, 11)` and `(31, 26)` codes too.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| CLI | "Command-line tool" | A program with argument parsing that dispatches to subcommands |
| Capstone | "Final project" | An artifact that *integrates* a unit's lessons into one runnable thing |
| Verifier vs prover | "Tests vs proofs" | A verifier checks instances; a prover establishes the universal claim |

## Further Reading

- *The Art of Unix Programming* by Eric Raymond — taxonomy of CLI design patterns.
- Python `argparse` documentation — for writing serious subcommands.
- The course's own future capstones (Phase 02-19) use this same "integrate-the-phase" pattern.
