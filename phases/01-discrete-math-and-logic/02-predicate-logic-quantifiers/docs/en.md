# Predicate Logic & Quantifiers

> Propositional logic talks about whole statements. Predicate logic looks inside them. That's where math actually lives.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 01, Lesson 01
**Time:** ~60 minutes

## Learning Objectives

- Distinguish *predicates* from *propositions*, and use the universal (∀) and existential (∃) quantifiers correctly.
- Identify free vs bound variables, and rename bound variables without changing meaning (α-conversion).
- Translate English statements with "every," "some," "no," "exists exactly one" into predicate logic.
- Recognize when ∀x ∃y P(x, y) and ∃y ∀x P(x, y) say different things, and why that matters for algorithms and security claims.

## The Problem

Propositional logic can say "Alice is logged in" (a single atomic proposition). It cannot say "every logged-in user has a session token" — that statement quantifies over users. Most real claims in math, security, and verification have quantifiers:

- "Every prime greater than 2 is odd."
- "For every input, the sorting algorithm terminates with a sorted output."
- "There exists a key that unlocks the door."
- "No two users share the same session token."

Each one is *trivially false* in some formalisms if you mis-parse the quantifier — and the bugs that follow can be subtle (an "exists" mistakenly placed inside a "forall" turns a security property into a much weaker one).

Predicate logic is the algebra of these statements. This lesson lays out the syntax, the equivalences, and a Python evaluator over finite domains so you can mechanically check claims like "every even number is the sum of two odds."

## The Concept

### Predicates

A **predicate** is a function from objects to truth values. Write `P(x)` for "x has property P," or `R(x, y)` for "x is related to y by R." A predicate by itself isn't true or false — it depends on what you plug in. `Prime(6)` is false; `Prime(7)` is true.

### Quantifiers

Two quantifiers turn a predicate into a proposition:

| Symbol | Reads as | Definition |
|--------|----------|------------|
| ∀x. P(x) | "for all x, P(x)" | True iff P(x) is true for *every* x in the domain |
| ∃x. P(x) | "there exists x s.t. P(x)" | True iff P(x) is true for *at least one* x in the domain |

`∃!x. P(x)` is a derived shorthand for "exactly one x": `∃x. P(x) ∧ ∀y. (P(y) → y = x)`.

### Free vs bound variables

In `∀x. P(x, y)`:
- `x` is **bound** by the quantifier — its name doesn't matter outside.
- `y` is **free** — it must be supplied by context (or another quantifier).

Renaming bound vars never changes meaning: `∀x. P(x, y)` ≡ `∀z. P(z, y)`. This is α-conversion. Programmers know this as scoping: the `i` in `for i in range(n)` shadows nothing outside.

### Translating English

| English | Predicate logic |
|---------|-----------------|
| Every dog is a mammal | ∀x. Dog(x) → Mammal(x) |
| Some dog is brown | ∃x. Dog(x) ∧ Brown(x) |
| No dog flies | ¬∃x. Dog(x) ∧ Flies(x), equivalently ∀x. Dog(x) → ¬Flies(x) |
| All who have a token have access | ∀u. (∃t. HasToken(u, t)) → Access(u) |

Two common translation traps:
1. "Every dog is a mammal" uses ∀ with implication (not conjunction): `∀x. Dog(x) ∧ Mammal(x)` would mean "everything is both a dog and a mammal."
2. "Some dog is brown" uses ∃ with conjunction (not implication): `∃x. Dog(x) → Brown(x)` is trivially true if there's anything in the domain that's not a dog.

### Order of quantifiers matters — a lot

`∀x ∃y. P(x, y)` is **NOT** the same as `∃y ∀x. P(x, y)`:

| Form | Reading |
|------|---------|
| ∀x ∃y. Loves(x, y) | Everyone has someone they love (y can depend on x) |
| ∃y ∀x. Loves(x, y) | There's someone (Beyoncé) whom everyone loves (one fixed y) |

In CS, this is *the* crucial distinction in security and cryptographic protocols:

> "For every adversary, there exists a key it can't break" (weaker — different key per adversary)
>
> vs.
>
> "There exists a key such that every adversary can't break it" (the cryptographic claim).

### De Morgan for quantifiers

The negation rules mirror propositional De Morgan:

```
¬∀x. P(x)  ≡  ∃x. ¬P(x)
¬∃x. P(x)  ≡  ∀x. ¬P(x)
```

"Not everyone loves chocolate" = "Someone doesn't love chocolate." Use these to push negation inward to get formulas into Prenex Normal Form (all quantifiers in front).

### Prenex normal form

