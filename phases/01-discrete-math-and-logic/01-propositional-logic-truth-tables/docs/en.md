# Propositional Logic & Truth Tables

> Boolean algebra is the bedrock of every if-statement, every circuit gate, every SQL `WHERE`, every type checker. Learn the algebra; the consequences fall out.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 00
**Time:** ~45 minutes

## Learning Objectives

- Define a *proposition* and explain what it means for two propositions to be *logically equivalent*.
- Build a truth table for any propositional formula by hand and verify it with code.
- Apply De Morgan's laws, distributivity, and the identity/idempotent/absorption laws to simplify expressions.
- Convert a formula to Conjunctive Normal Form (CNF) and Disjunctive Normal Form (DNF) and explain why those forms matter (SAT solvers, query optimization).

## The Problem

Every conditional you've ever written — `if (x > 0 && (y == 0 || !z))` — is propositional logic. So is every SQL `WHERE`, every package's feature gate, every circuit you'll build in Phase 06. The problem with stopping at "I know `&&` and `||`" is that you can't:

- Prove two `if`-conditions are equivalent (the optimizer does this constantly).
- Recognize when a complex chain of `&&` and `||` can be flattened into a flat AND-of-OR (the SAT solvers in Phase 17 *only* accept that form).
- Simplify a Karnaugh map down to a minimal gate-count circuit (Phase 06).
- Read the type-checker's diagnostic that says "this branch is unreachable because `x < 0 && x > 5` is unsatisfiable."

Propositional logic gives you the algebra to do all of those mechanically. This lesson is the algebra plus a small evaluator you write in Python.

## The Concept

### Propositions

A **proposition** is a statement that is either true or false. We use letters (P, Q, R) for the atomic propositions ("the user is logged in," "the cache hit"). Compound propositions are built with five operators:

| Operator | Symbol | Reads as | Python | Truth |
|----------|--------|----------|--------|-------|
| Negation | ¬P    | "not P"  | `not p`  | T → F, F → T |
| Conjunction | P ∧ Q | "P and Q" | `p and q` | T iff both T |
| Disjunction | P ∨ Q | "P or Q"  | `p or q`  | T iff either T |
| Implication | P → Q | "if P then Q" | `(not p) or q` | F only when P=T, Q=F |
| Biconditional | P ↔ Q | "P iff Q" | `p == q` | T iff P and Q match |

The one that surprises people is implication: **P → Q is true whenever P is false.** "If pigs fly, then 2 + 2 = 5" is true, because the premise is false. This is *material implication*, not the natural-language conditional you're used to.

### Truth tables

A truth table enumerates every combination of inputs and shows the output.

```
P  Q  | P→Q
─────────────
T  T  |  T
T  F  |  F
F  T  |  T
F  F  |  T
```

For n atomic propositions, the table has 2^n rows. With n=20 it's a million rows — handle-able by a computer, miserable by hand. (That blow-up is why SAT is hard: deciding whether a formula has *any* satisfying assignment is NP-complete; you may have to try all 2^n.)

### Logical equivalence

Two formulas are **logically equivalent** (≡) iff they have the same value in every row of the truth table. Equivalence laws let you rewrite without checking the table each time:

| Law | Form |
|-----|------|
| Double negation | ¬¬P ≡ P |
| Commutativity | P ∧ Q ≡ Q ∧ P; P ∨ Q ≡ Q ∨ P |
| Associativity | (P ∧ Q) ∧ R ≡ P ∧ (Q ∧ R); similarly ∨ |
| Distributivity | P ∧ (Q ∨ R) ≡ (P ∧ Q) ∨ (P ∧ R) and dual |
| Identity | P ∧ T ≡ P; P ∨ F ≡ P |
| Domination | P ∨ T ≡ T; P ∧ F ≡ F |
| Idempotent | P ∧ P ≡ P; P ∨ P ≡ P |
| **De Morgan** | ¬(P ∧ Q) ≡ ¬P ∨ ¬Q; ¬(P ∨ Q) ≡ ¬P ∧ ¬Q |
| Implication | P → Q ≡ ¬P ∨ Q |
| Contrapositive | P → Q ≡ ¬Q → ¬P |
| Biconditional | P ↔ Q ≡ (P → Q) ∧ (Q → P) |

De Morgan is the one you use daily. It's why "not (cache_hit and within_budget)" reads as "cache miss or out of budget."

### Tautology, contradiction, contingency

- **Tautology:** always true (e.g., `P ∨ ¬P`).
- **Contradiction:** always false (e.g., `P ∧ ¬P`).
- **Contingency:** sometimes true, sometimes false.

A SAT solver answers "is this satisfiable?" — is there at least one assignment making it true? Tautologies are trivially yes; contradictions are no; contingencies are yes (with witness).

### Normal forms: CNF and DNF

A literal is an atomic proposition or its negation. A *clause* is an OR of literals; a *term* is an AND of literals.

- **Conjunctive Normal Form (CNF):** an AND of clauses. E.g., `(P ∨ Q) ∧ (¬P ∨ R) ∧ (Q ∨ ¬R)`.
- **Disjunctive Normal Form (DNF):** an OR of terms. E.g., `(P ∧ ¬Q) ∨ (¬P ∧ R)`.

Every formula has an equivalent CNF and DNF (though either may be exponentially larger). SAT solvers consume CNF exclusively (Phase 17). Query optimizers in SQL push predicates around using CNF/DNF identities. Circuit synthesis tools accept DNF (sum-of-products).

