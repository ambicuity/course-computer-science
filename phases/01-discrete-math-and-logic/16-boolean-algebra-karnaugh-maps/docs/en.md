# Boolean Algebra & Karnaugh Maps

> Every digital circuit is a Boolean function. Minimizing the function is minimizing the gates — and that's the bridge from algebra to silicon.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 01, Lesson 01
**Time:** ~60 minutes

## Learning Objectives

- Manipulate Boolean expressions using the axioms (identity, complement, distributivity, De Morgan, absorption).
- Read a truth table into Sum-of-Products (SOP) and Product-of-Sums (POS) canonical forms.
- Use a 2-, 3-, or 4-variable Karnaugh map to find a minimal SOP expression by inspection.
- Apply the Quine-McCluskey algorithm computationally for larger inputs; recognize NP-hardness of optimal Boolean minimization.

## The Problem

You'll meet Boolean minimization in three places later in the course:

1. **Digital logic** (Phase 06): "Build a circuit that outputs 1 iff this 4-bit input is prime." The fewer gates, the faster and smaller the chip.
2. **Compiler optimization** (Phase 08): replacing `if (x && y) || (x && z)` with `if (x && (y || z))` is Boolean factoring — a function of the underlying algebra.
3. **Database query optimization** (Phase 10): rewriting `WHERE (a=1 AND b=2) OR (a=1 AND b=3)` to `WHERE a=1 AND b IN (2, 3)` is the same identity.

This lesson is the algebra and the visualization tools. Phase 06 then uses them to build an ALU.

## The Concept

### Axioms of Boolean algebra

Two values: 0 and 1. Three operations: AND (·), OR (+), NOT (¬, sometimes written as overbar). For all a, b, c:

| Name | Identity |
|------|----------|
| Identity        | a + 0 = a,    a · 1 = a |
| Null            | a + 1 = 1,    a · 0 = 0 |
| Idempotent      | a + a = a,    a · a = a |
| Complement      | a + ¬a = 1,   a · ¬a = 0 |
| Double negation | ¬¬a = a |
| Commutative     | a + b = b + a,    a · b = b · a |
| Associative     | (a + b) + c = a + (b + c),  (a · b) · c = a · (b · c) |
| Distributive    | a · (b + c) = a·b + a·c,  a + (b · c) = (a + b)(a + c) |
| **De Morgan**   | ¬(a + b) = ¬a · ¬b,    ¬(a · b) = ¬a + ¬b |
| Absorption      | a + a·b = a,    a · (a + b) = a |
| Consensus       | a·b + ¬a·c + b·c = a·b + ¬a·c |

The **distributive law over +** (last column of distributive row) is the surprising one — it doesn't exist for integer arithmetic.

### Canonical forms: SOP and POS

Every Boolean function on n variables can be written *uniquely* (up to ordering) as:

- **Disjunctive Normal Form / Sum-of-Products (SOP)** — OR of ANDs of literals. Each AND term = one row of the truth table where the function is 1.
- **Conjunctive Normal Form / Product-of-Sums (POS)** — AND of ORs of literals. Each OR clause = one row where the function is 0 (negated).

Example, f(a, b, c) = "majority":

```
a b c | f
0 0 0 | 0
0 0 1 | 0
0 1 0 | 0
0 1 1 | 1     → ¬a·b·c
1 0 0 | 0
1 0 1 | 1     → a·¬b·c
1 1 0 | 1     → a·b·¬c
1 1 1 | 1     → a·b·c
```

SOP: `f = ¬abc + a¬bc + ab¬c + abc`. POS comes from the four 0-rows.

These canonical forms are huge but easy to derive; minimization shrinks them.

### Karnaugh maps

A K-map is a 2D rearrangement of the truth table where adjacent cells differ in exactly one variable. For 4 variables:

```
        cd
        00   01   11   10
   ab ┌────┬────┬────┬────┐
   00 │ m0 │ m1 │ m3 │ m2 │
      ├────┼────┼────┼────┤
   01 │ m4 │ m5 │ m7 │ m6 │
      ├────┼────┼────┼────┤
   11 │m12 │m13 │m15 │m14 │
      ├────┼────┼────┼────┤
   10 │ m8 │ m9 │m11 │m10 │
      └────┴────┴────┴────┘
```

Note: the row/column labels are in **Gray code order** (00, 01, 11, 10) so adjacent cells differ in one bit. The grid also wraps — the left and right edges are adjacent, as are top and bottom.