Every closed formula can be rewritten so all quantifiers come first, followed by a quantifier-free body. E.g.:

`∀x. (P(x) → ∃y. Q(x, y))` becomes `∀x ∃y. (P(x) → Q(x, y))`.

Why care? Many proof systems (resolution, SMT) require formulas in PNF or a variant (Skolem normal form, where ∃ is replaced by Skolem functions). You'll see this in Phase 17.

### Skolemization (preview)

`∀x ∃y. P(x, y)` is replaced by `∀x. P(x, f(x))` where `f` is a fresh function symbol. The Skolem function *captures* the dependence of y on x. This is how SMT solvers strip existentials before going to SAT.

## Build It

### Step 1: Represent predicate logic ASTs

Predicate ASTs add `PredVar(name, args)`, `ForAll(var, body)`, `Exists(var, body)` on top of Lesson 01's propositional AST.

### Step 2: An interpretation = (domain, predicate truth-tables)

```python
@dataclass
class Interpretation:
    domain: List[object]
    predicates: Dict[str, Callable[..., bool]]
```

E.g., domain = `[1..50]`, `predicates["Prime"] = is_prime`.

### Step 3: Evaluate over a finite domain

For `ForAll(x, body)`: iterate every value in the domain, bind x to it, evaluate body, AND the results. For `Exists`: same but OR.

### Step 4: Verify "every prime > 2 is odd"

The lesson's `code/main.py` defines the predicates `Prime(x)`, `Odd(x)`, `Gt(x, y)` over `domain = range(2, 50)` and confirms `∀x. (Prime(x) ∧ Gt(x, 2)) → Odd(x)` evaluates to True.

### Step 5: Counterexample search for ∀ statements

When `∀x. P(x)` is false, the evaluator returns the *witness*: the first `x` that fails. This is what SMT solvers report as a "model" for the negation of a property.

Run `python3 code/main.py` to see all of the above.

## Use It

- **SMT solvers** (Z3, CVC5) accept predicate logic over rich theories (integers, reals, arrays, bitvectors). The first thing they do internally is Skolemize and CNF-ify.
- **Formal verification tools** (TLA+, Coq, Lean, Isabelle) all use predicate logic (often higher-order) as their core language. A TLA+ spec is essentially predicate logic over discrete states.
- **Type systems** with universal/existential types (Rust generics, Haskell `forall`, Java wildcards) are predicate logic at the type level: `forall T. T -> T` is the polymorphic identity.
- **Database queries** with `EXISTS` / `NOT EXISTS` are SQL's surface syntax for predicate logic over tables.

## Read the Source

- [Z3 tutorial](https://microsoft.github.io/z3guide/) — interactive intro to a real SMT solver; see quantifiers actually getting solved.
- *Mathematical Logic for Computer Science* (Ben-Ari) — clear textbook intro, light on prerequisites.
- [The TLA+ Hyperbook](https://lamport.azurewebsites.net/tla/hyperbook.html) — Leslie Lamport's guide to specifying systems in predicate logic.

## Ship It

This lesson ships **`outputs/predicate_check.py`** — a small CLI/library for checking predicate-logic formulas over finite domains, with counterexample reporting.

## Exercises

1. **Easy.** Translate "no two distinct integers have the same square" into predicate logic. Verify on `domain = range(-10, 11)`.
2. **Medium.** Verify the equivalence `∀x. P(x) → Q(x)` ≡ `¬∃x. (P(x) ∧ ¬Q(x))` on a few interpretations using the evaluator.
3. **Hard.** Write a function `prenex(f)` that puts a formula into Prenex Normal Form (all quantifiers in front). You'll need to handle alpha-renaming when pushing quantifiers past sub-formulas with name clashes.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Predicate | "A property" | A function from one or more objects to true/false |
| Quantifier | "∀ or ∃" | An operator that converts a predicate over x into a proposition by ranging over all (or some) x in the domain |
| Bound variable | "Local to the quantifier" | A variable named by an enclosing ∀ or ∃; its name doesn't affect meaning |
| Skolemization | "Eliminating ∃" | Replacing `∃y` with a function `f(...)` of the universally quantified vars in scope, capturing the dependence |
| Prenex normal form | "All quantifiers in front" | Equivalent rewriting that pulls every ∀/∃ to the outside |

## Further Reading

- *Logic in Computer Science* by Huth & Ryan, Chapters 2–3.
- [The Lean 4 tutorial](https://leanprover.github.io/theorem_proving_in_lean4/) — predicate logic in a modern proof assistant.
- [SMT-LIB v2 standard](https://smt-lib.org/) — what every modern SMT solver consumes.