The conversion recipe to CNF:
1. Eliminate `→` and `↔` using `P → Q ≡ ¬P ∨ Q`.
2. Push `¬` inward via De Morgan until it sits next to atoms only ("Negation Normal Form").
3. Distribute `∧` over `∨` (or the other way for DNF).

## Build It

We'll build a tiny propositional logic evaluator and truth-table generator in Python. Open `code/main.py`.

### Step 1: Represent formulas as ASTs

A formula is an AST: variables are leaves; operators are internal nodes.

```python
@dataclass(frozen=True)
class Var:  name: str
@dataclass(frozen=True)
class Not:  child: "Formula"
@dataclass(frozen=True)
class And:  left: "Formula"; right: "Formula"
@dataclass(frozen=True)
class Or:   left: "Formula"; right: "Formula"
@dataclass(frozen=True)
class Imp:  left: "Formula"; right: "Formula"
@dataclass(frozen=True)
class Iff:  left: "Formula"; right: "Formula"
```

### Step 2: Evaluate a formula given an assignment

```python
def evaluate(f, env):
    match f:
        case Var(name):  return env[name]
        case Not(c):     return not evaluate(c, env)
        case And(l, r):  return evaluate(l, env) and evaluate(r, env)
        case Or(l, r):   return evaluate(l, env) or evaluate(r, env)
        case Imp(l, r):  return (not evaluate(l, env)) or evaluate(r, env)
        case Iff(l, r):  return evaluate(l, env) == evaluate(r, env)
```

### Step 3: Enumerate variables and build the truth table

```python
def variables(f):
    match f:
        case Var(name): return {name}
        case Not(c):    return variables(c)
        case _:         return variables(f.left) | variables(f.right)

def truth_table(f):
    vars_sorted = sorted(variables(f))
    rows = []
    for bits in itertools.product([False, True], repeat=len(vars_sorted)):
        env = dict(zip(vars_sorted, bits))
        rows.append((env, evaluate(f, env)))
    return vars_sorted, rows
```

### Step 4: Test the equivalence of two formulas

```python
def equivalent(f, g):
    vars_all = sorted(variables(f) | variables(g))
    for bits in itertools.product([False, True], repeat=len(vars_all)):
        env = dict(zip(vars_all, bits))
        if evaluate(f, env) != evaluate(g, env):
            return False
    return True
```

This is the *truth-table method* for proving equivalence: O(2^n) but always correct.

### Step 5: Verify a non-trivial identity

```python
P, Q = Var("P"), Var("Q")
lhs = Not(And(P, Q))                  # ¬(P ∧ Q)
rhs = Or(Not(P), Not(Q))              # ¬P ∨ ¬Q
assert equivalent(lhs, rhs)           # De Morgan!
```

Run `python3 code/main.py` to see the demo prints.

## Use It

The same algebra underpins production tools:

- **SAT solvers** (MiniSat, CaDiCaL, Z3): consume CNF, search 2^n assignments using sophisticated pruning. Used in Phase 17 (formal methods), and in compilers for instruction scheduling.
- **SQL query optimizers**: rewrite `WHERE` predicates into CNF, push individual clauses down to base tables to filter early.
- **Logic-synthesis tools** (Yosys, ABC) for hardware: minimize DNF/CNF to gate count.
- **Type checkers** like Rust's exhaustiveness analyzer: prove that a `match` covers all cases by checking that the disjunction of patterns is a tautology over the type's value space.

## Read the Source

- [PySAT](https://pysathq.github.io/) — a Python wrapper around top SAT solvers; a 50-line example fits a real-world CNF problem.
- *How to Prove It* (Velleman), Chapters 1–2 — the cleanest informal treatment of propositional logic for beginners.
- [SMT-LIB standard](https://smt-lib.org/) — the format every SMT solver accepts; built on CNF underneath.

## Ship It

This lesson ships **`outputs/truth_table.py`** — a stand-alone CLI that prints a truth table for a formula passed as an expression. Useful for circuit verification homework, for proving simplifications, and as a building block in later lessons.

## Exercises

1. **Easy.** Build the truth table for `(P → Q) ∧ (Q → R) → (P → R)`. Is it a tautology? Verify with the lesson's `equivalent` against the formula `True`.
2. **Medium.** Implement `to_cnf(f)` that converts an AST into Conjunctive Normal Form using the three-step recipe (eliminate `→`/`↔`, push negation, distribute). Verify on three examples.
3. **Hard.** Wrap the lesson's evaluator with a brute-force SAT solver: given a formula, return either an assignment satisfying it or "UNSAT." Time it on increasingly large formulas and chart the exponential blow-up (n = 5, 10, 15, 20 vars).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Proposition | "A statement" | A claim that's either true or false, with no shades of grey |
| Tautology | "Always true" | A formula true under every assignment of its variables |
| Equivalent | "The same" | Two formulas with identical truth tables |
| CNF | "Normal form" | An AND of ORs of literals; the form SAT solvers accept |
| Implication | "If-then" | The Boolean function `(¬P ∨ Q)`; subtle because it's true when P is false |

## Further Reading

- *Discrete Mathematics and Its Applications* by Rosen — Chapter 1 is the canonical textbook treatment.
- [The DIMACS CNF format](http://www.satcompetition.org/2009/format-benchmarks2009.html) — the input format every SAT solver consumes.
- [Donald Knuth — *The Art of Computer Programming*, Vol. 4A](https://www-cs-faculty.stanford.edu/~knuth/taocp.html), Section 7.1.1 (Boolean basics) — exhaustive and beautiful.