**To minimize:**
1. Mark every cell where f = 1.
2. Cover all 1-cells with the smallest number of *rectangles* whose side lengths are powers of 2 (1×1, 1×2, 1×4, 2×2, 2×4, 4×4).
3. Each rectangle simplifies to a single product term (literals that DON'T vary across the rectangle).
4. OR the terms together.

Larger rectangles → fewer literals. A 2×4 = 8-cell rectangle drops 3 literals (only one survives).

### Quine-McCluskey (algorithmic)

K-maps are visual; past 4–5 variables they're unwieldy. **Quine-McCluskey** is the systematic equivalent:

1. List all minterms (1-rows) in binary.
2. Group by number of 1s.
3. Pairwise compare adjacent groups; if two minterms differ by one bit, combine into an *implicant* (with a `-` in the differing position).
4. Repeat until no more combinations are possible.
5. Use the implicant chart to pick a minimal cover (essential prime implicants + greedy / branch-and-bound for the rest).

Quine-McCluskey is correct but exponential-time in general — Boolean minimization is NP-hard. Practical tools (Espresso, ABC) use heuristics that work well in practice but don't guarantee optimality.

### Why minimization matters

| Setting | Why fewer terms |
|---------|-----------------|
| Hardware  | Fewer gates → smaller chip, less power, faster propagation (Phase 06) |
| Software (compiler) | Fewer branches, fewer mispredicts (Phase 08, 15) |
| Database  | Smaller index probes, better cache hits (Phase 10) |
| SAT solving | Smaller CNF, faster solve (Phase 17) |

## Build It

Open `code/main.py`. We'll implement SOP extraction, K-map drawing, and a small Quine-McCluskey.

### Step 1: Truth-table → SOP

For every input row where f = 1, write the AND term `(¬)v_i` based on whether v_i is 0 or 1 in that row. OR all such terms.

### Step 2: Print a K-map for 3-variable functions

ASCII rendering of the Gray-coded grid.

### Step 3: Quine-McCluskey

Iteratively merge minterms that differ in one bit. The result is the set of *prime implicants* — minimal product terms covering at least one minterm.

Apply to majority(a, b, c) — should reduce SOP from 4 terms (12 literals) to 3 terms (6 literals): `ab + ac + bc`.

### Step 4: A real-world simplification

`(a AND b) OR (a AND c) OR (a AND d)`. QM should give `a · (b + c + d)` after factoring — distributivity in action.

## Use It

- **Logic synthesis tools** (Yosys, ABC, Espresso) take RTL/Verilog and produce a netlist of gates — internally running advanced versions of QM.
- **Compiler short-circuiting**: `if (a && b) || (a && c)` is recognized as `a && (b || c)` and emitted that way; saves both gates and branch mispredictions.
- **Hardware K-maps**: instructors use them through ~4 variables; production work always uses CAD tools beyond that.
- **Don't-care optimization**: in circuit design, many input combinations "can't happen" and are marked `X` in the K-map; the synthesizer uses them as free choices to enlarge rectangles.

## Read the Source

- *Digital Design* by Mano & Ciletti — the standard intro; Chapter 3 is K-maps + QM.
- [Quine's 1955 paper on QM](https://www.jstor.org/stable/2308119) — surprisingly readable.
- [Espresso heuristic algorithm](https://en.wikipedia.org/wiki/Espresso_heuristic_logic_minimizer) — the practical workhorse for industrial minimization.

## Ship It

This lesson ships **`outputs/qm.py`** — Quine-McCluskey implementation (handles up to ~6 variables comfortably), plus a `kmap_ascii(table, vars)` printer.

## Exercises

1. **Easy.** Use a 3-variable K-map to minimize `f(a, b, c) = abc + a¬bc + a¬b¬c + ¬abc'`. By hand; verify with the lesson library.
2. **Medium.** Show using Boolean algebra: `ab + a¬bc + abc = a·(b + c)`. List each axiom used.
3. **Hard.** Build a truth table for "exactly 2 of 4 inputs are 1." Apply QM; the minimal SOP should have C(4, 2) = 6 product terms of 4 literals each — confirm the result and explain why this function has no factorable structure.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Minterm | "A row of the truth table where f=1" | A specific AND of literals that's true at exactly one input assignment |
| SOP / DNF | "Sum of products" | OR of AND clauses — the canonical "list every 1-row" form |
| POS / CNF | "Product of sums" | AND of OR clauses — the dual; SAT solvers love it |
| Implicant | "A rectangle on the K-map" | A product term that implies f (its set of satisfying assignments ⊆ f's 1-set) |
| Prime implicant | "Maximal rectangle" | An implicant that can't be merged with another to make a bigger one |

## Further Reading

- *Logic and Computer Design Fundamentals* by Mano, Kime, Martin — exhaustive textbook.
- *Synthesis and Optimization of Digital Circuits* by De Micheli — research-grade; chapters on Espresso and SIS.
- [The Berkeley ABC tool](https://github.com/berkeley-abc/abc) — open-source logic synthesizer used in research and the OpenROAD project.
